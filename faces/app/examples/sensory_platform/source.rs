use numinous_core::{Era, Raster, RoomInput, room_by_id_with};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::{hud, input_feedback, input_legend};

const FRAME_PHASE: f64 = 0.375;
const FRAME_VARIATION: u64 = 17;
const ROOM_ID: &str = "times-tables";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceReceipt {
    pub(super) room: &'static str,
    pub(super) variation: u64,
    pub(super) phase: f64,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) byte_length: usize,
    pub(super) lit_pixels: usize,
    pub(super) all_alpha_opaque: bool,
    pub(super) first_render_sha256: String,
    pub(super) repeat_render_sha256: String,
    pub(super) deterministic: bool,
    pub(super) components: [&'static str; 6],
}

pub(crate) struct SourceFrame {
    pub(crate) rgba: Vec<u8>,
    pub(crate) receipt: SourceReceipt,
}

pub(crate) fn compose_source(width: u32, height: u32) -> Result<SourceFrame, String> {
    let first = draw_source(width, height)?;
    let repeat = draw_source(width, height)?;
    Ok(SourceFrame {
        receipt: SourceReceipt {
            room: ROOM_ID,
            variation: FRAME_VARIATION,
            phase: FRAME_PHASE,
            width,
            height,
            byte_length: first.0.len(),
            lit_pixels: first.1,
            all_alpha_opaque: first.0.chunks_exact(4).all(|pixel| pixel[3] == 255),
            first_render_sha256: sha256_bytes(&first.0),
            repeat_render_sha256: sha256_bytes(&repeat.0),
            deterministic: first.0 == repeat.0,
            components: [
                "core room render_input",
                "App input feedback",
                "App room chrome",
                "App audio badge",
                "App spectrum meter",
                "core Modern era transform",
            ],
        },
        rgba: first.0,
    })
}

fn draw_source(width: u32, height: u32) -> Result<(Vec<u8>, usize), String> {
    let (width_usize, height_usize) = (width as usize, height as usize);
    let room = room_by_id_with(ROOM_ID, FRAME_VARIATION)
        .ok_or_else(|| format!("missing proof room {ROOM_ID}"))?;
    let inputs = [RoomInput::Key { ch: '5' }];
    let mut raster = Raster::with_accent(width_usize, height_usize, room.meta().accent);
    room.render_input(&mut raster, FRAME_PHASE, &inputs);
    input_feedback::draw(&mut raster, &inputs);
    hud::draw_room_chrome(
        &mut raster,
        room.as_ref(),
        &hud::RoomChrome {
            t: FRAME_PHASE,
            room_card: 0,
            show_info: false,
            show_help: false,
            show_journey: false,
            banner_active: false,
            the_show: false,
            studio: false,
            muted: false,
            level: 7,
            input_mode: input_legend::InputMode::KeyboardMouse,
            controller_face: input_legend::ControllerFace::Generic.into(),
        },
        &inputs,
        None,
        width_usize,
        height_usize,
    );
    hud::draw_audio_state(
        &mut raster,
        &hud::AudioState::new(hud::AudioSource::RoomScore, 45, false, true),
        width_usize,
    );
    hud::draw_spectrum_meter(
        &mut raster,
        &[0.15, 0.35, 0.70, 0.45, 0.25, 0.10, 0.05],
        width_usize,
        height_usize,
    );
    let lit_pixels = raster.lit_count();
    let mut rgba = raster.to_rgba();
    Era::Modern.apply(&mut rgba, width_usize, height_usize);
    Ok((rgba, lit_pixels))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex_digest(&Sha256::digest(bytes))
}

pub(super) fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::compose_source;

    #[test]
    fn proof_source_repeats_the_fully_composed_app_frame() {
        let source = compose_source(160, 120).expect("compose proof source");
        assert!(source.receipt.deterministic);
        assert_eq!(
            source.receipt.first_render_sha256,
            source.receipt.repeat_render_sha256
        );
        assert!(source.receipt.lit_pixels >= 100);
        assert!(source.receipt.all_alpha_opaque);
        assert_eq!(source.rgba.len(), 160 * 120 * 4);
    }
}
