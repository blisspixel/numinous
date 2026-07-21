use numinous_broadcast::{
    ConsentMachine, ConsentState, ConsentTicket, HandshakeResponse, MAX_HANDSHAKE_ATTEMPTS,
    PairingCode, PublicTool, PublicToolEvent, SessionId, configure_handshake_stream,
    configure_public_stream, numinous_compatibility, read_handshake_proof, read_handshake_response,
    write_handshake_request,
};
use serde_json::Value;
use std::fmt;
use std::io::{BufReader, Read};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

const WRITER_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug)]
struct SessionInner {
    machine: Option<Arc<ConsentMachine>>,
    session_id: Option<SessionId>,
    writer: Option<JoinHandle<()>>,
    monitor: Option<JoinHandle<()>>,
    failed_starts: u8,
}

/// Process-local producer for one explicitly consented App viewer session.
pub struct SessionBroadcast {
    inner: Mutex<SessionInner>,
    lifecycle: Mutex<()>,
}

impl SessionBroadcast {
    /// Creates a disabled producer with no listener, queue, or worker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SessionInner {
                machine: None,
                session_id: None,
                writer: None,
                monitor: None,
                failed_starts: 0,
            }),
            lifecycle: Mutex::new(()),
        }
    }

    /// Authenticates one short-lived loopback offer and starts public emission.
    pub fn start(&self, encoded_code: &str) -> Result<SessionSnapshot, SessionError> {
        let _lifecycle = self.lock_lifecycle();
        self.retire_previous()?;
        if self.lock().failed_starts >= MAX_HANDSHAKE_ATTEMPTS {
            return Err(SessionError::PairingRejected);
        }
        let result = self.start_authenticated(encoded_code);
        let mut inner = self.lock();
        match result {
            Ok(snapshot) => {
                inner.failed_starts = 0;
                Ok(snapshot)
            }
            Err(error) => {
                if error == SessionError::PairingRejected {
                    inner.failed_starts = inner.failed_starts.saturating_add(1);
                }
                Err(error)
            }
        }
    }

    fn start_authenticated(&self, encoded_code: &str) -> Result<SessionSnapshot, SessionError> {
        let compatibility = numinous_compatibility().map_err(|_| SessionError::Unavailable)?;
        let code = PairingCode::parse(encoded_code, SystemTime::now())
            .map_err(|_| SessionError::PairingRejected)?;
        let endpoint = code.endpoint();
        let stream =
            TcpStream::connect_timeout(&endpoint.into(), numinous_broadcast::HANDSHAKE_TIMEOUT)
                .map_err(|_| SessionError::PairingRejected)?;
        configure_handshake_stream(&stream).map_err(|_| SessionError::PairingRejected)?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|_| SessionError::Unavailable)?);
        let proof = read_handshake_proof(&mut reader).map_err(|_| SessionError::PairingRejected)?;
        if !code.verifies_host_proof(&proof) {
            return Err(SessionError::PairingRejected);
        }
        let request = code.handshake_request(compatibility.clone());
        let mut writer = stream.try_clone().map_err(|_| SessionError::Unavailable)?;
        write_handshake_request(&mut writer, &request)
            .map_err(|_| SessionError::PairingRejected)?;
        let response =
            read_handshake_response(&mut reader).map_err(|_| SessionError::PairingRejected)?;
        let (session_id, remote_epoch) = match response {
            HandshakeResponse::Accepted {
                session_id,
                consent_epoch,
                compatibility: host,
            } if host.is_compatible_with(&compatibility) => (session_id, consent_epoch),
            HandshakeResponse::Accepted { .. } | HandshakeResponse::Rejected => {
                return Err(SessionError::PairingRejected);
            }
        };

        let machine = Arc::new(ConsentMachine::new(session_id, compatibility));
        machine
            .begin_awaiting()
            .map_err(|_| SessionError::Unavailable)?;
        let local_epoch = machine.allow().map_err(|_| SessionError::Unavailable)?;
        if local_epoch != remote_epoch {
            machine.stop().map_err(|_| SessionError::Unavailable)?;
            return Err(SessionError::PairingRejected);
        }

        configure_public_stream(&stream).map_err(|_| SessionError::PairingRejected)?;
        let monitor_stream = stream.try_clone().map_err(|_| SessionError::Unavailable)?;
        let writer_machine = Arc::clone(&machine);
        let writer_handle = thread::Builder::new()
            .name("numinous-broadcast-writer".to_string())
            .spawn(move || writer_loop(stream, writer_machine))
            .map_err(|_| SessionError::Unavailable)?;
        let monitor_machine = Arc::clone(&machine);
        let monitor_handle = match thread::Builder::new()
            .name("numinous-broadcast-monitor".to_string())
            .spawn(move || monitor_loop(monitor_stream, monitor_machine))
        {
            Ok(handle) => handle,
            Err(_) => {
                let _ = machine.stop();
                let _ = writer_handle.join();
                return Err(SessionError::Unavailable);
            }
        };

        let mut inner = self.lock();
        inner.machine = Some(machine);
        inner.session_id = Some(session_id);
        inner.writer = Some(writer_handle);
        inner.monitor = Some(monitor_handle);
        Ok(snapshot(&inner))
    }

    /// Captures one validated public call at its consent boundary.
    #[must_use]
    pub fn capture(&self, tool: PublicTool, arguments: &Value) -> Option<PublicCall> {
        let machine = self.lock().machine.clone()?;
        let ticket = machine.capture()?;
        Some(PublicCall {
            machine,
            ticket,
            tool,
            arguments: arguments.clone(),
        })
    }

    /// Pauses emission after any already-admitted bounded write.
    pub fn pause(&self) -> Result<SessionSnapshot, SessionError> {
        let _lifecycle = self.lock_lifecycle();
        let machine = self.machine()?;
        machine.pause().map_err(|_| SessionError::InvalidState)?;
        Ok(self.status())
    }

    /// Resumes an authenticated paused connection under a fresh epoch.
    pub fn resume(&self) -> Result<SessionSnapshot, SessionError> {
        let _lifecycle = self.lock_lifecycle();
        let machine = self.machine()?;
        machine.resume().map_err(|_| SessionError::InvalidState)?;
        Ok(self.status())
    }

    /// Stops the session, transmits the final marker when possible, and joins workers.
    pub fn stop(&self) -> Result<SessionSnapshot, SessionError> {
        let _lifecycle = self.lock_lifecycle();
        let machine = self.machine()?;
        machine.stop().map_err(|_| SessionError::InvalidState)?;
        self.join_workers()?;
        Ok(self.status())
    }

    /// Returns a capability-free bounded status snapshot.
    #[must_use]
    pub fn status(&self) -> SessionSnapshot {
        snapshot(&self.lock())
    }

    fn machine(&self) -> Result<Arc<ConsentMachine>, SessionError> {
        self.lock().machine.clone().ok_or(SessionError::NoSession)
    }

    fn retire_previous(&self) -> Result<(), SessionError> {
        let previous = self.lock().machine.clone();
        if let Some(machine) = previous {
            if matches!(
                machine.status().state,
                ConsentState::Live | ConsentState::Paused
            ) {
                return Err(SessionError::AlreadyActive);
            }
            let _ = machine.stop();
            self.join_workers()?;
            let mut inner = self.lock();
            inner.machine = None;
            inner.session_id = None;
        }
        Ok(())
    }

    fn join_workers(&self) -> Result<(), SessionError> {
        let (writer, monitor) = {
            let mut inner = self.lock();
            (inner.writer.take(), inner.monitor.take())
        };
        join_worker(writer)?;
        join_worker(monitor)
    }

    fn lock(&self) -> MutexGuard<'_, SessionInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_lifecycle(&self) -> MutexGuard<'_, ()> {
        self.lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for SessionBroadcast {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SessionBroadcast {
    fn drop(&mut self) {
        if let Some(machine) = self
            .inner
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .machine
            .as_ref()
        {
            let _ = machine.stop();
        }
        let inner = self
            .inner
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let writer = inner.writer.take();
        let monitor = inner.monitor.take();
        let _ = join_worker(writer);
        let _ = join_worker(monitor);
    }
}

/// One consent ticket plus its already validated replay action.
pub struct PublicCall {
    machine: Arc<ConsentMachine>,
    ticket: ConsentTicket,
    tool: PublicTool,
    arguments: Value,
}

impl PublicCall {
    /// Commits a successful public result without touching the viewer socket.
    pub fn commit(self, result: &Value) {
        let Ok(event) = PublicToolEvent::new(self.tool, &self.arguments, result) else {
            let _ = self.machine.stop();
            return;
        };
        if self
            .machine
            .prepare_and_commit(self.ticket, &event)
            .is_err()
        {
            let _ = self.machine.stop();
        }
    }
}

/// Capability-free status exposed to the consenting guest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionSnapshot {
    /// Current consent state.
    pub state: &'static str,
    /// Nonsecret live-session identity.
    pub session_id: Option<String>,
    /// Current consent epoch.
    pub consent_epoch: u64,
    /// Sequence assigned to the next committed public event.
    pub next_public_sequence: u64,
    /// Cumulative dropped public events.
    pub dropped_public_events: u64,
    /// Events waiting for the writer.
    pub queued_events: usize,
    /// Conservatively reserved queue bytes.
    pub queued_bytes: usize,
}

/// A safe, capability-free control failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionError {
    /// A live or paused viewer already owns the process session.
    AlreadyActive,
    /// No authenticated viewer session exists.
    NoSession,
    /// The requested action does not match the consent state.
    InvalidState,
    /// The code, listener, handshake, or compatibility check was rejected.
    PairingRejected,
    /// A local invariant or worker could not be established.
    Unavailable,
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyActive => formatter.write_str("a viewer session is already active"),
            Self::NoSession => formatter.write_str("no viewer session is active"),
            Self::InvalidState => {
                formatter.write_str("that action is unavailable in the current state")
            }
            Self::PairingRejected => formatter.write_str("pairing was rejected or unavailable"),
            Self::Unavailable => formatter.write_str("the local broadcast is unavailable"),
        }
    }
}

fn snapshot(inner: &SessionInner) -> SessionSnapshot {
    let Some(machine) = inner.machine.as_ref() else {
        return SessionSnapshot {
            state: "disabled",
            session_id: None,
            consent_epoch: 0,
            next_public_sequence: 0,
            dropped_public_events: 0,
            queued_events: 0,
            queued_bytes: 0,
        };
    };
    let status = machine.status();
    SessionSnapshot {
        state: state_name(status.state),
        session_id: inner.session_id.map(|session_id| session_id.to_string()),
        consent_epoch: status.epoch,
        next_public_sequence: status.next_public_sequence,
        dropped_public_events: status.dropped_public_events,
        queued_events: status.queue.queued_events,
        queued_bytes: status.queue.queued_bytes,
    }
}

const fn state_name(state: ConsentState) -> &'static str {
    match state {
        ConsentState::Disabled => "disabled",
        ConsentState::AwaitingGuest => "awaiting_guest",
        ConsentState::Live => "live",
        ConsentState::Paused => "paused",
        ConsentState::Stopped => "stopped",
    }
}

fn writer_loop(mut stream: TcpStream, machine: Arc<ConsentMachine>) {
    loop {
        match machine.wait_lease(WRITER_POLL_INTERVAL) {
            Ok(Some(lease)) => {
                if lease.write_to(&mut stream).is_err() {
                    break;
                }
            }
            Ok(None) if machine.status().state == ConsentState::Stopped => break,
            Ok(None) => {}
            Err(_) => {
                let _ = machine.stop();
                break;
            }
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

fn monitor_loop(mut stream: TcpStream, machine: Arc<ConsentMachine>) {
    let mut unexpected = [0_u8; 1];
    loop {
        match stream.read(&mut unexpected) {
            Ok(0) | Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    let _ = machine.stop();
    let _ = stream.shutdown(Shutdown::Both);
}

fn join_worker(worker: Option<JoinHandle<()>>) -> Result<(), SessionError> {
    match worker {
        Some(worker) => worker.join().map_err(|_| SessionError::Unavailable),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionBroadcast, SessionError};
    use numinous_broadcast::{
        Compatibility, ConsentMachine, ControlMarker, HandshakeResponse, MAX_HANDSHAKE_ATTEMPTS,
        PairingGate, PairingOffer, PairingVerdict, PublicTool, PublicToolEvent, WireMessage,
        configure_handshake_stream, configure_public_stream, numinous_compatibility,
        read_handshake_request, read_public_message, write_handshake_proof,
        write_handshake_response,
    };
    use serde_json::json;
    use std::io::{BufReader, Read};
    use std::net::{Ipv4Addr, TcpListener, TcpStream};
    use std::num::NonZeroU16;
    use std::sync::{Arc, mpsc};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime};

    fn wait_for_state(session: &SessionBroadcast, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while session.status().state != expected && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(session.status().state, expected);
    }

    fn accept_guest(
        listener: TcpListener,
        mut gate: PairingGate,
        compatibility: Compatibility,
    ) -> TcpStream {
        let (mut stream, _) = listener.accept().expect("accept");
        configure_handshake_stream(&stream).expect("handshake bounds");
        write_handshake_proof(&mut stream, &gate.host_proof()).expect("host proof");
        let mut reader = BufReader::new(stream.try_clone().expect("reader clone"));
        let request = read_handshake_request(&mut reader).expect("handshake request");
        let PairingVerdict::Accepted { session_id } = gate.verify(&request, SystemTime::now())
        else {
            panic!("valid pairing must be accepted");
        };
        let host_machine = ConsentMachine::new(session_id, compatibility.clone());
        host_machine.begin_awaiting().expect("awaiting");
        let consent_epoch = host_machine.allow().expect("allow");
        write_handshake_response(
            &mut stream,
            &HandshakeResponse::Accepted {
                session_id,
                consent_epoch,
                compatibility,
            },
        )
        .expect("accepted response");
        configure_public_stream(&stream).expect("public bounds");
        stream
    }

    #[test]
    fn a_real_loopback_session_transmits_only_after_consent() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("loopback listener");
        let port = NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
        let offer = PairingOffer::generate(port, SystemTime::now()).expect("offer");
        let code = offer.display_code();
        let compatibility = numinous_compatibility().expect("compatibility");
        let gate = offer.into_gate(compatibility.clone());
        let (event_tx, event_rx) = mpsc::channel();

        let host = thread::spawn(move || {
            let stream = accept_guest(listener, gate, compatibility);
            let mut reader = BufReader::new(stream);
            let event =
                read_public_message::<_, PublicToolEvent>(&mut reader).expect("one public event");
            event_tx.send(event).expect("send event");
        });

        let session = SessionBroadcast::new();
        assert_eq!(session.status().state, "disabled");
        let started = session.start(&code).expect("start session");
        assert_eq!(started.state, "live");
        assert_eq!(started.next_public_sequence, 0);
        let call = session
            .capture(
                PublicTool::PlayRoom,
                &json!({"id": "times-tables", "t": 0.25}),
            )
            .expect("live public call");
        call.commit(&json!({"content": [], "structuredContent": {"room": "times-tables"}}));

        let WireMessage::Event(event) = event_rx.recv().expect("event received") else {
            panic!("first public message must be an event");
        };
        assert_eq!(event.public_sequence, 0);
        assert_eq!(event.event.tool, PublicTool::PlayRoom);
        assert_eq!(event.event.arguments["id"], "times-tables");
        host.join().expect("host");
        wait_for_state(&session, "stopped");
    }

    #[test]
    fn pause_resume_and_stop_cross_the_real_transport_in_order() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("loopback listener");
        let port = NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
        let offer = PairingOffer::generate(port, SystemTime::now()).expect("offer");
        let code = offer.display_code();
        let compatibility = numinous_compatibility().expect("compatibility");
        let gate = offer.into_gate(compatibility.clone());
        let (control_tx, control_rx) = mpsc::channel();

        let host = thread::spawn(move || {
            let stream = accept_guest(listener, gate, compatibility);
            let mut reader = BufReader::new(stream);
            for _ in 0..3 {
                let message =
                    read_public_message::<_, PublicToolEvent>(&mut reader).expect("control marker");
                let WireMessage::Control {
                    consent_epoch,
                    marker,
                    ..
                } = message
                else {
                    panic!("no public event was admitted");
                };
                control_tx
                    .send((consent_epoch, marker))
                    .expect("report marker");
            }
        });

        let session = SessionBroadcast::new();
        assert_eq!(session.start(&code).expect("start").consent_epoch, 2);
        assert_eq!(session.pause().expect("pause").state, "paused");
        assert_eq!(
            control_rx.recv().expect("paused marker"),
            (3, ControlMarker::Paused)
        );
        assert!(
            session
                .capture(PublicTool::PlayRoom, &json!({"id": "times-tables"}))
                .is_none()
        );
        assert_eq!(session.resume().expect("resume").state, "live");
        assert_eq!(
            control_rx.recv().expect("resumed marker"),
            (4, ControlMarker::Resumed)
        );
        assert_eq!(session.stop().expect("stop").state, "stopped");
        assert_eq!(
            control_rx.recv().expect("stopped marker"),
            (5, ControlMarker::Stopped)
        );
        host.join().expect("host");
    }

    #[test]
    fn pairing_failures_never_reflect_the_supplied_capability() {
        let session = SessionBroadcast::new();
        let secret = "numinous1.7.private-capability";
        let error = session.start(secret).expect_err("invalid code");
        assert_eq!(error, SessionError::PairingRejected);
        assert!(!error.to_string().contains(secret));
        assert_eq!(session.status().state, "disabled");
    }

    #[test]
    fn an_unproven_loopback_listener_receives_no_guest_bytes() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let port = NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
        let code = PairingOffer::generate(port, SystemTime::now())
            .expect("offer")
            .display_code();
        let guest = thread::spawn(move || SessionBroadcast::new().start(&code));

        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_millis(100)))
            .expect("bounded observation");
        let mut unexpected = [0_u8; 1];
        assert!(matches!(
            stream.read(&mut unexpected),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                )
        ));
        drop(stream);
        assert_eq!(
            guest.join().expect("guest thread"),
            Err(SessionError::PairingRejected)
        );
    }

    #[test]
    fn repeated_failed_starts_exhaust_the_process_pairing_budget() {
        let session = SessionBroadcast::new();
        for _ in 0..MAX_HANDSHAKE_ATTEMPTS {
            assert_eq!(session.start("invalid"), Err(SessionError::PairingRejected));
        }

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let port = NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
        let fresh_code = PairingOffer::generate(port, SystemTime::now())
            .expect("fresh offer")
            .display_code();
        assert_eq!(
            session.start(&fresh_code),
            Err(SessionError::PairingRejected)
        );
        assert!(matches!(
            listener.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
    }

    #[test]
    fn active_replacement_is_refused_and_a_stopped_session_can_be_replaced() {
        fn offer_and_host() -> (String, thread::JoinHandle<()>) {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
            let port =
                NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
            let offer = PairingOffer::generate(port, SystemTime::now()).expect("offer");
            let code = offer.display_code();
            let compatibility = numinous_compatibility().expect("compatibility");
            let gate = offer.into_gate(compatibility.clone());
            let host = thread::spawn(move || {
                let stream = accept_guest(listener, gate, compatibility);
                let mut reader = BufReader::new(stream);
                let message =
                    read_public_message::<_, PublicToolEvent>(&mut reader).expect("stopped marker");
                assert!(matches!(
                    message,
                    WireMessage::Control {
                        marker: ControlMarker::Stopped,
                        ..
                    }
                ));
            });
            (code, host)
        }

        let session = SessionBroadcast::default();
        let (first_code, first_host) = offer_and_host();
        session.start(&first_code).expect("first start");
        let error = session
            .start("not-even-parsed")
            .expect_err("active session must win");
        assert_eq!(error, SessionError::AlreadyActive);
        assert_eq!(error.to_string(), "a viewer session is already active");
        session.stop().expect("first stop");
        first_host.join().expect("first host");

        let (second_code, second_host) = offer_and_host();
        assert_eq!(
            session.start(&second_code).expect("replacement").state,
            "live"
        );
        session.stop().expect("second stop");
        second_host.join().expect("second host");
    }

    #[test]
    fn concurrent_starts_reserve_exactly_one_lifecycle_slot() {
        let first_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("first listener");
        let first_port = NonZeroU16::new(first_listener.local_addr().expect("address").port())
            .expect("first port");
        let first_offer =
            PairingOffer::generate(first_port, SystemTime::now()).expect("first offer");
        let first_code = first_offer.display_code();
        let compatibility = numinous_compatibility().expect("compatibility");
        let mut first_gate = first_offer.into_gate(compatibility.clone());
        let (request_ready_tx, request_ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let first_host = thread::spawn(move || {
            let (mut stream, _) = first_listener.accept().expect("first accept");
            configure_handshake_stream(&stream).expect("handshake bounds");
            write_handshake_proof(&mut stream, &first_gate.host_proof()).expect("host proof");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let request = read_handshake_request(&mut reader).expect("request");
            let PairingVerdict::Accepted { session_id } =
                first_gate.verify(&request, SystemTime::now())
            else {
                panic!("first pairing must authenticate");
            };
            request_ready_tx.send(()).expect("ready");
            release_rx.recv().expect("release first response");
            let host_machine = ConsentMachine::new(session_id, compatibility.clone());
            host_machine.begin_awaiting().expect("awaiting");
            let consent_epoch = host_machine.allow().expect("allow");
            write_handshake_response(
                &mut stream,
                &HandshakeResponse::Accepted {
                    session_id,
                    consent_epoch,
                    compatibility,
                },
            )
            .expect("accepted response");
            configure_public_stream(&stream).expect("public bounds");
            let mut reader = BufReader::new(stream);
            let message =
                read_public_message::<_, PublicToolEvent>(&mut reader).expect("stopped marker");
            assert!(matches!(
                message,
                WireMessage::Control {
                    marker: ControlMarker::Stopped,
                    ..
                }
            ));
        });

        let second_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("second listener");
        second_listener
            .set_nonblocking(true)
            .expect("nonblocking second listener");
        let second_port = NonZeroU16::new(second_listener.local_addr().expect("address").port())
            .expect("second port");
        let second_code = PairingOffer::generate(second_port, SystemTime::now())
            .expect("second offer")
            .display_code();

        let session = Arc::new(SessionBroadcast::new());
        let first_session = Arc::clone(&session);
        let first_start = thread::spawn(move || first_session.start(&first_code));
        request_ready_rx.recv().expect("first request reached host");

        let second_session = Arc::clone(&session);
        let (second_attempt_tx, second_attempt_rx) = mpsc::channel();
        let second_start = thread::spawn(move || {
            second_attempt_tx.send(()).expect("second attempt");
            second_session.start(&second_code)
        });
        second_attempt_rx.recv().expect("second start entered");
        release_tx.send(()).expect("release first start");

        assert!(first_start.join().expect("first start thread").is_ok());
        assert_eq!(
            second_start.join().expect("second start thread"),
            Err(SessionError::AlreadyActive)
        );
        assert!(matches!(
            second_listener.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
        session.stop().expect("stop winner");
        first_host.join().expect("first host");
    }

    #[test]
    fn malformed_internal_projection_stops_instead_of_weakening_the_wire_type() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let port = NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
        let offer = PairingOffer::generate(port, SystemTime::now()).expect("offer");
        let code = offer.display_code();
        let compatibility = numinous_compatibility().expect("compatibility");
        let gate = offer.into_gate(compatibility.clone());
        let host = thread::spawn(move || {
            let stream = accept_guest(listener, gate, compatibility);
            let mut reader = BufReader::new(stream);
            let message =
                read_public_message::<_, PublicToolEvent>(&mut reader).expect("stopped marker");
            assert!(matches!(
                message,
                WireMessage::Control {
                    marker: ControlMarker::Stopped,
                    ..
                }
            ));
        });

        let session = SessionBroadcast::new();
        session.start(&code).expect("start");
        let call = session
            .capture(PublicTool::PlayRoom, &json!([]))
            .expect("live ticket");
        call.commit(&json!({"content": []}));
        assert_eq!(session.status().state, "stopped");
        session.stop().expect("join stopped session");
        host.join().expect("host");
    }

    #[test]
    fn a_host_epoch_mismatch_is_rejected_before_workers_or_content() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let port = NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
        let offer = PairingOffer::generate(port, SystemTime::now()).expect("offer");
        let code = offer.display_code();
        let compatibility = numinous_compatibility().expect("compatibility");
        let mut gate = offer.into_gate(compatibility.clone());
        let host = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            configure_handshake_stream(&stream).expect("handshake bounds");
            write_handshake_proof(&mut stream, &gate.host_proof()).expect("host proof");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let request = read_handshake_request(&mut reader).expect("request");
            let PairingVerdict::Accepted { session_id } = gate.verify(&request, SystemTime::now())
            else {
                panic!("pairing must authenticate");
            };
            write_handshake_response(
                &mut stream,
                &HandshakeResponse::Accepted {
                    session_id,
                    consent_epoch: 99,
                    compatibility,
                },
            )
            .expect("mismatched response");
        });

        let session = SessionBroadcast::new();
        assert_eq!(session.start(&code), Err(SessionError::PairingRejected));
        assert_eq!(session.status().state, "disabled");
        host.join().expect("host");
    }

    #[test]
    fn control_error_copy_is_stable_and_capability_free() {
        let session = SessionBroadcast::new();
        assert_eq!(session.pause(), Err(SessionError::NoSession));
        for (error, expected) in [
            (SessionError::NoSession, "no viewer session is active"),
            (
                SessionError::InvalidState,
                "that action is unavailable in the current state",
            ),
            (
                SessionError::Unavailable,
                "the local broadcast is unavailable",
            ),
        ] {
            assert_eq!(error.to_string(), expected);
        }
        assert_eq!(
            super::state_name(numinous_broadcast::ConsentState::Disabled),
            "disabled"
        );
        assert_eq!(
            super::state_name(numinous_broadcast::ConsentState::AwaitingGuest),
            "awaiting_guest"
        );
    }
}
