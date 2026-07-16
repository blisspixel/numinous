//! Causal Doors: intervention vs observation (toy do-calculus).
//!
//! A three-node graph: rain -> sprinkler, rain -> wet, sprinkler -> wet.
//! OPEN: A VALVE and watch which edges still move the wet ground.
//! See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0xCADA_1004_0000_0001;

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

#[derive(Clone, Copy)]
enum Interv {
    None,
    ForceRain,
    ForceSprinkler,
}

fn interv(t: f64, hand: Option<(f64, f64)>) -> Interv {
    let u = if let Some((x, _)) = hand {
        x
    } else {
        phase_unit(t)
    };
    if u < 0.34 {
        Interv::None
    } else if u < 0.67 {
        Interv::ForceRain
    } else {
        Interv::ForceSprinkler
    }
}

/// Sample joint under observation or do().
fn sample(seed: u64, n: usize, interv: Interv) -> (f64, f64, f64) {
    // Returns P(wet), P(rain), P(sprinkler) estimates.
    let mut rng = SplitMix64::new(SEED ^ seed ^ (n as u64));
    let mut wet = 0u32;
    let mut rain = 0u32;
    let mut spr = 0u32;
    for _ in 0..n {
        let r = match interv {
            Interv::ForceRain => true,
            _ => rng.next_f64() < 0.3,
        };
        let s = match interv {
            Interv::ForceSprinkler => true,
            Interv::ForceRain => rng.next_f64() < 0.1, // rain suppresses sprinkler
            Interv::None => {
                if r {
                    rng.next_f64() < 0.1
                } else {
                    rng.next_f64() < 0.5
                }
            }
        };
        let w = if r || s {
            rng.next_f64() < 0.9
        } else {
            rng.next_f64() < 0.05
        };
        if r {
            rain += 1;
        }
        if s {
            spr += 1;
        }
        if w {
            wet += 1;
        }
    }
    let n = n as f64;
    (wet as f64 / n, rain as f64 / n, spr as f64 / n)
}

fn draw(canvas: &mut dyn Surface, interv: Interv, p_wet: f64, p_rain: f64, p_spr: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x * width.saturating_sub(1) as f64).round() as i32,
            (y * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let rain = to_px(0.5, 0.2);
    let spr = to_px(0.2, 0.55);
    let wet = to_px(0.8, 0.55);
    canvas.line(
        rain.0,
        rain.1,
        spr.0,
        spr.1,
        if matches!(interv, Interv::ForceSprinkler) {
            '.'
        } else {
            '*'
        },
    );
    canvas.line(rain.0, rain.1, wet.0, wet.1, '*');
    canvas.line(spr.0, spr.1, wet.0, wet.1, '*');
    canvas.plot(rain.0, rain.1, 'R');
    canvas.plot(spr.0, spr.1, 'S');
    canvas.plot(wet.0, wet.1, 'W');
    // Bars
    let bars = [(p_rain, 0.75, 'R'), (p_spr, 0.82, 'S'), (p_wet, 0.89, 'W')];
    for (p, yf, ch) in bars {
        let y = (yf * height as f64).round() as i32;
        let x0 = (0.15 * width as f64).round() as i32;
        let x1 = ((0.15 + p * 0.7) * width as f64).round() as i32;
        canvas.line(x0, y, x1, y, '=');
        canvas.plot(x0 - 2, y, ch);
    }
    if matches!(interv, Interv::ForceRain | Interv::ForceSprinkler) {
        canvas.plot(rain.0 + 2, rain.1 - 1, '!');
    }
}

/// Causal Doors room.
#[derive(Debug, Default)]
pub struct CausalDoors {
    seed: u64,
}

impl CausalDoors {
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

impl Room for CausalDoors {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "causal-doors",
            title: "Causal Doors",
            wing: "Number & Pattern",
            blurb: "Watching is not intervening. Force rain or the sprinkler and wetness answers \
                    differently. t and DRAG: OPEN A VALVE.",
            accent: [100, 160, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let i = interv(t, None);
        let (w, r, s) = sample(self.seed, 400, i);
        draw(canvas, i, w, r, s);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "do calc",
            root: 174.61,
            tempo: 96,
            line: &[0, 5, 7, 5, 0, 7, 12, 0],
            encodes: "intervention breaks edges that observation leaves intact",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: OPEN A VALVE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let i = interv(t, None);
        let (w, _, _) = sample(self.seed, 300, i);
        let name = match i {
            Interv::None => "OBS",
            Interv::ForceRain => "do(R)",
            Interv::ForceSprinkler => "do(S)",
        };
        Some(format!("P(W)={w:.2}  {name}  DRAG:VALVE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let i = interv(t, hands.last().copied());
        let (w, r, s) = sample(self.seed ^ hands.len() as u64, 500, i);
        draw(canvas, i, w, r, s);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let i = interv(t, hands.last().copied());
        let (w, r, s) = sample(self.seed ^ hands.len() as u64, 500, i);
        let name = match i {
            Interv::None => "OBS",
            Interv::ForceRain => "do(RAIN)",
            Interv::ForceSprinkler => "do(SPR)",
        };
        Some(format!("{name}  W={w:.2} R={r:.2} S={s:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Correlation is not intervention. Forcing the sprinkler on changes \
         wetness without changing rain; conditioning on wetness confounds them. \
         do-calculus is the grammar of that difference, worn as three doors."
    }
}

#[cfg(test)]
mod tests {
    use super::{CausalDoors, Interv, sample};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CausalDoors::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("VALVE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn valve_changes() {
        let r = CausalDoors::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn force_sprinkler_raises_wet() {
        let (w0, _, _) = sample(1, 800, Interv::None);
        let (w1, _, s1) = sample(1, 800, Interv::ForceSprinkler);
        assert!(s1 > 0.9);
        assert!(w1 >= w0 - 0.05);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        CausalDoors::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(CausalDoors::new().motif().unwrap().line.len() >= 6);
    }
}
