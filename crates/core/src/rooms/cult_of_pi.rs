//! Cult of Pi: approximation and corruption as code art.
//!
//! Exact decimal digits enter a low-flicker green channel. Every finite prefix
//! approaches pi without becoming its infinite expansion. Deterministic faults
//! alter the finite display, while each retained hand holds one bounded patch
//! exact. See `docs/ROOMS.md`.

use std::sync::OnceLock;

use crate::font::draw_text;
use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta, renderable_poke_count};
use crate::sound::SoundSpec;
use crate::surface::Surface;

const MAX_FIELD_CELLS: usize = 2048;
const GENERATED_DIGITS: usize = MAX_FIELD_CELLS * 2;
const PHASE_TICKS: usize = 64;
const SEED_SALT: u64 = 0x3141_5926_5358_9793;
const PI_HEADER: &str = "PI = 3.141592653589793...";
const DIGIT_TEXT: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

static PI_DIGITS: OnceLock<Vec<u8>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct FieldLayout {
    columns: usize,
    rows: usize,
    step_x: usize,
    step_y: usize,
    glyph_scale: i32,
    pixel_font: bool,
    origin_y: usize,
    surface_width: usize,
    surface_height: usize,
}

impl FieldLayout {
    fn cells(self) -> usize {
        self.columns.saturating_mul(self.rows)
    }
}

fn finite_phase(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Generate exact decimal digits with the bounded Rabinowitz-Wagon spigot.
fn generate_pi_digits(count: usize) -> Vec<u8> {
    if count == 0 {
        return Vec::new();
    }
    let boxes = count.saturating_mul(10) / 3 + 1;
    let mut remainders = vec![2_u64; boxes];
    let mut held_nines = 0_usize;
    let mut previous = 0_u64;
    let mut output = Vec::with_capacity(count + 1);

    for _ in 0..count {
        let mut carry = 0_u64;
        for index in (1..=boxes).rev() {
            let i = index as u64;
            let value = 10 * remainders[index - 1] + carry * i;
            let divisor = 2 * i - 1;
            remainders[index - 1] = value % divisor;
            carry = value / divisor;
        }
        remainders[0] = carry % 10;
        match carry / 10 {
            9 => held_nines += 1,
            10 => {
                output.push((previous + 1) as u8);
                output.extend(std::iter::repeat_n(0, held_nines));
                previous = 0;
                held_nines = 0;
            }
            digit => {
                output.push(previous as u8);
                previous = digit;
                output.extend(std::iter::repeat_n(9, held_nines));
                held_nines = 0;
            }
        }
    }
    output.push(previous as u8);
    output.into_iter().skip(1).take(count).collect()
}

fn digits() -> &'static [u8] {
    PI_DIGITS.get_or_init(|| generate_pi_digits(GENERATED_DIGITS))
}

fn layout(surface: &dyn Surface) -> Option<FieldLayout> {
    let (width, height) = surface.draw_bounds();
    if width == 0 || height == 0 {
        return None;
    }
    let pixel_font = surface.safe_char_aspect() >= 0.75;
    let glyph_scale = if pixel_font {
        (width.min(height) as i32 / 300).clamp(1, 3)
    } else {
        1
    };
    let base_x = if pixel_font {
        6 * glyph_scale as usize
    } else {
        1
    };
    let base_y = if pixel_font {
        8 * glyph_scale as usize
    } else {
        1
    };
    let interface_scale = (width as i32 / 400).clamp(1, 4);
    let header_y = if pixel_font {
        18 + 7 * (interface_scale + 1)
    } else {
        0
    } as usize;
    let origin_y = if pixel_font {
        header_y
            .saturating_add(8 * glyph_scale as usize)
            .saturating_add(4)
    } else {
        8
    };
    let available_height = height.saturating_sub(origin_y).max(1);
    let mut spacing = 1_usize;
    loop {
        let step_x = base_x.saturating_mul(spacing).max(1);
        let step_y = base_y.saturating_mul(spacing).max(1);
        let columns = width.div_ceil(step_x);
        let rows = available_height.div_ceil(step_y);
        if columns.saturating_mul(rows) <= MAX_FIELD_CELLS {
            return Some(FieldLayout {
                columns,
                rows,
                step_x,
                step_y,
                glyph_scale,
                pixel_font,
                origin_y,
                surface_width: width,
                surface_height: height,
            });
        }
        spacing += 1;
    }
}

fn cell_hash(seed: u64, tick: usize, index: usize) -> u64 {
    let mut rng = SplitMix64::new(
        SEED_SALT ^ seed.rotate_left(17) ^ (tick as u64).wrapping_mul(0x9E37_79B9) ^ index as u64,
    );
    rng.next_u64()
}

fn hand_points(pokes: &[(f64, f64)]) -> impl Iterator<Item = (f64, f64)> + '_ {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..].iter().filter_map(|&(x, y)| {
        if x.is_finite() && y.is_finite() {
            Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        } else {
            None
        }
    })
}

fn near_repair(column: usize, row: usize, field: FieldLayout, pokes: &[(f64, f64)]) -> bool {
    let x = ((column as f64 + 0.5) * field.step_x as f64) / field.surface_width as f64;
    let y = (field.origin_y as f64 + (row as f64 + 0.5) * field.step_y as f64)
        / field.surface_height as f64;
    hand_points(pokes).any(|(px, py)| (x - px).hypot(y - py) < 0.16)
}

/// Fixed analysis grid so status can grade placement without a canvas. The
/// geometry matches the normalized hand radius used by [`near_repair`].
fn analysis_field() -> FieldLayout {
    FieldLayout {
        columns: 32,
        rows: 16,
        step_x: 1,
        step_y: 1,
        glyph_scale: 1,
        pixel_font: false,
        origin_y: 0,
        surface_width: 32,
        surface_height: 16,
    }
}

fn newest_hand(inputs: &[RoomInput]) -> Option<(f64, f64)> {
    inputs.iter().rev().find_map(|input| match *input {
        RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. }
            if x.is_finite() && y.is_finite() =>
        {
            Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        }
        _ => None,
    })
}

/// Count display faults inside the newest hold radius, and name the exact
/// digit under the finger. Placement is a decision: hold a faulted patch to
/// save digits, or hold a clean patch and see FIX 0.
fn newest_patch_decision(seed: u64, t: f64, hand: (f64, f64)) -> (usize, u8) {
    let field = analysis_field();
    let phase = finite_phase(t);
    let tick = (phase * (PHASE_TICKS - 1) as f64).floor() as usize;
    let pokes = [hand];
    let mut saved = 0usize;
    let mut nearest_digit = digits()[0];
    let mut nearest_dist = f64::INFINITY;
    for index in 0..field.cells() {
        let column = index % field.columns;
        let row = index / field.columns;
        let cx = ((column as f64 + 0.5) * field.step_x as f64) / field.surface_width as f64;
        let cy = (field.origin_y as f64 + (row as f64 + 0.5) * field.step_y as f64)
            / field.surface_height as f64;
        let dist = (cx - hand.0).hypot(cy - hand.1);
        if dist < nearest_dist {
            nearest_dist = dist;
            nearest_digit = digits()[index % digits().len()];
        }
        if !near_repair(column, row, field, &pokes) {
            continue;
        }
        let hash = cell_hash(seed, tick, index);
        let threshold = phase * 0.42;
        let faulted = (hash % 10_000) as f64 / 10_000.0 < threshold;
        if faulted {
            saved = saved.saturating_add(1);
        }
    }
    (saved, nearest_digit)
}

fn glyph_for(
    exact: u8,
    _index: usize,
    _total: usize,
    hash: u64,
    corruption: f64,
    local_repair: bool,
) -> Option<(char, char)> {
    let threshold = corruption * 0.42;
    let changed = (hash % 10_000) as f64 / 10_000.0 < threshold;
    let shown = if changed && !local_repair {
        (exact + 1 + (hash % 9) as u8) % 10
    } else {
        exact
    };
    let mark = if local_repair {
        '#'
    } else if changed {
        '!'
    } else {
        '.'
    };
    Some((char::from(b'0' + shown), mark))
}

fn draw_hold_markers(surface: &mut dyn Surface, field: FieldLayout, pokes: &[(f64, f64)]) {
    let max_x = field.surface_width.saturating_sub(1) as i32;
    let min_y = field.origin_y.min(field.surface_height.saturating_sub(1)) as i32;
    let max_y = field.surface_height.saturating_sub(1) as i32;
    let radius_x = (field.surface_width as f64 * 0.16).round().max(1.0) as i32;
    let radius_y = (field.surface_height as f64 * 0.16).round().max(1.0) as i32;

    for (px, py) in hand_points(pokes) {
        let center_x = (px * max_x as f64).round() as i32;
        let center_y = (py * max_y as f64).round() as i32;
        let top = (center_x, (center_y - radius_y).clamp(min_y, max_y));
        let right = (
            (center_x + radius_x).clamp(0, max_x),
            center_y.clamp(min_y, max_y),
        );
        let bottom = (center_x, (center_y + radius_y).clamp(min_y, max_y));
        let left = (
            (center_x - radius_x).clamp(0, max_x),
            center_y.clamp(min_y, max_y),
        );
        surface.line(top.0, top.1, right.0, right.1, '+');
        surface.line(right.0, right.1, bottom.0, bottom.1, '+');
        surface.line(bottom.0, bottom.1, left.0, left.1, '+');
        surface.line(left.0, left.1, top.0, top.1, '+');
    }
}

fn render_field(surface: &mut dyn Surface, seed: u64, t: f64, pokes: &[(f64, f64)]) {
    let Some(field) = layout(surface) else {
        return;
    };
    let phase = finite_phase(t);
    let tick = (phase * (PHASE_TICKS - 1) as f64).floor() as usize;
    let total = field.cells();

    for index in 0..total {
        let column = index % field.columns;
        let row = index / field.columns;
        let exact = digits()[index];
        let hash = cell_hash(seed, tick, index);
        let local_repair = near_repair(column, row, field, pokes);
        let Some((glyph, mark)) = glyph_for(exact, index, total, hash, phase, local_repair) else {
            continue;
        };
        let x = column * field.step_x;
        let y = field.origin_y + row * field.step_y;
        if field.pixel_font {
            let text = DIGIT_TEXT[(glyph as u8 - b'0') as usize];
            draw_text(surface, text, x as i32, y as i32, field.glyph_scale, mark);
        } else {
            surface.plot(x as i32, y as i32, glyph);
        }
    }

    draw_hold_markers(surface, field, pokes);

    let header_scale = if field.pixel_font {
        field.glyph_scale.max(1)
    } else {
        1
    };
    let header_y = if field.pixel_font {
        let interface_scale = (surface.width() as i32 / 400).clamp(1, 4);
        18 + 7 * (interface_scale + 1)
    } else {
        0
    };
    draw_text(surface, PI_HEADER, 0, header_y, header_scale, '#');
}

/// A ritualized, decaying computation of pi.
#[derive(Debug, Default)]
pub struct CultOfPi {
    seed: u64,
}

impl CultOfPi {
    /// Create the canonical room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create a replayable variation.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for CultOfPi {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cult-of-pi",
            title: "Cult of Pi",
            wing: "Number & Pattern",
            blurb: "The exact digits of pi enter a finite channel, age, and develop faults. Click to restore and hold one local patch exact, but no finite screen can ever contain all of pi.",
            accent: [40, 210, 90],
        }
    }

    fn render(&self, surface: &mut dyn Surface, t: f64) {
        render_field(surface, self.seed, t, &[]);
    }

    fn reveal(&self) -> &'static str {
        "Every exact prefix truncated after n decimal places differs from pi by less than 10 to the negative n, but pi's expansion never ends. The display faults are ours, not pi's. A finite process can approach an exact limit forever without any finite frame becoming the whole thing."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Pythagorean communities joined number, musical harmony, and a disciplined way of life. Later tradition made them more uniformly secret and mystical than the early evidence allows, which is why this room borrows the ritual atmosphere without presenting legend as fact.",
            "The famous story says Hippasus was drowned for disclosing irrationality. No ancient source actually connects him to that discovery, so it is a powerful legend about mathematical shock, not established history.",
            "Six nines begin at decimal position 762, the Feynman point. Strange clusters are inevitable in a long random-looking sequence; surprise alone is not evidence of a message.",
        ]
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C decimal procession",
            root: 130.81,
            tempo: 70,
            line: &[3, 1, 4, 1, 5, 9, 2, 6, 5, 3],
            encodes: "the opening decimal digits of pi, one interval per digit",
        })
    }

    fn status(&self, t: f64) -> Option<String> {
        let phase = finite_phase(t);
        let tick = (phase * (PHASE_TICKS - 1) as f64).floor() as usize + 1;
        Some(format!(
            "CH{tick:02}/{PHASE_TICKS}  FAULT {:.0}%  HOLD:FIX",
            phase * 42.0
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let repairs = renderable_poke_count(inputs);
        if repairs == 0 {
            return self.status(t);
        }
        let phase = finite_phase(t);
        let tick = (phase * (PHASE_TICKS - 1) as f64).floor() as usize + 1;
        let retained = inputs
            .iter()
            .filter(|input| {
                matches!(
                    input,
                    RoomInput::PointerDown { .. } | RoomInput::PointerMove { .. }
                )
            })
            .count()
            > MAX_ROOM_POKES;
        let (saved, digit) = newest_hand(inputs)
            .map(|hand| newest_patch_decision(self.seed, t, hand))
            .unwrap_or((0, 0));
        // Placement decision: FIX names faults restored under the newest hold,
        // D names the exact digit at the finger. Compact enough for the footer.
        let kind = if retained { "RECENT" } else { "HELD" };
        Some(format!("{repairs} {kind} FIX{saved} D{digit} CH{tick:02}"))
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: RESTORE AND HOLD A PATCH")
    }

    fn render_poked(&self, surface: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        render_field(surface, self.seed, t, pokes);
    }

    fn sound(&self, t: f64) -> SoundSpec {
        let phase = finite_phase(t) as f32;
        let root = 130.81_f32;
        let frequencies: Vec<f32> = digits()[..10]
            .iter()
            .enumerate()
            .map(|(index, &digit)| {
                let drift = phase * ((index as f32 * 1.7).sin()) * 0.8;
                root * 2.0_f32.powf((digit as f32 + drift) / 12.0)
            })
            .collect();
        SoundSpec::arpeggio(&frequencies, 4.0, 0.18)
    }

    fn postcard_t(&self) -> f64 {
        0.42
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CultOfPi, MAX_FIELD_CELLS, PI_HEADER, digits, generate_pi_digits, glyph_for, layout,
        near_repair, newest_patch_decision,
    };
    use crate::MAX_ROOM_POKES;
    use crate::canvas::Canvas;
    use crate::room::Room;
    use crate::surface::Surface;

    #[test]
    fn spigot_starts_with_exact_pi_digits() {
        let text: String = generate_pi_digits(51)
            .into_iter()
            .map(|digit| char::from(b'0' + digit))
            .collect();
        assert_eq!(text, "314159265358979323846264338327950288419716939937510");
    }

    #[test]
    fn rendering_is_deterministic_and_degrades_with_phase() {
        let room = CultOfPi::new();
        let mut a = Canvas::new(60, 30);
        let mut b = Canvas::new(60, 30);
        let mut later = Canvas::new(60, 30);
        room.render(&mut a, 0.2);
        room.render(&mut b, 0.2);
        room.render(&mut later, 0.8);

        assert_eq!(a.to_text(), b.to_text());
        assert_ne!(a.to_text(), later.to_text());
        assert!(a.ink_count() > 400);
    }

    #[test]
    fn a_click_repairs_a_local_patch() {
        let room = CultOfPi::new();
        let mut base = Canvas::new(60, 30);
        let mut repaired = Canvas::new(60, 30);
        room.render(&mut base, 0.4);
        room.render_poked(&mut repaired, 0.4, &[(0.5, 0.5)]);

        assert_ne!(base.to_text(), repaired.to_text());

        let hash = (1..100_000)
            .find(|&hash| {
                glyph_for(3, 9, 10, hash, 1.0, false)
                    .is_some_and(|(glyph, _)| glyph.is_ascii_digit() && glyph != '3')
            })
            .expect("a deterministic corrupted digit");
        let (wrong, mark) = glyph_for(3, 9, 10, hash, 1.0, false).expect("corrupted glyph");
        assert_ne!(wrong, '3');
        assert_eq!(mark, '!', "faults must look different from exact holds");
        assert_eq!(glyph_for(3, 9, 10, hash, 1.0, true), Some(('3', '#')));

        let canvas = Canvas::new(60, 30);
        let field = layout(&canvas).expect("field");
        let column = field.columns / 2;
        let row = field.rows / 2;
        let hand = (
            ((column as f64 + 0.5) * field.step_x as f64) / field.surface_width as f64,
            (field.origin_y as f64 + (row as f64 + 0.5) * field.step_y as f64)
                / field.surface_height as f64,
        );
        assert!(near_repair(column, row, field, &[hand]));
        assert!(!near_repair(0, 0, field, &[hand]));
    }

    #[test]
    fn a_phase_zero_touch_truthfully_holds_a_patch_exact() {
        let room = CultOfPi::new();
        let inputs = crate::inputs_from_pokes(&[(0.5, 0.5)], 0.0);
        let mut base = Canvas::new(50, 24);
        let mut held = Canvas::new(50, 24);
        room.render(&mut base, 0.0);
        room.render_input(&mut held, 0.0, &inputs);

        let status = room.status_input(0.0, &inputs).expect("held status");
        assert!(status.starts_with("1 HELD FIX"));
        assert!(status.contains(" D"));
        assert!(status.contains(" CH01"));
        // Phase zero has no display faults: placement still reports FIX0.
        assert!(status.contains("FIX0"), "{status}");
        assert_ne!(base.to_text(), held.to_text());
        assert!(
            base.delta(&held)
                .is_some_and(|delta| delta.cells_changed > 0),
            "the held-patch boundary must be visible without color"
        );
    }

    #[test]
    fn status_names_when_only_the_recent_patch_window_is_held() {
        let room = CultOfPi::new();
        let inputs: Vec<crate::RoomInput> = (0..MAX_ROOM_POKES + 1)
            .map(|index| crate::RoomInput::PointerMove {
                x: index as f64 / MAX_ROOM_POKES as f64,
                y: 0.5,
                t: 0.5,
            })
            .collect();

        let status = room.status_input(0.5, &inputs).expect("recent status");
        assert!(status.starts_with("24 RECENT FIX"));
        assert!(status.contains(" CH32"));
        assert!(status.chars().count() <= 30, "{status}");
    }

    #[test]
    fn delayed_hold_status_grades_placement_against_faults() {
        let room = CultOfPi::new();
        let hands = [(0.26, 0.34), (0.54, 0.48), (0.76, 0.66)];
        let inputs = crate::inputs_from_pokes(&hands, 0.4);

        let status = room.status_input(0.4, &inputs).expect("hold status");
        assert!(status.starts_with("3 HELD FIX"));
        assert!(status.contains(" D"));
        assert!(status.contains(" CH26"));
        let (saved, digit) = newest_patch_decision(0, 0.4, *hands.last().expect("newest hand"));
        assert!(status.contains(&format!("FIX{saved}")));
        assert!(status.contains(&format!("D{digit}")));
        // A second placement at a different site can change the grade: the
        // status is about the newest decision, not a cumulative bag.
        let other = crate::inputs_from_pokes(&[(0.1, 0.1)], 0.4);
        let other_status = room.status_input(0.4, &other).expect("other");
        assert!(other_status.starts_with("1 HELD FIX"));
        assert_ne!(status, other_status);
    }

    #[test]
    fn raster_repairs_replace_wrong_glyphs_instead_of_overdrawing_them() {
        let room = CultOfPi::new();
        let pokes = [(0.5, 0.5)];
        let mut opening = crate::Raster::with_accent(640, 480, room.meta().accent);
        let mut degraded = crate::Raster::with_accent(640, 480, room.meta().accent);
        room.render_poked(&mut opening, 0.0, &pokes);
        room.render_poked(&mut degraded, 1.0, &pokes);

        let field = layout(&opening).expect("field");
        let opening_rgba = opening.to_rgba();
        let degraded_rgba = degraded.to_rgba();
        let mut compared_cells = 0;
        for index in 0..field.cells() {
            let column = index % field.columns;
            let row = index / field.columns;
            if !near_repair(column, row, field, &pokes) {
                continue;
            }
            compared_cells += 1;
            for y in field.origin_y + row * field.step_y
                ..(field.origin_y + (row + 1) * field.step_y).min(field.surface_height)
            {
                for x in
                    column * field.step_x..((column + 1) * field.step_x).min(field.surface_width)
                {
                    let offset = (y * field.surface_width + x) * 4;
                    assert_eq!(
                        &degraded_rgba[offset..offset + 4],
                        &opening_rgba[offset..offset + 4],
                        "held cell {column},{row} differs at pixel {x},{y}"
                    );
                }
            }
        }
        assert!(compared_cells > 0);
        assert_ne!(degraded_rgba, opening_rgba, "unheld digits still age");
    }

    #[test]
    fn raw_newest_tail_is_capped_before_invalid_points_are_filtered() {
        let room = CultOfPi::new();
        let finite = vec![(0.5, 0.5); crate::MAX_ROOM_POKES];
        let mut invalid_tail = finite;
        invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); crate::MAX_ROOM_POKES + 1]);
        let mut base = Canvas::new(60, 30);
        let mut actual = Canvas::new(60, 30);
        room.render(&mut base, 0.4);
        room.render_poked(&mut actual, 0.4, &invalid_tail);

        assert_eq!(base.to_text(), actual.to_text());
    }

    #[test]
    fn variation_is_replayable_and_seed_zero_is_canonical() {
        let default = CultOfPi::new();
        let zero = CultOfPi::new_with(0);
        let varied = CultOfPi::new_with(42);
        let mut a = Canvas::new(60, 30);
        let mut b = Canvas::new(60, 30);
        let mut c = Canvas::new(60, 30);
        default.render(&mut a, 0.5);
        zero.render(&mut b, 0.5);
        varied.render(&mut c, 0.5);

        assert_eq!(a.to_text(), b.to_text());
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn visible_field_begins_with_pi_and_has_no_blank_age_band() {
        let room = CultOfPi::new();
        let mut canvas = Canvas::new(60, 12);
        room.render(&mut canvas, 0.0);
        let field = layout(&canvas).expect("field");

        for (index, &digit) in digits().iter().take(40).enumerate() {
            let x = (index % field.columns) * field.step_x;
            let y = field.origin_y + (index / field.columns) * field.step_y;
            assert_eq!(canvas.cell(x, y), Some(char::from(b'0' + digit)));
        }
    }

    struct HostileSurface {
        plots: usize,
    }

    impl Surface for HostileSurface {
        fn width(&self) -> usize {
            usize::MAX
        }

        fn height(&self) -> usize {
            usize::MAX
        }

        fn plot(&mut self, _x: i32, _y: i32, _mark: char) {
            self.plots += 1;
        }
    }

    #[test]
    fn hostile_surface_keeps_field_and_draw_work_bounded() {
        let room = CultOfPi::new();
        let mut hostile = HostileSurface { plots: 0 };
        assert!(layout(&hostile).expect("bounded layout").cells() <= MAX_FIELD_CELLS);

        room.render(&mut hostile, f64::INFINITY);
        assert!(hostile.plots < 2_000_000, "{} plots", hostile.plots);
    }

    #[test]
    fn sound_degrades_without_becoming_nonfinite() {
        let room = CultOfPi::new();
        let exact = room.sound(0.0);
        let degraded = room.sound(1.0);
        assert_ne!(exact, degraded);
        assert!(
            degraded
                .notes
                .iter()
                .all(|note| note.freq.is_finite() && note.freq > 0.0)
        );
    }

    #[test]
    fn reveal_and_history_keep_the_boundaries_honest() {
        let room = CultOfPi::new();
        assert!(room.reveal().contains("truncated after n decimal places"));
        assert!(room.reveal().contains("display faults are ours"));
        assert!(room.deep_cuts()[1].contains("not established history"));
        assert_eq!(room.verb(), Some("CLICK: RESTORE AND HOLD A PATCH"));
    }

    #[test]
    fn every_frame_names_the_canonical_pi_prefix() {
        let room = CultOfPi::new_with(42);
        let mut actual = Canvas::new(180, 30);
        let mut header = Canvas::new(180, 30);
        room.render(&mut actual, 0.8);
        crate::draw_text(&mut header, PI_HEADER, 0, 0, 1, '#');

        for y in 0..30 {
            for x in 0..180 {
                if header.cell(x, y) == Some('#') {
                    assert_eq!(actual.cell(x, y), Some('#'));
                }
            }
        }
    }

    #[test]
    fn raster_header_sits_below_the_app_title_band() {
        let room = CultOfPi::new();
        let mut actual = crate::Raster::with_accent(640, 480, room.meta().accent);
        let mut header = crate::Raster::with_accent(640, 480, room.meta().accent);
        room.render(&mut actual, 0.8);
        let y = 18 + 7 * (1 + 1);
        crate::draw_text(&mut header, PI_HEADER, 0, y, 1, '#');

        let background = [10, 11, 15, 255];
        for (expected, observed) in header
            .to_rgba()
            .chunks_exact(4)
            .zip(actual.to_rgba().chunks_exact(4))
        {
            if expected != background {
                assert_ne!(observed, background);
            }
        }
    }
}
