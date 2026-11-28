//! Live audio drives the presentation without manufacturing a player's gesture.

use super::{App, tests::headless};
use numinous_audio::VisualizerSource;
use numinous_core::photosensitivity::regional::{RegionalFlashCapture, TimelineFlashPeaks};
use numinous_core::{BAND_COUNT, Era, Motion, Raster};
use std::time::{Duration, Instant};

const BASS: [f32; BAND_COUNT] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
const MID: [f32; BAND_COUNT] = [0.001, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
const SILENT: [f32; BAND_COUNT] = [0.0; BAND_COUNT];

fn julia_room() -> App {
    let mut app = headless("julia-music-beats");
    app.current = app
        .rooms
        .iter()
        .position(|r| r.meta().id == "julia")
        .unwrap();
    app.show_help = false;
    app.banner = None;
    app.room_card = 0;
    app.muted = false;
    app.motion = Motion::Full;
    app
}

#[test]
fn music_shares_its_clock_across_sources_and_leaves_the_players_hand_alone() {
    let mut app = julia_room();
    let start = Instant::now();
    let pulse = |app: &mut App, source, bands, offset_ms| {
        app.compose_frame_with_bands(
            Raster::new(64, 48),
            64,
            48,
            Some((bands, source)),
            start + Duration::from_millis(offset_ms),
        );
    };
    use VisualizerSource::{Loopback, OutputMix, RoomBed};
    pulse(&mut app, RoomBed, BASS, 0);
    assert!(app.pokes.is_empty());
    pulse(&mut app, OutputMix, SILENT, 1);
    pulse(&mut app, OutputMix, BASS, 2);
    pulse(&mut app, Loopback, SILENT, 3);
    pulse(&mut app, Loopback, MID, 4);
    assert_eq!(app.pokes.len(), 1);
    pulse(&mut app, OutputMix, SILENT, 400);
    app.motion = Motion::Reduced;
    pulse(&mut app, OutputMix, MID, 401);
    assert_eq!(app.pokes.len(), 1);
    assert!(app.inputs.is_empty());
    assert!(app.record_room_touch((0.45, 0.5)));
    app.motion = Motion::Full;
    app.poking = true;
    pulse(&mut app, OutputMix, SILENT, 402);
    pulse(&mut app, OutputMix, MID, 403);
    assert_eq!(app.pokes.last(), Some(&(0.45, 0.5)));
    app.poking = false;
    pulse(&mut app, Loopback, SILENT, 404);
    pulse(&mut app, Loopback, MID, 405);
    assert_eq!(app.pokes.len(), 3);
    assert_eq!(app.inputs.len(), 1);
}

#[test]
fn changing_bass_and_onset_share_the_next_displayed_phases_budget() {
    const SECONDS: f64 = 1.0 / 60.0;
    let mut app = julia_room();
    app.time_scale = numinous_core::MAX_TIME_SCALE;
    app.t = 0.98;
    let cap = numinous_core::ROOM_CYCLES_PER_SECOND * SECONDS * app.time_scale;
    let start = Instant::now();
    let mut peak: f64 = 0.0;
    for frame in 0..120 {
        let previous = app.t;
        app.advance_room_tick(SECONDS, SECONDS, false);
        let advance = (app.t - previous).rem_euclid(1.0);
        peak = peak.max(advance);
        assert!(advance <= cap + 1e-12, "frame {frame}: {advance} > {cap}");
        let displayed = app.t;
        let bands = if frame % 2 == 0 { SILENT } else { BASS };
        let now = start + Duration::from_secs_f64(f64::from(frame) * SECONDS);
        app.compose_frame_with_bands(
            Raster::new(64, 48),
            64,
            48,
            Some((bands, VisualizerSource::OutputMix)),
            now,
        );
        assert_eq!(app.t, displayed, "composing a frame spent future phase");
    }
    assert!(
        (peak - cap).abs() < 1e-12,
        "the 8x ceiling remains reachable"
    );
}

#[test]
fn repeated_draws_carry_one_attack_into_the_next_tick() {
    let mut app = julia_room();
    let dt = 1.0 / 60.0;
    let now = Instant::now();
    let frame = |app: &mut App| {
        app.compose_frame_with_bands(
            Raster::new(64, 48),
            64,
            48,
            Some((BASS, VisualizerSource::OutputMix)),
            now,
        );
    };
    frame(&mut app);
    let attack = app.pending_visualizer_nudge;
    assert!(attack > 0.0);
    frame(&mut app);
    assert_eq!(app.t, 0.0);
    let base = numinous_core::ROOM_CYCLES_PER_SECOND * dt * app.visualizer_scale;
    app.advance_room_tick(dt, dt, false);
    assert!((app.t - base - attack).abs() < 1e-12);
    let previous = app.t;
    app.advance_room_tick(dt, dt, false);
    assert!((app.t - previous - base).abs() < 1e-12);
}

#[test]
fn a_held_tick_discards_its_music_nudge() {
    let mut app = julia_room();
    let dt = 1.0 / 60.0;
    let start = app.t;
    app.pending_visualizer_nudge = numinous_core::max_visualizer_phase_nudge();
    app.paused = true;
    app.advance_room_tick(dt, dt, false);
    assert_eq!(app.t, start);
    app.paused = false;
    app.advance_room_tick(dt, dt, false);
    let ordinary = numinous_core::ROOM_CYCLES_PER_SECOND * dt;
    assert!((app.t - start - ordinary).abs() < 1e-12);
    app.pending_visualizer_nudge = numinous_core::max_visualizer_phase_nudge();
    app.advance_room_tick(dt, dt, true);
    assert_eq!(app.pending_visualizer_nudge, 0.0);
    app.pending_visualizer_nudge = numinous_core::max_visualizer_phase_nudge();
    app.reset_room_runtime();
    assert_eq!(app.pending_visualizer_nudge, 0.0);
}

/// Exercise the shader readback and the overlaid, transformed App frame. The
/// control clears only the cooldown, retaining the same bands, meter and phase.
fn capture_gpu_beats(app: &mut App, paced: bool) -> TimelineFlashPeaks {
    const W: usize = 240;
    const H: usize = 140;
    let mut capture = RegionalFlashCapture::new(W, H).unwrap();
    let start = Instant::now();
    for frame in 0..=120 {
        if !paced {
            app.last_spectrum_poke = None;
        }
        let seconds = 1.0 / 60.0;
        app.advance_room_tick(seconds, seconds, false);
        let inputs = app.presented_room_inputs();
        let gpu_frame = app.gpu_frame(W, H).expect("Julia GPU readback");
        let room = &app.rooms[app.current];
        let mut raster = Raster::from_rgba(W, H, room.meta().accent, &gpu_frame).unwrap();
        super::input_feedback::draw(&mut raster, &inputs);
        app.draw_room_interface(&mut raster, room.as_ref(), &inputs, W, H);
        let bands = match frame % 4 {
            0 => BASS,
            2 => MID,
            _ => SILENT,
        };
        let now = start + Duration::from_secs_f64(f64::from(frame) * seconds);
        let (rgba, w, h) = app.compose_frame_with_bands(
            raster,
            W,
            H,
            Some((bands, VisualizerSource::OutputMix)),
            now,
        );
        assert!(capture.push(app.dissolve.frame(rgba, w, h)));
    }
    capture.timeline_peaks(60).unwrap()
}

#[test]
#[ignore = "requires a working GPU adapter for the Julia shader readback"]
fn moving_beats_in_gpu_julia_stay_within_the_sampled_flash_budget() {
    let mut renderer = Some(numinous_gpu::FractalRenderer::new().expect("GPU adapter"));
    eprintln!("GPU Julia on {}", renderer.as_ref().unwrap().adapter_name());
    for paced in [false, true] {
        let mut app = julia_room();
        app.gpu = renderer.take();
        app.era = Era::EightBit;
        app.t = 0.44;
        app.time_scale = numinous_core::MIN_TIME_SCALE;
        let peaks = capture_gpu_beats(&mut app, paced);
        eprintln!("paced={paced}, admitted={}, {peaks:?}", app.pokes.len());
        if paced {
            assert!((2..=6).contains(&app.pokes.len()));
            for peak in [
                peaks.whole_general,
                peaks.whole_red,
                peaks.general.rate,
                peaks.red.rate,
            ] {
                assert!(peak as f64 <= numinous_core::photosensitivity::MAX_FLASHES_PER_SECOND);
            }
        } else {
            // Keep the unpaced trace as a live comparison across adapters
            // without requiring a future palette to reproduce the old flash.
            assert_eq!(app.pokes.len(), numinous_core::MAX_ROOM_POKES);
        }
        renderer = app.gpu.take();
    }
}
