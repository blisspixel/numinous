//! Bayes update: prior, likelihood, posterior as stacked bars.
//!
//! DRAG: TUNE L. See `docs/ROOMS.md`.

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

/// Likelihood ratio L = P(data|H)/P(data|~H), tuned by hand.
fn likelihood(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        0.2 + x * 4.5 + s
    } else {
        0.4 + phase_unit(t) * 3.5 + s
    }
    .clamp(0.15, 6.0)
}

fn prior(seed: u64) -> f64 {
    if seed == 0 {
        0.35
    } else {
        0.2 + (seed % 5) as f64 * 0.1
    }
}

fn posterior(prior: f64, lr: f64) -> f64 {
    // odds form: o' = o * L
    let o = prior / (1.0 - prior).max(1e-9);
    let op = o * lr;
    op / (1.0 + op)
}

fn draw(canvas: &mut dyn Surface, prior_p: f64, lr: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let post = posterior(prior_p, lr);
    let rows = [
        ("prior", prior_p, '#'),
        ("like", (lr / 6.0).clamp(0.05, 1.0), '='),
        ("post", post, '@'),
    ];
    let band = (height / 4).max(2);
    let pad = if seed == 0 { 0i32 } else { (seed % 2) as i32 };
    for (i, &(_, val, ch)) in rows.iter().enumerate() {
        let y0 = (i * band) as i32 + pad + 1;
        let w = (val * width.saturating_sub(2) as f64).round() as i32;
        for dy in 0..band.saturating_sub(1) as i32 {
            canvas.line(1, y0 + dy, 1 + w, y0 + dy, ch);
        }
    }
    // unity line
    let mid = width as i32 / 2;
    canvas.line(mid, 0, mid, height as i32 - 1, '|');
}

/// Bayes update room.
#[derive(Debug, Default)]
pub struct BayesUpdate {
    seed: u64,
}

impl BayesUpdate {
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

impl Room for BayesUpdate {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bayes-update",
            title: "Bayes Update",
            wing: "Chance & Noise",
            blurb: "Prior times likelihood becomes posterior. t and DRAG: TUNE L.",
            accent: [100, 80, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let pr = prior(self.seed);
        let lr = likelihood(t, None, self.seed);
        draw(canvas, pr, lr, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bayes-update",
            root: 233.08,
            tempo: 68,
            line: &[0, 5, 7, 5, 0, 7, 12, 7],
            encodes: "Bayes odds: posterior odds = prior odds times likelihood",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE L")
    }

    fn status(&self, t: f64) -> Option<String> {
        let pr = prior(self.seed);
        let lr = likelihood(t, None, self.seed);
        let po = posterior(pr, lr);
        Some(format!("L={lr:.1}  post={po:.2}  DRAG:L"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let pr = prior(self.seed);
        let lr = likelihood(t, hands.last().copied(), self.seed);
        draw(canvas, pr, lr, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let pr = prior(self.seed);
        let lr = likelihood(t, hands.last().copied(), self.seed);
        let po = posterior(pr, lr);
        Some(format!("L={lr:.2}  post={po:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Bayes' rule rewrites belief after data. In odds form it is simple: \
         posterior odds = prior odds times the likelihood ratio. A strong \
         likelihood can flip a weak prior; a weak one barely moves you."
    }
}

#[cfg(test)]
mod tests {
    use super::BayesUpdate;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = BayesUpdate::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("post"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn l_changes() {
        let r = BayesUpdate::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        BayesUpdate::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
