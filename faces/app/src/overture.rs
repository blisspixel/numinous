//! The App's Overture: one score, four rooms, then the dial.
//!
//! The clock is raw presentation time. It does not read the player's speed
//! setting or the visualizer. A beat change is not a visit and does not rebuild
//! the bed. The first room action ends the score lock and keeps the picture.

use std::sync::Arc;

use numinous_core::{OVERTURE, OvertureMoment, ROOM_BED_SOURCE_RATE};
use winit::keyboard::{Key, NamedKey};

use super::App;

/// A performance of [`OVERTURE`] while the score owns the picture and the phase.
pub(super) struct OverturePlay {
    /// Milliseconds into the full-motion piece, clamped by the score when read.
    pub(super) at_ms: u32,
    /// Presentation seconds accumulated so far. `at_ms` is this value in
    /// milliseconds, floored, so leftover fractions survive from frame to frame.
    pub(super) elapsed: f64,
    /// Reduced motion steps this still. Full motion leaves it empty.
    pub(super) reduced_beat: Option<usize>,
    /// Catalog index currently on stage, once the lead-in has ended.
    pub(super) presented_room: Option<usize>,
    /// The hand-off room's motif, shared with [`App::tune`] for the whole piece.
    pub(super) bed: Arc<Vec<f32>>,
    /// The near-black opening, before the first room.
    pub(super) lead_in: bool,
    /// The last key is held and Route Lab may be offered.
    pub(super) handoff: bool,
}

impl App {
    /// Close the menu and start the Overture from its opening.
    ///
    /// Replaying clears the Route Lab door until this performance holds the
    /// last dial again. The four rooms are the canonical variations the score
    /// was written against.
    pub(super) fn start_overture(&mut self) {
        self.close_menu();
        self.the_show = false;
        self.show_journey = false;
        self.paused = false;
        if !self.save_warning_showing() {
            self.banner = None;
        }
        self.route_lab_after_overture = false;
        self.menu.set_route_lab_door(false);
        self.install_overture_rooms();
        let bed = overture_bed();
        self.tune = Arc::clone(&bed);
        let reduced_beat = (!self.motion.animates()).then_some(0);
        self.overture = Some(OverturePlay {
            at_ms: 0,
            elapsed: 0.0,
            reduced_beat,
            presented_room: None,
            bed,
            lead_in: reduced_beat.is_none(),
            handoff: false,
        });
        self.room_wager = None;
        self.update_audio();
        self.apply_current_overture_moment();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    /// Leave the score, if one is running, and enter `route-lab`.
    ///
    /// The door is a choice. Nothing in the Overture calls this on its own.
    pub(super) fn enter_route_lab_door(&mut self) {
        self.release_overture();
        self.the_show = false;
        self.show_journey = false;
        let Some(index) = numinous_core::catalog_index("route-lab") else {
            return;
        };
        self.choose_route(None);
        self.goto_room_index(index);
    }

    /// End the score lock. The visible room, its phase, and the door flag stay.
    pub(super) fn release_overture(&mut self) {
        let camera = self.overture_mandelbrot_camera();
        if self.overture.take().is_none() {
            return;
        }
        if let Some(camera) = camera {
            self.mandelbrot_camera = camera;
        }
        self.sync_room_parameter_voice();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    /// The score's canonical view, shared by accelerated rendering and takeover.
    pub(super) fn overture_mandelbrot_camera(
        &self,
    ) -> Option<numinous_core::rooms::mandelbrot::MandelbrotCamera> {
        if self.rooms[self.current].meta().id != "mandelbrot"
            || !self.overture.as_ref().is_some_and(|play| !play.lead_in)
        {
            return None;
        }
        // The score installed the canonical room, regardless of the visit's
        // variation. Both render paths and the handoff must use that same view.
        Some(numinous_core::rooms::mandelbrot::MandelbrotCamera::from_phase(self.t, 0))
    }

    /// Move the score by raw presentation seconds.
    ///
    /// Pause, an open menu, study, and reduced motion hold `at_ms`. A jump
    /// lands on the destination once.
    pub(super) fn advance_overture(&mut self, seconds: f64) {
        if self.overture.is_none() || self.overture_clock_held() {
            return;
        }
        if seconds.is_finite()
            && seconds > 0.0
            && let Some(play) = self.overture.as_mut()
        {
            play.elapsed += seconds;
            if !play.elapsed.is_finite() || play.elapsed < 0.0 {
                play.elapsed = 0.0;
            }
            let ms = (play.elapsed * 1000.0).floor().max(0.0);
            play.at_ms = u32::try_from(ms as u64).unwrap_or(u32::MAX);
        }
        self.apply_current_overture_moment();
    }

    /// Keyboard half of the score lock.
    ///
    /// Returns whether the key was consumed. Pause, mute, the menu, volume,
    /// study, and The Show's own keys pass through. `r` is ignored. The first
    /// room action ends the lock and does not also change rooms.
    pub(super) fn overture_key(&mut self, key: &Key) -> bool {
        if self.overture.is_none() || self.overture_input_blocked() {
            return false;
        }
        let key = numinous_app::controls::normalized_command_key(key);
        if matches!(&key, Key::Character(text) if text.as_str() == "r") {
            return true;
        }
        if self
            .overture
            .as_ref()
            .is_some_and(|play| play.reduced_beat.is_some())
            && let Some(delta) = reduced_step_key(&key)
        {
            self.overture_step(delta);
            return true;
        }
        if overture_room_action(&key) {
            self.release_overture();
            return true;
        }
        false
    }

    /// Controller half of the score lock. Same contract as [`Self::overture_key`].
    ///
    /// A primary press and a held pointer return false after the caller records
    /// them on the room that was showing. Reset is ignored.
    pub(super) fn overture_gamepad(&mut self, command: super::gamepad::Command) -> bool {
        use super::gamepad::Command;
        if self.overture.is_none() || self.overture_input_blocked() {
            return false;
        }
        let reduced = self
            .overture
            .as_ref()
            .is_some_and(|play| play.reduced_beat.is_some());
        match command {
            Command::Reset => true,
            Command::Right | Command::NextRoom if reduced => {
                self.overture_step(1);
                true
            }
            Command::Left | Command::PreviousRoom if reduced => {
                self.overture_step(-1);
                true
            }
            Command::Left
            | Command::Right
            | Command::PreviousRoom
            | Command::NextRoom
            | Command::Up
            | Command::Down
            | Command::Faster
            | Command::Slower
            | Command::PhaseDelta(_) => {
                self.release_overture();
                true
            }
            Command::PrimaryDown
            | Command::PointerMoved { .. }
            | Command::PrimaryUp
            | Command::CancelPointer
            | Command::Pause
            | Command::Menu
            | Command::Back
            | Command::Inspect
            | Command::CycleRadio
            | Command::CycleEra
            | Command::ToggleMute
            | Command::VolumeDown
            | Command::VolumeUp => false,
        }
    }

    fn overture_clock_held(&self) -> bool {
        self.study.is_some()
            || self.paused
            || (self.show_help && self.menu.is_open())
            || !self.motion.animates()
            || self
                .overture
                .as_ref()
                .is_some_and(|play| play.reduced_beat.is_some())
    }

    fn overture_input_blocked(&self) -> bool {
        self.study.is_some() || (self.show_help && self.menu.is_open())
    }

    fn overture_step(&mut self, delta: isize) {
        let Some(current) = self.overture.as_ref().and_then(|play| play.reduced_beat) else {
            self.release_overture();
            return;
        };
        let last = OVERTURE.beats().len() - 1;
        if delta > 0 && current >= last {
            self.release_overture();
            return;
        }
        if delta < 0 && current == 0 {
            return;
        }
        let next = if delta > 0 { current + 1 } else { current - 1 };
        if let Some(play) = self.overture.as_mut() {
            play.reduced_beat = Some(next);
        }
        self.apply_current_overture_moment();
    }

    fn install_overture_rooms(&mut self) {
        for beat in OVERTURE.beats() {
            let Some(canonical) = numinous_core::room_by_id(beat.room_id()) else {
                continue;
            };
            if let Some(index) = self
                .rooms
                .iter()
                .position(|room| room.meta().id == beat.room_id())
            {
                self.rooms[index] = canonical;
            }
        }
    }

    fn apply_current_overture_moment(&mut self) {
        let Some(play) = self.overture.as_ref() else {
            return;
        };
        let moment = if let Some(beat) = play.reduced_beat {
            OVERTURE.reduced_moment(beat)
        } else {
            OVERTURE.moment_at(play.at_ms)
        };
        self.apply_overture_moment(moment);
    }

    fn apply_overture_moment(&mut self, moment: OvertureMoment) {
        if moment.handoff {
            self.route_lab_after_overture = true;
            self.menu.set_route_lab_door(true);
            if let Some(play) = self.overture.as_mut() {
                play.handoff = true;
            }
        }
        if moment.lead_in {
            if let Some(play) = self.overture.as_mut() {
                play.lead_in = true;
            }
            self.sync_room_parameter_voice();
            return;
        }
        self.present_overture_room(&moment);
        self.sync_room_parameter_voice();
    }

    /// Put the score's room on stage without counting a visit or rebuilding the bed.
    fn present_overture_room(&mut self, moment: &OvertureMoment) {
        let Some(room_id) = moment.room_id else {
            return;
        };
        let Some(index) = self.rooms.iter().position(|room| room.meta().id == room_id) else {
            return;
        };
        let same = self
            .overture
            .as_ref()
            .is_some_and(|play| play.presented_room == Some(index) && !play.lead_in);
        if !same {
            self.current = index;
            self.reset_room_runtime();
            if let Some(bed) = self.overture.as_ref().map(|play| Arc::clone(&play.bed)) {
                self.tune = bed;
            }
            self.dissolve.begin(self.motion);
            if let Some(play) = self.overture.as_mut() {
                play.presented_room = Some(index);
                play.lead_in = false;
            }
        } else if let Some(play) = self.overture.as_mut() {
            play.lead_in = false;
        }
        self.t = moment.phase;
    }
}

fn overture_bed() -> Arc<Vec<f32>> {
    let samples = OVERTURE
        .bed()
        .map(|motif| motif.arrangement().render_stereo(ROOM_BED_SOURCE_RATE))
        .unwrap_or_default();
    Arc::new(samples)
}

fn reduced_step_key(key: &Key) -> Option<isize> {
    match key {
        Key::Named(NamedKey::ArrowRight) => Some(1),
        Key::Named(NamedKey::ArrowLeft) => Some(-1),
        Key::Character(text) if text.as_str() == "d" => Some(1),
        Key::Character(text) if text.as_str() == "a" => Some(-1),
        _ => None,
    }
}

fn overture_room_action(key: &Key) -> bool {
    match key {
        Key::Named(
            NamedKey::ArrowLeft | NamedKey::ArrowRight | NamedKey::ArrowUp | NamedKey::ArrowDown,
        ) => true,
        Key::Character(text) => {
            let text = text.as_str();
            matches!(text, "a" | "d" | "w" | "s" | "u")
                || (text.len() == 1 && text.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use numinous_core::{Motion, OVERTURE, Raster};
    use winit::keyboard::{Key, NamedKey};

    use super::App;
    use crate::gamepad::Command;

    const SIZE: (usize, usize) = (160, 120);
    const FRAME: f64 = 1.0 / 60.0;

    fn started(name: &str, motion: Motion) -> App {
        let mut app = crate::tests::headless(name);
        app.close_menu();
        app.motion = motion;
        app.banner = None;
        app.room_card = 0;
        app.start_overture();
        app
    }

    fn at_ms(app: &App) -> u32 {
        app.overture.as_ref().expect("score").at_ms
    }

    fn jump(app: &mut App, ms: u32) {
        if let Some(play) = app.overture.as_mut() {
            play.at_ms = ms;
            play.elapsed = f64::from(ms) / 1000.0;
        }
        app.advance_overture(0.0);
    }

    fn presented(app: &mut App) -> Vec<u8> {
        let room = &app.rooms[app.current];
        let mut raster = Raster::with_accent(SIZE.0, SIZE.1, room.meta().accent);
        let lead_in = app.overture.as_ref().is_some_and(|play| play.lead_in);
        if !lead_in {
            room.render_input(&mut raster, app.t, &[]);
        }
        let (composed, width, height) = app.compose_frame(raster, SIZE.0, SIZE.1);
        app.dissolve.frame(composed, width, height).to_vec()
    }

    #[test]
    fn starting_the_score_clears_a_prior_celebration_but_keeps_save_warnings() {
        let mut app = started("overture-banner", Motion::Full);
        app.banner = Some(crate::feedback::level_up(2, 1));
        app.start_overture();
        assert!(app.banner.is_none());
        app.report_save_trouble(crate::SaveStore::Journey, "journey save", &"test refusal");
        assert!(app.save_warning_showing());
        app.start_overture();
        assert!(app.save_warning_showing());
    }

    #[test]
    fn a_delayed_tick_lands_once_on_the_score() {
        let mut app = started("overture-jump", Motion::Full);
        assert!(app.overture.as_ref().is_some_and(|play| play.lead_in));
        let before = app.journey.visited.clone();
        app.time_scale = 8.0;
        app.advance_room_tick(10.0, 10.0, false);
        let moment = OVERTURE.moment_at(10_000);
        assert_eq!(at_ms(&app), 10_000);
        assert_eq!(app.rooms[app.current].meta().id, moment.room_id.unwrap());
        assert_eq!(app.t, moment.phase);
        assert_eq!(app.phase_tick_elapsed, 0.0);
        assert_eq!(app.phase_spent, 0.0);
        assert_eq!(app.journey.visited, before);
        assert!(!app.route_lab_after_overture);
    }

    #[test]
    fn sixty_frames_sum_to_one_second_of_score() {
        let mut app = started("overture-frames", Motion::Full);
        for _ in 0..60 {
            app.advance_room_tick(FRAME, FRAME, false);
        }
        assert_eq!(at_ms(&app), 1_000);
    }

    #[test]
    fn beat_boundaries_match_the_score() {
        let mut app = started("overture-boundaries", Motion::Full);
        let samples = [
            0,
            5_000,
            5_000 + 8_000,
            OVERTURE.beat_start_ms(1).unwrap(),
            OVERTURE.beat_start_ms(2).unwrap(),
            72_000,
            OVERTURE.length_ms(),
        ];
        for ms in samples {
            jump(&mut app, ms);
            let moment = OVERTURE.moment_at(ms);
            assert_eq!(at_ms(&app), ms.min(OVERTURE.length_ms()));
            assert_eq!(
                app.overture
                    .as_ref()
                    .and_then(|play| play.lead_in.then_some(())),
                moment.lead_in.then_some(())
            );
            if let Some(room_id) = moment.room_id {
                assert_eq!(app.rooms[app.current].meta().id, room_id);
                assert_eq!(app.t, moment.phase);
            }
            assert_eq!(
                app.overture.as_ref().is_some_and(|play| play.handoff),
                moment.handoff
            );
        }
        assert!(app.route_lab_after_overture);
        assert!(app.menu.route_lab_door());
    }

    #[test]
    fn the_bed_is_one_allocation_across_a_beat_change() {
        let mut app = started("overture-bed", Motion::Full);
        let chaos_end = OVERTURE.beat_start_ms(0).unwrap() + OVERTURE.beats()[0].length_ms();
        jump(&mut app, chaos_end.saturating_sub(1));
        app.advance_presentation_time(10.0);
        let ptr = Arc::as_ptr(&app.tune);
        jump(&mut app, chaos_end);
        assert_eq!(app.rooms[app.current].meta().id, "mandelbrot");
        assert_eq!(Arc::as_ptr(&app.tune), ptr);
        assert!(Arc::ptr_eq(
            &app.tune,
            &app.overture.as_ref().expect("score").bed
        ));
    }

    #[test]
    fn the_handoff_voice_is_the_octave_and_not_the_four_lobe_goal() {
        let mut app = started("overture-octave", Motion::Full);
        jump(&mut app, 72_000);
        assert_eq!(app.rooms[app.current].meta().id, "times-tables");
        let voice = app
            .desired_room_parameter_sound()
            .expect("the held dial has a voice");
        assert!((f64::from(voice.ratio()) - 2.0).abs() < 1e-6);
        assert!(!app.rooms[app.current].goal_met(app.t, &[]));
        assert!(app.room_wager.is_none());
        assert!(!app.goal_announced);
    }

    #[test]
    fn pause_menu_and_study_hold_the_clock() {
        let mut app = started("overture-hold", Motion::Full);
        app.advance_room_tick(1.0, 1.0, false);
        assert_eq!(at_ms(&app), 1_000);
        app.toggle_pause();
        app.advance_room_tick(5.0, 5.0, false);
        assert_eq!(at_ms(&app), 1_000);
        app.toggle_pause();
        app.open_home_menu();
        app.advance_room_tick(5.0, 5.0, false);
        assert_eq!(at_ms(&app), 1_000);
        app.close_menu();
        app.advance_room_tick(5.0, 5.0, true);
        assert_eq!(at_ms(&app), 1_000);
        app.advance_room_tick(1.0, 1.0, false);
        assert_eq!(at_ms(&app), 2_000);
    }

    #[test]
    fn the_first_room_action_keeps_the_picture_at_every_beat() {
        for (index, beat) in OVERTURE.beats().iter().enumerate() {
            let mut app = started(&format!("overture-key-{index}"), Motion::Full);
            jump(&mut app, OVERTURE.beat_start_ms(index).unwrap());
            let room = beat.room_id();
            let phase = app.t;
            assert!(app.overture_key(&Key::Named(NamedKey::ArrowRight)));
            assert!(app.overture.is_none());
            assert_eq!(app.rooms[app.current].meta().id, room);
            assert_eq!(app.t, phase);

            let mut app = started(&format!("overture-pad-{index}"), Motion::Full);
            jump(&mut app, OVERTURE.beat_start_ms(index).unwrap());
            let phase = app.t;
            assert!(app.overture_gamepad(Command::NextRoom));
            assert!(app.overture.is_none());
            assert_eq!(app.rooms[app.current].meta().id, room);
            assert_eq!(app.t, phase);

            let mut app = started(&format!("overture-pointer-{index}"), Motion::Full);
            jump(&mut app, OVERTURE.beat_start_ms(index).unwrap());
            let phase = app.t;
            app.begin_pointer_at((0.5, 0.4));
            assert!(app.overture.is_none());
            assert_eq!(app.rooms[app.current].meta().id, room);
            assert_eq!(app.t, phase);
        }
        let mut app = started("overture-done", Motion::Full);
        jump(&mut app, OVERTURE.length_ms());
        assert!(app.route_lab_after_overture);
        let phase = app.t;
        assert!(app.overture_key(&Key::Character("d".into())));
        assert!(app.overture.is_none());
        assert_eq!(app.t, phase);
        assert!(app.route_lab_after_overture);
    }

    #[test]
    fn mandelbrot_takeover_keeps_the_rendered_view() {
        for variation in [0, 73] {
            let mut app = started(&format!("overture-camera-{variation}"), Motion::Full);
            app.set_variation_seed(variation);
            app.start_overture();
            let start = OVERTURE.beat_start_ms(1).unwrap();
            for offset in [0, 3_000, 10_000] {
                jump(&mut app, start + offset);
                let room = &app.rooms[app.current];
                let accent = room.meta().accent;
                let mut before = Raster::with_accent(SIZE.0, SIZE.1, accent);
                room.render_input(&mut before, app.t, &[]);
                let score_camera = app.overture_mandelbrot_camera().expect("score camera");
                let mut accelerated_source = Raster::with_accent(SIZE.0, SIZE.1, accent);
                score_camera.render(&mut accelerated_source);
                assert!(before.to_rgba() == accelerated_source.to_rgba());
                assert_eq!(
                    crate::live_mandelbrot_gpu_view(score_camera, 900, 700),
                    Some(crate::mandelbrot_gpu_view(app.t, 0, 900, 700, &[]))
                );
                assert_eq!(
                    crate::live_mandelbrot_gpu_view(score_camera, 2560, 1440),
                    Some(crate::mandelbrot_gpu_view(app.t, 0, 2560, 1440, &[]))
                );
                assert!(app.overture_key(&Key::Named(NamedKey::ArrowRight)));
                assert!(app.overture_mandelbrot_camera().is_none());
                let mut after = Raster::with_accent(SIZE.0, SIZE.1, accent);
                app.mandelbrot_camera.render(&mut after);
                assert!(
                    before.to_rgba() == after.to_rgba(),
                    "variation {variation}, offset {offset}"
                );
                app.mandelbrot_camera.advance(0.1);
                let continued = app.mandelbrot_camera.view();
                app.release_overture();
                assert_eq!(app.mandelbrot_camera.view(), continued);
                app.start_overture();
            }
        }
    }

    #[test]
    fn choosing_an_experiment_releases_the_score_without_resetting_the_dial() {
        for motion in [Motion::Full, Motion::Reduced] {
            for from_menu in [false, true] {
                let mut app = started(
                    &format!("overture-experiment-{motion:?}-{from_menu}"),
                    motion,
                );
                if motion.animates() {
                    jump(&mut app, OVERTURE.beat_start_ms(3).unwrap() + 1_000);
                } else {
                    for _ in 1..OVERTURE.beats().len() {
                        app.overture_gamepad(Command::NextRoom);
                    }
                }
                let phase = app.t;
                let journey = app.journey.clone();
                let bed = Arc::clone(&app.tune);
                let door = app.route_lab_after_overture;
                if from_menu {
                    app.open_home_menu();
                    app.apply_menu_intent(crate::menu::MenuIntent::ChooseExperiment);
                } else {
                    assert!(app.handle_chosen_experiment_key(&Key::Character("u".into()), false));
                    assert!(app.handle_chosen_experiment_key(&Key::Character("u".into()), true));
                }
                assert!(app.overture.is_none(), "the experiment owns the room");
                assert!(app.chosen_experiment_active());
                assert_eq!(app.rooms[app.current].meta().id, "times-tables");
                assert_eq!(app.t, phase);
                assert_eq!(app.journey, journey);
                assert_eq!(app.route_lab_after_overture, door);
                assert!(Arc::ptr_eq(&app.tune, &bed));
                app.advance_overture(30.0);
                assert_eq!(app.t, phase, "the score cannot take the dial back");
            }
        }
    }

    #[test]
    fn an_unavailable_experiment_does_not_release_the_score() {
        let mut app = started("overture-no-experiment", Motion::Full);
        jump(&mut app, OVERTURE.beat_start_ms(1).unwrap());
        assert!(!app.current_room_has_experiment());
        app.toggle_chosen_experiment();
        assert!(app.overture.is_some());
        assert!(!app.chosen_experiment_active());
    }

    #[test]
    fn pause_reset_and_the_show_key_do_not_steal_the_first_press() {
        let mut app = started("overture-pass", Motion::Full);
        jump(&mut app, 5_000);
        assert!(!app.overture_key(&Key::Named(NamedKey::Space)));
        assert!(app.overture.is_some());
        assert!(app.overture_key(&Key::Character("r".into())));
        assert!(app.overture.is_some());
        assert!(!app.overture_key(&Key::Character("b".into())));
        assert!(app.overture.is_some());
        assert!(!app.overture_gamepad(Command::Pause));
        assert!(app.overture.is_some());
        assert!(app.overture_gamepad(Command::Reset));
        assert!(app.overture.is_some());
        assert_eq!(app.rooms[app.current].meta().id, "chaos-game");
    }

    #[test]
    fn an_early_takeover_does_not_open_the_route_lab_door() {
        let mut app = started("overture-early", Motion::Full);
        jump(&mut app, OVERTURE.beat_start_ms(1).unwrap());
        assert!(!app.route_lab_after_overture);
        app.overture_key(&Key::Named(NamedKey::ArrowLeft));
        assert!(app.overture.is_none());
        assert!(!app.route_lab_after_overture);
        assert!(!app.menu.route_lab_door());
    }

    #[test]
    fn reduced_motion_steps_stills_and_the_last_step_releases() {
        let mut app = started("overture-reduced", Motion::Reduced);
        let opening = OVERTURE.reduced_moment(0);
        assert_eq!(app.rooms[app.current].meta().id, opening.room_id.unwrap());
        assert_eq!(app.t, opening.phase);
        app.advance_room_tick(30.0, 30.0, false);
        assert_eq!(app.t, opening.phase);
        assert!(app.overture_key(&Key::Named(NamedKey::ArrowLeft)));
        assert_eq!(app.t, opening.phase);
        assert!(app.overture.is_some());
        for index in 1..OVERTURE.beats().len() {
            assert!(app.overture_gamepad(Command::NextRoom));
            let still = OVERTURE.reduced_moment(index);
            assert_eq!(app.rooms[app.current].meta().id, still.room_id.unwrap());
            assert_eq!(app.t, still.phase);
        }
        assert!(app.route_lab_after_overture);
        assert!(app.overture.is_some());
        assert!(app.overture_key(&Key::Character("d".into())));
        assert!(app.overture.is_none());
        assert_eq!(
            app.rooms[app.current].meta().id,
            OVERTURE.beats().last().unwrap().room_id()
        );
        assert!(app.route_lab_after_overture);
    }

    #[test]
    fn replaying_closes_the_door_until_the_new_handoff() {
        let mut app = started("overture-replay", Motion::Full);
        jump(&mut app, OVERTURE.length_ms());
        assert!(app.route_lab_after_overture);
        app.start_overture();
        assert!(!app.route_lab_after_overture);
        assert!(!app.menu.route_lab_door());
        assert!(app.overture.as_ref().is_some_and(|play| play.lead_in));
    }

    #[test]
    fn the_three_dissolves_stay_inside_the_flash_budget() {
        for index in 0..3 {
            let mut app = started(&format!("overture-flash-{index}"), Motion::Full);
            let beat = &OVERTURE.beats()[index];
            let dissolve_at = OVERTURE.beat_start_ms(index).unwrap() + beat.length_ms();
            jump(&mut app, OVERTURE.beat_start_ms(index).unwrap());
            app.advance_presentation_time(10.0);
            jump(&mut app, dissolve_at.saturating_sub(1));
            let _ = presented(&mut app);
            jump(&mut app, dissolve_at);
            let mut series = Vec::with_capacity(36);
            for frame_index in 0..36 {
                if frame_index > 0 {
                    app.advance_room_tick(FRAME, FRAME, false);
                }
                series.push(numinous_core::photosensitivity::frame_luminance(
                    &presented(&mut app),
                ));
            }
            assert!(
                numinous_core::photosensitivity::within_budget(&series, 60.0),
                "dissolve {index} peak {}",
                numinous_core::photosensitivity::peak_flashes_per_second(&series, 60.0)
            );
        }
    }
}
