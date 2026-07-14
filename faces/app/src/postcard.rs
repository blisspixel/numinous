use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use numinous_core::{Era, Raster, Room, RoomInput};

pub(crate) const POSTCARD_SIZE: u32 = 900;
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

fn render_room_postcard_rgba(
    room: &dyn Room,
    phase: f64,
    inputs: &[RoomInput],
    era: Era,
) -> Vec<u8> {
    let mut raster = Raster::with_accent(
        POSTCARD_SIZE as usize,
        POSTCARD_SIZE as usize,
        room.meta().accent,
    );
    // The postcard's promise is the screen: the same gesture trail the live
    // frame renders from, so a held or just-flung room saves as seen.
    room.render_input(&mut raster, phase, inputs);
    let mut rgba = raster.to_rgba();
    era.apply(&mut rgba, POSTCARD_SIZE as usize, POSTCARD_SIZE as usize);
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

fn postcard_path(dir: &Path, room_id: &str, state_code: u64) -> PathBuf {
    dir.join(format!("numinous-{room_id}-{state_code:03}.png"))
}

fn postcard_collision_path(dir: &Path, room_id: &str, state_code: u64, serial: u16) -> PathBuf {
    dir.join(format!(
        "numinous-{room_id}-{state_code:03}-{serial:03}.png"
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
}
