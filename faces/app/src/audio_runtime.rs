//! Face-level audio source coordination and presentation feedback.

use std::sync::Arc;
use std::time::SystemTime;

use numinous_audio::{Bus, Transition};
use numinous_core::{ROOM_BED_SOURCE_RATE, Room};

use crate::audio_state::Program as AudioProgram;
use crate::room_phase::{effective_room_phase, has_finite_parameter_input};
use crate::{App, effective_room_inputs, feedback, menu, radio_cache, studio_panel};

/// One press of a volume control: keyboard, controller, or a Settings row.
pub(super) const VOLUME_STEP: f32 = 0.1;
/// One press of a radio, room, or effect level: the master's ten points.
const LEVEL_STEP_PERCENT: i16 = 10;

/// The audio half of a curated Studio recipe change, paired with the curve's
/// own morph and validated when the App is built.
const RECIPE_TRANSITION: Transition =
    match Transition::crossfade(studio_panel::RECIPE_MORPH_SECONDS as f32) {
        Some(transition) => transition,
        None => panic!("the Studio recipe morph must be a valid audio crossfade"),
    };

pub(super) fn selected_parameter_sound(
    program: AudioProgram,
    modal_active: bool,
    room: &dyn Room,
    phase: f64,
    inputs: &[numinous_core::RoomInput],
    the_show: bool,
) -> Option<numinous_core::ParametricSound> {
    let has_route_key = room.meta().id == "route-lab"
        && inputs
            .iter()
            .any(|input| matches!(input, numinous_core::RoomInput::Key { .. }));
    if program != AudioProgram::RoomScore
        || modal_active
        || !the_show && !has_finite_parameter_input(inputs) && !has_route_key
    {
        return None;
    }
    let effective_phase = effective_room_phase(room.meta().id, phase, inputs, the_show);
    room.parameter_sound(effective_phase, effective_room_inputs(inputs, the_show))
}

pub(super) fn life_step_audio_owned(
    program: AudioProgram,
    modal_active: bool,
    room_id: &str,
) -> bool {
    room_transient_audio_owned(program, modal_active) && room_id == "game-of-life"
}

pub(super) fn room_transient_audio_owned(program: AudioProgram, modal_active: bool) -> bool {
    program == AudioProgram::RoomScore && !modal_active
}

pub(super) fn selected_life_step_audio(
    program: AudioProgram,
    modal_active: bool,
    muted: bool,
    completed_steps: usize,
    session: &numinous_core::rooms::game_of_life::LifeSession,
    sample_rate: u32,
) -> Option<Vec<f32>> {
    if !life_step_audio_owned(program, modal_active, "game-of-life")
        || muted
        || completed_steps == 0
    {
        return None;
    }
    let samples = session.step_sound().render_stereo(sample_rate);
    (!samples.is_empty()).then_some(samples)
}

pub(super) fn selected_room_interaction_audio(
    program: AudioProgram,
    modal_active: bool,
    muted: bool,
    accepted: bool,
    room: &dyn Room,
    inputs: &[numinous_core::RoomInput],
    sample_rate: u32,
) -> Option<Vec<f32>> {
    if program != AudioProgram::RoomScore || modal_active || muted || !accepted {
        return None;
    }
    room.interaction_stereo(inputs, sample_rate)
        .filter(|samples| !samples.is_empty())
}

impl App {
    /// Publish Watch Agent sound once per selected public sequence.
    pub(super) fn sync_viewer_audio(&mut self) {
        if !self.session_viewer.is_open() {
            return;
        }
        self.audio_program = AudioProgram::WatchAgent;
        let selection = self.session_viewer.audio_selection();
        let sequence = selection.as_ref().map(|sel| sel.public_sequence());
        if !self.session_audio.select(sequence) {
            self.apply_levels();
            return;
        }
        self.publish_viewer_audio(selection.as_ref());
    }

    pub(super) fn publish_viewer_audio(
        &mut self,
        selection: Option<&numinous_app::session_viewer::AudioSelection>,
    ) {
        self.audio_program = AudioProgram::WatchAgent;
        let Some(player) = &self.player else {
            return;
        };
        player.clear_parameter_voice();
        player.clear_oneshot(Bus::Room);
        self.apply_levels();
        let stereo = match selection.and_then(|sel| sel.render(ROOM_BED_SOURCE_RATE)) {
            Some(mono) if !mono.is_empty() => mono
                .into_iter()
                .flat_map(|sample| [sample, sample])
                .collect::<Vec<_>>(),
            _ => vec![0.0, 0.0],
        };
        let _ = player.set_shared_stereo_at_rate(
            Bus::Room,
            Arc::new(stereo),
            ROOM_BED_SOURCE_RATE,
            Transition::QUICK,
        );
    }

    pub(super) fn change_volume(&mut self, step: f32) {
        self.volume = (self.volume + step).clamp(0.0, 1.0);
        self.banner = Some(feedback::volume(self.volume, self.muted));
        self.apply_levels();
        self.persist_preferences();
    }

    /// Send the master level, mute, and every bus level to the player.
    pub(super) fn apply_levels(&self) {
        if let Some(player) = &self.player {
            player.set_master_gain(if self.muted { 0.0 } else { self.volume });
            for bus in Bus::ALL {
                player.set_bus_gain(bus, f32::from(self.bus_level(bus)) / 100.0);
            }
        }
    }

    /// The player's level for `bus`, in whole percent beneath master.
    pub(super) fn bus_level(&self, bus: Bus) -> u8 {
        match bus {
            Bus::Music => self.music_volume_percent,
            Bus::Room => self.room_volume_percent,
            Bus::Effect => self.effect_volume_percent,
        }
    }

    fn change_bus_level(&mut self, bus: Bus, setting: menu::NumericSetting, step: menu::Step) {
        let slot = match bus {
            Bus::Music => &mut self.music_volume_percent,
            Bus::Room => &mut self.room_volume_percent,
            Bus::Effect => &mut self.effect_volume_percent,
        };
        let stepped = i16::from(*slot) + step.sign() as i16 * LEVEL_STEP_PERCENT;
        *slot = u8::try_from(stepped.clamp(0, 100)).unwrap_or(100);
        let percent = *slot;
        self.banner = Some(feedback::level(setting.title(), percent, self.muted));
        self.apply_levels();
        self.persist_preferences();
    }

    pub(super) fn toggle_mute(&mut self) {
        self.set_muted(!self.muted);
    }

    /// Record an explicit mute choice: it silences now and persists.
    pub(super) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.muted_preference = muted;
        self.apply_levels();
        self.persist_preferences();
    }

    /// Step one numeric Settings row.
    pub(super) fn adjust_setting(&mut self, setting: menu::NumericSetting, step: menu::Step) {
        match setting {
            menu::NumericSetting::MasterVolume => self.change_volume(step.sign() * VOLUME_STEP),
            menu::NumericSetting::MusicVolume => self.change_bus_level(Bus::Music, setting, step),
            menu::NumericSetting::RoomVolume => self.change_bus_level(Bus::Room, setting, step),
            menu::NumericSetting::EffectVolume => {
                self.change_bus_level(Bus::Effect, setting, step);
            }
        }
    }

    /// Tune in to the current dial position: build the playlist, join the
    /// broadcast mid-stream (the station was always on the air), and play.
    pub(super) fn tune_in(&mut self) {
        self.clear_pointer_state();
        self.radio_track = Arc::new(Vec::new());
        self.radio_track_rate = 44_100;
        self.radio_paths.clear();
        self.radio_until = None;
        let Some(i) = self.radio else {
            self.update_audio();
            if let Some(window) = &self.window {
                window.set_title(&self.title());
            }
            self.banner = Some(feedback::radio_off());
            return;
        };
        let st = &numinous_core::STATIONS[i];
        let dir = radio_cache::default_dir();
        self.radio_paths = radio_cache::station_tracks(&dir, st.id);
        // Join the broadcast live: the wall clock decides which track is on.
        let _ = self.sync_radio_to_wall_clock();
        // The dial speaks on screen, especially when the station is silent.
        let st = &numinous_core::STATIONS[i];
        self.banner = Some(feedback::radio(st.name, st.id, self.radio_paths.len()));
        self.update_audio();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    pub(super) fn sync_radio_at(&mut self, now_secs: f64) -> bool {
        // Watch Agent owns the source for the whole paired session.
        if self.studio || self.session_viewer.is_open() {
            return false;
        }
        if self.radio.is_none() {
            self.radio_track = Arc::new(Vec::new());
            self.radio_until = None;
            self.update_audio();
            return false;
        }
        let Some((index, position)) = radio_cache::live_position(&self.radio_paths, now_secs)
        else {
            self.radio_track = Arc::new(Vec::new());
            self.radio_until = None;
            self.update_audio();
            return false;
        };
        self.radio_index = index;
        let playing = self.radio_play_or_advance(position);
        if !playing {
            self.update_audio();
        }
        playing
    }

    pub(super) fn sync_radio_to_wall_clock(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs_f64())
            .unwrap_or(0.0);
        self.sync_radio_at(now)
    }

    pub(super) fn radio_play_or_advance(&mut self, offset: f64) -> bool {
        let track_count = self.radio_paths.len();
        if track_count == 0 {
            self.radio_track = Arc::new(Vec::new());
            self.radio_track_rate = 44_100;
            self.radio_until = None;
            return false;
        }
        self.radio_index %= track_count;
        let mut next_offset = offset;
        for _ in 0..track_count {
            if self.radio_play(next_offset) {
                return true;
            }
            self.radio_index = (self.radio_index + 1) % track_count;
            next_offset = 0.0;
        }
        self.radio_track = Arc::new(Vec::new());
        self.radio_track_rate = 44_100;
        self.radio_until = None;
        false
    }

    /// Put the current playlist entry on the air, starting `offset` seconds
    /// in: read it (mono or stereo), retain one source-rate stereo buffer, and
    /// hand it to the player for live rate conversion.
    pub(super) fn radio_play(&mut self, offset: f64) -> bool {
        self.radio_track = Arc::new(Vec::new());
        self.radio_track_rate = 44_100;
        self.radio_until = None;
        let Some(path) = self.radio_paths.get(self.radio_index) else {
            return false;
        };
        let device_rate = self.player.as_ref().map_or(44_100, |p| p.sample_rate());
        let Some(loaded) = radio_cache::load_track(path, offset, device_rate) else {
            return false;
        };
        self.radio_track = loaded.stereo;
        self.radio_track_rate = loaded.sample_rate;
        self.radio_until = Some(std::time::Instant::now() + loaded.remaining);
        self.audio_program = AudioProgram::Radio;
        if let Some(player) = &self.player {
            player.clear_parameter_voice();
            player.clear_oneshot(Bus::Room);
            let _ = player.set_shared_stereo_at_rate(
                Bus::Music,
                self.radio_track.clone(),
                self.radio_track_rate,
                Transition::QUICK,
            );
            self.apply_levels();
        }
        true
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

    pub(super) fn set_studio_edit_sound(&mut self, parsed: Option<numinous_core::SoundSpec>) {
        let spec = parsed.or_else(|| self.studio_panel.current_sound());
        self.set_studio_sound(spec);
    }

    pub(super) fn set_studio_sound(&mut self, spec: Option<numinous_core::SoundSpec>) {
        self.set_studio_sound_with_transition(spec, Transition::QUICK);
    }

    pub(super) fn set_studio_recipe_sound(&mut self, spec: Option<numinous_core::SoundSpec>) {
        self.set_studio_sound_with_transition(spec, RECIPE_TRANSITION);
    }

    fn set_studio_sound_with_transition(
        &mut self,
        spec: Option<numinous_core::SoundSpec>,
        transition: Transition,
    ) {
        self.audio_program = AudioProgram::Studio;
        let Some(player) = &self.player else {
            return;
        };
        player.clear_parameter_voice();
        player.clear_oneshot(Bus::Room);
        self.apply_levels();
        if let Some(spec) = spec {
            let _ = player.set_samples(Bus::Room, spec.render(player.sample_rate()), transition);
        }
    }

    /// Render the current room's stable score and crossfade to it.
    pub(super) fn update_audio(&mut self) {
        if self.session_viewer.is_open() {
            self.sync_viewer_audio();
            return;
        }
        if self.studio {
            self.audio_program = AudioProgram::Studio;
            if let Some(player) = &self.player {
                player.clear_parameter_voice();
                player.clear_oneshot(Bus::Room);
            }
            self.apply_levels();
            return;
        }
        if self.radio.is_some() && !self.radio_track.is_empty() {
            self.audio_program = AudioProgram::Radio;
            if let Some(player) = &self.player {
                player.clear_parameter_voice();
                player.clear_oneshot(Bus::Room);
            }
            self.apply_levels();
            return;
        }
        let switching_to_room_score = self.audio_program != AudioProgram::RoomScore;
        if switching_to_room_score {
            self.clear_pointer_state();
        }
        self.audio_program = AudioProgram::RoomScore;
        let Some(player) = &self.player else {
            return;
        };
        self.apply_levels();
        let rendered_room_score = self.tune.is_empty();
        if rendered_room_score {
            self.tune = Arc::new(match self.rooms[self.current].motif() {
                Some(motif) => motif.arrangement().render_stereo(ROOM_BED_SOURCE_RATE),
                None => numinous_core::compose(self.current as u64 + 1, 8)
                    .render(ROOM_BED_SOURCE_RATE)
                    .into_iter()
                    .flat_map(|sample| [sample, sample])
                    .collect(),
            });
        }
        if rendered_room_score || switching_to_room_score {
            let _ = player.set_shared_stereo_at_rate(
                Bus::Room,
                self.tune.clone(),
                ROOM_BED_SOURCE_RATE,
                Transition::QUICK,
            );
        }
        self.sync_room_parameter_voice();
    }

    pub(super) fn desired_room_parameter_sound(&self) -> Option<numinous_core::ParametricSound> {
        selected_parameter_sound(
            self.audio_program,
            self.modal_mode_active(),
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.the_show,
        )
    }

    pub(super) fn sync_room_parameter_voice(&self) {
        if !room_transient_audio_owned(self.audio_program, self.modal_mode_active()) {
            self.clear_transient_audio();
        }
        let Some(player) = &self.player else {
            return;
        };
        let voice = self.desired_room_parameter_sound();
        if let Some(voice) = voice {
            let _ = player.set_parameter_voice(voice.root_hz(), voice.ratio(), voice.gain());
        } else {
            player.clear_parameter_voice();
        }
    }

    /// Normalized room-bed spectrum for the visualizer meter (cached per room).
    fn room_spectrum_bands(&mut self) -> Option<[f32; numinous_core::BAND_COUNT]> {
        if let Some((idx, bands)) = self.spectrum_cache
            && idx == self.current
        {
            return Some(bands);
        }
        let motif = self.rooms.get(self.current)?.motif()?;
        let samples = motif
            .arrangement()
            .render_stereo(numinous_core::ROOM_BED_SOURCE_RATE);
        let bands =
            numinous_core::arrangement_spectrum(&samples, numinous_core::ROOM_BED_SOURCE_RATE);
        self.spectrum_cache = Some((self.current, bands));
        Some(bands)
    }

    /// Live visualizer bands from the preferred source, with graceful fallback.
    pub(super) fn visualizer_bands(
        &mut self,
    ) -> Option<(
        [f32; numinous_core::BAND_COUNT],
        numinous_audio::VisualizerSource,
    )> {
        match self.visualizer_source {
            numinous_audio::VisualizerSource::Loopback => {
                if let Some(capture) = self.loopback.as_ref() {
                    let samples = capture.snapshot_frames(2_048);
                    if samples.len() >= 64 {
                        let bands =
                            numinous_core::arrangement_spectrum(&samples, capture.sample_rate());
                        return Some((bands, numinous_audio::VisualizerSource::Loopback));
                    }
                }
                // Fall through to output mix, then room bed.
                if let Some(bands) = self.output_mix_bands() {
                    return Some((bands, numinous_audio::VisualizerSource::OutputMix));
                }
                self.room_spectrum_bands()
                    .map(|b| (b, numinous_audio::VisualizerSource::RoomBed))
            }
            numinous_audio::VisualizerSource::OutputMix => {
                if let Some(bands) = self.output_mix_bands() {
                    return Some((bands, numinous_audio::VisualizerSource::OutputMix));
                }
                self.room_spectrum_bands()
                    .map(|b| (b, numinous_audio::VisualizerSource::RoomBed))
            }
            numinous_audio::VisualizerSource::RoomBed
            | numinous_audio::VisualizerSource::Silent => self
                .room_spectrum_bands()
                .map(|b| (b, numinous_audio::VisualizerSource::RoomBed)),
        }
    }

    fn output_mix_bands(&self) -> Option<[f32; numinous_core::BAND_COUNT]> {
        let player = self.player.as_ref()?;
        let samples = player.snapshot_output_tap(2_048);
        if samples.len() < 64 {
            return None;
        }
        Some(numinous_core::arrangement_spectrum(
            &samples,
            player.sample_rate(),
        ))
    }

    /// Cycle visualizer source: room bed, output mix, loopback (when present).
    pub(super) fn cycle_visualizer_source(&mut self) {
        self.visualizer_source = match self.visualizer_source {
            numinous_audio::VisualizerSource::RoomBed
            | numinous_audio::VisualizerSource::Silent => {
                numinous_audio::VisualizerSource::OutputMix
            }
            numinous_audio::VisualizerSource::OutputMix => {
                if self.loopback.is_none() {
                    self.loopback = numinous_audio::InputCapture::try_open_loopback().ok();
                }
                if self.loopback.is_some() {
                    numinous_audio::VisualizerSource::Loopback
                } else {
                    numinous_audio::VisualizerSource::RoomBed
                }
            }
            numinous_audio::VisualizerSource::Loopback => {
                self.loopback = None;
                numinous_audio::VisualizerSource::RoomBed
            }
        };
        let label = match self.visualizer_source {
            numinous_audio::VisualizerSource::Loopback => self
                .loopback
                .as_ref()
                .map(|c| format!("VIZ {}", c.device_name()))
                .unwrap_or_else(|| "VIZ LOOPBACK".into()),
            other => format!("VIZ {}", other.label()),
        };
        self.banner = Some(feedback::Banner::status(label, 90));
    }

    pub(super) fn clear_transient_audio(&self) {
        #[cfg(test)]
        self.transient_audio_clears
            .set(self.transient_audio_clears.get().saturating_add(1));
        if let Some(player) = &self.player {
            player.clear_oneshot(Bus::Room);
        }
    }

    pub(super) fn play_room_interaction_audio(&self, accepted: bool) {
        #[cfg(test)]
        if selected_room_interaction_audio(
            self.audio_program,
            self.modal_mode_active(),
            self.muted,
            accepted,
            self.rooms[self.current].as_ref(),
            &self.inputs,
            48_000,
        )
        .is_some()
        {
            self.interaction_audio_events
                .set(self.interaction_audio_events.get().saturating_add(1));
        }
        let Some(player) = &self.player else {
            return;
        };
        let Some(samples) = selected_room_interaction_audio(
            self.audio_program,
            self.modal_mode_active(),
            self.muted,
            accepted,
            self.rooms[self.current].as_ref(),
            &self.inputs,
            player.sample_rate(),
        ) else {
            return;
        };
        player.play_stereo_oneshot(Bus::Room, samples, 0.65);
    }

    pub(super) fn play_life_step_audio(&self, completed_steps: usize) {
        let Some(player) = &self.player else {
            return;
        };
        let Some(samples) = selected_life_step_audio(
            self.audio_program,
            self.modal_mode_active(),
            self.muted,
            completed_steps,
            &self.life_session,
            player.sample_rate(),
        ) else {
            return;
        };
        player.play_stereo_oneshot(Bus::Room, samples, 0.65);
    }
}
