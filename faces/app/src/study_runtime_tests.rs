//! Headless acceptance tests for the App's optional study boundary.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use numinous_core::{RoomInput, StudyDepth, StudyLocale};
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

use super::gamepad::Command;
use super::menu::{MenuItemId, MenuOrigin, MenuRoute};
use super::{App, AudioProgram, Route};

type Files = BTreeMap<PathBuf, (Vec<u8>, SystemTime)>;

fn files_below(root: &Path) -> Files {
    fn visit(root: &Path, directory: &Path, files: &mut Files) {
        for entry in std::fs::read_dir(directory).expect("scratch directory") {
            let entry = entry.expect("scratch entry");
            let path = entry.path();
            let kind = entry.file_type().expect("scratch entry type");
            if kind.is_dir() {
                visit(root, &path, files);
            } else {
                assert!(kind.is_file(), "unexpected scratch entry: {path:?}");
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    (
                        std::fs::read(&path).expect("scratch file bytes"),
                        entry.metadata().unwrap().modified().unwrap(),
                    ),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn seed_stores(app: &App) {
    let root = app.preferences_file.parent().unwrap();
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::write(&app.journey_file, app.journey.to_text()).unwrap();
    std::fs::write(&app.scores_file, b"retained score bytes\n").unwrap();
    std::fs::write(&app.preferences_file, app.preferences().to_text()).unwrap();
    std::fs::write(&app.crash_log, b"retained diagnostic bytes\n").unwrap();
    let paths = super::local_state_paths();
    std::fs::write(paths.journal, b"retained journal bytes\n").unwrap();
    std::fs::write(paths.cairn, b"retained cairn bytes\n").unwrap();
}

fn select_room(app: &mut App, id: &str) {
    app.close_menu();
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == id)
        .unwrap();
    app.reset_current_room();
    app.motion = numinous_core::Motion::Full;
    app.banner = None;
    app.room_card = 0;
}

fn room_app(id: &str, name: &str) -> App {
    let mut app = super::tests::headless(name);
    select_room(&mut app, id);
    app.audio_program = AudioProgram::RoomScore;
    app.tune = Arc::new(vec![0.2, -0.1, 0.4, -0.3]);
    app.radio_track = Arc::new(vec![0.7, -0.6, 0.5, -0.4]);
    seed_stores(&app);
    app
}

fn route_state(app: &App) -> String {
    match &app.route {
        None => "catalog".to_string(),
        Some(Route::Wing(wing)) => format!("wing:{}:{:?}", wing.name, wing.rooms),
        Some(Route::Walk { walk, step }) => format!("walk:{}:{step}", walk.id),
    }
}

fn experiment_state(app: &App) -> String {
    format!(
        "{:?}",
        (
            &app.times_tables_aha,
            &app.buffon_aha,
            &app.galton_aha,
            &app.pendulum_aha,
            &app.kepler_aha,
            &app.parrondo_aha,
            &app.nontransitive_aha,
        )
    )
}

/// Physical capture and reader navigation may change. Accepted play may not.
struct RetainedPlay {
    scalars: String,
    route: String,
    inputs: Vec<RoomInput>,
    pokes: Vec<(f64, f64)>,
    life: String,
    camera: numinous_core::rooms::mandelbrot::MandelbrotCamera,
    experiments: String,
    journey: String,
    saved_journey: String,
    tune: Arc<Vec<f32>>,
    radio_track: Arc<Vec<f32>>,
    root: PathBuf,
    files: Files,
}

impl RetainedPlay {
    fn scalars(app: &App) -> String {
        format!(
            "{:?}",
            (
                (
                    app.current,
                    app.variation,
                    app.t,
                    app.paused,
                    app.time_scale,
                    app.visualizer_scale,
                    app.the_show,
                    app.chosen_experiment
                ),
                (
                    app.frame,
                    app.room_card,
                    app.screen_shake,
                    app.life_accumulator,
                    app.goal_announced,
                    app.show_crossfade_frames
                ),
                (
                    app.audio_program,
                    app.volume,
                    app.muted,
                    app.era,
                    app.motion,
                    app.preferred_window_mode,
                    app.radio,
                    app.radio_index,
                    app.radio_track_rate,
                    app.radio_until,
                    &app.radio_paths
                ),
            )
        )
    }

    fn capture(app: &App) -> Self {
        let root = app.preferences_file.parent().unwrap().to_path_buf();
        Self {
            scalars: Self::scalars(app),
            route: route_state(app),
            inputs: app.inputs.clone(),
            pokes: app.pokes.clone(),
            life: format!("{:?}", app.life_session),
            camera: app.mandelbrot_camera,
            experiments: experiment_state(app),
            journey: app.journey.to_text(),
            saved_journey: app.journey_saved.to_text(),
            tune: app.tune.clone(),
            radio_track: app.radio_track.clone(),
            files: files_below(&root),
            root,
        }
    }

    fn assert_play_unchanged(&self, app: &App) {
        assert_eq!(Self::scalars(app), self.scalars, "play and audio settings");
        assert_eq!(route_state(app), self.route, "chosen catalog route");
        assert_eq!(app.inputs, self.inputs, "accepted gesture history");
        assert_eq!(app.pokes, self.pokes, "accepted poke trail");
        assert!(
            format!("{:?}", app.life_session) == self.life,
            "full Life state changed"
        );
        assert_eq!(app.mandelbrot_camera, self.camera, "persistent camera");
        assert_eq!(
            experiment_state(app),
            self.experiments,
            "all staged experiments"
        );
        assert_eq!(app.journey.to_text(), self.journey, "Journey state");
        assert_eq!(
            app.journey_saved.to_text(),
            self.saved_journey,
            "persisted Journey baseline"
        );
        assert!(
            Arc::ptr_eq(&app.tune, &self.tune),
            "room audio buffer replaced"
        );
        assert!(
            Arc::ptr_eq(&app.radio_track, &self.radio_track),
            "radio audio buffer replaced"
        );
    }

    fn assert_unchanged(&self, app: &App) {
        self.assert_play_unchanged(app);
        assert_eq!(
            files_below(&self.root),
            self.files,
            "local files changed while reading"
        );
    }
}

fn key(text: &str) -> Key {
    Key::Character(text.into())
}

fn reader_depth(app: &App) -> StudyDepth {
    app.study.as_ref().expect("reader is open").reader.depth()
}

#[test]
fn fresh_and_paused_rooms_offer_explanation_and_direct_mathematics() {
    for id in std::iter::once("lissajous").chain(numinous_core::ENGINEERED_AHA_ROOM_IDS) {
        for paused in [false, true] {
            let mut app = room_app(id, &format!("study-fresh-{id}-{paused}"));
            app.paused = paused;
            let before = RetainedPlay::capture(&app);
            assert!(app.journey.visited.is_empty());
            assert!(app.journey.consolidated.is_empty());
            assert!(!app.chosen_experiment);

            assert!(app.handle_study_key(&key("E"), false));
            assert_eq!(reader_depth(&app), StudyDepth::Explanation);
            assert!(app.handle_study_key(&Key::Named(NamedKey::Enter), false));
            assert_eq!(reader_depth(&app), StudyDepth::Mathematics);
            let document = app.study.as_ref().unwrap().reader.document();
            assert_eq!(document.room_id, id);
            // Bound to the authored registry rather than to one room's name, so
            // adding a treatment does not silently make this assertion wrong.
            assert_eq!(
                document.has_depth(StudyDepth::Mathematics),
                numinous_core::AUTHORED_MATHEMATICS_ROOMS.contains(&id),
                "{id} disagrees with the advertised authored set"
            );
            assert!(document.has_depth(StudyDepth::Notes));
            app.advance_room_tick(0.05, 0.05, false);
            before.assert_unchanged(&app);
            assert!(app.handle_study_key(&Key::Named(NamedKey::Escape), false));
            assert!(app.study.is_none());
            assert_eq!(app.paused, paused);
            before.assert_unchanged(&app);
        }
    }
}

#[test]
fn every_chosen_experiment_can_be_read_before_a_wager_and_during_its_morph() {
    for id in numinous_core::ENGINEERED_AHA_ROOM_IDS {
        let mut app = room_app(id, &format!("study-experiment-{id}"));
        app.toggle_chosen_experiment();
        let untouched = RetainedPlay::capture(&app);
        assert!(app.open_room_study());
        assert!(app.handle_study_key(&Key::Named(NamedKey::Enter), false));
        untouched.assert_unchanged(&app);
        app.close_room_study();

        // Use the actual shared App gesture and wager-band paths.
        app.begin_pointer_at((0.0, 0.5));
        app.end_pointer_at((0.0, 0.5));
        app.begin_pointer_at((0.8, 0.95));
        assert!(
            app.can_advance_chosen_experiment(),
            "{id} earned a connection"
        );
        assert!(app.advance_chosen_experiment());
        let morph = RetainedPlay::capture(&app);
        app.handle_gamepad_command(Command::Inspect);
        assert_eq!(reader_depth(&app), StudyDepth::Explanation);
        app.handle_gamepad_command(Command::PrimaryDown);
        assert_eq!(reader_depth(&app), StudyDepth::Mathematics);
        for _ in 0..100 {
            app.advance_room_tick(0.05, 0.05, false);
        }
        morph.assert_unchanged(&app);
        app.handle_gamepad_command(Command::Inspect);
        assert!(app.study.is_none());
        assert!(app.chosen_experiment);
        assert!(app.journey.consolidated.is_empty());
        morph.assert_unchanged(&app);
    }
}

#[test]
fn cabinet_shortcuts_and_menu_action_restore_the_exact_navigation_state() {
    for origin in [MenuOrigin::Launch, MenuOrigin::Room] {
        for controller in [false, true] {
            let mut app = room_app("lissajous", "study-cabinet-return");
            app.menu.open_home(origin);
            app.show_help = true;
            app.menu.focus(MenuItemId::Games);
            let menu = app.menu.clone();
            let play = RetainedPlay::capture(&app);
            if controller {
                app.handle_gamepad_command(Command::Inspect);
            } else {
                assert!(app.handle_study_key(&key("e"), false));
            }
            assert!(app.study.is_some());
            assert!(!app.show_help);
            assert!(!app.menu.is_open());
            app.close_room_study();
            assert_eq!(app.menu, menu);
            assert!(app.show_help);
            play.assert_unchanged(&app);
        }
    }

    let mut app = room_app("lissajous", "study-cabinet-action");
    app.apply_menu_intent(super::menu::MenuIntent::EnterWalk);
    app.open_home_menu();
    app.menu.focus(MenuItemId::Explain);
    let menu = app.menu.clone();
    let play = RetainedPlay::capture(&app);
    app.activate_selected_menu_action();
    assert!(app.study.is_some());
    app.handle_gamepad_command(Command::Back);
    assert_eq!(app.menu, menu);
    play.assert_unchanged(&app);

    // The reader's explicit entry also preserves a Cabinet subpage stack.
    assert!(app.menu.focus(MenuItemId::Controls));
    app.activate_selected_menu_action();
    assert_eq!(app.menu.route(), MenuRoute::Controls);
    let menu = app.menu.clone();
    assert!(app.open_room_study());
    app.close_room_study();
    assert_eq!(app.menu, menu);
    assert_eq!(app.menu.origin(), MenuOrigin::Room);
}

#[test]
fn reader_controls_frames_and_real_ticks_preserve_the_retained_room() {
    for id in ["lissajous", "game-of-life", "mandelbrot"] {
        let mut app = room_app(id, &format!("study-barriers-{id}"));
        app.t = 0.37;
        app.time_scale = 2.0;
        app.visualizer_scale = 1.3;
        app.frame = 53;
        app.screen_shake = 7;
        assert!(app.life_session.launch((0.2, 0.2)));
        app.life_session.advance();
        app.life_accumulator = super::LIFE_STEP_SECONDS * 0.9;
        app.mandelbrot_camera.advance(0.8);
        app.begin_pointer_at((0.2, 0.3));
        app.move_pointer_to((0.6, 0.6), true);
        assert!(!app.inputs.is_empty());
        app.banner = None;
        let wing = numinous_core::wings()
            .into_iter()
            .find(|wing| wing.rooms.contains(&app.current))
            .unwrap();
        app.route = Some(Route::Wing(wing));
        assert!(app.journey.visit(id));
        app.journey.play();
        app.journey_saved = app.journey.clone();
        seed_stores(&app);
        let before = RetainedPlay::capture(&app);
        assert!(app.study_frame(360, 240).is_none());
        assert!(app.open_room_study());
        assert!(app.handle_study_key(&Key::Named(NamedKey::Enter), false));

        for (width, height) in [(360, 240), (900, 700)] {
            let frame = app.study_frame(width, height).expect("reader frame");
            assert_eq!(frame.len(), width * height * 4);
            assert!(
                frame
                    .chunks_exact(4)
                    .any(|pixel| pixel[..3] != [10, 11, 15])
            );
            before.assert_unchanged(&app);
        }
        for named in [
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::PageUp,
            NamedKey::PageDown,
            NamedKey::Home,
            NamedKey::End,
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
            NamedKey::Enter,
            NamedKey::Space,
            NamedKey::Tab,
            NamedKey::F4,
            NamedKey::F6,
            NamedKey::F9,
        ] {
            assert!(app.handle_study_key(&Key::Named(named), false));
        }
        for text in ["u", "r", "q", "a", "d", "w", "s", "b", "j", "1", "`"] {
            assert!(app.handle_study_key(&key(text), false));
        }
        for command in [
            Command::Reset,
            Command::Pause,
            Command::PreviousRoom,
            Command::NextRoom,
            Command::Slower,
            Command::Faster,
            Command::CycleEra,
            Command::CycleRadio,
            Command::PhaseDelta(0.4),
            Command::CancelPointer,
        ] {
            app.handle_gamepad_command(command);
        }
        app.begin_pointer_at((0.5, 0.5));
        app.move_pointer_to((0.9, 0.95), true);
        app.end_pointer_at((0.9, 0.95));
        app.mouse = (450.0, 350.0);
        assert!(app.handle_study_window_pointer(ElementState::Released));
        for lines in [0.25, -0.25, 5.0, f64::NAN, f64::INFINITY] {
            assert!(app.apply_wheel_delta(lines));
        }
        assert!(app.clear_study_pointer());
        for _ in 0..8 {
            app.advance_room_tick(0.05, 0.05, false);
        }
        assert!(app.study.is_some());
        assert!(!app.quit_requested);
        assert!(!app.console.is_open());
        assert_eq!(app.study_locale.as_str(), "en");
        before.assert_unchanged(&app);

        app.close_room_study();
        before.assert_unchanged(&app);
        let phase = app.t;
        let generation = app.life_session.generation();
        let camera = app.mandelbrot_camera;
        app.advance_room_tick(0.05, 0.05, false);
        assert_ne!(app.t, phase, "the real tick resumes after leaving study");
        if id == "game-of-life" {
            assert!(app.life_session.generation() > generation);
        }
        if id == "mandelbrot" {
            assert_ne!(app.mandelbrot_camera, camera);
        }
    }
}

#[test]
fn a_reader_owned_controller_batch_cannot_spill_back_into_play() {
    for already_open in [false, true] {
        let mut app = room_app("lissajous", "study-controller-batch");
        app.t = 0.41;
        let before = RetainedPlay::capture(&app);
        if already_open {
            assert!(app.open_room_study());
        }
        let mut commands = if already_open {
            Vec::new()
        } else {
            vec![Command::Inspect]
        };
        commands.extend([
            Command::PrimaryDown,
            Command::Back,
            Command::Reset,
            Command::NextRoom,
            Command::PhaseDelta(0.4),
            Command::Slower,
            Command::PrimaryDown,
            Command::PointerMoved {
                point: (0.8, 0.9),
                held: true,
            },
            Command::PrimaryUp,
            Command::CycleRadio,
            Command::CycleEra,
            Command::Menu,
        ]);
        let captured = app.handle_gamepad_batch(commands);
        assert!(captured);
        assert!(app.study.is_none());
        assert!(!app.show_help);
        app.advance_room_tick(0.05, 0.05, captured);
        before.assert_unchanged(&app);
        let captured = app.handle_gamepad_batch(Vec::new());
        assert!(!captured);
        app.advance_room_tick(0.05, 0.05, captured);
        assert_ne!(app.t, 0.41);
    }
}

#[test]
fn held_pointer_releases_and_keyboard_repeats_stay_captured_after_close() {
    let mut app = room_app("lissajous", "study-held-input");
    app.begin_pointer_at((0.15, 0.25));
    app.move_pointer_to((0.55, 0.65), true);
    assert!(app.poking);
    let before = RetainedPlay::capture(&app);
    assert!(app.handle_study_key(&key("e"), false));
    assert!(!app.poking);
    assert!(!app.dragging);
    for captured in [key("r"), key("u"), Key::Named(NamedKey::Enter)] {
        assert!(app.handle_study_key(&captured, false));
    }
    assert!(app.handle_study_key(&Key::Named(NamedKey::Escape), false));
    assert!(app.study.is_none());
    app.move_pointer_to((0.95, 0.95), true);
    app.end_pointer_at((0.95, 0.95));
    app.gamepad.set_cursor_for_test((0.85, 0.85));
    app.handle_gamepad_command(Command::PrimaryUp);
    for captured in [
        key("e"),
        key("r"),
        key("u"),
        Key::Named(NamedKey::Enter),
        Key::Named(NamedKey::Escape),
    ] {
        assert!(app.handle_study_key(&captured, true));
    }
    assert!(app.study.is_none());
    before.assert_unchanged(&app);

    app.release_study_key(&key("r"));
    assert!(!app.handle_study_key(&key("r"), false));
    app.begin_pointer_at((0.4, 0.45));
    app.end_pointer_at((0.4, 0.45));
    assert_eq!(&app.inputs[..before.inputs.len()], before.inputs.as_slice());
    assert!(matches!(
        app.inputs.last(),
        Some(RoomInput::PointerUp {
            x: 0.4,
            y: 0.45,
            ..
        })
    ));
    assert!(app.inputs.len() > before.inputs.len());
    assert!(
        app.handle_study_key(&key("e"), false),
        "a fresh press recovers without a delivered release"
    );
    assert!(app.study.is_some());
}

#[test]
fn keys_held_before_a_brief_reader_visit_require_release_or_a_fresh_press() {
    for (pressed, repeated) in [
        (
            Key::Named(NamedKey::ArrowRight),
            Key::Named(NamedKey::ArrowRight),
        ),
        (key("R"), key("r")),
    ] {
        let mut app = room_app("lissajous", "study-preheld-key");
        // This is the boundary called before ordinary room dispatch. The key
        // starts in play, and no repeat arrives during the reader visit.
        assert!(!app.handle_study_key(&pressed, false));
        let before = RetainedPlay::capture(&app);
        app.handle_gamepad_command(Command::Inspect);
        assert!(app.study.is_some());
        app.handle_gamepad_command(Command::Back);
        assert!(app.study.is_none());
        assert!(app.handle_study_key(&repeated, true));
        before.assert_unchanged(&app);

        app.release_study_key(&repeated);
        assert!(!app.handle_study_key(&repeated, false));
        assert!(!app.handle_study_key(&repeated, true));

        // A focus interruption may lose a release. Retaining its capture must
        // not prevent an intentional fresh press from recovering afterward.
        app.handle_gamepad_command(Command::Inspect);
        assert!(app.clear_study_pointer());
        app.handle_gamepad_command(Command::Back);
        assert!(app.handle_study_key(&repeated, true));
        assert!(!app.handle_study_key(&repeated, false));
        assert!(!app.handle_study_key(&repeated, true));
        before.assert_unchanged(&app);
    }
}

#[test]
fn reader_capture_retires_a_consumed_experiment_press_before_fresh_control() {
    let mut app = room_app("times-tables", "study-consumed-primary");
    app.toggle_chosen_experiment();
    app.begin_pointer_at((0.0, 0.5));
    app.end_pointer_at((0.0, 0.5));
    app.begin_pointer_at((0.8, 0.95));
    assert!(app.can_advance_chosen_experiment());
    app.handle_gamepad_command(Command::PrimaryDown);
    assert!(app.experiment_primary_consumed);
    let before = RetainedPlay::capture(&app);

    app.handle_gamepad_command(Command::Inspect);
    assert!(app.study.is_some());
    // Gamepad capture absorbs the physical release without a PrimaryUp
    // command. The App must recover even though no such command arrives.
    assert!(app.handle_gamepad_batch(Vec::new()));
    app.handle_gamepad_command(Command::Back);
    assert!(app.study.is_none());
    assert!(!app.experiment_primary_consumed);
    before.assert_unchanged(&app);

    app.gamepad.set_cursor_for_test((0.4, 0.45));
    app.handle_gamepad_command(Command::PrimaryDown);
    assert!(app.poking, "the first fresh primary press is admitted");
    assert!(app.inputs.len() > before.inputs.len());
    assert_eq!(&app.inputs[..before.inputs.len()], before.inputs.as_slice());
    assert!(matches!(
        app.inputs.last(),
        Some(RoomInput::PointerDown {
            x: 0.4,
            y: 0.45,
            ..
        })
    ));
    app.handle_gamepad_command(Command::PrimaryUp);
    assert!(!app.poking);
    assert!(matches!(
        app.inputs.last(),
        Some(RoomInput::PointerUp {
            x: 0.4,
            y: 0.45,
            ..
        })
    ));
}

#[test]
fn explicit_study_language_changes_only_the_preference_and_reader() {
    let mut app = room_app("lissajous", "study-language-preference");
    assert!(app.open_room_study());
    assert!(app.handle_study_key(&Key::Named(NamedKey::Enter), false));
    let before = RetainedPlay::capture(&app);
    let preferences = app.preferences();
    let ids: Vec<_> = app
        .study
        .as_ref()
        .unwrap()
        .reader
        .document()
        .blocks
        .iter()
        .map(|block| block.id.clone())
        .collect();
    assert!(app.handle_study_key(&key("L"), false));
    assert_eq!(app.study_locale.as_str(), "ja");
    assert_eq!(reader_depth(&app), StudyDepth::Mathematics);
    let document = app.study.as_ref().unwrap().reader.document();
    assert_eq!(document.locale.requested.as_str(), "ja");
    assert_eq!(document.locale.resolved, "ja");
    assert_eq!(
        document
            .blocks
            .iter()
            .map(|block| block.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    before.assert_play_unchanged(&app);
    let mut expected = preferences;
    expected.study_locale = StudyLocale::parse("ja").unwrap();
    assert_eq!(
        numinous_core::read_app_preferences_file(&app.preferences_file).unwrap(),
        expected
    );
    let after = files_below(&before.root);
    let preference = app.preferences_file.strip_prefix(&before.root).unwrap();
    assert_eq!(after.len(), before.files.len());
    assert_ne!(after[preference].0, before.files[preference].0);
    for (path, contents) in &before.files {
        if path != preference {
            assert_eq!(&after[path], contents, "changed {path:?}");
        }
    }
    assert!(app.handle_study_key(&key("L"), true));
    assert_eq!(
        files_below(&before.root),
        after,
        "repeat is not another preference action"
    );
    app.close_room_study();
    before.assert_play_unchanged(&app);
    assert_eq!(files_below(&before.root), after);
}

#[test]
fn study_shortcuts_leave_existing_text_and_activity_owners_in_control() {
    let mut app = room_app("lissajous", "study-studio-key-owner");
    app.enter_studio();
    assert!(!app.handle_study_key(&key("e"), false));
    assert!(!app.handle_study_gamepad(Command::Inspect));
    let length = app.studio_panel.source_len();
    let _ = app.studio_panel.push_text("e");
    assert_eq!(app.studio_panel.source_len(), length + 1);
    assert!(app.study.is_none());

    let mut app = room_app("lissajous", "study-console-key-owner");
    app.console.open();
    assert!(!app.handle_study_key(&key("E"), false));
    assert!(!app.handle_study_gamepad(Command::Inspect));
    assert!(app.handle_console_key(&key("E")));
    assert_eq!(app.console.buffer(), "E");
    assert!(app.study.is_none());

    let games: [fn(&mut App); 5] = [
        App::quiz_next,
        App::munch_start,
        App::nim_start,
        App::gauntlet_start,
        App::arcade_start,
    ];
    for (index, start) in games.into_iter().enumerate() {
        let mut app = room_app("lissajous", &format!("study-game-key-owner-{index}"));
        start(&mut app);
        let before = RetainedPlay::capture(&app);
        assert!(!app.handle_study_key(&key("e"), false));
        assert!(!app.open_room_study());
        app.handle_gamepad_command(Command::Inspect);
        assert!(app.study.is_none());
        assert!(app.activity_kind().is_some());
        before.assert_unchanged(&app);
    }

    let mut app = room_app("lissajous", "study-settings-key-owner");
    app.open_home_menu();
    app.menu.focus(MenuItemId::Settings);
    app.activate_selected_menu_action();
    assert_eq!(app.menu.route(), MenuRoute::Settings);
    assert!(!app.handle_study_key(&key("E"), false));
    assert!(!app.handle_study_gamepad(Command::Inspect));
    let era = app.era;
    assert!(app.handle_menu_key(&key("E"), false));
    assert_eq!(app.era, era.next());
    assert_eq!(app.menu.focused(), MenuItemId::VisualEra);
    assert!(app.study.is_none());
}
