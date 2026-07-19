use crate::fingerprint::{Compatibility, WIRE_VERSION};
use crate::hex;
use crate::wire::{HandshakeRequest, SessionId};
use getrandom::fill;
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
const PREFIX: &str = "numinous1";

#[derive(Clone, Copy)]
struct Capability([u8; 16]);

impl Capability {
    fn generate() -> Result<Self, PairingError> {
        let mut bytes = [0; 16];
        fill(&mut bytes).map_err(|_| PairingError::RandomUnavailable)?;
        Ok(Self(bytes))
    }

    fn matches_hex(&self, candidate: &str) -> bool {
        let decoded = hex::decode::<16>(candidate);
        let bytes = decoded.unwrap_or([0; 16]);
        bool::from(self.0.ct_eq(&bytes)) && decoded.is_some()
    }
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

    /// Builds the bounded authentication request sent to the local listener.
    #[must_use]
    pub fn handshake_request(&self, compatibility: Compatibility) -> HandshakeRequest {
        HandshakeRequest {
            wire_version: WIRE_VERSION,
            capability: hex::encode(&self.capability.0),
            compatibility,
        }
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
}

impl PairingGate {
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
        let valid = request.wire_version == WIRE_VERSION
            && request
                .compatibility
                .is_compatible_with(&self.compatibility)
            && self.code.capability.matches_hex(&request.capability);
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
    use super::{
        MAX_HANDSHAKE_ATTEMPTS, MAX_PAIRING_CODE_BYTES, PAIRING_TTL, PairingCode, PairingError,
        PairingOffer, PairingVerdict,
    };
    use crate::Compatibility;
    use std::error::Error;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::num::NonZeroU16;
    use std::time::{Duration, Instant, UNIX_EPOCH};

    fn compatibility() -> Compatibility {
        Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"]).expect("valid compatibility")
    }

    fn offer() -> PairingOffer {
        PairingOffer::generate(NonZeroU16::new(31_337).expect("nonzero port"), UNIX_EPOCH)
            .expect("pairing offer")
    }

    #[test]
    fn generated_code_is_bounded_loopback_only_and_round_trips() {
        let offer = offer();
        let encoded = offer.display_code();
        assert!(encoded.len() <= MAX_PAIRING_CODE_BYTES);
        assert!(!format!("{offer:?}").contains(&encoded));
        let decoded = PairingCode::parse(&encoded, UNIX_EPOCH).expect("valid code");
        assert_eq!(
            decoded.endpoint(),
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 31_337)
        );
        assert_eq!(decoded.handshake_request(compatibility()).wire_version, 1);
    }

    #[test]
    fn malformed_oversized_zero_port_and_expired_codes_fail_closed() {
        assert!(matches!(
            PairingCode::parse(&"x".repeat(MAX_PAIRING_CODE_BYTES + 1), UNIX_EPOCH),
            Err(PairingError::InvalidCode)
        ));
        assert!(matches!(
            PairingCode::parse(
                "numinous1.0.300000.00000000000000000000000000000000",
                UNIX_EPOCH
            ),
            Err(PairingError::InvalidCode)
        ));
        assert!(matches!(
            PairingCode::parse(
                "numinous2.1.300000.00000000000000000000000000000000",
                UNIX_EPOCH
            ),
            Err(PairingError::InvalidCode)
        ));
        let code = offer().display_code();
        assert!(matches!(
            PairingCode::parse(&code, UNIX_EPOCH + PAIRING_TTL),
            Err(PairingError::Expired)
        ));
    }

    #[test]
    fn successful_handshake_is_one_use_and_debug_redacts_the_capability() {
        let offer = offer();
        let code = offer.display_code();
        let request = PairingCode::parse(&code, UNIX_EPOCH)
            .expect("valid code")
            .handshake_request(compatibility());
        assert!(!format!("{request:?}").contains(&request.capability));
        let mut gate = offer.into_gate(compatibility());
        assert!(matches!(
            gate.verify(&request, UNIX_EPOCH),
            PairingVerdict::Accepted { .. }
        ));
        assert_eq!(gate.verify(&request, UNIX_EPOCH), PairingVerdict::Revoked);
    }

    #[test]
    fn eight_failed_handshakes_revoke_the_offer() {
        let offer = offer();
        let mut request = PairingCode::parse(&offer.display_code(), UNIX_EPOCH)
            .expect("valid code")
            .handshake_request(compatibility());
        request.capability = "00".repeat(16);
        let mut gate = offer.into_gate(compatibility());
        for remaining in (1..MAX_HANDSHAKE_ATTEMPTS).rev() {
            assert_eq!(
                gate.verify(&request, UNIX_EPOCH),
                PairingVerdict::Rejected {
                    attempts_remaining: remaining
                }
            );
        }
        assert_eq!(gate.verify(&request, UNIX_EPOCH), PairingVerdict::Revoked);
        assert!(gate.is_revoked());
    }

    #[test]
    fn same_roster_semantic_mismatch_is_rejected_before_content() {
        let offer = offer();
        let code = PairingCode::parse(&offer.display_code(), UNIX_EPOCH).expect("valid code");
        let different = Compatibility::from_catalogs(["life"], ["lorenz"], ["quiz"])
            .expect("valid compatibility");
        let request = code.handshake_request(different);
        let mut gate = offer.into_gate(compatibility());
        assert_eq!(
            gate.verify(&request, UNIX_EPOCH),
            PairingVerdict::Rejected {
                attempts_remaining: MAX_HANDSHAKE_ATTEMPTS - 1
            }
        );
    }

    #[test]
    fn expiry_is_checked_again_at_the_host_gate() {
        let initial_offer = offer();
        let request = PairingCode::parse(&initial_offer.display_code(), UNIX_EPOCH)
            .expect("valid code")
            .handshake_request(compatibility());
        let mut gate = initial_offer.into_gate(compatibility());
        assert_eq!(
            gate.verify(
                &request,
                UNIX_EPOCH + PAIRING_TTL + Duration::from_millis(1)
            ),
            PairingVerdict::Expired
        );
    }

    #[test]
    fn backward_wall_clock_cannot_extend_the_host_deadline() {
        let monotonic_start = Instant::now();
        let offer = PairingOffer::generate_at(
            NonZeroU16::new(31_337).expect("nonzero port"),
            UNIX_EPOCH + Duration::from_secs(60),
            monotonic_start,
        )
        .expect("pairing offer");
        let request =
            PairingCode::parse(&offer.display_code(), UNIX_EPOCH + Duration::from_secs(60))
                .expect("valid code")
                .handshake_request(compatibility());
        let mut gate = offer.into_gate(compatibility());
        assert_eq!(
            gate.verify_at(
                &request,
                UNIX_EPOCH,
                monotonic_start + PAIRING_TTL + Duration::from_millis(1)
            ),
            PairingVerdict::Expired
        );
    }

    #[test]
    fn explicit_revocation_clock_failure_and_debug_are_fail_closed() {
        let initial_offer = offer();
        let request = PairingCode::parse(&initial_offer.display_code(), UNIX_EPOCH)
            .expect("valid code")
            .handshake_request(compatibility());
        let mut gate = initial_offer.into_gate(compatibility());
        assert!(format!("{gate:?}").contains("PairingGate"));
        assert!(!format!("{gate:?}").contains(&request.capability));
        assert_eq!(
            gate.verify(&request, UNIX_EPOCH - Duration::from_millis(1)),
            PairingVerdict::Revoked
        );
        assert!(gate.is_revoked());

        let mut revoked = offer().into_gate(compatibility());
        revoked.revoke();
        assert!(revoked.is_revoked());
        assert_eq!(
            revoked.verify(&request, UNIX_EPOCH),
            PairingVerdict::Revoked
        );
    }

    #[test]
    fn extra_code_parts_and_public_error_categories_are_rejected() {
        let code = format!("{}.extra", offer().display_code());
        assert!(matches!(
            PairingCode::parse(&code, UNIX_EPOCH),
            Err(PairingError::InvalidCode)
        ));
        let errors = [
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
        ];
        for (error, expected) in errors {
            assert_eq!(error.to_string(), expected);
            assert!(error.source().is_none());
        }
    }
}
