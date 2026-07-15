//! Cellular Automata: elementary (Wolfram) rules on a line.
//!
//! A single row of cells evolves generation by generation, each cell's next
//! state decided only by itself and its two neighbors via an 8-bit rule number.
//! The space-time history is drawn top to bottom. Rule 90 draws a Sierpinski
//! triangle; Rule 30 pours out chaos; Rule 110 is Turing-complete. See
//! `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::Surface;

/// A curated tour of notable elementary rules, indexed by phase `t`.
///
/// The full 0..=255 dial belongs to the interactive GPU version; the headless
/// face sweeps these standouts so every frame is worth looking at.
const NOTABLE_RULES: [u8; 8] = [90, 30, 110, 54, 150, 18, 60, 105];

/// The Cellular Automata room.
#[derive(Debug, Default)]
pub struct CellularAutomata {
    seed: u64,
}

impl CellularAutomata {
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

    /// The elementary rule selected by phase `t` (clamped into range).
    fn rule_for(t: f64, seed: u64) -> u8 {
        let t = if t.is_finite() {
            t.clamp(0.0, 0.999)
        } else {
            0.0
        };
        let span = NOTABLE_RULES.len();
        let index = (t * span as f64) as usize;
        let base = NOTABLE_RULES[index.min(span - 1)];
        if seed == 0 {
            base
        } else {
            // Nonzero seeds must visibly re-deal while seed 0 keeps exact postcards.
            let offset = (seed as usize % (span - 1)) + 1;
            NOTABLE_RULES[(index + offset) % span]
        }
    }
}

fn normalized_index(value: f64, len: usize) -> usize {
    ((value.clamp(0.0, 1.0) * len as f64) as usize).min(len.saturating_sub(1))
}

fn poked_cells(
    pokes: &[(f64, f64)],
    width: usize,
    height: usize,
) -> impl Iterator<Item = (usize, usize)> + '_ {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..].iter().filter_map(move |&(px, py)| {
        if px.is_finite() && py.is_finite() {
            Some((normalized_index(px, width), normalized_index(py, height)))
        } else {
            None
        }
    })
}

fn render_history(
    canvas: &mut dyn Surface,
    width: usize,
    height: usize,
    rule: u8,
    events: &[(usize, usize)],
) {
    let mut cells = vec![false; width];
    cells[width / 2] = true;
    for y in 0..height {
        for &(x, event_y) in events {
            if event_y == y {
                cells[x] = !cells[x];
            }
        }
        for (x, &alive) in cells.iter().enumerate() {
            if alive {
                canvas.plot(x as i32, y as i32, '*');
            }
        }
        cells = next_generation(&cells, rule);
    }
}

impl Room for CellularAutomata {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cellular-automata",
            title: "Cellular Automata",
            wing: "Emergence",
            blurb: "One line of cells and one tiny rule per cell; sweep t across notable rules, \
                    where Rule 90 draws a Sierpinski triangle and Rule 30 pours out chaos.",
            accent: [70, 200, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        render_history(canvas, width, height, Self::rule_for(t, self.seed), &[]);
    }

    fn reveal(&self) -> &'static str {
        "Rule 110 is as powerful as any computer ever built, and Rule 30's chaos \
         was good enough to ship as a random number generator. A one-line rule \
         with the power of a universe."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "D minor machine",
            root: 146.83,
            tempo: 128,
            line: &[0, 12, 1, 11, 2, 10, 3, 9],
            encodes: "one row rewriting itself into a whole time field",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: FLIP A CELL")
    }

    fn status(&self, t: f64) -> Option<String> {
        let rule = Self::rule_for(t, self.seed);
        Some(format!(
            "RULE {rule}   ONE ROW REWRITES ITSELF   CLICK: FLIP A SEED CELL"
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let events: Vec<_> = poked_cells(pokes, width, height).collect();
        if events.is_empty() {
            self.render(canvas, t);
            return;
        }
        render_history(canvas, width, height, Self::rule_for(t, self.seed), &events);
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Rule 30's chaos was good enough that Mathematica shipped it as a random \
             number generator for years. A one-line rule out-randomed human \
             engineering.",
            "Cambridge North railway station is clad in panels patterned by Rule 30. \
             Thousands of commuters walk past a Turing-adjacent computation every \
             morning and think it is decoration.",
        ]
    }
}

/// Advance an elementary cellular automaton by one generation.
///
/// Cells outside the row are treated as dead. Each cell's next state is bit
/// `(left, center, right)` of the 8-bit `rule`.
fn next_generation(cells: &[bool], rule: u8) -> Vec<bool> {
    let n = cells.len();
    (0..n)
        .map(|i| {
            let left = i > 0 && cells[i - 1];
            let center = cells[i];
            let right = i + 1 < n && cells[i + 1];
            let pattern =
                (usize::from(left) << 2) | (usize::from(center) << 1) | usize::from(right);
            (rule >> pattern) & 1 == 1
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CellularAutomata, next_generation, poked_cells, render_history};
    use crate::canvas::Canvas;
    use crate::room::MAX_ROOM_POKES;
    use crate::room::Room;
    use crate::surface::Surface;

    fn char_at(canvas: &Canvas, x: usize, y: usize) -> char {
        canvas
            .to_text()
            .lines()
            .nth(y)
            .and_then(|line| line.chars().nth(x))
            .unwrap_or(' ')
    }

    #[test]
    fn rule_for_starts_at_rule_90() {
        assert_eq!(CellularAutomata::rule_for(0.0, 0), 90);
    }

    #[test]
    fn rule_90_is_left_xor_right() {
        let start = [false, false, true, false, false];
        let next = next_generation(&start, 90);
        assert_eq!(next, vec![false, true, false, true, false]);
    }

    #[test]
    fn render_is_deterministic() {
        let room = CellularAutomata::new();
        let mut a = Canvas::new(41, 21);
        let mut b = Canvas::new(41, 21);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = CellularAutomata::new();
        let mut canvas = Canvas::new(41, 21);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 1);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = CellularAutomata::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(10, 10);
        for t in [-1.0, 0.0, 0.999, 2.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::INFINITY, f64::NAN)]);
        }
    }

    #[test]
    fn reveal_mentions_rule_110() {
        assert!(CellularAutomata::new().reveal().contains("Rule 110"));
    }

    #[test]
    fn new_with_zero_matches_default_and_poked_changes() {
        let r0 = CellularAutomata::new_with(0);
        let r_def = CellularAutomata::new();
        let mut a = Canvas::new(41, 21);
        let mut b = Canvas::new(41, 21);
        r0.render(&mut a, 0.0);
        r_def.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        let mut cp = Canvas::new(41, 21);
        r0.render_poked(&mut cp, 0.0, &[(0.5, 0.5)]);
        assert_ne!(cp.to_text(), a.to_text());
    }

    #[test]
    fn poked_cells_preserve_order_clamp_and_filter() {
        let cells: Vec<_> = poked_cells(
            &[(0.2, 0.3), (f64::NAN, 0.5), (2.0, -1.0), (1.0, 1.0)],
            41,
            21,
        )
        .collect();
        assert_eq!(cells, vec![(8, 6), (40, 0), (40, 20)]);
    }

    #[test]
    fn poked_cell_is_part_of_the_automaton_history() {
        let room = CellularAutomata::new();
        let mut actual = Canvas::new(41, 21);
        room.render_poked(&mut actual, 0.0, &[(0.5, 0.0)]);

        let mut expected = Canvas::new(41, 21);
        render_history(
            &mut expected,
            41,
            21,
            CellularAutomata::rule_for(0.0, 0),
            &[(20, 0)],
        );
        assert_eq!(actual.to_text(), expected.to_text());
        assert_eq!(char_at(&actual, 20, 0), ' ');
        assert_eq!(char_at(&actual, 19, 1), ' ');
        assert_eq!(char_at(&actual, 20, 1), ' ');
        assert_eq!(char_at(&actual, 21, 1), ' ');

        let mut post_overlay = Canvas::new(41, 21);
        room.render(&mut post_overlay, 0.0);
        post_overlay.plot(20, 0, '*');
        assert_ne!(actual.to_text(), post_overlay.to_text());
    }

    #[test]
    fn duplicate_flips_are_replayed_not_deduplicated() {
        let room = CellularAutomata::new();
        let mut duplicate = Canvas::new(41, 21);
        let mut base = Canvas::new(41, 21);
        room.render_poked(&mut duplicate, 0.0, &[(0.5, 0.0), (0.5, 0.0)]);
        room.render(&mut base, 0.0);
        assert_eq!(duplicate.to_text(), base.to_text());
    }

    #[test]
    fn edge_cell_remains_addressable() {
        let room = CellularAutomata::new();
        let mut actual = Canvas::new(41, 21);
        let mut expected = Canvas::new(41, 21);
        room.render_poked(&mut actual, 0.0, &[(1.0, 1.0)]);
        render_history(
            &mut expected,
            41,
            21,
            CellularAutomata::rule_for(0.0, 0),
            &[(40, 20)],
        );
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn poked_phase_normalization_matches_base_rules() {
        let room = CellularAutomata::new();
        for (t, expected_t) in [
            (-1.0, 0.0),
            (0.0, 0.0),
            (0.999, 0.999),
            (1.0, 0.999),
            (2.0, 0.999),
            (f64::NAN, 0.0),
            (f64::INFINITY, 0.0),
        ] {
            let mut actual = Canvas::new(41, 21);
            let mut expected = Canvas::new(41, 21);
            room.render_poked(&mut actual, t, &[(0.25, 0.25)]);
            render_history(
                &mut expected,
                41,
                21,
                CellularAutomata::rule_for(expected_t, 0),
                &[(10, 5)],
            );
            assert_eq!(actual.to_text(), expected.to_text(), "t={t:?}");
        }
    }

    #[test]
    fn poked_cells_use_the_newest_bounded_finite_points() {
        let room = CellularAutomata::new();
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 0.0))
            .collect();
        let mut old = vec![(0.9, 0.0); MAX_ROOM_POKES + 11];
        old.extend(newest.clone());

        let mut expected = Canvas::new(41, 21);
        let mut actual = Canvas::new(41, 21);
        room.render_poked(&mut expected, 0.0, &newest);
        room.render_poked(&mut actual, 0.0, &old);
        assert_eq!(actual.to_text(), expected.to_text());

        let all_events: Vec<_> = old
            .iter()
            .filter_map(|&(px, py)| {
                if px.is_finite() && py.is_finite() {
                    Some((
                        ((px.clamp(0.0, 1.0) * 41.0) as usize).min(40),
                        ((py.clamp(0.0, 1.0) * 21.0) as usize).min(20),
                    ))
                } else {
                    None
                }
            })
            .collect();
        let mut uncapped = Canvas::new(41, 21);
        render_history(
            &mut uncapped,
            41,
            21,
            CellularAutomata::rule_for(0.0, 0),
            &all_events,
        );
        assert_ne!(uncapped.to_text(), expected.to_text());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_cell_identity() {
        let room = CellularAutomata::new();
        let finite = [(0.4, 0.6)];
        let with_bad_points = [
            (f64::NAN, 0.1),
            (f64::INFINITY, 0.2),
            finite[0],
            (0.3, f64::NEG_INFINITY),
        ];
        let mut expected = Canvas::new(41, 21);
        let mut actual = Canvas::new(41, 21);
        room.render_poked(&mut expected, 0.0, &finite);
        room.render_poked(&mut actual, 0.0, &with_bad_points);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = CellularAutomata::new();
        let mut with_invalid_tail = vec![(0.4, 0.0); MAX_ROOM_POKES - 1];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        let mut expected = Canvas::new(41, 21);
        let mut actual = Canvas::new(41, 21);
        room.render(&mut expected, 0.0);
        room.render_poked(&mut actual, 0.0, &with_invalid_tail);
        assert_eq!(actual.to_text(), expected.to_text());

        let filtered_events: Vec<_> =
            poked_cells(&with_invalid_tail[..MAX_ROOM_POKES - 1], 41, 21).collect();
        let mut filter_first = Canvas::new(41, 21);
        render_history(
            &mut filter_first,
            41,
            21,
            CellularAutomata::rule_for(0.0, 0),
            &filtered_events,
        );
        assert_ne!(filter_first.to_text(), expected.to_text());
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let r0 = CellularAutomata::new_with(0);
        let r42 = CellularAutomata::new_with(42);
        let mut a = Canvas::new(41, 21);
        let mut c = Canvas::new(41, 21);
        r0.render(&mut a, 0.0);
        r42.render(&mut c, 0.0);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn a_hostile_huge_surface_renders_bounded_without_panicking() {
        use crate::surface::Surface;
        struct HugeSurface;
        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                usize::MAX
            }
            fn height(&self) -> usize {
                usize::MAX
            }
            fn plot(&mut self, _x: i32, _y: i32, _ch: char) {}
        }
        // `vec![false; width]` on a raw usize::MAX width used to overflow the
        // allocation; draw_bounds clamps it to MAX_DIM, as the Room contract asks.
        let mut surface = HugeSurface;
        CellularAutomata::new().render(&mut surface, 0.5);
    }
}
