//! Koch snowflake: closed Koch curve as a polygonal island.
//!
//! Distinct from the existing Koch coast room (if open-path); this is the
//! closed triangular snowflake. DRAG: SET THE ORDER. See `docs/ROOMS.md`.

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

fn order(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 5.0) as usize).clamp(1, 6)
    } else {
        (2 + (phase_unit(t) * 3.0) as usize).clamp(1, 5)
    }
}

fn koch_edge(a: (f64, f64), b: (f64, f64), n: usize, out: &mut Vec<(f64, f64)>) {
    if n == 0 || out.len() > 12_000 {
        out.push(b);
        return;
    }
    let (ax, ay) = a;
    let (bx, by) = b;
    let dx = bx - ax;
    let dy = by - ay;
    let p1 = (ax + dx / 3.0, ay + dy / 3.0);
    let p3 = (ax + 2.0 * dx / 3.0, ay + 2.0 * dy / 3.0);
    // Peak rotated +60 degrees.
    let mx = p3.0 - p1.0;
    let my = p3.1 - p1.1;
    let p2 = (
        p1.0 + 0.5 * mx - 0.866_025_403_78 * my,
        p1.1 + 0.866_025_403_78 * mx + 0.5 * my,
    );
    koch_edge(a, p1, n - 1, out);
    koch_edge(p1, p2, n - 1, out);
    koch_edge(p2, p3, n - 1, out);
    koch_edge(p3, b, n - 1, out);
}

fn snowflake(n: usize) -> Vec<(f64, f64)> {
    let a = (0.1, 0.25);
    let b = (0.9, 0.25);
    let c = (0.5, 0.25 + 0.8 * 0.866_025_403_78);
    let mut out = vec![a];
    koch_edge(a, b, n, &mut out);
    koch_edge(b, c, n, &mut out);
    koch_edge(c, a, n, &mut out);
    out
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(x, y)) in pts.iter().enumerate() {
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, if i % 4 == 0 { '#' } else { '*' });
        }
        prev = Some((px, py));
    }
}

/// Koch snowflake room.
#[derive(Debug, Default)]
pub struct KochSnowflake {
    seed: u64,
}

impl KochSnowflake {
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

impl Room for KochSnowflake {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "koch-snowflake",
            title: "Koch Snowflake",
            wing: "Fractals",
            blurb: "Closed Koch curve: infinite coast, finite area. t and DRAG: SET THE ORDER.",
            accent: [100, 180, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let mut o = order(t, None);
        if self.seed != 0 {
            o = (o + (self.seed % 2) as usize).clamp(1, 6);
        }
        draw(canvas, &snowflake(o));
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "snowflake",
            root: 261.63,
            tempo: 86,
            line: &[0, 3, 7, 3, 12, 7, 3, 0],
            encodes: "three Koch coasts closing into an island",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ORDER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let o = order(t, None);
        let n = snowflake(o).len();
        Some(format!("order={o}  pts={n}  DRAG:ORDER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let o = order(t, hands.last().copied());
        draw(canvas, &snowflake(o));
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, 'o');
                canvas.line(px, py - 2, px, py + 2, 'o');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let o = order(t, hands.last().copied());
        let n = snowflake(o).len();
        Some(format!("ORDER={o}  pts={n}"))
    }

    fn reveal(&self) -> &'static str {
        "The Koch snowflake is a closed Koch curve. Perimeter diverges with \
         iteration while area stays finite: a continuous loop that is nowhere \
         smooth in the limit."
    }
}

#[cfg(test)]
mod tests {
    use super::{KochSnowflake, snowflake};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = KochSnowflake::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = KochSnowflake::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn grows() {
        assert!(snowflake(1).len() < snowflake(3).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        KochSnowflake::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(KochSnowflake::new().motif().unwrap().line.len() >= 6);
    }
}
