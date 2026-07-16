//! Soap Film / Steiner tree: least path meeting at 120 degrees.
//!
//! Pins on the plate; a discrete relaxation finds the Steiner topology.
//! PIN: HOLD A WIRE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

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

fn default_pins(t: f64, seed: u64) -> Vec<(f64, f64)> {
    let wobble = phase_unit(t) * 0.05;
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    vec![
        (0.2 + s, 0.25 + wobble),
        (0.8 - s, 0.28),
        (0.75, 0.78 - wobble),
        (0.22, 0.75),
    ]
}

/// Fermat-Torricelli / Steiner point for three terminals: minimize sum of distances.
fn steiner3(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> (f64, f64) {
    // If all angles of triangle < 120, the Torricelli point has 120 deg angles.
    // Iterate geometric median (Weiszfeld) as a robust toy.
    let mut p = ((a.0 + b.0 + c.0) / 3.0, (a.1 + b.1 + c.1) / 3.0);
    for _ in 0..40 {
        let pts = [a, b, c];
        let mut num_x = 0.0;
        let mut num_y = 0.0;
        let mut den = 0.0;
        for q in pts {
            let d = (p.0 - q.0).hypot(p.1 - q.1).max(1e-6);
            num_x += q.0 / d;
            num_y += q.1 / d;
            den += 1.0 / d;
        }
        p = (num_x / den, num_y / den);
    }
    p
}

fn total_len(pins: &[(f64, f64)], steiner: (f64, f64)) -> f64 {
    pins.iter()
        .map(|p| (p.0 - steiner.0).hypot(p.1 - steiner.1))
        .sum()
}

fn mst_len(pins: &[(f64, f64)]) -> f64 {
    // Prim MST length for comparison.
    let n = pins.len();
    if n < 2 {
        return 0.0;
    }
    let mut in_tree = vec![false; n];
    in_tree[0] = true;
    let mut len = 0.0;
    for _ in 1..n {
        let mut best = f64::MAX;
        let mut best_j = 0;
        for i in 0..n {
            if !in_tree[i] {
                continue;
            }
            for j in 0..n {
                if in_tree[j] {
                    continue;
                }
                let d = (pins[i].0 - pins[j].0).hypot(pins[i].1 - pins[j].1);
                if d < best {
                    best = d;
                    best_j = j;
                }
            }
        }
        in_tree[best_j] = true;
        len += best;
    }
    len
}

fn draw(canvas: &mut dyn Surface, pins: &[(f64, f64)], junction: (f64, f64)) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |p: (f64, f64)| -> (i32, i32) {
        (
            (p.0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (p.1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let j = to_px(junction);
    for &p in pins {
        let q = to_px(p);
        canvas.line(j.0, j.1, q.0, q.1, '*');
        canvas.plot(q.0, q.1, 'P');
    }
    canvas.plot(j.0, j.1, 'O');
}

/// Soap Film / Steiner room.
#[derive(Debug, Default)]
pub struct SoapFilm {
    seed: u64,
}

impl SoapFilm {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }
    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for SoapFilm {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "soap-film",
            title: "Soap Film",
            wing: "Shape & Space",
            blurb: "A film finds least length; Steiner junctions meet at 120 degrees. t wobbles \
                    pins; CLICK: PIN A WIRE.",
            accent: [180, 220, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let pins = default_pins(t, self.seed);
        // Use first three for Torricelli; fourth attaches to junction.
        let j = steiner3(pins[0], pins[1], pins[2]);
        draw(canvas, &pins, j);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "soap steiner",
            root: 246.94,
            tempo: 88,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "least length meeting at one hundred twenty degrees",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PIN A WIRE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let pins = default_pins(t, self.seed);
        let j = steiner3(pins[0], pins[1], pins[2]);
        let l = total_len(&pins[..3], j);
        let m = mst_len(&pins[..3]);
        Some(format!("film={l:.2}  mst={m:.2}  CLICK:PIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let mut pins = default_pins(t, self.seed);
        if let Some(&p) = hands.last() {
            // Replace nearest pin with hand.
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for (i, q) in pins.iter().enumerate() {
                let d = (q.0 - p.0).hypot(q.1 - p.1);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            pins[best] = p;
        }
        let j = steiner3(pins[0], pins[1], pins[2]);
        draw(canvas, &pins, j);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let mut pins = default_pins(t, self.seed);
        if let Some(&p) = hands.last() {
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for (i, q) in pins.iter().enumerate() {
                let d = (q.0 - p.0).hypot(q.1 - p.1);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            pins[best] = p;
        }
        let j = steiner3(pins[0], pins[1], pins[2]);
        let l = total_len(&pins[..3], j);
        let m = mst_len(&pins[..3]);
        Some(format!(
            "PIN film={l:.2}  save={:.0}%",
            (1.0 - l / m.max(1e-6)) * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "A soap film between pins finds a least-length network. When a Steiner \
         point appears, edges meet at 120 degrees: nature's calculus of \
         variations, visible as light on liquid."
    }
}

#[cfg(test)]
mod tests {
    use super::{SoapFilm, mst_len, steiner3, total_len};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SoapFilm::new().status(0.3).unwrap();
        assert!(s.contains("CLICK") || s.contains("PIN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn pin_changes() {
        let r = SoapFilm::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.4,
                    y: 0.4,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn steiner_not_worse_than_mst() {
        let a = (0.2, 0.2);
        let b = (0.8, 0.2);
        let c = (0.5, 0.8);
        let j = steiner3(a, b, c);
        let film = total_len(&[a, b, c], j);
        let mst = mst_len(&[a, b, c]);
        assert!(film <= mst + 1e-6);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        SoapFilm::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(SoapFilm::new().motif().unwrap().line.len() >= 6);
    }
}
