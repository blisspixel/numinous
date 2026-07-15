//! The First Rain: percolation's cliff at p about 0.5927.
//!
//! Each site on a square lattice is open with probability p. Clusters of open
//! sites merge; at a sharp threshold a spanning cluster first touches top and
//! bottom. That is the rain finding a path through the soil. `t` sets p; DRAG
//! dials the rain harder under the hand. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Lattice side (fixed simulation grid).
const N: usize = 48;
/// Classic site-percolation threshold on the square lattice (approx).
const P_CRIT: f64 = 0.592746;
/// Base seed for the rain field.
const FIELD_SEED: u64 = 0xFA14_5EED_0000_0001;
/// Salt for nonzero variation field remix.
const VARIATION_SALT: u64 = 0xFA14_5EED_0000_0002;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

/// Ambient occupancy probability from phase.
fn ambient_p(t: f64) -> f64 {
    // Sweep from dry to flooded, centered near criticality at mid-phase.
    0.35 + phase_unit(t) * 0.45
}

/// Hand x dials rain intensity.
fn p_from_hand(x: f64) -> f64 {
    0.20 + x.clamp(0.0, 1.0) * 0.70
}

fn open_field(p: f64, seed: u64) -> Vec<bool> {
    let mut rng = SplitMix64::new(FIELD_SEED ^ seed ^ VARIATION_SALT);
    let p = p.clamp(0.0, 1.0);
    (0..N * N).map(|_| rng.next_f64() < p).collect()
}

/// Union-find spanning check: does any top-row open site connect to bottom?
fn spans(open: &[bool]) -> (bool, usize, usize) {
    let mut parent: Vec<usize> = (0..N * N + 2).collect();
    let mut size = vec![1usize; N * N + 2];
    let top = N * N;
    let bottom = N * N + 1;

    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    fn union(parent: &mut [usize], size: &mut [usize], a: usize, b: usize) {
        let mut ra = find(parent, a);
        let mut rb = find(parent, b);
        if ra == rb {
            return;
        }
        if size[ra] < size[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        parent[rb] = ra;
        size[ra] += size[rb];
    }

    for y in 0..N {
        for x in 0..N {
            let i = y * N + x;
            if !open[i] {
                continue;
            }
            if y == 0 {
                union(&mut parent, &mut size, i, top);
            }
            if y + 1 == N {
                union(&mut parent, &mut size, i, bottom);
            }
            if x + 1 < N && open[i + 1] {
                union(&mut parent, &mut size, i, i + 1);
            }
            if y + 1 < N && open[i + N] {
                union(&mut parent, &mut size, i, i + N);
            }
        }
    }

    let open_count = open.iter().filter(|&&c| c).count();
    let spanning = find(&mut parent, top) == find(&mut parent, bottom);
    let cluster = if spanning {
        size[find(&mut parent, top)].saturating_sub(2) // virtual top/bottom nodes
    } else {
        // Largest open component (approx: size of root with most open).
        let mut best = 0usize;
        for (i, &is_open) in open.iter().enumerate() {
            if is_open {
                let r = find(&mut parent, i);
                best = best.max(size[r]);
            }
        }
        best
    };
    (spanning, open_count, cluster)
}

fn draw_rain(canvas: &mut dyn Surface, open: &[bool], spanning: bool) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for gy in 0..N {
        for gx in 0..N {
            if !open[gy * N + gx] {
                continue;
            }
            let left = gx * width / N;
            let right = (((gx + 1) * width / N).max(left + 1)).min(width);
            let top = gy * height / N;
            let bottom = (((gy + 1) * height / N).max(top + 1)).min(height);
            let ch = if spanning { '#' } else { ':' };
            for py in top..bottom {
                for px in left..right {
                    canvas.plot(px as i32, py as i32, ch);
                }
            }
        }
    }
}

/// The First Rain room.
#[derive(Debug, Default)]
pub struct FirstRain {
    seed: u64,
}

impl FirstRain {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn p_at(&self, t: f64, pokes: &[(f64, f64)]) -> f64 {
        let hands = finite_pokes(pokes);
        if let Some(&(x, _)) = hands.last() {
            p_from_hand(x)
        } else {
            ambient_p(t)
        }
    }
}

impl Room for FirstRain {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "first-rain",
            title: "The First Rain",
            wing: "Emergence",
            blurb: "Sites open with probability p; clusters merge until one spans top to bottom. \
                    That cliff sits near p=0.5927. t rains harder; DRAG: MAKE IT RAIN dials p under \
                    the hand.",
            accent: [80, 140, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let p = ambient_p(t);
        let open = open_field(p, self.seed);
        let (spanning, _, _) = spans(&open);
        draw_rain(canvas, &open, spanning);
    }

    fn postcard_t(&self) -> f64 {
        // Near critical: (0.5927-0.35)/0.45 ≈ 0.54
        0.54
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "percolation cliff",
            root: 174.61,
            tempo: 100,
            line: &[0, 0, 5, 7, 12, 7, 5, 0],
            encodes: "quiet dry soil until one wet path suddenly spans",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: MAKE IT RAIN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = ambient_p(t);
        let open = open_field(p, self.seed);
        let (spanning, _open_n, _cluster) = spans(&open);
        let state = if spanning { "SPAN" } else { "DRY" };
        Some(format!("p={p:.3}  pc={P_CRIT:.3}  {state}  DRAG:RAIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let p = self.p_at(t, pokes);
        let open = open_field(p, self.seed);
        let (spanning, _, _) = spans(&open);
        draw_rain(canvas, &open, spanning);
        let (width, height) = canvas.draw_bounds();
        if width > 0 && height > 0 {
            for &(x, y) in &hands {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.plot(px, py, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let p = self.p_at(t, &pokes);
        let open = open_field(p, self.seed);
        let (spanning, _open_n, cluster) = spans(&open);
        let state = if spanning { "SPAN" } else { "DRY" };
        let delta = p - P_CRIT;
        Some(format!(
            "RAIN p={p:.3}  dpc={delta:+.3}  C{cluster}  {state}"
        ))
    }

    fn reveal(&self) -> &'static str {
        "Below a critical wetness nothing spans; above it a path from sky to \
         deep soil appears almost at once. That cliff is not gradual: site \
         percolation on the square lattice has p_c about 0.592746. Universality \
         says many different microscopic rains share the same cliff shape."
    }
}

#[cfg(test)]
mod tests {
    use super::{FirstRain, N, P_CRIT, ambient_p, open_field, p_from_hand, spans};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn dry_field_does_not_span() {
        let open = open_field(0.1, 0);
        let (spanning, open_n, _) = spans(&open);
        assert!(!spanning);
        assert!(open_n < N * N / 2);
    }

    #[test]
    fn flooded_field_spans() {
        let open = open_field(0.95, 0);
        let (spanning, _, _) = spans(&open);
        assert!(spanning);
    }

    #[test]
    fn ambient_p_crosses_critical() {
        assert!(ambient_p(0.0) < P_CRIT);
        assert!(ambient_p(1.0) > P_CRIT);
    }

    #[test]
    fn hand_raises_rain() {
        assert!(p_from_hand(1.0) > p_from_hand(0.0));
    }

    #[test]
    fn first_contact_status_invites_rain() {
        let room = FirstRain::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("DRAG") || open.contains("RAIN"), "{open}");
        assert!(open.contains("p="), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn rain_changes_status() {
        let room = FirstRain::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.9,
            y: 0.5,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("rain");
        assert_ne!(after, open);
        assert!(after.contains("RAIN") || after.contains("p="), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = FirstRain::new();
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        room.render(&mut a, 0.7);
        room.render(&mut b, 0.7);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn hand_changes_the_rain() {
        let room = FirstRain::new();
        let mut dry = Canvas::new(40, 28);
        let mut wet = Canvas::new(40, 28);
        room.render_poked(&mut dry, 0.0, &[(0.1, 0.5)]);
        room.render_poked(&mut wet, 0.0, &[(0.95, 0.5)]);
        assert_ne!(dry.to_text(), wet.to_text());
    }

    #[test]
    fn variation_remixes_the_field() {
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        FirstRain::new_with(0).render(&mut a, 0.6);
        FirstRain::new_with(4).render(&mut b, 0.6);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = FirstRain::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_names_percolation_or_critical() {
        let text = FirstRain::new().reveal().to_ascii_lowercase();
        assert!(text.contains("critical") || text.contains("percolat") || text.contains("0.59"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = FirstRain::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}
