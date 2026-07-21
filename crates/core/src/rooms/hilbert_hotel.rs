//! Hilbert's Hotel: full house, room for one more, until the reals arrive.
//!
//! Countable infinity reshuffles to admit guests; uncountable demand cannot.
//! ADMIT: THE NEXT GUEST. See `docs/ROOMS.md`.

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

#[derive(Clone, Copy)]
enum Mode {
    Full,
    PlusOne,
    Bus,
    Reals,
}

fn mode(t: f64, hand: Option<(f64, f64)>) -> Mode {
    let u = if let Some((x, _)) = hand {
        x
    } else {
        phase_unit(t)
    };
    if u < 0.25 {
        Mode::Full
    } else if u < 0.5 {
        Mode::PlusOne
    } else if u < 0.75 {
        Mode::Bus
    } else {
        Mode::Reals
    }
}

fn draw(canvas: &mut dyn Surface, mode: Mode, shown: usize) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let rooms = shown.clamp(8, 40);
    for i in 0..rooms {
        let x = ((0.08 + i as f64 * 0.9 / rooms as f64) * width as f64).round() as i32;
        let y0 = (0.75 * height as f64).round() as i32;
        let y1 = (0.35 * height as f64).round() as i32;
        match mode {
            Mode::Full => {
                canvas.line(x, y0, x, y1, '#');
                canvas.plot(x, y1 - 1, 'G');
            }
            Mode::PlusOne => {
                // Guest n moves to n+1; room 1 free.
                let label = if i == 0 { '+' } else { 'G' };
                canvas.line(x, y0, x, y1, '*');
                canvas.plot(x, y1 - 1, label);
            }
            Mode::Bus => {
                // Even rooms to bus; odds stay (toy).
                let ch = if i % 2 == 0 { 'B' } else { 'G' };
                canvas.line(x, y0, x, y1, if i % 2 == 0 { '#' } else { '*' });
                canvas.plot(x, y1 - 1, ch);
            }
            Mode::Reals => {
                // Dense mess: cannot list.
                canvas.line(x, y0, x, y1, '.');
                if i % 3 == 0 {
                    canvas.plot(x, y1 - 1, '?');
                }
            }
        }
    }
    // Lobby line.
    canvas.line(
        0,
        (0.8 * height as f64).round() as i32,
        width.saturating_sub(1) as i32,
        (0.8 * height as f64).round() as i32,
        '=',
    );
}

/// Hilbert Hotel room.
#[derive(Debug, Default)]
pub struct HilbertHotel {
    seed: u64,
}

impl HilbertHotel {
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

impl Room for HilbertHotel {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hilbert-hotel",
            title: "Hilbert's Hotel",
            wing: "Number & Pattern",
            blurb: "Full hotel, room for one more bus, until the reals check in. t and DRAG: ADMIT \
                    THE NEXT GUEST.",
            accent: [160, 120, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let m = mode(t, None);
        let shown = 16
            + (phase_unit(t) * 16.0) as usize
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 4) as usize
            };
        draw(canvas, m, shown);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hotel aleph",
            root: 130.81,
            tempo: 84,
            line: &[0, 0, 5, 7, 12, 12, 7, 5],
            encodes: "countable reshuffles; uncountable demand cannot",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: ADMIT THE NEXT GUEST")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mode(t, None);
        let name = match m {
            Mode::Full => "FULL",
            Mode::PlusOne => "+1 OK",
            Mode::Bus => "BUS OK",
            Mode::Reals => "REALS?",
        };
        Some(format!("hotel={name}  aleph0  DRAG:ADMIT"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = mode(t, hands.last().copied());
        let shown = 18 + hands.len() * 2;
        draw(canvas, m, shown);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let m = mode(t, hands.last().copied());
        let msg = match m {
            Mode::Full => "FULL n=0 free  ADMIT",
            Mode::PlusOne => "SHIFT n->n+1  fits=1",
            Mode::Bus => "EVENS free  bus=1",
            Mode::Reals => "REALS no list  c>aleph0",
        };
        Some(msg.into())
    }

    fn reveal(&self) -> &'static str {
        "Hilbert's hotel is always full and always has room for one more \
         countable guest: map n to n+1, or n to 2n for a bus. A continuum of \
         guests has no such list; countable infinity is not the whole story."
    }
}

#[cfg(test)]
mod tests {
    use super::HilbertHotel;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HilbertHotel::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("ADMIT"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn admit_changes() {
        let r = HilbertHotel::new();
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
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        HilbertHotel::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(HilbertHotel::new().motif().unwrap().line.len() >= 6);
    }
}
