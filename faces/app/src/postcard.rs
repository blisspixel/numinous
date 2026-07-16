use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use numinous_core::rooms::game_of_life::LifeSession;
use numinous_core::{Era, Raster, Room, RoomInput};

pub(crate) const POSTCARD_SIZE: u32 = 900;
/// Share loops are smaller than still postcards so a short animated APNG stays
/// a practical file size while remaining readable in a feed.
pub(crate) const LOOP_SIZE: u32 = 480;
/// Twenty-four frames at 12 fps is a two-second cycle: long enough to read
/// motion, short enough to encode on a keypress.
pub(crate) const LOOP_FRAMES: u32 = 24;
/// Frame delay numerator/denominator in seconds (1/12 s per frame).
const LOOP_DELAY_NUM: u16 = 1;
const LOOP_DELAY_DEN: u16 = 12;
const MAX_POSTCARD_COLLISIONS: u16 = 999;

pub(crate) fn default_postcard_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
}

pub(crate) fn write_room_postcard(
    room: &dyn Room,
    phase: f64,
    inputs: &[RoomInput],
    era: Era,
    dir: &Path,
) -> std::io::Result<PathBuf> {
    let rgba = render_room_postcard_rgba(room, phase, inputs, era);
    write_rendered_postcard(room.meta().id, u64::from(phase_code(phase)), &rgba, dir)
}

pub(crate) fn write_rendered_postcard(
    room_id: &str,
    state_code: u64,
    rgba: &[u8],
    dir: &Path,
) -> std::io::Result<PathBuf> {
    let encoded = encode_rgba_png(POSTCARD_SIZE, POSTCARD_SIZE, rgba)?;
    for path in postcard_paths(dir, room_id, state_code) {
        match write_png_file(&path, &encoded) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "all postcard filenames for this room and state already exist",
    ))
}

/// Export one full phase cycle of the current visit as a looping APNG.
///
/// Gesture history and Visual Era match the still postcard path so a short
/// loop is the same visit the player is watching, not a clean re-deal.
pub(crate) fn write_room_loop(
    room: &dyn Room,
    start_phase: f64,
    inputs: &[RoomInput],
    era: Era,
    dir: &Path,
) -> std::io::Result<PathBuf> {
    let frames = render_room_loop_frames(room, start_phase, inputs, era);
    write_rendered_loop(
        room.meta().id,
        u64::from(phase_code(start_phase)),
        &frames,
        dir,
    )
}

/// Export advancing generations from a Life visit without mutating the live
/// session. The first frame is the current generation; each later frame is
/// one B3/S23 step so the loop is the causal universe, not a phase scrub.
pub(crate) fn write_life_loop(
    room_id: &str,
    accent: [u8; 3],
    session: &LifeSession,
    era: Era,
    dir: &Path,
) -> std::io::Result<PathBuf> {
    let frames = render_life_loop_frames(session, accent, era);
    write_rendered_loop(room_id, session.generation(), &frames, dir)
}

pub(crate) fn write_rendered_loop(
    room_id: &str,
    state_code: u64,
    frames: &[Vec<u8>],
    dir: &Path,
) -> std::io::Result<PathBuf> {
    if frames.len() != LOOP_FRAMES as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "short loop requires exactly {LOOP_FRAMES} frames, got {}",
                frames.len()
            ),
        ));
    }
    let expected = (LOOP_SIZE as usize) * (LOOP_SIZE as usize) * 4;
    for (index, frame) in frames.iter().enumerate() {
        if frame.len() != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "short loop frame {index} has {} bytes, expected {expected}",
                    frame.len()
                ),
            ));
        }
    }
    let encoded = encode_rgba_apng(LOOP_SIZE, LOOP_SIZE, frames)?;
    for path in loop_paths(dir, room_id, state_code) {
        match write_png_file(&path, &encoded) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "all short-loop filenames for this room and state already exist",
    ))
}

fn render_room_postcard_rgba(
    room: &dyn Room,
    phase: f64,
    inputs: &[RoomInput],
    era: Era,
) -> Vec<u8> {
    render_room_rgba(room, phase, inputs, era, POSTCARD_SIZE)
}

fn render_room_loop_frames(
    room: &dyn Room,
    start_phase: f64,
    inputs: &[RoomInput],
    era: Era,
) -> Vec<Vec<u8>> {
    let start = if start_phase.is_finite() {
        start_phase
    } else {
        0.0
    };
    (0..LOOP_FRAMES)
        .map(|index| {
            let phase = start + f64::from(index) / f64::from(LOOP_FRAMES);
            render_room_rgba(room, phase, inputs, era, LOOP_SIZE)
        })
        .collect()
}

fn render_life_loop_frames(session: &LifeSession, accent: [u8; 3], era: Era) -> Vec<Vec<u8>> {
    let mut session = session.clone();
    let size = LOOP_SIZE as usize;
    let mut frames = Vec::with_capacity(LOOP_FRAMES as usize);
    for _ in 0..LOOP_FRAMES {
        let mut raster = Raster::with_accent(size, size, accent);
        session.render(&mut raster);
        let mut rgba = raster.to_rgba();
        era.apply(&mut rgba, size, size);
        frames.push(rgba);
        session.advance();
    }
    frames
}

fn render_room_rgba(
    room: &dyn Room,
    phase: f64,
    inputs: &[RoomInput],
    era: Era,
    size: u32,
) -> Vec<u8> {
    let dim = size as usize;
    let mut raster = Raster::with_accent(dim, dim, room.meta().accent);
    // Share captures the live visit: the same gesture trail the window
    // renders from, so a held or just-flung room saves as seen.
    room.render_input(&mut raster, phase, inputs);
    let mut rgba = raster.to_rgba();
    era.apply(&mut rgba, dim, dim);
    rgba
}

fn postcard_paths<'a>(
    dir: &'a Path,
    room_id: &'a str,
    state_code: u64,
) -> impl Iterator<Item = PathBuf> + 'a {
    std::iter::once(postcard_path(dir, room_id, state_code)).chain(
        (1..=MAX_POSTCARD_COLLISIONS)
            .map(move |serial| postcard_collision_path(dir, room_id, state_code, serial)),
    )
}

fn loop_paths<'a>(
    dir: &'a Path,
    room_id: &'a str,
    state_code: u64,
) -> impl Iterator<Item = PathBuf> + 'a {
    std::iter::once(loop_path(dir, room_id, state_code)).chain(
        (1..=MAX_POSTCARD_COLLISIONS)
            .map(move |serial| loop_collision_path(dir, room_id, state_code, serial)),
    )
}

/// Keep share filenames inside the destination directory even if a room id
/// ever carried separators or shell-hostile characters.
fn filename_safe_id(room_id: &str) -> String {
    // Strip leading dots first so a id like "..hidden" does not become a
    // leading-underscore artifact after character filtering.
    let stripped = room_id.trim_start_matches('.');
    let safe: String = stripped
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "room".to_string()
    } else {
        safe
    }
}

fn postcard_path(dir: &Path, room_id: &str, state_code: u64) -> PathBuf {
    let id = filename_safe_id(room_id);
    dir.join(format!("numinous-{id}-{state_code:03}.png"))
}

fn postcard_collision_path(dir: &Path, room_id: &str, state_code: u64, serial: u16) -> PathBuf {
    let id = filename_safe_id(room_id);
    dir.join(format!("numinous-{id}-{state_code:03}-{serial:03}.png"))
}

fn loop_path(dir: &Path, room_id: &str, state_code: u64) -> PathBuf {
    let id = filename_safe_id(room_id);
    dir.join(format!("numinous-{id}-loop-{state_code:03}.png"))
}

fn loop_collision_path(dir: &Path, room_id: &str, state_code: u64, serial: u16) -> PathBuf {
    let id = filename_safe_id(room_id);
    dir.join(format!(
        "numinous-{id}-loop-{state_code:03}-{serial:03}.png"
    ))
}

fn phase_code(phase: f64) -> u32 {
    if phase.is_finite() {
        (phase.clamp(0.0, 9.99) * 100.0) as u32
    } else {
        0
    }
}

fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoded = Vec::new();
    let mut encoder = png::Encoder::new(&mut encoded, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(png_error)?;
    writer.write_image_data(rgba).map_err(png_error)?;
    drop(writer);
    Ok(encoded)
}

fn encode_rgba_apng(width: u32, height: u32, frames: &[Vec<u8>]) -> std::io::Result<Vec<u8>> {
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // Fast compression keeps a 24-frame keypress export responsive.
        encoder.set_compression(png::Compression::Fast);
        encoder
            .set_animated(frames.len() as u32, 0)
            .map_err(png_error)?;
        encoder
            .set_frame_delay(LOOP_DELAY_NUM, LOOP_DELAY_DEN)
            .map_err(png_error)?;
        encoder
            .set_dispose_op(png::DisposeOp::Background)
            .map_err(png_error)?;
        let mut writer = encoder.write_header().map_err(png_error)?;
        for frame in frames {
            writer.write_image_data(frame).map_err(png_error)?;
        }
        writer.finish().map_err(png_error)?;
    }
    Ok(encoded)
}

fn write_png_file(path: &Path, encoded: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(encoded)
}

fn png_error(error: png::EncodingError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_codes_are_stable_and_filename_safe() {
        assert_eq!(phase_code(0.0), 0);
        assert_eq!(phase_code(0.42), 42);
        assert_eq!(phase_code(1.0), 100);
        assert_eq!(phase_code(-1.0), 0);
        assert_eq!(phase_code(f64::NAN), 0);
        assert_eq!(phase_code(f64::INFINITY), 0);
        assert_eq!(phase_code(100.0), 999);
    }

    #[test]
    fn postcard_path_uses_room_id_and_state_code() {
        let path = postcard_path(Path::new("out"), "times_tables", 42);
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("numinous-times_tables-042.png")
        );
        let collision = postcard_collision_path(Path::new("out"), "times_tables", 42, 7);
        assert_eq!(
            collision.file_name().and_then(|name| name.to_str()),
            Some("numinous-times_tables-042-007.png")
        );
    }

    #[test]
    fn share_filenames_reject_path_separators_and_dots() {
        let hostile = postcard_path(Path::new("out"), "../evil/room", 1);
        assert_eq!(
            hostile.file_name().and_then(|name| name.to_str()),
            Some("numinous-_evil_room-001.png")
        );
        assert_eq!(hostile.parent(), Some(Path::new("out")));
        let dotted = loop_path(Path::new("out"), "..hidden", 2);
        assert_eq!(
            dotted.file_name().and_then(|name| name.to_str()),
            Some("numinous-hidden-loop-002.png")
        );
        assert_eq!(filename_safe_id(""), "room");
        assert_eq!(filename_safe_id("..."), "room");
    }

    #[test]
    fn postcard_render_includes_pokes_and_visual_era() {
        let rooms = numinous_core::all_rooms();
        let room = rooms
            .iter()
            .find(|room| room.meta().id == "julia")
            .expect("julia room exists")
            .as_ref();

        let touch = [RoomInput::PointerDown {
            x: 0.9,
            y: 0.1,
            t: 0.35,
        }];
        let plain = render_room_postcard_rgba(room, 0.35, &[], Era::Modern);
        let poked = render_room_postcard_rgba(room, 0.35, &touch, Era::Modern);
        let phosphor = render_room_postcard_rgba(room, 0.35, &touch, Era::Phosphor);

        assert_ne!(plain, poked, "postcards should preserve placed hands");
        assert_ne!(
            poked, phosphor,
            "postcards should preserve the selected visual era"
        );
    }

    #[test]
    fn write_room_postcard_creates_a_valid_png() {
        let rooms = numinous_core::all_rooms();
        let room = rooms.first().expect("at least one room").as_ref();
        let dir = std::env::temp_dir().join("numinous_postcard_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create postcard dir");

        let touch = [RoomInput::PointerDown {
            x: 0.25,
            y: 0.75,
            t: 0.25,
        }];
        let path =
            write_room_postcard(room, 0.25, &touch, Era::Modern, &dir).expect("write postcard");
        let second_path = write_room_postcard(room, 0.25, &touch, Era::Modern, &dir)
            .expect("write colliding postcard");
        assert_ne!(
            path, second_path,
            "postcard writes should create a fresh file instead of replacing the previous frame"
        );
        let file = std::fs::File::open(&path).expect("open postcard");
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let mut reader = decoder.read_info().expect("read png header");
        let info = reader.info();

        assert_eq!(info.width, POSTCARD_SIZE);
        assert_eq!(info.height, POSTCARD_SIZE);
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);

        let mut frame = vec![0; reader.output_buffer_size()];
        let output = reader.next_frame(&mut frame).expect("decode postcard");
        let pixels = &frame[..output.buffer_size()];
        assert_eq!(output.width, POSTCARD_SIZE);
        assert_eq!(output.height, POSTCARD_SIZE);
        assert_eq!(pixels.len(), (POSTCARD_SIZE * POSTCARD_SIZE * 4) as usize);
        assert!(
            pixels
                .chunks_exact(4)
                .any(|pixel| pixel != [10, 11, 15, 255]),
            "postcard should contain room ink, not only the background"
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(second_path);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn loop_path_names_room_and_state() {
        let path = loop_path(Path::new("out"), "times_tables", 42);
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("numinous-times_tables-loop-042.png")
        );
        let collision = loop_collision_path(Path::new("out"), "times_tables", 42, 3);
        assert_eq!(
            collision.file_name().and_then(|name| name.to_str()),
            Some("numinous-times_tables-loop-042-003.png")
        );
    }

    #[test]
    fn write_room_loop_creates_a_multi_frame_apng() {
        let rooms = numinous_core::all_rooms();
        let room = rooms
            .iter()
            .find(|room| room.meta().id == "times-tables")
            .or_else(|| rooms.first())
            .expect("at least one room")
            .as_ref();
        let dir = std::env::temp_dir().join("numinous_short_loop_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create loop dir");

        let touch = [RoomInput::PointerDown {
            x: 0.4,
            y: 0.6,
            t: 0.1,
        }];
        let path = write_room_loop(room, 0.1, &touch, Era::Modern, &dir).expect("write loop");
        let second =
            write_room_loop(room, 0.1, &touch, Era::Modern, &dir).expect("write colliding loop");
        assert_ne!(path, second, "loop writes must not overwrite");

        let file = std::fs::File::open(&path).expect("open loop");
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let mut reader = decoder.read_info().expect("read loop header");
        let info = reader.info();
        assert_eq!(info.width, LOOP_SIZE);
        assert_eq!(info.height, LOOP_SIZE);
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let animation = info
            .animation_control
            .expect("short loop is an animated PNG");
        assert_eq!(animation.num_frames, LOOP_FRAMES);
        assert_eq!(animation.num_plays, 0, "loops play indefinitely");

        let mut frame = vec![0; reader.output_buffer_size()];
        let first = reader.next_frame(&mut frame).expect("decode first frame");
        assert_eq!(first.width, LOOP_SIZE);
        assert!(
            frame[..first.buffer_size()]
                .chunks_exact(4)
                .any(|pixel| pixel != [10, 11, 15, 255]),
            "loop frame should contain room ink"
        );

        let plain = render_room_loop_frames(room, 0.0, &[], Era::Modern);
        assert_eq!(plain.len(), LOOP_FRAMES as usize);
        assert_ne!(
            plain[0],
            plain[LOOP_FRAMES as usize / 2],
            "untouched phase sweep should change at least one mid-cycle frame"
        );
        let held = render_room_loop_frames(room, 0.1, &touch, Era::Modern);
        assert_ne!(
            held[0], plain[0],
            "short loops should preserve placed hands on the opening frame"
        );
        // Times Tables freezes K under a held dial, so a shared loop of a held
        // visit is honest motion-of-state, not a re-deal of the ambient sweep.
        assert_eq!(
            held[0],
            held[LOOP_FRAMES as usize / 2],
            "held dial visits should keep the hand-chosen multiplier across frames"
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(second);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn life_loop_advances_without_mutating_the_source_session() {
        let mut session = LifeSession::new(0);
        assert!(session.launch((0.5, 0.5)));
        let before_gen = session.generation();
        let before_launches = session.launches();

        let dir = std::env::temp_dir().join("numinous_life_loop_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create life loop dir");
        let path = write_life_loop("game-of-life", [40, 210, 90], &session, Era::Modern, &dir)
            .expect("write life loop");

        assert_eq!(session.generation(), before_gen);
        assert_eq!(session.launches(), before_launches);

        let file = std::fs::File::open(&path).expect("open life loop");
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let reader = decoder.read_info().expect("read life loop header");
        let animation = reader
            .info()
            .animation_control
            .expect("life loop is animated");
        assert_eq!(animation.num_frames, LOOP_FRAMES);

        let frames = render_life_loop_frames(&session, [40, 210, 90], Era::Modern);
        assert_ne!(
            frames[0], frames[1],
            "Life short loop should show generation motion"
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(dir);
    }
}
