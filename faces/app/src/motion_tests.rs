//! Headless acceptance tests for the App's motion law: one dissolve through
//! the stage on every change of room, and dials that glide in presented
//! frames while the accepted history stays exact.

use numinous_core::{Motion, Raster, RoomInput};

use super::gamepad::Command;
use super::menu::MenuIntent;
use super::{App, effective_room_phase};
use crate::dissolve::STAGE;

const SIZE: (usize, usize) = (160, 120);
const FRAME: f64 = 1.0 / 60.0;

fn quiet_app(name: &str, motion: Motion) -> App {
    let mut app = super::tests::headless(name);
    app.close_menu();
    app.banner = None;
    app.room_card = 0;
    app.motion = motion;
    app
}

/// The composed frame and the frame actually presented for the current room,
/// built as `draw` and `present_raster` build them, minus the window.
fn frame(app: &mut App) -> (Vec<u8>, Vec<u8>) {
    let inputs = app.presented_room_inputs();
    let room = &app.rooms[app.current];
    let mut raster = Raster::with_accent(SIZE.0, SIZE.1, room.meta().accent);
    let phase = effective_room_phase(room.meta().id, app.t, &app.inputs, app.the_show);
    room.render_input(&mut raster, phase, &inputs);
    let (composed, width, height) = app.compose_frame(raster, SIZE.0, SIZE.1);
    let shown = app.dissolve.frame(composed.clone(), width, height).to_vec();
    (composed, shown)
}

/// An App that has presented a frame and has no change in progress, so the
/// next change has a picture to leave from.
fn presented_app(name: &str, motion: Motion) -> App {
    let mut app = quiet_app(name, motion);
    let _ = frame(&mut app);
    app.advance_presentation_time(10.0);
    assert!(app.dissolve.is_steady());
    app
}

fn luminance(rgba: &[u8]) -> f64 {
    numinous_core::photosensitivity::frame_luminance(rgba)
}

fn select(app: &mut App, id: &str) {
    let index = app
        .rooms
        .iter()
        .position(|room| room.meta().id == id)
        .unwrap_or_else(|| panic!("{id} is in the catalog"));
    app.goto_room_index(index);
    app.advance_presentation_time(10.0);
}

#[test]
fn every_way_of_changing_rooms_enters_through_the_dissolve() {
    type RoomChange = (&'static str, fn(&mut App));
    let ways: Vec<RoomChange> = vec![
        ("arrow or strafe key", |app| app.switch(1)),
        ("arrow back", |app| app.switch(-1)),
        ("controller bumper", |app| {
            app.handle_gamepad_command(Command::NextRoom);
        }),
        ("controller d-pad", |app| {
            app.handle_gamepad_command(Command::Left);
        }),
        ("number slot", |app| app.enter_room_slot('3')),
        ("number slot ten", |app| app.enter_room_slot('0')),
        ("wing door", |app| {
            app.apply_menu_intent(MenuIntent::EnterWing(0))
        }),
        ("walk door", |app| {
            app.apply_menu_intent(MenuIntent::EnterWalk)
        }),
        ("flagship door", |app| {
            app.apply_menu_intent(MenuIntent::TouchTheFlagship);
        }),
        ("console goto", |app| {
            let _ = app.run_console_command(crate::console::Command::Goto("rule-30".into()));
        }),
        ("console vary", |app| {
            let _ = app.run_console_command(crate::console::Command::Vary(7));
        }),
        ("step inside a wing", |app| {
            app.apply_menu_intent(MenuIntent::EnterWing(0));
            app.advance_presentation_time(10.0);
            assert!(app.dissolve.is_steady());
            app.switch(1);
        }),
        ("The Show drifting on", |app| {
            app.toggle_show();
            for _ in 0..2000 {
                let room = app.current;
                app.advance_room_tick(0.05, 0.0, false);
                if app.current != room {
                    return;
                }
            }
            panic!("The Show never drifted to another room");
        }),
    ];
    for (way, change) in ways {
        let mut app = presented_app("numinous_app_test_every_way", Motion::Full);
        // Start from the far end of the catalog, so no door or slot leads
        // back to the room already on screen.
        app.goto_room_index(app.rooms.len() - 1);
        app.advance_presentation_time(10.0);
        let before = (app.current, app.variation);
        change(&mut app);
        assert!(
            !app.dissolve.is_steady(),
            "{way} changed rooms without the dissolve"
        );
        assert_ne!(
            (app.current, app.variation),
            before,
            "{way} did not change the room at all"
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }
}

#[test]
fn the_room_index_is_written_in_one_place() {
    // The behavioural test above walks every way that exists today. This
    // keeps a new one from writing the room index directly and cutting.
    let sources = [
        include_str!("main.rs"),
        include_str!("audio_runtime.rs"),
        include_str!("creation_runtime.rs"),
        include_str!("game_runtime.rs"),
        include_str!("input_runtime.rs"),
        include_str!("room_runtime.rs"),
        include_str!("route_runtime.rs"),
        include_str!("study_runtime.rs"),
    ];
    let production: Vec<String> = sources
        .iter()
        .map(|source| {
            let source = source.replace("\r\n", "\n");
            match source.split_once("#[cfg(test)]\nmod ") {
                Some((before, _)) => before.to_string(),
                None => source,
            }
        })
        .collect();
    let count = |needle: &str| {
        production
            .iter()
            .map(|source| source.matches(needle).count())
            .sum::<usize>()
    };
    assert_eq!(count("self.current = "), 1, "one write of the room index");
    assert_eq!(
        count("self.dissolve.begin("),
        1,
        "one start of the dissolve"
    );
    let entry = production[0]
        .split_once("fn enter_room(")
        .and_then(|(_, rest)| rest.split_once("\n    }\n"))
        .map(|(body, _)| body)
        .expect("enter_room in main.rs");
    assert!(entry.contains("self.current = "));
    assert!(entry.contains("self.dissolve.begin("));
}

#[test]
fn a_change_of_room_passes_through_the_stage_and_never_outshines_either_room() {
    let mut app = presented_app("numinous_app_test_dissolve_frames", Motion::Full);
    let (leaving, shown) = frame(&mut app);
    assert_eq!(shown, leaving, "a steady frame is presented untouched");
    let stage = luminance(&[STAGE[0], STAGE[1], STAGE[2], 255]);
    assert!(luminance(&leaving) > stage * 1.5, "the leaving room is lit");

    app.switch(1);
    let mut levels = Vec::new();
    let mut darkest = f64::INFINITY;
    let mut reached_the_stage = false;
    let mut previous = luminance(&leaving);
    for tick in 0..60 {
        app.advance_room_tick(FRAME, FRAME, false);
        let (arriving, shown) = frame(&mut app);
        let light = luminance(&shown);
        // No pixel of a change is brighter than the same pixel in the room
        // leaving or the room arriving, so the two never add up.
        for ((shown, old), new) in shown
            .chunks_exact(4)
            .zip(leaving.chunks_exact(4))
            .zip(arriving.chunks_exact(4))
        {
            for channel in 0..3 {
                assert!(
                    shown[channel] <= old[channel].max(new[channel]),
                    "tick {tick}: a channel outshone both rooms"
                );
            }
        }
        assert!(light <= luminance(&leaving).max(luminance(&arriving)) + 1e-12);
        // One frame never jumps far: the change eases rather than cuts.
        assert!(
            (light - previous).abs() < 0.25 * luminance(&leaving).max(luminance(&arriving)),
            "tick {tick}: the light jumped from {previous} to {light}"
        );
        previous = light;
        if shown
            .chunks_exact(4)
            .all(|pixel| (0..3).all(|channel| pixel[channel] <= STAGE[channel]))
        {
            reached_the_stage = true;
        }
        darkest = darkest.min(light);
        levels.push(app.dissolve.level());
        if tick == 59 {
            assert_eq!(shown, arriving, "the arriving room settles untouched");
        }
    }
    assert!(
        reached_the_stage,
        "the change never passed through the stage"
    );
    assert!(darkest <= stage + 1e-12);
    // Down to the stage, then up, once each.
    let bottom = levels
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))
        .map(|(index, _)| index)
        .expect("levels were recorded");
    assert!(levels[bottom] < 1e-3);
    assert!(levels[..=bottom].windows(2).all(|pair| pair[1] <= pair[0]));
    assert!(levels[bottom..].windows(2).all(|pair| pair[1] >= pair[0]));
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn reduced_motion_changes_rooms_with_a_brief_plain_fade() {
    let mut full = presented_app("numinous_app_test_full_dissolve", Motion::Full);
    let mut reduced = presented_app("numinous_app_test_reduced_dissolve", Motion::Reduced);
    full.switch(1);
    reduced.switch(1);
    for app in [&mut full, &mut reduced] {
        assert!(!app.dissolve.is_steady(), "every change still dissolves");
        app.advance_presentation_time(0.04);
    }
    // Half way out, a plain line has lost exactly half its light; the eased
    // curve has barely started.
    assert!((reduced.dissolve.level() - 0.5).abs() < 1e-9);
    assert!(full.dissolve.level() > 0.9);
    for app in [&mut full, &mut reduced] {
        app.advance_presentation_time(0.17);
    }
    assert!(reduced.dissolve.is_steady(), "a fifth of a second in all");
    assert!(!full.dissolve.is_steady());
    let _ = std::fs::remove_file(&full.journey_file);
    let _ = std::fs::remove_file(&reduced.journey_file);
}

/// The current room drawn with `inputs` at the App's phase.
fn draw(app: &App, inputs: &[RoomInput]) -> Vec<u8> {
    let room = &app.rooms[app.current];
    let phase = effective_room_phase(room.meta().id, app.t, &app.inputs, app.the_show);
    let mut raster = Raster::with_accent(SIZE.0, SIZE.1, room.meta().accent);
    room.render_input(&mut raster, phase, inputs);
    raster.to_rgba()
}

/// Press, let a frame pass, then drag: the order a real hand arrives in.
fn press_and_drag(app: &mut App, from: (f64, f64), to: (f64, f64)) {
    app.begin_pointer_at(from);
    app.advance_presentation_time(FRAME);
    app.move_pointer_to(to, true);
}

fn newest_point(inputs: &[RoomInput]) -> (f64, f64) {
    crate::hand_spring::open_hand(inputs)
        .expect("an open gesture")
        .1
}

#[test]
fn a_dial_glides_in_presented_frames_while_the_history_stays_exact() {
    let mut app = quiet_app("numinous_app_test_dial_glides", Motion::Full);
    select(&mut app, "times-tables");
    press_and_drag(&mut app, (0.2, 0.5), (0.8, 0.5));
    let accepted = app.inputs.clone();
    assert_eq!(newest_point(&accepted), (0.8, 0.5));

    // The frame drawn now still shows the dial where the hand last was.
    let presented = app.presented_room_inputs();
    let (x, y) = newest_point(&presented);
    assert!((x - 0.2).abs() < 1e-12 && y == 0.5, "({x}, {y})");
    app.advance_presentation_time(FRAME);
    let presented = app.presented_room_inputs();
    let (x, _) = newest_point(&presented);
    assert!(x > 0.2 && x < 0.8, "mid-glide the dial is between: {x}");

    // The readout names the picture, and both differ from the raw hand.
    let room = &app.rooms[app.current];
    let phase = effective_room_phase(room.meta().id, app.t, &app.inputs, app.the_show);
    assert_ne!(
        room.status_input(phase, &presented),
        room.status_input(phase, &accepted)
    );
    assert_ne!(draw(&app, &presented), draw(&app, &accepted));

    // Accepted play is untouched by any of it.
    assert_eq!(app.inputs, accepted, "the spring wrote into the history");
    for _ in 0..60 {
        app.advance_presentation_time(FRAME);
    }
    assert_eq!(app.inputs, accepted);
    // And at rest the presented frame is the raw frame, bit for bit.
    let rested = app.presented_room_inputs();
    assert_eq!(rested, accepted);
    assert_eq!(draw(&app, &rested), draw(&app, &accepted));
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn holds_clicks_and_flings_present_exactly_what_the_hand_did() {
    let mut checked = [false; 3];
    for room in numinous_core::all_rooms() {
        let verb = room.verb().unwrap_or("");
        let id = room.meta().id;
        let kind = if verb.starts_with("HOLD") {
            0
        } else if verb.starts_with("CLICK") {
            1
        } else if crate::room_input::room_keeps_drag_after_release(id) {
            2
        } else {
            continue;
        };
        if checked[kind] {
            continue;
        }
        checked[kind] = true;
        let mut app = quiet_app("numinous_app_test_exact_hands", Motion::Full);
        select(&mut app, id);
        press_and_drag(&mut app, (0.3, 0.4), (0.7, 0.6));
        app.advance_presentation_time(FRAME);
        assert_eq!(
            app.presented_room_inputs(),
            app.inputs,
            "{id} ({verb}) must present the raw hand"
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }
    assert_eq!(checked, [true; 3], "a hold, a click, and a fling room");
}

#[test]
fn reduced_motion_applies_a_dial_at_once() {
    let mut app = quiet_app("numinous_app_test_dial_direct", Motion::Reduced);
    select(&mut app, "times-tables");
    press_and_drag(&mut app, (0.2, 0.5), (0.8, 0.5));
    assert_eq!(app.presented_room_inputs(), app.inputs);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn the_show_presents_no_hand_at_all() {
    let mut app = quiet_app("numinous_app_test_show_no_hand", Motion::Full);
    select(&mut app, "times-tables");
    press_and_drag(&mut app, (0.2, 0.5), (0.8, 0.5));
    app.toggle_show();
    assert!(app.presented_room_inputs().is_empty());
    let _ = std::fs::remove_file(&app.journey_file);
}
