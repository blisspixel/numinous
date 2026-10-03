use super::{
    App, Key, NamedKey, controls, game_draw, gamepad, input_legend, menu, mouse_input, room_input,
    wager,
};
use crate::audio_runtime::VOLUME_STEP;

impl App {
    /// Route Lab owns only its construction keys during ordinary room play.
    pub(super) fn handle_route_lab_key(&mut self, key: &Key, repeat: bool) -> bool {
        if self.rooms[self.current].meta().id != "route-lab"
            || self.modal_mode_active()
            || self.menu.is_open()
            || self.show_help
            || self.show_journey
            || self.console.is_open()
            || self.the_show
            || self.paused
            || self.chosen_experiment
            || self.room_wager.is_some()
            || self.project_resume.is_some()
            || self.share_naming.is_some()
            || self.gallery.is_some()
        {
            return false;
        }
        let Key::Character(text) = key else {
            return false;
        };
        let mut characters = text.chars();
        let Some(ch) = characters.next().map(|ch| ch.to_ascii_lowercase()) else {
            return false;
        };
        if characters.next().is_some()
            || !matches!(
                ch,
                '1'..='6' | 'g' | 'i' | 'j' | 'l' | 'c' | 'z' | 't' | ',' | '.'
            )
        {
            return false;
        }
        if !repeat {
            self.clear_pointer_state();
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            self.compact_room_inputs();
            room_input::record_key(&mut self.inputs, ch);
            self.room_card = 0;
            self.maybe_announce_room_goal();
            self.sync_room_parameter_voice();
            self.play_room_interaction_audio(true);
        }
        true
    }

    pub(super) fn handle_global_audio_key(&mut self, key: &Key, repeat: bool) -> bool {
        let Key::Character(text) = key else {
            return false;
        };
        // A text field owns the whole printable range while it is open, or
        // the letters the global shortcuts claim cannot be typed: M made
        // MANDELBROT unspellable in a fractal instrument, and every press
        // flipped mute besides. The formula editor already carved out its
        // own minus and equals for the same reason; a name is free prose,
        // so it needs the carve-out entire.
        if self.share_naming.is_some() {
            return false;
        }
        if text.eq_ignore_ascii_case("m") {
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            if !repeat {
                self.toggle_mute();
            }
            return true;
        }
        let step = match text.as_str() {
            "[" => Some(-VOLUME_STEP),
            "]" => Some(VOLUME_STEP),
            "-" if !self.studio => Some(-VOLUME_STEP),
            "=" if !self.studio => Some(VOLUME_STEP),
            _ => None,
        };
        if let Some(step) = step {
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            self.change_volume(step);
            return true;
        }
        false
    }

    /// One step from the Muncher toward a clicked board cell.
    fn arcade_step_toward(from: usize, to: usize) -> Option<numinous_core::munch_arcade::Action> {
        let cols = numinous_core::munchers::COLS;
        let (fr, fc) = (from / cols, from % cols);
        let (tr, tc) = (to / cols, to % cols);
        if tr < fr {
            Some(numinous_core::munch_arcade::Action::Up)
        } else if tr > fr {
            Some(numinous_core::munch_arcade::Action::Down)
        } else if tc < fc {
            Some(numinous_core::munch_arcade::Action::Left)
        } else if tc > fc {
            Some(numinous_core::munch_arcade::Action::Right)
        } else {
            None
        }
    }

    /// A click lands in the games: cells, heaps, choices, and stages all answer.
    fn click(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        let (width, height) = (size.width as usize, size.height as usize);
        if width == 0 || height == 0 {
            return;
        }
        let (mx, my) = self.mouse;
        if let Some(play) = &mut self.munch {
            if play.graded.is_some() {
                return;
            }
            let feedback =
                if let Some(cell) = game_draw::MunchLayout::new(width, height).hit(mx, my) {
                    play.cursor = cell;
                    let was = play.bites.contains(&cell);
                    controls::toggle_munch_bite(&mut play.bites, cell);
                    play.flash_bite(cell);
                    let now = play.bites.contains(&cell);
                    Some((play.board.clone(), play.seed, cell, was, now))
                } else {
                    None
                };
            if let Some((board, seed, cell, was, now)) = feedback {
                self.munch_bite_feedback(&board, seed, cell, was, now);
            }
            return;
        }
        if let Some(quiz) = &self.quiz {
            if quiz.flash.is_some() {
                self.quiz_next();
                return;
            }
            let layout = game_draw::QuizChoiceLayout::new(width, height, quiz.round.choices.len());
            if let Some(index) = layout.hit(my, quiz.round.choices.len())
                && let Some(choice) = quiz.round.choices.get(index)
            {
                let letter = choice.letter;
                self.quiz_answer(letter);
            }
            return;
        }
        if self.nim.as_ref().is_some_and(|play| play.over.is_none()) {
            let heaps = self
                .nim
                .as_ref()
                .map(|play| play.heaps.clone())
                .unwrap_or_default();
            if let Some((heap, take)) = game_draw::NimLayout::new(width, height).hit(mx, my, &heaps)
            {
                if let Some(play) = self.nim.as_mut() {
                    play.selected = heap;
                    let max_take = play.heaps.get(heap).copied().unwrap_or(1).max(1);
                    play.take = take.max(1).min(max_take);
                }
                // A click that names both heap and stones commits the move.
                self.nim_move();
            }
            return;
        }
        if let Some(play) = &mut self.arcade {
            if play.over {
                return;
            }
            if let Some(cell) = game_draw::MunchLayout::new(width, height).hit(mx, my) {
                let muncher = play.run.muncher;
                if cell == muncher {
                    self.arcade_act(numinous_core::munch_arcade::Action::Eat);
                } else if let Some(action) = Self::arcade_step_toward(muncher, cell) {
                    self.arcade_act(action);
                }
            }
            return;
        }
        if let Some(run) = &self.gauntlet {
            match run.stage {
                0 => {
                    if run.munch.graded.is_some() {
                        return;
                    }
                    if let Some(cell) = game_draw::MunchLayout::new(width, height).hit(mx, my) {
                        if let Some(run) = self.gauntlet.as_mut() {
                            run.munch.cursor = cell;
                            controls::toggle_munch_bite(&mut run.munch.bites, cell);
                            run.munch.flash_bite(cell);
                        }
                        self.play_munch_crunch(cell as u64 ^ 0x6A17);
                    }
                }
                1 => {
                    if run.quiz.flash.is_some() {
                        return;
                    }
                    let choices = run.quiz.round.choices.len();
                    let layout = game_draw::QuizChoiceLayout::new(width, height, choices);
                    if let Some(index) = layout.hit(my, choices)
                        && let Some(letter) = self
                            .gauntlet
                            .as_ref()
                            .and_then(|g| g.quiz.round.choices.get(index).map(|c| c.letter))
                    {
                        self.gauntlet_key(&Key::Character(letter.to_string().into()));
                    }
                }
                _ => {}
            }
        }
    }

    fn set_mouse_from_normalized(&mut self, point: (f64, f64)) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        self.mouse = (
            point.0.clamp(0.0, 1.0) * f64::from(size.width),
            point.1.clamp(0.0, 1.0) * f64::from(size.height),
        );
    }

    pub(super) fn normalized_mouse_point(&self) -> Option<(f64, f64)> {
        self.window.as_ref().and_then(|window| {
            let size = window.inner_size();
            mouse_input::normalized_window_point(self.mouse, (size.width, size.height))
        })
    }

    pub(super) fn begin_pointer_at(&mut self, point: (f64, f64)) {
        if self.handle_study_pointer_down(point) {
            return;
        }
        if self.handle_gallery_pointer(point, true) {
            return;
        }
        if self.handle_route_pointer(point, true) {
            return;
        }
        if self.paused {
            return;
        }
        self.set_mouse_from_normalized(point);
        let action = mouse_input::left_press_action(self.left_press_context());
        self.set_pointer_state(mouse_input::pointer_state_after_left_press(action));
        match action {
            mouse_input::LeftPressAction::GameClick => self.click(),
            mouse_input::LeftPressAction::RoomPoke => {
                // Times Tables bottom band: commit the place wager without
                // turning the dial (y is ignored by the dial, but a clean
                // commit beat keeps the generation act distinct).
                if self.current_room_is_times_tables()
                    && self.chosen_experiment_active()
                    && point.1 >= numinous_core::rooms::times_tables_aha::WAGER_BAND_Y
                    && matches!(
                        self.times_tables_aha.beat(),
                        numinous_core::rooms::times_tables_aha::AhaBeat::Prime
                            | numinous_core::rooms::times_tables_aha::AhaBeat::Explore
                    )
                {
                    let place =
                        numinous_core::rooms::times_tables_aha::CardioidHome::from_unit_x(point.0);
                    if self.commit_times_tables_wager(place) {
                        self.poking = false;
                        return;
                    }
                }
                // Buffon bottom band: commit the number wager without a throw.
                if self.current_room_is_buffon()
                    && self.chosen_experiment_active()
                    && point.1 >= numinous_core::rooms::buffon_aha::WAGER_BAND_Y
                    && matches!(
                        self.buffon_aha.beat(),
                        numinous_core::rooms::buffon_aha::AhaBeat::Prime
                            | numinous_core::rooms::buffon_aha::AhaBeat::Explore
                    )
                {
                    let guess = numinous_core::rooms::buffon_aha::guess_from_unit_x(point.0);
                    if self.commit_buffon_wager(guess) {
                        self.poking = false;
                        return;
                    }
                }
                // Double Pendulum bottom band: call the twin's ending without
                // adding another release to the experiment.
                if self.current_room_is_pendulum()
                    && self.chosen_experiment_active()
                    && point.1 >= numinous_core::rooms::pendulum_aha::WAGER_BAND_Y
                    && matches!(
                        self.pendulum_aha.beat(),
                        numinous_core::rooms::pendulum_aha::AhaBeat::Prime
                    )
                {
                    let ending = numinous_core::rooms::pendulum_aha::Ending::from_unit_x(point.0);
                    if self.commit_pendulum_call(ending) {
                        self.poking = false;
                        return;
                    }
                }
                // Kepler bottom band: call the near-sun speed relation without
                // changing the ellipse underneath the commitment.
                if self.current_room_is_kepler()
                    && self.chosen_experiment_active()
                    && point.1 >= numinous_core::rooms::kepler_aha::WAGER_BAND_Y
                    && matches!(
                        self.kepler_aha.beat(),
                        numinous_core::rooms::kepler_aha::AhaBeat::Prime
                    )
                {
                    let relation =
                        numinous_core::rooms::kepler_aha::SpeedRelation::from_unit_x(point.0);
                    if self.commit_kepler_call(relation) {
                        self.poking = false;
                        return;
                    }
                }
                // Parrondo bottom band: call the winning policy without
                // changing the sampled walk underneath the commitment.
                if self.current_room_is_parrondo()
                    && self.chosen_experiment_active()
                    && point.1 >= numinous_core::rooms::parrondo_aha::WAGER_BAND_Y
                    && matches!(
                        self.parrondo_aha.beat(),
                        numinous_core::rooms::parrondo_aha::AhaBeat::Prime
                    )
                {
                    let policy = numinous_core::rooms::parrondo::Policy::from_unit_x(point.0);
                    if self.commit_parrondo_call(policy) {
                        self.poking = false;
                        return;
                    }
                }
                // Nontransitive Dice bottom band: call the counter without
                // choosing a different die underneath the commitment.
                if self.current_room_is_nontransitive()
                    && self.chosen_experiment_active()
                    && point.1 >= numinous_core::rooms::nontransitive_aha::WAGER_BAND_Y
                    && matches!(
                        self.nontransitive_aha.beat(),
                        numinous_core::rooms::nontransitive_aha::AhaBeat::Prime
                    )
                {
                    let die = numinous_core::rooms::nontransitive::Die::from_unit_x(point.0);
                    if self.commit_nontransitive_call(die) {
                        self.poking = false;
                        return;
                    }
                }
                // A posed call owns its band: a press there commits the
                // call instead of touching the room underneath.
                if !self.the_show
                    && self.room_wager.as_ref().is_some_and(wager::RoomWager::open)
                    && point.1 >= wager::WAGER_BAND_Y
                {
                    if let Some(posed) = self.room_wager.as_mut() {
                        posed.aim_at(point.0);
                    }
                    self.commit_room_wager();
                    self.poking = false;
                    return;
                }
                // Galton bottom band: commit the peak wager without a drop.
                if self.current_room_is_galton()
                    && self.chosen_experiment_active()
                    && point.1 >= numinous_core::rooms::galton_aha::WAGER_BAND_Y
                    && matches!(
                        self.galton_aha.beat(),
                        numinous_core::rooms::galton_aha::AhaBeat::Prime
                    )
                {
                    let bin = numinous_core::rooms::galton_board::bin_from_unit_x(point.0);
                    if self.commit_galton_wager(bin) {
                        self.poking = false;
                        return;
                    }
                }
                self.poking = true;
                self.record_room_touch(point);
                self.sync_times_tables_aha();
                self.sync_buffon_aha();
                self.sync_galton_aha();
                self.sync_pendulum_aha();
                self.sync_kepler_aha();
                self.sync_parrondo_aha();
                self.sync_nontransitive_aha();
                if self.rooms[self.current].meta().id == "mandelbrot"
                    && let Some(window) = &self.window
                {
                    let size = window.inner_size();
                    let _ = self.mandelbrot_camera.dive(
                        point.0,
                        point.1,
                        size.width as usize,
                        size.height as usize,
                    );
                }
            }
            mouse_input::LeftPressAction::PhaseDrag | mouse_input::LeftPressAction::Ignore => {}
        }
    }

    pub(super) fn move_pointer_to(&mut self, point: (f64, f64), held: bool) {
        if self.handle_study_pointer_move(point) {
            return;
        }
        if self.handle_gallery_pointer(point, false) {
            return;
        }
        if self.handle_route_pointer(point, false) {
            return;
        }
        if self.paused {
            return;
        }
        self.set_mouse_from_normalized(point);
        if self.current_room_is_times_tables()
            && self.chosen_experiment_active()
            && matches!(
                self.times_tables_aha.beat(),
                numinous_core::rooms::times_tables_aha::AhaBeat::Prime
            )
        {
            if point.1 >= numinous_core::rooms::times_tables_aha::WAGER_BAND_Y {
                self.times_tables_aha.set_hover(Some(
                    numinous_core::rooms::times_tables_aha::CardioidHome::from_unit_x(point.0),
                ));
            } else {
                self.times_tables_aha.set_hover(None);
            }
        }
        if self.current_room_is_buffon()
            && self.chosen_experiment_active()
            && matches!(
                self.buffon_aha.beat(),
                numinous_core::rooms::buffon_aha::AhaBeat::Prime
            )
        {
            if point.1 >= numinous_core::rooms::buffon_aha::WAGER_BAND_Y {
                self.buffon_aha.set_hover(Some(
                    numinous_core::rooms::buffon_aha::guess_from_unit_x(point.0),
                ));
            } else {
                self.buffon_aha.set_hover(None);
            }
        }
        if self.current_room_is_pendulum()
            && self.chosen_experiment_active()
            && matches!(
                self.pendulum_aha.beat(),
                numinous_core::rooms::pendulum_aha::AhaBeat::Prime
            )
        {
            if point.1 >= numinous_core::rooms::pendulum_aha::WAGER_BAND_Y {
                self.pendulum_aha.set_hover(Some(
                    numinous_core::rooms::pendulum_aha::Ending::from_unit_x(point.0),
                ));
            } else {
                self.pendulum_aha.set_hover(None);
            }
        }
        if self.current_room_is_kepler()
            && self.chosen_experiment_active()
            && matches!(
                self.kepler_aha.beat(),
                numinous_core::rooms::kepler_aha::AhaBeat::Prime
            )
        {
            if point.1 >= numinous_core::rooms::kepler_aha::WAGER_BAND_Y {
                self.kepler_aha.set_hover(Some(
                    numinous_core::rooms::kepler_aha::SpeedRelation::from_unit_x(point.0),
                ));
            } else {
                self.kepler_aha.set_hover(None);
            }
        }
        if self.current_room_is_parrondo()
            && self.chosen_experiment_active()
            && matches!(
                self.parrondo_aha.beat(),
                numinous_core::rooms::parrondo_aha::AhaBeat::Prime
            )
        {
            if point.1 >= numinous_core::rooms::parrondo_aha::WAGER_BAND_Y {
                self.parrondo_aha.set_hover(Some(
                    numinous_core::rooms::parrondo::Policy::from_unit_x(point.0),
                ));
            } else {
                self.parrondo_aha.set_hover(None);
            }
        }
        if self.current_room_is_nontransitive()
            && self.chosen_experiment_active()
            && matches!(
                self.nontransitive_aha.beat(),
                numinous_core::rooms::nontransitive_aha::AhaBeat::Prime
            )
        {
            if point.1 >= numinous_core::rooms::nontransitive_aha::WAGER_BAND_Y {
                self.nontransitive_aha.set_hover(Some(
                    numinous_core::rooms::nontransitive::Die::from_unit_x(point.0),
                ));
            } else {
                self.nontransitive_aha.set_hover(None);
            }
        }
        if !self.the_show
            && self.room_wager.as_ref().is_some_and(wager::RoomWager::open)
            && point.1 >= wager::WAGER_BAND_Y
            && let Some(posed) = self.room_wager.as_mut()
        {
            posed.aim_at(point.0);
        }
        if self.current_room_is_galton()
            && self.chosen_experiment_active()
            && matches!(
                self.galton_aha.beat(),
                numinous_core::rooms::galton_aha::AhaBeat::Prime
            )
        {
            if point.1 >= numinous_core::rooms::galton_aha::WAGER_BAND_Y {
                self.galton_aha.set_hover(Some(
                    numinous_core::rooms::galton_board::bin_from_unit_x(point.0),
                ));
            } else {
                self.galton_aha.set_hover(None);
            }
        }
        if held && self.poking && room_input::extend_poke_trail(&mut self.pokes, point) {
            self.compact_room_inputs();
            let accepted = room_input::record_pointer_move(&mut self.inputs, point, self.t);
            self.maybe_announce_room_goal();
            self.sync_times_tables_aha();
            self.sync_buffon_aha();
            self.sync_galton_aha();
            self.sync_pendulum_aha();
            self.sync_kepler_aha();
            self.sync_parrondo_aha();
            self.sync_nontransitive_aha();
            self.sync_room_parameter_voice();
            self.play_room_interaction_audio(accepted);
        }
    }

    pub(super) fn end_pointer_at(&mut self, point: (f64, f64)) {
        self.route_pointer_held = false;
        if self.handle_study_pointer_up(point) {
            return;
        }
        if self.route_active && !self.show_help {
            return;
        }
        self.set_mouse_from_normalized(point);
        let room = &self.rooms[self.current];
        let room_id = room.meta().id;
        let verb = room.verb().unwrap_or("");
        let mode = room_input::release_mode(room_id, verb);
        let was_dial_drag =
            self.poking && self.pokes.len() > 1 && mode == room_input::ReleaseMode::Dial;
        if self.poking {
            self.compact_room_inputs();
        }
        let accepted =
            self.poking && room_input::record_pointer_up(&mut self.inputs, point, self.t, mode);
        if was_dial_drag {
            // Dial drags leave no sticky trail; plant rooms keep a collapsed plant.
            self.pokes.clear();
        }
        self.set_pointer_state(mouse_input::pointer_state_after_left_release());
        self.maybe_announce_room_goal();
        self.sync_times_tables_aha();
        self.sync_buffon_aha();
        self.sync_galton_aha();
        self.sync_pendulum_aha();
        self.sync_kepler_aha();
        self.sync_parrondo_aha();
        self.sync_nontransitive_aha();
        self.sync_room_parameter_voice();
        self.play_room_interaction_audio(accepted);
    }

    pub(super) fn apply_wheel_delta(&mut self, lines: f64) -> bool {
        if self.handle_study_wheel(lines) {
            return true;
        }
        if self.route_active {
            return true;
        }
        if self.studio
            || self.paused
            || self.show_help && self.menu.is_open()
            || lines == 0.0
            || !lines.is_finite()
        {
            return false;
        }
        self.input_mode = input_legend::InputMode::KeyboardMouse;
        if self.current_room_is_life() {
            self.time_scale = if lines.is_sign_positive() {
                (self.time_scale * 2.0).min(numinous_core::MAX_TIME_SCALE)
            } else {
                (self.time_scale / 2.0).max(numinous_core::MIN_TIME_SCALE)
            };
            return true;
        }
        self.t = (self.t + lines * 0.02).rem_euclid(1.0);
        self.update_audio();
        true
    }

    fn gamepad_direction(&mut self, command: gamepad::Command) {
        if self.show_help {
            let layout = self.menu_layout();
            match command {
                gamepad::Command::Up => {
                    if layout.is_compact() {
                        self.menu.focus_next(-1);
                    } else {
                        self.menu.move_spatial(&layout, menu::Direction::Up);
                    }
                }
                gamepad::Command::Down => {
                    if layout.is_compact() {
                        self.menu.focus_next(1);
                    } else {
                        self.menu.move_spatial(&layout, menu::Direction::Down);
                    }
                }
                gamepad::Command::Left => {
                    if let Some(intent) = self.menu.adjust_focused(menu::Step::Down) {
                        self.apply_menu_intent(intent);
                    } else if layout.is_compact() {
                        self.menu.focus_next(-1);
                    } else {
                        self.menu.move_spatial(&layout, menu::Direction::Left);
                    }
                }
                gamepad::Command::Right => {
                    if let Some(intent) = self.menu.adjust_focused(menu::Step::Up) {
                        self.apply_menu_intent(intent);
                    } else if layout.is_compact() {
                        self.menu.focus_next(1);
                    } else {
                        self.menu.move_spatial(&layout, menu::Direction::Right);
                    }
                }
                _ => {}
            }
            return;
        }
        if self.studio || self.show_journey || self.the_show {
            return;
        }
        let key = match command {
            gamepad::Command::Up => Key::Named(NamedKey::ArrowUp),
            gamepad::Command::Down => Key::Named(NamedKey::ArrowDown),
            gamepad::Command::Left => Key::Named(NamedKey::ArrowLeft),
            gamepad::Command::Right => Key::Named(NamedKey::ArrowRight),
            _ => return,
        };
        if let Some(play) = &mut self.arcade {
            if let Some(action) = controls::arcade_action_for_key(&key)
                && !play.over
            {
                self.arcade_act(action);
            }
        } else if let Some(stage) = self.gauntlet.as_ref().map(|run| run.stage) {
            match stage {
                1 | 2 => {
                    let letter = match command {
                        gamepad::Command::Up => 'A',
                        gamepad::Command::Right => 'B',
                        gamepad::Command::Down => 'C',
                        gamepad::Command::Left => 'D',
                        _ => return,
                    };
                    self.gauntlet_key(&Key::Character(letter.to_string().into()));
                }
                3 => match command {
                    gamepad::Command::Up => {
                        self.controller_digit = (self.controller_digit + 1) % 10;
                        if let Some(run) = &mut self.gauntlet {
                            run.message = format!(
                                "SELECTED DIGIT {}. SOUTH ADDS, NORTH SUBMITS.",
                                self.controller_digit
                            );
                        }
                    }
                    gamepad::Command::Down => {
                        self.controller_digit = (self.controller_digit + 9) % 10;
                        if let Some(run) = &mut self.gauntlet {
                            run.message = format!(
                                "SELECTED DIGIT {}. SOUTH ADDS, NORTH SUBMITS.",
                                self.controller_digit
                            );
                        }
                    }
                    gamepad::Command::Left => {
                        self.gauntlet_key(&Key::Named(NamedKey::Backspace));
                    }
                    gamepad::Command::Right => self.gamepad_primary(),
                    _ => {}
                },
                _ => self.gauntlet_key(&key),
            }
        } else if self.munch.is_some() {
            self.munch_key(&key);
        } else if self.nim.is_some() {
            self.nim_key(&key);
        } else if self.quiz.is_some() {
            let letter = match command {
                gamepad::Command::Up => 'A',
                gamepad::Command::Right => 'B',
                gamepad::Command::Down => 'C',
                gamepad::Command::Left => 'D',
                _ => return,
            };
            self.quiz_answer(letter);
        } else {
            match command {
                gamepad::Command::Left => self.switch(-1),
                gamepad::Command::Right => self.switch(1),
                gamepad::Command::Up => {
                    self.time_scale = (self.time_scale * 2.0).min(numinous_core::MAX_TIME_SCALE)
                }
                gamepad::Command::Down => {
                    self.time_scale = (self.time_scale / 2.0).max(numinous_core::MIN_TIME_SCALE)
                }
                _ => {}
            }
        }
    }

    fn gamepad_primary(&mut self) {
        if self.show_help {
            self.activate_selected_menu_action();
        } else if let Some(over) = self.arcade.as_ref().map(|play| play.over) {
            if over {
                self.arcade = None;
                self.update_audio();
            } else {
                self.arcade_act(numinous_core::munch_arcade::Action::Eat);
            }
        } else if self.gauntlet.as_ref().is_some_and(|run| run.stage == 3) {
            self.gauntlet_key(&Key::Character(
                char::from(b'0' + self.controller_digit).to_string().into(),
            ));
        } else if self.gauntlet.is_some() {
            self.gauntlet_key(&Key::Named(NamedKey::Space));
        } else if self.munch.is_some() {
            self.munch_key(&Key::Named(NamedKey::Space));
        } else if self.nim.is_some() {
            self.nim_key(&Key::Named(NamedKey::Enter));
        } else if self.quiz.as_ref().is_some_and(|quiz| quiz.flash.is_some()) {
            self.quiz_next();
        } else if self.quiz.is_some() {
            self.quiz_answer('A');
        } else if self.can_advance_chosen_experiment() {
            self.experiment_primary_consumed = self.advance_chosen_experiment();
        } else if let Some(point) = self.gamepad.cursor() {
            self.begin_pointer_at(point);
        }
    }

    pub(super) fn activate_menu_choice(&mut self, choice: input_legend::MenuChoice) {
        self.close_menu();
        match choice {
            input_legend::MenuChoice::Quiz => self.quiz_next(),
            input_legend::MenuChoice::Munch => self.munch_start(),
            input_legend::MenuChoice::Nim => self.nim_start(),
            input_legend::MenuChoice::Gauntlet => self.gauntlet_start(),
            input_legend::MenuChoice::Arcade => self.arcade_start(),
            input_legend::MenuChoice::Show => self.toggle_show(),
            input_legend::MenuChoice::Studio => self.enter_studio(),
            input_legend::MenuChoice::Journey => self.toggle_journey(),
            input_legend::MenuChoice::WatchAgent => self.open_session_viewer(),
        }
    }

    fn gamepad_back(&mut self) {
        if self.show_help {
            self.menu_back();
        } else if self.the_show {
            self.toggle_show();
        } else if self.show_journey {
            self.show_journey = false;
        } else if self.chosen_experiment_active() {
            self.leave_chosen_experiment();
        } else if let Some(kind) = self.activity_kind() {
            self.open_activity_menu(kind);
        } else {
            self.open_home_menu();
        }
    }

    fn gamepad_menu(&mut self) {
        self.clear_pointer_state();
        if self.the_show {
            self.toggle_show();
        }
        self.show_journey = false;
        if self.show_help {
            self.close_menu();
        } else if let Some(kind) = self.activity_kind() {
            self.open_activity_menu(kind);
        } else {
            self.open_home_menu();
        }
    }

    fn gamepad_confirm_secondary(&mut self) {
        if self.arcade.is_some()
            || self.quiz.is_some()
            || self.studio
            || self.show_help && self.menu.is_open()
        {
            return;
        }
        if self.gauntlet.is_some() {
            self.gauntlet_key(&Key::Named(NamedKey::Enter));
        } else if self.munch.is_some() {
            self.munch_key(&Key::Named(NamedKey::Enter));
        } else if self.nim.is_some() {
            self.nim_key(&Key::Named(NamedKey::Enter));
        } else {
            self.cycle_radio();
        }
    }

    pub(super) fn handle_gamepad_command(&mut self, command: gamepad::Command) {
        if matches!(
            command,
            gamepad::Command::PrimaryUp | gamepad::Command::CancelPointer
        ) {
            self.route_primary_held = false;
        }
        match command {
            gamepad::Command::ToggleMute => {
                self.input_mode = input_legend::InputMode::Controller;
                self.toggle_mute();
                return;
            }
            gamepad::Command::VolumeDown => {
                self.input_mode = input_legend::InputMode::Controller;
                self.change_volume(-VOLUME_STEP);
                return;
            }
            gamepad::Command::VolumeUp => {
                self.input_mode = input_legend::InputMode::Controller;
                self.change_volume(VOLUME_STEP);
                return;
            }
            _ => {}
        }
        let experiment_release =
            command == gamepad::Command::PrimaryUp && self.experiment_primary_consumed;
        if experiment_release {
            self.experiment_primary_consumed = false;
        }
        if self.handle_study_gamepad(command) || experiment_release {
            return;
        }
        if self.experiment_primary_consumed
            && matches!(
                command,
                gamepad::Command::PrimaryDown | gamepad::Command::PointerMoved { .. }
            )
        {
            return;
        }
        if command == gamepad::Command::CancelPointer {
            self.experiment_primary_consumed = false;
        }
        if self.show_help {
            self.input_mode = input_legend::InputMode::Controller;
            match command {
                gamepad::Command::Up
                | gamepad::Command::Down
                | gamepad::Command::Left
                | gamepad::Command::Right => self.gamepad_direction(command),
                gamepad::Command::PrimaryDown => self.gamepad_primary(),
                gamepad::Command::CycleRadio => self.cycle_radio(),
                gamepad::Command::Back => self.gamepad_back(),
                gamepad::Command::Menu => self.gamepad_menu(),
                _ => {}
            }
            return;
        }
        if self.handle_gallery_gamepad(command) {
            return;
        }
        if self.handle_route_gamepad(command) {
            return;
        }
        if self.session_viewer.is_open() {
            if command != gamepad::Command::CancelPointer {
                self.input_mode = input_legend::InputMode::Controller;
            }
            match command {
                gamepad::Command::Back | gamepad::Command::Menu => {
                    self.open_activity_menu(menu::ActivityKind::SharedPlay);
                }
                gamepad::Command::Pause => self.session_viewer.toggle_display_pause(),
                gamepad::Command::Left => self.session_viewer.scrub(-1),
                gamepad::Command::Right => self.session_viewer.scrub(1),
                gamepad::Command::Up => self.session_viewer.scroll_result(-1),
                gamepad::Command::Down => self.session_viewer.scroll_result(1),
                gamepad::Command::PreviousRoom => self.session_viewer.pan_result(-4),
                gamepad::Command::NextRoom => self.session_viewer.pan_result(4),
                _ => {}
            }
            return;
        }
        if self.studio
            && matches!(
                command,
                gamepad::Command::Up | gamepad::Command::Down | gamepad::Command::Reset
            )
        {
            self.input_mode = input_legend::InputMode::Controller;
            let key = match command {
                gamepad::Command::Up => NamedKey::ArrowUp,
                gamepad::Command::Down => NamedKey::ArrowDown,
                _ => NamedKey::Home,
            };
            self.handle_studio_parameter_key(&Key::Named(key), false);
            return;
        }
        if self.paused
            && !(command == gamepad::Command::Back && self.chosen_experiment_active())
            && !(command == gamepad::Command::PrimaryDown && self.can_advance_chosen_experiment())
            && !matches!(
                command,
                gamepad::Command::Pause
                    | gamepad::Command::PrimaryUp
                    | gamepad::Command::CancelPointer
            )
        {
            return;
        }
        if self.show_help
            && self.modal_mode_active()
            && !matches!(
                command,
                gamepad::Command::PrimaryDown
                    | gamepad::Command::PrimaryUp
                    | gamepad::Command::Back
                    | gamepad::Command::Menu
                    | gamepad::Command::CancelPointer
            )
        {
            return;
        }
        if command != gamepad::Command::CancelPointer {
            self.input_mode = input_legend::InputMode::Controller;
        }
        match command {
            gamepad::Command::PrimaryDown => self.gamepad_primary(),
            gamepad::Command::PrimaryUp => {
                if let Some(point) = self.gamepad.cursor() {
                    self.end_pointer_at(point);
                }
            }
            gamepad::Command::Back => self.gamepad_back(),
            gamepad::Command::Menu => self.gamepad_menu(),
            gamepad::Command::Inspect => {}
            gamepad::Command::Reset => self.reset_current_room(),
            gamepad::Command::PreviousRoom if !self.modal_mode_active() => self.switch(-1),
            gamepad::Command::NextRoom if !self.modal_mode_active() => self.switch(1),
            gamepad::Command::Slower => {
                self.time_scale = (self.time_scale / 2.0).max(numinous_core::MIN_TIME_SCALE);
            }
            gamepad::Command::Faster => {
                self.time_scale = (self.time_scale * 2.0).min(numinous_core::MAX_TIME_SCALE);
            }
            gamepad::Command::Up
            | gamepad::Command::Down
            | gamepad::Command::Left
            | gamepad::Command::Right => self.gamepad_direction(command),
            gamepad::Command::CycleEra => self.cycle_visual_era(),
            gamepad::Command::CycleRadio => self.gamepad_confirm_secondary(),
            gamepad::Command::Pause => self.toggle_pause(),
            gamepad::Command::PointerMoved { point, held } => {
                self.move_pointer_to(point, held);
            }
            gamepad::Command::PhaseDelta(delta)
                if !self.modal_mode_active() && self.current_room_is_life() =>
            {
                self.time_scale = (self.time_scale * 2.0_f64.powf(delta * 4.0))
                    .clamp(numinous_core::MIN_TIME_SCALE, numinous_core::MAX_TIME_SCALE);
            }
            gamepad::Command::PhaseDelta(delta) if !self.modal_mode_active() => {
                self.t = (self.t + delta).rem_euclid(1.0);
                self.sync_room_parameter_voice();
            }
            gamepad::Command::CancelPointer => self.clear_pointer_state(),
            gamepad::Command::ToggleMute
            | gamepad::Command::VolumeDown
            | gamepad::Command::VolumeUp => {}
            gamepad::Command::PreviousRoom
            | gamepad::Command::NextRoom
            | gamepad::Command::PhaseDelta(_) => {}
        }
    }
}

#[cfg(test)]
mod route_lab_keyboard_tests {
    use super::{App, Key, NamedKey};
    use crate::input_legend::InputMode;
    use numinous_core::RoomInput;

    fn route_app(name: &str) -> App {
        let mut app = crate::tests::headless(name);
        app.close_menu();
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "route-lab")
            .expect("Route Lab in the real catalog");
        app.reset_current_room();
        app
    }

    fn status(app: &App) -> String {
        app.rooms[app.current]
            .status_input(app.t, &app.inputs)
            .expect("Route Lab status")
    }

    fn press(app: &mut App, text: &str) {
        assert!(app.handle_route_lab_key(&Key::Character(text.into()), false));
    }

    #[test]
    fn native_route_keys_select_improve_restore_greedy_and_edit_the_same_road() {
        let mut app = route_app("route-native-keys");
        let current = app.current;
        let bed = app.tune.clone();
        app.input_mode = InputMode::Controller;
        for ch in '1'..='6' {
            press(&mut app, &ch.to_string());
            assert_eq!(app.inputs.last(), Some(&RoomInput::Key { ch }));
            assert_eq!(app.current, current);
        }
        assert_eq!(app.input_mode, InputMode::KeyboardMouse);
        assert_eq!(app.room_card, 0);
        press(&mut app, "G");
        assert!(status(&app).contains("ORDER=ABCD"));
        assert_eq!(
            app.desired_room_parameter_sound()
                .expect("key voice")
                .ratio(),
            9.0 / 8.0
        );
        press(&mut app, "I");
        assert!(status(&app).contains("ORDER=ABDC"));
        assert!(app.rooms[current].goal_met(app.t, &app.inputs));
        assert_eq!(
            app.desired_room_parameter_sound()
                .expect("optimal voice")
                .ratio(),
            1.0
        );
        assert!(app.goal_announced);
        press(&mut app, "l");
        assert!(status(&app).contains("BD=4"));
        press(&mut app, "J");
        assert!(status(&app).contains("BD=3"));
        for _ in 0..12 {
            press(&mut app, "j");
        }
        assert!(status(&app).contains("BD=1"));
        for _ in 0..12 {
            press(&mut app, "L");
        }
        assert!(status(&app).contains("BD=9"));
        assert!(
            std::sync::Arc::ptr_eq(&bed, &app.tune),
            "keys preserve the room bed"
        );
        assert!(!app.show_journey, "J edits the road only in this room");
        assert!(app.inputs.len() <= numinous_core::MAX_ROOM_INPUTS);

        app.reset_current_room();
        assert!(app.inputs.is_empty());
        assert!(!app.goal_announced);
        assert!(status(&app).contains("BD=3"));
        assert!(app.desired_room_parameter_sound().is_none());
    }

    #[test]
    fn native_route_key_repeats_cannot_change_or_duplicate_the_experiment() {
        let mut app = route_app("route-native-repeat");
        press(&mut app, "l");
        let inputs = app.inputs.clone();
        let before = status(&app);
        for text in ["1", "6", "g", "i", "j", "l", "c", "z", "t", ",", "."] {
            assert!(app.handle_route_lab_key(&Key::Character(text.into()), true));
            assert_eq!(app.inputs, inputs);
            assert_eq!(status(&app), before);
        }
    }

    #[test]
    fn native_route_closure_undo_and_trace_survive_controller_moves_and_compaction() {
        let mut app = route_app("route-native-workbench");
        press(&mut app, "2");
        press(&mut app, "l");
        press(&mut app, "c");
        assert!(status(&app).contains("AB=closed"));
        app.begin_pointer_at((0.7, 0.67));
        assert!(status(&app).contains("SEARCH 0/"));
        let initial = status(&app);
        for step in 0..180 {
            let point = ((step % 5) as f64 * 0.2 + 0.1, 0.67);
            app.handle_gamepad_command(crate::gamepad::Command::PointerMoved { point, held: true });
            assert_eq!(
                status(&app),
                initial,
                "control-row move must not edit or step"
            );
            assert!(app.inputs.len() <= numinous_core::MAX_ROOM_INPUTS);
        }
        app.end_pointer_at((0.9, 0.67));
        assert_eq!(status(&app), initial);
        press(&mut app, ".");
        assert!(status(&app).contains("SEARCH 1/"));
        press(&mut app, ",");
        assert!(status(&app).contains("SEARCH 0/"));
        press(&mut app, "z");
        assert!(status(&app).contains("STEP TO START"));
        press(&mut app, "t");
        assert!(!status(&app).contains("closed"));
        assert!(status(&app).contains("BD=4"));
        press(&mut app, "z");
        assert!(status(&app).contains("ORDER=ABDC"));
        assert!(status(&app).contains("BD=3"));
        press(&mut app, "z");
        assert!(status(&app).contains("ORDER=ABCD"));
        assert_eq!(
            app.desired_room_parameter_sound().unwrap().ratio(),
            9.0 / 8.0
        );
        press(&mut app, "c");
        press(&mut app, ".");
        press(&mut app, "c");
        assert!(status(&app).contains("disconnected"));
        assert!(app.desired_room_parameter_sound().is_none());
        app.reset_current_room();
        assert!(app.inputs.is_empty());
        assert!(app.desired_room_parameter_sound().is_none());
        assert!(status(&app).starts_with("DRAG:  ORDER=ABCD cost=9"));
    }

    #[test]
    fn long_native_route_sessions_preserve_road_order_voice_and_goal_under_the_history_cap() {
        use numinous_core::Room;

        let mut app = route_app("route-native-long-session");
        let reference = numinous_core::rooms::route_lab::RouteLab::new();
        let check = |app: &App, road_x: f64, order: char| {
            let expected = [
                RoomInput::PointerDown {
                    x: road_x,
                    y: 0.75,
                    t: 0.0,
                },
                RoomInput::Key { ch: order },
            ];
            assert_eq!(
                app.rooms[app.current].status_input(app.t, &app.inputs),
                reference.status_input(0.0, &expected),
                "older road/order state must survive: {:?}",
                app.inputs
            );
            assert_eq!(
                app.rooms[app.current].goal_met(app.t, &app.inputs),
                reference.goal_met(0.0, &expected)
            );
            assert_eq!(
                app.desired_room_parameter_sound(),
                reference.parameter_sound(0.0, &expected)
            );
            assert!(app.inputs.len() <= numinous_core::MAX_ROOM_INPUTS);
        };
        app.begin_pointer_at((0.15, 0.75));
        app.end_pointer_at((0.15, 0.75));
        check(&app, 0.15, '1');
        for cycle in 0..8 {
            press(&mut app, "l");
            check(&app, 0.25, if cycle == 0 { '1' } else { '2' });
            press(&mut app, "j");
            check(&app, 0.15, if cycle == 0 { '1' } else { '2' });
            app.begin_pointer_at((0.05, 0.4));
            assert!(app.poking);
            check(&app, 0.15, '1');
            for step in 0..36 {
                let point = (if step % 2 == 0 { 0.2 } else { 0.3 }, 0.4);
                if step % 3 == 0 {
                    app.handle_gamepad_command(crate::gamepad::Command::PointerMoved {
                        point,
                        held: true,
                    });
                } else {
                    app.move_pointer_to(point, true);
                }
                check(&app, 0.15, '2');
            }
            if cycle % 2 == 0 {
                app.end_pointer_at((0.3, 0.4));
            } else {
                // A keyboard command cancels the held pointer through the
                // ordinary App path, which appends two cancellation events.
                press(&mut app, "2");
                assert!(!app.poking);
            }
            check(&app, 0.15, '2');
            press(&mut app, "j");
            check(&app, 0.0, '2');
            press(&mut app, "l");
            check(&app, 0.15, '2');
            press(&mut app, "g");
            check(&app, 0.15, '1');
            press(&mut app, "i");
            check(&app, 0.15, '2');
        }
        assert!(app.goal_announced);
        app.reset_current_room();
        assert!(app.inputs.is_empty());
        assert!(!app.goal_announced);
        assert!(app.desired_room_parameter_sound().is_none());
    }

    #[test]
    fn native_route_keys_yield_to_every_existing_modal_owner() {
        let owners: [fn(&mut App); 14] = [
            |app| app.open_home_menu(),
            |app| app.show_help = true,
            |app| app.show_journey = true,
            |app| app.console.open(),
            |app| app.the_show = true,
            |app| app.paused = true,
            |app| app.chosen_experiment = true,
            |app| app.enter_studio(),
            |app| app.quiz_next(),
            |app| app.munch_start(),
            |app| app.nim_start(),
            |app| app.gauntlet_start(),
            |app| app.arcade_start(),
            |app| {
                assert!(app.open_room_study());
            },
        ];
        for (index, owner) in owners.into_iter().enumerate() {
            let mut app = route_app(&format!("route-native-modal-{index}"));
            owner(&mut app);
            let inputs = app.inputs.clone();
            for text in ["1", "g", "i", "j", "l", "c", "z", "t", ",", "."] {
                assert!(!app.handle_route_lab_key(&Key::Character(text.into()), false));
                assert_eq!(app.inputs, inputs, "modal owner {index}");
            }
        }
    }

    #[test]
    fn native_route_keys_leave_navigation_and_global_commands_to_their_owners() {
        let mut app = route_app("route-native-global");
        for key in [
            Key::Named(NamedKey::ArrowLeft),
            Key::Named(NamedKey::ArrowRight),
            Key::Named(NamedKey::ArrowUp),
            Key::Named(NamedKey::ArrowDown),
            Key::Named(NamedKey::Escape),
            Key::Character("q".into()),
            Key::Character("m".into()),
            Key::Character("]".into()),
            Key::Character("e".into()),
            Key::Character("r".into()),
            Key::Character("n".into()),
            Key::Character("p".into()),
            Key::Character("k".into()),
        ] {
            assert!(!app.handle_route_lab_key(&key, false));
        }
        let current = app.current;
        app.switch(-1);
        assert_ne!(app.current, current);
        let inputs = app.inputs.clone();
        for text in ["1", "g", "i", "j", "l", "c", "z", "t", ",", "."] {
            assert!(!app.handle_route_lab_key(&Key::Character(text.into()), false));
            assert_eq!(app.inputs, inputs);
        }
    }
}
