use numinous_core::{Journey, Raster, Scoreboard, Surface};

use crate::input_legend::{self, InputMode};

pub(crate) fn draw_help_overlay(
    raster: &mut Raster,
    width: usize,
    height: usize,
    selected_game: Option<usize>,
    input_mode: InputMode,
    activity_paused: bool,
) {
    raster.clear_rows(0, height as i32);
    raster.line(0, 0, width.saturating_sub(1) as i32, 0, '-');
    raster.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        height.saturating_sub(1) as i32,
        '-',
    );
    let compact_controller = input_mode == InputMode::Controller
        && !activity_paused
        && width <= 420
        && height <= 300
        && selected_game.is_some();
    let semantic = if compact_controller {
        input_legend::compact_controller_help_lines(selected_game.unwrap_or(0))
    } else {
        input_legend::help_lines(input_mode, selected_game, activity_paused)
    };
    let (lines, scale, line_step) = if compact_controller {
        overlay_layout_up_to(&semantic, width, height, 2)
    } else {
        overlay_layout(&semantic, width, height)
    };
    draw_centered_lines(raster, &lines, width, height, scale, line_step);
}

pub(crate) fn journey_lines(
    journey: &Journey,
    board: &Scoreboard,
    room_count: usize,
    input_mode: InputMode,
) -> Vec<String> {
    let mut lines = vec![
        format!("LV {}  [{}]", journey.level(), journey.level_bar(12)),
        format!(
            "{} XP  {}",
            journey.sparks(),
            journey.rank().name().to_uppercase()
        ),
        format!(
            "{} OF {} ROOMS   {} WINS",
            journey.visited.len(),
            room_count,
            journey.wins
        ),
        format!("{} PLAYS IN THIS LOCAL JOURNEY", journey.plays),
    ];
    if journey.streak > 1 {
        lines.push(format!("DAILY STREAK {}", journey.streak));
    }
    let earned: Vec<&str> = numinous_core::trophies(journey, board)
        .into_iter()
        .filter(|t| t.earned)
        .map(|t| t.name)
        .collect();
    lines.push(format!("TROPHIES {}", earned.len()));
    for name in earned.iter().take(6) {
        lines.push(format!("  {}", name.to_uppercase()));
    }
    let lit = numinous_core::resonances(journey, board)
        .into_iter()
        .filter(|r| r.active)
        .count();
    if lit > 0 {
        lines.push(format!("RESONANCES {lit}"));
    }
    lines.push(input_legend::journey_close(input_mode));
    lines
}

pub(crate) fn draw_journey_overlay(
    raster: &mut Raster,
    journey: &Journey,
    board: &Scoreboard,
    room_count: usize,
    width: usize,
    height: usize,
    input_mode: InputMode,
) {
    raster.clear_rows(0, height as i32);
    raster.line(0, 0, width.saturating_sub(1) as i32, 0, '-');
    raster.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        height.saturating_sub(1) as i32,
        '-',
    );
    let semantic = journey_lines(journey, board, room_count, input_mode);
    let (lines, scale, line_step) = overlay_layout(&semantic, width, height);
    draw_centered_lines(raster, &lines, width, height, scale, line_step);
}

fn pause_lines(input_mode: InputMode) -> Vec<String> {
    vec!["PAUSED".to_string(), input_legend::pause_resume(input_mode)]
}

fn pause_band_bounds(line_count: usize, scale: i32, line_step: i32, height: usize) -> (i32, i32) {
    let content_height = line_count as i32 * line_step * scale;
    let top = ((height as i32 - content_height) / 2).max(0);
    (
        (top - 8).max(0),
        (top + content_height + 8).min(height as i32),
    )
}

pub(crate) fn draw_pause_overlay(
    raster: &mut Raster,
    width: usize,
    height: usize,
    input_mode: InputMode,
) {
    let semantic = pause_lines(input_mode);
    let (lines, scale, line_step) = overlay_layout(&semantic, width, height);
    let (band_top, band_bottom) = pause_band_bounds(lines.len(), scale, line_step, height);
    raster.clear_rows(band_top, band_bottom);
    raster.line(0, band_top, width.saturating_sub(1) as i32, band_top, '-');
    raster.line(
        0,
        band_bottom.saturating_sub(1),
        width.saturating_sub(1) as i32,
        band_bottom.saturating_sub(1),
        '-',
    );
    draw_centered_lines(raster, &lines, width, height, scale, line_step);
}

pub(crate) fn draw_banner(raster: &mut Raster, lines: &[String], width: usize, height: usize) {
    let (lines, scale, line_step) = overlay_layout(lines, width, height);
    let line_height = line_step * scale;
    let content_height = lines.len() as i32 * line_height;
    let top = (height as i32 / 6).max(8);
    let band_top = (top - 8).max(0);
    let band_bottom = (top + content_height + 8).min(height as i32);
    raster.clear_rows(band_top, band_bottom);
    raster.line(0, band_top, width.saturating_sub(1) as i32, band_top, '-');
    raster.line(
        0,
        band_bottom.saturating_sub(1),
        width.saturating_sub(1) as i32,
        band_bottom.saturating_sub(1),
        '-',
    );
    for (i, line) in lines.iter().enumerate() {
        let text_width = line.chars().count() as i32 * 6 * scale;
        let left = ((width as i32 - text_width) / 2).max(10);
        numinous_core::draw_text(raster, line, left, top + i as i32 * line_height, scale, '#');
    }
}

fn overlay_layout<T: AsRef<str>>(
    semantic: &[T],
    width: usize,
    height: usize,
) -> (Vec<String>, i32, i32) {
    let largest = (width as i32 / 300).clamp(1, 4);
    overlay_layout_up_to(semantic, width, height, largest)
}

fn overlay_layout_up_to<T: AsRef<str>>(
    semantic: &[T],
    width: usize,
    height: usize,
    largest: i32,
) -> (Vec<String>, i32, i32) {
    for scale in (1..=largest).rev() {
        let columns = ((width as i32 - 20) / (6 * scale)).max(8) as usize;
        let lines: Vec<String> = semantic
            .iter()
            .flat_map(|line| {
                let line = line.as_ref();
                if line.is_empty() {
                    vec![String::new()]
                } else {
                    numinous_core::wrap_text(line, columns)
                }
            })
            .collect();
        let line_step = if scale == 1 { 9 } else { 11 };
        if lines.len() as i32 * line_step * scale <= height.saturating_sub(12) as i32 {
            return (lines, scale, line_step);
        }
    }

    let columns = ((width as i32 - 20) / 6).max(8) as usize;
    let lines = semantic
        .iter()
        .flat_map(|line| {
            let line = line.as_ref();
            if line.is_empty() {
                vec![String::new()]
            } else {
                numinous_core::wrap_text(line, columns)
            }
        })
        .collect();
    (lines, 1, 9)
}

#[cfg(test)]
fn line_fits(line: &str, width: usize, scale: i32) -> bool {
    let available = width as i32 - 20;
    let needed = line.chars().count() as i32 * 6 * scale;
    needed <= available
}

fn draw_centered_lines(
    raster: &mut Raster,
    lines: &[String],
    width: usize,
    height: usize,
    scale: i32,
    line_step: i32,
) {
    let line_height = line_step * scale;
    let top = (height as i32 / 2) - (lines.len() as i32 * line_height) / 2;
    for (i, line) in lines.iter().enumerate() {
        let text_width = line.chars().count() as i32 * 6 * scale;
        let left = ((width as i32 - text_width) / 2).max(10);
        numinous_core::draw_text(raster, line, left, top + i as i32 * line_height, scale, '#');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numinous_core::Surface;

    #[test]
    fn help_overlay_draws_playtest_controls() {
        let mut raster = Raster::with_accent(420, 360, [120, 220, 190]);
        for y in 0..360 {
            raster.line(0, y, 419, y, '#');
        }
        draw_help_overlay(
            &mut raster,
            420,
            360,
            Some(2),
            InputMode::KeyboardMouse,
            false,
        );
        assert!(raster.lit_count() > 300);
        assert_eq!(raster.width(), 420);
        assert_eq!(raster.height(), 360);
        let rgba = raster.to_rgba();
        let cleared = (2 * 420 + 2) * 4;
        assert_eq!(&rgba[cleared..cleared + 4], [10, 11, 15, 255]);
    }

    #[test]
    fn help_overlay_lines_fit_the_default_window() {
        for input_mode in [InputMode::KeyboardMouse, InputMode::Controller] {
            for (width, height) in [(900, 700), (360, 240)] {
                let compact = input_mode == InputMode::Controller && width <= 420 && height <= 300;
                let semantic = if compact {
                    input_legend::compact_controller_help_lines(4)
                } else {
                    input_legend::help_lines(input_mode, Some(4), false)
                };
                let (lines, scale, line_step) = if compact {
                    overlay_layout_up_to(&semantic, width, height, 2)
                } else {
                    overlay_layout(&semantic, width, height)
                };
                assert!(lines.len() as i32 * line_step * scale <= height as i32);
                for line in lines {
                    assert!(
                        line_fits(&line, width, scale),
                        "{line} should fit at {width}"
                    );
                }
                if compact {
                    assert_eq!(scale, 2);
                }
            }
        }
    }

    #[test]
    fn layout_falls_back_safely_when_no_candidate_can_fit() {
        let semantic = ["", "A BOUNDED LINE THAT MUST WRAP"];
        let (lines, scale, line_step) = overlay_layout(&semantic, 40, 0);
        assert_eq!(scale, 1);
        assert_eq!(line_step, 9);
        assert_eq!(lines.first().map(String::as_str), Some(""));
        assert!(lines.iter().any(|line| line.contains("BOUNDED")));
    }

    #[test]
    fn journey_lines_report_progress_and_close_action() {
        let mut journey = Journey::default();
        journey.visit("lissajous");
        journey.play();
        let board = Scoreboard::default();
        let keyboard = journey_lines(&journey, &board, 30, InputMode::KeyboardMouse);
        let controller = journey_lines(&journey, &board, 30, InputMode::Controller);
        assert!(keyboard.iter().any(|line| line.contains("1 OF 30 ROOMS")));
        assert!(keyboard.iter().any(|line| line.starts_with("TROPHIES ")));
        assert_eq!(keyboard.last().map(String::as_str), Some("J CLOSES"));
        assert_eq!(controller.last().map(String::as_str), Some("EAST CLOSES"));
    }

    #[test]
    fn pause_overlay_is_visible_bounded_and_mode_aware_at_compact_size() {
        let (width, height) = (360, 240);
        let mut keyboard = Raster::with_accent(width, height, [120, 220, 190]);
        let mut controller = Raster::with_accent(width, height, [120, 220, 190]);
        for y in 0..height as i32 {
            keyboard.line(0, y, width as i32 - 1, y, '#');
            controller.line(0, y, width as i32 - 1, y, '#');
        }

        draw_pause_overlay(&mut keyboard, width, height, InputMode::KeyboardMouse);
        draw_pause_overlay(&mut controller, width, height, InputMode::Controller);

        for input_mode in [InputMode::KeyboardMouse, InputMode::Controller] {
            let semantic = pause_lines(input_mode);
            let (lines, scale, line_step) = overlay_layout(&semantic, width, height);
            assert!(lines.iter().all(|line| line_fits(line, width, scale)));
            let (top, bottom) = pause_band_bounds(lines.len(), scale, line_step, height);
            assert!(top >= 0);
            assert!(bottom <= height as i32);
        }
        assert_ne!(keyboard.to_rgba(), controller.to_rgba());
        assert_eq!(
            &keyboard.to_rgba()[..width * 4],
            &controller.to_rgba()[..width * 4],
            "input copy remains inside the pause band"
        );
    }

    #[test]
    fn banner_draws_over_existing_frame() {
        let mut raster = Raster::with_accent(420, 300, [120, 220, 190]);
        for y in 0..300 {
            raster.line(0, y, 419, y, '#');
        }
        draw_banner(
            &mut raster,
            &[
                String::from("LEVEL UP  LV 2"),
                String::from("A LONG PIECE OF LORE THAT MUST WRAP INSIDE THE WINDOW"),
            ],
            420,
            300,
        );
        assert!(raster.lit_count() > 40);
        let rgba = raster.to_rgba();
        let quiet = (55 * 420 + 2) * 4;
        assert_eq!(&rgba[quiet..quiet + 4], [10, 11, 15, 255]);
    }

    #[test]
    fn banner_copy_fits_default_and_small_windows() {
        let semantic = vec![
            "LEVEL UP  LV 42".to_string(),
            "MORE OF YOUR MIND CAN LIVE HERE BEFORE IT FADES FROM NUMINOUS.".to_string(),
            "BOON BANKED: NUMINOUS CHOOSE".to_string(),
        ];
        for (width, height) in [(900, 700), (360, 240)] {
            let (lines, scale, line_step) = overlay_layout(&semantic, width, height);
            assert!(lines.len() as i32 * line_step * scale <= height as i32);
            assert!(lines.iter().all(|line| line_fits(line, width, scale)));
        }
    }
}
