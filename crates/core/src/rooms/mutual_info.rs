//! Mutual information: joint histogram of two correlated bits.
//!
//! DRAG: TUNE R. See `docs/ROOMS.md`.

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

fn corr(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.04
    };
    if let Some((x, _)) = hand {
        (x * 0.98 + s).clamp(0.0, 1.0)
    } else {
        (phase_unit(t) * 0.95 + s).clamp(0.0, 1.0)
    }
}

fn hash_u(i: u64, salt: u64) -> f64 {
    let mut x = i.wrapping_mul(0xD1B5_4A32_D192_ED03).wrapping_add(salt);
    x ^= x >> 29;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 32;
    (x as f64) / (u64::MAX as f64)
}

/// Binary mutual information I(X;Y) for P(X=Y)=c biased fair margins.
fn mutual_bits(c: f64) -> f64 {
    // joint: p00=p11=c/2, p01=p10=(1-c)/2 when margins fair and P(equal)=c
    let c = c.clamp(0.0, 1.0);
    let p_eq = c * 0.5 + (1.0 - c) * 0.25; // not right - simpler model:
    // Use: X bernoulli 1/2, Y = X with prob r else flip.
    let r = c;
    let p11 = 0.5 * r;
    let p10 = 0.5 * (1.0 - r);
    let p01 = 0.5 * (1.0 - r);
    let p00 = 0.5 * r;
    let _ = p_eq;
    let hx = 1.0; // fair binary
    let mut hxy = 0.0;
    for p in [p00, p01, p10, p11] {
        if p > 1e-15 {
            hxy -= p * p.log2();
        }
    }
    // H(Y)=1 also fair; I = H(X)+H(Y)-H(X,Y) = 2 - H(X,Y)
    let _ = hx;
    (2.0 - hxy).clamp(0.0, 1.0)
}

fn draw(canvas: &mut dyn Surface, r: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // sample cloud
    let n = (width * height / 4).clamp(80, 600);
    let mut counts = [[0u32; 2]; 2];
    for i in 0..n {
        let u = hash_u(i as u64, seed.wrapping_add(11));
        let v = hash_u(i as u64 + 999, seed.wrapping_mul(3) + 5);
        let x = if u < 0.5 { 0usize } else { 1 };
        let y = if v < r { x } else { 1 - x };
        counts[x][y] += 1;
        let px = if x == 0 {
            (width as f64 * 0.25 + (u - 0.25).abs() * width as f64 * 0.3) as i32
        } else {
            (width as f64 * 0.65 + (u - 0.5).abs() * width as f64 * 0.3) as i32
        };
        let py = if y == 0 {
            (height as f64 * 0.25 + v * height as f64 * 0.2) as i32
        } else {
            (height as f64 * 0.65 + v * height as f64 * 0.2) as i32
        };
        let px = px.clamp(0, width as i32 - 1);
        let py = py.clamp(0, height as i32 - 1);
        canvas.line(px, py, px, py, if x == y { '#' } else { '.' });
    }
    // I(r) curve on bottom strip
    let mut prev: Option<(i32, i32)> = None;
    let base = height as i32 - 3;
    for col in 0..width {
        let rr = col as f64 / width.saturating_sub(1).max(1) as f64;
        let mi = mutual_bits(rr);
        let y = base - (mi * 4.0).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, y, '=');
        }
        prev = Some((col as i32, y));
    }
    let mx = (r * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(mx, base - 5, mx, base, '|');
}

/// Mutual information room.
#[derive(Debug, Default)]
pub struct MutualInfo {
    seed: u64,
}

impl MutualInfo {
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

impl Room for MutualInfo {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mutual-info",
            title: "Mutual Information",
            wing: "Chance & Noise",
            blurb: "How much X tells you about Y. t and DRAG: TUNE R.",
            accent: [110, 90, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, corr(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "mutual-info",
            root: 174.61,
            tempo: 72,
            line: &[0, 5, 3, 8, 7, 3, 0, 5],
            encodes: "I(X;Y)=H(X)+H(Y)-H(X,Y): shared surprise between variables",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = corr(t, None, self.seed);
        let mi = mutual_bits(r);
        Some(format!("r={r:.2}  I={mi:.2}b  DRAG:R"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = corr(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
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
        let r = corr(t, hands.last().copied(), self.seed);
        let mi = mutual_bits(r);
        // residual surprise after learning X: H(Y|X) = H(Y) - I = 1 - I for fair Y
        let cond = (1.0 - mi).max(0.0);
        Some(format!("I={mi:.3}b  H(Y|X)={cond:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Mutual information I(X;Y) is the shared surprise of two variables: \
         how many bits learning X saves about Y. Independent variables share \
         none; perfect copies share everything. Here r is the copy probability."
    }
}

#[cfg(test)]
mod tests {
    use super::MutualInfo;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MutualInfo::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains('I'));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r_changes() {
        let r = MutualInfo::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        MutualInfo::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}
