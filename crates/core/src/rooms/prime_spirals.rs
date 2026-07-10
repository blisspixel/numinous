//! Prime Spirals (Ulam): order hiding in the most patternless numbers.
//!
//! Write the whole numbers in a square spiral out from the center and light up
//! the primes. The primes, famously unpredictable, snap into unmistakable
//! diagonal streaks. `t` shifts the starting number. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::Surface;

/// How far `t` shifts the starting number (41 gives Euler's long prime diagonal).
const MAX_START_OFFSET: u64 = 40;
/// Cap on cells walked, so a huge canvas stays bounded (see `.agent` skill S7).
const MAX_CELLS: usize = 200_000;

/// The Prime Spirals room.
#[derive(Debug, Default)]
pub struct PrimeSpirals {
    seed: u64,
}

impl PrimeSpirals {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// The number at the center of the spiral for phase `t`.
    fn start_for(t: f64) -> u64 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        1 + (t * MAX_START_OFFSET as f64).round() as u64
    }

    fn varied_start_for(&self, t: f64) -> u64 {
        let start = Self::start_for(t);
        if self.seed == 0 {
            start
        } else {
            start + 1 + self.seed % 97
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HighlightDiagonals {
    x: i32,
    y: i32,
    diff: i32,
    sum: i32,
}

impl HighlightDiagonals {
    fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            diff: x - y,
            sum: x + y,
        }
    }

    fn contains(self, x: i32, y: i32) -> bool {
        x - y == self.diff || x + y == self.sum
    }
}

fn normalized_cell(value: f64, len: usize) -> i32 {
    ((value.clamp(0.0, 1.0) * len as f64) as usize).min(len.saturating_sub(1)) as i32
}

fn poked_diagonals(pokes: &[(f64, f64)], width: usize, height: usize) -> Vec<HighlightDiagonals> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(px, py)| {
            if px.is_finite() && py.is_finite() {
                Some(HighlightDiagonals::new(
                    normalized_cell(px, width),
                    normalized_cell(py, height),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn walk_spiral(width: usize, height: usize, start: u64, mut visit: impl FnMut(i32, i32, u64)) {
    let cap = width
        .max(height)
        .saturating_mul(width.max(height))
        .min(MAX_CELLS);
    let mut n = start;
    let (mut x, mut y) = ((width / 2) as i32, (height / 2) as i32);
    visit(x, y, n);

    // Ulam spiral: step right, up, left, down, with segment lengths
    // 1, 1, 2, 2, 3, 3, ... walking one cell at a time.
    let dirs = [(1, 0), (0, -1), (-1, 0), (0, 1)];
    let mut dir = 0usize;
    let mut segment = 1u64;
    let mut visited = 1usize;
    'walk: loop {
        for _ in 0..2 {
            for _ in 0..segment {
                if visited >= cap {
                    break 'walk;
                }
                let (dx, dy) = dirs[dir];
                x += dx;
                y += dy;
                n += 1;
                visited += 1;
                visit(x, y, n);
            }
            dir = (dir + 1) % 4;
        }
        segment += 1;
    }
}

fn draw_spiral(
    canvas: &mut dyn Surface,
    width: usize,
    height: usize,
    start: u64,
    highlights: &[HighlightDiagonals],
) {
    walk_spiral(width, height, start, |x, y, n| {
        if is_prime(n) {
            let mark = if highlights.iter().any(|h| h.contains(x, y)) {
                '+'
            } else {
                '*'
            };
            canvas.plot(x, y, mark);
        }
    });
    for highlight in highlights {
        canvas.plot(highlight.x, highlight.y, '#');
    }
}

impl Room for PrimeSpirals {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "prime-spirals",
            title: "Prime Spirals",
            wing: "Number & Pattern",
            blurb: "Write the whole numbers in a spiral and light up the primes; the most \
                    patternless numbers we know snap into diagonal streaks. t shifts the start.",
            accent: [190, 70, 170],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        draw_spiral(canvas, width, height, self.varied_start_for(t), &[]);
    }

    fn reveal(&self) -> &'static str {
        "Primes are famously unpredictable, and a million-dollar prize (the \
         Riemann Hypothesis) rides on how they are spread out. Yet arrange them \
         in a spiral and they line up in diagonal streaks nobody has fully \
         explained. There is a pattern in the most patternless thing we know."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Stanislaw Ulam found this in 1963 by doodling numbers in a grid during a \
             boring conference talk. Some of the best mathematics starts as not \
             paying attention.",
            "Euler's polynomial n squared plus n plus 41 produces primes for every n \
             from 0 to 39, and quadratics like it are exactly why the primes fall \
             into diagonal streaks here. The streaks trace prime-rich quadratic polynomials, and why those are so rich is still open the streaks.",
        ]
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "E prime rays",
            root: 164.81,
            tempo: 126,
            line: &[0, 7, 2, 9, 4, 11, 5, 12],
            encodes: "prime hits leaving diagonal traces through the grid",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: HIGHLIGHT A SPIRAL")
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
        let highlights = poked_diagonals(pokes, width, height);
        if highlights.is_empty() {
            self.render(canvas, t);
            return;
        }
        draw_spiral(canvas, width, height, self.varied_start_for(t), &highlights);
    }
}

/// Return whether `n` is prime, by trial division.
fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    let mut d = 3u64;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{HighlightDiagonals, PrimeSpirals, draw_spiral, is_prime, poked_diagonals};
    use crate::canvas::Canvas;
    use crate::room::MAX_ROOM_POKES;
    use crate::room::Room;

    fn char_at(canvas: &Canvas, x: usize, y: usize) -> char {
        canvas
            .to_text()
            .lines()
            .nth(y)
            .and_then(|line| line.chars().nth(x))
            .unwrap_or(' ')
    }

    fn count_char(canvas: &Canvas, mark: char) -> usize {
        canvas.to_text().chars().filter(|&c| c == mark).count()
    }

    fn has_far_highlight(canvas: &Canvas, x: usize, y: usize) -> bool {
        canvas.to_text().lines().enumerate().any(|(row, line)| {
            line.chars()
                .enumerate()
                .any(|(col, ch)| ch == '+' && row.abs_diff(y).saturating_add(col.abs_diff(x)) > 2)
        })
    }

    fn assert_highlights_are_exact_diagonals(canvas: &Canvas, highlight: HighlightDiagonals) {
        let mut saw_base_prime = false;
        let mut saw_highlight = false;
        for (row, line) in canvas.to_text().lines().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let on_selected = highlight.contains(col as i32, row as i32);
                match ch {
                    '+' => {
                        assert!(on_selected, "unexpected highlight at ({col},{row})");
                        saw_highlight = true;
                    }
                    '*' if !on_selected => saw_base_prime = true,
                    _ => {}
                }
            }
        }
        assert!(saw_highlight, "selected diagonal did not highlight primes");
        assert!(
            saw_base_prime,
            "base primes outside the selected diagonals disappeared"
        );
    }

    #[test]
    fn primality_is_correct() {
        for p in [2, 3, 5, 7, 11, 13, 41, 97, 7919] {
            assert!(is_prime(p), "{p} should be prime");
        }
        for c in [0, 1, 4, 6, 9, 100, 7917] {
            assert!(!is_prime(c), "{c} should not be prime");
        }
    }

    #[test]
    fn start_defaults_to_one() {
        assert_eq!(PrimeSpirals::start_for(0.0), 1);
    }

    #[test]
    fn new_with_zero_matches_default() {
        let mut a = Canvas::new(41, 25);
        let mut b = Canvas::new(41, 25);
        PrimeSpirals::new().render(&mut a, 0.3);
        PrimeSpirals::new_with(0).render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let mut a = Canvas::new(41, 25);
        let mut b = Canvas::new(41, 25);
        PrimeSpirals::new_with(0).render(&mut a, 0.3);
        PrimeSpirals::new_with(42).render(&mut b, 0.3);
        assert_ne!(a.to_text(), b.to_text());

        let mut poked_a = Canvas::new(41, 25);
        let mut poked_b = Canvas::new(41, 25);
        PrimeSpirals::new_with(0).render_poked(&mut poked_a, 0.3, &[(0.4, 0.6)]);
        PrimeSpirals::new_with(42).render_poked(&mut poked_b, 0.3, &[(0.4, 0.6)]);
        assert_ne!(poked_a.to_text(), poked_b.to_text());
    }

    #[test]
    fn render_is_deterministic() {
        let room = PrimeSpirals::new();
        let mut a = Canvas::new(41, 25);
        let mut b = Canvas::new(41, 25);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = PrimeSpirals::new();
        let mut canvas = Canvas::new(41, 25);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = PrimeSpirals::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(7, 7);
        for t in [-2.0, 0.0, 0.999, 3.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_names_the_hypothesis() {
        assert!(PrimeSpirals::new().reveal().contains("Riemann Hypothesis"));
    }

    #[test]
    fn poked_changes_output() {
        let r0 = PrimeSpirals::new();
        let mut a = Canvas::new(41, 25);
        let mut p = Canvas::new(41, 25);
        r0.render(&mut a, 0.0);
        r0.render_poked(&mut p, 0.0, &[(0.5, 0.5)]);
        assert_ne!(p.to_text(), a.to_text());
    }

    #[test]
    fn poked_diagonals_preserve_order_clamp_and_filter() {
        let diagonals = poked_diagonals(
            &[(0.2, 0.3), (f64::NAN, 0.5), (2.0, -1.0), (1.0, 1.0)],
            41,
            25,
        );
        assert_eq!(
            diagonals,
            vec![
                HighlightDiagonals::new(8, 7),
                HighlightDiagonals::new(40, 0),
                HighlightDiagonals::new(40, 24)
            ]
        );
    }

    #[test]
    fn poked_highlights_prime_diagonals_not_just_the_clicked_cell() {
        let room = PrimeSpirals::new();
        let mut actual = Canvas::new(41, 25);
        room.render_poked(&mut actual, 0.0, &[(0.5, 0.5)]);

        assert_eq!(char_at(&actual, 20, 12), '#');
        assert!(count_char(&actual, '+') > 5);
        assert!(has_far_highlight(&actual, 20, 12));
        assert_highlights_are_exact_diagonals(&actual, HighlightDiagonals::new(20, 12));
    }

    #[test]
    fn edge_diagonal_remains_addressable() {
        let room = PrimeSpirals::new();
        let mut actual = Canvas::new(41, 25);
        room.render_poked(&mut actual, 0.0, &[(1.0, 1.0)]);
        assert_eq!(char_at(&actual, 40, 24), '#');
        assert!(has_far_highlight(&actual, 40, 24));
        assert_highlights_are_exact_diagonals(&actual, HighlightDiagonals::new(40, 24));
    }

    #[test]
    fn poked_diagonals_use_the_newest_bounded_finite_points() {
        let room = PrimeSpirals::new();
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 1.0))
            .collect();
        let mut old: Vec<_> = (0..MAX_ROOM_POKES + 11)
            .map(|i| (((i as f64) + 0.5) / (MAX_ROOM_POKES + 11) as f64, 0.0))
            .collect();
        old.extend(newest.clone());

        let mut expected = Canvas::new(41, 25);
        let mut actual = Canvas::new(41, 25);
        room.render_poked(&mut expected, 0.0, &newest);
        room.render_poked(&mut actual, 0.0, &old);
        assert_eq!(actual.to_text(), expected.to_text());

        let all_highlights: Vec<_> = old
            .iter()
            .filter_map(|&(px, py)| {
                if px.is_finite() && py.is_finite() {
                    Some(HighlightDiagonals::new(
                        ((px.clamp(0.0, 1.0) * 41.0) as usize).min(40) as i32,
                        ((py.clamp(0.0, 1.0) * 25.0) as usize).min(24) as i32,
                    ))
                } else {
                    None
                }
            })
            .collect();
        let mut uncapped = Canvas::new(41, 25);
        draw_spiral(
            &mut uncapped,
            41,
            25,
            PrimeSpirals::start_for(0.0),
            &all_highlights,
        );
        assert_ne!(uncapped.to_text(), expected.to_text());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_diagonal_identity() {
        let room = PrimeSpirals::new();
        let finite = [(0.4, 0.6)];
        let with_bad_points = [
            (f64::NAN, 0.1),
            (f64::INFINITY, 0.2),
            finite[0],
            (0.3, f64::NEG_INFINITY),
        ];
        let mut expected = Canvas::new(41, 25);
        let mut actual = Canvas::new(41, 25);
        room.render_poked(&mut expected, 0.0, &finite);
        room.render_poked(&mut actual, 0.0, &with_bad_points);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = PrimeSpirals::new();
        let mut with_invalid_tail = vec![(0.4, 0.0); MAX_ROOM_POKES - 1];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        let mut expected = Canvas::new(41, 25);
        let mut actual = Canvas::new(41, 25);
        room.render(&mut expected, 0.0);
        room.render_poked(&mut actual, 0.0, &with_invalid_tail);
        assert_eq!(actual.to_text(), expected.to_text());

        let filter_first_highlights =
            poked_diagonals(&with_invalid_tail[..MAX_ROOM_POKES - 1], 41, 25);
        let mut filter_first = Canvas::new(41, 25);
        draw_spiral(
            &mut filter_first,
            41,
            25,
            PrimeSpirals::start_for(0.0),
            &filter_first_highlights,
        );
        assert_ne!(filter_first.to_text(), expected.to_text());
    }
}
