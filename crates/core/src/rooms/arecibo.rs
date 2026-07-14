//! The Arecibo puzzle: a message that only a species that does math can read.
//!
//! A stream of bits looks like noise until you arrange it into a grid, and it can
//! only be arranged one way, because its length is a semiprime with exactly two
//! factors. Here the message is 143 bits, and 143 is 11 times 13, nothing else.
//! `t` sweeps the width you try; only at the true width does the picture appear.
//! (The real 1974 message was 1,679 bits: 23 times 73.) See `docs/ROOMS.md`.

use crate::MAX_ROOM_POKES;
use crate::room::{Room, RoomInput, RoomMeta, pokes_from_inputs};
use crate::surface::Surface;

/// The hidden payload, drawn at the one width that reads it: a big letter pi.
const ART: [&str; 13] = [
    "...........",
    "###########",
    "###########",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...###.",
    "...........",
    "...........",
];
/// Smallest and largest widths `t` will try.
const MIN_WIDTH: usize = 6;
const MAX_WIDTH: usize = 18;

/// The Arecibo message room.
#[derive(Debug, Default)]
pub struct Arecibo {
    seed: u64,
}

impl Arecibo {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// The deliberately unsolved width shown before the player chooses one.
    fn width_for(_t: f64, seed: u64) -> usize {
        const WRONG_WIDTHS: [usize; 11] = [10, 6, 7, 8, 9, 12, 14, 15, 16, 17, 18];
        WRONG_WIDTHS[(seed % WRONG_WIDTHS.len() as u64) as usize]
    }

    fn width_from_point(px: f64) -> usize {
        MIN_WIDTH
            + ((px.clamp(0.0, 1.0) * (MAX_WIDTH - MIN_WIDTH) as f64).round() as usize)
                .min(MAX_WIDTH - MIN_WIDTH)
    }

    fn poked_attempts(pokes: &[(f64, f64)]) -> impl Iterator<Item = usize> + '_ {
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        pokes[start..]
            .iter()
            .rev()
            .filter_map(|&(px, py)| {
                if !px.is_finite() || !py.is_finite() {
                    return None;
                }
                Some(Self::width_from_point(px))
            })
            .take(1)
    }

    fn width_status(width: usize) -> String {
        let total = bits().len();
        let rows = total / width;
        let remainder = total % width;
        if remainder == 0 && width == 11 {
            format!("{total} BITS   WIDTH {width} x {rows}   SIGNAL LOCKED: PI")
        } else if remainder == 0 && width == 13 {
            format!("{total} BITS   WIDTH {width} x {rows}   FACTOR PAIR FOUND   TRY WIDTH 11")
        } else {
            format!("{total} BITS   WIDTH {width} x {rows}   {remainder} BITS LEFT OVER")
        }
    }
}

/// The message as a flat bitstream (row-major over `ART`).
fn bits() -> Vec<bool> {
    ART.iter()
        .flat_map(|row| row.chars().map(|c| c == '#'))
        .collect()
}

fn cell_pixels(cell: usize, cells: usize, pixels: usize) -> std::ops::Range<usize> {
    (cell * pixels).div_ceil(cells)..((cell + 1) * pixels).div_ceil(cells)
}

fn inset_cell(range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let len = range.end.saturating_sub(range.start);
    if len < 6 {
        return range;
    }
    let inset = (len / 10).max(1);
    range.start + inset..range.end.saturating_sub(inset)
}

fn grid_cell_pixels(
    cell: usize,
    cells: usize,
    pixels: usize,
    square_cell: usize,
) -> std::ops::Range<usize> {
    if square_cell == 0 {
        return inset_cell(cell_pixels(cell, cells, pixels));
    }
    let grid_pixels = square_cell.saturating_mul(cells);
    let origin = pixels.saturating_sub(grid_pixels) / 2;
    inset_cell(origin + cell * square_cell..origin + (cell + 1) * square_cell)
}

fn draw_message(
    canvas: &mut dyn Surface,
    stream: &[bool],
    grid_w: usize,
    ch: char,
    shift_x: i32,
    shift_y: i32,
) {
    let width = canvas.width();
    let height = canvas.height();
    let Ok(width_i32) = i32::try_from(width) else {
        return;
    };
    let Ok(height_i32) = i32::try_from(height) else {
        return;
    };
    let grid_h = stream.len().div_ceil(grid_w);
    let square_cell = (width / grid_w).min(height / grid_h);
    for (index, &bit) in stream.iter().enumerate() {
        if !bit {
            continue;
        }
        let gx = index % grid_w;
        let gy = index / grid_w;
        for py in grid_cell_pixels(gy, grid_h, height, square_cell) {
            for px in grid_cell_pixels(gx, grid_w, width, square_cell) {
                let Some(x) = i32::try_from(px).ok().and_then(|x| x.checked_add(shift_x)) else {
                    continue;
                };
                let Some(y) = i32::try_from(py).ok().and_then(|y| y.checked_add(shift_y)) else {
                    continue;
                };
                if x >= 0 && x < width_i32 && y >= 0 && y < height_i32 {
                    canvas.plot(x, y, ch);
                }
            }
        }
    }
}

impl Room for Arecibo {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "arecibo",
            title: "Arecibo Message",
            wing: "Signals & Codes",
            blurb: "A stream of bits that looks like noise until you line it up at the right width. \
                    The length is a semiprime, so it has one nontrivial rectangle up to rotation.",
            accent: [120, 230, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let grid_w = Self::width_for(t, self.seed);
        let stream = bits();
        draw_message(canvas, &stream, grid_w, '#', 0, 0);
    }

    fn postcard_t(&self) -> f64 {
        0.42
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(Self::width_status(Self::width_for(t, self.seed)))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = pokes_from_inputs(inputs);
        let width = Self::poked_attempts(&pokes)
            .next()
            .unwrap_or_else(|| Self::width_for(t, self.seed));
        Some(Self::width_status(width))
    }

    fn reveal(&self) -> &'static str {
        "This message is 143 bits, and 143 is 11 times 13, so it has one \
         nontrivial rectangular factor pair, up to rotation. Any species that could factor it would \
         find the picture. In 1974 we sent 1,679 bits, which is 23 times 73, at a \
         star cluster 25,000 light-years away. The reply is not due for a while."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "B binary beacon",
            root: 123.47,
            tempo: 72,
            line: &[0, 12, 0, 7, 0, 12, 5, 0, 7, 12],
            encodes: "a prime-rectangle signal pulsing across space",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK LEFT OR RIGHT: TRY A WIDTH")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let grid_w = Self::poked_attempts(pokes)
            .next()
            .unwrap_or_else(|| Self::width_for(t, self.seed));
        let stream = bits();
        draw_message(canvas, &stream, grid_w, '#', 0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::{ART, Arecibo, MAX_WIDTH, MIN_WIDTH, bits, grid_cell_pixels};
    use crate::MAX_ROOM_POKES;
    use crate::canvas::Canvas;
    use crate::room::Room;

    /// The secret width: the smaller factor of 143 (11 x 13).
    const TRUE_WIDTH: usize = 11;

    #[test]
    fn the_message_length_is_the_semiprime() {
        assert_eq!(bits().len(), 11 * 13);
        assert_eq!(ART.len(), 13);
        assert!(ART.iter().all(|row| row.chars().count() == 11));
    }

    #[test]
    fn opening_width_is_unsolved_and_does_not_auto_solve() {
        assert_ne!(Arecibo::width_for(0.0, 0), TRUE_WIDTH);
        assert_eq!(Arecibo::width_for(0.0, 0), Arecibo::width_for(1.0, 0));
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_width() {
        assert_eq!(Arecibo::width_for(f64::NAN, 0), Arecibo::width_for(0.0, 0));
        assert_eq!(
            Arecibo::width_for(f64::INFINITY, 0),
            Arecibo::width_for(0.0, 0)
        );
        assert_eq!(
            Arecibo::width_for(f64::NEG_INFINITY, 42),
            Arecibo::width_for(0.0, 42)
        );
    }

    #[test]
    fn the_true_width_shows_more_of_the_picture_than_a_wrong_one() {
        let mut right = Canvas::new(44, 26);
        let x = (TRUE_WIDTH - MIN_WIDTH) as f64 / (MAX_WIDTH - MIN_WIDTH) as f64;
        Arecibo::new().render_poked(&mut right, 0.0, &[(x, 0.5)]);
        assert!(right.ink_count() > 20);
    }

    #[test]
    fn decoded_message_uses_centered_square_cells_with_visible_boundaries() {
        let width = 220;
        let height = 130;
        let cell = 10;
        let x_cells = grid_cell_pixels(0, 11, width, cell);
        let y_cells = grid_cell_pixels(1, 13, height, cell);
        assert_eq!(x_cells, 56..64);
        assert_eq!(y_cells, 11..19);

        let mut canvas = Canvas::new(width, height);
        let x = (TRUE_WIDTH - MIN_WIDTH) as f64 / (MAX_WIDTH - MIN_WIDTH) as f64;
        Arecibo::new().render_poked(&mut canvas, 0.0, &[(x, 0.5)]);
        let text = canvas.to_text();
        let char_at = |x: usize, y: usize| {
            text.lines()
                .nth(y)
                .and_then(|line| line.chars().nth(x))
                .unwrap_or(' ')
        };

        assert_eq!(char_at(56, 11), '#');
        assert_eq!(char_at(55, 11), ' ');
        assert_eq!(char_at(64, 11), ' ');
        assert_eq!(char_at(0, 11), ' ');
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Arecibo::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [f64::NAN, f64::NEG_INFINITY, -1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
        room.render_poked(&mut canvas, f64::INFINITY, &[(f64::INFINITY, f64::NAN)]);
    }

    #[test]
    fn all_invalid_pokes_render_the_base_frame() {
        let room = Arecibo::new();
        let mut base = Canvas::new(44, 26);
        let mut poked = Canvas::new(44, 26);

        room.render(&mut base, f64::INFINITY);
        room.render_poked(
            &mut poked,
            f64::INFINITY,
            &[(f64::NAN, f64::INFINITY), (f64::NEG_INFINITY, 0.5)],
        );

        assert_eq!(poked.to_text(), base.to_text());
    }

    #[test]
    fn latest_finite_attempt_owns_the_candidate_width() {
        let attempts: Vec<_> = Arecibo::poked_attempts(&[
            (-1.0, 0.0),
            (f64::NAN, 0.5),
            (0.5, f64::INFINITY),
            (0.5, 0.5),
            (2.0, 1.0),
        ])
        .collect();

        assert_eq!(attempts, vec![MAX_WIDTH]);
    }

    #[test]
    fn poked_attempts_use_the_newest_bounded_raw_tail() {
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        let newest: Vec<_> = many[many.len() - MAX_ROOM_POKES..].to_vec();
        many[0] = (1.0, 1.0);

        let expected: Vec<_> = Arecibo::poked_attempts(&newest).collect();
        let actual: Vec<_> = Arecibo::poked_attempts(&many).collect();

        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 1);
    }

    #[test]
    fn nonfinite_pokes_do_not_shift_attempt_identity() {
        let finite = [(0.25, 0.25), (0.75, 0.75)];
        let with_bad_points = [(f64::NAN, 0.0), finite[0], (0.0, f64::INFINITY), finite[1]];

        assert_eq!(
            Arecibo::poked_attempts(&with_bad_points).collect::<Vec<_>>(),
            Arecibo::poked_attempts(&finite).collect::<Vec<_>>()
        );
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.25, 0.5); MAX_ROOM_POKES];
        with_invalid_tail.push((f64::NAN, f64::INFINITY));

        let attempts: Vec<_> = Arecibo::poked_attempts(&with_invalid_tail).collect();

        assert_eq!(attempts, vec![Arecibo::width_from_point(0.25)]);
    }

    #[test]
    fn oversized_poke_slices_render_like_their_newest_bounded_tail() {
        let room = Arecibo::new();
        let discarded_prefix = vec![(1.0, 1.0), (0.9, 0.1), (0.8, 0.2)];
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    (f64::from((i % 7) as u32) + 0.5) / 7.0,
                    (f64::from((i % 5) as u32) + 0.5) / 5.0,
                )
            })
            .collect();
        let mut all = discarded_prefix.clone();
        all.extend_from_slice(&newest);

        let mut expected = Canvas::new(44, 26);
        let mut actual = Canvas::new(44, 26);
        let mut prefix_only = Canvas::new(44, 26);
        room.render_poked(&mut expected, 0.42, &newest);
        room.render_poked(&mut actual, 0.42, &all);
        room.render_poked(&mut prefix_only, 0.42, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(prefix_only.to_text(), expected.to_text());
    }

    #[test]
    fn reveal_explains_the_semiprime() {
        let reveal = Arecibo::new().reveal();
        assert!(reveal.contains("11 times 13"));
        assert!(reveal.contains("up to rotation"));
    }

    #[test]
    fn status_distinguishes_the_readable_orientation_from_its_factor_pair() {
        let wrong = Arecibo::width_status(10);
        assert!(wrong.contains("3 BITS LEFT OVER"));
        let readable = Arecibo::width_status(11);
        assert!(readable.contains("11 x 13"));
        assert!(readable.contains("SIGNAL LOCKED: PI"));
        let rotated_shape = Arecibo::width_status(13);
        assert!(rotated_shape.contains("13 x 11"));
        assert!(rotated_shape.contains("FACTOR PAIR FOUND"));
        assert!(rotated_shape.contains("TRY WIDTH 11"));
        assert!(!rotated_shape.contains("SIGNAL LOCKED"));
    }

    #[test]
    fn new_with_zero_matches_default_and_poked_changes() {
        let r0 = Arecibo::new_with(0);
        let r_def = Arecibo::new();
        let mut a = Canvas::new(44, 26);
        let mut b = Canvas::new(44, 26);
        r0.render(&mut a, 0.42);
        r_def.render(&mut b, 0.42);
        assert_eq!(a.to_text(), b.to_text());
        let mut cp = Canvas::new(44, 26);
        r0.render_poked(&mut cp, 0.42, &[(0.5, 0.5)]);
        assert_ne!(cp.to_text(), a.to_text());
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let r0 = Arecibo::new_with(0);
        let r42 = Arecibo::new_with(42);
        let mut a = Canvas::new(44, 26);
        let mut c = Canvas::new(44, 26);
        r0.render(&mut a, 0.42);
        r42.render(&mut c, 0.42);
        assert_ne!(a.to_text(), c.to_text());
    }
}
