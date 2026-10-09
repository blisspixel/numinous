use crate::fingerprint::{Compatibility, WIRE_VERSION};
use crate::hex;
use crate::wire::{HandshakeHello, HandshakeProof, HandshakeRequest, SessionId};
use getrandom::fill;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::error::Error;
use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::num::NonZeroU16;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

/// Maximum encoded pairing-code length.
pub const MAX_PAIRING_CODE_BYTES: usize = 128;
/// Lifetime of a pairing offer.
pub const PAIRING_TTL: Duration = Duration::from_secs(5 * 60);
/// Failed handshakes allowed before an offer is revoked.
pub const MAX_HANDSHAKE_ATTEMPTS: u8 = 8;
const PREFIX: &str = "numinous2";
const PROOF_DOMAIN: &[u8] = b"numinous local broadcast transcript v2\0";

#[derive(Clone, Copy)]
struct Capability([u8; 16]);

impl Capability {
    fn generate() -> Result<Self, PairingError> {
        let mut bytes = [0; 16];
        fill(&mut bytes).map_err(|_| PairingError::RandomUnavailable)?;
        Ok(Self(bytes))
    }
}

impl HandshakeHello {
    /// Draws a fresh, nonsecret challenge for exactly one connection.
    pub fn generate() -> Result<Self, PairingError> {
        Ok(Self {
            wire_version: WIRE_VERSION,
            nonce: fresh_nonce()?,
        })
    }
}

fn fresh_nonce() -> Result<String, PairingError> {
    let mut bytes = [0; 32];
    fill(&mut bytes).map_err(|_| PairingError::RandomUnavailable)?;
    Ok(hex::encode(&bytes))
}

/// A parsed guest-side target containing a one-use secret capability.
#[derive(Clone)]
pub struct PairingCode {
    port: NonZeroU16,
    expires_at_unix_ms: u64,
    capability: Capability,
}

impl PairingCode {
    /// Parses and validates a pairing code at `now`.
    pub fn parse(input: &str, now: SystemTime) -> Result<Self, PairingError> {
        if input.is_empty() || input.len() > MAX_PAIRING_CODE_BYTES || !input.is_ascii() {
            return Err(PairingError::InvalidCode);
        }
        let mut parts = input.split('.');
        if parts.next() != Some(PREFIX) {
            return Err(PairingError::InvalidCode);
        }
        let port = parts
            .next()
            .and_then(|part| part.parse().ok())
            .and_then(NonZeroU16::new)
            .ok_or(PairingError::InvalidCode)?;
        let expires_at_unix_ms = parts
            .next()
            .and_then(|part| part.parse().ok())
            .ok_or(PairingError::InvalidCode)?;
        let capability = parts
            .next()
            .and_then(hex::decode)
            .map(Capability)
            .ok_or(PairingError::InvalidCode)?;
        if parts.next().is_some() {
            return Err(PairingError::InvalidCode);
        }
        if unix_millis(now)? >= expires_at_unix_ms {
            return Err(PairingError::Expired);
        }
        Ok(Self {
            port,
            expires_at_unix_ms,
            capability,
        })
    }

    /// Returns the fixed loopback endpoint encoded by this code.
    #[must_use]
    pub const fn endpoint(&self) -> SocketAddrV4 {
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, self.port.get())
    }

    /// Builds a guest proof only after the fresh host transcript is verified.
    pub fn handshake_request(
        &self,
        hello: &HandshakeHello,
        proof: &HandshakeProof,
        compatibility: Compatibility,
    ) -> Result<HandshakeRequest, PairingError> {
        if !self.verifies_host_proof(hello, proof)
            || !proof.compatibility.is_compatible_with(&compatibility)
        {
            return Err(PairingError::InvalidCode);
        }
        Ok(HandshakeRequest {
            wire_version: WIRE_VERSION,
            proof: hex::encode(
                &self
                    .transcript_proof(b"guest", proof)
                    .ok_or(PairingError::InvalidCode)?,
            ),
            compatibility,
        })
    }

    /// Verifies the host proof against the fresh challenge sent on this connection.
    #[must_use]
    pub fn verifies_host_proof(&self, hello: &HandshakeHello, proof: &HandshakeProof) -> bool {
        hello.wire_version == WIRE_VERSION
            && proof.wire_version == WIRE_VERSION
            && hello.nonce == proof.client_nonce
            && self.matches_transcript(b"host", proof, &proof.proof)
    }

    fn transcript_proof(&self, role: &[u8], proof: &HandshakeProof) -> Option<[u8; 32]> {
        let client = hex::decode::<32>(&proof.client_nonce)?;
        let server = hex::decode::<32>(&proof.server_nonce)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.capability.0).ok()?;
        mac.update(PROOF_DOMAIN);
        mac.update(role);
        mac.update(&proof.wire_version.to_be_bytes());
        mac.update(&self.port.get().to_be_bytes());
        mac.update(&self.expires_at_unix_ms.to_be_bytes());
        mac.update(&client);
        mac.update(&server);
        mac.update(proof.session_id.to_string().as_bytes());
        mac.update(&proof.consent_epoch.to_be_bytes());
        mac.update(&proof.compatibility.wire_version.to_be_bytes());
        mac.update(&proof.compatibility.replay_abi_version.to_be_bytes());
        mac.update(proof.compatibility.fingerprint.as_bytes());
        Some(mac.finalize().into_bytes().into())
    }

    fn matches_transcript(&self, role: &[u8], proof: &HandshakeProof, candidate: &str) -> bool {
        let Some(expected) = self.transcript_proof(role, proof) else {
            return false;
        };
        let decoded = hex::decode::<32>(candidate);
        bool::from(expected.ct_eq(&decoded.unwrap_or([0; 32]))) && decoded.is_some()
    }

    fn encode(&self) -> String {
        format!(
            "{PREFIX}.{}.{}.{}",
            self.port,
            self.expires_at_unix_ms,
            hex::encode(&self.capability.0)
        )
    }
}

impl fmt::Debug for PairingCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PairingCode")
            .field("endpoint", &self.endpoint())
            .field("expires_at_unix_ms", &self.expires_at_unix_ms)
            .field("capability", &"[REDACTED]")
            .finish()
    }
}

/// A fresh human-side pairing offer.
pub struct PairingOffer {
    code: PairingCode,
    session_id: SessionId,
    deadline: Instant,
}

impl PairingOffer {
    /// Creates a five-minute offer for an already-bound loopback port.
    pub fn generate(port: NonZeroU16, now: SystemTime) -> Result<Self, PairingError> {
        Self::generate_at(port, now, Instant::now())
    }

    fn generate_at(
        port: NonZeroU16,
        now: SystemTime,
        monotonic_now: Instant,
    ) -> Result<Self, PairingError> {
        let now_ms = unix_millis(now)?;
        let ttl_ms =
            u64::try_from(PAIRING_TTL.as_millis()).map_err(|_| PairingError::ClockOutOfRange)?;
        let expires_at_unix_ms = now_ms
            .checked_add(ttl_ms)
            .ok_or(PairingError::ClockOutOfRange)?;
        let code = PairingCode {
            port,
            expires_at_unix_ms,
            capability: Capability::generate()?,
        };
        debug_assert!(code.encode().len() <= MAX_PAIRING_CODE_BYTES);
        Ok(Self {
            code,
            session_id: SessionId::generate().map_err(|_| PairingError::RandomUnavailable)?,
            deadline: monotonic_now
                .checked_add(PAIRING_TTL)
                .ok_or(PairingError::ClockOutOfRange)?,
        })
    }

    /// Returns the code the human may choose to share with a guest.
    #[must_use]
    pub fn display_code(&self) -> String {
        self.code.encode()
    }

    /// Converts the offer into a one-use authentication gate.
    #[must_use]
    pub fn into_gate(self, compatibility: Compatibility) -> PairingGate {
        PairingGate {
            code: self.code,
            session_id: self.session_id,
            compatibility,
            deadline: self.deadline,
            failures: 0,
            revoked: false,
            challenge: None,
        }
    }
}

impl fmt::Debug for PairingOffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PairingOffer")
            .field("code", &self.code)
            .field("session_id", &self.session_id)
            .finish()
    }
}

/// One-use host-side authentication state.
pub struct PairingGate {
    code: PairingCode,
    session_id: SessionId,
    compatibility: Compatibility,
    deadline: Instant,
    failures: u8,
    revoked: bool,
    challenge: Option<HandshakeProof>,
}

impl PairingGate {
    /// Replaces the per-connection challenge and authenticates its transcript.
    pub fn host_proof(&mut self, hello: &HandshakeHello) -> Result<HandshakeProof, PairingError> {
        self.challenge = None;
        if self.revoked
            || hello.wire_version != WIRE_VERSION
            || hex::decode::<32>(&hello.nonce).is_none()
        {
            return Err(PairingError::InvalidCode);
        }
        let consent =
            crate::consent::ConsentMachine::new(self.session_id, self.compatibility.clone());
        consent
            .begin_awaiting()
            .map_err(|_| PairingError::InvalidCode)?;
        let consent_epoch = consent.allow().map_err(|_| PairingError::InvalidCode)?;
        let mut proof = HandshakeProof {
            wire_version: WIRE_VERSION,
            client_nonce: hello.nonce.clone(),
            server_nonce: fresh_nonce()?,
            session_id: self.session_id,
            consent_epoch,
            compatibility: self.compatibility.clone(),
            proof: String::new(),
        };
        proof.proof = hex::encode(
            &self
                .code
                .transcript_proof(b"host", &proof)
                .ok_or(PairingError::InvalidCode)?,
        );
        self.challenge = Some(proof.clone());
        Ok(proof)
    }

    /// Verifies one bounded handshake without reflecting secret material.
    pub fn verify(&mut self, request: &HandshakeRequest, now: SystemTime) -> PairingVerdict {
        self.verify_at(request, now, Instant::now())
    }

    fn verify_at(
        &mut self,
        request: &HandshakeRequest,
        now: SystemTime,
        monotonic_now: Instant,
    ) -> PairingVerdict {
        if self.revoked {
            return PairingVerdict::Revoked;
        }
        let Ok(now_ms) = unix_millis(now) else {
            self.revoked = true;
            return PairingVerdict::Revoked;
        };
        if now_ms >= self.code.expires_at_unix_ms || monotonic_now >= self.deadline {
            self.revoked = true;
            return PairingVerdict::Expired;
        }
        let challenge = self.challenge.take();
        let valid = request.wire_version == WIRE_VERSION
            && request
                .compatibility
                .is_compatible_with(&self.compatibility)
            && challenge.is_some_and(|proof| {
                self.code
                    .matches_transcript(b"guest", &proof, &request.proof)
            });
        if valid {
            self.revoked = true;
            return PairingVerdict::Accepted {
                session_id: self.session_id,
            };
        }
        self.failures = self.failures.saturating_add(1);
        if self.failures >= MAX_HANDSHAKE_ATTEMPTS {
            self.revoked = true;
            PairingVerdict::Revoked
        } else {
            PairingVerdict::Rejected {
                attempts_remaining: MAX_HANDSHAKE_ATTEMPTS - self.failures,
            }
        }
    }

    /// Revokes the capability without accepting another handshake.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Returns whether no future handshake can succeed.
    #[must_use]
    pub const fn is_revoked(&self) -> bool {
        self.revoked
    }
}

impl fmt::Debug for PairingGate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PairingGate")
            .field("code", &self.code)
            .field("session_id", &self.session_id)
            .field("compatibility", &self.compatibility)
            .field("failures", &self.failures)
            .field("revoked", &self.revoked)
            .finish()
    }
}

/// Result of one host-side handshake verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairingVerdict {
    /// The one-use capability and compatibility declaration were accepted.
    Accepted {
        /// Nonsecret identity assigned to the live session.
        session_id: SessionId,
    },
    /// The request failed and a bounded number of attempts remain.
    Rejected {
        /// Failed attempts remaining before revocation.
        attempts_remaining: u8,
    },
    /// The pairing offer reached its expiry time.
    Expired,
    /// The offer was consumed, stopped, or exhausted by failed attempts.
    Revoked,
}

/// Failure to create or parse a pairing offer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairingError {
    /// The code is malformed, oversized, or unsupported.
    InvalidCode,
    /// The code has reached its expiry time.
    Expired,
    /// The system clock cannot be represented by the wire format.
    ClockOutOfRange,
    /// Operating-system cryptographic randomness is unavailable.
    RandomUnavailable,
}

impl fmt::Display for PairingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode => formatter.write_str("invalid pairing code"),
            Self::Expired => formatter.write_str("pairing code expired"),
            Self::ClockOutOfRange => {
                formatter.write_str("system clock is outside the supported range")
            }
            Self::RandomUnavailable => {
                formatter.write_str("operating-system randomness is unavailable")
            }
        }
    }
}

impl Error for PairingError {}

fn unix_millis(time: SystemTime) -> Result<u64, PairingError> {
    let millis = time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| PairingError::ClockOutOfRange)?
        .as_millis();
    u64::try_from(millis).map_err(|_| PairingError::ClockOutOfRange)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Compatibility;

    fn compatibility() -> Compatibility {
        Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"]).expect("compatibility")
    }

    fn offer() -> PairingOffer {
        PairingOffer::generate(NonZeroU16::new(31_337).unwrap(), UNIX_EPOCH).unwrap()
    }

    fn prepare(offer: PairingOffer) -> (PairingCode, PairingGate) {
        let code = PairingCode::parse(&offer.display_code(), UNIX_EPOCH).unwrap();
        (code, offer.into_gate(compatibility()))
    }

    fn request(code: &PairingCode, gate: &mut PairingGate) -> HandshakeRequest {
        let hello = HandshakeHello::generate().unwrap();
        let proof = gate.host_proof(&hello).unwrap();
        code.handshake_request(&hello, &proof, compatibility())
            .unwrap()
    }

    #[test]
    fn codes_are_bounded_loopback_only_expiring_and_versioned() {
        let offer = offer();
        let encoded = offer.display_code();
        assert!(encoded.len() <= MAX_PAIRING_CODE_BYTES);
        assert!(!format!("{offer:?}").contains(&encoded));
        let code = PairingCode::parse(&encoded, UNIX_EPOCH).unwrap();
        assert_eq!(
            code.endpoint(),
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 31_337)
        );
        assert_eq!(code.encode(), encoded);
        for bad in [
            "x".repeat(MAX_PAIRING_CODE_BYTES + 1),
            "numinous2.0.300000.00000000000000000000000000000000".into(),
            "numinous1.1.300000.00000000000000000000000000000000".into(),
            format!("{encoded}.extra"),
            "non-ascii-\u{00e9}".into(),
        ] {
            assert!(matches!(
                PairingCode::parse(&bad, UNIX_EPOCH),
                Err(PairingError::InvalidCode)
            ));
        }
        assert!(matches!(
            PairingCode::parse(&encoded, UNIX_EPOCH + PAIRING_TTL),
            Err(PairingError::Expired)
        ));
    }

    #[test]
    fn successful_pairing_is_one_use_and_never_transmits_the_capability() {
        let (code, mut gate) = prepare(offer());
        let hello = HandshakeHello::generate().unwrap();
        let proof = gate.host_proof(&hello).unwrap();
        let request = code
            .handshake_request(&hello, &proof, compatibility())
            .unwrap();
        let secret = hex::encode(&code.capability.0);
        for wire in [
            serde_json::to_string(&hello).unwrap(),
            serde_json::to_string(&proof).unwrap(),
            serde_json::to_string(&request).unwrap(),
            format!("{code:?} {gate:?} {proof:?} {request:?}"),
        ] {
            assert!(!wire.contains(&secret));
        }
        assert!(!format!("{request:?}").contains(&request.proof));
        assert!(!format!("{proof:?}").contains(&proof.proof));
        assert!(
            matches!(gate.verify(&request, UNIX_EPOCH), PairingVerdict::Accepted { session_id } if session_id == proof.session_id)
        );
        assert_eq!(gate.verify(&request, UNIX_EPOCH), PairingVerdict::Revoked);
        assert!(gate.host_proof(&hello).is_err());
    }

    #[test]
    fn proofs_bind_fresh_challenges_roles_endpoint_and_session_metadata() {
        let (code, mut gate) = prepare(offer());
        let hello = HandshakeHello::generate().unwrap();
        let proof = gate.host_proof(&hello).unwrap();
        assert!(code.verifies_host_proof(&hello, &proof));
        let fresh = HandshakeHello::generate().unwrap();
        assert!(!code.verifies_host_proof(&fresh, &proof));
        let (other_code, _) = prepare(offer());
        assert!(!other_code.verifies_host_proof(&hello, &proof));
        let mut changed_code = code.clone();
        changed_code.port = NonZeroU16::new(31_338).unwrap();
        assert!(!changed_code.verifies_host_proof(&hello, &proof));
        changed_code = code.clone();
        changed_code.expires_at_unix_ms += 1;
        assert!(!changed_code.verifies_host_proof(&hello, &proof));
        for change in 0..7 {
            let mut bad = proof.clone();
            match change {
                0 => bad.wire_version += 1,
                1 => bad.server_nonce = "00".repeat(32),
                2 => bad.session_id = SessionId::generate().unwrap(),
                3 => bad.consent_epoch += 1,
                4 => bad.compatibility.replay_abi_version += 1,
                5 => bad.proof = "not-hex".into(),
                _ => bad.proof = hex::encode(&code.transcript_proof(b"guest", &proof).unwrap()),
            }
            assert!(!code.verifies_host_proof(&hello, &bad));
            assert!(
                code.handshake_request(&hello, &bad, compatibility())
                    .is_err()
            );
        }
        let mut bad_hello = hello;
        bad_hello.nonce = "not-hex".into();
        assert!(gate.host_proof(&bad_hello).is_err());
    }

    #[test]
    fn recorded_guest_proof_fails_after_a_new_connection_challenge() {
        let (code, mut gate) = prepare(offer());
        let hello = HandshakeHello::generate().unwrap();
        let first = gate.host_proof(&hello).unwrap();
        let old = code
            .handshake_request(&hello, &first, compatibility())
            .unwrap();
        let second = gate.host_proof(&hello).unwrap();
        assert_ne!(first.server_nonce, second.server_nonce);
        assert!(matches!(
            gate.verify(&old, UNIX_EPOCH),
            PairingVerdict::Rejected { .. }
        ));
        let good = request(&code, &mut gate);
        assert!(matches!(
            gate.verify(&good, UNIX_EPOCH),
            PairingVerdict::Accepted { .. }
        ));
    }

    #[test]
    fn eight_complete_invalid_proofs_revoke_the_offer() {
        let (code, mut gate) = prepare(offer());
        for index in 1..=MAX_HANDSHAKE_ATTEMPTS {
            let mut bad = request(&code, &mut gate);
            bad.proof = "00".repeat(32);
            let expected = if index == MAX_HANDSHAKE_ATTEMPTS {
                PairingVerdict::Revoked
            } else {
                PairingVerdict::Rejected {
                    attempts_remaining: MAX_HANDSHAKE_ATTEMPTS - index,
                }
            };
            assert_eq!(gate.verify(&bad, UNIX_EPOCH), expected);
        }
        assert!(gate.is_revoked());
    }

    #[test]
    fn semantic_mismatch_is_rejected_before_content() {
        let (code, mut gate) = prepare(offer());
        let different = Compatibility::from_catalogs(["life"], ["lorenz"], ["quiz"]).unwrap();
        let hello = HandshakeHello::generate().unwrap();
        let proof = gate.host_proof(&hello).unwrap();
        assert!(
            code.handshake_request(&hello, &proof, different.clone())
                .is_err()
        );
        let mut bad = code
            .handshake_request(&hello, &proof, compatibility())
            .unwrap();
        bad.compatibility = different;
        assert!(matches!(
            gate.verify(&bad, UNIX_EPOCH),
            PairingVerdict::Rejected { .. }
        ));
    }

    #[test]
    fn expiry_and_backward_clock_cannot_extend_a_gate() {
        let (code, mut gate) = prepare(offer());
        let request = request(&code, &mut gate);
        assert_eq!(
            gate.verify(&request, UNIX_EPOCH + PAIRING_TTL),
            PairingVerdict::Expired
        );
        let start = Instant::now();
        let offer = PairingOffer::generate_at(
            NonZeroU16::new(31_337).unwrap(),
            UNIX_EPOCH + Duration::from_secs(60),
            start,
        )
        .unwrap();
        let (code, mut gate) = prepare(offer);
        let request = self::request(&code, &mut gate);
        assert_eq!(
            gate.verify_at(&request, UNIX_EPOCH, start + PAIRING_TTL),
            PairingVerdict::Expired
        );
    }

    #[test]
    fn explicit_revocation_and_clock_failure_are_fail_closed() {
        let (code, mut gate) = prepare(offer());
        let request = request(&code, &mut gate);
        assert_eq!(
            gate.verify(&request, UNIX_EPOCH - Duration::from_millis(1)),
            PairingVerdict::Revoked
        );
        let (_, mut revoked) = prepare(offer());
        revoked.revoke();
        assert_eq!(
            revoked.verify(&request, UNIX_EPOCH),
            PairingVerdict::Revoked
        );
        for (error, text) in [
            (PairingError::InvalidCode, "invalid pairing code"),
            (PairingError::Expired, "pairing code expired"),
            (
                PairingError::ClockOutOfRange,
                "system clock is outside the supported range",
            ),
            (
                PairingError::RandomUnavailable,
                "operating-system randomness is unavailable",
            ),
        ] {
            assert_eq!(error.to_string(), text);
            assert!(error.source().is_none());
        }
    }
}
