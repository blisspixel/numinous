use super::{
    App, Key, NamedKey, controls, feedback, game_draw, gamepad, input_legend, menu, mouse_input,
    room_input, wager,
};

impl App {
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
            "[" => Some(-0.1),
            "]" => Some(0.1),
            "-" if !self.studio => Some(-0.1),
            "=" if !self.studio => Some(0.1),
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
                    && !self.the_show
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
                    && !self.the_show
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
                    && !self.the_show
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
                    && !self.the_show
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
                    && !self.the_show
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
                    && !self.the_show
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
                    && !self.the_show
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
        if self.paused {
            return;
        }
        self.set_mouse_from_normalized(point);
        if self.current_room_is_times_tables()
            && !self.the_show
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
            && !self.the_show
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
            && !self.the_show
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
            && !self.the_show
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
            && !self.the_show
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
            && !self.the_show
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
            && !self.the_show
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
        self.set_mouse_from_normalized(point);
        let room = &self.rooms[self.current];
        let room_id = room.meta().id;
        let verb = room.verb().unwrap_or("");
        let mode = room_input::release_mode(room_id, verb);
        let was_dial_drag =
            self.poking && self.pokes.len() > 1 && mode == room_input::ReleaseMode::Dial;
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
                (self.time_scale * 2.0).min(8.0)
            } else {
                (self.time_scale / 2.0).max(0.25)
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
                    if let Some(intent) = self.menu.adjust_focused(-10) {
                        self.apply_menu_intent(intent);
                    } else if layout.is_compact() {
                        self.menu.focus_next(-1);
                    } else {
                        self.menu.move_spatial(&layout, menu::Direction::Left);
                    }
                }
                gamepad::Command::Right => {
                    if let Some(intent) = self.menu.adjust_focused(10) {
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
                gamepad::Command::Up => self.time_scale = (self.time_scale * 2.0).min(8.0),
                gamepad::Command::Down => self.time_scale = (self.time_scale / 2.0).max(0.25),
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

    pub(super) fn cycle_radio(&mut self) {
        let stations = numinous_core::STATIONS.len();
        self.radio = match self.radio {
            None => Some(0),
            Some(i) if i + 1 < stations => Some(i + 1),
            Some(_) => None,
        };
        self.tune_in();
    }

    pub(super) fn skip_radio_track(&mut self) {
        self.clear_pointer_state();
        let Some(station) = self.radio else {
            self.banner = Some(feedback::radio_skip_needs_station());
            return;
        };
        let station_name = numinous_core::STATIONS[station].name;
        let track_count = self.radio_paths.len();
        if track_count == 0 {
            self.banner = Some(feedback::radio_skip_unavailable(station_name));
            return;
        }
        self.radio_index = (self.radio_index + 1) % track_count;
        if self.radio_play_or_advance(0.0) {
            self.banner = Some(feedback::radio_skip(
                station_name,
                self.radio_index + 1,
                track_count,
            ));
        } else {
            self.update_audio();
            self.banner = Some(feedback::radio_skip_unavailable(station_name));
        }
    }

    pub(super) fn handle_gamepad_command(&mut self, command: gamepad::Command) {
        match command {
            gamepad::Command::ToggleMute => {
                self.input_mode = input_legend::InputMode::Controller;
                self.toggle_mute();
                return;
            }
            gamepad::Command::VolumeDown => {
                self.input_mode = input_legend::InputMode::Controller;
                self.change_volume(-0.1);
                return;
            }
            gamepad::Command::VolumeUp => {
                self.input_mode = input_legend::InputMode::Controller;
                self.change_volume(0.1);
                return;
            }
            _ => {}
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
        if self.paused
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
            gamepad::Command::Inspect => self.toggle_inspect(),
            gamepad::Command::Reset => self.reset_current_room(),
            gamepad::Command::PreviousRoom if !self.modal_mode_active() => self.switch(-1),
            gamepad::Command::NextRoom if !self.modal_mode_active() => self.switch(1),
            gamepad::Command::Slower => {
                self.time_scale = (self.time_scale / 2.0).max(0.25);
            }
            gamepad::Command::Faster => {
                self.time_scale = (self.time_scale * 2.0).min(8.0);
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
                self.time_scale = (self.time_scale * 2.0_f64.powf(delta * 4.0)).clamp(0.25, 8.0);
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
