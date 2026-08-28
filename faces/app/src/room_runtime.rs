//! Face-level live room coordination.

use numinous_core::Room;

use super::{
    App, BUFFON_MORPH_SECONDS, GALTON_MORPH_SECONDS, KEPLER_MORPH_SECONDS, LIFE_STEP_SECONDS,
    MAX_LIFE_STEPS_PER_TICK, NONTRANSITIVE_MORPH_SECONDS, PARRONDO_MORPH_SECONDS,
    PENDULUM_MORPH_SECONDS, TIMES_TABLES_MORPH_SECONDS, effective_room_phase, feedback,
    has_finite_parameter_input, room_input, wager,
};

impl App {
    pub(super) fn reset_room_runtime(&mut self) {
        self.clear_pointer_state();
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

    pub(super) fn current_status_override(&self, width: usize) -> Option<String> {
        if self.current_room_is_life() {
            return Some(if width <= 400 {
                self.life_session.compact_status()
            } else {
                self.life_session.status()
            });
        }
        if self.current_room_is_times_tables() && !self.the_show {
            let phase = effective_room_phase("times-tables", self.t, &self.inputs, self.the_show);
            let dial = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.times_tables_aha.status(dial.as_deref()));
        }
        if self.current_room_is_buffon() && !self.the_show {
            let phase = effective_room_phase("buffon-needle", self.t, &self.inputs, self.the_show);
            let throws = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.buffon_aha.status(throws.as_deref()));
        }
        if self.current_room_is_pendulum() && !self.the_show {
            let phase =
                effective_room_phase("double-pendulum", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.pendulum_aha.status(readout.as_deref()));
        }
        if self.current_room_is_kepler() && !self.the_show {
            let phase = effective_room_phase("kepler-laws", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.kepler_aha.status(readout.as_deref()));
        }
        if self.current_room_is_parrondo() && !self.the_show {
            let phase = effective_room_phase("parrondo", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.parrondo_aha.status(readout.as_deref()));
        }
        if self.current_room_is_nontransitive() && !self.the_show {
            let phase = effective_room_phase("nontransitive", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.nontransitive_aha.status(readout.as_deref()));
        }
        if let Some(posed) = &self.room_wager {
            return Some(posed.status());
        }
        if self.current_room_is_galton() && !self.the_show {
            let phase = effective_room_phase("galton-board", self.t, &self.inputs, self.the_show);
            let pile = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.galton_aha.status(pile.as_deref()));
        }
        None
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

    /// U poses the room's own prediction, or closes an open one.
    ///
    /// Every room with a moving numeric readout can be called, which is
    /// most of the catalog; the flagship rooms keep their hand-staged
    /// ahas instead, because a bespoke five-beat arc outranks the generic
    /// one where it exists.
    pub(super) fn toggle_room_wager(&mut self) {
        if self.the_show || self.studio || self.arcade.is_some() {
            return;
        }
        if self.room_wager.take().is_some() {
            return;
        }
        if numinous_core::is_engineered_aha_room(self.rooms[self.current].meta().id) {
            self.banner = Some(feedback::Banner::status(
                "THIS ROOM STAGES ITS OWN WAGER",
                feedback::REFUSAL_FRAMES,
            ));
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
        if !self.current_room_is_times_tables() || self.the_show {
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
        if !self.current_room_is_buffon() || self.the_show {
            return;
        }
        let throws = numinous_core::rooms::buffon_needle::BuffonNeedle::throw_count(&self.inputs);
        self.buffon_aha.note_throws(throws);
    }

    /// Keep the Galton aha in step with the waves the pile is built from.
    pub(super) fn sync_galton_aha(&mut self) {
        if !self.current_room_is_galton() || self.the_show {
            return;
        }
        let waves = numinous_core::rooms::galton_board::wave_count_from_inputs(&self.inputs);
        let coin = numinous_core::rooms::galton_board::selected_coin_from_inputs(&self.inputs)
            .unwrap_or(2);
        self.galton_aha.note_waves(waves, coin);
    }

    /// Keep the Double Pendulum aha in step with completed releases.
    pub(super) fn sync_pendulum_aha(&mut self) {
        if !self.current_room_is_pendulum() || self.the_show {
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
        if !self.current_room_is_kepler() || self.the_show {
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
        if !self.current_room_is_parrondo() || self.the_show {
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
        if !self.current_room_is_nontransitive() || self.the_show {
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

    /// E / Inspect: summon staged aha on flagship rooms; elsewhere toggle reveal.
    pub(super) fn toggle_inspect(&mut self) {
        if self.the_show || self.studio {
            return;
        }
        if self.current_room_is_times_tables() {
            use numinous_core::rooms::times_tables_aha::AhaBeat;
            if self.times_tables_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.times_tables_aha.can_summon()
                || matches!(self.times_tables_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.times_tables_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            // Generation first: do not open the punchline card early.
            self.show_info = false;
            return;
        }
        if self.current_room_is_buffon() {
            use numinous_core::rooms::buffon_aha::AhaBeat;
            if self.buffon_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.buffon_aha.can_summon()
                || matches!(self.buffon_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.buffon_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_galton() {
            use numinous_core::rooms::galton_aha::AhaBeat;
            if self.galton_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.galton_aha.can_summon()
                || matches!(self.galton_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.galton_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_pendulum() {
            use numinous_core::rooms::pendulum_aha::AhaBeat;
            if self.pendulum_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.pendulum_aha.can_summon()
                || matches!(self.pendulum_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.pendulum_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_kepler() {
            use numinous_core::rooms::kepler_aha::AhaBeat;
            if self.kepler_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.kepler_aha.can_summon()
                || matches!(self.kepler_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.kepler_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_parrondo() {
            use numinous_core::rooms::parrondo_aha::AhaBeat;
            if self.parrondo_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.parrondo_aha.can_summon()
                || matches!(self.parrondo_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.parrondo_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_nontransitive() {
            use numinous_core::rooms::nontransitive_aha::AhaBeat;
            if self.nontransitive_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.nontransitive_aha.can_summon()
                || matches!(self.nontransitive_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.nontransitive_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        self.show_info = !self.show_info;
    }

    pub(super) fn commit_times_tables_wager(
        &mut self,
        place: numinous_core::rooms::times_tables_aha::CardioidHome,
    ) -> bool {
        if !self.current_room_is_times_tables() || self.the_show {
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
        if !self.current_room_is_times_tables() || self.the_show || self.paused {
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
        if !self.current_room_is_buffon() || self.the_show {
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
        if !self.current_room_is_buffon() || self.the_show || self.paused {
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
        if !self.current_room_is_galton() || self.the_show {
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
        if !self.current_room_is_galton() || self.the_show || self.paused {
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
        if !self.current_room_is_pendulum() || self.the_show {
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
        if !self.current_room_is_pendulum() || self.the_show || self.paused {
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
        if !self.current_room_is_kepler() || self.the_show {
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
        if !self.current_room_is_kepler() || self.the_show || self.paused {
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
        if !self.current_room_is_parrondo() || self.the_show {
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
        if !self.current_room_is_parrondo() || self.the_show || self.paused {
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
        if !self.current_room_is_nontransitive() || self.the_show {
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
        if !self.current_room_is_nontransitive() || self.the_show || self.paused {
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
