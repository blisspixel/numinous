//! The Magnet: Ising criticality, felt as heat.
//!
//! Spins on a square lattice prefer neighbors that match. Heat fights order.
//! Below a critical temperature the plate magnetizes; above it the field
//! dissolves into noise. The cliff is universal: many microscopics share one
//! shape. `t` sets ambient temperature; DRAG: TURN THE HEAT. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Lattice side.
const N: usize = 40;
/// Coupling J = 1; k_B = 1. Critical T for 2D square Ising (Onsager).
const T_CRIT: f64 = 2.269_185;
/// Monte Carlo sweeps per render (Metropolis).
const SWEEPS: usize = 28;
/// Base seed for thermal noise.
const FIELD_SEED: u64 = 0x15E1_5EED_0000_0001;
const VARIATION_SALT: u64 = 0x15E1_5EED_0000_0002;

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

/// Temperature from ambient phase: cold -> hot through Tc.
fn ambient_temp(t: f64) -> f64 {
    1.2 + phase_unit(t) * 2.4 // 1.2 .. 3.6
}

/// Hand x turns the heat (left cold, right hot).
fn temp_from_hand(x: f64) -> f64 {
    0.8 + x.clamp(0.0, 1.0) * 3.0
}

fn spin_at(spins: &[i8], x: usize, y: usize) -> i8 {
    spins[y * N + x]
}

/// Metropolis Ising from a fixed seed; returns spins and magnetization in [-1,1].
fn equilibrate(temp: f64, seed: u64) -> (Vec<i8>, f64, f64) {
    let temp = temp.clamp(0.05, 8.0);
    let mut rng = SplitMix64::new(FIELD_SEED ^ seed ^ VARIATION_SALT);
    // Start cold-ordered or hot-random depending on T so the cliff is visible fast.
    let mut spins = vec![1i8; N * N];
    if temp > T_CRIT {
        for s in &mut spins {
            *s = if rng.next_f64() < 0.5 { 1 } else { -1 };
        }
    }
    let beta = 1.0 / temp;
    let cells = N * N;
    for _ in 0..SWEEPS {
        for _ in 0..cells {
            let x = rng.below(N as u64) as usize;
            let y = rng.below(N as u64) as usize;
            let s = spin_at(&spins, x, y);
            let xm = if x == 0 { N - 1 } else { x - 1 };
            let xp = if x + 1 == N { 0 } else { x + 1 };
            let ym = if y == 0 { N - 1 } else { y - 1 };
            let yp = if y + 1 == N { 0 } else { y + 1 };
            let nn = spin_at(&spins, xm, y)
                + spin_at(&spins, xp, y)
                + spin_at(&spins, x, ym)
                + spin_at(&spins, x, yp);
            // Delta E for flip s -> -s is 2 s * sum neighbors (J=1).
            let de = 2.0 * f64::from(s) * f64::from(nn);
            if de <= 0.0 || rng.next_f64() < (-beta * de).exp() {
                spins[y * N + x] = -s;
            }
        }
    }
    let sum: i32 = spins.iter().map(|&s| i32::from(s)).sum();
    let m = f64::from(sum) / cells as f64;
    // Mean energy per spin (rough).
    let mut e = 0.0;
    for y in 0..N {
        for x in 0..N {
            let s = spin_at(&spins, x, y);
            let xp = if x + 1 == N { 0 } else { x + 1 };
            let yp = if y + 1 == N { 0 } else { y + 1 };
            e -= f64::from(s) * f64::from(spin_at(&spins, xp, y));
            e -= f64::from(s) * f64::from(spin_at(&spins, x, yp));
        }
    }
    e /= cells as f64;
    (spins, m, e)
}

fn draw_spins(canvas: &mut dyn Surface, spins: &[i8]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for y in 0..N {
        for x in 0..N {
            let up = spins[y * N + x] > 0;
            let left = x * width / N;
            let right = (((x + 1) * width / N).max(left + 1)).min(width);
            let top = y * height / N;
            let bottom = (((y + 1) * height / N).max(top + 1)).min(height);
            let ch = if up { '#' } else { '.' };
            for py in top..bottom {
                for px in left..right {
                    canvas.plot(px as i32, py as i32, ch);
                }
            }
        }
    }
}

/// The Magnet room.
#[derive(Debug, Default)]
pub struct TheMagnet {
    seed: u64,
}

impl TheMagnet {
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

    fn temp_at(&self, t: f64, pokes: &[(f64, f64)]) -> f64 {
        let hands = finite_pokes(pokes);
        if let Some(&(x, _)) = hands.last() {
            temp_from_hand(x)
        } else {
            ambient_temp(t)
        }
    }
}

impl Room for TheMagnet {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "the-magnet",
            title: "The Magnet",
            wing: "Emergence",
            blurb: "Spins lock with their neighbors until heat wins. Cross the critical temperature \
                    and order dissolves. t sets the heat; DRAG: TURN THE HEAT. Universality: one \
                    cliff for many microscopics.",
            accent: [200, 60, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let temp = ambient_temp(t);
        let (spins, _, _) = equilibrate(temp, self.seed);
        draw_spins(canvas, &spins);
    }

    fn postcard_t(&self) -> f64 {
        // Near Tc: (2.269-1.2)/2.4 ≈ 0.445
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "critical heat",
            root: 196.0,
            tempo: 92,
            line: &[0, 0, 7, 0, 12, 0, 7, 0],
            encodes: "quiet order cracking into thermal noise at Tc",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TURN THE HEAT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let temp = ambient_temp(t);
        let (_, m, _) = equilibrate(temp, self.seed);
        let phase = if temp < T_CRIT - 0.15 {
            "ORDER"
        } else if temp > T_CRIT + 0.15 {
            "NOISE"
        } else {
            "CRIT"
        };
        Some(format!(
            "T={temp:.2}  Tc={T_CRIT:.2}  M={m:+.2}  {phase}  DRAG:HEAT"
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let temp = self.temp_at(t, pokes);
        let (spins, _, _) = equilibrate(temp, self.seed);
        draw_spins(canvas, &spins);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let temp = self.temp_at(t, &pokes);
        let (_, m, e) = equilibrate(temp, self.seed);
        let dtc = temp - T_CRIT;
        let phase = if temp < T_CRIT - 0.15 {
            "ORDER"
        } else if temp > T_CRIT + 0.15 {
            "NOISE"
        } else {
            "CRIT"
        };
        let _ = e;
        Some(format!(
            "HEAT T={temp:.2}  dTc={dtc:+.2}  M={m:+.2}  {phase}"
        ))
    }

    fn reveal(&self) -> &'static str {
        "Order is not gradual here. Below a critical temperature the plate \
         chooses a global magnetization; above it, only local flickers remain. \
         Onsager solved the square Ising model exactly: Tc is about 2.269 J/kB. \
         Many different microscopics share that same cliff: universality."
    }
}

#[cfg(test)]
mod tests {
    use super::{T_CRIT, TheMagnet, ambient_temp, equilibrate, temp_from_hand};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn cold_plate_is_highly_magnetized() {
        let (_, m, _) = equilibrate(1.0, 0);
        assert!(m.abs() > 0.7, "cold M={m}");
    }

    #[test]
    fn hot_plate_is_weakly_magnetized() {
        let (_, m, _) = equilibrate(4.0, 0);
        assert!(m.abs() < 0.35, "hot M={m}");
    }

    #[test]
    fn ambient_temp_crosses_tc() {
        assert!(ambient_temp(0.0) < T_CRIT);
        assert!(ambient_temp(1.0) > T_CRIT);
    }

    #[test]
    fn hand_raises_heat() {
        assert!(temp_from_hand(1.0) > temp_from_hand(0.0));
    }

    #[test]
    fn first_contact_status_invites_heat() {
        let room = TheMagnet::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("DRAG") || open.contains("HEAT"), "{open}");
        assert!(open.contains("T="), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn heat_changes_status() {
        let room = TheMagnet::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.95,
            y: 0.5,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("heat");
        assert_ne!(after, open);
        assert!(after.contains("HEAT") || after.contains("T="), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = TheMagnet::new();
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        room.render(&mut a, 0.2);
        room.render(&mut b, 0.2);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 30);
    }

    #[test]
    fn heat_changes_the_picture() {
        let room = TheMagnet::new();
        let mut cold = Canvas::new(40, 28);
        let mut hot = Canvas::new(40, 28);
        room.render_poked(&mut cold, 0.0, &[(0.05, 0.5)]);
        room.render_poked(&mut hot, 0.0, &[(0.95, 0.5)]);
        assert_ne!(cold.to_text(), hot.to_text());
    }

    #[test]
    fn variation_remixes_thermal_path() {
        let mut a = Canvas::new(36, 24);
        let mut b = Canvas::new(36, 24);
        TheMagnet::new_with(0).render(&mut a, 0.7);
        TheMagnet::new_with(3).render(&mut b, 0.7);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = TheMagnet::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_names_critical_or_onsager() {
        let text = TheMagnet::new().reveal().to_ascii_lowercase();
        assert!(text.contains("critical") || text.contains("onsager") || text.contains("ising"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = TheMagnet::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}
