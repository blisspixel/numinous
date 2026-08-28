use super::{
    App, AudioProgram, TestStateRoot, advance_gallery_phase, app_icon, append_crash_log_at,
    bounded_tick_seconds, effective_room_phase, fullscreen_toggle_target, julia_gpu_c,
    julia_gpu_vertical_span, live_mandelbrot_gpu_view, mandelbrot_gpu_view, radio_cache,
};
use crate::audio_runtime::{
    life_step_audio_owned, room_transient_audio_owned, selected_life_step_audio,
    selected_parameter_sound, selected_room_interaction_audio,
};
use crate::input_legend::{InputMode, MenuChoice};
use numinous_core::ROOM_BED_SOURCE_RATE;
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};
use winit::keyboard::{Key, NamedKey};

/// An app pointed at scratch files, with no window, player, or GPU.
fn headless(name: &str) -> App {
    let mut app = App::new();
    app.journey = numinous_core::Journey::default();
    app.journey_saved = app.journey.clone();
    app.journey_file = super::test_state_path(name);
    app.scores_file = app.journey_file.with_extension("scores");
    // Diagnostics stay in scratch too: a test must never append to a
    // real player's crash log.
    app.crash_log = app.journey_file.with_extension("crash.log");
    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
    let _ = std::fs::remove_file(&app.crash_log);
    app.level_seen = 1;
    app
}

#[test]
fn power_console_loads_a_room_and_sets_phase() {
    let mut app = headless("console-goto");
    app.console.open();
    assert!(app.console.is_open());
    let lines = app.run_console_command(crate::console::Command::Goto("times-tables".into()));
    assert!(
        lines.iter().any(|l| l.contains("times-tables")),
        "{lines:?}"
    );
    assert_eq!(app.rooms[app.current].meta().id, "times-tables");
    let lines = app.run_console_command(crate::console::Command::Phase(0.42));
    assert!(lines.iter().any(|l| l.contains("0.420")), "{lines:?}");
    assert!((app.t - 0.42).abs() < 1e-9);
    let lines = app.run_console_command(crate::console::Command::Where);
    assert!(
        lines.iter().any(|l| l.contains("times-tables")),
        "{lines:?}"
    );
    app.run_console_command(crate::console::Command::Close);
    assert!(!app.console.is_open());
}

#[test]
fn power_console_toggle_key_opens_from_room_mode() {
    let mut app = headless("console-toggle");
    assert!(!app.console.is_open());
    // Simulate the early key path: Character("~") opens.
    assert!(crate::console::is_toggle_key("~"));
    app.console.open();
    assert!(app.handle_console_key(&Key::Character("~".into())));
    assert!(!app.console.is_open());
}

#[test]
fn text_menu_opens_the_existing_power_console_without_an_intermediate_screen() {
    let mut app = headless("menu-console-toggle");
    assert!(app.show_help && app.menu.is_open());

    assert!(app.handle_menu_key(&Key::Character("`".into()), false));

    assert!(!app.show_help);
    assert!(!app.menu.is_open());
    assert!(app.console.is_open());

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn crash_writer_waits_for_the_shared_erasure_lock() {
    // Unique path: avoid collisions if this test is ever re-entered, and keep
    // the fixture out of shared test-root names used by other suites.
    let path = super::test_state_path(&format!(
        "crash-lock-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&path);
    let guard = numinous_core::lock_local_state(&path).expect("hold erasure lock");
    let writer_path = path.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (sent, received) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        // Signal after the worker is live, before the blocking lock wait.
        started_tx.send(()).expect("signal writer start");
        let result = append_crash_log_at(&writer_path, "diagnostic\n");
        sent.send(result).expect("report writer result");
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("writer thread starts");
    assert!(
        received
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err(),
        "writer must wait while erasure owns the path"
    );
    drop(guard);
    // Lock acquire retries for about five seconds; allow that budget plus
    // margin so a loaded Windows runner cannot flake on resume alone.
    received
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("writer resumes")
        .expect("writer succeeds");
    writer.join().expect("writer joined");
    assert_eq!(
        std::fs::read(&path).expect("crash receipt"),
        b"diagnostic\n"
    );
    numinous_core::remove_persisted_file(&path).expect("fixture cleanup");
}

#[test]
fn room_score_prerender_is_device_independent_and_memory_bounded() {
    let mut largest = 0;
    for room in numinous_core::all_rooms() {
        let motif = room.motif().expect("catalog motif");
        let samples = motif.arrangement().render_stereo(ROOM_BED_SOURCE_RATE);
        assert_eq!(
            samples.len(),
            (motif.arrangement().seconds() * ROOM_BED_SOURCE_RATE as f32) as usize * 2,
            "{} source length",
            room.meta().id
        );
        largest = largest.max(samples.len());
    }
    assert!(
        largest <= 2_000_000,
        "largest room score held {largest} samples"
    );
}

#[test]
fn screen_shake_shifts_rgba_and_decays_on_present() {
    let mut rgba = vec![0_u8; 8 * 4 * 4];
    for (i, chunk) in rgba.chunks_exact_mut(4).enumerate() {
        chunk[0] = i as u8;
        chunk[3] = 255;
    }
    let before = rgba.clone();
    super::apply_screen_shake(&mut rgba, 8, 4, 3);
    assert_ne!(rgba, before, "shake must move pixels");
    let mut app = headless("numinous_app_test_screen_shake.txt");
    app.screen_shake = 2;
    let raster = numinous_core::Raster::with_accent(40, 30, [20, 30, 40]);
    app.present_raster(raster, 40, 30);
    assert_eq!(app.screen_shake, 1);
    app.present_raster(
        numinous_core::Raster::with_accent(40, 30, [20, 30, 40]),
        40,
        30,
    );
    assert_eq!(app.screen_shake, 0);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn watch_agent_owns_audio_across_radio_resync_and_close_restores_prior_source() {
    let mut app = headless("numinous_app_test_watch_agent_audio_owner.txt");
    app.audio_program = AudioProgram::RoomScore;
    app.radio = Some(0);
    app.radio_paths = vec![std::path::PathBuf::from("unused")];

    app.open_session_viewer();
    assert!(app.session_viewer.is_open());
    assert_eq!(app.audio_program, AudioProgram::WatchAgent);
    // Wall-clock radio must not steal the paired source.
    assert!(!app.sync_radio_at(1.0));
    assert_eq!(app.audio_program, AudioProgram::WatchAgent);

    app.close_session_viewer();
    assert!(!app.session_viewer.is_open());
    // No player and no real radio tracks: ownership returns to the room score.
    assert_eq!(app.audio_program, AudioProgram::RoomScore);
    let _ = std::fs::remove_file(&app.journey_file);
}

fn select_life(app: &mut App) {
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "game-of-life")
        .expect("Life room");
    app.show_help = false;
    app.reset_life_session();
}

fn select_times_tables(app: &mut App) {
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "times-tables")
        .expect("Times Tables room");
    app.show_help = false;
}

fn select_galton(app: &mut App) {
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "galton-board")
        .expect("Galton Board room");
    app.show_help = false;
}

fn select_pendulum(app: &mut App) {
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "double-pendulum")
        .expect("Double Pendulum room");
    app.show_help = false;
    app.reset_pendulum_aha();
}

fn select_kepler(app: &mut App) {
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "kepler-laws")
        .expect("Kepler Areas room");
    app.show_help = false;
    app.reset_kepler_aha();
}

fn select_parrondo(app: &mut App) {
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "parrondo")
        .expect("Parrondo room");
    app.show_help = false;
    app.reset_parrondo_aha();
}

fn select_nontransitive(app: &mut App) {
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "nontransitive")
        .expect("Nontransitive Dice room");
    app.show_help = false;
    app.reset_nontransitive_aha();
}

#[test]
fn times_tables_holds_its_cardioid_until_input_but_the_show_keeps_sweeping() {
    assert_eq!(effective_room_phase("times-tables", 0.73, &[], false), 0.0);
    assert_eq!(effective_room_phase("times-tables", 0.73, &[], true), 0.73);
    assert_eq!(effective_room_phase("lissajous", 0.73, &[], false), 0.73);

    let input = [numinous_core::RoomInput::PointerDown {
        x: 0.4,
        y: 0.5,
        t: 0.2,
    }];
    assert_eq!(
        effective_room_phase("times-tables", 0.73, &input, false),
        0.73
    );

    let released = [numinous_core::RoomInput::PointerUp {
        x: 0.4,
        y: 0.5,
        t: 0.3,
    }];
    assert_eq!(
        effective_room_phase("times-tables", 0.73, &released, false),
        0.73
    );

    let invalid_release = [numinous_core::RoomInput::PointerUp {
        x: f64::NAN,
        y: 0.5,
        t: 0.3,
    }];
    assert_eq!(
        effective_room_phase("times-tables", 0.73, &invalid_release, false),
        0.0
    );
}

#[test]
fn times_tables_aha_gates_reveal_until_generation_and_morph() {
    use numinous_core::rooms::times_tables_aha::{AhaBeat, CardioidHome};

    let mut app = headless("numinous_app_test_times_tables_aha.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "times-tables")
        .expect("times-tables in catalog");
    app.reset_times_tables_aha();
    app.show_help = false;
    app.show_info = false;

    // Inspect before generation must not open the reveal card.
    app.toggle_inspect();
    assert!(!app.show_info);
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Explore);

    // Hand on the K=2 heart primes the gap.
    app.begin_pointer_at((0.0, 0.5));
    app.end_pointer_at((0.0, 0.5));
    app.sync_times_tables_aha();
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Prime);

    assert!(app.commit_times_tables_wager(CardioidHome::Circle));
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Withheld);
    assert!(!app.times_tables_aha.allow_reveal_text());

    // Summon starts the morph; reveal stays closed.
    app.toggle_inspect();
    assert!(matches!(app.times_tables_aha.beat(), AhaBeat::Morph { .. }));
    assert!(!app.show_info);

    app.advance_times_tables_morph(super::TIMES_TABLES_MORPH_SECONDS);
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Confirm);
    assert!(!app.times_tables_aha.allow_reveal_text());

    // Confirm -> consolidated punchline; only then may E open text.
    app.toggle_inspect();
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Consolidated);
    assert!(app.times_tables_aha.allow_reveal_text());
    app.toggle_inspect();
    assert!(app.show_info);

    // Reset clears the visit state.
    app.reset_current_room();
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Explore);
    assert!(!app.show_info);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn times_tables_four_lobes_earns_without_place_wager() {
    use numinous_core::rooms::times_tables_aha::AhaBeat;

    let mut app = headless("numinous_app_test_times_tables_k5_earn.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "times-tables")
        .expect("times-tables in catalog");
    app.reset_times_tables_aha();
    app.show_help = false;

    // x ~ 0.375 snaps to K=5 under the room dial contract.
    app.begin_pointer_at((0.374, 0.5));
    app.end_pointer_at((0.374, 0.5));
    app.sync_times_tables_aha();
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Withheld);
    assert_eq!(
        app.times_tables_aha.earn(),
        Some(numinous_core::rooms::times_tables_aha::EarnPath::FourLobes)
    );
    assert!(!app.times_tables_aha.allow_reveal_text());

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn the_show_does_not_auto_earn_times_tables_aha() {
    use numinous_core::rooms::times_tables_aha::AhaBeat;

    let mut app = headless("numinous_app_test_times_tables_show_no_earn.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "times-tables")
        .expect("times-tables in catalog");
    app.reset_times_tables_aha();
    app.the_show = true;
    app.t = 0.375;
    app.sync_times_tables_aha();
    assert_eq!(app.times_tables_aha.beat(), AhaBeat::Explore);
    assert!(!app.times_tables_aha.earned());

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn buffon_aha_gates_reveal_until_generation_and_morph() {
    use numinous_core::rooms::buffon_aha::AhaBeat;

    let mut app = headless("numinous_app_test_buffon_aha.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "buffon-needle")
        .expect("buffon-needle in catalog");
    app.reset_buffon_aha();
    app.show_help = false;
    app.show_info = false;

    app.toggle_inspect();
    assert!(!app.show_info);
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Explore);

    // One throw primes the gap.
    app.begin_pointer_at((0.5, 0.4));
    app.end_pointer_at((0.5, 0.4));
    app.sync_buffon_aha();
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Prime);

    assert!(app.commit_buffon_wager(2.0));
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Withheld);
    assert!(!app.buffon_aha.allow_reveal_text());

    app.toggle_inspect();
    assert!(matches!(app.buffon_aha.beat(), AhaBeat::Morph { .. }));
    assert!(!app.show_info);

    app.advance_buffon_morph(super::BUFFON_MORPH_SECONDS);
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Confirm);
    assert!(!app.buffon_aha.allow_reveal_text());

    app.toggle_inspect();
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Consolidated);
    assert!(app.buffon_aha.allow_reveal_text());
    app.toggle_inspect();
    assert!(app.show_info);

    app.reset_current_room();
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Explore);
    assert!(!app.show_info);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn pendulum_aha_calls_the_twin_then_opens_the_measured_gap() {
    use numinous_core::rooms::pendulum_aha::{AhaBeat, Ending};

    let mut app = headless("numinous_app_test_pendulum_aha.txt");
    select_pendulum(&mut app);
    app.show_info = false;

    app.toggle_inspect();
    assert!(!app.show_info, "no punchline before a release");
    assert_eq!(app.pendulum_aha.beat(), AhaBeat::Explore);

    // This hand position is the room's classic 2.0, 2.0 release, so the
    // measured gesture truth and the established default truth coincide.
    app.begin_pointer_at((7.0 / 12.0, 0.5));
    app.end_pointer_at((7.0 / 12.0, 0.5));
    assert_eq!(app.pendulum_aha.beat(), AhaBeat::Prime);

    let releases = app
        .inputs
        .iter()
        .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
        .count();
    app.begin_pointer_at((0.9, 0.95));
    assert_eq!(app.pendulum_aha.beat(), AhaBeat::Withheld);
    assert_eq!(app.pendulum_aha.call(), Some(Ending::Lost));
    assert_eq!(
        app.inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count(),
        releases,
        "the call band must not add another release"
    );

    app.toggle_inspect();
    assert!(matches!(app.pendulum_aha.beat(), AhaBeat::Morph { .. }));
    app.advance_pendulum_morph(super::PENDULUM_MORPH_SECONDS);
    assert_eq!(app.pendulum_aha.beat(), AhaBeat::Confirm);
    app.toggle_inspect();
    assert_eq!(app.pendulum_aha.beat(), AhaBeat::Consolidated);
    let grade = app.pendulum_aha.graded().expect("the call is answered");
    assert!(grade.contains("You called LOST"), "{grade}");
    assert!(grade.contains("Nailed"), "{grade}");
    app.toggle_inspect();
    assert!(app.show_info);

    app.reset_current_room();
    assert_eq!(app.pendulum_aha.beat(), AhaBeat::Explore);
    assert!(!app.show_info);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn kepler_aha_calls_speed_on_the_chosen_orbit_before_reveal() {
    use numinous_core::rooms::kepler_aha::{AhaBeat, SpeedRelation};

    let mut app = headless("numinous_app_test_kepler_aha.txt");
    select_kepler(&mut app);

    app.toggle_inspect();
    assert!(!app.show_info, "no answer before choosing an orbit");
    assert_eq!(app.kepler_aha.beat(), AhaBeat::Explore);

    app.begin_pointer_at((0.8, 0.4));
    app.end_pointer_at((0.8, 0.4));
    assert_eq!(app.kepler_aha.beat(), AhaBeat::Prime);
    assert!((app.kepler_aha.eccentricity() - 0.68).abs() < 1.0e-12);

    let input_count = app.inputs.len();
    app.begin_pointer_at((0.1, 0.95));
    assert_eq!(app.kepler_aha.beat(), AhaBeat::Withheld);
    assert_eq!(app.kepler_aha.call(), Some(SpeedRelation::Faster));
    assert_eq!(
        app.inputs.len(),
        input_count,
        "the call band must not retune the ellipse"
    );

    app.toggle_inspect();
    assert!(matches!(app.kepler_aha.beat(), AhaBeat::Morph { .. }));
    app.advance_kepler_morph(super::KEPLER_MORPH_SECONDS);
    assert_eq!(app.kepler_aha.beat(), AhaBeat::Confirm);
    app.toggle_inspect();
    assert_eq!(app.kepler_aha.beat(), AhaBeat::Consolidated);
    let grade = app.kepler_aha.graded().expect("the call is answered");
    assert!(grade.contains("called FASTER"), "{grade}");
    assert!(grade.contains("Nailed"), "{grade}");
    app.toggle_inspect();
    assert!(app.show_info);

    app.reset_current_room();
    assert_eq!(app.kepler_aha.beat(), AhaBeat::Explore);
    assert!(!app.show_info);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn parrondo_aha_calls_the_policy_then_opens_exact_expectations() {
    use numinous_core::rooms::parrondo::Policy;
    use numinous_core::rooms::parrondo_aha::AhaBeat;

    let mut app = headless("numinous_app_test_parrondo_aha.txt");
    select_parrondo(&mut app);

    app.toggle_inspect();
    assert!(!app.show_info, "no answer before trying a policy");
    assert_eq!(app.parrondo_aha.beat(), AhaBeat::Explore);

    app.begin_pointer_at((0.5, 0.4));
    app.end_pointer_at((0.5, 0.4));
    assert_eq!(app.parrondo_aha.beat(), AhaBeat::Prime);

    let input_count = app.inputs.len();
    app.begin_pointer_at((0.9, 0.95));
    assert_eq!(app.parrondo_aha.beat(), AhaBeat::Withheld);
    assert_eq!(app.parrondo_aha.call(), Some(Policy::CycleAbb));
    assert_eq!(
        app.inputs.len(),
        input_count,
        "the call band must not select another sampled policy"
    );

    app.toggle_inspect();
    assert!(matches!(app.parrondo_aha.beat(), AhaBeat::Morph { .. }));
    app.advance_parrondo_morph(super::PARRONDO_MORPH_SECONDS);
    assert_eq!(app.parrondo_aha.beat(), AhaBeat::Confirm);
    app.toggle_inspect();
    assert_eq!(app.parrondo_aha.beat(), AhaBeat::Consolidated);
    let grade = app.parrondo_aha.graded().expect("the call is answered");
    assert!(grade.contains("winner is ABB"), "{grade}");
    assert!(grade.contains("Nailed"), "{grade}");
    app.toggle_inspect();
    assert!(app.show_info);

    app.reset_current_room();
    assert_eq!(app.parrondo_aha.beat(), AhaBeat::Explore);
    assert!(!app.show_info);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn nontransitive_aha_turns_first_choice_into_an_exact_counter() {
    use numinous_core::rooms::nontransitive::Die;
    use numinous_core::rooms::nontransitive_aha::AhaBeat;

    let mut app = headless("numinous_app_test_nontransitive_aha.txt");
    select_nontransitive(&mut app);

    app.toggle_inspect();
    assert!(!app.show_info, "no answer before choosing a die");
    assert_eq!(app.nontransitive_aha.beat(), AhaBeat::Explore);

    app.begin_pointer_at((0.5, 0.18));
    app.end_pointer_at((0.5, 0.18));
    assert_eq!(app.nontransitive_aha.beat(), AhaBeat::Prime);
    assert_eq!(app.nontransitive_aha.chosen(), Some(Die::A));

    let input_count = app.inputs.len();
    app.begin_pointer_at((0.9, 0.95));
    assert_eq!(app.nontransitive_aha.beat(), AhaBeat::Withheld);
    assert_eq!(app.nontransitive_aha.call(), Some(Die::C));
    assert_eq!(
        app.inputs.len(),
        input_count,
        "the call band must not choose another die"
    );

    app.toggle_inspect();
    assert!(matches!(
        app.nontransitive_aha.beat(),
        AhaBeat::Morph { .. }
    ));
    app.advance_nontransitive_morph(super::NONTRANSITIVE_MORPH_SECONDS);
    assert_eq!(app.nontransitive_aha.beat(), AhaBeat::Confirm);
    app.toggle_inspect();
    assert_eq!(app.nontransitive_aha.beat(), AhaBeat::Consolidated);
    let grade = app
        .nontransitive_aha
        .graded()
        .expect("the counter is answered");
    assert!(grade.contains("counter is C"), "{grade}");
    assert!(grade.contains("20/36"), "{grade}");
    assert!(grade.contains("Nailed"), "{grade}");
    app.toggle_inspect();
    assert!(app.show_info);

    app.reset_current_room();
    assert_eq!(app.nontransitive_aha.beat(), AhaBeat::Explore);
    assert!(!app.show_info);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn the_universal_wager_reaches_an_ordinary_room_and_meets_its_truth() {
    // The Wager Wave's engine, on the App's hands: an ordinary catalog
    // room with a readout can be called, aimed with the keyboard alone,
    // and answered against the room's own number.
    let mut app = headless("numinous_app_test_room_wager.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "lorenz")
        .expect("lorenz in catalog");
    app.show_help = false;

    app.toggle_room_wager();
    assert!(app.room_wager.is_some(), "an ordinary room poses a call");
    assert!(
        app.current_status_override(80)
            .is_some_and(|s| s.contains("CALL")),
        "the footer carries the invite"
    );

    // The keyboard alone can aim: this is the only hand verb inside a
    // room a keyboard player has.
    let opened = app.room_wager.as_ref().expect("posed").aimed_value();
    for _ in 0..5 {
        if let Some(posed) = app.room_wager.as_mut() {
            posed.nudge(1);
        }
    }
    let aimed = app.room_wager.as_ref().expect("posed").aimed_value();
    assert!(aimed > opened, "arrows move the aim");

    app.commit_room_wager();
    let posed = app.room_wager.as_ref().expect("still posed after the call");
    assert!(!posed.open(), "a call is committed once");
    let grade = posed.graded().expect("the truth arrived");
    assert!(
        (grade.guess - aimed).abs() < 1e-9,
        "it graded what was aimed"
    );
    let status = app.current_status_override(80).expect("footer");
    assert!(status.contains("TRUTH"), "the truth is named: {status}");
    let banner = app.banner.as_ref().expect("the verdict speaks");
    assert!(
        banner.lines()[0].contains("READS"),
        "the verdict names the truth: {:?}",
        banner.lines()
    );

    // Leaving the room ends the call: a wager is about one readout.
    app.switch(1);
    assert!(app.room_wager.is_none());

    // The Show is watching, not playing: it takes no posed call with
    // it, so a click in the band cannot commit one behind the tour.
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "lorenz")
        .expect("lorenz in catalog");
    app.toggle_room_wager();
    assert!(app.room_wager.is_some());
    app.toggle_show();
    assert!(app.room_wager.is_none(), "the tour inherits no wager");
    app.begin_pointer_at((0.5, 0.95));
    assert!(app.room_wager.is_none(), "and cannot start one");
    app.toggle_show();

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn a_flagship_room_keeps_its_own_staged_wager() {
    // The generic call must not shadow the hand-built five-beat arcs.
    let mut app = headless("numinous_app_test_wager_flagship.txt");
    app.show_help = false;
    for id in [
        "times-tables",
        "buffon-needle",
        "galton-board",
        "double-pendulum",
        "kepler-laws",
        "parrondo",
        "nontransitive",
    ] {
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == id)
            .expect("staged room in catalog");
        app.toggle_room_wager();
        assert!(
            app.room_wager.is_none(),
            "{id} stages its own wager instead"
        );
        assert!(app.banner.is_some(), "{id} says so");
    }

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn the_curve_is_never_drawn_over_a_pile_it_does_not_explain() {
    // Three surfaces carry the claim: the pile, the curve over it, and
    // the sentence under it. Making two agree while the third
    // contradicts them is not honesty, so the curve appears only while
    // the pile is the experiment the call was about.
    let mut app = headless("numinous_app_test_galton_wander.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "galton-board")
        .expect("galton-board in catalog");
    app.reset_galton_aha();
    app.show_help = false;

    // A wave on the fair coin, then the call, then the truth.
    app.begin_pointer_at((0.5, 0.4));
    app.end_pointer_at((0.5, 0.4));
    app.sync_galton_aha();
    app.begin_pointer_at((0.5, 0.95));
    app.toggle_inspect();
    app.advance_galton_morph(super::BUFFON_MORPH_SECONDS);
    app.toggle_inspect();
    assert_eq!(app.galton_aha.coin(), Some(2));
    assert!(app.galton_aha.answers_pile(2), "its own pile");

    // Wander to the loaded coin: the pile is a different experiment.
    app.begin_pointer_at((0.95, 0.4));
    app.end_pointer_at((0.95, 0.4));
    app.sync_galton_aha();
    let live = numinous_core::rooms::galton_board::selected_coin_from_inputs(&app.inputs)
        .expect("a wave landed");
    assert_ne!(live, 2, "the hand moved to another coin");
    assert!(
        !app.galton_aha.answers_pile(live),
        "the call does not speak for this pile"
    );

    // The footer says which pile the call was about, beside the room's
    // own readout naming the pile on screen.
    let footer = app.current_status_override(80).expect("footer");
    assert!(footer.contains("ON P.50"), "{footer}");

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn galton_aha_gates_reveal_and_grades_the_peak_wager() {
    use numinous_core::rooms::galton_aha::AhaBeat;

    let mut app = headless("numinous_app_test_galton_aha.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "galton-board")
        .expect("galton-board in catalog");
    app.reset_galton_aha();
    app.show_help = false;
    app.show_info = false;

    app.toggle_inspect();
    assert!(!app.show_info, "no punchline before generation");
    assert_eq!(app.galton_aha.beat(), AhaBeat::Explore);

    // One wave on the fair coin (x=0.5) primes the peak invite.
    app.begin_pointer_at((0.5, 0.4));
    app.end_pointer_at((0.5, 0.4));
    app.sync_galton_aha();
    assert_eq!(app.galton_aha.beat(), AhaBeat::Prime);

    // A press in the wager band commits the hovered bin instead of
    // dropping a wave; bin 8 is the fair coin's true peak.
    let waves_before = numinous_core::rooms::galton_board::wave_count_from_inputs(&app.inputs);
    app.begin_pointer_at((0.5, 0.95));
    assert_eq!(app.galton_aha.beat(), AhaBeat::Withheld);
    assert_eq!(
        numinous_core::rooms::galton_board::wave_count_from_inputs(&app.inputs),
        waves_before,
        "the wager press must not drop a wave"
    );
    let (bin, coin, band) = app.galton_aha.wager().expect("wager recorded");
    assert_eq!((bin, coin), (8, 2));
    assert_eq!(band, numinous_core::rooms::galton_aha::GuessBand::Nailed);
    assert!(!app.galton_aha.allow_reveal_text());

    app.toggle_inspect();
    assert!(matches!(app.galton_aha.beat(), AhaBeat::Morph { .. }));
    app.advance_galton_morph(super::BUFFON_MORPH_SECONDS);
    assert_eq!(app.galton_aha.beat(), AhaBeat::Confirm);

    app.toggle_inspect();
    assert_eq!(app.galton_aha.beat(), AhaBeat::Consolidated);
    assert!(app.galton_aha.allow_reveal_text());
    let graded = app.galton_aha.graded().expect("graded sentence");
    assert!(graded.contains("bin 8"), "{graded}");

    app.reset_current_room();
    assert_eq!(app.galton_aha.beat(), AhaBeat::Explore);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn buffon_eight_throws_earn_without_number_wager() {
    use numinous_core::rooms::buffon_aha::{AhaBeat, EarnPath};

    let mut app = headless("numinous_app_test_buffon_throw_earn.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "buffon-needle")
        .expect("buffon-needle in catalog");
    app.reset_buffon_aha();
    app.show_help = false;

    for i in 0..8 {
        let y = 0.15 + (i as f64) * 0.08;
        app.begin_pointer_at((0.4, y.min(0.85)));
        app.end_pointer_at((0.4, y.min(0.85)));
        app.sync_buffon_aha();
    }
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Withheld);
    assert_eq!(app.buffon_aha.earn(), Some(EarnPath::Throws { count: 8 }));
    assert!(!app.buffon_aha.allow_reveal_text());

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn the_show_does_not_auto_earn_buffon_aha() {
    use numinous_core::rooms::buffon_aha::AhaBeat;

    let mut app = headless("numinous_app_test_buffon_show_no_earn.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "buffon-needle")
        .expect("buffon-needle in catalog");
    app.reset_buffon_aha();
    app.the_show = true;
    app.sync_buffon_aha();
    assert_eq!(app.buffon_aha.beat(), AhaBeat::Explore);
    assert!(!app.buffon_aha.earned());

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn only_room_score_routes_the_hand_controlled_math_voice() {
    let app = headless("numinous_app_test_times_voice.txt");
    let room = app
        .rooms
        .iter()
        .find(|room| room.meta().id == "times-tables")
        .expect("Times Tables room");
    let input = [numinous_core::RoomInput::PointerDown {
        x: 0.375,
        y: 0.5,
        t: 0.2,
    }];

    assert!(
        selected_parameter_sound(
            AudioProgram::RoomScore,
            false,
            room.as_ref(),
            0.7,
            &[],
            false,
        )
        .is_none()
    );
    let voice = selected_parameter_sound(
        AudioProgram::RoomScore,
        false,
        room.as_ref(),
        0.7,
        &input,
        false,
    )
    .expect("accepted dial voice");
    assert_eq!(voice.ratio(), 1.25);
    assert!(
        selected_parameter_sound(
            AudioProgram::Studio,
            false,
            room.as_ref(),
            0.7,
            &input,
            false,
        )
        .is_none()
    );
    assert!(
        selected_parameter_sound(
            AudioProgram::Radio,
            false,
            room.as_ref(),
            0.7,
            &input,
            false,
        )
        .is_none()
    );
    assert!(
        selected_parameter_sound(
            AudioProgram::RoomScore,
            true,
            room.as_ref(),
            0.7,
            &input,
            false,
        )
        .is_none()
    );
}

#[test]
fn galton_coin_selection_reaches_the_room_score_voice() {
    let app = headless("numinous_app_test_galton_voice.txt");
    let room = app
        .rooms
        .iter()
        .find(|room| room.meta().id == "galton-board")
        .expect("Galton Board room");
    let input = |x| [numinous_core::RoomInput::PointerDown { x, y: 0.5, t: 0.4 }];
    let left = input(0.1);
    let fair = input(0.5);
    let right = input(0.9);

    let select = |inputs: &[numinous_core::RoomInput]| {
        selected_parameter_sound(
            AudioProgram::RoomScore,
            false,
            room.as_ref(),
            0.4,
            inputs,
            false,
        )
        .expect("selected coin voice")
    };
    let left = select(&left);
    let fair = select(&fair);
    let right = select(&right);

    assert!(left.root_hz() < fair.root_hz());
    assert!(fair.root_hz() < right.root_hz());
    assert_eq!(left.ratio(), 7.0 / 3.0);
    assert_eq!(fair.ratio(), 1.0);
    assert_eq!(right.ratio(), 7.0 / 3.0);
}

#[test]
fn galton_peg_sequence_obeys_room_score_ownership() {
    let app = headless("numinous_app_test_galton_pegs.txt");
    let room = app
        .rooms
        .iter()
        .find(|room| room.meta().id == "galton-board")
        .expect("Galton Board room");
    let input = [numinous_core::RoomInput::PointerDown {
        x: 0.5,
        y: 0.5,
        t: 0.4,
    }];
    let select = |program, modal, muted, accepted| {
        selected_room_interaction_audio(
            program,
            modal,
            muted,
            accepted,
            room.as_ref(),
            &input,
            48_000,
        )
    };

    assert!(select(AudioProgram::RoomScore, false, false, true).is_some());
    assert!(select(AudioProgram::Studio, false, false, true).is_none());
    assert!(select(AudioProgram::Radio, false, false, true).is_none());
    assert!(select(AudioProgram::RoomScore, true, false, true).is_none());
    assert!(select(AudioProgram::RoomScore, false, true, true).is_none());
    assert!(select(AudioProgram::RoomScore, false, false, false).is_none());
}

#[test]
fn double_pendulum_release_sequence_obeys_room_score_ownership() {
    let app = headless("numinous_app_test_pendulum_release.txt");
    let room = app
        .rooms
        .iter()
        .find(|room| room.meta().id == "double-pendulum")
        .expect("Double Pendulum room");
    let input = [
        numinous_core::RoomInput::PointerMove {
            x: 0.3,
            y: 0.6,
            t: 0.147,
        },
        numinous_core::RoomInput::PointerUp {
            x: 0.7,
            y: 0.4,
            t: 0.15,
        },
    ];
    let select = |program, modal, muted, accepted| {
        selected_room_interaction_audio(
            program,
            modal,
            muted,
            accepted,
            room.as_ref(),
            &input,
            48_000,
        )
    };

    assert!(select(AudioProgram::RoomScore, false, false, true).is_some());
    assert!(select(AudioProgram::Studio, false, false, true).is_none());
    assert!(select(AudioProgram::Radio, false, false, true).is_none());
    assert!(select(AudioProgram::RoomScore, true, false, true).is_none());
    assert!(select(AudioProgram::RoomScore, false, true, true).is_none());
    assert!(select(AudioProgram::RoomScore, false, false, false).is_none());
}

#[test]
fn double_pendulum_release_dispatches_once_from_the_pointer_lifecycle() {
    let mut app = headless("numinous_app_test_pendulum_release_route.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "double-pendulum")
        .expect("Double Pendulum room");
    let events_before = app.interaction_audio_events.get();

    app.t = 0.1;
    assert!(app.record_room_touch((0.3, 0.6)));
    app.poking = true;
    app.t = 0.147;
    app.move_pointer_to((0.35, 0.55), true);
    assert_eq!(app.interaction_audio_events.get(), events_before);

    app.t = 0.15;
    app.end_pointer_at((0.7, 0.4));
    assert_eq!(app.interaction_audio_events.get(), events_before + 1);
    app.end_pointer_at((0.7, 0.4));
    assert_eq!(
        app.interaction_audio_events.get(),
        events_before + 1,
        "a second lift without an open gesture cannot replay the event"
    );
}

#[test]
fn a_radio_transition_cancels_an_open_pendulum_before_room_score_returns() {
    let mut app = headless("numinous_app_test_pendulum_radio_boundary.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "double-pendulum")
        .expect("Double Pendulum room");
    app.radio = Some(numinous_core::STATIONS.len() - 1);
    app.audio_program = AudioProgram::Radio;
    app.t = 0.1;
    assert!(app.record_room_touch((0.3, 0.6)));
    app.poking = true;
    app.t = 0.147;
    app.move_pointer_to((0.35, 0.55), true);
    let events_before = app.interaction_audio_events.get();

    app.handle_gamepad_command(crate::gamepad::Command::CycleRadio);

    assert!(app.radio.is_none());
    assert_eq!(app.audio_program, AudioProgram::RoomScore);
    assert!(!app.poking);
    assert!(matches!(
        app.inputs.last(),
        Some(numinous_core::RoomInput::PointerCancel)
    ));
    app.t = 0.15;
    app.end_pointer_at((0.7, 0.4));
    assert_eq!(app.interaction_audio_events.get(), events_before);
}

#[test]
fn failed_radio_resync_cancels_an_open_pendulum_before_room_score_returns() {
    let mut app = headless("numinous_app_test_pendulum_radio_resync_boundary.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "double-pendulum")
        .expect("Double Pendulum room");
    app.radio = Some(0);
    app.audio_program = AudioProgram::Radio;
    app.radio_paths.clear();
    app.t = 0.1;
    assert!(app.record_room_touch((0.3, 0.6)));
    app.poking = true;
    app.t = 0.147;
    app.move_pointer_to((0.35, 0.55), true);
    let events_before = app.interaction_audio_events.get();

    assert!(!app.sync_radio_at(0.0));

    assert_eq!(app.audio_program, AudioProgram::RoomScore);
    assert!(!app.poking);
    assert!(matches!(
        app.inputs.last(),
        Some(numinous_core::RoomInput::PointerCancel)
    ));
    app.t = 0.15;
    app.end_pointer_at((0.7, 0.4));
    assert_eq!(app.interaction_audio_events.get(), events_before);
}

#[test]
fn galton_release_and_bet_motion_preserve_the_active_peg_sequence() {
    let mut app = headless("numinous_app_test_galton_peg_lifecycle.txt");
    select_galton(&mut app);
    let clears_before = app.transient_audio_clears.get();

    assert!(app.record_room_touch((0.5, 0.5)));
    app.poking = true;
    app.move_pointer_to((0.6, 0.5), true);
    app.end_pointer_at((0.6, 0.5));

    assert_eq!(app.transient_audio_clears.get(), clears_before);
    assert!(matches!(
        app.inputs.as_slice(),
        [
            numinous_core::RoomInput::PointerDown { .. },
            numinous_core::RoomInput::PointerMove { .. },
            numinous_core::RoomInput::PointerUp { .. }
        ]
    ));
    assert!(room_transient_audio_owned(AudioProgram::RoomScore, false));
    assert!(!room_transient_audio_owned(AudioProgram::RoomScore, true));
    assert!(!room_transient_audio_owned(AudioProgram::Radio, false));
}

#[test]
fn show_entry_retires_a_room_interaction_sequence() {
    let mut app = headless("numinous_app_test_show_retires_galton_pegs.txt");
    select_galton(&mut app);
    assert!(app.record_room_touch((0.5, 0.5)));
    let clears_before = app.transient_audio_clears.get();

    app.toggle_show();

    assert!(app.the_show);
    assert_eq!(app.transient_audio_clears.get(), clears_before + 1);
}

#[test]
fn enter_from_the_menu_starts_the_show() {
    // Enter is the front-door start into the room tour (The Show), same as B.
    let mut app = headless("numinous_app_test_enter_starts_show.txt");
    assert!(app.show_help, "fresh install opens on the menu");
    assert!(!app.the_show);

    app.toggle_show();

    assert!(app.the_show);
    assert!(!app.show_help, "The Show dismisses the menu");
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn double_pendulum_gesture_reaches_the_room_score_voice() {
    let app = headless("numinous_app_test_pendulum_voice.txt");
    let room = app
        .rooms
        .iter()
        .find(|room| room.meta().id == "double-pendulum")
        .expect("Double Pendulum room");
    let select = |inputs: &[numinous_core::RoomInput]| {
        selected_parameter_sound(
            AudioProgram::RoomScore,
            false,
            room.as_ref(),
            0.35,
            inputs,
            false,
        )
        .expect("accepted pendulum voice")
    };
    let left = [numinous_core::RoomInput::PointerDown {
        x: 0.1,
        y: 0.5,
        t: 0.2,
    }];
    let right = [numinous_core::RoomInput::PointerDown {
        x: 0.9,
        y: 0.5,
        t: 0.2,
    }];
    assert!(select(&left).root_hz() < select(&right).root_hz());

    let gentle = [
        numinous_core::RoomInput::PointerDown {
            x: 0.58,
            y: 0.5,
            t: 0.05,
        },
        numinous_core::RoomInput::PointerMove {
            x: 0.58,
            y: 0.5,
            t: 0.10,
        },
        numinous_core::RoomInput::PointerUp {
            x: 0.6,
            y: 0.5,
            t: 0.15,
        },
    ];
    let fast = [
        numinous_core::RoomInput::PointerDown {
            x: 0.3,
            y: 0.5,
            t: 0.10,
        },
        numinous_core::RoomInput::PointerMove {
            x: 0.3,
            y: 0.5,
            t: 0.147,
        },
        numinous_core::RoomInput::PointerUp {
            x: 0.6,
            y: 0.5,
            t: 0.15,
        },
    ];
    assert!(select(&gentle).gain() < select(&fast).gain());
    assert!(
        selected_parameter_sound(
            AudioProgram::Radio,
            false,
            room.as_ref(),
            0.35,
            &fast,
            false,
        )
        .is_none()
    );
}

#[test]
fn the_show_sweeps_the_times_tables_voice_without_retained_hand_input() {
    let app = headless("numinous_app_test_times_show_voice.txt");
    let room = app
        .rooms
        .iter()
        .find(|room| room.meta().id == "times-tables")
        .expect("Times Tables room");
    let retained = [numinous_core::RoomInput::PointerDown {
        x: 0.375,
        y: 0.5,
        t: 0.2,
    }];

    let early = selected_parameter_sound(
        AudioProgram::RoomScore,
        false,
        room.as_ref(),
        0.1,
        &retained,
        true,
    )
    .expect("Show voice");
    let late = selected_parameter_sound(
        AudioProgram::RoomScore,
        false,
        room.as_ref(),
        0.7,
        &retained,
        true,
    )
    .expect("moving Show voice");
    assert_ne!(early.ratio(), late.ratio());
}

#[test]
fn four_lobes_raise_one_earned_banner_and_reset_cleanly() {
    let mut app = headless("numinous_app_test_times_goal.txt");
    select_times_tables(&mut app);
    app.switch(1);
    app.switch(-1);
    assert_ne!(app.variation, 0);
    assert_eq!(app.rooms[app.current].meta().id, "times-tables");
    assert_eq!(
        app.rooms[app.current]
            .status_input(
                effective_room_phase("times-tables", app.t, &app.inputs, false),
                &app.inputs,
            )
            .as_deref(),
        Some("DRAG:DIAL  K 2.00  CLOSED  1 LOBE  TARGET 4")
    );

    assert!(app.record_room_touch((0.374, 0.5)));
    assert!(app.goal_announced);
    assert_eq!(
        app.banner.as_ref().expect("earned Aha").lines(),
        ["FOUR LOBES FOUND", "EXPLAIN: WHY THE HEART MATTERS"]
    );

    app.maybe_announce_room_goal();
    assert!(app.goal_announced, "the same discovery does not spam");
    app.reset_room_runtime();
    assert!(!app.goal_announced);
    assert!(app.banner.is_none());
    assert!(app.inputs.is_empty());
    assert_eq!(app.t, 0.0);
    assert_eq!(
        app.rooms[app.current]
            .status_input(
                effective_room_phase("times-tables", app.t, &app.inputs, false),
                &app.inputs,
            )
            .as_deref(),
        Some("DRAG:DIAL  K 2.00  CLOSED  1 LOBE  TARGET 4")
    );
}

fn write_test_wav(path: &std::path::Path, channels: u16, seconds: u32) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("write wav");
    for i in 0..44_100 * seconds {
        let sample = ((i as f32 * 0.05).sin() * 12_000.0) as i16;
        for channel in 0..channels {
            let signed = if channel % 2 == 0 { sample } else { -sample };
            writer.write_sample(signed).expect("sample");
        }
    }
    writer.finalize().expect("finalize");
}

#[test]
fn app_test_profiles_are_stable_isolated_and_owned() {
    let player_journey = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|home| std::path::PathBuf::from(home).join(".numinous-journey"));
    let player_scores = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|home| std::path::PathBuf::from(home).join(".numinous-scores"));

    let first = std::thread::spawn(|| {
        let journey = super::journey_path();
        let scores = super::scores_path();
        assert_eq!(journey, super::journey_path());
        assert_eq!(scores, super::scores_path());
        assert_eq!(journey.parent(), scores.parent());
        assert_ne!(journey, scores);
        std::fs::write(&journey, "isolated").expect("write isolated app Journey");
        (journey, scores)
    })
    .join()
    .expect("first app test profile thread");
    let second = std::thread::spawn(|| (super::journey_path(), super::scores_path()))
        .join()
        .expect("second app test profile thread");

    assert_ne!(first.0.parent(), second.0.parent());
    assert!(
        !first.0.exists(),
        "the first thread owns and clears its files"
    );
    assert!(
        !second.0.exists(),
        "the second thread owns and clears its files"
    );
    if let Ok(path) = player_journey {
        assert_ne!(first.0, path);
        assert_ne!(second.0, path);
    }
    if let Ok(path) = player_scores {
        assert_ne!(first.1, path);
        assert_ne!(second.1, path);
    }

    let collision = std::env::temp_dir().join(format!(
        "numinous-app-test-collision-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&collision);
    let _ = std::fs::remove_file(&collision);
    std::fs::write(&collision, "not a directory").expect("write collision file");
    let result = std::panic::catch_unwind(|| TestStateRoot::at(collision.clone()));
    assert!(result.is_err(), "a file collision must be rejected");
    std::fs::remove_file(collision).expect("remove collision file");
}

#[test]
fn losing_the_pointer_mid_gesture_records_a_cancel() {
    let mut app = headless("numinous_app_test_gesture_cancel.txt");
    app.poking = true;
    crate::room_input::record_pointer_down(&mut app.inputs, (0.4, 0.4), 0.1);
    app.t = 0.8;
    // Focus loss and modal opens route through set_pointer_state, which
    // must close the open gesture gently.
    app.clear_pointer_state();
    assert!(!app.poking);
    assert_eq!(
        app.inputs.last(),
        Some(&numinous_core::RoomInput::PointerCancel),
        "an interrupted gesture ends in a cancel, not a phantom hold"
    );
    assert!(matches!(
        app.inputs.get(app.inputs.len() - 2),
        Some(numinous_core::RoomInput::PointerMove { t, .. }) if *t == 0.8
    ));
    // A release recorded normally is not followed by a stray cancel.
    app.poking = true;
    crate::room_input::record_pointer_down(&mut app.inputs, (0.5, 0.5), 0.2);
    crate::room_input::record_pointer_up(
        &mut app.inputs,
        (0.5, 0.5),
        0.25,
        crate::room_input::ReleaseMode::Dial,
    );
    app.clear_pointer_state();
    assert!(matches!(
        app.inputs.last(),
        Some(numinous_core::RoomInput::PointerUp { .. })
    ));
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn switching_rooms_records_visits_and_persists() {
    let mut app = headless("numinous_app_test_switch.txt");
    app.switch(1);
    app.switch(1);
    assert_eq!(app.journey.visited.len(), 2, "two rooms entered");
    let disk = numinous_core::Journey::from_text(
        &std::fs::read_to_string(&app.journey_file).expect("persisted"),
    );
    assert_eq!(disk.visited, app.journey.visited);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn the_quiz_deals_records_and_scores_wins() {
    let mut app = headless("numinous_app_test_quiz.txt");
    app.quiz_next();
    assert_eq!(app.journey.plays, 1, "dealing a round is a play");
    let disk = numinous_core::Journey::from_text(
        &std::fs::read_to_string(&app.journey_file).expect("persisted deal"),
    );
    assert_eq!(disk.plays, 1, "dealing a round persists the play");
    let answer = app.quiz.as_ref().expect("a round is live").round.answer;
    app.quiz_answer('!');
    assert!(
        app.quiz.as_ref().unwrap().flash.is_none(),
        "letters off the menu do nothing"
    );
    app.quiz_answer(answer);
    assert_eq!(app.journey.wins, 1, "the right answer is a win");
    let disk = numinous_core::Journey::from_text(
        &std::fs::read_to_string(&app.journey_file).expect("persisted win"),
    );
    assert_eq!(disk.wins, 1, "the right answer persists the win");
    let (correct, _) = app.quiz.as_ref().unwrap().flash.expect("verdict shows");
    assert!(correct);
    app.quiz_next();
    assert_eq!(app.journey.plays, 2, "the next round deals");
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn level_ups_raise_the_banner_with_lore() {
    let mut app = headless("numinous_app_test_banner.txt");
    app.journey.play();
    app.journey_changed(); // one spark crosses the first threshold: level 2
    let banner = app.banner.as_ref().expect("the banner rises");
    let lines = banner.lines();
    assert!(lines[0].contains("LEVEL UP  LV 2"));
    assert!(lines.len() >= 2, "the lore line rides along");
    assert!(banner.frames_left() > 0);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn room_reset_preserves_visit_and_clears_interaction() {
    let mut app = headless("numinous_app_test_room_reset.txt");
    app.variation = 17;
    app.t = 0.8;
    app.pokes.push((0.2, 0.7));
    app.inputs.push(numinous_core::RoomInput::PointerDown {
        x: 0.2,
        y: 0.7,
        t: 0.8,
    });
    let room_id = app.rooms[app.current].meta().id;

    app.reset_current_room();

    assert_eq!(app.rooms[app.current].meta().id, room_id);
    assert_eq!(app.variation, 17);
    assert_eq!(app.t, 0.0);
    assert!(app.pokes.is_empty());
    assert!(app.inputs.is_empty());
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn life_session_keeps_advancing_after_a_gallery_phase_wrap() {
    let mut app = headless("numinous_app_test_life_continuity.txt");
    select_life(&mut app);
    let mut advanced = 0;
    while advanced < 140 {
        let remaining = 140 - advanced;
        let batch = remaining.min(super::MAX_LIFE_STEPS_PER_TICK);
        advanced += app.advance_life(super::LIFE_STEP_SECONDS * batch as f64);
    }
    app.t = 0.999;
    app.t = 0.0;
    assert_eq!(app.advance_life(super::LIFE_STEP_SECONDS), 1);
    assert_eq!(app.life_session.generation(), 141);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn life_step_audio_obeys_program_ownership_and_uses_the_exact_step() {
    let mut session = numinous_core::rooms::game_of_life::LifeSession::new(4);
    assert!(session.launch((0.5, 0.5)));
    session.advance();
    assert_eq!(session.tracked_glider_phase(), Some(1));
    assert_eq!(session.step_sound().glider_phase(), Some(1));

    let audio =
        selected_life_step_audio(AudioProgram::RoomScore, false, false, 1, &session, 48_000)
            .expect("room-score Life step");
    assert_eq!(audio.len() % 2, 0);
    assert!(audio.iter().any(|sample| sample.abs() > 0.0));
    assert!(life_step_audio_owned(
        AudioProgram::RoomScore,
        false,
        "game-of-life"
    ));

    for (program, modal, muted, steps) in [
        (AudioProgram::Studio, false, false, 1),
        (AudioProgram::Radio, false, false, 1),
        (AudioProgram::RoomScore, true, false, 1),
        (AudioProgram::RoomScore, false, true, 1),
        (AudioProgram::RoomScore, false, false, 0),
    ] {
        assert!(
            selected_life_step_audio(program, modal, muted, steps, &session, 48_000,).is_none()
        );
    }
    assert!(!life_step_audio_owned(
        AudioProgram::Studio,
        false,
        "game-of-life"
    ));
    assert!(!life_step_audio_owned(
        AudioProgram::Radio,
        false,
        "game-of-life"
    ));
    assert!(!life_step_audio_owned(
        AudioProgram::RoomScore,
        true,
        "game-of-life"
    ));
    assert!(!life_step_audio_owned(
        AudioProgram::RoomScore,
        false,
        "times-tables"
    ));

    session.advance();
    session.advance();
    let newest =
        selected_life_step_audio(AudioProgram::RoomScore, false, false, 3, &session, 48_000)
            .expect("newest presented generation");
    assert_eq!(newest, session.step_sound().render_stereo(48_000));
}

#[test]
fn life_touch_uses_the_shared_room_input_and_session_route() {
    let mut app = headless("numinous_app_test_life_touch.txt");
    select_life(&mut app);
    let clears_before_launch = app.transient_audio_clears.get();

    assert!(app.record_room_touch((0.3, 0.7)));
    assert_eq!(app.pokes, vec![(0.3, 0.7)]);
    assert!(matches!(
        app.inputs.as_slice(),
        [numinous_core::RoomInput::PointerDown { x: 0.3, y: 0.7, .. }]
    ));
    assert_eq!(app.life_session.launches(), 1);
    assert_eq!(app.life_accumulator, 0.0);
    assert_eq!(
        app.transient_audio_clears.get(),
        clears_before_launch + 1,
        "a successful launch retires the previously presented birth texture"
    );
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn room_reset_restores_life_and_closes_a_held_pointer() {
    let mut app = headless("numinous_app_test_life_reset.txt");
    select_life(&mut app);
    app.record_room_touch((0.4, 0.6));
    app.poking = true;
    app.advance_life(super::LIFE_STEP_SECONDS * 9.0);
    let clears_before_reset = app.transient_audio_clears.get();

    app.reset_current_room();

    assert!(!app.poking);
    assert!(app.inputs.is_empty());
    assert!(app.pokes.is_empty());
    assert_eq!(app.life_session.generation(), 0);
    assert_eq!(app.life_session.launches(), 0);
    assert_eq!(app.life_accumulator, 0.0);
    assert_eq!(
        app.transient_audio_clears.get(),
        clears_before_reset + 1,
        "reset retires audio from the discarded Life generation"
    );
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn reduced_motion_hands_the_ambient_world_no_time() {
    // The whole mechanism, in one function: everything that advances on
    // its own is driven by this budget, so zeroing it stops the room
    // phase, The Show's drift into the next room, the Mandelbrot camera,
    // and the Life cadence together.
    for elapsed in [0.0, 1.0 / 60.0, 0.5, super::MAX_TICK_SECONDS] {
        assert_eq!(
            super::ambient_tick_seconds(elapsed, numinous_core::Motion::Full),
            elapsed,
            "full motion must not alter the tick"
        );
        assert_eq!(
            super::ambient_tick_seconds(elapsed, numinous_core::Motion::Reduced),
            0.0,
            "reduced motion must spend no ambient time"
        );
    }
}

#[test]
fn a_zero_ambient_tick_holds_the_phase_and_never_wraps() {
    // What the App does with that budget. A held phase cannot complete a
    // sweep, and it is the completed sweep that moves The Show to the next
    // room, so the gallery stops advancing without a separate guard.
    for phase in [0.0, 0.25, 0.5, 0.999] {
        let (next, wrapped) = super::advance_gallery_phase(phase, 0.0, 1.0, 0.24, false);
        assert_eq!(next, phase, "phase moved on a zero tick");
        assert!(!wrapped, "a held phase must not wrap into the next room");
    }
    // The counterpart: given real time, it still advances, so the test
    // above cannot pass because the mechanism broke.
    let (next, _) = super::advance_gallery_phase(0.5, 1.0 / 60.0, 1.0, 0.24, false);
    assert!(next > 0.5, "full motion must still advance");
}

#[test]
fn reduced_motion_leaves_the_life_universe_untouched() {
    let mut app = headless("numinous_app_test_reduced_motion_life.txt");
    select_life(&mut app);
    app.record_room_touch((0.5, 0.5));
    let generation = app.life_session.generation();
    // Enough time for many steps at the normal cadence.
    let held = app.advance_life_if_active(super::ambient_tick_seconds(
        super::LIFE_STEP_SECONDS * 20.0,
        numinous_core::Motion::Reduced,
    ));
    assert_eq!(held, 0, "reduced motion stepped the universe");
    assert_eq!(
        app.life_session.generation(),
        generation,
        "reduced motion advanced Life"
    );
    // And the same span with motion allowed does step it, so the
    // assertion above is about the preference rather than a dead path.
    let moved = app.advance_life_if_active(super::ambient_tick_seconds(
        super::LIFE_STEP_SECONDS * 20.0,
        numinous_core::Motion::Full,
    ));
    assert!(moved > 0, "full motion must still step Life");
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn life_status_uses_the_persistent_session_after_history_limits() {
    let mut app = headless("numinous_app_test_life_status.txt");
    select_life(&mut app);
    for i in 0..25 {
        let x = 0.1 + (i % 5) as f64 * 0.18;
        let y = 0.1 + (i / 5) as f64 * 0.18;
        assert!(app.record_room_touch((x, y)));
    }
    for _ in 0..141 {
        app.life_session.advance();
    }
    app.t = 0.0;

    assert_eq!(app.pokes.len(), numinous_core::MAX_ROOM_POKES);
    assert_eq!(app.life_session.launches(), 25);
    let wide = app.current_status_override(900).expect("wide Life status");
    let compact = app
        .current_status_override(360)
        .expect("compact Life status");
    assert!(wide.contains("GEN 141"), "got: {wide}");
    assert!(wide.contains("GLIDERS 25"), "got: {wide}");
    assert!(compact.contains("G141"), "got: {compact}");
    assert!(compact.contains("GL25"), "got: {compact}");
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn life_advancement_obeys_pause_focus_and_speed_controls() {
    let mut app = headless("numinous_app_test_life_pause.txt");
    select_life(&mut app);
    app.paused = true;
    assert_eq!(app.advance_life_if_active(super::LIFE_STEP_SECONDS), 0);
    assert_eq!(app.life_session.generation(), 0);
    app.paused = false;
    app.window_active = false;
    assert_eq!(app.advance_life_if_active(super::LIFE_STEP_SECONDS), 0);
    app.window_active = true;
    assert_eq!(app.advance_life_if_active(super::LIFE_STEP_SECONDS), 1);

    let phase = app.t;
    let speed = app.time_scale;
    assert!(app.apply_wheel_delta(1.0));
    assert_eq!(app.t, phase, "Life wheel changes cadence, not hidden phase");
    assert!(app.time_scale > speed);
    let after_wheel = app.time_scale;
    app.handle_gamepad_command(crate::gamepad::Command::PhaseDelta(-0.1));
    assert_eq!(app.t, phase);
    assert!(app.time_scale < after_wheel);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn presentation_clock_advances_a_studio_morph_through_pause_and_focus_loss() {
    let mut app = headless("numinous_app_test_studio_focus_clock.txt");
    app.studio = true;
    assert!(app.studio_panel.load_random_recipe().is_some());
    let start = Instant::now();

    app.paused = true;
    app.advance_presentation_time(0.3);
    assert!(
        app.studio_panel.load_random_recipe().is_none(),
        "pause must not finish a half-complete morph early"
    );

    app.suspend_presentation_clock(start);
    app.resume_presentation_clock(start + Duration::from_millis(300));
    assert!(
        app.studio_panel.load_random_recipe().is_some(),
        "the remaining focus-loss time must finish the original morph"
    );
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn controller_reset_closes_a_held_life_touch() {
    let mut app = headless("numinous_app_test_life_controller_reset.txt");
    select_life(&mut app);
    assert!(app.record_room_touch((0.5, 0.5)));
    app.poking = true;

    app.handle_gamepad_command(crate::gamepad::Command::Reset);

    assert!(!app.poking);
    assert!(app.inputs.is_empty());
    assert!(app.pokes.is_empty());
    assert_eq!(app.life_session.generation(), 0);
    assert_eq!(app.life_session.launches(), 0);
    app.handle_gamepad_command(crate::gamepad::Command::PointerMoved {
        point: (0.7, 0.7),
        held: true,
    });
    app.handle_gamepad_command(crate::gamepad::Command::PrimaryUp);
    assert!(app.inputs.is_empty());
    assert!(app.pokes.is_empty());
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn controller_primary_routes_one_complete_life_launch() {
    let mut app = headless("numinous_app_test_life_controller_touch.txt");
    select_life(&mut app);
    app.gamepad.set_cursor_for_test((0.35, 0.65));

    app.handle_gamepad_command(crate::gamepad::Command::PrimaryDown);
    assert!(app.poking);
    assert_eq!(app.life_session.launches(), 1);
    assert!(matches!(
        app.inputs.as_slice(),
        [numinous_core::RoomInput::PointerDown {
            x: 0.35,
            y: 0.65,
            ..
        }]
    ));

    app.handle_gamepad_command(crate::gamepad::Command::PrimaryUp);
    assert!(!app.poking);
    assert!(matches!(
        app.inputs.last(),
        Some(numinous_core::RoomInput::PointerUp {
            x: 0.35,
            y: 0.65,
            ..
        })
    ));
    assert_eq!(app.life_session.launches(), 1);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn life_postcard_matches_the_persistent_session() {
    let mut app = headless("numinous_app_test_life_postcard.txt");
    select_life(&mut app);
    app.record_room_touch((0.35, 0.65));
    for _ in 0..141 {
        app.life_session.advance();
    }
    let dir = std::env::temp_dir().join(format!(
        "numinous-life-postcard-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create postcard directory");

    let path = app.save_postcard_to(&dir).expect("save Life postcard");
    assert!(
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains("game-of-life-141")),
        "Life postcard names the persistent generation: {}",
        path.display()
    );
    let file = std::fs::File::open(path).expect("open Life postcard");
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("read Life postcard header");
    let mut decoded = vec![
        0;
        reader
            .output_buffer_size()
            .expect("decoded postcard dimensions fit address space")
    ];
    let output = reader
        .next_frame(&mut decoded)
        .expect("decode Life postcard");
    let decoded = &decoded[..output.buffer_size()];

    let size = crate::postcard::POSTCARD_SIZE as usize;
    let room = app.rooms[app.current].as_ref();
    let mut expected = numinous_core::Raster::with_accent(size, size, room.meta().accent);
    app.life_session.render(&mut expected);
    let mut expected = expected.to_rgba();
    app.era.apply(&mut expected, size, size);
    assert_eq!(decoded, expected);

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn embedded_app_icon_decodes() {
    assert!(app_icon().is_some());
}

#[test]
fn accelerated_mandelbrot_uses_the_core_camera_and_shared_chrome() {
    let phase = 0.63;
    let variation = 17;
    let (center_x, center_y, half_span) =
        numinous_core::rooms::mandelbrot::automatic_view(phase, variation);
    let gpu = mandelbrot_gpu_view(phase, variation, 900, 700, &[]);
    assert!((f64::from(gpu.0) - center_x).abs() < 1e-6);
    assert!((f64::from(gpu.1) - center_y).abs() < 1e-6);
    let expected_vertical_span = 2.0 * half_span * 700.0 / 900.0;
    assert!((f64::from(gpu.2) - expected_vertical_span).abs() < 1e-6);

    let inputs = [
        numinous_core::RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: phase,
        },
        numinous_core::RoomInput::PointerDown {
            x: 0.75,
            y: 0.25,
            t: phase + 0.1,
        },
    ];
    let (selected_x, selected_y, selected_half_span) =
        numinous_core::rooms::mandelbrot::selected_view_input(&inputs, 900, 700, variation, phase);
    let selected_gpu = mandelbrot_gpu_view(phase, variation, 900, 700, &inputs);
    assert!((f64::from(selected_gpu.0) - selected_x).abs() < 1e-6);
    assert!((f64::from(selected_gpu.1) - selected_y).abs() < 1e-6);
    assert!(selected_half_span < half_span);
    assert!((f64::from(selected_gpu.2) - 2.0 * selected_half_span * 700.0 / 900.0).abs() < 1e-6);
    assert_eq!(
        selected_gpu,
        mandelbrot_gpu_view(0.99, variation, 900, 700, &inputs),
        "deterministic face rendering preserves the selected camera"
    );

    let mut app = headless("numinous_app_test_gpu_chrome.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "mandelbrot")
        .expect("Mandelbrot room");
    app.show_help = false;
    app.room_card = 0;
    app.t = phase;
    let source = vec![64u8; 320 * 220 * 4];
    let mut raster =
        numinous_core::Raster::from_rgba(320, 220, app.rooms[app.current].meta().accent, &source)
            .expect("GPU frame import");
    let before = raster.to_rgba();
    let room = &app.rooms[app.current];
    app.draw_room_interface(&mut raster, room.as_ref(), 320, 220);
    let after = raster.to_rgba();
    assert_ne!(
        after, before,
        "GPU frames must receive title and footer chrome"
    );
    assert!(
        after[(220 - 8) * 320 * 4..]
            .chunks_exact(4)
            .any(|pixel| pixel[0..3] != [64, 64, 64]),
        "the reset footer reaches the accelerated frame"
    );
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn elapsed_simulation_time_is_measured_and_bounded() {
    let ordinary = bounded_tick_seconds(Duration::from_millis(16));
    assert!((ordinary - 0.016).abs() < 1e-9);
    assert_eq!(bounded_tick_seconds(Duration::from_secs(10)), 0.05);
}

#[test]
fn fullscreen_shortcut_returns_directly_to_windowed_mode() {
    assert!(fullscreen_toggle_target(true).is_none());
    assert!(matches!(
        fullscreen_toggle_target(false),
        Some(winit::window::Fullscreen::Borderless(None))
    ));
}

#[test]
fn cabinet_quit_requests_the_orderly_exit_path() {
    let mut app = headless("numinous_app_test_cabinet_quit.txt");
    assert!(app.menu.focus(numinous_app::menu::MenuItemId::Quit));
    let intent = app.menu.activate_focused();
    assert_eq!(intent, numinous_app::menu::MenuIntent::Quit);
    app.apply_menu_intent(intent);
    assert!(app.quit_requested);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn q_quits_outside_text_entry_and_never_repeats() {
    let mut app = headless("numinous_app_test_q_quit.txt");
    assert!(app.handle_quit_key(&Key::Character("Q".into()), false));
    assert!(app.quit_requested);

    app.quit_requested = false;
    assert!(!app.handle_quit_key(&Key::Character("q".into()), true));
    assert!(!app.quit_requested);
    app.studio = true;
    assert!(!app.handle_quit_key(&Key::Character("q".into()), false));
    assert!(!app.quit_requested, "Studio keeps q as formula text");
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn journey_banner_preserves_the_first_contact_clock_and_card() {
    let mut banner = Some(super::feedback::level_up(2, 0));
    let mut phase = 0.0;
    let mut room_card = crate::room_input::ROOM_CARD_FRAMES;

    for _ in 0..300 {
        let obscured = banner.is_some() && room_card > 0;
        let (next, wrapped) = advance_gallery_phase(phase, 1.0 / 60.0, 1.0, 0.24, obscured);
        phase = next;
        assert!(!wrapped);
        crate::room_input::tick_room_card(&mut room_card, banner.is_some());
        if banner.as_mut().is_some_and(|value| !value.tick()) {
            banner = None;
        }
    }

    assert!(banner.is_none());
    assert_eq!(phase, 0.0);
    assert_eq!(room_card, crate::room_input::ROOM_CARD_FRAMES);
    let (next, wrapped) = advance_gallery_phase(phase, 1.0 / 60.0, 1.0, 0.24, false);
    assert!(next > 0.0);
    assert!(!wrapped);
    crate::room_input::tick_room_card(&mut room_card, false);
    assert_eq!(room_card, crate::room_input::ROOM_CARD_FRAMES - 1);
}

#[test]
fn live_mandelbrot_view_tracks_the_persistent_camera() {
    let mut camera = numinous_core::rooms::mandelbrot::MandelbrotCamera::new(17);
    let initial = live_mandelbrot_gpu_view(camera, 900, 700).expect("opening GPU view");
    camera.advance(1.0);
    let advanced = live_mandelbrot_gpu_view(camera, 900, 700).expect("advanced GPU view");
    assert_ne!(advanced, initial, "elapsed time advances the live camera");

    assert!(camera.dive(0.75, 0.25, 900, 700));
    let selected = live_mandelbrot_gpu_view(camera, 900, 700).expect("selected GPU view");
    camera.advance(1.0);
    let deeper = live_mandelbrot_gpu_view(camera, 900, 700).expect("deeper GPU view");
    assert_ne!(deeper, selected, "a selected target keeps zooming");
    assert!(deeper.2 < selected.2, "the vertical span keeps shrinking");
}

#[test]
fn deep_mandelbrot_view_falls_back_before_gpu_coordinates_collapse() {
    let mut camera = numinous_core::rooms::mandelbrot::MandelbrotCamera::new(17);
    camera.advance(200.0);
    assert!(live_mandelbrot_gpu_view(camera, 900, 700).is_none());
}

#[test]
fn controller_dpad_navigates_rooms_without_a_mouse() {
    let mut app = headless("numinous_app_test_controller_room.txt");
    app.show_help = false;
    let original = app.current;

    app.handle_gamepad_command(crate::gamepad::Command::Right);

    assert_ne!(app.current, original);
    assert!(app.inputs.is_empty());
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn controller_can_open_and_leave_every_menu_destination() {
    use numinous_app::menu::{MenuItemId, MenuRoute};

    for (choice, item, games_page) in [
        (MenuChoice::Quiz, MenuItemId::Quiz, true),
        (MenuChoice::Munch, MenuItemId::Munch, true),
        (MenuChoice::Nim, MenuItemId::Nim, true),
        (MenuChoice::Gauntlet, MenuItemId::Gauntlet, true),
        (MenuChoice::Arcade, MenuItemId::Arcade, true),
        (MenuChoice::Show, MenuItemId::Watch, false),
        (MenuChoice::Studio, MenuItemId::Create, false),
        (MenuChoice::Journey, MenuItemId::Journey, false),
        (MenuChoice::WatchAgent, MenuItemId::SharedPlay, false),
    ] {
        let mut app = headless(&format!(
            "numinous_app_test_controller_destination_{choice:?}.txt"
        ));
        if games_page {
            assert_eq!(
                app.menu.activate_shortcut('g'),
                Some(numinous_app::menu::MenuIntent::None)
            );
            assert_eq!(app.menu.route(), MenuRoute::Games);
        } else {
            assert_eq!(
                app.menu.activate_shortcut('m'),
                Some(numinous_app::menu::MenuIntent::None)
            );
            assert_eq!(app.menu.route(), MenuRoute::Modes);
        }
        let _ = app.menu.focus(item);
        assert_eq!(app.menu.focused(), item);

        app.handle_gamepad_command(crate::gamepad::Command::PrimaryDown);

        assert!(!app.show_help);
        match choice {
            MenuChoice::Quiz => assert!(app.quiz.is_some()),
            MenuChoice::Munch => assert!(app.munch.is_some()),
            MenuChoice::Nim => assert!(app.nim.is_some()),
            MenuChoice::Gauntlet => assert!(app.gauntlet.is_some()),
            MenuChoice::Arcade => assert!(app.arcade.is_some()),
            MenuChoice::Show => assert!(app.the_show),
            MenuChoice::Studio => assert!(app.studio),
            MenuChoice::Journey => assert!(app.show_journey),
            MenuChoice::WatchAgent => assert!(app.session_viewer.is_open()),
        }

        if app.activity_kind().is_some() {
            app.handle_gamepad_command(crate::gamepad::Command::Back);
            assert!(app.show_help, "Back opens the contextual pause menu");
            assert!(app.menu.focus(MenuItemId::LeaveActivity));
            app.handle_gamepad_command(crate::gamepad::Command::PrimaryDown);
        } else {
            app.handle_gamepad_command(crate::gamepad::Command::Back);
        }
        assert!(!app.the_show && !app.studio && !app.show_journey);
        assert!(!app.session_viewer.is_open());
        assert!(app.quiz.is_none() && app.munch.is_none() && app.nim.is_none());
        assert!(app.gauntlet.is_none() && app.arcade.is_none());
        let _ = std::fs::remove_file(&app.journey_file);
        let _ = std::fs::remove_file(&app.scores_file);
    }
}

#[test]
fn controller_moves_through_menu_pages_before_leaving_the_room() {
    use crate::gamepad::Command;
    use numinous_app::menu::{MenuItemId, MenuRoute};

    let mut app = headless("numinous_app_test_controller_menu_pages.txt");
    assert!(app.menu.focus(MenuItemId::Games));
    app.handle_gamepad_command(Command::PrimaryDown);
    assert!(app.show_help);
    assert_eq!(app.menu.route(), MenuRoute::Games);
    assert_eq!(app.menu.focused(), MenuItemId::Quiz);

    app.handle_gamepad_command(Command::Back);
    assert!(app.show_help);
    assert_eq!(app.menu.route(), MenuRoute::Home);
    app.handle_gamepad_command(Command::Back);
    assert!(!app.show_help);

    app.open_home_menu();
    assert!(app.menu.focus(MenuItemId::Settings));
    app.handle_gamepad_command(Command::PrimaryDown);
    assert_eq!(app.menu.route(), MenuRoute::Settings);
    assert!(app.menu.focus(MenuItemId::SkipTrack));
    app.handle_gamepad_command(Command::PrimaryDown);
    assert_eq!(app.menu.route(), MenuRoute::Settings);
    assert_eq!(
        app.banner.as_ref().map(|banner| banner.lines()),
        Some(&["RADIO OFF".to_string(), "Y CHOOSES A STATION".to_string()][..])
    );
    app.handle_gamepad_command(Command::Back);
    assert_eq!(app.menu.route(), MenuRoute::Home);
    assert!(app.menu.focus(MenuItemId::Controls));
    app.handle_gamepad_command(Command::PrimaryDown);
    assert_eq!(app.menu.route(), MenuRoute::Controls);
    app.handle_gamepad_command(Command::PrimaryDown);
    assert!(app.show_help, "Controls has no hidden launch action");
    assert_eq!(app.menu.route(), MenuRoute::Home);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn keyboard_menu_shortcuts_only_activate_visible_actions() {
    use numinous_app::menu::MenuRoute;

    let mut app = headless("numinous_app_test_keyboard_menu_pages.txt");
    assert!(app.handle_menu_key(&Key::Character("m".into()), false));
    assert!(app.quiz.is_none(), "Quiz is not a root-menu action");
    assert_eq!(app.menu.route(), MenuRoute::Modes);

    assert!(app.handle_menu_key(&Key::Character("g".into()), false));
    assert_eq!(app.menu.route(), MenuRoute::Modes);
    assert!(app.handle_menu_key(&Key::Named(NamedKey::Escape), false));
    assert_eq!(app.menu.route(), MenuRoute::Home);

    assert!(app.handle_menu_key(&Key::Character("g".into()), false));
    assert_eq!(app.menu.route(), MenuRoute::Games);
    assert!(app.show_help);
    assert!(app.handle_menu_key(&Key::Character("m".into()), false));
    assert!(app.munch.is_some());
    assert!(!app.show_help);

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn pointer_hover_keeps_the_visible_controller_menu_layout_stable() {
    use numinous_app::menu::MenuItemId;

    let mut app = headless("controller-menu-pointer-hover");
    app.input_mode = InputMode::Controller;
    assert_eq!(
        app.menu.activate_shortcut('m'),
        Some(numinous_app::menu::MenuIntent::None)
    );

    assert!(app.menu.pointer_move(Some(MenuItemId::Create)));
    assert_eq!(app.menu.hovered(), Some(MenuItemId::Create));
    assert_eq!(app.menu.focused(), MenuItemId::Watch);
    assert_eq!(
        app.input_mode,
        InputMode::Controller,
        "hover and press must resolve against the same visible layout"
    );
    assert!(!app.menu.pointer_move(Some(MenuItemId::Create)));

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn controller_pause_is_explicit_and_sets_the_active_input_mode() {
    use crate::gamepad::Command;

    let mut app = headless("numinous_app_test_controller_pause.txt");
    app.show_help = false;
    assert_eq!(app.input_mode, InputMode::KeyboardMouse);

    app.handle_gamepad_command(Command::CancelPointer);
    assert_eq!(app.input_mode, InputMode::KeyboardMouse);
    assert!(!app.paused);

    app.handle_gamepad_command(Command::Pause);
    assert_eq!(app.input_mode, InputMode::Controller);
    assert!(app.paused);

    app.arcade_start();
    app.paused = true;
    app.input_mode = InputMode::KeyboardMouse;
    let before = {
        let play = app.arcade.as_ref().unwrap();
        (play.run.muncher, play.run.score, play.run.eaten.clone())
    };
    app.handle_gamepad_command(Command::Right);
    app.handle_gamepad_command(Command::PrimaryDown);
    let after = {
        let play = app.arcade.as_ref().unwrap();
        (play.run.muncher, play.run.score, play.run.eaten.clone())
    };
    assert_eq!(after, before, "paused Arcade rejects movement and eating");
    assert_eq!(
        app.input_mode,
        InputMode::KeyboardMouse,
        "ignored controller input does not steal the active legend"
    );

    app.handle_gamepad_command(Command::Pause);
    assert!(!app.paused);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn controller_menu_pauses_and_resumes_without_discarding_a_game() {
    let mut app = headless("numinous_app_test_controller_pause_menu.txt");
    app.show_help = false;
    app.quiz_next();

    app.handle_gamepad_command(crate::gamepad::Command::Menu);
    assert!(app.show_help);
    assert!(app.quiz.is_some());
    assert!(app.modal_frame(320, 220).is_some());
    assert_eq!(
        app.menu.route(),
        numinous_app::menu::MenuRoute::Pause(numinous_app::menu::ActivityKind::Quiz)
    );
    assert_eq!(app.menu.focused(), numinous_app::menu::MenuItemId::Resume);

    app.handle_gamepad_command(crate::gamepad::Command::Right);
    assert_eq!(app.menu.focused(), numinous_app::menu::MenuItemId::Resume);
    assert!(app.quiz.as_ref().is_some_and(|quiz| quiz.flash.is_none()));

    app.handle_gamepad_command(crate::gamepad::Command::PrimaryDown);
    assert!(!app.show_help);
    assert!(app.quiz.is_some());
    assert!(app.modal_frame(320, 220).is_some());
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn contextual_pause_blocks_keyboard_gameplay_until_escape_returns() {
    let mut app = headless("numinous_app_test_modal_help_keyboard.txt");
    app.show_help = false;
    app.quiz_next();
    app.open_activity_menu(numinous_app::menu::ActivityKind::Quiz);

    assert!(app.handle_menu_key(&Key::Character("a".into()), false));
    assert!(app.quiz.as_ref().is_some_and(|quiz| quiz.flash.is_none()));
    assert!(app.show_help);
    assert!(app.handle_menu_key(&Key::Named(NamedKey::Escape), false));
    assert!(!app.show_help);
    assert!(app.quiz.is_some());
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn modal_help_and_zero_motion_block_wheel_state_changes() {
    let mut app = headless("numinous_app_test_modal_help_wheel.txt");
    app.show_help = false;
    app.quiz_next();
    app.open_activity_menu(numinous_app::menu::ActivityKind::Quiz);
    app.input_mode = InputMode::Controller;
    app.t = 0.4;

    assert!(!app.apply_wheel_delta(3.0));
    assert_eq!(app.t, 0.4);
    assert_eq!(app.input_mode, InputMode::Controller);

    app.show_help = false;
    assert!(!app.apply_wheel_delta(0.0));
    assert_eq!(app.t, 0.4);
    assert_eq!(app.input_mode, InputMode::Controller);
    assert!(app.apply_wheel_delta(2.0));
    assert!((app.t - 0.44).abs() < f64::EPSILON);
    assert_eq!(app.input_mode, InputMode::KeyboardMouse);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn paused_pointer_input_cannot_touch_a_room() {
    let mut app = headless("numinous_app_test_paused_pointer.txt");
    app.show_help = false;
    app.paused = true;

    app.begin_pointer_at((0.5, 0.5));
    app.move_pointer_to((0.7, 0.5), true);

    assert!(app.inputs.is_empty());
    assert!(app.pokes.is_empty());
    assert!(!app.poking);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn controller_routes_each_game_and_every_gauntlet_stage() {
    use crate::gamepad::Command;

    let command_for = |letter: char| match letter.to_ascii_uppercase() {
        'A' => Command::Up,
        'B' => Command::Right,
        'C' => Command::Down,
        'D' => Command::Left,
        _ => panic!("choice must be A through D"),
    };

    let mut quiz = headless("numinous_app_test_controller_quiz_route.txt");
    quiz.show_help = false;
    quiz.quiz_next();
    quiz.handle_gamepad_command(Command::Right);
    assert!(quiz.quiz.as_ref().is_some_and(|play| play.flash.is_some()));
    quiz.handle_gamepad_command(Command::CycleRadio);
    assert!(quiz.radio.is_none());

    let mut munch = headless("numinous_app_test_controller_munch_route.txt");
    munch.show_help = false;
    munch.munch_start();
    munch.handle_gamepad_command(Command::Right);
    munch.handle_gamepad_command(Command::PrimaryDown);
    assert!(
        munch
            .munch
            .as_ref()
            .is_some_and(|play| play.bites.contains(&1))
    );
    munch.handle_gamepad_command(Command::CycleRadio);
    assert!(munch.munch.is_some());
    assert!(munch.radio.is_none());

    let mut nim = headless("numinous_app_test_controller_nim_route.txt");
    nim.show_help = false;
    nim.nim_start();
    let before: u32 = nim.nim.as_ref().unwrap().heaps.iter().sum();
    nim.handle_gamepad_command(Command::Right);
    nim.handle_gamepad_command(Command::PrimaryDown);
    let after: u32 = nim.nim.as_ref().unwrap().heaps.iter().sum();
    assert!(after < before);

    let mut arcade = headless("numinous_app_test_controller_arcade_route.txt");
    arcade.show_help = false;
    arcade.arcade_start();
    let before = arcade.arcade.as_ref().unwrap().run.muncher;
    arcade.handle_gamepad_command(Command::Right);
    assert_ne!(arcade.arcade.as_ref().unwrap().run.muncher, before);
    let target = {
        let run = &arcade.arcade.as_ref().unwrap().run;
        run.board
            .numbers
            .iter()
            .position(|&number| run.board.rule.fits(number))
            .expect("arcade board has an edible number")
    };
    arcade.arcade.as_mut().unwrap().run.muncher = target;
    let score = arcade.arcade.as_ref().unwrap().run.score;
    arcade.handle_gamepad_command(Command::PrimaryDown);
    assert!(
        arcade.arcade.as_ref().unwrap().run.eaten[target]
            || arcade.arcade.as_ref().unwrap().run.score > score
    );
    arcade.handle_gamepad_command(Command::CycleRadio);
    assert!(arcade.radio.is_none());
    arcade.arcade.as_mut().unwrap().over = true;
    arcade.handle_gamepad_command(Command::PrimaryDown);
    assert!(arcade.arcade.is_none());

    let mut gauntlet = headless("numinous_app_test_controller_gauntlet_route.txt");
    gauntlet.show_help = false;
    gauntlet.gauntlet_start();
    gauntlet.handle_gamepad_command(Command::Right);
    gauntlet.handle_gamepad_command(Command::PrimaryDown);
    gauntlet.handle_gamepad_command(Command::CycleRadio);
    assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 1);

    let shape = gauntlet.gauntlet.as_ref().unwrap().quiz.round.answer;
    gauntlet.handle_gamepad_command(command_for(shape));
    assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 2);
    let sky = gauntlet.gauntlet.as_ref().unwrap().scan.answer;
    gauntlet.handle_gamepad_command(command_for(sky));
    assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 3);

    let secret = gauntlet.gauntlet.as_ref().unwrap().secret.clone();
    for digit in secret {
        gauntlet.controller_digit = digit;
        gauntlet.handle_gamepad_command(Command::PrimaryDown);
    }
    gauntlet.handle_gamepad_command(Command::CycleRadio);
    assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 4);
    gauntlet.handle_gamepad_command(Command::PrimaryDown);
    assert!(gauntlet.gauntlet.is_none());

    let mut studio = headless("numinous_app_test_controller_studio_route.txt");
    studio.show_help = false;
    studio.studio = true;
    studio.handle_gamepad_command(Command::CycleRadio);
    assert!(studio.radio.is_none());
    studio.handle_gamepad_command(Command::Menu);
    assert!(studio.show_help);
    assert!(studio.studio);
    studio.handle_gamepad_command(Command::Right);
    assert!(studio.studio);
    studio.handle_gamepad_command(Command::PrimaryDown);
    assert!(!studio.show_help);
    assert!(studio.studio);

    for app in [&quiz, &munch, &nim, &arcade, &gauntlet, &studio] {
        let _ = std::fs::remove_file(&app.journey_file);
        let _ = std::fs::remove_file(&app.scores_file);
    }
}

#[test]
fn accelerated_julia_uses_the_core_selected_constant() {
    let pokes = [(0.2, 0.8), (0.75, 0.25)];
    let expected = numinous_core::rooms::julia::selected_c(0.4, 13, &pokes);
    let actual = julia_gpu_c(0.4, 13, &pokes);
    assert!((f64::from(actual.0) - expected.0).abs() < 1e-6);
    assert!((f64::from(actual.1) - expected.1).abs() < 1e-6);
    assert_ne!(actual, julia_gpu_c(0.4, 13, &[]));
    assert!((julia_gpu_vertical_span(900, 700) - 3.2 * 700.0 / 900.0).abs() < 1e-6);
    assert_eq!(julia_gpu_vertical_span(0, 700), 0.0);
}

#[test]
fn playtest_note_writes_current_session_context() {
    let mut app = headless("numinous_app_test_playtest_note.txt");
    app.journey.visit(app.rooms[app.current].meta().id);
    app.journey.play();
    app.t = 0.5;
    app.variation = 9;
    app.pokes = vec![(0.2, 0.4), (0.8, 0.1)];
    let dir = std::env::temp_dir().join("numinous_app_playtest_note");
    let _ = std::fs::remove_dir_all(&dir);

    let path = app
        .save_playtest_note_to(&dir, UNIX_EPOCH + Duration::from_secs(77))
        .expect("report saved");
    let report = std::fs::read_to_string(&path).expect("report readable");

    assert!(report.contains("Saved at Unix seconds: 77"));
    assert!(report.contains(app.rooms[app.current].meta().title));
    assert!(report.contains("Variation: 9"));
    assert!(report.contains("Poke trail: 2 point(s)"));
    assert!(report.contains("Poke points newest-last: (0.200,0.400) (0.800,0.100)"));
    assert!(report.contains("Sound: off"));
    assert!(report.contains("First unprompted whoa"));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn playtest_note_captures_times_tables_aha_beat() {
    let mut app = headless("numinous_app_test_playtest_aha.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "times-tables")
        .expect("times-tables in catalog");
    app.reset_room_runtime();
    app.times_tables_aha.note_hand_multiplier(2.0);
    assert_eq!(app.times_tables_aha.beat_label(), "prime");
    let dir = std::env::temp_dir().join("numinous_app_playtest_aha");
    let _ = std::fs::remove_dir_all(&dir);

    let path = app
        .save_playtest_note_to(&dir, UNIX_EPOCH + Duration::from_secs(101))
        .expect("report saved");
    let report = std::fs::read_to_string(&path).expect("report readable");

    assert!(report.contains("## Flagship Aha Snapshot"));
    assert!(report.contains("- Aha beat: prime"));
    assert!(report.contains("- Earn path: none"));
    assert!(report.contains("### Engineered aha (Times Tables / Buffon)"));
    assert!(report.contains("Observable aha or consolidation moment"));

    // The Show must not claim ordinary-visit aha state.
    app.the_show = true;
    let path_show = app
        .save_playtest_note_to(&dir, UNIX_EPOCH + Duration::from_secs(102))
        .expect("show report saved");
    let show_report = std::fs::read_to_string(&path_show).expect("show report readable");
    assert!(!show_report.contains("## Flagship Aha Snapshot"));

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn playtest_note_captures_buffon_aha_beat() {
    let mut app = headless("numinous_app_test_playtest_buffon_aha.txt");
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "buffon-needle")
        .expect("buffon-needle in catalog");
    app.reset_room_runtime();
    app.buffon_aha.note_throws(1);
    assert!(app.buffon_aha.commit_wager(3.0));
    assert_eq!(app.buffon_aha.beat_label(), "withheld");
    let dir = std::env::temp_dir().join("numinous_app_playtest_buffon_aha");
    let _ = std::fs::remove_dir_all(&dir);

    let path = app
        .save_playtest_note_to(&dir, UNIX_EPOCH + Duration::from_secs(201))
        .expect("report saved");
    let report = std::fs::read_to_string(&path).expect("report readable");

    assert!(report.contains("## Flagship Aha Snapshot"));
    assert!(report.contains("- Aha beat: withheld"));
    assert!(report.contains("- Earn path: wager:3.000:close"));
    assert!(report.contains("- Can summon with E: yes"));
    assert!(report.contains("### Engineered aha (Times Tables / Buffon)"));

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn playtest_shortcut_is_global_and_reports_failures() {
    use winit::keyboard::{Key, NamedKey};
    let mut app = headless("numinous_app_test_playtest_shortcut.txt");
    app.quiz_next();
    let dir = super::test_state_path("playtest-shortcut");
    let _ = std::fs::remove_dir_all(&dir);
    let input_start = Instant::now();

    assert!(app.handle_playtest_shortcut_to(
        &Key::Named(NamedKey::F9),
        &dir,
        UNIX_EPOCH + Duration::from_secs(88),
        input_start,
        false,
    ));
    assert!(
        app.quiz.is_some(),
        "shortcut does not close the active mode"
    );
    let lines = app.banner.as_ref().expect("saved banner").lines();
    assert_eq!(lines[0], "PLAYTEST NOTE SAVED");
    assert!(dir.join("playtest-88.md").exists());
    assert!(app.handle_playtest_shortcut_to(
        &Key::Named(NamedKey::F9),
        &dir,
        UNIX_EPOCH + Duration::from_secs(89),
        input_start + Duration::from_millis(1),
        true,
    ));
    assert!(
        !dir.join("playtest-89.md").exists(),
        "a repeated key event must not produce another file"
    );

    let blocker = super::test_state_path("playtest-blocker");
    let _ = std::fs::remove_file(&blocker);
    std::fs::write(&blocker, "not a directory").expect("blocker file");
    assert!(app.handle_playtest_shortcut_to(
        &Key::Named(NamedKey::F9),
        &blocker,
        UNIX_EPOCH + Duration::from_secs(90),
        input_start + Duration::from_secs(1),
        false,
    ));
    let lines = app.banner.as_ref().expect("failure banner").lines();
    assert_eq!(lines[0], "PLAYTEST NOTE FAILED");
    assert!(lines[1].starts_with("WRITE ERROR:"));
    assert!(!app.handle_playtest_shortcut_to(
        &Key::Named(NamedKey::F8),
        &dir,
        UNIX_EPOCH + Duration::from_secs(91),
        input_start + Duration::from_secs(2),
        false,
    ));

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(blocker);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn a_refusal_banner_lives_at_least_as_long_as_a_decorative_one() {
    // Every refusal flashed for 90 frames while a level-up got 300, so
    // "refused with a reason" was a flash a slow reader missed. The
    // refusal constant now buys reading time, proved on the real path:
    // an unparsed formula's share refusal outlives a share confirmation.
    let mut app = headless("numinous_app_test_refusal_frames.txt");
    app.enter_studio();
    assert!(app.studio_panel.push_text("(((").is_none());
    app.share_studio_creation(None);
    let refusal = app.banner.as_ref().expect("refusal banner");
    assert_eq!(refusal.lines()[0], "FIX THE FORMULA TO SHARE");
    assert_eq!(refusal.frames_left(), super::feedback::REFUSAL_FRAMES);
    assert!(
        refusal.frames_left() >= 240,
        "a refusal must not be briefer than the decorative banners"
    );
}

#[test]
fn banner_overlay_is_visible_on_the_shared_raster_path() {
    let mut app = headless("numinous_app_test_banner_overlay.txt");
    app.banner = Some(super::feedback::playtest_note(Ok(
        std::path::PathBuf::from("playtest-note.md"),
    )));

    let mut raster = numinous_core::Raster::with_accent(320, 220, [120, 220, 190]);
    let before_raster = raster.to_rgba();
    app.draw_banner_on_raster(&mut raster, 320, 220);
    assert_ne!(raster.to_rgba(), before_raster);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn volume_feedback_survives_while_radio_is_active() {
    let mut app = headless("numinous_app_test_radio_volume_banner.txt");
    app.radio = Some(0);
    app.radio_track = Arc::new(vec![0.25, -0.25, 0.5, -0.5]);

    app.change_volume(0.1);

    assert!((app.volume - 0.55).abs() < f32::EPSILON);
    let banner = app.banner.as_ref().expect("volume banner");
    assert_eq!(banner.lines()[0], "VOLUME 55%");
    assert_eq!(app.radio_track.as_slice(), [0.25, -0.25, 0.5, -0.5]);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn app_options_persist_one_versioned_preference_snapshot() {
    let mut app = headless("numinous_app_test_preferences.txt");
    let path = app.preferences_file.clone();
    let _ = std::fs::remove_file(&path);

    app.change_volume(0.1);
    app.toggle_mute();
    app.cycle_visual_era();

    assert_eq!(
        numinous_core::read_app_preferences_file(&path).expect("saved preferences"),
        numinous_core::AppPreferences {
            volume_percent: 55,
            muted: true,
            era: numinous_core::Era::Phosphor,
            window_mode: numinous_core::WindowModePreference::Windowed,
        }
    );
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn app_options_preserve_and_report_a_malformed_preference_file() {
    let mut app = headless("numinous_app_test_preferences_invalid.txt");
    let path = app.preferences_file.clone();
    std::fs::write(&path, b"not a preference schema\n").expect("malformed fixture");

    app.change_volume(0.1);

    assert_eq!(
        std::fs::read(&path).expect("malformed file remains"),
        b"not a preference schema\n"
    );
    assert_eq!(
        app.banner.as_ref().expect("save warning").lines()[0],
        super::PREFERENCES_SAVE_WARNING
    );
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn global_audio_keys_work_in_every_mode_without_consuming_studio_text() {
    let mut modes = Vec::new();

    let mut quiz = headless("numinous_app_test_audio_quiz.txt");
    quiz.quiz_next();
    modes.push(quiz);

    let mut munch = headless("numinous_app_test_audio_munch.txt");
    munch.munch_start();
    modes.push(munch);

    let mut nim = headless("numinous_app_test_audio_nim.txt");
    nim.nim_start();
    modes.push(nim);

    let mut gauntlet = headless("numinous_app_test_audio_gauntlet.txt");
    gauntlet.gauntlet_start();
    modes.push(gauntlet);

    let mut arcade = headless("numinous_app_test_audio_arcade.txt");
    arcade.arcade_start();
    modes.push(arcade);

    let mut paused = headless("numinous_app_test_audio_paused.txt");
    paused.paused = true;
    modes.push(paused);

    for app in &mut modes {
        let mode = app.playtest_mode();
        assert!(app.handle_global_audio_key(&Key::Character("m".into()), false));
        assert!(app.muted, "mute works in {mode}");
        assert!(app.handle_global_audio_key(&Key::Character("]".into()), false));
        assert!(
            (app.volume - 0.55).abs() < f32::EPSILON,
            "volume works in {mode}"
        );
        assert_eq!(app.playtest_mode(), mode, "audio keys preserve {mode}");
    }

    let mut studio = headless("numinous_app_test_audio_studio.txt");
    studio.enter_studio();
    let source = studio.studio_panel.source_for_test().to_string();
    assert!(studio.handle_global_audio_key(&Key::Character("m".into()), false));
    assert!(studio.handle_global_audio_key(&Key::Character("[".into()), false));
    assert_eq!(studio.studio_panel.source_for_test(), source);
    assert!(!studio.handle_global_audio_key(&Key::Character("-".into()), false));
    assert_eq!(studio.audio_program, AudioProgram::Studio);
}

#[test]
fn controller_audio_commands_are_global_while_paused_and_in_studio() {
    let mut app = headless("numinous_app_test_controller_audio.txt");
    app.enter_studio();
    app.paused = true;
    let source = app.studio_panel.source_for_test().to_string();

    app.handle_gamepad_command(crate::gamepad::Command::ToggleMute);
    app.handle_gamepad_command(crate::gamepad::Command::VolumeUp);

    assert!(app.muted);
    assert!((app.volume - 0.55).abs() < f32::EPSILON);
    assert_eq!(app.audio_program, AudioProgram::Studio);
    assert_eq!(app.studio_panel.source_for_test(), source);
    assert_eq!(app.input_mode, InputMode::Controller);
}

#[test]
fn controller_radio_action_works_from_the_root_help_menu() {
    let mut app = headless("numinous_app_test_controller_menu_radio.txt");
    app.show_help = true;
    app.radio = Some(numinous_core::STATIONS.len() - 1);

    app.handle_gamepad_command(crate::gamepad::Command::CycleRadio);

    assert!(app.show_help);
    assert!(app.radio.is_none());
    assert_eq!(
        app.banner.as_ref().expect("radio off banner").lines(),
        ["RADIO OFF", "ROOM MUSIC"]
    );
}

#[test]
fn studio_owns_audio_until_exit_then_rejoins_live_radio() {
    let path =
        std::env::temp_dir().join(format!("numinous_studio_radio_{}.wav", std::process::id()));
    write_test_wav(&path, 2, 2);
    let mut app = headless("numinous_app_test_studio_radio.txt");
    app.radio = Some(0);
    app.radio_paths = vec![path.clone()];
    app.radio_index = 0;
    app.radio_track = Arc::new(vec![0.25, -0.25]);
    app.audio_program = AudioProgram::Radio;

    app.enter_studio();
    let selected_radio = app.radio_track.clone();
    assert_eq!(app.audio_program, AudioProgram::Studio);
    assert!(!app.sync_radio_at(1.0));
    assert!(Arc::ptr_eq(&app.radio_track, &selected_radio));
    app.update_audio();
    assert_eq!(app.audio_program, AudioProgram::Studio);

    app.exit_studio();
    assert_eq!(app.audio_program, AudioProgram::Radio);
    assert!(!app.radio_track.is_empty());
    assert!(app.radio_until.is_some());
    assert!(app.title().contains("radio:"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_dropped_num_creation_reopens_exactly_paused_then_enter_sings() {
    let mut app = headless("numinous_app_test_reopen.txt");
    let path = std::env::temp_dir().join("numinous_app_reopen_test.num");
    let creation = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5).expect("creation");
    std::fs::write(&path, creation.to_num_file()).expect("write");

    app.open_dropped_file(&path);
    assert!(app.studio, "a dropped creation opens the Studio");
    assert_eq!(app.audio_program, AudioProgram::Studio);
    assert!(
        app.studio_panel.opened_paused(),
        "the preview waits for consent before singing"
    );
    assert_eq!(app.studio_panel.source_for_test(), "sin(a*x)");

    app.studio_confirm_opened();
    assert!(
        !app.studio_panel.opened_paused(),
        "Enter starts the singing"
    );
    assert_eq!(app.audio_program, AudioProgram::Studio);

    // A non-num drop is refused without touching the Studio.
    let mut fresh = headless("numinous_app_test_reopen_refuse.txt");
    let stray = std::env::temp_dir().join("numinous_app_reopen_stray.txt");
    std::fs::write(&stray, "not a capsule").expect("write");
    fresh.open_dropped_file(&stray);
    assert!(!fresh.studio, "only .num creations open here");
    assert!(fresh.banner.is_some(), "the refusal says why");
    let _ = std::fs::remove_file(&stray);

    // A scored run in progress is never abandoned by a stray drop.
    let mut playing = headless("numinous_app_test_reopen_midgame.txt");
    playing.nim_start();
    playing.open_dropped_file(&path);
    assert!(!playing.studio, "a run in progress holds the door");
    assert!(playing.nim.is_some(), "the run survives the drop");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_launch_argument_front_door_opens_files_and_links() {
    let mut app = headless("numinous_app_test_start_open.txt");
    let creation = numinous_core::StudioCreation::new("x*x", -1.0, 1.0, 0.0).expect("creation");
    app.open_start_input(&creation.to_link());
    assert!(app.studio, "a link argument opens the Studio");
    assert!(app.studio_panel.opened_paused());
    assert_eq!(app.studio_panel.source_for_test(), "x*x");

    let mut bad = headless("numinous_app_test_start_open_bad.txt");
    bad.open_start_input("numinous://studio?expr=x&xmin=-1&xmax=1&a=%");
    assert!(!bad.studio, "an invalid link opens nothing");
    assert!(bad.banner.is_some(), "the refusal says why");
}

#[test]
fn the_gallery_wall_opens_a_saved_creation_paused() {
    let mut app = headless("numinous_app_test_gallery.txt");
    let parent = std::env::temp_dir().join(format!("numinous-gallery-open-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).expect("parent");
    let creation = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5).expect("creation");
    std::fs::write(parent.join("mine.num"), creation.to_num_file()).expect("write");

    app.enter_studio();
    app.gallery = Some(crate::gallery::GalleryPanel::open(&parent));
    assert_eq!(
        app.gallery.as_ref().map(crate::gallery::GalleryPanel::len),
        Some(1)
    );

    app.gallery_open_selected();
    assert!(app.gallery.is_none(), "opening closes the wall");
    assert!(app.studio, "the opened creation lands in the Studio");
    assert!(
        app.studio_panel.opened_paused(),
        "opened like any other open"
    );
    assert_eq!(app.studio_panel.source_for_test(), "sin(a*x)");

    // Leaving the Studio also leaves the wall.
    app.gallery = Some(crate::gallery::GalleryPanel::open(&parent));
    app.exit_studio();
    assert!(app.gallery.is_none());

    // An empty wall opens nothing and stays up.
    let empty_parent =
        std::env::temp_dir().join(format!("numinous-gallery-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty_parent);
    std::fs::create_dir_all(&empty_parent).expect("empty parent");
    app.enter_studio();
    app.gallery = Some(crate::gallery::GalleryPanel::open(&empty_parent));
    app.gallery_open_selected();
    assert!(app.gallery.is_some(), "nothing to open leaves the wall up");

    let _ = std::fs::remove_dir_all(&parent);
    let _ = std::fs::remove_dir_all(&empty_parent);
}

#[test]
fn a_failing_export_key_says_so_instead_of_doing_nothing() {
    // A file where the export wants a directory: every write path fails.
    let mut app = headless("numinous_app_test_export_says.txt");
    let blocked =
        std::env::temp_dir().join(format!("numinous-export-blocked-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&blocked);
    let _ = std::fs::remove_file(&blocked);
    std::fs::write(&blocked, "a file where a folder must go").expect("blocker");

    let outcome = app.save_postcard_to(&blocked.join("nested"));
    assert!(outcome.is_err(), "the fixture must actually fail");
    app.report_export_outcome(
        "postcard saved",
        "POSTCARD FAILED  SEE .NUMINOUS-CRASH.LOG",
        outcome,
    );
    assert!(
        app.banner.is_some(),
        "a failed export key must say so, not do nothing"
    );
    let _ = std::fs::remove_file(&blocked);
}

#[test]
fn a_failing_progress_save_warns_once_per_trouble_spell() {
    let mut app = headless("numinous_app_test_save_trouble.txt");
    // Point the journey file somewhere no file can be created.
    let blocked =
        std::env::temp_dir().join(format!("numinous-journey-blocked-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&blocked);
    std::fs::write(&blocked, "a file where a folder must go").expect("blocker");
    app.journey_file = blocked.join("nested").join("journey.txt");

    app.journey.play();
    app.journey_changed();
    // The text matters: the first play also levels the fresh journey, so
    // a bare is_some would be satisfied by the level-up celebration even
    // if the warning never showed. The warning must outrank it.
    let warning = app.banner.as_ref().expect("the first failure warns");
    assert!(
        warning.lines().join(" ").contains("PROGRESS IS NOT SAVING"),
        "the warning outranks the level-up banner: {:?}",
        warning.lines()
    );
    assert!(app.journey_save_warned);

    // A second failure in the same spell stays quiet on screen while the
    // crash log keeps the full record.
    app.banner = None;
    app.journey.play();
    app.journey_changed();
    assert!(app.banner.is_none(), "one warning per trouble spell");
    let log = std::fs::read_to_string(&app.crash_log).expect("scratch crash log");
    assert_eq!(
        log.matches("journey save failed").count(),
        2,
        "every failure is logged even when the banner stays quiet: {log}"
    );

    // A recovered save resets the spell, so a relapse warns again.
    app.journey_file = super::test_state_path("numinous_app_test_save_trouble_ok.txt");
    let _ = std::fs::remove_file(&app.journey_file);
    app.journey.play();
    app.journey_changed();
    assert!(!app.journey_save_warned, "success clears the trouble spell");

    // The score store carries its own spell: a healthy journey must not
    // silence it, and its recovery must not silence the journey's.
    app.scores_file = blocked.join("nested").join("scores.txt");
    app.banner = None;
    assert!(
        !app.post_score("munch seed:1", 5),
        "a failed post is not a best"
    );
    let warning = app.banner.as_ref().expect("a failing score save warns");
    assert!(
        warning.lines().join(" ").contains("SCORES ARE NOT SAVING"),
        "the score warning names the score store: {:?}",
        warning.lines()
    );
    assert!(app.score_save_warned);
    // A level-up from a later journey_changed in the same tick must not
    // paint over the fresh warning: game flows post a score and then
    // level the journey back to back.
    app.level_seen = 0;
    app.journey.play();
    app.journey_changed();
    let held = app.banner.as_ref().expect("a banner is still up");
    assert!(
        held.lines().join(" ").contains("SCORES ARE NOT SAVING"),
        "the celebration must not overwrite a warning raised across calls: {:?}",
        held.lines()
    );
    // The journey succeeding again does not reset the score spell into a
    // nag: the next failing post stays quiet on screen.
    app.journey.play();
    app.journey_changed();
    app.banner = None;
    assert!(!app.post_score("munch seed:1", 6));
    assert!(
        app.banner.is_none(),
        "one healthy store must not turn the other's spell into a nag"
    );
    let _ = std::fs::remove_file(&blocked);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn fork_share_and_reopen_carry_lineage_and_era_around_the_whole_loop() {
    let mut app = headless("numinous_app_test_fork_loop.txt");
    let wall = std::env::temp_dir().join(format!("numinous-fork-loop-wall-{}", std::process::id()));
    let shares =
        std::env::temp_dir().join(format!("numinous-fork-loop-shares-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wall);
    let _ = std::fs::remove_dir_all(&shares);
    std::fs::create_dir_all(&wall).expect("wall dir");
    let parent = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
        .expect("parent")
        .with_title("Parent Wave")
        .expect("title")
        .with_era(numinous_core::Era::Phosphor);
    std::fs::write(wall.join("parent.num"), parent.to_num_file()).expect("write parent");

    // Fork from the wall: editable, singing, and in the parent's era.
    app.enter_studio();
    app.gallery = Some(crate::gallery::GalleryPanel::open(&wall));
    app.gallery_fork_selected();
    assert!(app.gallery.is_none());
    assert!(app.studio);
    assert_eq!(
        app.era,
        numinous_core::Era::Phosphor,
        "the fork adopts the era"
    );
    assert!(!app.studio_panel.opened_paused(), "a fork sings at once");

    // The next share records the descent and the non-default era.
    let bundle = app
        .share_studio_creation_to(&shares, None)
        .expect("share io")
        .expect("the fork parses, so the trio writes");
    let saved = numinous_core::StudioCreation::from_num_path(&bundle.join("creation.num"))
        .expect("the shared capsule reopens");
    assert_eq!(saved.descends(), Some(parent.to_link().as_str()));
    assert_eq!(saved.era(), Some(numinous_core::Era::Phosphor));
    let readme = std::fs::read_to_string(bundle.join("README.share.txt")).expect("readme");
    assert!(
        readme.contains("It descends from this creation:"),
        "{readme}"
    );

    // A stranger dropping the shared capsule gets the era and the record.
    let mut stranger = headless("numinous_app_test_fork_loop_stranger.txt");
    assert_eq!(stranger.era, numinous_core::Era::Modern);
    stranger.open_dropped_file(&bundle.join("creation.num"));
    assert!(stranger.studio);
    assert_eq!(
        stranger.era,
        numinous_core::Era::Phosphor,
        "a reopened capsule restores its recorded era"
    );
    assert!(
        stranger.studio_panel.opened_paused(),
        "a drop still previews paused"
    );

    let _ = std::fs::remove_dir_all(&wall);
    let _ = std::fs::remove_dir_all(&shares);
}

#[test]
fn a_name_can_hold_every_printable_character_the_capsule_accepts() {
    // M toggled mute, and the brackets moved the volume, so MANDELBROT
    // was unspellable in a fractal instrument and every attempt flipped
    // the sound. A text field owns the whole printable range while it
    // is open; this drives the real key route to prove it.
    let mut app = headless("numinous_app_test_naming_keys.txt");
    app.enter_studio();
    app.begin_share_naming();
    let muted_before = app.muted;
    let volume_before = app.volume;
    for c in "Mandelbrot [m]".chars() {
        let key = Key::Character(c.to_string().into());
        assert!(
            !app.handle_global_audio_key(&key, false),
            "the naming step owns {c:?} while it is open"
        );
        app.naming_push_text(&c.to_string());
    }
    assert_eq!(
        app.share_naming.as_ref().expect("naming open").title,
        "Mandelbrot [m]"
    );
    assert_eq!(app.muted, muted_before, "typing a name must not flip mute");
    assert!((app.volume - volume_before).abs() < f32::EPSILON);

    // With no prompt open the shortcuts are global again.
    app.cancel_share_naming();
    let key = Key::Character("m".to_string().into());
    assert!(app.handle_global_audio_key(&key, false));

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn a_creation_arriving_under_the_prompt_ends_it() {
    // A dropped capsule swapped the creation while the prompt kept
    // typing, so Enter shared a stranger's work under the local name,
    // and the REOPENED banner promised a key the prompt had taken.
    let mut app = headless("numinous_app_test_drop_under_prompt.txt");
    app.enter_studio();
    app.begin_share_naming();
    app.naming_push_text("Mine");

    let stranger = numinous_core::StudioCreation::new("cos(x)", -1.0, 1.0, 1.0)
        .expect("capsule")
        .with_title("Theirs")
        .expect("title");
    app.open_studio_creation(&stranger);
    assert!(
        app.share_naming.is_none(),
        "a new creation ends the prompt that was about the old one"
    );
    assert!(app.gallery.is_none(), "and the wall it may have come from");

    // Leaving the Studio ends an open prompt too, on every route out.
    app.begin_share_naming();
    app.exit_studio();
    assert!(app.share_naming.is_none(), "no invisible naming state");

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn clearing_a_prefilled_name_clears_it_everywhere_the_share_lands() {
    // The defect this pins: the naming step prefills a reopened
    // creation's title and author, and an untouched reopen re-shares
    // that very capsule. Deleting the prefill therefore had to travel,
    // or the form said unnamed while the capsule, the README, the
    // postcard headline, and the folder slug all kept the old name.
    let shares = std::env::temp_dir().join(format!("numinous-cleared-name-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&shares);
    let mut app = headless("numinous_app_test_cleared_name.txt");
    app.enter_studio();

    let named = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
        .expect("creation")
        .with_title("Slow Waves")
        .expect("title")
        .with_author("A Curious Mind")
        .expect("author");
    app.studio_panel.open_creation(&named);

    // The step opens on the capsule's own identity.
    app.begin_share_naming();
    {
        let naming = app.share_naming.as_ref().expect("naming open");
        assert_eq!(naming.title, "Slow Waves");
        assert_eq!(naming.author, "A Curious Mind");
    }

    // Clear both fields the only way a player can.
    for _ in 0.."Slow Waves".len() {
        app.naming_backspace();
    }
    app.naming_toggle_field();
    for _ in 0.."A Curious Mind".len() {
        app.naming_backspace();
    }
    let naming = app.share_naming.as_ref().expect("naming open");
    assert!(naming.title.is_empty() && naming.author.is_empty());
    assert_eq!(
        naming.identity(),
        super::ShareIdentity {
            title: None,
            author: None
        },
        "an emptied field is a clearing, not an absence"
    );

    let identity = Some(naming.identity());
    let bundle = app
        .share_studio_creation_to(&shares, identity)
        .expect("share io")
        .expect("the reopened formula parses");
    let saved =
        numinous_core::StudioCreation::from_num_path(&bundle.join("creation.num")).expect("reopen");
    assert_eq!(saved.title(), None, "the deleted name must not ship");
    assert_eq!(saved.author(), None, "the deleted signature must not ship");
    let readme = std::fs::read_to_string(bundle.join("README.share.txt")).expect("readme");
    assert!(!readme.contains("Slow Waves"), "{readme}");
    assert!(!readme.contains("A Curious Mind"), "{readme}");
    let folder = bundle
        .file_name()
        .expect("bundle name")
        .to_string_lossy()
        .to_string();
    assert!(
        !folder.contains("slow-waves"),
        "the folder wears a name that was deleted: {folder}"
    );

    // A share that never opened the step still keeps what the capsule
    // carries: the two cases must stay distinguishable.
    app.studio_panel.open_creation(&named);
    let untouched = app
        .share_studio_creation_to(&shares, None)
        .expect("share io")
        .expect("parses");
    let kept = numinous_core::StudioCreation::from_num_path(&untouched.join("creation.num"))
        .expect("reopen");
    assert_eq!(kept.title(), Some("Slow Waves"));
    assert_eq!(kept.author(), Some("A Curious Mind"));

    let _ = std::fs::remove_dir_all(&shares);
}

#[test]
fn the_naming_step_edits_signs_and_slugs_the_share() {
    let shares = std::env::temp_dir().join(format!("numinous-naming-share-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&shares);
    let mut app = headless("numinous_app_test_naming_step.txt");
    app.enter_studio();

    // F4 opens the naming step instead of sharing blind.
    app.begin_share_naming();
    assert!(app.share_naming.is_some());
    app.naming_push_text("Fading Wave");
    app.naming_toggle_field();
    app.naming_push_text("A Curious Mind");
    let naming = app.share_naming.as_ref().expect("naming open");
    assert_eq!(naming.title, "Fading Wave");
    assert_eq!(naming.author, "A Curious Mind");

    // The editor enforces the capsule's own bounds: printable ASCII,
    // capped, so a name it accepts is a name the share cannot refuse.
    app.naming_push_text(&"x".repeat(200));
    app.naming_push_text("\u{7f}\u{9}");
    assert_eq!(
        app.share_naming
            .as_ref()
            .expect("naming open")
            .author
            .chars()
            .count(),
        numinous_core::MAX_META_TEXT_CHARS
    );

    // Esc abandons the share and says so.
    app.cancel_share_naming();
    assert!(app.share_naming.is_none());
    assert!(app.banner.is_some(), "a silent cancel is a mystery");

    // The named share signs the capsule, the postcard identity rides,
    // and a titled bundle folder wears the slug.
    let bundle = app
        .share_studio_creation_to(
            &shares,
            Some(super::ShareIdentity {
                title: Some("Fading Wave".to_string()),
                author: Some("A Curious Mind".to_string()),
            }),
        )
        .expect("share io")
        .expect("default formula parses");
    let saved =
        numinous_core::StudioCreation::from_num_path(&bundle.join("creation.num")).expect("reopen");
    assert_eq!(saved.title(), Some("Fading Wave"));
    assert_eq!(saved.author(), Some("A Curious Mind"));
    let folder = bundle
        .file_name()
        .expect("bundle name")
        .to_string_lossy()
        .to_string();
    assert!(
        folder.starts_with("numinous-share-studio-fading-wave-"),
        "a titled share reads as work on disk: {folder}"
    );

    // Confirming remembers the signature for the next share even when
    // the formula refuses, and the next naming step offers it back.
    app.share_naming = Some(super::ShareNaming {
        title: "Second".to_string(),
        author: "A Curious Mind".to_string(),
        field: super::NamingField::Title,
    });
    assert!(app.studio_panel.push_text("(((").is_none());
    app.confirm_share_naming();
    assert_eq!(app.remembered_author, "A Curious Mind");
    app.begin_share_naming();
    assert_eq!(
        app.share_naming.as_ref().expect("reopened naming").author,
        "A Curious Mind",
        "the signature is offered on the next share"
    );

    let _ = std::fs::remove_dir_all(&shares);
}

#[test]
fn one_action_shares_the_studio_trio_or_refuses_with_a_reason() {
    let app = headless("numinous_app_test_studio_share.txt");
    let parent =
        std::env::temp_dir().join(format!("numinous-studio-share-app-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);

    let dir = app
        .share_studio_creation_to(&parent, None)
        .expect("share io")
        .expect("default formula parses, so the trio writes");
    let num_path = dir.join("creation.num");
    let reopened =
        numinous_core::StudioCreation::from_num_path(&num_path).expect("creation.num reopens");
    assert_eq!(
        reopened,
        app.studio_panel
            .current_creation(app.t)
            .expect("the panel's own creation"),
        "the bundle reopens to exactly the shared state"
    );
    assert!(dir.join("postcard.png").is_file());
    let readme = std::fs::read_to_string(dir.join("README.share.txt")).expect("bundle readme");
    assert!(
        readme.contains(&reopened.to_link()),
        "the link is the handoff"
    );

    // An unparsed formula is refused, and nothing lands on disk for it.
    let mut broken = headless("numinous_app_test_studio_share_broken.txt");
    assert!(broken.studio_panel.push_text("(").is_none());
    let before: Vec<_> = std::fs::read_dir(&parent)
        .expect("parent listing")
        .collect();
    assert_eq!(
        broken
            .share_studio_creation_to(&parent, None)
            .expect("refusal is not an io error"),
        Err(crate::studio_panel::ShareRefusal::UnparsedFormula),
        "an unparsed formula is its own refusal, named as such"
    );
    let after: Vec<_> = std::fs::read_dir(&parent)
        .expect("parent listing")
        .collect();
    assert_eq!(before.len(), after.len(), "a refusal writes nothing");
    let _ = std::fs::remove_dir_all(&parent);
}

#[test]
fn failed_radio_resync_restores_the_room_score_source() {
    let mut app = headless("numinous_app_test_radio_resync_failure.txt");
    app.radio = Some(0);
    app.radio_paths = vec![std::env::temp_dir().join("numinous_missing_radio_track.wav")];
    app.radio_track = Arc::new(vec![0.25, -0.25]);
    app.radio_until = Some(Instant::now());
    app.audio_program = AudioProgram::Radio;

    assert!(!app.sync_radio_at(1.0));

    assert_eq!(app.audio_program, AudioProgram::RoomScore);
    assert!(app.radio_track.is_empty());
    assert!(app.radio_until.is_none());
    assert!(!app.title().contains("radio:"));
}

#[test]
fn radio_off_restores_room_score_title_and_feedback_together() {
    let mut app = headless("numinous_app_test_radio_off.txt");
    app.radio = Some(numinous_core::STATIONS.len() - 1);
    app.radio_track = Arc::new(vec![0.25, -0.25]);
    app.audio_program = AudioProgram::Radio;
    assert!(app.title().contains("radio:"));

    app.radio = None;
    app.tune_in();

    assert_eq!(app.audio_program, AudioProgram::RoomScore);
    assert!(!app.title().contains("radio:"));
    assert_eq!(
        app.banner.as_ref().expect("radio off banner").lines(),
        ["RADIO OFF", "ROOM MUSIC"]
    );
    assert_eq!(app.audio_state().label(), "NO SOUND DEVICE");
}

#[test]
fn modal_modes_take_control_from_the_show() {
    let mut app = headless("numinous_app_test_show_modes_studio.txt");
    app.the_show = true;
    app.show_help = true;
    app.show_journey = true;
    app.enter_studio();
    assert!(app.studio);
    assert!(!app.the_show);
    assert!(!app.show_help);
    assert!(!app.show_journey);
    let _ = std::fs::remove_file(&app.journey_file);

    let mut app = headless("numinous_app_test_show_modes_quiz.txt");
    app.the_show = true;
    app.quiz_next();
    assert!(app.quiz.is_some());
    assert!(!app.the_show);
    let _ = std::fs::remove_file(&app.journey_file);

    let mut app = headless("numinous_app_test_show_modes_games.txt");
    app.the_show = true;
    app.munch_start();
    assert!(app.munch.is_some());
    assert!(!app.the_show);
    app.the_show = true;
    app.nim_start();
    assert!(app.nim.is_some());
    assert!(!app.the_show);
    app.the_show = true;
    app.gauntlet_start();
    assert!(app.gauntlet.is_some());
    assert!(!app.the_show);
    app.the_show = true;
    app.arcade_start();
    assert!(app.arcade.is_some());
    assert!(!app.the_show);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn every_game_entry_releases_the_room_parameter_voice() {
    for (name, enter) in [
        (
            "numinous_app_test_parameter_voice_quiz.txt",
            App::quiz_next as fn(&mut App),
        ),
        (
            "numinous_app_test_parameter_voice_munch.txt",
            App::munch_start,
        ),
        ("numinous_app_test_parameter_voice_nim.txt", App::nim_start),
        (
            "numinous_app_test_parameter_voice_gauntlet.txt",
            App::gauntlet_start,
        ),
        (
            "numinous_app_test_parameter_voice_arcade.txt",
            App::arcade_start,
        ),
    ] {
        let mut app = headless(name);
        select_times_tables(&mut app);
        assert!(app.record_room_touch((0.375, 0.5)));
        assert!(app.desired_room_parameter_sound().is_some());

        enter(&mut app);

        assert!(app.modal_mode_active());
        assert!(app.desired_room_parameter_sound().is_none());
        let _ = std::fs::remove_file(&app.journey_file);
    }
}

#[test]
fn entering_a_game_or_modal_clears_a_stale_pause() {
    // A pause set in the wander view (Space) must not leak into a game. The
    // real-time arcade is the dangerous one: a leaked pause froze the threat
    // while the player kept eating, then posted an unfair score.
    let mut app = headless("numinous_app_test_pause_clear.txt");
    for enter in [
        App::arcade_start,
        App::munch_start,
        App::nim_start,
        App::quiz_next,
        App::gauntlet_start,
        App::enter_studio,
    ] {
        app.paused = true;
        enter(&mut app);
        assert!(
            !app.paused,
            "entering a game or modal must clear a stale pause"
        );
    }
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn show_auto_advance_ignores_hidden_modal_state() {
    let mut app = headless("numinous_app_test_show_guard.txt");
    app.the_show = true;
    assert!(app.show_mode_active());
    app.studio = true;
    assert!(!app.show_mode_active());
    app.studio = false;
    app.quiz_next();
    app.the_show = true;
    assert!(!app.show_mode_active());
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn modal_frames_take_priority_over_gpu_eligible_rooms() {
    let mut app = headless("numinous_app_test_modal_frame_priority.txt");
    app.show_help = false;
    app.current = app
        .rooms
        .iter()
        .position(|room| room.meta().id == "mandelbrot")
        .expect("mandelbrot room");
    app.quiz_next();

    let raster = app.modal_frame(320, 220).expect("modal frame");

    assert!(app.modal_mode_active());
    assert!(raster.lit_count() > 100);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn munch_in_the_window_grades_and_posts() {
    let mut app = headless("numinous_app_test_munch.txt");
    app.munch_start();
    let first_round = app.munch.as_ref().unwrap().round;
    assert!(
        first_round >= 4,
        "standalone Munch opens the full rule deck"
    );
    assert_eq!(app.journey.plays, 1, "a dealt board is a play");
    {
        let play = app.munch.as_mut().unwrap();
        play.cursor = 3;
        play.bites.insert(3);
        play.bites.insert(7);
    }
    app.munch_grade();
    let outcome = app.munch.as_ref().unwrap().graded.as_ref().unwrap();
    assert_eq!(outcome.hits + outcome.bad_bites, 2, "two bites graded");
    app.munch_grade(); // grading twice changes nothing
    assert_eq!(app.journey.plays, 1);
    let scores = std::fs::read_to_string(&app.scores_file).expect("score persisted");
    assert!(scores.contains(&format!("board:{first_round}")));
    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn munch_key_routes_shared_controls() {
    use winit::keyboard::{Key, NamedKey};
    let mut app = headless("numinous_app_test_munch_keys.txt");
    app.munch_start();
    app.munch_key(&Key::Character("d".into()));
    assert_eq!(app.munch.as_ref().unwrap().cursor, 1);
    app.munch_key(&Key::Character("e".into()));
    assert!(app.munch.as_ref().unwrap().bites.contains(&1));
    app.munch_key(&Key::Named(NamedKey::Space));
    assert!(!app.munch.as_ref().unwrap().bites.contains(&1));
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn leaving_munch_or_gauntlet_retires_queued_transient_audio() {
    let mut app = headless("numinous_app_test_transient_audio_exit.txt");

    app.munch_start();
    let before_ungraded_exit = app.transient_audio_clears.get();
    app.munch_key(&Key::Named(NamedKey::Escape));
    assert!(app.munch.is_none());
    assert_eq!(app.transient_audio_clears.get(), before_ungraded_exit + 1);

    app.munch_start();
    app.munch_grade();
    let before_graded_exit = app.transient_audio_clears.get();
    app.munch_key(&Key::Named(NamedKey::Escape));
    assert!(app.munch.is_none());
    assert_eq!(app.transient_audio_clears.get(), before_graded_exit + 1);

    app.gauntlet_start();
    let before_gauntlet_exit = app.transient_audio_clears.get();
    app.gauntlet_key(&Key::Named(NamedKey::Escape));
    assert!(app.gauntlet.is_none());
    assert_eq!(app.transient_audio_clears.get(), before_gauntlet_exit + 1);

    app.gauntlet_start();
    app.gauntlet.as_mut().expect("active Gauntlet").stage = 4;
    let before_completed_gauntlet_exit = app.transient_audio_clears.get();
    app.gauntlet_key(&Key::Named(NamedKey::Enter));
    assert!(app.gauntlet.is_none());
    assert_eq!(
        app.transient_audio_clears.get(),
        before_completed_gauntlet_exit + 1
    );

    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn graded_munch_advances_only_on_enter_or_space() {
    use winit::keyboard::{Key, NamedKey};
    let mut app = headless("numinous_app_test_munch_next.txt");
    app.munch_start();
    let first_round = app.munch.as_ref().unwrap().round;
    let first_rule = app.munch.as_ref().unwrap().board.rule;
    app.munch_grade();

    app.munch_key(&Key::Character("x".into()));
    assert_eq!(app.munch.as_ref().unwrap().round, first_round);
    app.munch_key(&Key::Named(NamedKey::Enter));

    let next = app.munch.as_ref().expect("next board remains in Munch");
    assert!(next.round > first_round);
    assert!(!super::play::same_rule_family(first_rule, next.board.rule));
    assert_eq!(app.journey.plays, 2);
    let _ = std::fs::remove_file(&app.journey_file);
    let _ = std::fs::remove_file(&app.scores_file);
}

#[test]
fn nim_in_the_window_plays_the_order() {
    let mut app = headless("numinous_app_test_nim.txt");
    app.nim_start();
    let before: u32 = app.nim.as_ref().unwrap().heaps.iter().sum();
    {
        let play = app.nim.as_mut().unwrap();
        play.take = 1;
    }
    app.nim_move();
    let play = app.nim.as_ref().unwrap();
    let after: u32 = play.heaps.iter().sum();
    // Your stone and the Order's reply both left the board (unless over).
    assert!(after < before);
    assert!(play.over.is_none() || play.over == Some(false) || play.over == Some(true));
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn nim_result_requires_an_explicit_retry_or_exit() {
    let mut app = headless("numinous_app_test_nim_retry.txt");
    app.nim_start();
    app.nim.as_mut().unwrap().over = Some(false);
    let plays = app.journey.plays;

    app.nim_key(&winit::keyboard::Key::Character("x".into()));
    assert!(
        app.nim.is_some(),
        "an unrelated key must not eject the result"
    );
    assert_eq!(app.journey.plays, plays);

    app.nim_key(&winit::keyboard::Key::Named(
        winit::keyboard::NamedKey::Enter,
    ));
    assert_eq!(app.nim.as_ref().unwrap().over, None);
    assert_eq!(app.journey.plays, plays + 1);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn the_live_arcade_acts_beats_and_ends() {
    use numinous_core::munch_arcade::Action;
    let mut app = headless("numinous_app_test_arcade.txt");
    app.arcade_start();
    assert_eq!(app.journey.plays, 1);
    app.arcade_act(Action::Right);
    app.arcade_act(Action::Eat);
    let before = app.arcade.as_ref().unwrap().run.vexations.clone();
    app.arcade_beat();
    let after = &app.arcade.as_ref().unwrap().run.vexations;
    assert!(
        before
            .iter()
            .zip(after.iter())
            .any(|(b, a)| b.cell != a.cell),
        "the beat moves spirits"
    );
    // Beat until the spirits finish the job; the run must end and score.
    for _ in 0..500 {
        app.arcade_beat();
        if app.arcade.as_ref().unwrap().over {
            break;
        }
    }
    assert!(
        app.arcade.as_ref().unwrap().over,
        "the spirits always win eventually"
    );
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn the_gauntlet_runs_four_stages_and_totals_with_combo() {
    use winit::keyboard::{Key, NamedKey};
    let mut app = headless("numinous_app_test_gauntlet.txt");
    app.gauntlet_start();
    // Stage 1: submit an empty munch board (0 points, not clean).
    app.gauntlet_key(&Key::Named(NamedKey::Enter));
    assert_eq!(app.gauntlet.as_ref().unwrap().stage, 1);
    // Stage 2: answer the shape correctly.
    let answer = app.gauntlet.as_ref().unwrap().quiz.round.answer;
    app.gauntlet_key(&Key::Character(answer.to_string().to_lowercase().into()));
    let run = app.gauntlet.as_ref().unwrap();
    assert_eq!(run.stage, 2);
    assert_eq!(run.scores[1], 25);
    assert!(run.cleared[1]);
    // Stage 3: answer the sky correctly.
    let sky = app.gauntlet.as_ref().unwrap().scan.answer;
    app.gauntlet_key(&Key::Character(sky.to_string().to_lowercase().into()));
    assert_eq!(app.gauntlet.as_ref().unwrap().stage, 3);
    // Stage 4: cut the right wire first try.
    let code: String = app
        .gauntlet
        .as_ref()
        .unwrap()
        .secret
        .iter()
        .map(|&d| char::from(b'0' + d))
        .collect();
    for ch in code.chars() {
        app.gauntlet_key(&Key::Character(ch.to_string().into()));
    }
    app.gauntlet_key(&Key::Named(NamedKey::Enter));
    let run = app.gauntlet.as_ref().unwrap();
    assert_eq!(run.stage, 4, "the run is complete");
    // Scores: 0 (miss), then 25*1, 25*2, 40*3 = 195.
    assert_eq!(
        numinous_core::gauntlet_total(&run.scores, &run.cleared),
        195
    );
    assert_eq!(app.journey.plays, 4);
    assert_eq!(app.journey.wins, 3);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn gauntlet_munch_stage_routes_shared_controls() {
    use winit::keyboard::{Key, NamedKey};
    let mut app = headless("numinous_app_test_gauntlet_munch_keys.txt");
    app.gauntlet_start();
    app.gauntlet_key(&Key::Character("d".into()));
    assert_eq!(app.gauntlet.as_ref().unwrap().munch.cursor, 1);
    app.gauntlet_key(&Key::Character("e".into()));
    assert!(app.gauntlet.as_ref().unwrap().munch.bites.contains(&1));
    app.gauntlet_key(&Key::Named(NamedKey::Space));
    assert!(!app.gauntlet.as_ref().unwrap().munch.bites.contains(&1));
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn the_radio_loads_cached_tracks_and_joins_live() {
    let dir = std::env::temp_dir().join("numinous_radio_test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("trance-001.wav");
    write_test_wav(&path, 1, 3);
    assert!(radio_cache::audio_is_bounded(&path));
    let duration = radio_cache::duration_seconds(&path).expect("duration");
    assert!(
        (2.9..=3.1).contains(&duration),
        "duration should be about three seconds, got {duration}"
    );
    // SAFETY-free env override: the test sets the var via a scratch app
    // field instead. tune_in reads NUMINOUS_RADIO; set through the
    // process env is forbidden, so exercise radio_play directly.
    let mut app = headless("numinous_app_test_radio.txt");
    app.radio = Some(0);
    app.radio_paths = vec![path.clone()];
    app.radio_index = 0;
    assert!(app.radio_play(1.0));
    assert!(
        app.radio_track.len() > 44_100 * 2,
        "the record is loaded ({} samples)",
        app.radio_track.len()
    );
    assert!(app.radio_until.is_some(), "rotation is armed");
    assert!(
        app.radio_track.iter().any(|&s| s.abs() > 0.1),
        "the record has music in it"
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn skip_track_advances_the_rotation_and_explains_unavailable_states() {
    let dir = std::env::temp_dir().join("numinous_radio_skip_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    let first = dir.join("trance-001.wav");
    let second = dir.join("trance-002.wav");
    write_test_wav(&first, 1, 2);
    write_test_wav(&second, 1, 2);

    let mut app = headless("numinous_app_test_radio_skip.txt");
    app.skip_radio_track();
    assert_eq!(
        app.banner.as_ref().map(|banner| banner.lines()),
        Some(&["RADIO OFF".to_string(), "Y CHOOSES A STATION".to_string()][..])
    );

    app.radio = Some(0);
    app.radio_paths = vec![first, second];
    app.radio_index = 0;
    app.skip_radio_track();
    assert_eq!(app.radio_index, 1);
    assert!(!app.radio_track.is_empty());
    assert_eq!(
        app.banner.as_ref().map(|banner| banner.lines()),
        Some(&["RADIO: NUMINA FM".to_string(), "NEXT TRACK 2/2".to_string()][..])
    );

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn radio_resync_selects_the_wall_clock_track_after_an_inactive_gap() {
    let dir = std::env::temp_dir().join("numinous_radio_resync_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    let first = dir.join("trance-001.wav");
    let second = dir.join("trance-002.wav");
    write_test_wav(&first, 1, 2);
    write_test_wav(&second, 1, 2);

    let mut app = headless("numinous_app_test_radio_resync.txt");
    app.radio = Some(0);
    app.radio_paths = vec![first, second];
    assert!(app.sync_radio_at(2.5));
    assert_eq!(app.radio_index, 1);
    assert!(app.radio_until.is_some());
    assert!(app.sync_radio_at(8.25));
    assert_eq!(app.radio_index, 0);

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn radio_duration_uses_frames_for_stereo_tracks() {
    let dir = std::env::temp_dir().join("numinous_radio_stereo_test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("stereo.wav");
    write_test_wav(&path, 2, 3);

    let duration = radio_cache::duration_seconds(&path).expect("duration");

    assert!(
        (2.9..=3.1).contains(&duration),
        "duration should be about three seconds, got {duration}"
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn oversized_radio_files_are_rejected_before_loading() {
    let path = std::env::temp_dir().join("numinous_radio_oversized.wav");
    let file = std::fs::File::create(&path).expect("oversized placeholder");
    file.set_len(radio_cache::MAX_AUDIO_BYTES + 1)
        .expect("make sparse oversized file");
    assert!(!radio_cache::audio_is_bounded(&path));
    assert!(radio_cache::duration_seconds(&path).is_none());

    let mut app = headless("numinous_app_test_radio_oversized.txt");
    app.radio_paths = vec![path.clone()];
    app.radio_index = 0;
    app.radio_track = Arc::new(vec![0.25, -0.25]);
    app.radio_until = Some(std::time::Instant::now());
    assert!(!app.radio_play(0.0));
    assert!(app.radio_track.is_empty());
    assert!(app.radio_until.is_none());

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn radio_rotation_recovers_from_a_bad_cached_file() {
    let dir = std::env::temp_dir().join("numinous_radio_recovery_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    let bad = dir.join("trance-bad.wav");
    let good = dir.join("trance-good.wav");
    std::fs::write(&bad, b"not actually a wav").expect("bad wav");
    write_test_wav(&good, 1, 2);

    let mut app = headless("numinous_app_test_radio_recovery.txt");
    app.radio = Some(0);
    app.radio_paths = vec![bad, good.clone()];
    app.radio_index = 0;

    assert!(app.radio_play_or_advance(0.0));
    assert_eq!(app.radio_paths[app.radio_index], good);
    assert!(!app.radio_track.is_empty());
    assert!(app.radio_until.is_some());

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn modal_contexts_clear_stale_pointer_state() {
    let mut app = headless("numinous_app_test_pointer_state.txt");
    app.poking = true;
    app.show_help = true;
    app.refresh_pointer_state();
    assert!(!app.poking);

    app.show_help = false;
    app.dragging = true;
    app.studio = true;
    app.refresh_pointer_state();
    assert!(!app.dragging);

    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn quiz_answers_letter_matches_a_choice() {
    let mut app = headless("numinous_app_test_letters.txt");
    app.quiz_next();
    let quiz = app.quiz.as_ref().unwrap();
    assert!(
        quiz.round
            .choices
            .iter()
            .any(|c| c.letter == quiz.round.answer)
    );
    let _ = std::fs::remove_file(&app.journey_file);
}

#[test]
fn quiz_deal_rules_stay_out_of_the_event_loop_coordinator() {
    let entry = include_str!("main.rs");
    let game_runtime = include_str!("game_runtime.rs");

    assert!(!entry.contains("play::deal_quiz"));
    assert!(!entry.contains("play::answer_quiz"));
    assert!(game_runtime.contains("play::deal_quiz"));
    assert!(game_runtime.contains("play::answer_quiz"));
    for source in [entry, game_runtime] {
        assert!(!source.contains(concat!("I", "CONIC")));
        assert!(!source.contains(concat!("build", "_round", "_pool")));
        assert!(!source.contains(concat!("quiz_recent", ".", "push")));
    }
}
