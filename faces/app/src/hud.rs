use numinous_core::{Raster, Room};

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
}

pub(crate) fn room_action(room: &dyn Room) -> &'static str {
    numinous_core::room_touch_action(room)
}

fn arrival_lines(room: &dyn Room, columns: usize) -> Vec<String> {
    let mut lines = vec![room_action(room).to_string()];
    lines.extend(
        numinous_core::wrap_text(&room.meta().blurb.to_uppercase(), columns)
            .into_iter()
            .take(2),
    );
    lines
}

fn hint_text(room: &dyn Room, t: f64, muted: bool) -> String {
    let verb = room_action(room);
    let mut hint = match room.status(t) {
        Some(readout) => format!("{verb}   {readout}   ESC MENU"),
        None => format!("{verb}     ESC  PLAY + MENU   E INSPECT"),
    };
    if muted {
        hint.push_str("   (MUTED)");
    }
    hint
}

pub(crate) fn draw_room_chrome(
    raster: &mut Raster,
    room: &dyn Room,
    state: &RoomChrome,
    width: usize,
    height: usize,
) {
    let scale = (width as i32 / 400).clamp(1, 4);
    if !state.the_show && !state.studio && !state.show_help && !state.show_journey {
        raster.dim_rows(0, 14 + 7 * (scale + 1), 45);
        let card_lines = if state.room_card > 0 && !state.show_info {
            3
        } else {
            0
        };
        raster.dim_rows(
            height as i32 - (14 + card_lines * 9) * scale,
            height as i32,
            45,
        );
    }

    if state.the_show {
        if state.t < 0.12 || state.t > 0.9 {
            raster.dim_rows(height as i32 - 34 * scale, height as i32, 45);
        }
        if state.t < 0.12 {
            numinous_core::draw_text(
                raster,
                &room.meta().title.to_uppercase(),
                width as i32 / 10,
                height as i32 - 24 * scale,
                scale + 1,
                '#',
            );
        } else if state.t > 0.9 {
            let columns = ((width as i32 / (6 * scale)) - 8).max(12) as usize;
            for (i, line) in numinous_core::wrap_text(&room.reveal().to_uppercase(), columns)
                .iter()
                .take(3)
                .enumerate()
            {
                numinous_core::draw_text(
                    raster,
                    line,
                    width as i32 / 10,
                    height as i32 - (30 - i as i32 * 9) * scale,
                    scale,
                    '#',
                );
            }
        }
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
        if state.room_card > 0
            && !state.show_info
            && !state.show_help
            && !state.show_journey
            && !state.banner_active
        {
            let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
            for (i, line) in arrival_lines(room, columns).iter().take(3).enumerate() {
                numinous_core::draw_text(
                    raster,
                    line,
                    10,
                    height as i32 - (39 - i as i32 * 9) * scale,
                    scale,
                    '#',
                );
            }
        }
        let level = format!("LV {}", state.level);
        let lx = width as i32 - (level.len() as i32 * 6 * scale) - 10;
        numinous_core::draw_text(raster, &level, lx, 10, scale, '#');
    }

    if state.show_info && !state.the_show && !state.studio {
        let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
        let line_height = 9 * scale;
        for (i, line) in numinous_core::wrap_text(&room.reveal().to_uppercase(), columns)
            .iter()
            .enumerate()
        {
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
        numinous_core::draw_text(
            raster,
            &hint_text(room, state.t, state.muted),
            10,
            height as i32 - 10 * scale,
            scale,
            '-',
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

    #[test]
    fn arrival_lines_always_name_an_action() {
        let quiet = room("the-pour");
        let interactive = room("game-of-life");
        assert_eq!(room_action(quiet.as_ref()), DEFAULT_TOUCH_ROOM_ACTION);
        assert!(
            room_action(interactive.as_ref()).starts_with("CLICK:"),
            "interactive rooms keep their verb"
        );
        assert_eq!(
            arrival_lines(quiet.as_ref(), 32)[0],
            DEFAULT_TOUCH_ROOM_ACTION,
            "quiet rooms still teach what hands do"
        );
    }

    #[test]
    fn hint_text_includes_status_and_mute_state() {
        let room = room("game-of-life");
        let hint = hint_text(room.as_ref(), 0.25, true);
        assert!(hint.starts_with("CLICK:"));
        assert!(hint.contains("ESC"));
        assert!(hint.contains("(MUTED)"));
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
            },
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
            },
            320,
            220,
        );
        assert_eq!(raster.to_rgba(), before);
    }
}
