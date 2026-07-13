//! Headless visual QA matrix for the windowed app.
//!
//! The matrix exercises every catalog room before and after interaction, every
//! app game state, overlays, progression, reset flow, and small viewports using
//! the same room, HUD, overlay, and game drawing modules as the live app.
//! Run: `cargo run -p numinous-app --example screens`.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use numinous_core::{Journey, Raster, Room, RoomInput, Scoreboard, Surface, all_rooms};

#[allow(dead_code)]
#[path = "../src/feedback.rs"]
mod feedback;
#[allow(dead_code)]
#[path = "../src/game_draw.rs"]
mod game_draw;
#[path = "../src/hud.rs"]
mod hud;
#[allow(dead_code)]
#[path = "../src/overlays.rs"]
mod overlays;
#[allow(dead_code)]
#[path = "../src/play.rs"]
mod play;
#[allow(dead_code)]
#[path = "../src/studio_panel.rs"]
mod studio_panel;

const OUTPUT: &str = "renders/qa-app";
const DEFAULT_SIZE: (usize, usize) = (900, 700);
const ROOM_SIZE: (usize, usize) = (640, 480);
const SMALL_SIZE: (usize, usize) = (360, 240);

fn save(raster: &Raster, relative: &str, manifest: &mut Vec<String>) {
    assert_eq!(
        (raster.width(), raster.height()),
        expected_dimensions(relative),
        "{relative} has its declared dimensions"
    );
    assert!(raster.lit_count() > 20, "{relative} is not a blank screen");
    let path = Path::new(OUTPUT).join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create screenshot directory");
    }
    let file = File::create(&path).expect("create png");
    let mut encoder = png::Encoder::new(
        BufWriter::new(file),
        raster.width() as u32,
        raster.height() as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer
        .write_image_data(&raster.to_rgba())
        .expect("png data");
    manifest.push(relative.replace('\\', "/"));
    println!("wrote {}", path.display());
}

fn expected_dimensions(relative: &str) -> (usize, usize) {
    match relative.split('/').next() {
        Some("rooms") => {
            if relative.contains("-small-") {
                SMALL_SIZE
            } else {
                ROOM_SIZE
            }
        }
        Some("games" | "overlays") => {
            if relative.contains("-small-") {
                SMALL_SIZE
            } else {
                DEFAULT_SIZE
            }
        }
        Some("flows") => DEFAULT_SIZE,
        _ => panic!("unknown QA capture category: {relative}"),
    }
}

fn expected_paths(rooms: &[Box<dyn Room>]) -> BTreeSet<String> {
    assert_eq!(rooms.len(), 31, "current catalog size");
    let mut expected = BTreeSet::new();
    for room in rooms {
        let id = room.meta().id;
        expected.extend([
            format!("rooms/{id}-base-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!("rooms/{id}-arrival-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!("rooms/{id}-interacted-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!("rooms/{id}-delayed-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            format!(
                "rooms/{id}-arrival-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            format!(
                "rooms/{id}-interacted-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
        ]);
    }
    for index in 0..4 {
        expected.insert(format!(
            "flows/times-tables-phase-{index}-{}x{}.png",
            DEFAULT_SIZE.0, DEFAULT_SIZE.1
        ));
    }
    expected.extend([
        "flows/mandelbrot-before-reset.png".to_string(),
        "flows/mandelbrot-after-reset.png".to_string(),
    ]);
    for name in [
        "launch-help",
        "room-inspect",
        "journey-level-42",
        "level-up-banner",
    ] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!("overlays/{name}-{label}-{}x{}.png", size.0, size.1));
        }
    }
    for phase in ["arrival", "departure"] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!(
                "overlays/the-show-{phase}-{label}-{}x{}.png",
                size.0, size.1
            ));
        }
    }
    for name in [
        "studio",
        "quiz-question",
        "quiz-correct",
        "quiz-wrong",
        "munch-play",
        "munch-result",
        "arcade-live",
        "arcade-caught",
        "arcade-clear",
        "arcade-over",
        "nim-live",
        "nim-win",
        "nim-loss",
    ] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!("games/{name}-{label}-{}x{}.png", size.0, size.1));
        }
    }
    for stage in 0..=4 {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            expected.insert(format!(
                "games/gauntlet-stage-{stage}-{label}-{}x{}.png",
                size.0, size.1
            ));
        }
    }
    assert_eq!(expected.len(), 240, "documented QA scenario count");
    expected
}

fn changed_pixels(before: &Raster, after: &Raster) -> usize {
    before
        .to_rgba()
        .chunks_exact(4)
        .zip(after.to_rgba().chunks_exact(4))
        .filter(|(left, right)| left != right)
        .count()
}

fn room_by_id<'a>(rooms: &'a [Box<dyn Room>], id: &str) -> &'a dyn Room {
    rooms
        .iter()
        .find(|room| room.meta().id == id)
        .map(Box::as_ref)
        .unwrap_or_else(|| panic!("missing room {id}"))
}

fn room_screen(
    room: &dyn Room,
    t: f64,
    inputs: &[RoomInput],
    size: (usize, usize),
    room_card: u64,
    show_info: bool,
    level: u32,
) -> Raster {
    let (width, height) = size;
    let mut raster = room_content(room, t, inputs, size);
    hud::draw_room_chrome(
        &mut raster,
        room,
        &hud::RoomChrome {
            t,
            room_card,
            show_info,
            show_help: false,
            show_journey: false,
            banner_active: false,
            the_show: false,
            studio: false,
            muted: false,
            level,
        },
        inputs,
        width,
        height,
    );
    raster
}

fn room_content(room: &dyn Room, t: f64, inputs: &[RoomInput], size: (usize, usize)) -> Raster {
    let mut raster = Raster::with_accent(size.0, size.1, room.meta().accent);
    room.render_input(&mut raster, t, inputs);
    raster
}

fn show_screen(room: &dyn Room, t: f64, size: (usize, usize)) -> Raster {
    let (width, height) = size;
    let mut raster = Raster::with_accent(width, height, room.meta().accent);
    room.render(&mut raster, t);
    hud::draw_room_chrome(
        &mut raster,
        room,
        &hud::RoomChrome {
            t,
            room_card: 0,
            show_info: false,
            show_help: false,
            show_journey: false,
            banner_active: false,
            the_show: true,
            studio: false,
            muted: false,
            level: 7,
        },
        &[],
        width,
        height,
    );
    raster
}

fn save_sizes(
    name: &str,
    manifest: &mut Vec<String>,
    mut draw: impl FnMut(usize, usize) -> Raster,
) {
    for (label, (width, height)) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
        save(
            &draw(width, height),
            &format!("games/{name}-{label}-{width}x{height}.png"),
            manifest,
        );
    }
}

fn studio_screen(width: usize, height: usize) -> Raster {
    let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
    studio_panel::StudioPanel::default().draw(&mut raster, width, height, 0.35);
    raster
}

fn gauntlet(seed: u64) -> play::GauntletPlay {
    play::GauntletPlay {
        seed,
        stage: 0,
        munch: play::MunchPlay {
            board: numinous_core::build_board(seed, 0),
            seed,
            round: 0,
            cursor: 0,
            bites: BTreeSet::new(),
            graded: None,
        },
        quiz: play::QuizPlay {
            round: numinous_core::build_round(seed, 1, 44, 18),
            flash: None,
        },
        scan: numinous_core::build_scan(seed, 4),
        secret: numinous_core::secret_code(seed ^ 0x0000_6A17_0000_0B0B, 4),
        wire: "314".to_string(),
        wire_lines: vec!["1 LOCKED  1 LOOSE".to_string()],
        scores: vec![80, 100, 60, 50],
        cleared: vec![true, true, false, true],
        message: "STAGE REVIEW  COMBO AND CONSEQUENCES STAY VISIBLE".to_string(),
    }
}

fn main() {
    let output = Path::new(OUTPUT);
    if output.exists() {
        std::fs::remove_dir_all(output).expect("remove stale screenshot matrix");
    }
    let rooms = all_rooms();
    let mut manifest = Vec::new();

    for room in &rooms {
        let id = room.meta().id;
        let phase = 0.0;
        let immediate = [RoomInput::PointerDown {
            x: 0.53,
            y: 0.47,
            t: phase,
        }];
        let delayed_phase = (phase + 0.2).min(1.0);
        let gesture = [
            RoomInput::PointerDown {
                x: 0.35,
                y: 0.35,
                t: phase,
            },
            RoomInput::PointerMove {
                x: 0.53,
                y: 0.47,
                t: (phase + 0.08).min(1.0),
            },
            RoomInput::PointerMove {
                x: 0.70,
                y: 0.60,
                t: (phase + 0.12).min(1.0),
            },
            RoomInput::PointerUp {
                x: 0.70,
                y: 0.60,
                t: (phase + 0.14).min(1.0),
            },
        ];
        let raw_base = room_content(room.as_ref(), phase, &[], ROOM_SIZE);
        let raw_interacted = room_content(room.as_ref(), phase, &immediate, ROOM_SIZE);
        let raw_delayed_base = room_content(room.as_ref(), delayed_phase, &[], ROOM_SIZE);
        let raw_delayed = room_content(room.as_ref(), delayed_phase, &gesture, ROOM_SIZE);
        let base = room_screen(room.as_ref(), phase, &[], ROOM_SIZE, 0, false, 7);
        let interacted = room_screen(room.as_ref(), phase, &immediate, ROOM_SIZE, 0, false, 7);
        let delayed = room_screen(
            room.as_ref(),
            delayed_phase,
            &gesture,
            ROOM_SIZE,
            0,
            false,
            7,
        );
        assert!(
            changed_pixels(&raw_base, &raw_interacted) >= 100,
            "{id} immediate room-content response must be legible"
        );
        assert!(
            changed_pixels(&raw_delayed_base, &raw_delayed) >= 100,
            "{id} delayed room-content gesture response must be legible"
        );
        save(
            &base,
            &format!("rooms/{id}-base-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &room_screen(room.as_ref(), phase, &[], ROOM_SIZE, 240, false, 7),
            &format!("rooms/{id}-arrival-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &interacted,
            &format!("rooms/{id}-interacted-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &delayed,
            &format!("rooms/{id}-delayed-{}x{}.png", ROOM_SIZE.0, ROOM_SIZE.1),
            &mut manifest,
        );
        save(
            &room_screen(room.as_ref(), phase, &[], SMALL_SIZE, 240, false, 7),
            &format!(
                "rooms/{id}-arrival-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            &mut manifest,
        );
        save(
            &room_screen(
                room.as_ref(),
                delayed_phase,
                &gesture,
                SMALL_SIZE,
                0,
                false,
                7,
            ),
            &format!(
                "rooms/{id}-interacted-small-{}x{}.png",
                SMALL_SIZE.0, SMALL_SIZE.1
            ),
            &mut manifest,
        );
    }
    assert_eq!(manifest.len(), rooms.len() * 6, "six states per room");

    let times = room_by_id(&rooms, "times-tables");
    for (index, phase) in [0.0, 0.24, 0.51, 0.88].into_iter().enumerate() {
        save(
            &room_screen(times, phase, &[], (900, 700), 0, false, 7),
            &format!("flows/times-tables-phase-{index}-900x700.png"),
            &mut manifest,
        );
    }

    let mandelbrot = room_by_id(&rooms, "mandelbrot");
    let dive = [RoomInput::PointerDown {
        x: 0.5,
        y: 0.5,
        t: 0.6,
    }];
    save(
        &room_screen(mandelbrot, 0.6, &dive, (900, 700), 0, false, 7),
        "flows/mandelbrot-before-reset.png",
        &mut manifest,
    );
    save(
        &room_screen(mandelbrot, 0.0, &[], (900, 700), 0, false, 7),
        "flows/mandelbrot-after-reset.png",
        &mut manifest,
    );

    let launch = room_by_id(&rooms, "times-tables");
    let golden = room_by_id(&rooms, "golden-angle");
    let mut journey = Journey {
        plays: 1_000,
        wins: 37,
        secrets: 12,
        ..Default::default()
    };
    journey.visited = rooms
        .iter()
        .map(|room| room.meta().id.to_string())
        .collect();
    for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
        let (width, height) = size;
        let mut help = room_screen(launch, 0.12, &[], size, 0, false, 1);
        overlays::draw_help_overlay(&mut help, width, height);
        save(
            &help,
            &format!("overlays/launch-help-{label}-{width}x{height}.png"),
            &mut manifest,
        );

        save(
            &room_screen(golden, 0.0, &[], size, 0, true, 1),
            &format!("overlays/room-inspect-{label}-{width}x{height}.png"),
            &mut manifest,
        );

        let mut journey_screen = room_screen(golden, 0.0, &[], size, 0, false, 42);
        overlays::draw_journey_overlay(
            &mut journey_screen,
            &journey,
            &Scoreboard::default(),
            rooms.len(),
            width,
            height,
        );
        save(
            &journey_screen,
            &format!("overlays/journey-level-42-{label}-{width}x{height}.png"),
            &mut manifest,
        );

        let mut banner = room_screen(golden, 0.0, &[], size, 0, false, 12);
        let level = feedback::level_up(12, 3);
        overlays::draw_banner(&mut banner, level.lines(), width, height);
        save(
            &banner,
            &format!("overlays/level-up-banner-{label}-{width}x{height}.png"),
            &mut manifest,
        );
    }

    for (phase_name, phase) in [("arrival", 0.05), ("departure", 0.95)] {
        for (label, size) in [("default", DEFAULT_SIZE), ("small", SMALL_SIZE)] {
            save(
                &show_screen(golden, phase, size),
                &format!(
                    "overlays/the-show-{phase_name}-{label}-{}x{}.png",
                    size.0, size.1
                ),
                &mut manifest,
            );
        }
    }

    save_sizes("studio", &mut manifest, studio_screen);

    let quiz_round = numinous_core::build_round(19, 1, 44, 18);
    let quiz_play = play::QuizPlay {
        round: quiz_round,
        flash: None,
    };
    save_sizes("quiz-question", &mut manifest, |width, height| {
        game_draw::draw_quiz(&rooms, &quiz_play, width, height)
    });
    let quiz_correct = play::QuizPlay {
        round: numinous_core::build_round(19, 1, 44, 18),
        flash: Some((true, 40)),
    };
    save_sizes("quiz-correct", &mut manifest, |width, height| {
        game_draw::draw_quiz(&rooms, &quiz_correct, width, height)
    });
    let quiz_wrong = play::QuizPlay {
        round: numinous_core::build_round(19, 1, 44, 18),
        flash: Some((false, 40)),
    };
    save_sizes("quiz-wrong", &mut manifest, |width, height| {
        game_draw::draw_quiz(&rooms, &quiz_wrong, width, height)
    });

    let (munch_round, munch_board) = play::deal_munch(23, numinous_core::FULL_DECK_ROUND, None);
    let mut munch = play::MunchPlay {
        board: munch_board,
        seed: 23,
        round: munch_round,
        cursor: 17,
        bites: BTreeSet::new(),
        graded: None,
    };
    save_sizes("munch-play", &mut manifest, |width, height| {
        game_draw::draw_munch(&munch, 20, width, height)
    });
    munch.bites = (0..munch.board.numbers.len()).collect();
    let bites: Vec<_> = munch.bites.iter().copied().collect();
    munch.graded = Some(numinous_core::grade_munch(&munch.board, &bites));
    save_sizes("munch-result", &mut manifest, |width, height| {
        game_draw::draw_munch(&munch, 20, width, height)
    });

    let arcade_live = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: None,
        over: false,
    };
    save_sizes("arcade-live", &mut manifest, |width, height| {
        game_draw::draw_arcade(&arcade_live, width, height)
    });
    let arcade_caught = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: Some((true, 40)),
        over: false,
    };
    save_sizes("arcade-caught", &mut manifest, |width, height| {
        game_draw::draw_arcade(&arcade_caught, width, height)
    });
    let arcade_clear = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: Some((false, 40)),
        over: false,
    };
    save_sizes("arcade-clear", &mut manifest, |width, height| {
        game_draw::draw_arcade(&arcade_clear, width, height)
    });
    let arcade_over = play::ArcadePlay {
        run: numinous_core::munch_arcade::Arcade::new(29),
        seed: 29,
        flash: None,
        over: true,
    };
    save_sizes("arcade-over", &mut manifest, |width, height| {
        game_draw::draw_arcade(&arcade_over, width, height)
    });

    let nim = play::NimPlay {
        heaps: numinous_core::nim_new(31),
        seed: 31,
        selected: 1,
        take: 2,
        message: "THE ORDER TOOK 1 FROM HEAP 1.".to_string(),
        over: None,
    };
    save_sizes("nim-live", &mut manifest, |width, height| {
        game_draw::draw_nim(&nim, width, height)
    });
    let nim_over = play::NimPlay {
        heaps: vec![0, 0, 0],
        seed: 31,
        selected: 0,
        take: 1,
        message: "THE LAST STONE IS YOURS.".to_string(),
        over: Some(true),
    };
    save_sizes("nim-win", &mut manifest, |width, height| {
        game_draw::draw_nim(&nim_over, width, height)
    });
    let nim_loss = play::NimPlay {
        message: "THE ORDER TOOK THE LAST STONE.".to_string(),
        over: Some(false),
        ..nim_over
    };
    save_sizes("nim-loss", &mut manifest, |width, height| {
        game_draw::draw_nim(&nim_loss, width, height)
    });

    let mut gauntlet = gauntlet(37);
    for stage in 0..=4 {
        gauntlet.stage = stage;
        save_sizes(
            &format!("gauntlet-stage-{stage}"),
            &mut manifest,
            |width, height| game_draw::draw_gauntlet(&rooms, &gauntlet, 20, width, height),
        );
    }

    manifest.sort();
    let actual: BTreeSet<_> = manifest.iter().cloned().collect();
    assert_eq!(
        actual.len(),
        manifest.len(),
        "QA scenario paths must be unique"
    );
    assert_eq!(
        actual,
        expected_paths(&rooms),
        "complete exact QA scenario inventory"
    );
    let manifest_path = PathBuf::from(OUTPUT).join("MANIFEST.txt");
    std::fs::write(
        &manifest_path,
        format!("{} screenshots\n{}\n", manifest.len(), manifest.join("\n")),
    )
    .expect("write manifest");
    println!("wrote {}", manifest_path.display());
}
