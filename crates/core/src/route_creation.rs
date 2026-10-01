//! Portable authored street networks, independent of editor undo and playback.
//!
//! Canonical content identifies a creation and an optional parent. The digest
//! detects inconsistent bytes; it does not attest authorship or custody.

use std::fmt;

use crate::route::{MAX_ROUTE_ROADS, MAX_ROUTE_STOPS, Road};
use crate::route_workbench::{
    EditableRoad, RouteTownSnapshot, RouteWorkbench, RouteWorkbenchError, RouteWorkbenchSnapshot,
};
use crate::sha256::{digest, hex};

/// Versioned portable route creation header.
pub const ROUTE_CAPSULE_HEADER: &str = "NUMINOUS_ROUTE 1";
/// Maximum bytes accepted in a portable route creation.
pub const MAX_ROUTE_CAPSULE_BYTES: usize = 8 * 1024;

/// Why a portable route creation could not be admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteCreationError {
    /// Input exceeds the portable byte limit.
    TooLarge,
    /// The version or field structure is not supported.
    InvalidFormat(&'static str),
    /// The authored network or delivery order is structurally invalid.
    InvalidNetwork(RouteWorkbenchError),
    /// The declared identity does not match canonical authored content.
    Identity,
}

impl fmt::Display for RouteCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(f, "route creation exceeds {MAX_ROUTE_CAPSULE_BYTES} bytes"),
            Self::InvalidFormat(reason) => write!(f, "invalid route creation: {reason}"),
            Self::InvalidNetwork(error) => write!(f, "invalid route creation: {error}"),
            Self::Identity => write!(f, "route creation identity does not match its content"),
        }
    }
}

impl std::error::Error for RouteCreationError {}

/// An immutable authored network, delivery order, and optional parent identity.
///
/// Infeasible networks remain portable. Derived routes, local revision ids,
/// undo entries, and recorded searches are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCreation {
    town: RouteTownSnapshot,
    parent: Option<[u8; 32]>,
}

impl RouteCreation {
    /// Validate a new authored network without assigning a parent.
    pub fn new(town: RouteTownSnapshot) -> Result<Self, RouteCreationError> {
        Self::with_parent(town, None)
    }

    fn with_parent(
        town: RouteTownSnapshot,
        parent: Option<[u8; 32]>,
    ) -> Result<Self, RouteCreationError> {
        let workbench = RouteWorkbench::from_snapshot(RouteWorkbenchSnapshot {
            revision: 0,
            current: town,
            undo: Vec::new(),
            trace: None,
        })
        .map_err(RouteCreationError::InvalidNetwork)?;
        let creation = Self {
            town: workbench.town().clone(),
            parent,
        };
        if creation.to_capsule().len() > MAX_ROUTE_CAPSULE_BYTES {
            return Err(RouteCreationError::TooLarge);
        }
        Ok(creation)
    }

    /// Read bounded portable text and verify its canonical content identity.
    ///
    /// This accepts data only. A path, identifier, or URI does not read a file.
    pub fn from_capsule(text: &str) -> Result<Self, RouteCreationError> {
        if text.len() > MAX_ROUTE_CAPSULE_BYTES {
            return Err(RouteCreationError::TooLarge);
        }
        let mut lines = text.lines();
        if lines.next() != Some(ROUTE_CAPSULE_HEADER) {
            return Err(RouteCreationError::InvalidFormat("unsupported header"));
        }
        let junctions = scalar(lines.next(), "junctions")?;
        let count = scalar(lines.next(), "roads")?;
        if count > MAX_ROUTE_ROADS {
            return Err(RouteCreationError::InvalidFormat("too many roads"));
        }
        let mut roads = Vec::with_capacity(count);
        for _ in 0..count {
            let fields: Vec<_> = lines.next().unwrap_or_default().split(' ').collect();
            if fields.len() != 5 || fields[0] != "road" {
                return Err(RouteCreationError::InvalidFormat("expected road fields"));
            }
            roads.push(EditableRoad {
                road: Road {
                    from: number(fields[1])?,
                    to: number(fields[2])?,
                    cost: u32::try_from(number(fields[3])?)
                        .map_err(|_| RouteCreationError::InvalidFormat("invalid road cost"))?,
                },
                open: match fields[4] {
                    "open" => true,
                    "closed" => false,
                    _ => {
                        return Err(RouteCreationError::InvalidFormat(
                            "invalid road availability",
                        ));
                    }
                },
            });
        }
        let stops = indices(lines.next(), "stops")?;
        let order = indices(lines.next(), "order")?;
        let parent = field(lines.next(), "parent")?;
        let parent = if parent == "-" {
            None
        } else {
            Some(parse_digest(parent)?)
        };
        let identity = parse_digest(field(lines.next(), "identity")?)?;
        if lines.next().is_some() {
            return Err(RouteCreationError::InvalidFormat(
                "unexpected trailing fields",
            ));
        }
        let creation = Self::with_parent(
            RouteTownSnapshot {
                junctions,
                roads,
                stops,
                order,
            },
            parent,
        )?;
        if creation.identity() != identity {
            return Err(RouteCreationError::Identity);
        }
        Ok(creation)
    }

    /// Canonical portable text, including the verified content identity.
    #[must_use]
    pub fn to_capsule(&self) -> String {
        let body = self.content();
        format!("{body}identity {}\n", hex(&digest(body.as_bytes())))
    }

    /// SHA-256 of versioned canonical authored content and parent identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        digest(self.content().as_bytes())
    }

    /// Lowercase hexadecimal content identity.
    #[must_use]
    pub fn identity_hex(&self) -> String {
        hex(&self.identity())
    }

    /// The declared parent creation identity, when explicitly remixed.
    #[must_use]
    pub const fn parent_identity(&self) -> Option<[u8; 32]> {
        self.parent
    }

    /// Canonical authored network and delivery order.
    #[must_use]
    pub const fn town(&self) -> &RouteTownSnapshot {
        &self.town
    }

    /// Make a child with this creation's identity as parent, leaving it intact.
    pub fn remix(&self, town: RouteTownSnapshot) -> Result<Self, RouteCreationError> {
        Self::with_parent(town, Some(self.identity()))
    }

    /// Update authored content while preserving this creation's existing parent.
    ///
    /// Ordinary editing does not create another lineage generation. Use
    /// [`Self::remix`] only when explicitly starting a child creation.
    pub fn with_network(&self, town: RouteTownSnapshot) -> Result<Self, RouteCreationError> {
        Self::with_parent(town, self.parent)
    }

    /// Reopen with fresh revision zero, empty undo, and no search playback.
    #[must_use]
    pub fn open(&self) -> RouteWorkbench {
        RouteWorkbench::from_snapshot(RouteWorkbenchSnapshot {
            revision: 0,
            current: self.town.clone(),
            undo: Vec::new(),
            trace: None,
        })
        .expect("route creation was structurally validated")
    }

    fn content(&self) -> String {
        let mut text = format!(
            "{ROUTE_CAPSULE_HEADER}\njunctions {}\nroads {}\n",
            self.town.junctions,
            self.town.roads.len()
        );
        for editable in &self.town.roads {
            let road = editable.road;
            text.push_str(&format!(
                "road {} {} {} {}\n",
                road.from,
                road.to,
                road.cost,
                if editable.open { "open" } else { "closed" }
            ));
        }
        for (key, values) in [("stops", &self.town.stops), ("order", &self.town.order)] {
            text.push_str(key);
            for value in values {
                text.push_str(&format!(" {value}"));
            }
            text.push('\n');
        }
        text.push_str(&format!(
            "parent {}\n",
            self.parent
                .map_or_else(|| "-".into(), |parent| hex(&parent))
        ));
        text
    }
}

fn field<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str, RouteCreationError> {
    let (name, value) = line
        .unwrap_or_default()
        .split_once(' ')
        .ok_or(RouteCreationError::InvalidFormat("missing field"))?;
    if name != key || value.is_empty() {
        return Err(RouteCreationError::InvalidFormat("unexpected field"));
    }
    Ok(value)
}

fn number(value: &str) -> Result<usize, RouteCreationError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RouteCreationError::InvalidFormat(
            "expected unsigned integer",
        ));
    }
    value
        .parse()
        .map_err(|_| RouteCreationError::InvalidFormat("integer exceeds its bound"))
}

fn scalar(line: Option<&str>, key: &str) -> Result<usize, RouteCreationError> {
    number(field(line, key)?)
}

fn indices(line: Option<&str>, key: &str) -> Result<Vec<usize>, RouteCreationError> {
    let mut values = Vec::new();
    for value in field(line, key)?.split(' ') {
        if values.len() == MAX_ROUTE_STOPS {
            return Err(RouteCreationError::InvalidFormat("too many stops"));
        }
        values.push(number(value)?);
    }
    Ok(values)
}

fn parse_digest(value: &str) -> Result<[u8; 32], RouteCreationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RouteCreationError::InvalidFormat(
            "expected lowercase content digest",
        ));
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| RouteCreationError::InvalidFormat("invalid content digest"))?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::RouteError;
    use crate::route_workbench::RouteEdit;

    #[test]
    fn portable_roundtrip_preserves_authored_state_and_discards_session() {
        let mut workbench = RouteWorkbench::first_town();
        workbench
            .apply(RouteEdit::RoadCost {
                from: 1,
                to: 3,
                cost: 7,
            })
            .unwrap();
        workbench.apply(RouteEdit::Order(vec![0, 3, 2, 1])).unwrap();
        workbench.start_trace(0, 3).unwrap();
        workbench.seek_trace(2).unwrap();
        let creation = RouteCreation::new(workbench.town().clone()).unwrap();
        let restored = RouteCreation::from_capsule(&creation.to_capsule()).unwrap();
        assert_eq!(creation, restored);
        let snapshot = restored.open().snapshot();
        assert_eq!(snapshot.current, workbench.snapshot().current);
        assert_eq!(snapshot.revision, 0);
        assert!(snapshot.undo.is_empty());
        assert!(snapshot.trace.is_none());
        assert_eq!(restored.open().compare(), workbench.compare());
    }

    #[test]
    fn remix_binds_parent_without_changing_it_and_canonicalizes_network() {
        let parent = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let original = parent.to_capsule();
        let mut town = parent.town().clone();
        town.roads.reverse();
        for road in &mut town.roads {
            std::mem::swap(&mut road.road.from, &mut road.road.to);
        }
        town.stops.swap(1, 3);
        assert_eq!(RouteCreation::new(town.clone()).unwrap(), parent);
        town.roads
            .iter_mut()
            .find(|road| road.road.from == 3 && road.road.to == 1)
            .unwrap()
            .road
            .cost = 5;
        let child = parent.remix(town).unwrap();
        assert_eq!(child.parent_identity(), Some(parent.identity()));
        assert_ne!(child.identity(), parent.identity());
        assert_eq!(parent.to_capsule(), original);
        assert_eq!(
            RouteCreation::from_capsule(&child.to_capsule()).unwrap(),
            child
        );
    }

    #[test]
    fn ordinary_edits_preserve_parent_and_only_explicit_remix_adds_generation() {
        let original = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let child = original.remix(original.town().clone()).unwrap();
        let mut network = child.town().clone();
        network.roads[3].road.cost = 5;
        let edited = child.with_network(network).unwrap();
        assert_eq!(edited.parent_identity(), Some(original.identity()));
        assert_ne!(edited.identity(), child.identity());
        let reopened = RouteCreation::from_capsule(&edited.to_capsule()).unwrap();
        assert_eq!(reopened.parent_identity(), Some(original.identity()));
        let grandchild = edited.remix(edited.town().clone()).unwrap();
        assert_eq!(grandchild.parent_identity(), Some(edited.identity()));
        assert_eq!(child.parent_identity(), Some(original.identity()));
        assert!(original.parent_identity().is_none());
    }

    #[test]
    fn infeasible_networks_and_closed_costs_remain_portable() {
        let mut town = RouteWorkbench::first_town().town().clone();
        for road in &mut town.roads {
            road.open = false;
        }
        town.roads[3].road.cost = 999;
        let creation = RouteCreation::new(town.clone()).unwrap();
        let reopened = RouteCreation::from_capsule(&creation.to_capsule())
            .unwrap()
            .open();
        assert_eq!(reopened.town(), &town);
        assert_eq!(
            reopened.compare(),
            Err(RouteError::Unreachable { from: 0, to: 1 })
        );
        town.stops = vec![0];
        town.order = vec![0];
        assert_eq!(
            RouteCreation::new(town).unwrap().open().compare(),
            Err(RouteError::Size)
        );
    }

    #[test]
    fn tampering_unknown_fields_and_oversized_inputs_fail_closed() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let text = creation.to_capsule();
        assert_eq!(
            RouteCreation::from_capsule(&text.replace("road 1 3 3", "road 1 3 5")),
            Err(RouteCreationError::Identity)
        );
        assert!(
            RouteCreation::from_capsule(&text.replace("NUMINOUS_ROUTE 1", "NUMINOUS_ROUTE 2"))
                .is_err()
        );
        assert!(RouteCreation::from_capsule(&(text.clone() + "extra value\n")).is_err());
        assert!(RouteCreation::from_capsule(&text.replace("roads 5", "roads 97")).is_err());
        assert!(RouteCreation::from_capsule("C:\\private.route").is_err());
        let oversized =
            RouteCreation::from_capsule(&"x".repeat(MAX_ROUTE_CAPSULE_BYTES + 1)).unwrap_err();
        assert_eq!(oversized, RouteCreationError::TooLarge);
        assert_eq!(oversized.to_string(), "route creation exceeds 8192 bytes");
        let mut malformed = creation.town().clone();
        malformed.order = vec![0, 1, 1, 3];
        assert!(RouteCreation::new(malformed).is_err());
    }

    #[test]
    fn largest_admitted_authored_network_fits_portable_cap() {
        let mut roads = Vec::new();
        'outer: for from in 0..32 {
            for to in from + 1..32 {
                roads.push(EditableRoad {
                    road: Road {
                        from,
                        to,
                        cost: 999,
                    },
                    open: false,
                });
                if roads.len() == MAX_ROUTE_ROADS {
                    break 'outer;
                }
            }
        }
        let town = RouteTownSnapshot {
            junctions: 32,
            roads,
            stops: (22..32).collect(),
            order: (22..32).collect(),
        };
        let parent = RouteCreation::new(town.clone()).unwrap();
        let child = parent.remix(town).unwrap();
        let text = child.to_capsule();
        assert!(text.len() < MAX_ROUTE_CAPSULE_BYTES);
        assert_eq!(RouteCreation::from_capsule(&text).unwrap(), child);
    }

    #[test]
    fn malformed_portable_fields_are_refused_before_a_creation_can_open() {
        let original = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let capsule = original.to_capsule();
        for malformed in [
            "NUMINOUS_ROUTE 1\n".to_string(),
            capsule.replace("junctions 4", "junctions"),
            capsule.replace("junctions 4", "junctions "),
            capsule.replace("junctions 4", "vertices 4"),
            capsule.replace("junctions 4", "junctions -4"),
            capsule.replace("junctions 4", "junctions 18446744073709551616"),
            capsule.replace("roads 5", "roads 6"),
            capsule.replace("road 0 1 1 open", "road 0 1 open"),
            capsule.replace("road 0 1 1 open", "road 0 1 4294967296 open"),
            capsule.replace("road 0 1 1 open", "road 0 1 1 maybe"),
            capsule.replace("stops 0 1 2 3", "stops 0 1 2 3 4 5 6 7 8 9 10"),
            capsule.replace("parent -", "parent invalid"),
            capsule.replace(&original.identity_hex(), &"G".repeat(64)),
        ] {
            let error = RouteCreation::from_capsule(&malformed).unwrap_err();
            assert!(matches!(error, RouteCreationError::InvalidFormat(_)));
            assert!(error.to_string().starts_with("invalid route creation:"));
        }
        assert_eq!(original.to_capsule(), capsule);
        assert_eq!(original.open().compare().unwrap().current.cost, 9);
    }

    #[test]
    fn failed_authored_edits_leave_source_and_lineage_available_for_repair() {
        let parent = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let child = parent.remix(parent.town().clone()).unwrap();
        let before = child.to_capsule();
        for invalid in [
            {
                let mut town = child.town().clone();
                town.roads.push(town.roads[0]);
                town
            },
            {
                let mut town = child.town().clone();
                town.roads[0].road.cost = 0;
                town
            },
            {
                let mut town = child.town().clone();
                town.order = vec![1, 0, 2, 3];
                town
            },
        ] {
            for refused in [child.with_network(invalid.clone()), child.remix(invalid)] {
                let error = refused.unwrap_err();
                assert!(matches!(error, RouteCreationError::InvalidNetwork(_)));
                assert!(error.to_string().starts_with("invalid route creation:"));
            }
            assert_eq!(child.to_capsule(), before);
        }
        let mut repaired = child.town().clone();
        repaired.roads[3].open = false;
        let edited = child.with_network(repaired).unwrap();
        assert_eq!(edited.parent_identity(), Some(parent.identity()));
        assert_eq!(child.open().compare().unwrap().current.cost, 9);
        assert_eq!(edited.open().compare().unwrap().current.cost, 9);
    }
}
