//! Sunflowers among the handshakes of six guests.
//!
//! Guests are numbered 1 through 6. A board's hexadecimal code stores one bit
//! per edge in the Party board's lexicographic order, starting at the low bit.

/// The common rule behind the string readings.
pub const RULES: &str = "One string joins exactly two guests. Three strings make a sunflower when each pair shares the same set of guests.";

/// The open invitation, before a player asks for the proof.
pub const CHALLENGE: &str = "How many strings can you keep without making a flower?";

/// The finite challenge's optional reveal.
pub const EXPLANATION: &str = "Six strings can stay. Three at one guest make a flower, so each guest can hold at most two. With six strings, every guest must hold two. That leaves a six-guest ring or two triangles. Alternating strings on the ring give three disjoint pairs. Two separate triangles escape.";

/// A six-guest board. The low fifteen bits represent present edges.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Board(u16);

/// The three pairwise intersections of three distinct strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inspection {
    /// Selected edges, in inspection order, with the smaller endpoint first.
    pub edges: [[u8; 2]; 3],
    /// Common guest in each pair of edges, in order AB, AC, BC.
    pub intersections: [Vec<u8>; 3],
    /// The shared intersection for a sunflower. Disjoint strings give an empty core.
    pub core: Option<Vec<u8>>,
}

/// One board and the first sunflower in edge order, if one exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reading {
    /// Every present edge in ascending lexicographic order.
    pub edges: Vec<[u8; 2]>,
    /// A deterministic witness from this board.
    pub witness: Option<Inspection>,
}

fn normalized(mut edge: [u8; 2]) -> Result<[u8; 2], &'static str> {
    if !edge.iter().all(|v| (1..=6).contains(v)) {
        return Err("Guests must be numbered 1 through 6.");
    }
    edge.sort_unstable();
    if edge[0] == edge[1] {
        return Err("A string needs two different guests.");
    }
    Ok(edge)
}

// The caller has checked the endpoints.
fn bit([a, b]: [u8; 2]) -> u16 {
    let i = super::edge_index(6, usize::from(a - 1), usize::from(b - 1))
        .expect("ordered guests on the six-guest board");
    1 << i
}

fn intersection(a: [u8; 2], b: [u8; 2]) -> Option<u8> {
    a.into_iter().find(|guest| b.contains(guest))
}

fn intersections([a, b, c]: [[u8; 2]; 3]) -> [Option<u8>; 3] {
    [intersection(a, b), intersection(a, c), intersection(b, c)]
}

fn inspection(edges: [[u8; 2]; 3], common: [Option<u8>; 3]) -> Inspection {
    let [ab, ac, bc] = common;
    Inspection {
        edges,
        intersections: common.map(|guest| guest.into_iter().collect()),
        core: (ab == ac && ac == bc).then(|| ab.into_iter().collect()),
    }
}

impl Board {
    /// Restore an exact board from four hexadecimal digits.
    pub fn from_code(code: &str) -> Result<Self, &'static str> {
        if code.len() == 4
            && code.bytes().all(|c| c.is_ascii_hexdigit())
            && let Ok(mask @ 0..=0x7fff) = u16::from_str_radix(code, 16)
        {
            return Ok(Self(mask));
        }
        Err("Use four hexadecimal digits from 0000 to 7FFF.")
    }

    /// Construct a board from distinct strings, allowing either endpoint order.
    pub fn from_edges(edges: &[[u8; 2]]) -> Result<Self, &'static str> {
        if edges.len() > 15 {
            return Err("The board has at most fifteen strings.");
        }
        let mut board = Self::default();
        for edge in edges {
            let mask = bit(normalized(*edge)?);
            if board.0 & mask != 0 {
                return Err("Each string may appear only once.");
            }
            board.0 |= mask;
        }
        Ok(board)
    }

    /// A portable exact board code.
    #[must_use]
    pub fn code(self) -> String {
        format!("{:04X}", self.0)
    }

    /// Add a missing string or erase a present one.
    pub fn toggle(&mut self, edge: [u8; 2]) -> Result<(), &'static str> {
        self.0 ^= bit(normalized(edge)?);
        Ok(())
    }

    /// Inspect three distinct strings that are actually present.
    pub fn inspect(self, chosen: [[u8; 2]; 3]) -> Result<Inspection, &'static str> {
        let [a, b, c] = chosen.map(normalized);
        let edges = [a?, b?, c?];
        let selected = Self::from_edges(&edges)?;
        if self.0 & selected.0 != selected.0 {
            return Err("Select three strings already on the board.");
        }
        Ok(inspection(edges, intersections(edges)))
    }

    /// List every present string and locate a sunflower, when one exists.
    #[must_use]
    pub fn reading(self) -> Reading {
        let edges: Vec<_> = (1..6)
            .flat_map(|a| (a + 1..=6).map(move |b| [a, b]))
            .filter(|&edge| self.0 & bit(edge) != 0)
            .collect();
        for (i, &a) in edges.iter().enumerate() {
            for (j, &b) in edges.iter().enumerate().skip(i + 1) {
                for &c in &edges[j + 1..] {
                    let triple = [a, b, c];
                    let common = intersections(triple);
                    if common[0] == common[1] && common[1] == common[2] {
                        return Reading {
                            edges,
                            witness: Some(inspection(triple, common)),
                        };
                    }
                }
            }
        }
        Reading {
            edges,
            witness: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_miss_and_replay_preserve_intersections() {
        let triangle = [[1, 2], [1, 3], [2, 3]];
        let mut board = Board::from_edges(&triangle).unwrap();
        let near = board.inspect(triangle).unwrap();
        assert_eq!(near.intersections, [vec![1], vec![2], vec![3]]);
        assert_eq!(near.core, None);
        assert_eq!(board.code(), "0023");
        assert_eq!(Board::from_code("0023"), Ok(board));
        assert!(board.inspect([[1, 2], [2, 1], [2, 3]]).is_err());
        assert!(board.inspect([[1, 2], [2, 3], [3, 4]]).is_err());
        for invalid in [[2, 2], [7, 1], [0, 2]] {
            assert!(board.toggle(invalid).is_err());
        }
        assert!(Board::from_edges(&[[1, 2]; 16]).is_err());
        for code in ["", "8000", "FFFF", "xyz0", "12345", " 123"] {
            assert!(Board::from_code(code).is_err());
        }
        board.toggle([2, 1]).unwrap();
        assert_eq!(board.code(), "0022");
    }

    #[test]
    fn all_boards_agree_with_degrees_and_perfect_matchings() {
        let full: Vec<_> = (1..6)
            .flat_map(|a| (a + 1..=6).map(move |b| [a, b]))
            .collect();
        let mut matchings = Vec::new();
        for (i, a) in full.iter().enumerate() {
            for (j, b) in full.iter().enumerate().skip(i + 1) {
                for (k, c) in full.iter().enumerate().skip(j + 1) {
                    let mut guests = [a[0], a[1], b[0], b[1], c[0], c[1]];
                    guests.sort_unstable();
                    if guests == [1, 2, 3, 4, 5, 6] {
                        matchings.push((1 << i) | (1 << j) | (1 << k));
                    }
                }
            }
        }
        assert_eq!(matchings.len(), 15);
        let mut maximum = 0;
        let mut optima = 0;
        for mask in 0..=0x7fff_u16 {
            let board = Board(mask);
            let reading = board.reading();
            let expected: Vec<_> = full
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, edge)| *edge)
                .collect();
            assert_eq!(reading.edges, expected);
            let star =
                (1..=6).any(|v| reading.edges.iter().filter(|e| e.contains(&v)).count() >= 3);
            let matching = matchings.iter().any(|m| mask & m == *m);
            assert_eq!(reading.witness.is_some(), star || matching);
            if let Some(w) = reading.witness {
                assert_eq!(board.inspect(w.edges), Ok(w.clone()));
                let core = w.core.unwrap();
                if let Some(v) = core.first() {
                    assert!(w.edges.iter().all(|e| e.contains(v)));
                } else {
                    assert!(w.intersections.iter().all(Vec::is_empty));
                }
            } else {
                maximum = maximum.max(reading.edges.len());
                optima += usize::from(reading.edges.len() == 6);
            }
        }
        assert_eq!((maximum, optima), (6, 10));
    }
}
