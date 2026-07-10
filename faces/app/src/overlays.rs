use numinous_core::{Journey, Raster, Scoreboard};

const HELP_LINES: &[&str] = &[
    "PLAY (PRESS A LETTER)",
    "G          THE QUIZ: NAME THE MATH",
    "C          MUNCH: EAT WHAT FITS",
    "N          NIM: BEAT THE ORDER",
    "T          THE GAUNTLET: ONE RUN",
    "V          THE ARCADE: EAT WHILE HUNTED",
    "",
    "WANDER",
    "A / D      PREV / NEXT ROOM    1-9 JUMP",
    "W / S      TIME SPEED   MOUSE  SCRUB",
    "E          INSPECT    Q  ERA    R  RESTART",
    "B          THE SHOW   TAB  THE STUDIO",
    "J          JOURNEY    F  FULLSCREEN",
    "Y          RADIO    P  POSTCARD",
    "F9         PLAYTEST NOTE",
    "M          SOUND   -/= VOLUME   SPACE PAUSE",
    "",
    "ESC        CLOSE MENU AND WANDER",
];

pub(crate) fn draw_help_overlay(raster: &mut Raster, width: usize, height: usize) {
    raster.dim(22);
    let scale = overlay_scale(width);
    draw_centered_lines(raster, HELP_LINES.iter().copied(), width, height, scale, 11);
}

pub(crate) fn journey_lines(
    journey: &Journey,
    board: &Scoreboard,
    room_count: usize,
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
    lines.push("J CLOSES".to_string());
    lines
}

pub(crate) fn draw_journey_overlay(
    raster: &mut Raster,
    journey: &Journey,
    board: &Scoreboard,
    room_count: usize,
    width: usize,
    height: usize,
) {
    raster.dim(22);
    let scale = overlay_scale(width);
    let lines = journey_lines(journey, board, room_count);
    draw_centered_lines(
        raster,
        lines.iter().map(String::as_str),
        width,
        height,
        scale,
        11,
    );
}

pub(crate) fn draw_banner(raster: &mut Raster, lines: &[String], width: usize, height: usize) {
    let scale = overlay_scale(width);
    let line_height = 12 * scale;
    let top = height as i32 / 6;
    for (i, line) in lines.iter().enumerate() {
        numinous_core::draw_text(
            raster,
            line,
            width as i32 / 8,
            top + i as i32 * line_height,
            scale,
            '#',
        );
    }
}

fn overlay_scale(width: usize) -> i32 {
    (width as i32 / 300).clamp(2, 4)
}

#[cfg(test)]
fn line_fits(line: &str, width: usize, scale: i32) -> bool {
    let left = width as i32 / 8;
    let available = width as i32 - left;
    let needed = line.chars().count() as i32 * 6 * scale;
    needed <= available
}

fn draw_centered_lines<'a>(
    raster: &mut Raster,
    lines: impl Iterator<Item = &'a str>,
    width: usize,
    height: usize,
    scale: i32,
    line_step: i32,
) {
    let lines: Vec<&str> = lines.collect();
    let line_height = line_step * scale;
    let top = (height as i32 / 2) - (lines.len() as i32 * line_height) / 2;
    for (i, line) in lines.iter().enumerate() {
        numinous_core::draw_text(
            raster,
            line,
            width as i32 / 8,
            top + i as i32 * line_height,
            scale,
            '#',
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numinous_core::Surface;

    #[test]
    fn help_overlay_draws_playtest_controls() {
        let mut raster = Raster::with_accent(420, 360, [120, 220, 190]);
        draw_help_overlay(&mut raster, 420, 360);
        assert!(raster.lit_count() > 300);
        assert_eq!(raster.width(), 420);
        assert_eq!(raster.height(), 360);
    }

    #[test]
    fn help_overlay_lines_fit_the_default_window() {
        let width = 900;
        let scale = overlay_scale(width);
        for line in HELP_LINES {
            assert!(line_fits(line, width, scale), "{line} should fit");
        }
    }

    #[test]
    fn journey_lines_report_progress_and_close_action() {
        let mut journey = Journey::default();
        journey.visit("lissajous");
        journey.play();
        let board = Scoreboard::default();
        let lines = journey_lines(&journey, &board, 30);
        assert!(lines.iter().any(|line| line.contains("1 OF 30 ROOMS")));
        assert!(lines.iter().any(|line| line.starts_with("TROPHIES ")));
        assert_eq!(lines.last().map(String::as_str), Some("J CLOSES"));
    }

    #[test]
    fn banner_draws_over_existing_frame() {
        let mut raster = Raster::with_accent(420, 300, [120, 220, 190]);
        draw_banner(&mut raster, &[String::from("LEVEL UP  LV 2")], 420, 300);
        assert!(raster.lit_count() > 40);
    }
}
