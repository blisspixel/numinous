use crate::hex;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

include!(concat!(env!("OUT_DIR"), "/build_semantic_id.rs"));

#[cfg(test)]
#[path = "../build_support.rs"]
mod build_support;

/// Broadcast wire protocol version.
pub const WIRE_VERSION: u16 = 1;
/// Deterministic core replay ABI version.
pub const REPLAY_ABI_VERSION: u16 = 1;
const MAX_CATALOG_IDENTITIES: usize = 1_024;
const MAX_IDENTITY_BYTES: usize = 128;

/// A SHA-256 identity for the wire schema, replay ABI, catalog, and build.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct CompatibilityFingerprint([u8; 32]);

impl CompatibilityFingerprint {
    /// Returns the raw fingerprint bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CompatibilityFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("CompatibilityFingerprint")
            .field(&hex::encode(&self.0))
            .finish()
    }
}

impl fmt::Display for CompatibilityFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex::encode(&self.0))
    }
}

impl Serialize for CompatibilityFingerprint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for CompatibilityFingerprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        hex::decode(&value)
            .map(Self)
            .ok_or_else(|| de::Error::custom("invalid compatibility fingerprint"))
    }
}

/// The exact compatibility declaration exchanged before public content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Compatibility {
    /// Wire schema version.
    pub wire_version: u16,
    /// Deterministic replay ABI version.
    pub replay_abi_version: u16,
    /// Digest covering replay semantics and the visible catalog.
    pub fingerprint: CompatibilityFingerprint,
}

impl Compatibility {
    /// Builds the compatibility identity from the three public rosters.
    ///
    /// Identities must be unique, nonempty ASCII strings no longer than 128
    /// bytes. At most 1,024 identities may appear in each roster.
    pub fn from_catalogs<R, S, G, RI, SI, GI>(
        rooms: R,
        simulations: S,
        games: G,
    ) -> Result<Self, CompatibilityError>
    where
        R: IntoIterator<Item = RI>,
        S: IntoIterator<Item = SI>,
        G: IntoIterator<Item = GI>,
        RI: AsRef<str>,
        SI: AsRef<str>,
        GI: AsRef<str>,
    {
        Self::from_catalogs_and_build(rooms, simulations, games, BUILD_SEMANTIC_ID)
    }

    fn from_catalogs_and_build<R, S, G, RI, SI, GI>(
        rooms: R,
        simulations: S,
        games: G,
        build_semantic_id: [u8; 32],
    ) -> Result<Self, CompatibilityError>
    where
        R: IntoIterator<Item = RI>,
        S: IntoIterator<Item = SI>,
        G: IntoIterator<Item = GI>,
        RI: AsRef<str>,
        SI: AsRef<str>,
        GI: AsRef<str>,
    {
        let rooms = normalized("room", rooms)?;
        let simulations = normalized("simulation", simulations)?;
        let games = normalized("game", games)?;
        let mut digest = Sha256::new();
        digest.update(b"numinous-broadcast-compatibility-v1\0");
        digest.update(WIRE_VERSION.to_le_bytes());
        digest.update(REPLAY_ABI_VERSION.to_le_bytes());
        digest.update(build_semantic_id);
        update_group(&mut digest, b"rooms", &rooms);
        update_group(&mut digest, b"simulations", &simulations);
        update_group(&mut digest, b"games", &games);
        Ok(Self {
            wire_version: WIRE_VERSION,
            replay_abi_version: REPLAY_ABI_VERSION,
            fingerprint: CompatibilityFingerprint(digest.finalize().into()),
        })
    }

    /// Returns whether both peers implement the same replay contract.
    #[must_use]
    pub fn is_compatible_with(&self, peer: &Self) -> bool {
        self == peer
    }
}

fn normalized<I, T>(kind: &'static str, values: I) -> Result<Vec<String>, CompatibilityError>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let mut output = Vec::new();
    for value in values {
        if output.len() == MAX_CATALOG_IDENTITIES {
            return Err(CompatibilityError::TooMany { kind });
        }
        let value = value.as_ref();
        if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || !value.is_ascii() {
            return Err(CompatibilityError::InvalidIdentity { kind });
        }
        output.push(value.to_owned());
    }
    output.sort_unstable();
    if output.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CompatibilityError::DuplicateIdentity { kind });
    }
    Ok(output)
}

fn update_group(digest: &mut Sha256, label: &[u8], identities: &[String]) {
    digest.update((label.len() as u64).to_le_bytes());
    digest.update(label);
    digest.update((identities.len() as u64).to_le_bytes());
    for identity in identities {
        digest.update((identity.len() as u64).to_le_bytes());
        digest.update(identity.as_bytes());
    }
}

/// Failure to construct a canonical compatibility fingerprint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityError {
    /// A roster contains more than 1,024 identities.
    TooMany {
        /// Roster kind.
        kind: &'static str,
    },
    /// An identity is empty, non-ASCII, or longer than 128 bytes.
    InvalidIdentity {
        /// Roster kind.
        kind: &'static str,
    },
    /// A roster contains the same identity more than once.
    DuplicateIdentity {
        /// Roster kind.
        kind: &'static str,
    },
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooMany { kind } => write!(formatter, "too many {kind} identities"),
            Self::InvalidIdentity { kind } => write!(formatter, "invalid {kind} identity"),
            Self::DuplicateIdentity { kind } => write!(formatter, "duplicate {kind} identity"),
        }
    }
}

impl Error for CompatibilityError {}

#[cfg(test)]
mod tests {
    use super::{Compatibility, CompatibilityError};
    use std::error::Error;

    #[test]
    fn catalog_order_does_not_change_the_fingerprint() {
        let first = Compatibility::from_catalogs(["life", "times"], ["lorenz"], ["munch"])
            .expect("valid catalogs");
        let second = Compatibility::from_catalogs(["times", "life"], ["lorenz"], ["munch"])
            .expect("valid catalogs");
        assert_eq!(first, second);
    }

    #[test]
    fn same_roster_semantic_mismatch_is_rejected() {
        let first =
            Compatibility::from_catalogs_and_build(["life"], ["lorenz"], ["munch"], [1; 32])
                .expect("valid catalogs");
        let second =
            Compatibility::from_catalogs_and_build(["life"], ["lorenz"], ["munch"], [2; 32])
                .expect("valid catalogs");
        assert!(!first.is_compatible_with(&second));
    }

    #[test]
    fn roster_validation_is_bounded_and_unambiguous() {
        assert_eq!(
            Compatibility::from_catalogs(["life", "life"], ["lorenz"], ["munch"]),
            Err(CompatibilityError::DuplicateIdentity { kind: "room" })
        );
        assert_eq!(
            Compatibility::from_catalogs([""], ["lorenz"], ["munch"]),
            Err(CompatibilityError::InvalidIdentity { kind: "room" })
        );
        let too_many = (0..=1_024).map(|index| format!("room-{index}"));
        assert_eq!(
            Compatibility::from_catalogs(too_many, ["lorenz"], ["munch"]),
            Err(CompatibilityError::TooMany { kind: "room" })
        );
    }

    #[test]
    fn fingerprint_json_is_canonical_and_strict() {
        let compatibility =
            Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"]).expect("valid catalogs");
        let json = serde_json::to_string(&compatibility).expect("serialize compatibility");
        let round_trip = serde_json::from_str(&json).expect("deserialize compatibility");
        assert_eq!(compatibility, round_trip);
        let canonical = compatibility.fingerprint.to_string();
        let uppercase = json.replace(&canonical, &canonical.to_ascii_uppercase());
        assert!(serde_json::from_str::<Compatibility>(&uppercase).is_err());
        let owned = serde_json::to_value(compatibility.fingerprint).expect("owned fingerprint");
        let decoded = serde_json::from_value(owned).expect("decode owned fingerprint");
        assert_eq!(compatibility.fingerprint, decoded);
        assert_eq!(compatibility.fingerprint.as_bytes().len(), 32);
        assert!(format!("{:?}", compatibility.fingerprint).contains("CompatibilityFingerprint"));
    }

    #[test]
    fn compatibility_errors_are_specific_and_source_free() {
        let errors = [
            (
                CompatibilityError::TooMany { kind: "room" },
                "too many room identities",
            ),
            (
                CompatibilityError::InvalidIdentity { kind: "game" },
                "invalid game identity",
            ),
            (
                CompatibilityError::DuplicateIdentity { kind: "simulation" },
                "duplicate simulation identity",
            ),
        ];
        for (error, expected) in errors {
            assert_eq!(error.to_string(), expected);
            assert!(error.source().is_none());
        }
    }
}
