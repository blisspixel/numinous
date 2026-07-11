//! The Party Problem: try to escape Ramsey's theorem. You cannot.
//!
//! Color every handshake between guests red or blue, trying to avoid any
//! one-color triangle. On five guests it can be done (the pentagon knows
//! how). On six it is impossible, R(3,3) = 6, and the game exists so you
//! can feel the walls close in. See `docs/PLAYFUL.md`.

/// An edge between guests `a < b`, colored or not yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shade {
    /// Not yet chosen.
    Open,
    /// Red.
    Red,
    /// Blue.
    Blue,
}

/// The party: `n` guests, all handshakes, each shaded or open.
#[derive(Debug, Clone)]
pub struct Party {
    /// Guest count.
    pub guests: usize,
    /// Edge shades, indexed by [`edge_index`].
    pub edges: Vec<Shade>,
}

/// The index of edge (a, b), a < b, in the flat edge list.
#[must_use]
pub fn edge_index(guests: usize, a: usize, b: usize) -> Option<usize> {
    if a >= b || b >= guests {
        return None;
    }
    // Edges sorted by (a, b): those before row a, plus b's offset.
    let before: usize = (0..a).map(|i| guests - 1 - i).sum();
    Some(before + (b - a - 1))
}

impl Party {
    /// A fresh party of `n` guests, all handshakes open.
    #[must_use]
    pub fn new(guests: usize) -> Self {
        Self {
            guests,
            // saturating_sub so `guests == 0` cannot underflow (a debug panic);
            // zero or one guest simply has no edges.
            edges: vec![Shade::Open; guests * guests.saturating_sub(1) / 2],
        }
    }

    /// Shade edge (a, b). False if out of range or already shaded.
    pub fn shade(&mut self, a: usize, b: usize, shade: Shade) -> bool {
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        let Some(i) = edge_index(self.guests, a, b) else {
            return false;
        };
        if self.edges[i] != Shade::Open || shade == Shade::Open {
            return false;
        }
        self.edges[i] = shade;
        true
    }

    /// The first one-color triangle, if any: (a, b, c, its shade).
    #[must_use]
    pub fn mono_triangle(&self) -> Option<(usize, usize, usize, Shade)> {
        for a in 0..self.guests {
            for b in (a + 1)..self.guests {
                for c in (b + 1)..self.guests {
                    let ab = self.edges[edge_index(self.guests, a, b)?];
                    let bc = self.edges[edge_index(self.guests, b, c)?];
                    let ac = self.edges[edge_index(self.guests, a, c)?];
                    if ab != Shade::Open && ab == bc && bc == ac {
                        return Some((a, b, c, ab));
                    }
                }
            }
        }
        None
    }

    /// How many handshakes have been shaded.
    #[must_use]
    pub fn shaded(&self) -> usize {
        self.edges.iter().filter(|&&e| e != Shade::Open).count()
    }

    /// Whether every handshake is shaded.
    #[must_use]
    pub fn complete(&self) -> bool {
        self.edges.iter().all(|&e| e != Shade::Open)
    }
}

/// The pentagon's escape: the known 2-coloring of five guests with no
/// one-color triangle (outer ring red, inner star blue).
#[must_use]
pub fn pentagon_escape() -> Party {
    let mut party = Party::new(5);
    for i in 0..5 {
        let ring = (i, (i + 1) % 5);
        let star = (i, (i + 2) % 5);
        let _ = party.shade(ring.0.min(ring.1), ring.0.max(ring.1), Shade::Red);
        let _ = party.shade(star.0.min(star.1), star.0.max(star.1), Shade::Blue);
    }
    party
}

#[cfg(test)]
mod tests {
    use super::{Party, Shade, edge_index, pentagon_escape};

    #[test]
    fn edge_indexing_is_a_bijection() {
        let mut seen = std::collections::BTreeSet::new();
        for a in 0..6 {
            for b in (a + 1)..6 {
                seen.insert(edge_index(6, a, b).expect("valid edge"));
            }
        }
        assert_eq!(seen.len(), 15, "K6 has fifteen distinct handshakes");
        assert_eq!(seen.iter().max(), Some(&14));
        assert!(edge_index(6, 3, 3).is_none());
        assert!(edge_index(6, 0, 6).is_none());
    }

    #[test]
    fn five_guests_can_escape_and_the_pentagon_proves_it() {
        let party = pentagon_escape();
        assert!(party.complete(), "all ten handshakes shaded");
        assert!(party.mono_triangle().is_none(), "and no one-color triangle");
    }

    #[test]
    fn six_guests_cannot_escape_ramsey() {
        // Exhaustive: every 2-coloring of K6 contains a mono triangle.
        // 2^15 colorings; the theorem, verified by brute force.
        for mask in 0u32..(1 << 15) {
            let mut party = Party::new(6);
            for (i, e) in party.edges.iter_mut().enumerate() {
                *e = if mask & (1 << i) != 0 {
                    Shade::Red
                } else {
                    Shade::Blue
                };
            }
            assert!(
                party.mono_triangle().is_some(),
                "coloring {mask:#x} escaped: publish immediately"
            );
        }
    }

    #[test]
    fn triangles_are_detected_and_shading_is_guarded() {
        let mut party = Party::new(4);
        assert!(party.shade(0, 1, Shade::Red));
        assert!(!party.shade(1, 0, Shade::Blue), "already shaded");
        assert!(!party.shade(0, 9, Shade::Red), "no such guest");
        assert!(party.shade(1, 2, Shade::Red));
        assert!(party.mono_triangle().is_none());
        assert!(party.shade(0, 2, Shade::Red));
        let (a, b, c, shade) = party.mono_triangle().expect("triangle");
        assert_eq!((a, b, c), (0, 1, 2));
        assert_eq!(shade, Shade::Red);
    }
}
