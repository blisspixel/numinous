//! QA mirror: compose the app's screens exactly as `main.rs` draws them and
//! write PNGs, so every screen can be reviewed (and critiqued) headlessly.
//! Keep the layout constants in sync with `src/main.rs`; this exists because
//! the screen itself cannot be captured from an automation session.
//! Run: cargo run -p numinous-app --example screens

use std::fs::File;
use std::io::BufWriter;

use numinous_core::{Raster, Surface, all_rooms, draw_text, wrap_text};

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

fn main() {
    let (width, height) = (900usize, 900usize);
    let rooms = all_rooms();
    let scale = (width as i32 / 500).clamp(1, 3);

    // Screen 1: launch state, the help overlay over the first room.
    {
        let room = &rooms[0];
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, 0.12);
        raster.dim(22);
        let menu_scale = (width as i32 / 300).clamp(2, 4);
        let lines = [
            "PLAY",
            "G          THE QUIZ: NAME THE MATH",
            "C          MUNCH: EAT WHAT FITS",
            "N          NIM: BEAT THE ORDER",
            "T          THE GAUNTLET: ONE RUN",
            "",
            "WANDER",
            "A / D      PREV / NEXT ROOM    1-9 JUMP",
            "W / S      TIME SPEED   MOUSE  SCRUB",
            "E          INSPECT    Q  ERA    R  RESTART",
            "B          THE SHOW   TAB  THE STUDIO",
            "J          JOURNEY    F  FULLSCREEN",
            "Y          RADIO STATIONS    P  POSTCARD",
            "M          SOUND      SPACE  PAUSE",
            "",
            "ESC        CLOSE MENU AND WANDER",
        ];
        let line_height = 11 * menu_scale;
        let top = (height as i32 / 2) - (lines.len() as i32 * line_height) / 2;
        for (i, line) in lines.iter().enumerate() {
            draw_text(
                &mut raster,
                line,
                width as i32 / 8,
                top + i as i32 * line_height,
                menu_scale,
                '#',
            );
        }
        save(&raster, "renders/qa3/app-1-launch-help.png");
    }

    // Screen 2: the main view with title HUD and hint bar.
    {
        let room = &rooms[16]; // lorenz
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, 0.7);
        draw_text(
            &mut raster,
            &room.meta().title.to_uppercase(),
            10,
            10,
            scale + 1,
            '#',
        );
        // The arrival card under the title, as the app draws it.
        let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
        for (i, line) in wrap_text(&room.meta().blurb.to_uppercase(), columns)
            .iter()
            .take(3)
            .enumerate()
        {
            draw_text(
                &mut raster,
                line,
                10,
                10 + (2 + i as i32) * 9 * scale,
                scale,
                '#',
            );
        }
        draw_text(
            &mut raster,
            "(E FOR THE WHOLE STORY)",
            10,
            10 + 5 * 9 * scale,
            scale,
            '-',
        );
        let level = "LV 7";
        let lx = width as i32 - (level.len() as i32 * 6 * scale) - 10;
        draw_text(&mut raster, level, lx, 10, scale, '#');
        draw_text(
            &mut raster,
            "G QUIZ   C MUNCH   N NIM   T RUN   E INSPECT   ESC MENU",
            10,
            height as i32 - 10 * scale,
            scale,
            '-',
        );
        save(&raster, "renders/qa3/app-2-main.png");
    }

    // Screen 3: the reveal overlay (E, inspect) on the golden angle.
    {
        let room = &rooms[3];
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, 0.0);
        draw_text(
            &mut raster,
            &room.meta().title.to_uppercase(),
            10,
            10,
            scale + 1,
            '#',
        );
        let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
        let line_height = 9 * scale;
        for (i, line) in wrap_text(&room.reveal().to_uppercase(), columns)
            .iter()
            .enumerate()
        {
            draw_text(
                &mut raster,
                line,
                10,
                10 + (2 + i as i32) * line_height,
                scale,
                '#',
            );
        }
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
