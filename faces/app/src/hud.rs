use numinous_core::{Raster, Room, RoomInput, Surface};

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
    controls: &'static str,
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

fn footer_copy(room: &dyn Room, t: f64, inputs: &[RoomInput], muted: bool) -> FooterCopy {
    let status = room
        .status_input(t, inputs)
        .unwrap_or_else(|| "E INSPECT".to_string());
    FooterCopy {
        action: room_action(room).to_string(),
        status: if muted {
            format!("{status}   MUTED")
        } else {
            status
        },
        controls: "R RESET ROOM   ESC MENU",
    }
}

pub(crate) fn draw_room_chrome(
    raster: &mut Raster,
    room: &dyn Room,
    state: &RoomChrome,
    inputs: &[RoomInput],
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
        arrival_lines(room, columns)
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
        if state.t < 0.12 {
            raster.dim_rows(height as i32 - 34 * scale, height as i32, 45);
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
            let lines = numinous_core::wrap_text(&room.reveal().to_uppercase(), columns);
            let band_height = (lines.len() as i32 * 9 * scale + 16).min(height as i32);
            let band_top = height as i32 - band_height;
            raster.clear_rows(band_top, height as i32);
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
        let footer = footer_copy(room, state.t, inputs, state.muted);
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
            footer.controls,
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
    fn compact_arrival_copy_is_bounded_and_marks_truncation() {
        let room = room("golden-angle");
        let lines = arrival_lines(room.as_ref(), 24);

        assert_eq!(lines.len(), 4, "one action plus three blurb lines");
        assert_eq!(lines[0], room_action(room.as_ref()));
        assert!(lines[3].ends_with("..."));
        assert!(lines.iter().all(|line| line.chars().count() <= 24));
    }

    #[test]
    fn footer_keeps_controls_fixed_while_status_changes() {
        let room = room("times-tables");
        let closed = footer_copy(room.as_ref(), 0.0, &[], false);
        let open = footer_copy(room.as_ref(), 0.1, &[], true);

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
    fn footer_uses_interaction_aware_status() {
        let room = room("game-of-life");
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.25,
        }];
        let footer = footer_copy(room.as_ref(), 0.25, &input, false);
        assert!(footer.action.starts_with("CLICK:"));
        assert!(footer.status.contains("1 GLIDER"));
        assert_eq!(footer.controls, "R RESET ROOM   ESC MENU");
    }

    #[test]
    fn compact_galton_result_keeps_the_flip_field_before_fixed_controls() {
        let room = room("galton-board");
        let input = [RoomInput::PointerDown {
            x: 0.94,
            y: 0.5,
            t: 0.25,
        }];
        let footer = footer_copy(room.as_ref(), 0.25, &input, false);
        let controls_width = footer.controls.chars().count() as i32 * 6;
        let controls_x = 360 - controls_width - 10;
        let fitted = fit_footer_text(&footer.status, controls_x - 20, 1);

        assert_eq!(fitted, footer.status);
        assert!(fitted.ends_with("R-FLIPS"));
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
            &[],
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
            &[],
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
                },
                &[],
                420,
                300,
            );
        }
        assert_ne!(arrival.to_rgba(), departure.to_rgba());
        assert!(arrival.lit_count() > 100);
        assert!(departure.lit_count() > 100);
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
            &[],
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
