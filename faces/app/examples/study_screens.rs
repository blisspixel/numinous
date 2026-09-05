//! Deterministic visual review frames for the optional room study reader.
//!
//! Run from the workspace root:
//! `cargo run -p numinous-app --example study_screens`.
//! The example writes `renders/study/`: requested English and Japanese, all
//! three depths, both supported sizes, and explicit unavailable-depth/language
//! cases. Each case records the first and last viewport using the live reader.
//! The JSON inventory distinguishes requested, document and block languages.

use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use numinous_app::input_legend::{ControllerCopy, InputMode};
use numinous_app::study_reader::{ReaderCommand, StudyReader};
use numinous_core::{StudyDepth, StudyLocale, room_by_id};
use serde_json::{Value, json};

type CaptureResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug)]
struct Case {
    room: &'static str,
    locale: &'static str,
    depth: StudyDepth,
    size: (u32, u32),
}

fn capture(case: Case, directory: &Path) -> CaptureResult<Vec<Value>> {
    let room = room_by_id(case.room)
        .ok_or_else(|| io::Error::other(format!("missing room {}", case.room)))?;
    let locale = StudyLocale::parse(case.locale)?;
    let mut reader = StudyReader::new(room.as_ref(), &locale)?;
    reader.navigate(ReaderCommand::Select(case.depth));
    let (width, height) = case.size;
    let mut frames = Vec::with_capacity(2);
    for (position, command) in [("start", ReaderCommand::Start), ("end", ReaderCommand::End)] {
        // The first draw establishes measured document geometry before End.
        let _ = reader.render(
            width,
            height,
            InputMode::KeyboardMouse,
            ControllerCopy::default(),
        )?;
        reader.navigate(command);
        let rgba = reader.render(
            width,
            height,
            InputMode::KeyboardMouse,
            ControllerCopy::default(),
        )?;
        let repeat = reader.render(
            width,
            height,
            InputMode::KeyboardMouse,
            ControllerCopy::default(),
        )?;
        if rgba != repeat {
            return Err(
                io::Error::other("an unchanged study reader produced different pixels").into(),
            );
        }
        let filename = format!(
            "{}-{}-{}-{width}x{height}-{position}.png",
            case.room,
            case.locale,
            case.depth.as_str()
        );
        let file = File::create(directory.join(&filename))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgba)?;
        writer.finish()?;
        let document = reader.document();
        frames.push(json!({
            "file": &filename,
            "room": case.room,
            "requested_locale": document.locale.requested.as_str(),
            "document_locale": document.locale.resolved,
            "block_locales": document.blocks_at(case.depth).map(|block| block.locale.resolved).collect::<Vec<_>>(),
            "depth": case.depth.as_str(),
            "depth_available": document.has_depth(case.depth),
            "width": width,
            "height": height,
            "position": position,
            "scroll_pixels": reader.scroll(),
            "input": "keyboard_mouse",
        }));
        println!("wrote {}", directory.join(filename).display());
    }
    Ok(frames)
}

fn main() -> CaptureResult<()> {
    let directory = Path::new("renders/study");
    std::fs::create_dir_all(directory)?;
    let mut frames = Vec::new();
    for size in [(360, 240), (900, 700)] {
        for locale in ["en", "ja"] {
            for depth in StudyDepth::ALL {
                frames.extend(capture(
                    Case {
                        room: "lissajous",
                        locale,
                        depth,
                        size,
                    },
                    directory,
                )?);
            }
        }
        for (room, locale, depth) in [
            ("times-tables", "en", StudyDepth::Mathematics),
            ("times-tables", "ja", StudyDepth::Explanation),
            ("lissajous", "haw", StudyDepth::Mathematics),
        ] {
            frames.extend(capture(
                Case {
                    room,
                    locale,
                    depth,
                    size,
                },
                directory,
            )?);
        }
    }
    let mut inventory = BufWriter::new(File::create(directory.join("index.json"))?);
    serde_json::to_writer_pretty(
        &mut inventory,
        &json!({
            "schema": "numinous.study-screens.v1",
            "version": env!("CARGO_PKG_VERSION"),
            "frames": frames,
        }),
    )?;
    inventory.write_all(b"\n")?;
    inventory.flush()?;
    Ok(())
}
