use numinous_core::{Raster, Room, RoomInput, Surface};

use crate::input_legend::{self, InputMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioSource {
    RoomScore,
    Studio,
    Radio(&'static str),
    NoDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AudioState {
    source: AudioSource,
    volume_percent: u8,
    muted: bool,
    active: bool,
}

impl AudioState {
    pub(crate) fn new(source: AudioSource, volume_percent: u8, muted: bool, active: bool) -> Self {
        Self {
            source,
            volume_percent: volume_percent.min(100),
            muted,
            active,
        }
    }

    pub(crate) fn no_device() -> Self {
        Self::new(AudioSource::NoDevice, 0, false, false)
    }

    pub(crate) fn label(self) -> String {
        let source = match self.source {
            AudioSource::RoomScore => "ROOM MUSIC".to_string(),
            AudioSource::Studio => "STUDIO".to_string(),
            AudioSource::Radio(station) => format!("RADIO {}", station.to_uppercase()),
            AudioSource::NoDevice => return "NO SOUND DEVICE".to_string(),
        };
        if self.muted {
            format!("{source}: MUTED")
        } else if self.volume_percent == 0 {
            format!("{source}: VOL 0")
        } else if !self.active {
            format!("{source}: BACKGROUND SILENT")
        } else {
            format!("{source}: VOL {}%", self.volume_percent)
        }
    }
}

pub(crate) fn draw_audio_state(raster: &mut Raster, state: &AudioState, width: usize) {
    let scale = if width >= 720 { 2 } else { 1 };
    let level_reserve = if scale == 2 { 110 } else { 0 };
    let label = fit_footer_text(
        &state.label(),
        width.saturating_sub(20 + level_reserve) as i32,
        scale,
    );
    let x = width
        .saturating_sub(label.chars().count() * 6 * scale as usize)
        .saturating_sub(level_reserve)
        .saturating_sub(10) as i32;
    numinous_core::draw_text(raster, &label, x, 2, scale, '.');
}

pub(crate) struct RoomChrome {
    pub(crate) t: f64,
    pub(crate) room_card: u64,
    pub(crate) show_info: bool,
    pub(crate) show_help: bool,
    pub(crate) show_journey: bool,
    pub(crate) banner_active: bool,
    pub(crate) the_show: bool,
    pub(crate) studio: bool,
    pub(crate) muted: bool,
    pub(crate) level: u32,
    pub(crate) input_mode: InputMode,
}

pub(crate) fn room_action(room: &dyn Room) -> &'static str {
    numinous_core::room_touch_action(room)
}

fn displayed_room_action(room: &dyn Room, input_mode: InputMode) -> String {
    input_legend::room_action(input_mode, room_action(room))
}

fn journey_level_label(level: u32) -> String {
    format!("JOURNEY LV {level}")
}

fn arrival_lines(room: &dyn Room, columns: usize, input_mode: InputMode) -> Vec<String> {
    let mut lines = vec![displayed_room_action(room, input_mode)];
    if let Some(goal) = room.goal() {
        lines.push(format!("GOAL: {goal}"));
    }
    let mut blurb = numinous_core::wrap_text(&room.meta().blurb.to_uppercase(), columns);
    if blurb.len() > 3 {
        blurb.truncate(3);
        let last = blurb.last_mut().expect("truncated blurb has a final line");
        if last.chars().count() + 3 > columns {
            *last = last.chars().take(columns.saturating_sub(3)).collect();
        }
        last.push_str("...");
    }
    lines.extend(blurb);
    lines
}

#[derive(Debug, PartialEq, Eq)]
struct FooterCopy {
    action: String,
    status: String,
    controls: String,
}

fn fit_footer_text(text: &str, pixel_budget: i32, scale: i32) -> String {
    let columns = (pixel_budget / (6 * scale.max(1))).max(0) as usize;
    let length = text.chars().count();
    if length <= columns {
        return text.to_string();
    }
    if columns <= 3 {
        return ".".repeat(columns);
    }
    let mut fitted = text.chars().take(columns - 3).collect::<String>();
    fitted.push_str("...");
    fitted
}

fn footer_copy(
    room: &dyn Room,
    t: f64,
    inputs: &[RoomInput],
    muted: bool,
    input_mode: InputMode,
    status_override: Option<&str>,
) -> FooterCopy {
    let status = status_override.map_or_else(
        || {
            room.status_input(t, inputs)
                .unwrap_or_else(|| input_legend::room_inspect(input_mode))
        },
        str::to_owned,
    );
    FooterCopy {
        action: displayed_room_action(room, input_mode),
        status: if muted {
            format!("{status}   MUTED")
        } else {
            status
        },
        controls: input_legend::room_controls(input_mode),
    }
}

fn show_control_band_height(scale: i32) -> i32 {
    18 * scale
}

pub(crate) fn draw_room_chrome(
    raster: &mut Raster,
    room: &dyn Room,
    state: &RoomChrome,
    inputs: &[RoomInput],
    status_override: Option<&str>,
    width: usize,
    height: usize,
) {
    let scale = (width as i32 / 400).clamp(1, 4);
    let reveal_lines = if state.show_info && !state.the_show && !state.studio {
        let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
        numinous_core::wrap_text(&room.reveal().to_uppercase(), columns)
    } else {
        Vec::new()
    };
    let arrival = if state.room_card > 0
        && !state.show_info
        && !state.show_help
        && !state.show_journey
        && !state.banner_active
        && !state.the_show
        && !state.studio
    {
        let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
        arrival_lines(room, columns, state.input_mode)
    } else {
        Vec::new()
    };
    if !state.the_show && !state.studio && !state.show_help && !state.show_journey {
        if reveal_lines.is_empty() {
            let title_bottom = 14 + 7 * (scale + 1);
            raster.clear_rows(0, title_bottom);
            raster.line(0, title_bottom - 1, width as i32 - 1, title_bottom - 1, '-');
        } else {
            let reveal_bottom = 18 + (2 + reveal_lines.len() as i32) * 9 * scale;
            raster.clear_rows(0, reveal_bottom);
            raster.line(
                0,
                reveal_bottom - 1,
                width as i32 - 1,
                reveal_bottom - 1,
                '-',
            );
        }
        let card_lines = arrival.len() as i32;
        let footer_top = height as i32 - (24 + card_lines * 9) * scale;
        raster.clear_rows(footer_top, height as i32);
        raster.line(0, footer_top, width as i32 - 1, footer_top, '-');
    }

    if state.the_show {
        let control_band_height = show_control_band_height(scale).min(height as i32);
        let content_bottom = height as i32 - control_band_height;
        if state.t < 0.12 {
            raster.dim_rows((content_bottom - 34 * scale).max(0), content_bottom, 45);
            numinous_core::draw_text(
                raster,
                &room.meta().title.to_uppercase(),
                width as i32 / 10,
                content_bottom - 24 * scale,
                scale + 1,
                '#',
            );
        } else if state.t > 0.9 {
            let columns = ((width as i32 / (6 * scale)) - 8).max(12) as usize;
            let lines = numinous_core::wrap_text(&room.reveal().to_uppercase(), columns);
            let band_height = (lines.len() as i32 * 9 * scale + 16).min(content_bottom);
            let band_top = content_bottom - band_height;
            raster.clear_rows(band_top, content_bottom);
            raster.line(0, band_top, width.saturating_sub(1) as i32, band_top, '-');
            for (i, line) in lines.iter().enumerate() {
                numinous_core::draw_text(
                    raster,
                    line,
                    width as i32 / 10,
                    band_top + 8 + i as i32 * 9 * scale,
                    scale,
                    '#',
                );
            }
        }
        raster.clear_rows(content_bottom, height as i32);
        raster.line(
            0,
            content_bottom,
            width.saturating_sub(1) as i32,
            content_bottom,
            '-',
        );
        let controls = fit_footer_text(
            &input_legend::show_controls(state.input_mode),
            width as i32 - 20,
            scale,
        );
        numinous_core::draw_text(
            raster,
            &controls,
            10,
            height as i32 - 11 * scale,
            scale,
            '.',
        );
    }

    if !state.the_show && !state.studio {
        numinous_core::draw_text(
            raster,
            &room.meta().title.to_uppercase(),
            10,
            10,
            scale + 1,
            '#',
        );
        if !arrival.is_empty() {
            let footer_band_top = height as i32 - 24 * scale;
            let line_count = arrival.len() as i32;
            for (i, line) in arrival.iter().enumerate() {
                numinous_core::draw_text(
                    raster,
                    line,
                    10,
                    footer_band_top - (line_count - i as i32) * 9 * scale,
                    scale,
                    '#',
                );
            }
        }
        let level = journey_level_label(state.level);
        let level_scale = 1;
        let lx = width as i32 - (level.len() as i32 * 6 * level_scale) - 10;
        let ly = if scale > 1 { 20 } else { 10 };
        numinous_core::draw_text(raster, &level, lx, ly, level_scale, '#');
    }

    if state.show_info && !state.the_show && !state.studio {
        let line_height = 9 * scale;
        for (i, line) in reveal_lines.iter().enumerate() {
            numinous_core::draw_text(
                raster,
                line,
                10,
                10 + (2 + i as i32) * line_height,
                scale,
                '#',
            );
        }
    }

    if !state.show_help && !state.the_show && !state.studio {
        let footer = footer_copy(
            room,
            state.t,
            inputs,
            state.muted,
            state.input_mode,
            status_override,
        );
        let controls_width = footer.controls.chars().count() as i32 * 6 * scale;
        let controls_x = width as i32 - controls_width - 10;
        let action = fit_footer_text(&footer.action, width as i32 - 20, scale);
        let status = fit_footer_text(&footer.status, controls_x - 20, scale);
        if state.room_card == 0 || state.show_info || state.banner_active {
            numinous_core::draw_text(raster, &action, 10, height as i32 - 19 * scale, scale, '.');
        }
        numinous_core::draw_text(raster, &status, 10, height as i32 - 10 * scale, scale, '.');
        numinous_core::draw_text(
            raster,
            &footer.controls,
            controls_x,
            height as i32 - 10 * scale,
            scale,
            '.',
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numinous_core::{DEFAULT_TOUCH_ROOM_ACTION, Surface};

    fn room(id: &str) -> Box<dyn Room> {
        numinous_core::all_rooms_with(0)
            .into_iter()
            .find(|room| room.meta().id == id)
            .expect("room exists")
    }

    /// A verbless room: every catalog room answers the hand now, so the
    /// default-action fallback is proven against a synthetic room, the same
    /// shape any future room has on the day it is born.
    struct Newborn;

    impl Room for Newborn {
        fn meta(&self) -> numinous_core::RoomMeta {
            numinous_core::RoomMeta {
                id: "newborn",
                title: "Newborn",
                wing: "Tests",
                blurb: "A room that has not yet learned to answer the hand.",
                accent: [0, 0, 0],
            }
        }
        fn render(&self, _surface: &mut dyn numinous_core::surface::Surface, _t: f64) {}
        fn reveal(&self) -> &'static str {
            "Even a newborn room teaches what hands do."
        }
    }

    #[test]
    fn arrival_lines_always_name_an_action() {
        let quiet = Newborn;
        let interactive = room("game-of-life");
        assert_eq!(room_action(&quiet), DEFAULT_TOUCH_ROOM_ACTION);
        assert_eq!(
            room_action(interactive.as_ref()),
            "AIM + CLICK: PLACE A 5-CELL GLIDER"
        );
        assert_eq!(
            arrival_lines(&quiet, 32, InputMode::KeyboardMouse)[0],
            DEFAULT_TOUCH_ROOM_ACTION,
            "quiet rooms still teach what hands do"
        );
    }

    #[test]
    fn compact_arrival_copy_is_bounded_and_marks_truncation() {
        let room = room("golden-angle");
        let lines = arrival_lines(room.as_ref(), 24, InputMode::KeyboardMouse);

        assert_eq!(lines.len(), 4, "one action plus three blurb lines");
        assert_eq!(lines[0], room_action(room.as_ref()));
        assert!(lines[3].ends_with("..."));
        assert!(lines.iter().all(|line| line.chars().count() <= 24));
    }

    #[test]
    fn flagship_arrival_names_its_earned_goal() {
        let room = room("times-tables");
        let lines = arrival_lines(room.as_ref(), 40, InputMode::KeyboardMouse);

        assert_eq!(lines[0], "DRAG: TURN THE DIAL");
        assert_eq!(lines[1], "GOAL: LAND ON EXACTLY 4 LOBES");
    }

    #[test]
    fn footer_keeps_controls_fixed_while_status_changes() {
        let room = room("times-tables");
        let closed = footer_copy(
            room.as_ref(),
            0.0,
            &[],
            false,
            InputMode::KeyboardMouse,
            None,
        );
        let open = footer_copy(
            room.as_ref(),
            0.1,
            &[],
            true,
            InputMode::KeyboardMouse,
            None,
        );

        assert_ne!(closed.status, open.status);
        assert_eq!(closed.action, open.action);
        assert_eq!(closed.controls, "R RESET ROOM   ESC MENU");
        assert_eq!(closed.controls, open.controls);
        assert!(open.status.ends_with("MUTED"));
    }

    #[test]
    fn footer_copy_is_clipped_without_moving_controls() {
        assert_eq!(fit_footer_text("ABCDEFGHIJ", 42, 1), "ABCD...");
        assert_eq!(fit_footer_text("ABC", 18, 1), "ABC");
        assert_eq!(fit_footer_text("LONG", 12, 1), "..");
    }

    #[test]
    fn audio_state_labels_name_source_level_and_effective_silence() {
        let cases = [
            (
                AudioState::new(AudioSource::RoomScore, 45, false, true),
                "ROOM MUSIC: VOL 45%",
            ),
            (
                AudioState::new(AudioSource::Studio, 70, false, true),
                "STUDIO: VOL 70%",
            ),
            (
                AudioState::new(AudioSource::Radio("NUMINA FM"), 30, false, true),
                "RADIO NUMINA FM: VOL 30%",
            ),
            (
                AudioState::new(AudioSource::RoomScore, 45, true, true),
                "ROOM MUSIC: MUTED",
            ),
            (
                AudioState::new(AudioSource::RoomScore, 0, false, true),
                "ROOM MUSIC: VOL 0",
            ),
            (
                AudioState::new(AudioSource::RoomScore, 45, false, false),
                "ROOM MUSIC: BACKGROUND SILENT",
            ),
            (AudioState::no_device(), "NO SOUND DEVICE"),
        ];

        for (state, expected) in cases {
            assert_eq!(state.label(), expected);
        }
    }

    #[test]
    fn audio_state_badge_is_visible_and_bounded_at_supported_sizes() {
        for (width, height) in [(360, 240), (900, 700)] {
            let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
            let state = AudioState::new(AudioSource::RoomScore, 45, false, true);
            let label = state.label();
            draw_audio_state(&mut raster, &state, width);

            let rgba = raster.to_rgba();
            let mut changed = Vec::new();
            for (index, pixel) in rgba.chunks_exact(4).enumerate() {
                if pixel[..3] != [10, 11, 15] {
                    changed.push((index % width, index / width));
                }
            }
            assert!(!changed.is_empty());
            assert!(changed.iter().all(|&(x, y)| x < width && y < 20));
            assert!(changed.iter().any(|&(x, _)| x >= width / 2));
            let scale = if width >= 720 { 2 } else { 1 };
            let level_reserve = if scale == 2 { 110 } else { 0 };
            let x = width
                .saturating_sub(label.chars().count() * 6 * scale)
                .saturating_sub(level_reserve)
                .saturating_sub(10);
            for token in [":", "VOL", "45%"] {
                let offset = label.find(token).expect("audio token") * 6 * scale;
                assert!(
                    changed.iter().any(|&(px, _)| {
                        px >= x + offset && px < x + offset + token.len() * 6 * scale
                    }),
                    "{token} must have visible pixels at width {width}"
                );
            }
            if width >= 720 {
                assert!(changed.iter().any(|&(_, y)| y >= 9));
                assert!(changed.iter().all(|&(x, _)| x < width - 110));
            }
        }
    }

    #[test]
    fn room_copy_follows_the_active_input_mode() {
        let room = room("golden-angle");
        let keyboard = footer_copy(
            room.as_ref(),
            0.0,
            &[],
            false,
            InputMode::KeyboardMouse,
            None,
        );
        let controller = footer_copy(room.as_ref(), 0.0, &[], false, InputMode::Controller, None);

        assert_eq!(keyboard.action, "CLICK: PLANT A SEED");
        // First-contact status names the room state; inspect only appears when
        // a room has no status at all.
        assert_eq!(
            keyboard.status,
            "GOLDEN ANGLE 137.5 DEG   CLICK: PLANT A SEED"
        );
        assert_eq!(keyboard.controls, "R RESET ROOM   ESC MENU");
        assert_eq!(controller.action, "SOUTH: PLANT A SEED");
        assert_eq!(
            controller.status,
            "GOLDEN ANGLE 137.5 DEG   CLICK: PLANT A SEED"
        );
        assert_eq!(controller.controls, "L3 RESET ROOM   START MENU");
        assert!(controller.controls.chars().count() * 6 <= 360 - 20);
    }

    #[test]
    fn room_header_names_global_journey_progress() {
        assert_eq!(super::journey_level_label(42), "JOURNEY LV 42");
    }

    #[test]
    fn footer_uses_interaction_aware_status() {
        let room = room("game-of-life");
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.25,
        }];
        let footer = footer_copy(
            room.as_ref(),
            0.25,
            &input,
            false,
            InputMode::KeyboardMouse,
            None,
        );
        assert_eq!(footer.action, "AIM + CLICK: PLACE A 5-CELL GLIDER");
        assert!(footer.status.contains("PLANTED 5"));
        assert!(footer.status.contains("GLIDER 1"));
        assert_eq!(footer.controls, "R RESET ROOM   ESC MENU");
    }

    #[test]
    fn compact_life_status_keeps_cause_and_controller_truth() {
        let room = room("game-of-life");
        let mut session = numinous_core::rooms::game_of_life::LifeSession::new(0);
        session.launch((0.5, 0.5));
        session.advance();
        let status = session.compact_status();
        for mode in [InputMode::KeyboardMouse, InputMode::Controller] {
            let footer = footer_copy(room.as_ref(), 0.0, &[], false, mode, Some(&status));
            let controls_width = footer.controls.chars().count() as i32 * 6;
            let controls_x = 360 - controls_width - 10;
            assert_eq!(fit_footer_text(&footer.status, controls_x - 20, 1), status);
            assert!(footer.status.starts_with("BORN"));
            assert!(footer.status.contains("DIED"));
        }
        let controller = footer_copy(
            room.as_ref(),
            0.0,
            &[],
            false,
            InputMode::Controller,
            Some(&status),
        );
        assert_eq!(
            controller.action,
            "LEFT STICK + SOUTH: PLACE A 5-CELL GLIDER"
        );
        assert_eq!(controller.controls, "L3 RESET ROOM   START MENU");
    }

    #[test]
    fn compact_galton_result_keeps_the_run_and_landing_before_fixed_controls() {
        let room = room("galton-board");
        let input = [RoomInput::PointerDown {
            x: 0.94,
            y: 0.5,
            t: 0.25,
        }];
        let footer = footer_copy(
            room.as_ref(),
            0.25,
            &input,
            false,
            InputMode::KeyboardMouse,
            None,
        );
        let controls_width = footer.controls.chars().count() as i32 * 6;
        let controls_x = 360 - controls_width - 10;
        let fitted = fit_footer_text(&footer.status, controls_x - 20, 1);

        assert_eq!(fitted, footer.status);
        assert!(fitted.starts_with("P.70"));
        assert!(fitted.contains("1x64=64"));
        assert!(fitted.contains("LAST"));
        assert!(fitted.ends_with('R'));

        let controller = footer_copy(
            room.as_ref(),
            0.25,
            &input,
            false,
            InputMode::Controller,
            None,
        );
        assert_eq!(
            controller.action,
            "LEFT STICK + SOUTH: PICK COIN, DROP 64 BALLS"
        );
    }

    #[test]
    fn compact_galton_full_run_keeps_total_and_landing() {
        let room = room("galton-board");
        let inputs = vec![
            RoomInput::PointerDown {
                x: 0.94,
                y: 0.5,
                t: 0.25,
            };
            numinous_core::MAX_ROOM_POKES
        ];
        let footer = footer_copy(
            room.as_ref(),
            0.25,
            &inputs,
            false,
            InputMode::KeyboardMouse,
            None,
        );
        let controls_width = footer.controls.chars().count() as i32 * 6;
        let controls_x = 360 - controls_width - 10;
        let fitted = fit_footer_text(&footer.status, controls_x - 20, 1);

        assert_eq!(fitted, footer.status);
        assert!(fitted.starts_with("P.70"));
        assert!(fitted.contains("FULL=1536"));
        assert!(fitted.contains("LAST"));
        assert!(fitted.ends_with('R'));
    }

    #[test]
    fn room_chrome_draws_without_a_game_mode() {
        let room = room("lissajous");
        let mut raster = Raster::with_accent(320, 220, room.meta().accent);
        draw_room_chrome(
            &mut raster,
            room.as_ref(),
            &RoomChrome {
                t: 0.0,
                room_card: 240,
                show_info: false,
                show_help: false,
                show_journey: false,
                banner_active: false,
                the_show: false,
                studio: false,
                muted: false,
                level: 3,
                input_mode: InputMode::KeyboardMouse,
            },
            &[],
            None,
            320,
            220,
        );
        assert!(raster.lit_count() > 100);
        assert_eq!(raster.width(), 320);
        assert_eq!(raster.height(), 220);
    }

    #[test]
    fn studio_mode_suppresses_room_chrome() {
        let room = room("lissajous");
        let mut raster = Raster::with_accent(320, 220, [120, 220, 190]);
        let before = raster.to_rgba();
        draw_room_chrome(
            &mut raster,
            room.as_ref(),
            &RoomChrome {
                t: 0.5,
                room_card: 240,
                show_info: false,
                show_help: false,
                show_journey: false,
                banner_active: false,
                the_show: false,
                studio: true,
                muted: true,
                level: 3,
                input_mode: InputMode::KeyboardMouse,
            },
            &[],
            None,
            320,
            220,
        );
        assert_eq!(raster.to_rgba(), before);
    }

    #[test]
    fn the_show_draws_arrival_and_departure_copy() {
        let room = room("lorenz");
        let mut arrival = Raster::with_accent(420, 300, room.meta().accent);
        let mut departure = Raster::with_accent(420, 300, room.meta().accent);
        for (raster, t) in [(&mut arrival, 0.05), (&mut departure, 0.95)] {
            room.render(raster, t);
            draw_room_chrome(
                raster,
                room.as_ref(),
                &RoomChrome {
                    t,
                    room_card: 0,
                    show_info: false,
                    show_help: false,
                    show_journey: false,
                    banner_active: false,
                    the_show: true,
                    studio: false,
                    muted: false,
                    level: 1,
                    input_mode: InputMode::KeyboardMouse,
                },
                &[],
                None,
                420,
                300,
            );
        }
        assert_ne!(arrival.to_rgba(), departure.to_rgba());
        assert!(arrival.lit_count() > 100);
        assert!(departure.lit_count() > 100);
    }

    #[test]
    fn show_input_copy_stays_inside_one_fixed_compact_band() {
        let room = room("lorenz");
        let (width, height) = (360, 240);
        let mut keyboard = Raster::with_accent(width, height, room.meta().accent);
        let mut controller = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut keyboard, 0.5);
        room.render(&mut controller, 0.5);

        for (raster, input_mode) in [
            (&mut keyboard, InputMode::KeyboardMouse),
            (&mut controller, InputMode::Controller),
        ] {
            draw_room_chrome(
                raster,
                room.as_ref(),
                &RoomChrome {
                    t: 0.5,
                    room_card: 0,
                    show_info: false,
                    show_help: false,
                    show_journey: false,
                    banner_active: false,
                    the_show: true,
                    studio: false,
                    muted: false,
                    level: 1,
                    input_mode,
                },
                &[],
                None,
                width,
                height,
            );
        }

        let band_top = (height as i32 - show_control_band_height(1)) as usize;
        let split = band_top * width * 4;
        let keyboard_pixels = keyboard.to_rgba();
        let controller_pixels = controller.to_rgba();
        assert_eq!(
            &keyboard_pixels[..split],
            &controller_pixels[..split],
            "input copy must not move the Show content"
        );
        assert_ne!(&keyboard_pixels[split..], &controller_pixels[split..]);
        for copy in [
            input_legend::show_controls(InputMode::KeyboardMouse),
            input_legend::show_controls(InputMode::Controller),
        ] {
            assert!(copy.chars().count() * 6 <= width - 20);
        }
    }

    #[test]
    fn inspection_copy_gets_a_clear_content_sized_panel() {
        let room = room("golden-angle");
        let (width, height) = (900, 900);
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, 0.0);
        draw_room_chrome(
            &mut raster,
            room.as_ref(),
            &RoomChrome {
                t: 0.0,
                room_card: 0,
                show_info: true,
                show_help: false,
                show_journey: false,
                banner_active: false,
                the_show: false,
                studio: false,
                muted: false,
                level: 1,
                input_mode: InputMode::KeyboardMouse,
            },
            &[],
            None,
            width,
            height,
        );

        let rgba = raster.to_rgba();
        let quiet_pixel = (70 * width + width / 2) * 4;
        assert_eq!(&rgba[quiet_pixel..quiet_pixel + 4], [10, 11, 15, 255]);
        let lower_lit = rgba[120 * width * 4..]
            .chunks_exact(4)
            .filter(|pixel| *pixel != [10, 11, 15, 255])
            .count();
        assert!(lower_lit > 100, "room art remains visible below the panel");
    }
}
