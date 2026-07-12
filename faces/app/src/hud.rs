use numinous_core::{Raster, Room, Surface};

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
    let reveal_lines = if state.show_info && !state.the_show && !state.studio {
        let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
        numinous_core::wrap_text(&room.reveal().to_uppercase(), columns)
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
        let card_lines = if state.room_card > 0 && !state.show_info {
            3
        } else {
            0
        };
        let footer_top = height as i32 - (14 + card_lines * 9) * scale;
        raster.clear_rows(footer_top, height as i32);
        raster.line(0, footer_top, width as i32 - 1, footer_top, '-');
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
        numinous_core::draw_text(
            raster,
            &hint_text(room, state.t, state.muted),
            10,
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
        assert!(
            room_action(interactive.as_ref()).starts_with("CLICK:"),
            "interactive rooms keep their verb"
        );
        assert_eq!(
            arrival_lines(&quiet, 32)[0],
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
            },
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
