//! QA mirror: compose the app's screens exactly as `main.rs` draws them and
//! write PNGs, so every screen can be reviewed (and critiqued) headlessly.
//! Keep the layout constants in sync with `src/main.rs`; this exists because
//! the screen itself cannot be captured from an automation session.
//! Run: cargo run -p numinous-app --example screens

use std::fs::File;
use std::io::BufWriter;

use numinous_core::{Raster, Surface, all_rooms, draw_text};

#[path = "../src/hud.rs"]
mod hud;
#[allow(dead_code)]
#[path = "../src/overlays.rs"]
mod overlays;

fn save(raster: &Raster, path: &str) {
    let (w, h) = (raster.width(), raster.height());
    let file = File::create(path).expect("create png");
    let mut encoder = png::Encoder::new(BufWriter::new(file), w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer
        .write_image_data(&raster.to_rgba())
        .expect("png data");
    println!("wrote {path}");
}

fn room_by_id<'a>(
    rooms: &'a [Box<dyn numinous_core::Room>],
    id: &str,
) -> &'a dyn numinous_core::Room {
    rooms
        .iter()
        .find(|room| room.meta().id == id)
        .map(Box::as_ref)
        .unwrap_or_else(|| panic!("missing room {id}"))
}

fn main() {
    let (width, height) = (900usize, 900usize);
    let rooms = all_rooms();
    // Screen 1: launch state, the help overlay over the first room.
    {
        let room = room_by_id(&rooms, "times-tables");
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, 0.12);
        overlays::draw_help_overlay(&mut raster, width, height);
        save(&raster, "renders/qa3/app-1-launch-help.png");
    }

    // Screen 2: the main view with title HUD and hint bar.
    {
        let room = room_by_id(&rooms, "lorenz");
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, 0.7);
        hud::draw_room_chrome(
            &mut raster,
            room,
            &hud::RoomChrome {
                t: 0.7,
                room_card: 240,
                show_info: false,
                show_help: false,
                show_journey: false,
                banner_active: false,
                the_show: false,
                studio: false,
                muted: false,
                level: 7,
            },
            width,
            height,
        );
        save(&raster, "renders/qa3/app-2-main.png");
    }

    // Screen 3: the reveal overlay (E, inspect) on the golden angle.
    {
        let room = room_by_id(&rooms, "golden-angle");
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, 0.0);
        hud::draw_room_chrome(
            &mut raster,
            room,
            &hud::RoomChrome {
                t: 0.0,
                room_card: 0,
                show_info: true,
                show_help: false,
                show_journey: false,
                banner_active: false,
                the_show: false,
                studio: false,
                muted: false,
                level: 1,
            },
            width,
            height,
        );
        save(&raster, "renders/qa3/app-3-inspect.png");
    }

    // Screen 4: the Studio, mid-composition.
    {
        use std::f64::consts::TAU;
        let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
        let sscale = (width as i32 / 500).clamp(1, 3);
        draw_text(&mut raster, "THE STUDIO", 10, 10, sscale, '-');
        let typed = "Y = SIN(A*X) + X/3_";
        draw_text(&mut raster, typed, 10, 10 + 12 * sscale, sscale + 1, '#');
        let expr = numinous_core::parse("sin(a*x) + x/3").expect("parse");
        let a = 0.35 * TAU;
        let (xmin, xmax) = (-TAU, TAU);
        let samples: Vec<(usize, f64)> = (0..width)
            .map(|i| {
                let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
                (i, numinous_core::eval(&expr, x, a))
            })
            .collect();
        let ymin = samples.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let ymax = samples
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        let yspan = (ymax - ymin).max(1e-9);
        let top = (60 * sscale) as f64;
        let plot_h = height as f64 - top - 12.0;
        let mut previous: Option<(i32, i32)> = None;
        for &(i, y) in &samples {
            let sx = i as i32;
            let sy = (top + (1.0 - (y - ymin) / yspan) * plot_h) as i32;
            if let Some((px, py)) = previous {
                raster.line(px, py, sx, sy, '#');
            }
            previous = Some((sx, sy));
        }
        save(&raster, "renders/qa3/app-4-studio.png");
    }
}
