//! Face-level live room coordination.

use numinous_core::Room;
use winit::keyboard::{Key, NamedKey};

use super::{
    App, BUFFON_MORPH_SECONDS, GALTON_MORPH_SECONDS, KEPLER_MORPH_SECONDS, LIFE_STEP_SECONDS,
    MAX_LIFE_STEPS_PER_TICK, NONTRANSITIVE_MORPH_SECONDS, PARRONDO_MORPH_SECONDS,
    PENDULUM_MORPH_SECONDS, TIMES_TABLES_MORPH_SECONDS, controls, effective_room_phase, feedback,
    has_finite_parameter_input, input_legend, room_input, wager,
};

impl App {
    pub(super) fn reset_room_runtime(&mut self) {
        self.clear_pointer_state();
        self.chosen_experiment = false;
        self.experiment_primary_consumed = false;
        self.menu
            .set_experiment_available(self.current_room_has_experiment() && !self.the_show);
        if self.goal_announced {
            self.banner = None;
        }
        room_input::reset_room_view(
            &mut self.t,
            &mut self.room_card,
            &mut self.pokes,
            &mut self.inputs,
        );
        self.mandelbrot_camera.reset(self.variation);
        self.reset_life_session();
        self.reset_times_tables_aha();
        self.reset_buffon_aha();
        self.reset_galton_aha();
        self.reset_pendulum_aha();
        self.reset_kepler_aha();
        self.reset_parrondo_aha();
        self.reset_nontransitive_aha();
        // A call is about one room's readout; carrying it across the
        // doorway would grade the wrong number.
        self.room_wager = None;
        self.goal_announced = false;
    }

    pub(super) fn reset_current_room(&mut self) {
        self.reset_room_runtime();
        self.spectrum_cache = None;
        self.update_audio();
    }

    pub(super) fn current_room_is_life(&self) -> bool {
        self.rooms[self.current].meta().id == "game-of-life"
    }

    pub(super) fn current_room_is_times_tables(&self) -> bool {
        self.rooms[self.current].meta().id == "times-tables"
    }

    pub(super) fn current_room_is_galton(&self) -> bool {
        self.rooms[self.current].meta().id == "galton-board"
    }

    pub(super) fn current_room_is_buffon(&self) -> bool {
        self.rooms[self.current].meta().id == "buffon-needle"
    }

    pub(super) fn current_room_is_pendulum(&self) -> bool {
        self.rooms[self.current].meta().id == "double-pendulum"
    }

    pub(super) fn current_room_is_kepler(&self) -> bool {
        self.rooms[self.current].meta().id == "kepler-laws"
    }

    pub(super) fn current_room_is_parrondo(&self) -> bool {
        self.rooms[self.current].meta().id == "parrondo"
    }

    pub(super) fn current_room_is_nontransitive(&self) -> bool {
        self.rooms[self.current].meta().id == "nontransitive"
    }

    pub(super) fn current_room_has_experiment(&self) -> bool {
        numinous_core::is_engineered_aha_room(self.rooms[self.current].meta().id)
    }

    pub(super) fn chosen_experiment_active(&self) -> bool {
        self.chosen_experiment
            && self.current_room_has_experiment()
            && !self.the_show
            && !self.modal_mode_active()
            && !self.console.is_open()
            && !self.show_journey
    }

    /// Choose or leave a staged path without changing the accepted room history.
    pub(super) fn toggle_chosen_experiment(&mut self) {
        if !self.current_room_has_experiment()
            || self.the_show
            || self.modal_mode_active()
            || self.console.is_open()
            || self.show_journey
        {
            return;
        }
        if self.leave_chosen_experiment() {
            return;
        }
        self.chosen_experiment = true;
        // Existing observations may prime or earn a connection. Selection alone
        // supplies no wager, observation, or consolidation.
        self.sync_times_tables_aha();
        self.sync_buffon_aha();
        self.sync_galton_aha();
        self.sync_pendulum_aha();
        self.sync_kepler_aha();
        self.sync_parrondo_aha();
        self.sync_nontransitive_aha();
    }

    /// Return to free play while retaining calls and earned experiment progress.
    pub(super) fn leave_chosen_experiment(&mut self) -> bool {
        std::mem::take(&mut self.chosen_experiment)
    }

    /// Route path selection before ordinary key handling can cancel a gesture.
    pub(super) fn handle_chosen_experiment_key(&mut self, key: &Key, repeat: bool) -> bool {
        if !self.current_room_has_experiment()
            || self.the_show
            || self.show_help
            || self.modal_mode_active()
            || self.console.is_open()
            || self.show_journey
        {
            return false;
        }
        match controls::normalized_command_key(key) {
            Key::Character(text) if text.as_str() == "u" => {
                if !repeat {
                    self.toggle_chosen_experiment();
                }
                true
            }
            Key::Named(NamedKey::Escape) if self.chosen_experiment => {
                if !repeat {
                    self.leave_chosen_experiment();
                }
                true
            }
            Key::Named(NamedKey::Enter) if self.chosen_experiment => {
                if !repeat {
                    self.advance_chosen_experiment();
                }
                true
            }
            _ => false,
        }
    }

    /// Keep core status contracts intact while naming the App's actual action.
    pub(super) fn chosen_experiment_status(&self, status: String) -> String {
        if !self.can_advance_chosen_experiment() {
            return status;
        }
        let primary = match self.input_mode {
            input_legend::InputMode::KeyboardMouse => "ENTER".to_string(),
            input_legend::InputMode::Controller => self
                .gamepad
                .controller_copy()
                .action_token(input_legend::ControllerAction::Primary),
        };
        status
            .replace("PRESS E", &format!("PRESS {primary}"))
            .replace("BOTH E ", &format!("BOTH {primary} "))
            .replace("PI HIDES E ", &format!("PI HIDES {primary} "))
    }

    pub(super) fn current_status_override(&self, width: usize) -> Option<String> {
        if self.current_room_is_life() {
            return Some(if width <= 400 {
                self.life_session.compact_status()
            } else {
                self.life_session.status()
            });
        }
        if self.current_room_is_times_tables() && self.chosen_experiment_active() {
            let phase = effective_room_phase("times-tables", self.t, &self.inputs, self.the_show);
            let dial = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(
                self.chosen_experiment_status(self.times_tables_aha.status(dial.as_deref())),
            );
        }
        if self.current_room_is_buffon() && self.chosen_experiment_active() {
            let phase = effective_room_phase("buffon-needle", self.t, &self.inputs, self.the_show);
            let throws = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.chosen_experiment_status(self.buffon_aha.status(throws.as_deref())));
        }
        if self.current_room_is_pendulum() && self.chosen_experiment_active() {
            let phase =
                effective_room_phase("double-pendulum", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(
                self.chosen_experiment_status(self.pendulum_aha.status(readout.as_deref())),
            );
        }
        if self.current_room_is_kepler() && self.chosen_experiment_active() {
            let phase = effective_room_phase("kepler-laws", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.chosen_experiment_status(self.kepler_aha.status(readout.as_deref())));
        }
        if self.current_room_is_parrondo() && self.chosen_experiment_active() {
            let phase = effective_room_phase("parrondo", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(
                self.chosen_experiment_status(self.parrondo_aha.status(readout.as_deref())),
            );
        }
        if self.current_room_is_nontransitive() && self.chosen_experiment_active() {
            let phase = effective_room_phase("nontransitive", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(
                self.chosen_experiment_status(self.nontransitive_aha.status(readout.as_deref())),
            );
        }
        if let Some(posed) = &self.room_wager {
            return Some(posed.status());
        }
        if self.current_room_is_galton() && self.chosen_experiment_active() {
            let phase = effective_room_phase("galton-board", self.t, &self.inputs, self.the_show);
            let pile = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.chosen_experiment_status(self.galton_aha.status(pile.as_deref())));
        }
        None
    }

    /// Draw the selected experiment; false leaves the ordinary room surface intact.
    pub(super) fn draw_chosen_experiment(&self, surface: &mut dyn numinous_core::Surface) -> bool {
        if !self.chosen_experiment_active() {
            return false;
        }
        let room = &self.rooms[self.current];
        let room_inputs = &self.inputs;
        if room.meta().id == "times-tables" && self.times_tables_aha.uses_aha_plate() {
            let phase = effective_room_phase(room.meta().id, self.t, &self.inputs, self.the_show);
            let k = numinous_core::rooms::times_tables::TimesTables::new_with(self.variation)
                .live_multiplier(phase, room_inputs);
            numinous_core::rooms::times_tables_aha::render_aha_plate(
                surface,
                self.times_tables_aha.beat(),
                k,
            );
        } else {
            let phase = effective_room_phase(room.meta().id, self.t, &self.inputs, self.the_show);
            room.render_input(surface, phase, room_inputs);
            if room.meta().id == "times-tables"
                && matches!(
                    self.times_tables_aha.beat(),
                    numinous_core::rooms::times_tables_aha::AhaBeat::Prime
                )
            {
                numinous_core::rooms::times_tables_aha::render_wager_options(
                    surface,
                    self.times_tables_aha.hover(),
                );
            }
            if room.meta().id == "buffon-needle" {
                if matches!(
                    self.buffon_aha.beat(),
                    numinous_core::rooms::buffon_aha::AhaBeat::Prime
                ) {
                    numinous_core::rooms::buffon_aha::render_guess_band(
                        surface,
                        self.buffon_aha.hover(),
                    );
                }
                if self.buffon_aha.uses_circle_overlay() {
                    let progress = match self.buffon_aha.beat() {
                        numinous_core::rooms::buffon_aha::AhaBeat::Morph { progress } => progress,
                        _ => 1.0,
                    };
                    numinous_core::rooms::buffon_aha::render_circle_overlay(surface, progress);
                }
            }
            if room.meta().id == "double-pendulum" {
                if matches!(
                    self.pendulum_aha.beat(),
                    numinous_core::rooms::pendulum_aha::AhaBeat::Prime
                ) {
                    numinous_core::rooms::pendulum_aha::render_ending_band(
                        surface,
                        self.pendulum_aha.hover(),
                    );
                }
                if self.pendulum_aha.uses_curve_overlay() {
                    let progress = match self.pendulum_aha.beat() {
                        numinous_core::rooms::pendulum_aha::AhaBeat::Morph { progress } => progress,
                        _ => 1.0,
                    };
                    numinous_core::rooms::pendulum_aha::render_gap_curve_for_inputs(
                        surface,
                        progress,
                        self.variation,
                        &self.inputs,
                    );
                }
            }
            if room.meta().id == "kepler-laws" {
                if matches!(
                    self.kepler_aha.beat(),
                    numinous_core::rooms::kepler_aha::AhaBeat::Prime
                ) {
                    numinous_core::rooms::kepler_aha::render_speed_band(
                        surface,
                        self.kepler_aha.hover(),
                    );
                }
                if self.kepler_aha.uses_time_overlay() {
                    let progress = match self.kepler_aha.beat() {
                        numinous_core::rooms::kepler_aha::AhaBeat::Morph { progress } => progress,
                        _ => 1.0,
                    };
                    numinous_core::rooms::kepler_aha::render_equal_time_overlay(
                        surface,
                        progress,
                        self.kepler_aha.eccentricity(),
                    );
                }
            }
            if room.meta().id == "parrondo" {
                if matches!(
                    self.parrondo_aha.beat(),
                    numinous_core::rooms::parrondo_aha::AhaBeat::Prime
                ) {
                    numinous_core::rooms::parrondo_aha::render_policy_band(
                        surface,
                        self.parrondo_aha.hover(),
                    );
                }
                if self.parrondo_aha.uses_expectation_overlay() {
                    let progress = match self.parrondo_aha.beat() {
                        numinous_core::rooms::parrondo_aha::AhaBeat::Morph { progress } => progress,
                        _ => 1.0,
                    };
                    numinous_core::rooms::parrondo_aha::render_expectation_overlay(
                        surface, progress,
                    );
                }
            }
            if room.meta().id == "nontransitive" {
                if matches!(
                    self.nontransitive_aha.beat(),
                    numinous_core::rooms::nontransitive_aha::AhaBeat::Prime
                ) {
                    numinous_core::rooms::nontransitive_aha::render_counter_band(
                        surface,
                        self.nontransitive_aha.hover(),
                    );
                }
                if self.nontransitive_aha.uses_outcome_grid()
                    && let Some(chosen) = self.nontransitive_aha.chosen()
                {
                    let progress = match self.nontransitive_aha.beat() {
                        numinous_core::rooms::nontransitive_aha::AhaBeat::Morph { progress } => {
                            progress
                        }
                        _ => 1.0,
                    };
                    numinous_core::rooms::nontransitive_aha::render_outcome_grid(
                        surface, progress, chosen,
                    );
                }
            }
            if room.meta().id == "galton-board" {
                if matches!(
                    self.galton_aha.beat(),
                    numinous_core::rooms::galton_aha::AhaBeat::Prime
                ) {
                    numinous_core::rooms::galton_aha::render_bin_band(
                        surface,
                        self.galton_aha.hover(),
                    );
                }
                // The curve answers the call, so it is the called
                // coin's curve, and it is drawn only while the pile
                // underneath is that same experiment. A player who
                // wanders to another coin gets no curve over the wrong
                // pile; the footer says which pile the call was about,
                // and the curve returns when they do.
                let live_coin =
                    numinous_core::rooms::galton_board::selected_coin_from_inputs(&self.inputs)
                        .unwrap_or(2);
                if self.galton_aha.uses_outline_overlay() && self.galton_aha.answers_pile(live_coin)
                {
                    let progress = match self.galton_aha.beat() {
                        numinous_core::rooms::galton_aha::AhaBeat::Morph { progress } => progress,
                        _ => 1.0,
                    };
                    let coin = self.galton_aha.coin().unwrap_or(live_coin);
                    numinous_core::rooms::galton_aha::render_outline_overlay(
                        surface, progress, coin,
                    );
                }
            }
        }
        true
    }

    pub(super) fn reset_life_session(&mut self) {
        self.life_session = numinous_core::rooms::game_of_life::LifeSession::new(self.variation);
        self.life_accumulator = 0.0;
        self.clear_transient_audio();
    }

    pub(super) fn reset_times_tables_aha(&mut self) {
        self.times_tables_aha = numinous_core::rooms::times_tables_aha::TimesTablesAha::new();
        if self.current_room_is_times_tables() {
            self.show_info = false;
        }
    }

    pub(super) fn reset_buffon_aha(&mut self) {
        self.buffon_aha = numinous_core::rooms::buffon_aha::BuffonAha::new();
        if self.current_room_is_buffon() {
            self.show_info = false;
        }
    }

    pub(super) fn reset_pendulum_aha(&mut self) {
        self.pendulum_aha = numinous_core::rooms::pendulum_aha::PendulumAha::new(self.variation);
        if self.current_room_is_pendulum() {
            self.show_info = false;
        }
    }

    pub(super) fn reset_kepler_aha(&mut self) {
        let eccentricity = numinous_core::rooms::kepler_laws::eccentricity_for_inputs(
            self.t,
            &self.inputs,
            self.variation,
        );
        self.kepler_aha = numinous_core::rooms::kepler_aha::KeplerAha::new(eccentricity);
        if self.current_room_is_kepler() {
            self.show_info = false;
        }
    }

    pub(super) fn reset_parrondo_aha(&mut self) {
        self.parrondo_aha = numinous_core::rooms::parrondo_aha::ParrondoAha::new();
        if self.current_room_is_parrondo() {
            self.show_info = false;
        }
    }

    pub(super) fn reset_nontransitive_aha(&mut self) {
        self.nontransitive_aha = numinous_core::rooms::nontransitive_aha::NontransitiveAha::new();
        if self.current_room_is_nontransitive() {
            self.show_info = false;
        }
    }

    /// U chooses a staged experiment or toggles an ordinary room's readout wager.
    pub(super) fn toggle_room_wager(&mut self) {
        if self.the_show || self.studio || self.arcade.is_some() {
            return;
        }
        if self.room_wager.take().is_some() {
            return;
        }
        if self.current_room_has_experiment() {
            self.toggle_chosen_experiment();
            return;
        }
        let room = self.rooms[self.current].as_ref();
        match wager::RoomWager::pose(room, self.variation) {
            Some(posed) => {
                self.show_info = false;
                self.room_wager = Some(posed);
            }
            None => {
                self.banner = Some(feedback::Banner::status(
                    "THIS ROOM READS NO NUMBER TO CALL",
                    feedback::REFUSAL_FRAMES,
                ));
            }
        }
    }

    /// Commit the posed call and meet the truth.
    pub(super) fn commit_room_wager(&mut self) {
        let Some(mut posed) = self.room_wager.take() else {
            return;
        };
        let room = self.rooms[self.current].as_ref();
        if posed.commit(room).is_some()
            && let Some(verdict) = posed.verdict()
        {
            self.banner = Some(feedback::Banner::status(
                verdict.to_uppercase(),
                feedback::REFUSAL_FRAMES,
            ));
        }
        self.room_wager = Some(posed);
    }

    pub(super) fn reset_galton_aha(&mut self) {
        self.galton_aha = numinous_core::rooms::galton_aha::GaltonAha::new();
        if self.current_room_is_galton() {
            self.show_info = false;
        }
    }

    /// Keep the Times Tables aha in step with hand dial and the four-lobe goal.
    pub(super) fn sync_times_tables_aha(&mut self) {
        if !self.current_room_is_times_tables() || !self.chosen_experiment_active() {
            return;
        }
        let phase = effective_room_phase("times-tables", self.t, &self.inputs, false);
        let room = numinous_core::rooms::times_tables::TimesTables::new_with(self.variation);
        if has_finite_parameter_input(&self.inputs) {
            let k = room.live_multiplier(phase, &self.inputs);
            self.times_tables_aha.note_hand_multiplier(k);
        }
        if room.goal_met(phase, &self.inputs) {
            let _ = self.times_tables_aha.note_four_lobes();
        }
    }

    /// Keep the Buffon aha in step with player throws.
    pub(super) fn sync_buffon_aha(&mut self) {
        if !self.current_room_is_buffon() || !self.chosen_experiment_active() {
            return;
        }
        let throws = numinous_core::rooms::buffon_needle::BuffonNeedle::throw_count(&self.inputs);
        self.buffon_aha.note_throws(throws);
    }

    /// Keep the Galton aha in step with the waves the pile is built from.
    pub(super) fn sync_galton_aha(&mut self) {
        if !self.current_room_is_galton() || !self.chosen_experiment_active() {
            return;
        }
        let waves = numinous_core::rooms::galton_board::wave_count_from_inputs(&self.inputs);
        let coin = numinous_core::rooms::galton_board::selected_coin_from_inputs(&self.inputs)
            .unwrap_or(2);
        self.galton_aha.note_waves(waves, coin);
    }

    /// Keep the Double Pendulum aha in step with completed releases.
    pub(super) fn sync_pendulum_aha(&mut self) {
        if !self.current_room_is_pendulum() || !self.chosen_experiment_active() {
            return;
        }
        let room = numinous_core::rooms::double_pendulum::DoublePendulum::new_with(self.variation);
        if let Some(gap) = room.divergence_at_full_sweep_for_inputs(&self.inputs) {
            let _ = self.pendulum_aha.bind_truth_gap(gap);
        }
        let drops = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        self.pendulum_aha.note_drops(drops);
    }

    /// Keep the Kepler aha bound to the ellipse chosen by completed drags.
    pub(super) fn sync_kepler_aha(&mut self) {
        if !self.current_room_is_kepler() || !self.chosen_experiment_active() {
            return;
        }
        let eccentricity = numinous_core::rooms::kepler_laws::eccentricity_for_inputs(
            self.t,
            &self.inputs,
            self.variation,
        );
        let _ = self.kepler_aha.bind_eccentricity(eccentricity);
        let tunings = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        self.kepler_aha.note_tunings(tunings);
    }

    /// Keep the Parrondo aha in step with completed policy selections.
    pub(super) fn sync_parrondo_aha(&mut self) {
        if !self.current_room_is_parrondo() || !self.chosen_experiment_active() {
            return;
        }
        let selections = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        self.parrondo_aha.note_selections(selections);
    }

    /// Keep the dice aha bound to the newest completed die choice.
    pub(super) fn sync_nontransitive_aha(&mut self) {
        if !self.current_room_is_nontransitive() || !self.chosen_experiment_active() {
            return;
        }
        let choices = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        let chosen = numinous_core::rooms::nontransitive::selected_die_from_inputs(&self.inputs);
        self.nontransitive_aha.note_choices(chosen, choices);
    }

    pub(super) fn record_current_aha_consolidation(&mut self) {
        if !self.chosen_experiment_active() {
            return;
        }
        let room_id = self.rooms[self.current].meta().id;
        let consolidated = match room_id {
            "times-tables" => self.times_tables_aha.allow_reveal_text(),
            "buffon-needle" => self.buffon_aha.allow_reveal_text(),
            "galton-board" => self.galton_aha.allow_reveal_text(),
            "double-pendulum" => self.pendulum_aha.allow_reveal_text(),
            "kepler-laws" => self.kepler_aha.allow_reveal_text(),
            "parrondo" => self.parrondo_aha.allow_reveal_text(),
            "nontransitive" => self.nontransitive_aha.allow_reveal_text(),
            _ => false,
        };
        if consolidated && self.journey.consolidate(room_id) {
            self.journey_changed();
        }
    }

    /// Whether the selected experiment has an earned connection to show.
    pub(super) fn can_advance_chosen_experiment(&self) -> bool {
        if !self.chosen_experiment_active() {
            return false;
        }
        match self.rooms[self.current].meta().id {
            "times-tables" => self.times_tables_aha.can_summon(),
            "buffon-needle" => self.buffon_aha.can_summon(),
            "galton-board" => self.galton_aha.can_summon(),
            "double-pendulum" => self.pendulum_aha.can_summon(),
            "kepler-laws" => self.kepler_aha.can_summon(),
            "parrondo" => self.parrondo_aha.can_summon(),
            "nontransitive" => self.nontransitive_aha.can_summon(),
            _ => false,
        }
    }

    /// Show or confirm an earned connection, without opening or gating study.
    pub(super) fn advance_chosen_experiment(&mut self) -> bool {
        if !self.can_advance_chosen_experiment() {
            return false;
        }
        let advanced = match self.rooms[self.current].meta().id {
            "times-tables" => self.times_tables_aha.summon(),
            "buffon-needle" => self.buffon_aha.summon(),
            "galton-board" => self.galton_aha.summon(),
            "double-pendulum" => self.pendulum_aha.summon(),
            "kepler-laws" => self.kepler_aha.summon(),
            "parrondo" => self.parrondo_aha.summon(),
            "nontransitive" => self.nontransitive_aha.summon(),
            _ => false,
        };
        if advanced {
            self.record_current_aha_consolidation();
        }
        advanced
    }

    pub(super) fn commit_times_tables_wager(
        &mut self,
        place: numinous_core::rooms::times_tables_aha::CardioidHome,
    ) -> bool {
        if !self.current_room_is_times_tables() || !self.chosen_experiment_active() {
            return false;
        }
        if self.times_tables_aha.commit_wager(place) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("GUESSED {}", place.label()),
                90,
            ));
            true
        } else {
            false
        }
    }

    pub(super) fn advance_times_tables_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_times_tables() || !self.chosen_experiment_active() || self.paused {
            return;
        }
        if !matches!(
            self.times_tables_aha.beat(),
            numinous_core::rooms::times_tables_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        let delta = elapsed / TIMES_TABLES_MORPH_SECONDS;
        self.times_tables_aha.advance_morph(delta);
    }

    pub(super) fn commit_buffon_wager(&mut self, guess: f64) -> bool {
        if !self.current_room_is_buffon() || !self.chosen_experiment_active() {
            return false;
        }
        if self.buffon_aha.commit_wager(guess) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(format!("GUESSED {guess:.2}"), 90));
            true
        } else {
            false
        }
    }

    pub(super) fn advance_buffon_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_buffon() || !self.chosen_experiment_active() || self.paused {
            return;
        }
        if !matches!(
            self.buffon_aha.beat(),
            numinous_core::rooms::buffon_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        let delta = elapsed / BUFFON_MORPH_SECONDS;
        self.buffon_aha.advance_morph(delta);
    }

    pub(super) fn commit_galton_wager(&mut self, bin: usize) -> bool {
        if !self.current_room_is_galton() || !self.chosen_experiment_active() {
            return false;
        }
        let coin = numinous_core::rooms::galton_board::selected_coin_from_inputs(&self.inputs)
            .unwrap_or(2);
        if self.galton_aha.commit_wager(bin, coin) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(format!("GUESSED BIN {bin}"), 90));
            true
        } else {
            false
        }
    }

    pub(super) fn advance_galton_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_galton() || !self.chosen_experiment_active() || self.paused {
            return;
        }
        if !matches!(
            self.galton_aha.beat(),
            numinous_core::rooms::galton_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        let delta = elapsed / GALTON_MORPH_SECONDS;
        self.galton_aha.advance_morph(delta);
    }

    pub(super) fn commit_pendulum_call(
        &mut self,
        ending: numinous_core::rooms::pendulum_aha::Ending,
    ) -> bool {
        if !self.current_room_is_pendulum() || !self.chosen_experiment_active() {
            return false;
        }
        if self.pendulum_aha.commit_call(ending) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", ending.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    pub(super) fn advance_pendulum_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_pendulum() || !self.chosen_experiment_active() || self.paused {
            return;
        }
        if !matches!(
            self.pendulum_aha.beat(),
            numinous_core::rooms::pendulum_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.pendulum_aha
            .advance_morph(elapsed / PENDULUM_MORPH_SECONDS);
    }

    pub(super) fn commit_kepler_call(
        &mut self,
        relation: numinous_core::rooms::kepler_aha::SpeedRelation,
    ) -> bool {
        if !self.current_room_is_kepler() || !self.chosen_experiment_active() {
            return false;
        }
        if self.kepler_aha.commit_call(relation) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", relation.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    pub(super) fn advance_kepler_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_kepler() || !self.chosen_experiment_active() || self.paused {
            return;
        }
        if !matches!(
            self.kepler_aha.beat(),
            numinous_core::rooms::kepler_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.kepler_aha
            .advance_morph(elapsed / KEPLER_MORPH_SECONDS);
    }

    pub(super) fn commit_parrondo_call(
        &mut self,
        policy: numinous_core::rooms::parrondo::Policy,
    ) -> bool {
        if !self.current_room_is_parrondo() || !self.chosen_experiment_active() {
            return false;
        }
        if self.parrondo_aha.commit_call(policy) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", policy.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    pub(super) fn advance_parrondo_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_parrondo() || !self.chosen_experiment_active() || self.paused {
            return;
        }
        if !matches!(
            self.parrondo_aha.beat(),
            numinous_core::rooms::parrondo_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.parrondo_aha
            .advance_morph(elapsed / PARRONDO_MORPH_SECONDS);
    }

    pub(super) fn commit_nontransitive_call(
        &mut self,
        die: numinous_core::rooms::nontransitive::Die,
    ) -> bool {
        if !self.current_room_is_nontransitive() || !self.chosen_experiment_active() {
            return false;
        }
        if self.nontransitive_aha.commit_call(die) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", die.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    pub(super) fn advance_nontransitive_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_nontransitive() || !self.chosen_experiment_active() || self.paused
        {
            return;
        }
        if !matches!(
            self.nontransitive_aha.beat(),
            numinous_core::rooms::nontransitive_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.nontransitive_aha
            .advance_morph(elapsed / NONTRANSITIVE_MORPH_SECONDS);
    }

    pub(super) fn record_room_touch(&mut self, point: (f64, f64)) -> bool {
        let poke_added = room_input::push_poke(&mut self.pokes, point);
        let input_added = room_input::record_pointer_down(&mut self.inputs, point, self.t);
        if poke_added && input_added && self.current_room_is_life() {
            let launched = self.life_session.launch(point);
            if launched {
                self.life_accumulator = 0.0;
                self.clear_transient_audio();
            }
            return launched;
        }
        let accepted = poke_added && input_added;
        if accepted {
            self.maybe_announce_room_goal();
            self.sync_room_parameter_voice();
            self.play_room_interaction_audio(true);
        }
        accepted
    }

    pub(super) fn maybe_announce_room_goal(&mut self) {
        if self.goal_announced || !self.rooms[self.current].goal_met(self.t, &self.inputs) {
            return;
        }
        self.goal_announced = true;
        self.banner = Some(feedback::room_goal(
            self.rooms[self.current]
                .goal()
                .unwrap_or("DISCOVERY COMPLETE"),
        ));
    }

    pub(super) fn advance_life(&mut self, elapsed: f64) -> usize {
        if !self.current_room_is_life() || !elapsed.is_finite() || elapsed <= 0.0 {
            return 0;
        }
        let max_backlog = LIFE_STEP_SECONDS * MAX_LIFE_STEPS_PER_TICK as f64;
        self.life_accumulator = (self.life_accumulator + elapsed).min(max_backlog);
        let steps = ((self.life_accumulator + 1e-9) / LIFE_STEP_SECONDS).floor() as usize;
        let steps = steps.min(MAX_LIFE_STEPS_PER_TICK);
        for _ in 0..steps {
            self.life_session.advance();
        }
        self.life_accumulator -= steps as f64 * LIFE_STEP_SECONDS;
        // A catch-up tick presents only the newest generation. Voice that same
        // state once instead of replaying a stale burst after the picture.
        self.play_life_step_audio(steps);
        steps
    }

    pub(super) fn advance_life_if_active(&mut self, elapsed: f64) -> usize {
        if !self.window_active
            || self.paused
            || self.dragging
            || self.show_help && self.menu.is_open()
        {
            return 0;
        }
        self.advance_life(elapsed * self.time_scale * self.visualizer_scale)
    }
}
