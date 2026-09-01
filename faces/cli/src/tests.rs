use super::{
    Cli, Command, RoomRenderInput, SonifyLayer, TerminalStyle, bounded_response_detail,
    describe_report, load_studio_creation, max_track_bytes, meta_json, not_found_message,
    open_studio_report, parse_gesture_arg, parse_poke_arg, parse_pokes, read_bounded,
    render_report, rooms_report, run, save_studio_creation, validate_pcm_body,
};

#[test]
fn explicit_command_stack_returns_success_and_contains_panics() {
    assert_eq!(
        super::run_on_command_stack(|| std::process::ExitCode::SUCCESS),
        std::process::ExitCode::SUCCESS
    );
    assert_eq!(
        super::run_on_command_stack(|| panic!("command stack probe")),
        std::process::ExitCode::FAILURE
    );
}

#[test]
fn test_persistence_paths_never_resolve_to_the_player_profile() {
    assert!(super::journey_path().starts_with(std::env::temp_dir()));
    assert!(super::scores_path().starts_with(std::env::temp_dir()));
}

#[test]
fn updater_accepts_only_the_managed_install_shape() {
    let state = super::TestStateRoot::new();
    let root = state.path.join("managed");
    let binary_dir = root.join("bin");
    std::fs::create_dir_all(&binary_dir).expect("managed bin should be creatable");
    let executable = binary_dir.join(if cfg!(windows) {
        "numinous.exe"
    } else {
        "numinous"
    });
    std::fs::write(&executable, b"binary").expect("test binary should be writable");
    assert!(super::managed_install_root(&executable).is_err());

    std::fs::write(
        root.join(".numinous-install-root"),
        b"Numinous install root v2\nroot.abcdef\n",
    )
    .expect("test marker should be writable");
    assert_eq!(
        super::managed_install_root(&executable).expect("managed shape should be accepted"),
        root
    );
    assert!(
        super::managed_install_root(&state.path.join(executable.file_name().unwrap())).is_err()
    );
}

#[test]
fn updater_stages_the_embedded_installer_privately() {
    let path = super::write_update_installer().expect("updater should stage");
    let bytes = std::fs::read(&path).expect("staged updater should be readable");
    assert_eq!(bytes, super::UPDATE_INSTALLER.as_bytes());
    let name = path.file_name().and_then(std::ffi::OsStr::to_str).unwrap();
    let suffix = if cfg!(windows) { ".ps1" } else { ".sh" };
    assert!(name.starts_with("numinous-update-"));
    assert!(name.ends_with(suffix));
    assert_eq!(name.len(), "numinous-update-".len() + 32 + suffix.len());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path)
            .expect("updater metadata should be readable")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }
    std::fs::remove_file(path).expect("staged updater should be removable");
}

#[test]
fn updater_preserves_the_existing_path_choice() {
    let installer = std::path::Path::new("maintenance/staged-installer");
    let command = super::maintenance_process(installer, "123", super::MaintenanceAction::Update);
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let preserve_flag = if cfg!(windows) {
        "-NoModifyPath"
    } else {
        "--no-modify-path"
    };
    assert!(args.iter().any(|arg| arg == preserve_flag));
    let uninstall_flag = if cfg!(windows) {
        "-Uninstall"
    } else {
        "--uninstall"
    };
    assert!(!args.iter().any(|arg| arg == uninstall_flag));
    assert_eq!(
        command.get_current_dir(),
        Some(std::path::Path::new("maintenance"))
    );
    assert!(std::path::Path::new(command.get_program()).is_absolute());
}

#[test]
fn uninstaller_waits_for_the_cli_and_requests_only_removal() {
    let installer = std::path::Path::new("maintenance/staged-installer");
    let command = super::maintenance_process(installer, "456", super::MaintenanceAction::Uninstall);
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let uninstall_flag = if cfg!(windows) {
        "-Uninstall"
    } else {
        "--uninstall"
    };
    let wait_flag = if cfg!(windows) {
        "-WaitForProcessId"
    } else {
        "--wait-for-pid"
    };
    assert!(args.iter().any(|arg| arg == uninstall_flag));
    assert!(args.windows(2).any(|pair| pair == [wait_flag, "456"]));
    assert!(!args.iter().any(|arg| arg == "--release-archive"));
    assert!(!args.iter().any(|arg| arg == "-ReleaseArchive"));
}

#[test]
fn uninstall_is_a_first_class_cli_command() {
    use clap::Parser;
    let cli = super::Cli::try_parse_from(["numinous", "uninstall"]).expect("parse");
    assert!(matches!(cli.command, Some(super::Command::Uninstall)));
}

#[test]
fn redirected_home_report_is_plain_and_useful() {
    let report = super::home_report(&numinous_core::Journey::default(), false, true);
    assert!(
        !report.contains('\x1b'),
        "redirected output must not contain ANSI"
    );
    assert!(report.starts_with("NUMINOUS: math you can feel\n"));
    assert!(report.contains("Today's room:"));
    assert!(report.contains("\n  numinous watch"));
    assert!(report.contains("\n  numinous rooms"));
    assert!(report.contains("\n  numinous play"));
    assert!(report.contains("\n  numinous --help"));
    assert!(report.lines().count() <= 16, "plain home stays concise");
}

#[test]
fn interactive_home_report_keeps_the_full_color_cabinet() {
    // The chain must be alive to show: a streak with no recorded daily
    // is exactly the dead-chain claim the display no longer makes.
    let journey = numinous_core::Journey {
        streak: 3,
        last_daily: super::pick_day(),
        ..Default::default()
    };
    let report = super::home_report(&journey, true, true);
    assert!(
        report.starts_with("\x1b[38;2;"),
        "cabinet starts in truecolor"
    );
    assert!(
        report.contains("\x1b[48;2;"),
        "cabinet paints its background"
    );
    assert!(
        report.contains("\x1b[0m"),
        "cabinet restores terminal color"
    );
    assert!(report.contains('▀'), "cabinet keeps its half-block raster");
    assert!(report.contains("NUMINOUS   LV"));
    assert!(report.contains("streak 3"));
    assert!(report.contains("\n  numinous watch"));
}

#[test]
fn eof_is_a_neutral_departure_from_every_cli_game() {
    fn check(
        name: &str,
        run: impl FnOnce(
            &mut numinous_core::Journey,
            &mut std::io::Cursor<Vec<u8>>,
        ) -> std::process::ExitCode,
    ) {
        let scores = super::scores_path();
        let _ = std::fs::remove_file(&scores);
        let mut journey = numinous_core::Journey::default();
        let before = journey.clone();
        let mut eof = std::io::Cursor::new(Vec::new());
        assert_eq!(run(&mut journey, &mut eof), std::process::ExitCode::SUCCESS);
        assert_eq!(journey, before, "{name} counted EOF as play");
        assert!(!scores.exists(), "{name} posted a score for EOF");
    }

    check("crack", |journey, input| {
        super::crack_with_input(1, 4, 1, journey, input)
    });
    check("bench", |journey, input| {
        super::bench_with_input(journey, input)
    });
    check("seti", |journey, input| {
        super::seti_with_input(1, 4, 1, journey, input)
    });
    check("aliens", |journey, input| {
        super::aliens_with_input(1, 1, journey, input)
    });
    check("munch", |journey, input| {
        super::munch_with_input(1, 1, journey, input)
    });
    check("arcade", |journey, input| {
        super::arcade_with_input(1, journey, input)
    });
    check("hackenbush", |journey, input| {
        super::hackenbush_with_input(1, journey, input)
    });
    check("party", |journey, input| {
        super::party_with_input(journey, input)
    });
    check("fifteen", |journey, input| {
        super::fifteen_with_input(1, 1, journey, input)
    });
    check("nim", |journey, input| {
        super::nim_with_input(1, journey, input)
    });
    check("gauntlet", |journey, input| {
        super::gauntlet_with_input(1, journey, input)
    });
    check("quiz", |journey, input| {
        super::quiz_with_input(1, 1, 40, 18, 4, journey, input)
    });
}

#[test]
fn partial_departures_preserve_completed_round_ledgers() {
    fn check(
        name: &str,
        key: &str,
        input: Vec<u8>,
        run: impl FnOnce(
            &mut numinous_core::Journey,
            &mut std::io::Cursor<Vec<u8>>,
        ) -> std::process::ExitCode,
    ) {
        let scores = super::scores_path();
        let _ = std::fs::remove_file(&scores);
        let mut journey = numinous_core::Journey::default();
        let mut input = std::io::Cursor::new(input);
        assert_eq!(
            run(&mut journey, &mut input),
            std::process::ExitCode::SUCCESS
        );
        assert_eq!(journey.plays, 1, "{name} lost its completed play");
        assert_eq!(
            journey.wins, 0,
            "{name} counted a deliberately wrong answer"
        );
        let board = super::load_scores();
        assert_eq!(board.entries.len(), 1, "{name} posted an extra score");
        assert_eq!(
            board.entries.get(key),
            Some(&0),
            "{name} changed its partial key"
        );
    }

    check(
        "seti",
        "seti seed:1 rounds:1",
        b"Z\n".to_vec(),
        |journey, input| super::seti_with_input(1, 4, 2, journey, input),
    );
    check(
        "aliens",
        "aliens seed:1 rounds:1",
        b"Z\n".to_vec(),
        |journey, input| super::aliens_with_input(1, 2, journey, input),
    );
    let fifteen_truth = numinous_core::fifteen::solvable(&numinous_core::fifteen::deal(1, 0));
    let wrong_fifteen = if fifteen_truth { b"U\n" } else { b"S\n" };
    check(
        "fifteen",
        "fifteen seed:1 rounds:1",
        wrong_fifteen.to_vec(),
        |journey, input| super::fifteen_with_input(1, 2, journey, input),
    );
    check(
        "quiz",
        "quiz seed:1 rounds:1",
        b"Z\n".to_vec(),
        |journey, input| super::quiz_with_input(2, 1, 40, 18, 4, journey, input),
    );
}

#[test]
fn abandoned_aggregate_and_board_sessions_keep_only_earned_state() {
    let scores = super::scores_path();
    let _ = std::fs::remove_file(&scores);
    let mut gauntlet_journey = numinous_core::Journey::default();
    assert_eq!(
        super::gauntlet_with_input(
            1,
            &mut gauntlet_journey,
            &mut std::io::Cursor::new(b"\n".to_vec()),
        ),
        std::process::ExitCode::SUCCESS
    );
    assert_eq!(gauntlet_journey.plays, 1);
    assert!(
        super::load_scores().entries.is_empty(),
        "abandoned gauntlet posted a total"
    );

    let mut munch_journey = numinous_core::Journey::default();
    assert_eq!(
        super::munch_with_input(
            1,
            2,
            &mut munch_journey,
            &mut std::io::Cursor::new(b"\n".to_vec()),
        ),
        std::process::ExitCode::SUCCESS
    );
    assert_eq!(munch_journey.plays, 1);
    let board = super::load_scores();
    assert_eq!(board.entries.len(), 1);
    assert!(
        board
            .entries
            .contains_key(&numinous_core::munch_score_key(1, 0))
    );
}

#[test]
fn room_render_boundaries_reject_unsafe_input_and_accept_phase_wrap() {
    for (width, height, phase) in [
        (0, 20, 0.0),
        (40, 0, 0.0),
        (40, 20, -1.0),
        (40, 20, 1.0),
        (40, 20, f64::NAN),
        (40, 20, f64::INFINITY),
        (4097, 20, 0.0),
    ] {
        assert!(
            super::validate_render_request(width, height, phase).is_err(),
            "accepted {width}x{height} at {phase}"
        );
    }
    assert!(super::validate_render_request(4096, 4096, 0.5).is_ok());

    let wrapped = vec![
        "down:0.2,0.3,0.8".to_string(),
        "move:0.4,0.5,0.2".to_string(),
    ];
    assert!(super::parse_gestures(&wrapped).is_ok());
    let ordered = vec![
        "down:0.2,0.3,0.2".to_string(),
        "move:0.4,0.5,0.8".to_string(),
        "up:0.4,0.5,0.8".to_string(),
    ];
    assert!(super::parse_gestures(&ordered).is_ok());
}

#[test]
fn test_persistence_paths_are_stable_per_test_and_isolated_between_threads() {
    let journey = super::journey_path();
    assert_eq!(journey, super::journey_path());
    assert_ne!(journey, super::scores_path());

    let other = std::thread::spawn(|| {
        let path = super::journey_path();
        std::fs::write(&path, b"other test").expect("test state should be writable");
        path
    })
    .join()
    .expect("path worker should finish");
    assert_ne!(journey, other);
    assert!(!other.exists());
    assert!(
        !other
            .parent()
            .expect("state path should have a parent")
            .exists()
    );
}

#[test]
fn test_state_root_clears_stale_data_rejects_files_and_cleans_on_drop() {
    let parent = super::journey_path()
        .parent()
        .expect("state path should have a parent")
        .to_path_buf();
    let stale_root = parent.join("stale-root");
    std::fs::create_dir_all(&stale_root).expect("stale root should be creatable");
    std::fs::write(stale_root.join("old.txt"), b"stale").expect("stale state should be writable");

    let root = super::TestStateRoot::at(stale_root.clone());
    assert!(stale_root.exists());
    assert!(!stale_root.join("old.txt").exists());
    drop(root);
    assert!(!stale_root.exists());

    let file_collision = parent.join("file-collision");
    std::fs::write(&file_collision, b"not a directory").expect("collision file should be writable");
    let rejected = std::panic::catch_unwind(|| super::TestStateRoot::at(file_collision.clone()));
    assert!(rejected.is_err());
    std::fs::remove_file(file_collision).expect("collision file should be removable");
}
use clap::{CommandFactory, Parser};
use numinous_core::room_by_id;
use serde_json::Value;

#[test]
fn derived_command_catalog_is_internally_consistent() {
    super::Cli::command().debug_assert();
}

#[test]
fn bounded_response_reader_distinguishes_exact_and_oversized_bodies() {
    let exact = read_bounded(std::io::Cursor::new(b"1234"), 4).expect("bounded read");
    assert_eq!(exact.as_deref(), Some(b"1234".as_slice()));
    let oversized = read_bounded(std::io::Cursor::new(b"12345"), 4).expect("bounded read");
    assert!(oversized.is_none());
}

#[test]
fn overlong_choose_and_game_input_leave_progress_unchanged() {
    let mut input = vec![b'1'; super::MAX_CLI_INPUT_BYTES + 1];
    input.push(b'\n');

    let mut choosing = numinous_core::Journey::from_text("plays 1");
    let before = choosing.clone();
    assert_eq!(
        super::choose_with_input(&mut choosing, &mut std::io::Cursor::new(input.clone())),
        std::process::ExitCode::SUCCESS
    );
    assert_eq!(choosing, before, "overlong choice spent a boon");

    let scores = super::scores_path();
    let _ = std::fs::remove_file(&scores);
    let mut playing = numinous_core::Journey::default();
    let before = playing.clone();
    assert_eq!(
        super::crack_with_input(1, 4, 1, &mut playing, &mut std::io::Cursor::new(input),),
        std::process::ExitCode::SUCCESS
    );
    assert_eq!(playing, before, "overlong game line counted as a move");
    assert!(!scores.exists(), "overlong game line posted a score");
}

#[test]
fn music_response_helpers_bound_diagnostics_duration_and_pcm_shape() {
    assert_eq!(
        bounded_response_detail(std::io::Cursor::new(b"detail")),
        "detail"
    );
    assert_eq!(
        bounded_response_detail(std::io::Cursor::new(vec![b'x'; 8 * 1024 + 1])),
        "response detail unavailable or oversized"
    );
    assert_eq!(max_track_bytes(10), Some(12 * 44_100 * 4));
    assert_eq!(max_track_bytes(u64::MAX), None);
    assert_eq!(
        validate_pcm_body(&[0]),
        Err("The tower sent an incomplete 16-bit stereo frame")
    );
    assert_eq!(
        validate_pcm_body(&vec![0; 8_820 * 2 + 2]),
        Err("The tower sent an incomplete 16-bit stereo frame")
    );
    assert_eq!(
        validate_pcm_body(&[0, 0, 0, 0]),
        Err("The tower sent almost nothing")
    );
    assert!(validate_pcm_body(&vec![0; 8_820 * 2]).is_ok());
}

#[test]
fn music_response_detail_escapes_terminal_controls() {
    let detail = bounded_response_detail(std::io::Cursor::new(
        b"plain\x1b[31m\nforged\rline\x07\tend",
    ));
    assert!(detail.starts_with("plain"));
    assert!(detail.ends_with("end"));
    assert!(
        !detail.chars().any(char::is_control),
        "diagnostic retained a control character: {detail:?}"
    );

    let exact = bounded_response_detail(std::io::Cursor::new(vec![b'x'; 8 * 1024]));
    assert_eq!(exact.len(), 8 * 1024);
    let oversized = bounded_response_detail(std::io::Cursor::new(vec![b'x'; 8 * 1024 + 1]));
    assert_eq!(oversized, "response detail unavailable or oversized");
}

#[test]
fn untrusted_diagnostic_values_never_emit_terminal_controls() {
    let hostile = "probe\u{1b}[31m\u{7}\rname";
    let unused_wav = std::env::temp_dir().join("numinous_hostile_expression.wav");
    let diagnostics = [
        parse_poke_arg(hostile).expect_err("invalid poke"),
        parse_gesture_arg(hostile).expect_err("invalid gesture"),
        not_found_message(hostile),
        super::sim_run(hostile, &[], 40, 20).expect_err("unknown simulation"),
        load_studio_creation(hostile).expect_err("missing Studio path"),
        super::plot_report(hostile, -1.0, 1.0, 0.0, 40, 20).expect_err("invalid plot expression"),
        super::sing_wav(hostile, -1.0, 1.0, 8, 1.0, &unused_wav)
            .expect_err("invalid song expression"),
    ];
    for diagnostic in diagnostics {
        assert!(diagnostic.contains("\\u{1b}"), "{diagnostic:?}");
        assert!(
            diagnostic
                .chars()
                .all(|character| !character.is_control() || character == '\n'),
            "diagnostic retained a terminal control: {diagnostic:?}"
        );
    }
    assert!(!unused_wav.exists());
}

#[test]
fn valid_expression_reports_escape_control_whitespace() {
    let report = super::plot_report("x\t", -1.0, 1.0, 0.0, 40, 20)
        .expect("expression with trailing whitespace");
    assert!(report.contains("y = x\\t"));
    assert!(
        report
            .chars()
            .all(|character| !character.is_control() || character == '\n')
    );
}

#[test]
fn apng_frame_budget_uses_checked_constant_space() {
    assert_eq!(super::apng_frame_bytes(4096, 4096), Ok(64 * 1024 * 1024));
    assert!(super::apng_frame_bytes(4097, 4096).is_err());
    assert!(super::apng_frame_bytes(usize::MAX, 2).is_err());
}

#[test]
fn music_request_does_not_follow_redirects_or_forward_the_key() {
    let destination = std::net::TcpListener::bind("127.0.0.1:0").expect("destination");
    let destination_address = destination.local_addr().expect("destination address");
    let origin = std::net::TcpListener::bind("127.0.0.1:0").expect("origin");
    let origin_address = origin.local_addr().expect("origin address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = origin.accept().expect("origin request");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .expect("read timeout");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = std::io::Read::read(&mut stream, &mut chunk).expect("read request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }
        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
            .expect("complete request headers");
        let content_length = String::from_utf8_lossy(&request[..header_end])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = std::io::Read::read(&mut stream, &mut chunk).expect("read body");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }
        let response_body = "redirect body retained";
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: http://{destination_address}/capture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        std::io::Write::write_all(&mut stream, response.as_bytes()).expect("redirect");
        request
    });

    let result = super::send_music_request(
        &format!("http://{origin_address}/music"),
        "dummy-validation-key",
        "{}",
        std::time::Duration::from_secs(2),
    );
    match result {
        Err(error) => match *error {
            super::MusicRequestError::HttpStatus(response) => {
                assert_eq!(response.status(), 302);
                let detail = response
                    .into_body()
                    .read_to_string()
                    .expect("read redirect response body");
                assert_eq!(detail, "redirect body retained");
            }
            other => panic!("unexpected redirect error: {other:?}"),
        },
        Ok(response) => panic!("redirect returned HTTP {}", response.status()),
    }
    let request = server.join().expect("origin server");
    let request = String::from_utf8(request).expect("ASCII request");
    assert!(
        request
            .to_ascii_lowercase()
            .contains("xi-api-key: dummy-validation-key")
    );

    destination.set_nonblocking(true).expect("nonblocking");
    let error = destination
        .accept()
        .expect_err("redirect destination received the request")
        .kind();
    assert_eq!(error, std::io::ErrorKind::WouldBlock);
}

#[test]
fn rooms_report_lists_times_tables() {
    let report = rooms_report(false);
    assert!(report.contains("times-tables"));
    assert!(!report.contains("tetractys"));
}

#[test]
fn rooms_report_json_is_a_non_empty_array() {
    let text = rooms_report(true);
    let value: Value = serde_json::from_str(&text).expect("valid json");
    let rooms = value.as_array().expect("room catalog");
    assert!(!rooms.is_empty());
    assert!(rooms.iter().all(|room| room["id"] != "tetractys"));
}

#[test]
fn env_file_key_reads_key_without_unbounded_file_loads() {
    let dir = std::env::temp_dir().join("numinous_cli_env_file_key");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("test dir");
    let path = dir.join(".env");

    std::fs::write(
        &path,
        "# local secrets\nOTHER=value\nELEVENLABS_API_KEY='test-key'\n",
    )
    .expect("write env file");
    assert_eq!(super::env_file_key_from(&path).expect("key"), "test-key");

    std::fs::write(&path, "x".repeat(super::MAX_ENV_FILE_BYTES as usize + 1))
        .expect("write oversized env file");
    assert!(super::env_file_key_from(&path).is_err());

    std::fs::write(&path, "OTHER=value\n").expect("write keyless env file");
    assert!(super::env_file_key_from(&path).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn describe_known_room_reports_its_wing() {
    let text = describe_report(
        "times-tables",
        false,
        false,
        &numinous_core::Journey::default(),
    )
    .expect("known room");
    assert!(text.contains("Number & Pattern"));
    assert!(text.contains(
        "Action: TURN THE DIAL (phase here: numinous render times-tables --t 0.375; --poke x,y is a second hand)"
    ));
    assert!(text.contains("Goal: LAND ON EXACTLY 4 LOBES"));
}

#[test]
fn describe_json_carries_the_id() {
    let text = describe_report(
        "times-tables",
        true,
        false,
        &numinous_core::Journey::default(),
    )
    .expect("known room");
    let value: Value = serde_json::from_str(&text).expect("valid json");
    assert_eq!(value["id"], "times-tables");
    assert_eq!(value["action"], "DRAG: TURN THE DIAL");
    assert_eq!(value["goal"], "LAND ON EXACTLY 4 LOBES");
}

#[test]
fn describe_is_a_safe_doorway_without_the_reveal() {
    let text = describe_report(
        "times-tables",
        false,
        false,
        &numinous_core::Journey::default(),
    )
    .expect("known room");
    assert!(!text.contains("Reveal:"));
    assert!(!text.contains("Set the dial to 2"));
    assert!(text.contains("Play: numinous render times-tables"));
}

#[test]
fn describe_json_omits_explanation_fields() {
    let text = describe_report(
        "times-tables",
        true,
        false,
        &numinous_core::Journey::default(),
    )
    .expect("known room");
    let value: Value = serde_json::from_str(&text).expect("valid json");
    for field in ["reveal", "concept", "deep_cuts", "citation"] {
        assert!(value.get(field).is_none(), "{field} leaked: {value}");
    }
    assert_eq!(value["next"]["command"], "numinous render times-tables");
}

#[test]
fn describe_unknown_room_guides_the_user() {
    let err = describe_report(
        "no-such-room",
        false,
        false,
        &numinous_core::Journey::default(),
    )
    .expect_err("unknown room");
    assert!(err.contains("numinous rooms"), "{err}");
}

#[test]
fn render_known_room_has_ink() {
    let text = render_report("times-tables", 40, 20, 0.0, false, RoomRenderInput::plain())
        .expect("known room");
    assert!(text.contains('*'));
    assert!(text.contains(
        "Action: TURN THE DIAL (phase here: numinous render times-tables --t 0.375; --poke x,y is a second hand)"
    ));
    assert!(text.contains("Goal: LAND ON EXACTLY 4 LOBES"));
    assert!(!text.contains("Aha earned:"));
}

#[test]
fn times_tables_goal_earns_the_aha_and_reveal_from_hand_input() {
    let report = render_report(
        "times-tables",
        72,
        32,
        0.8,
        false,
        RoomRenderInput::new(0, &[(0.374, 0.5)]),
    )
    .expect("goal render");

    assert!(report.contains("Status: K 5.00  CLOSED  4 LOBES  FOUND"));
    assert!(report.contains("Goal: LAND ON EXACTLY 4 LOBES"));
    assert!(report.contains("Aha earned: LAND ON EXACTLY 4 LOBES"));
    assert!(report.contains("Reveal: Set the dial to 2"));
}

#[test]
fn times_tables_exact_phase_meets_the_goal_without_opening_the_reveal() {
    let report = render_report(
        "times-tables",
        72,
        32,
        0.375,
        false,
        RoomRenderInput::plain(),
    )
    .expect("ambient target render");

    assert!(report.contains("Status: K 5.00  CLOSED  4 LOBES  FOUND"));
    assert!(!report.contains("Aha earned:"));
    assert!(!report.contains("Reveal:"));
}

#[test]
fn reveal_requires_play_and_engineered_consolidation() {
    let fresh = numinous_core::Journey::default();
    assert!(super::reveal_report("mandelbrot", false, false, &fresh).is_err());

    let mut played = fresh.clone();
    played.visit("mandelbrot");
    let ordinary = super::reveal_report("mandelbrot", false, false, &played)
        .expect("played ordinary room reveals");
    assert!(ordinary.contains("zoom") && ordinary.contains("Times Tables"));

    played.visit("times-tables");
    assert!(super::reveal_report("times-tables", false, false, &played).is_err());
    played.consolidate("times-tables");
    let aha = super::reveal_report("times-tables", false, false, &played)
        .expect("consolidated Aha reveals");
    assert!(aha.contains("Mandelbrot"));
}

#[test]
fn render_unknown_room_is_error() {
    assert!(render_report("no-such-room", 10, 10, 0.0, false, RoomRenderInput::plain(),).is_err());
}

#[test]
fn parse_pokes_keep_hand_points_normalized() {
    assert_eq!(parse_poke_arg("0.25,0.75"), Ok((0.25, 0.75)));
    assert_eq!(
        parse_pokes(&["0,1".to_string(), "1,0".to_string()]),
        Ok(vec![(0.0, 1.0), (1.0, 0.0)])
    );
    assert!(parse_poke_arg("0.5").is_err());
    assert!(parse_poke_arg("-0.1,0.5").is_err());
    assert!(parse_poke_arg("0.5,NaN").is_err());
    let too_many = vec!["0.5,0.5".to_string(); numinous_core::MAX_ROOM_POKES + 1];
    assert!(parse_pokes(&too_many).is_err());
}

#[test]
fn render_report_uses_hand_points_when_supplied() {
    let resting = render_report(
        "double-pendulum",
        50,
        30,
        0.25,
        false,
        RoomRenderInput::plain(),
    )
    .expect("resting room");
    let poked = render_report(
        "double-pendulum",
        50,
        30,
        0.25,
        false,
        RoomRenderInput::new(0, &[(0.2, 0.8)]),
    )
    .expect("poked room");
    assert_ne!(
        resting, poked,
        "a supplied hand point should steer the frame"
    );
    let last_only = render_report(
        "double-pendulum",
        50,
        30,
        0.25,
        false,
        RoomRenderInput::new(0, &[(0.8, 0.2)]),
    )
    .expect("last-only poked room");
    let newest_last = render_report(
        "double-pendulum",
        50,
        30,
        0.25,
        false,
        RoomRenderInput::new(0, &[(0.2, 0.8), (0.8, 0.2)]),
    )
    .expect("multi-poked room");
    assert_eq!(
        last_only, newest_last,
        "Double Pendulum should treat the newest hand point as the re-drop"
    );
}

#[test]
fn interacted_render_reports_the_room_specific_consequence() {
    let resting = render_report(
        "cult-of-pi",
        50,
        24,
        0.0,
        false,
        RoomRenderInput::new(0, &[]),
    )
    .expect("resting cult render");
    let report = render_report(
        "cult-of-pi",
        50,
        24,
        0.0,
        false,
        RoomRenderInput::new(0, &[(0.5, 0.5)]),
    )
    .expect("cult render");
    assert!(
        report.contains("Status: 1 HELD FIX0 D") && report.contains(" CH01"),
        "interaction status must reach the CLI: {report}"
    );
    assert!(
        report.split("\nStatus:").next() != resting.split("\nStatus:").next(),
        "a phase-zero hold must visibly change the character frame: {report}"
    );
}

#[test]
fn static_render_reports_every_available_core_readout() {
    let phase = 0.37;
    let mut checked = 0;
    for room in numinous_core::all_rooms() {
        let Some(status) = room.status(phase) else {
            continue;
        };
        let report = render_report(
            room.meta().id,
            50,
            24,
            phase,
            false,
            RoomRenderInput::plain(),
        )
        .expect("catalog room renders");
        assert!(
            report.contains(&format!(
                "Status: {}",
                super::scrub_touch_fragments(&status)
            )),
            "{} must expose its shared-core readout, gesture verbs scrubbed",
            room.meta().id
        );
        assert!(
            report.contains(&format!(
                "Action: {}",
                super::terminal_action_line(room.as_ref())
            )),
            "{} must expose its action translated for a keyboard face",
            room.meta().id
        );
        if let Some(goal) = room.goal() {
            assert!(
                report.contains(&format!("Goal: {goal}")),
                "{} must expose its shared-core goal",
                room.meta().id
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 11,
        "status coverage unexpectedly shrank: {checked}"
    );
}

#[test]
fn default_munch_session_reaches_the_complete_rule_deck() {
    use clap::Parser;
    let cli = super::Cli::try_parse_from(["numinous", "munch"]).expect("parse");
    let Some(super::Command::Munch { rounds, .. }) = cli.command else {
        panic!("munch command");
    };
    assert!(
        rounds as u64 > numinous_core::FULL_DECK_ROUND,
        "the default session must reach multiple full-deck boards"
    );
}

#[test]
fn crack_rejects_unsupported_code_lengths_before_play() {
    let mut journey = numinous_core::Journey {
        plays: 100,
        ..Default::default()
    };
    for digits in [
        numinous_core::MIN_CODE_DIGITS - 1,
        numinous_core::MAX_CODE_DIGITS + 1,
    ] {
        let before = journey.clone();
        let code = run(
            Command::Crack {
                seed: 1,
                daily: false,
                digits,
                attempts: 1,
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey, before);
    }
}

#[test]
fn compact_life_poke_matches_a_phase_stamped_click() {
    use numinous_core::RoomInput;
    let point = (0.23, 0.71);
    let phase = 0.47;
    let compact = render_report(
        "game-of-life",
        64,
        48,
        phase,
        false,
        RoomRenderInput::new(0, &[point]),
    )
    .expect("compact poke");
    let event = [RoomInput::PointerDown {
        x: point.0,
        y: point.1,
        t: phase,
    }];
    let gesture = render_report(
        "game-of-life",
        64,
        48,
        phase,
        false,
        RoomRenderInput::with_gesture(0, &event),
    )
    .expect("gesture");
    assert_eq!(compact, gesture);
}

#[test]
fn life_gesture_replays_a_causal_launch_then_evolution() {
    use numinous_core::RoomInput;
    let variation = 7;
    let final_phase = 0.5;
    let launch_phase = 0.1;
    let point = (0.23, 0.71);
    let event = [RoomInput::PointerDown {
        x: point.0,
        y: point.1,
        t: launch_phase,
    }];
    let report = render_report(
        "game-of-life",
        64,
        48,
        final_phase,
        false,
        RoomRenderInput::with_gesture(variation, &event),
    )
    .expect("causal Life replay");
    let repeated = render_report(
        "game-of-life",
        64,
        48,
        final_phase,
        false,
        RoomRenderInput::with_gesture(variation, &event),
    )
    .expect("repeated Life replay");
    let untouched = render_report(
        "game-of-life",
        64,
        48,
        final_phase,
        false,
        RoomRenderInput::new(variation, &[]),
    )
    .expect("untouched Life frame");
    let compact_now = render_report(
        "game-of-life",
        64,
        48,
        final_phase,
        false,
        RoomRenderInput::new(variation, &[point]),
    )
    .expect("same-phase compact poke");

    let mut session = numinous_core::rooms::game_of_life::LifeSession::new(variation);
    for _ in 0..14 {
        session.advance();
    }
    assert!(session.launch(point));
    for _ in 14..70 {
        session.advance();
    }
    let mut canvas = numinous_core::Canvas::new(64, 48);
    session.render(&mut canvas);
    let room = numinous_core::room_by_id("game-of-life").expect("Life room");
    let expected = format!(
        "{}Status: {}\nAction: {}\n",
        canvas.to_text(),
        super::scrub_touch_fragments(&session.status()),
        super::terminal_action_line(room.as_ref())
    );

    assert_eq!(report, expected);
    assert_eq!(report, repeated);
    assert_ne!(report, untouched);
    assert_ne!(report, compact_now);
    assert!(report.contains("Status: BORN"), "got: {report}");
    assert!(report.contains("GEN 70"), "got: {report}");
    assert!(report.contains("GLIDER 1"), "got: {report}");
}

#[test]
fn render_exposes_one_explicit_replayable_variation_seed() {
    let cli = Cli::try_parse_from(["numinous", "render", "game-of-life", "--variation", "7"])
        .expect("explicit variation parses");
    assert!(matches!(
        cli.command,
        Some(Command::Render {
            variation: Some(7),
            vary: false,
            ..
        })
    ));
    assert!(
        Cli::try_parse_from([
            "numinous",
            "render",
            "game-of-life",
            "--variation",
            "7",
            "--vary",
        ])
        .is_err(),
        "clock variation and an explicit seed are mutually exclusive"
    );
}

#[test]
fn invalid_render_pokes_do_not_record_progress() {
    let mut journey = numinous_core::Journey::default();
    let before = journey.clone();
    let code = run(
        Command::Render {
            id: "double-pendulum".to_string(),
            width: 30,
            height: 20,
            t: 0.0,
            out: None,
            color: false,
            era: "modern".to_string(),
            vary: false,
            variation: None,
            pokes: vec!["2,0.5".to_string()],
            gestures: Vec::new(),
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey, before);
}

#[test]
fn meta_json_has_expected_fields() {
    let room = room_by_id("times-tables").expect("known room");
    let value = meta_json(&room.meta());
    for key in ["id", "title", "wing", "blurb"] {
        assert!(value.get(key).is_some(), "missing key {key}");
    }
}

#[test]
fn every_composed_screen_is_escape_free_without_color() {
    // Rendering escape-free is not enough on its own: the frame gets
    // composed into a larger report, and a hardcoded reset sitting next to
    // it puts the escapes straight back. Assert on the composed output,
    // which is what a player actually receives.
    let room = numinous_core::room_by_id("times-tables").expect("room");
    let mut raster = numinous_core::Raster::with_accent(24, 16, room.meta().accent);
    room.render(&mut raster, 0.4);

    let mono_report = super::render_color_report(
        "times-tables",
        24,
        16,
        0.4,
        false,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: false,
        },
        RoomRenderInput::plain(),
    )
    .expect("known room");
    let mono_home = super::home_report(&numinous_core::Journey::default(), true, false);
    let mono_era = super::ansi_in_era(&raster, numinous_core::Era::Phosphor, false);

    for (name, screen) in [
        ("render report", mono_report),
        ("home screen", mono_home),
        ("era frame", mono_era),
    ] {
        assert!(
            !screen.contains('\u{1b}'),
            "{name} still emits an escape without color: {:?}",
            screen
                .split('\u{1b}')
                .nth(1)
                .map(|tail| tail.chars().take(12).collect::<String>())
        );
    }

    // The watch frame is the one exception, and only for cursor control:
    // it must still home the cursor and erase the line, or the live loop
    // cannot paint in place. It must carry no color.
    let watch = super::watch_frame(
        room.as_ref(),
        0.4,
        24,
        16,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: false,
        },
    );
    assert!(!watch.contains("38;2;"), "watch frame colored: {watch:?}");
    assert!(!watch.contains("\u{1b}[0m"), "watch frame reset: {watch:?}");
    for sequence in watch.split('\u{1b}').skip(1) {
        assert!(
            sequence.starts_with('[') && matches!(sequence.chars().nth(1), Some('H' | 'K')),
            "only cursor control may survive, got {sequence:?}"
        );
    }
}

#[test]
fn color_is_still_color_when_it_is_allowed() {
    // The counterpart: turning the switch on must still produce truecolor,
    // so an over-eager escape purge cannot pass the test above.
    let report = super::render_color_report(
        "times-tables",
        24,
        16,
        0.4,
        false,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: true,
        },
        RoomRenderInput::plain(),
    )
    .expect("known room");
    assert!(report.contains("\u{1b}[38;2;"), "color mode lost its color");
}

#[test]
fn no_color_is_honored_when_present_and_not_empty() {
    // The no-color.org convention is about presence. A player who sets
    // NO_COLOR=0 still set it, and still meant it, so truthiness must not
    // enter into it.
    for (value, expected_color) in [
        (None, true),
        (Some(""), true),
        (Some("1"), false),
        (Some("0"), false),
        (Some("false"), false),
        (Some("no"), false),
        (Some(" "), false),
    ] {
        let observed = super::color_allowed_for(value.map(std::ffi::OsStr::new));
        assert_eq!(
            observed, expected_color,
            "NO_COLOR={value:?} should give color={expected_color}"
        );
    }
}

#[test]
fn a_diagnostic_prints_one_line_however_it_was_terminated() {
    // report_diagnostic is the single guarantee point, so it has to hold
    // for messages that carry no newline, one, several, or a CRLF pair.
    for raw in [
        "no newline",
        "one\n",
        "two\n\n",
        "crlf\r\n",
        "mixed\r\n\r\n",
    ] {
        let printed = format!("{}\n", raw.trim_end_matches(['\r', '\n']));
        assert!(printed.ends_with('\n'), "{raw:?} -> {printed:?}");
        assert_eq!(
            printed.matches('\n').count(),
            1,
            "{raw:?} must print exactly one line: {printed:?}"
        );
        assert!(!printed.contains('\r'), "{raw:?} -> {printed:?}");
    }
}

#[test]
fn every_cli_diagnostic_terminates_its_line_exactly_once() {
    // A message that does not end the line leaves the next shell prompt
    // stranded mid-row; one that ends twice prints a blank line. Messages
    // are built in dozens of places and arrive with and without their own
    // newline, so the guarantee lives at the single point that writes them.
    let unused_wav = std::env::temp_dir().join("numinous_unterminated_check.wav");
    let diagnostics = [
        not_found_message("no-such-room"),
        super::plot_report("sin(x", -1.0, 1.0, 0.0, 40, 20).expect_err("unbalanced plot"),
        super::plot_report("", -1.0, 1.0, 0.0, 40, 20).expect_err("empty plot"),
        super::plot_report("x @ 2", -1.0, 1.0, 0.0, 40, 20).expect_err("bad plot token"),
        super::sing_wav("sin(x", -1.0, 1.0, 8, 1.0, &unused_wav).expect_err("unbalanced song"),
        super::validate_render_request(0, 10, 0.0).expect_err("zero width"),
        super::validate_render_request(10, 10, 5.0).expect_err("out of range t"),
        super::sim_run("not-a-sim", &[], 40, 20).expect_err("unknown simulation"),
        super::load_studio_creation("no-such-file.num").expect_err("missing Studio file"),
        super::load_studio_creation("numinous://studio/zzz").expect_err("invalid share link"),
    ];
    for diagnostic in diagnostics {
        // Messages may or may not carry their own newline; report()
        // normalizes both to exactly one.
        let printed = format!("{}\n", diagnostic.strip_suffix('\n').unwrap_or(&diagnostic));
        assert!(printed.ends_with('\n'), "must end its line: {printed:?}");
        assert!(
            !printed.ends_with("\n\n"),
            "must end with exactly one newline: {printed:?}"
        );
        assert!(
            !printed.trim_end().is_empty(),
            "a diagnostic must say something: {diagnostic:?}"
        );
    }
}

#[test]
fn not_found_message_suggests_the_room_and_points_at_the_catalog() {
    let message = not_found_message("times-table");
    assert!(message.contains("times-tables"), "{message:?}");
    assert!(message.contains("Did you mean"), "{message:?}");
    assert!(message.contains("numinous rooms"), "{message:?}");
    assert!(message.ends_with('\n'), "{message:?}");
}

#[test]
fn not_found_message_never_dumps_the_catalog() {
    // The failure this replaced answered a typo with every id in the
    // catalog, so the message grew with the product. Keep it bounded.
    let rooms = numinous_core::all_rooms();
    let message = not_found_message("qqqqzzzzxxxxwwww");
    assert!(
        message.len() < 200,
        "not-found must stay small, got {} bytes: {message:?}",
        message.len()
    );
    let named = rooms
        .iter()
        .filter(|room| message.contains(room.meta().id))
        .count();
    assert!(
        named <= numinous_core::MAX_ROOM_SUGGESTIONS,
        "message named {named} rooms: {message:?}"
    );
    assert!(message.contains("numinous rooms"), "{message:?}");
    assert!(message.ends_with('\n'), "{message:?}");
}

#[test]
fn render_png_writes_a_non_empty_file() {
    let mut path = std::env::temp_dir();
    path.push("numinous_cli_render_test.png");
    let message = super::render_png(
        "times-tables",
        64,
        48,
        0.0,
        &path,
        false,
        numinous_core::Era::Modern,
        RoomRenderInput::plain(),
    )
    .expect("render png");
    assert!(message.contains("wrote"));
    let size = std::fs::metadata(&path).expect("file exists").len();
    assert!(size > 0, "png should not be empty");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn render_png_unknown_room_is_error() {
    let path = std::env::temp_dir().join("numinous_cli_should_not_exist.png");
    assert!(
        super::render_png(
            "no-such-room",
            10,
            10,
            0.0,
            &path,
            false,
            numinous_core::Era::Modern,
            RoomRenderInput::plain(),
        )
        .is_err()
    );
}

#[test]
fn loop_subcommand_parses_share_defaults() {
    let cli = Cli::try_parse_from([
        "numinous",
        "loop",
        "times-tables",
        "--out",
        "loop.png",
        "--poke",
        "0.4,0.5",
    ])
    .expect("loop parses");
    assert!(matches!(
        cli.command,
        Some(Command::Loop {
            size: 480,
            t: 0.0,
            variation: 0,
            ..
        })
    ));
}

#[test]
fn share_subcommand_parses_bundle_defaults() {
    let cli = Cli::try_parse_from([
        "numinous",
        "share",
        "lorenz",
        "--out",
        "shares",
        "--era",
        "phosphor",
        "--variation",
        "3",
    ])
    .expect("share parses");
    assert!(matches!(
        cli.command,
        Some(Command::Share {
            size: 480,
            t: 0.0,
            variation: 3,
            era,
            ..
        }) if era == "phosphor"
    ));
}

#[test]
fn render_loop_apng_writes_a_multi_frame_file() {
    let path = std::env::temp_dir().join("numinous_cli_loop_test.png");
    let _ = std::fs::remove_file(&path);
    let message = super::render_loop_apng(
        "times-tables",
        64,
        0.0,
        &path,
        false,
        numinous_core::Era::Modern,
        RoomRenderInput::plain(),
        false,
    )
    .expect("render loop");
    assert!(message.contains("wrote"));
    assert!(message.contains("24 frames"));
    let file = std::fs::File::open(&path).expect("open loop");
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let reader = decoder.read_info().expect("read loop header");
    let animation = reader
        .info()
        .animation_control
        .expect("CLI short loop is animated");
    assert_eq!(animation.num_frames, super::LOOP_FRAMES);
    assert_eq!(animation.num_plays, 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sonify_wav_writes_a_non_empty_file() {
    let mut path = std::env::temp_dir();
    path.push("numinous_cli_sonify_test.wav");
    let message = super::sonify_wav("lissajous", 0.0, &path, false, RoomRenderInput::plain())
        .expect("sonify");
    assert!(message.contains("wrote"));
    let size = std::fs::metadata(&path).expect("file exists").len();
    assert!(size > 0, "wav should not be empty");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn room_bed_export_exactly_quantizes_the_shared_stereo_source() {
    let first = std::env::temp_dir().join("numinous_cli_room_bed.wav");
    let second = std::env::temp_dir().join("numinous_cli_room_bed_repeat.wav");
    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
    let input = RoomRenderInput::new(42, &[]);

    let report = super::sonify_wav_layer(
        "times-tables",
        0.0,
        &first,
        false,
        input,
        SonifyLayer::RoomBed,
    )
    .expect("room bed");
    super::sonify_wav_layer(
        "times-tables",
        0.0,
        &second,
        false,
        input,
        SonifyLayer::RoomBed,
    )
    .expect("repeat room bed");
    assert!(report.contains("room bed, 40.00s, 79 events, stereo 16000 Hz"));
    assert!(report.contains("stable pre-master bed only"));

    let bytes = std::fs::read(&first).expect("WAV bytes");
    assert_eq!(bytes, std::fs::read(&second).expect("repeat WAV"));
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut offset = 12usize;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start.checked_add(size).expect("bounded RIFF chunk");
        assert!(end <= bytes.len(), "RIFF chunk must fit the file");
        if id == b"fmt " {
            format = Some(&bytes[start..end]);
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = end + size % 2;
    }
    let format = format.expect("format chunk");
    assert!(format.len() >= 16);
    assert_eq!(u16::from_le_bytes(format[0..2].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(format[2..4].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(format[4..8].try_into().unwrap()),
        numinous_core::ROOM_BED_SOURCE_RATE
    );
    assert_eq!(
        u32::from_le_bytes(format[8..12].try_into().unwrap()),
        64_000
    );
    assert_eq!(u16::from_le_bytes(format[12..14].try_into().unwrap()), 4);
    assert_eq!(u16::from_le_bytes(format[14..16].try_into().unwrap()), 16);

    let room = numinous_core::all_rooms_with(42)
        .into_iter()
        .find(|room| room.meta().id == "times-tables")
        .expect("varied room");
    let arrangement = room.motif().expect("motif").arrangement();
    let expected = arrangement
        .render_stereo(numinous_core::ROOM_BED_SOURCE_RATE)
        .into_iter()
        .flat_map(|sample| numinous_core::quantize_pcm16(sample).to_le_bytes())
        .collect::<Vec<_>>();
    assert_eq!(data.expect("data chunk"), expected);

    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
}

#[test]
fn sonify_replays_times_tables_hand_input_into_the_sound() {
    let dir = std::env::temp_dir();
    let target = dir.join("numinous_cli_times_target.wav");
    let other = dir.join("numinous_cli_times_other.wav");
    let gesture_path = dir.join("numinous_cli_times_gesture.wav");
    let target_points = [(0.374, 0.5)];
    let other_points = [(0.75, 0.5)];
    let gesture = [numinous_core::RoomInput::PointerDown {
        x: 0.374,
        y: 0.5,
        t: 0.8,
    }];

    let report = super::sonify_wav(
        "times-tables",
        0.8,
        &target,
        false,
        RoomRenderInput::new(0, &target_points),
    )
    .expect("target sound");
    super::sonify_wav(
        "times-tables",
        0.8,
        &other,
        false,
        RoomRenderInput::new(0, &other_points),
    )
    .expect("other dial sound");
    super::sonify_wav(
        "times-tables",
        0.8,
        &gesture_path,
        false,
        RoomRenderInput::with_gesture(0, &gesture),
    )
    .expect("gesture sound");

    assert!(report.contains("Status: K 5.00  CLOSED  4 LOBES  FOUND"));
    let target_audio = std::fs::read(&target).expect("target WAV");
    assert_ne!(
        target_audio,
        std::fs::read(&other).expect("other WAV"),
        "different effective multipliers must produce different audio"
    );
    assert_eq!(
        target_audio,
        std::fs::read(&gesture_path).expect("gesture WAV"),
        "a compact poke and equivalent gesture must sonify identically"
    );
    let _ = std::fs::remove_file(target);
    let _ = std::fs::remove_file(other);
    let _ = std::fs::remove_file(gesture_path);
}

#[test]
fn sonify_replays_the_selected_galton_coin_across_input_forms() {
    let dir = std::env::temp_dir();
    let left = dir.join("numinous_cli_galton_left.wav");
    let fair = dir.join("numinous_cli_galton_fair.wav");
    let gesture_path = dir.join("numinous_cli_galton_gesture.wav");
    let left_points = [(0.1, 0.5)];
    let fair_points = [(0.5, 0.5)];
    let gesture = [numinous_core::RoomInput::PointerDown {
        x: 0.1,
        y: 0.5,
        t: 0.4,
    }];

    let report = super::sonify_wav(
        "galton-board",
        0.4,
        &left,
        false,
        RoomRenderInput::new(0, &left_points),
    )
    .expect("left coin sound");
    super::sonify_wav(
        "galton-board",
        0.4,
        &fair,
        false,
        RoomRenderInput::new(0, &fair_points),
    )
    .expect("fair coin sound");
    super::sonify_wav(
        "galton-board",
        0.4,
        &gesture_path,
        false,
        RoomRenderInput::with_gesture(0, &gesture),
    )
    .expect("gesture coin sound");

    assert!(
        report.contains("Status: DROP 1x64=64") && report.contains("P.30"),
        "got: {report}"
    );
    let left_audio = std::fs::read(&left).expect("left WAV");
    assert_ne!(
        left_audio,
        std::fs::read(&fair).expect("fair WAV"),
        "different selected probabilities must produce different audio"
    );
    assert_eq!(
        left_audio,
        std::fs::read(&gesture_path).expect("gesture WAV"),
        "a compact poke and equivalent gesture must sonify identically"
    );
    let _ = std::fs::remove_file(left);
    let _ = std::fs::remove_file(fair);
    let _ = std::fs::remove_file(gesture_path);
}

#[test]
fn sonify_replays_the_pendulum_drop_across_input_forms() {
    let dir = std::env::temp_dir();
    let compact = dir.join("numinous_cli_pendulum_compact.wav");
    let gesture_path = dir.join("numinous_cli_pendulum_gesture.wav");
    let other = dir.join("numinous_cli_pendulum_other.wav");
    let point = [(0.7, 0.25)];
    let other_point = [(0.2, 0.75)];
    let gesture = [numinous_core::RoomInput::PointerDown {
        x: 0.7,
        y: 0.25,
        t: 0.4,
    }];

    let report = super::sonify_wav(
        "double-pendulum",
        0.4,
        &compact,
        false,
        RoomRenderInput::new(0, &point),
    )
    .expect("compact drop sound");
    super::sonify_wav(
        "double-pendulum",
        0.4,
        &gesture_path,
        false,
        RoomRenderInput::with_gesture(0, &gesture),
    )
    .expect("gesture drop sound");
    super::sonify_wav(
        "double-pendulum",
        0.4,
        &other,
        false,
        RoomRenderInput::new(0, &other_point),
    )
    .expect("other drop sound");

    assert!(report.contains("Status: PINNED"));
    let compact_audio = std::fs::read(&compact).expect("compact WAV");
    assert_eq!(
        compact_audio,
        std::fs::read(&gesture_path).expect("gesture WAV"),
        "a compact poke and equivalent held gesture must sonify identically"
    );
    assert_ne!(
        compact_audio,
        std::fs::read(&other).expect("other WAV"),
        "different hand-chosen angles must produce different audio"
    );
    let _ = std::fs::remove_file(compact);
    let _ = std::fs::remove_file(gesture_path);
    let _ = std::fs::remove_file(other);
}

#[test]
fn sonify_parses_replay_input_and_rejects_invalid_or_mixed_forms() {
    let cli = Cli::try_parse_from([
        "numinous",
        "sonify",
        "times-tables",
        "--out",
        "times.wav",
        "--poke",
        "0.374,0.5",
    ])
    .expect("sonify poke parses");
    assert!(matches!(
        cli.command,
        Some(Command::Sonify { ref pokes, ref gestures, .. })
            if pokes == &["0.374,0.5"] && gestures.is_empty()
    ));

    for (name, pokes, gestures) in [
        ("invalid", vec!["2,0.5".to_string()], Vec::new()),
        (
            "mixed",
            vec!["0.5,0.5".to_string()],
            vec!["down:0.5,0.5,0".to_string()],
        ),
    ] {
        let path = std::env::temp_dir().join(format!("numinous_cli_sonify_{name}.wav"));
        let _ = std::fs::remove_file(&path);
        let mut journey = numinous_core::Journey::default();
        let before = journey.clone();
        let code = run(
            Command::Sonify {
                id: "times-tables".to_string(),
                t: 0.0,
                layer: SonifyLayer::Mathematical,
                variation: 0,
                out: path.clone(),
                pokes,
                gestures,
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey, before, "invalid input must not record progress");
        assert!(!path.exists(), "invalid input must not write output");
    }
}

#[test]
fn room_bed_rejects_controls_that_cannot_affect_it_before_progress_or_output() {
    for (name, t, pokes, gestures) in [
        ("phase", 0.5, Vec::new(), Vec::new()),
        ("poke", 0.0, vec!["0.5,0.5".to_string()], Vec::new()),
        (
            "gesture",
            0.0,
            Vec::new(),
            vec!["down:0.5,0.5,0".to_string()],
        ),
    ] {
        let path = std::env::temp_dir().join(format!("numinous_cli_bed_{name}.wav"));
        let _ = std::fs::remove_file(&path);
        let mut journey = numinous_core::Journey::default();
        let before = journey.clone();
        let code = run(
            Command::Sonify {
                id: "times-tables".to_string(),
                t,
                layer: SonifyLayer::RoomBed,
                variation: 7,
                out: path.clone(),
                pokes,
                gestures,
            },
            &mut journey,
        );
        assert_eq!(code, std::process::ExitCode::FAILURE);
        assert_eq!(journey, before);
        assert!(!path.exists());
    }
}

#[test]
fn wav_writer_rejects_invalid_channel_framing_before_creating_a_file() {
    let path = std::env::temp_dir().join("numinous_cli_invalid_channels.wav");
    let _ = std::fs::remove_file(&path);
    assert!(super::write_wav(&path, &[], 16_000, 0).is_err());
    assert!(super::write_wav(&path, &[0.0], 16_000, 2).is_err());
    assert!(!path.exists());
}

#[test]
fn sonify_unknown_room_is_error() {
    let path = std::env::temp_dir().join("numinous_cli_no.wav");
    assert!(
        super::sonify_wav("no-such-room", 0.0, &path, false, RoomRenderInput::plain(),).is_err()
    );
}

#[test]
fn gallery_writes_one_image_per_room() {
    let dir = std::env::temp_dir().join("numinous_gallery_test");
    let _ = std::fs::remove_dir_all(&dir);
    let message = super::gallery(&dir, 40, 40).expect("gallery");
    assert!(message.contains("wrote"));
    let files = std::fs::read_dir(&dir).expect("dir exists").count();
    assert_eq!(files, numinous_core::all_rooms().len());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gallery_replaces_a_hard_link_by_name_without_mutating_its_peer() {
    let dir = std::env::temp_dir().join("numinous_gallery_hard_link_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).expect("gallery fixture");
    let first_id = numinous_core::all_rooms()
        .into_iter()
        .next()
        .expect("catalog is not empty")
        .meta()
        .id;
    let peer = dir.join("peer.txt");
    std::fs::write(&peer, b"sentinel").expect("peer sentinel");
    let derived = dir.join(format!("{first_id}.png"));
    std::fs::hard_link(&peer, &derived).expect("hard link fixture");

    super::gallery(&dir, 40, 40).expect("gallery replaces ordinary names");

    assert_eq!(std::fs::read(&peer).expect("preserved peer"), b"sentinel");
    assert_ne!(std::fs::read(&derived).expect("new PNG"), b"sentinel");
    std::fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn gallery_refuses_a_non_file_derived_member() {
    let dir = std::env::temp_dir().join("numinous_gallery_directory_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).expect("gallery fixture");
    let first_id = numinous_core::all_rooms()
        .into_iter()
        .next()
        .expect("catalog is not empty")
        .meta()
        .id;
    let derived = dir.join(format!("{first_id}.png"));
    std::fs::create_dir(&derived).expect("directory fixture");

    assert!(super::gallery(&dir, 40, 40).is_err());
    assert!(derived.is_dir());
    std::fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn contact_sheet_writes_a_non_empty_file() {
    let path = std::env::temp_dir().join("numinous_contact_test.png");
    let message = super::contact_sheet(&path, 3, 32).expect("contact sheet");
    assert!(message.contains("contact sheet"));
    let size = std::fs::metadata(&path).expect("file exists").len();
    assert!(size > 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn contact_sheet_survives_absurd_dimensions() {
    // `cols * tile` overflowed usize before the Raster clamp (a panic under
    // overflow-checks). A huge cols with a small tile hits that same multiply
    // while keeping the render cheap; the clamp must bound it.
    let path = std::env::temp_dir().join("numinous_contact_huge_test.png");
    let message = super::contact_sheet(&path, usize::MAX, 8).expect("bounded sheet");
    assert!(message.contains("contact sheet"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn play_frame_shows_the_room() {
    let room = numinous_core::room_by_id("times-tables").expect("room");
    let frame = super::play_frame(room.as_ref(), 0.0, 30, 15);
    assert!(frame.contains("Times Tables"));
    assert!(frame.contains(&super::terminal_action_line(room.as_ref())));
    assert!(frame.contains('*'));
}

/// Prose the player reads.
///
/// Only the surfaces that are sentences: the access report pads a
/// two-column table on purpose and is pinned by its own tests, so it
/// would fail this rule for the right reasons.
///
/// Status lines lay out columns with two spaces on purpose. A run of
/// three or more inside a sentence is the tell of a wrapped string
/// literal, and a trailing space is the tell of the same thing at a
/// line end. That mistake has shipped four times in one cycle, twice
/// inside fixes for itself, so it gets a lock rather than more care.
fn assert_reads_as_prose(label: &str, text: &str) {
    for line in text.lines() {
        // Leading indentation is layout (the access report indents its
        // explanations); a run after the first word is the tell.
        assert!(
            !line.trim_start().contains("   "),
            "{label} carries a run of spaces: {line:?}"
        );
        assert_eq!(
            line.trim_end(),
            line,
            "{label} carries trailing whitespace: {line:?}"
        );
    }
}

#[test]
fn the_copy_a_player_reads_never_carries_a_wrapped_literal() {
    let room = numinous_core::room_by_id("lorenz").expect("room");
    assert_reads_as_prose(
        "the posed call",
        &super::call_report("lorenz", None, 7).expect("posed"),
    );
    assert_reads_as_prose(
        "the graded call",
        &super::call_report("lorenz", Some(1.0), 7).expect("graded"),
    );
    assert_reads_as_prose("the not-found message", &super::not_found_message("qqzz"));
    assert_reads_as_prose("the exit tease", &super::viewing_epilogue(room.as_ref()));
    assert_reads_as_prose(
        "the action line",
        &super::terminal_action_line(room.as_ref()),
    );
    assert_reads_as_prose(
        "the room bridge",
        &super::room_bridge_message("times-tables").expect("bridge"),
    );
    let silent = numinous_core::all_rooms()
        .into_iter()
        .find(|room| numinous_core::pose_prediction(room.as_ref(), 1).is_none());
    if let Some(room) = silent {
        let refusal = super::call_report(room.meta().id, None, 1).expect_err("refusal");
        assert_reads_as_prose("the call refusal", &refusal);
    }
}

#[test]
fn the_call_poses_a_question_and_then_answers_it() {
    // The terminal's half of the universal wager: ask once to hear the
    // question, answer with a number, and the truth is named whichever
    // way the call went. Both halves are deterministic and stateless,
    // so the same seed keeps the same question between two runs.
    let posed = super::call_report("lorenz", None, 7).expect("lorenz reads a number");
    assert!(posed.contains("Call it before you look"), "{posed}");
    assert!(
        posed.contains("--guess"),
        "the answer route is named: {posed}"
    );
    assert!(
        !posed.contains("rate"),
        "no verb this command cannot hear: {posed}"
    );
    for line in posed.lines() {
        assert!(!line.contains("  "), "no space runs in player copy: {line}");
    }

    // The graded half names the truth and speaks a band.
    let graded = super::call_report("lorenz", Some(10.0), 7).expect("graded");
    assert!(graded.contains("You called 10"), "{graded}");
    assert!(
        graded.contains("Nailed") || graded.contains("fertile") || graded.contains("gap"),
        "{graded}"
    );

    // Aiming the truth lands the top band, so the grading is real and
    // not merely spoken.
    let room = numinous_core::room_by_id("lorenz").expect("room");
    let prediction = numinous_core::pose_prediction(room.as_ref(), 7).expect("posed");
    let truth = numinous_core::grade_prediction(room.as_ref(), &prediction, 0.0)
        .expect("truth")
        .actual;
    let exact = super::call_report("lorenz", Some(truth), 7).expect("graded");
    assert!(exact.contains("Nailed"), "{exact}");
}

#[test]
fn a_room_with_no_number_says_so_instead_of_inventing_one() {
    let silent = numinous_core::all_rooms()
        .into_iter()
        .find(|room| numinous_core::pose_prediction(room.as_ref(), 1).is_none());
    if let Some(room) = silent {
        let refusal = super::call_report(room.meta().id, None, 1)
            .expect_err("a room with no readout refuses the call");
        assert!(refusal.contains("no moving number"), "{refusal}");
    }
}

#[test]
fn the_terminal_hand_names_a_command_that_exists() {
    // The Action line promised `numinous room <id> --poke x,y`, which is
    // not a subcommand: the one hand this face advertised could not be
    // used. Pinned against the parser itself, so a rename cannot quietly
    // make this copy false again.
    let room = numinous_core::room_by_id("agm-mean").expect("room");
    let line = super::terminal_action_line(room.as_ref());
    let command = line
        .split("numinous ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("the line names a command");
    let parsed = super::Cli::try_parse_from(["numinous", command, "agm-mean", "--poke", "0.5,0.5"]);
    assert!(
        parsed.is_ok(),
        "the Action line names `numinous {command}`, which does not parse"
    );
}

#[test]
fn an_exact_room_id_bridges_even_when_clap_has_a_guess() {
    // The guard deferred to clap's did-you-mean before checking whether
    // the token was a room at all, so scores of real room ids fell
    // through to a stock parser error instead of the bridge.
    let with_a_near_command = numinous_core::all_rooms()
        .into_iter()
        .map(|room| room.meta().id)
        .find(|id| {
            let error = match super::Cli::try_parse_from(["numinous", id]) {
                Ok(_) => return false,
                Err(error) => error,
            };
            error
                .get(clap::error::ContextKind::SuggestedSubcommand)
                .is_some()
        });
    if let Some(id) = with_a_near_command {
        let message = super::room_bridge_message(id).expect("an exact room bridges");
        assert!(message.contains("is a room, not a command"), "{message}");
    }
}

#[test]
fn the_exit_tease_ends_on_a_sentence_not_inside_a_number() {
    assert_eq!(
        super::first_sentence("Pi is 3.14159, and it hides here. Then more."),
        "Pi is 3.14159, and it hides here."
    );
    assert_eq!(
        super::first_sentence("Wait... then this. And more."),
        "Wait... then this."
    );
    assert_eq!(
        super::first_sentence("No terminator here"),
        "No terminator here"
    );
    assert_eq!(super::first_sentence("Ends here."), "Ends here.");
    // Every catalog reveal must tease as a whole sentence, never a
    // fragment cut inside a decimal.
    for room in numinous_core::all_rooms() {
        let tease = super::first_sentence(room.reveal());
        let after = &room.reveal()[tease.len().min(room.reveal().len())..];
        assert!(
            after.is_empty() || after.starts_with(' ') || after.starts_with('\n'),
            "{} teases a fragment: {tease}",
            room.meta().id
        );
    }
}

#[test]
fn the_remix_line_survives_a_paste_into_a_shell() {
    let creation = numinous_core::StudioCreation::new("sin(a*x)", -1.0, 1.0, 1.0).expect("capsule");
    let report = super::open_studio_report(&creation.to_link(), 24, 8).expect("report");
    let line = report
        .lines()
        .find(|line| line.starts_with("remix it:"))
        .expect("the remix verb is named");
    // The link carries & separators; unquoted they split the command in
    // every common shell.
    assert!(
        line.contains("fork \"numinous://"),
        "the link must be quoted: {line}"
    );
    assert!(line.contains("\" --out"), "{line}");
}

#[test]
fn scrub_touch_fragments_drops_gesture_columns_and_keeps_readings() {
    assert_eq!(
        super::scrub_touch_fragments("gamma=0.70  DRAG:TUNE"),
        "gamma=0.70"
    );
    assert_eq!(
        super::scrub_touch_fragments("HOLD: POUR SAND  PILE 12"),
        "PILE 12"
    );
    assert_eq!(super::scrub_touch_fragments("CLICK: SEED A GAP"), "");
    assert_eq!(
        super::scrub_touch_fragments("r=1.41  DRAG:R  agm=1.19"),
        "r=1.41  agm=1.19"
    );
    assert_eq!(
        super::scrub_touch_fragments("plain reading"),
        "plain reading"
    );
    // A bare verb column goes too; a reading that merely contains the
    // letters keeps its place.
    assert_eq!(super::scrub_touch_fragments("N 12  DRAG"), "N 12");
    assert_eq!(super::scrub_touch_fragments("CLICK  GEN 4"), "GEN 4");
    assert_eq!(
        super::scrub_touch_fragments("OVERLAP 3  CLICKS 9"),
        "OVERLAP 3  CLICKS 9"
    );
}

#[test]
fn terminal_action_line_translates_the_hand_and_keeps_the_lever() {
    let room = numinous_core::room_by_id("agm-mean").expect("room");
    let line = super::terminal_action_line(room.as_ref());
    assert!(line.contains("TUNE R"), "lever lost: {line}");
    assert!(line.contains("--poke"), "no terminal hand: {line}");
    assert!(!line.contains("DRAG"), "gesture verb survives: {line}");
    if let Some(ambient) = numinous_core::all_rooms()
        .into_iter()
        .find(|room| room.verb().is_none())
    {
        assert_eq!(
            super::terminal_action_line(ambient.as_ref()),
            numinous_core::DEFAULT_ROOM_ACTION
        );
    }
}

#[test]
fn no_live_frame_advertises_a_gesture_this_face_cannot_hear() {
    // The critique that set this lock: watch and tour printed DRAG:TUNE
    // with no input handling at all. The terminal face translates or
    // scrubs; it never advertises a verb it has no route for. Tour
    // frames ride watch_frame, so this covers all three live loops.
    let style = super::TerminalStyle {
        era: numinous_core::Era::parse("modern").expect("era"),
        color: false,
    };
    for room in numinous_core::all_rooms() {
        for t in [0.0, 0.35, 0.7] {
            let watch = super::watch_frame(room.as_ref(), t, 40, 20, style);
            let play = super::play_frame(room.as_ref(), t, 40, 20);
            for verb in super::UNHEARD_TOUCH_VERBS {
                let marker = format!("{verb}:");
                assert!(
                    !watch.contains(&marker),
                    "{} watch frame advertises {marker} at t={t}",
                    room.meta().id
                );
                assert!(
                    !play.contains(&marker),
                    "{} play frame advertises {marker} at t={t}",
                    room.meta().id
                );
            }
        }
    }
}

#[test]
fn a_room_typed_as_a_command_gets_the_bridge_not_a_parser_error() {
    let message = super::room_bridge_message("times-tables").expect("bridge");
    assert!(message.contains("is a room, not a command"), "{message}");
    assert!(message.contains("numinous watch times-tables"), "{message}");
    assert!(
        message.contains("numinous describe times-tables"),
        "{message}"
    );
}

#[test]
fn a_near_room_token_bridges_with_suggestions() {
    let message = super::room_bridge_message("times-table").expect("bridge");
    assert!(message.contains("times-tables"), "{message}");
    assert!(message.contains("numinous watch"), "{message}");
}

#[test]
fn garbage_tokens_stay_with_the_parser_voice() {
    assert!(super::room_bridge_message("qqqqzzzzxxxxwwww").is_none());
}

#[test]
fn an_unknown_subcommand_error_carries_the_token_the_bridge_needs() {
    // report_cli_parse_error extracts the offending token from clap's
    // error context; this pins that contract against clap upgrades.
    let error = match super::Cli::try_parse_from(["numinous", "times-tables"]) {
        Ok(_) => panic!("a room id parsed as a subcommand"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
    let Some(clap::error::ContextValue::String(token)) =
        error.get(clap::error::ContextKind::InvalidSubcommand)
    else {
        panic!("clap no longer exposes the invalid subcommand token");
    };
    assert_eq!(token, "times-tables");
}

#[test]
fn the_viewing_epilogue_teases_the_reveal_and_routes_to_the_story() {
    let room = numinous_core::room_by_id("times-tables").expect("room");
    let epilogue = super::viewing_epilogue(room.as_ref());
    let mut lines = epilogue.trim().lines();
    let tease = lines.next().expect("tease line");
    assert!(tease.ends_with('.'), "one sentence: {tease}");
    assert!(room.reveal().starts_with(tease), "not the reveal: {tease}");
    assert_eq!(
        lines.next(),
        Some("The story: numinous describe times-tables")
    );
    assert_eq!(lines.next(), None, "the epilogue is two lines");
}

#[test]
fn an_absent_latch_never_reads_as_interrupted() {
    // A host where the Ctrl+C handler cannot install keeps the old
    // die-on-signal behavior; it must never fake an interrupt.
    assert!(!super::interrupted(&None));
}

#[test]
fn gesture_args_parse_and_reject_bad_events() {
    use numinous_core::RoomInput;
    assert_eq!(
        super::parse_gesture_arg("down:0.3,0.4,0.1"),
        Ok(RoomInput::PointerDown {
            x: 0.3,
            y: 0.4,
            t: 0.1
        })
    );
    assert_eq!(
        super::parse_gesture_arg("cancel"),
        Ok(RoomInput::PointerCancel)
    );
    assert!(super::parse_gesture_arg("wiggle:0.1,0.2,0.3").is_err());
    assert!(super::parse_gesture_arg("down:1.5,0.2,0.3").is_err());
    assert!(super::parse_gesture_arg("down:0.1,0.2").is_err());
    let too_many: Vec<String> = (0..=numinous_core::MAX_ROOM_INPUTS)
        .map(|_| "cancel".to_string())
        .collect();
    assert!(super::parse_gestures(&too_many).is_err());
}

#[test]
fn a_gesture_render_matches_the_poke_bridge_for_legacy_rooms() {
    use numinous_core::RoomInput;
    let gesture = [
        RoomInput::PointerDown {
            x: 0.3,
            y: 0.7,
            t: 0.25,
        },
        RoomInput::PointerMove {
            x: 0.5,
            y: 0.5,
            t: 0.26,
        },
        RoomInput::PointerUp {
            x: 0.5,
            y: 0.5,
            t: 0.27,
        },
    ];
    let via_gesture = super::render_report(
        "voronoi",
        40,
        20,
        0.25,
        false,
        super::RoomRenderInput::with_gesture(0, &gesture),
    )
    .expect("gesture render succeeds");
    let via_pokes = super::render_report(
        "voronoi",
        40,
        20,
        0.25,
        false,
        super::RoomRenderInput::new(0, &[(0.3, 0.7), (0.5, 0.5)]),
    )
    .expect("poke render succeeds");
    assert_eq!(via_gesture, via_pokes, "the bridge answers identically");
}

#[test]
fn a_gesture_pins_the_pendulum_in_the_terminal_too() {
    use numinous_core::RoomInput;
    let held = [RoomInput::PointerDown {
        x: 0.3,
        y: 0.4,
        t: 0.1,
    }];
    let early = super::render_report(
        "double-pendulum",
        50,
        30,
        0.2,
        false,
        super::RoomRenderInput::with_gesture(0, &held),
    )
    .expect("held render succeeds");
    let late = super::render_report(
        "double-pendulum",
        50,
        30,
        0.9,
        false,
        super::RoomRenderInput::with_gesture(0, &held),
    )
    .expect("held render succeeds");
    assert_eq!(early, late, "a pinned bob ignores the clock in the CLI too");
}

#[test]
fn play_frames_always_lead_with_the_verb() {
    // Every catalog room answers the hand now; live play frames carry
    // the room's own lever, translated for a keyboard face, never the
    // generic fallback.
    let room = numinous_core::room_by_id("slope-rider").expect("room");
    let frame = super::play_frame(room.as_ref(), 0.0, 30, 15);
    assert!(frame.contains(&super::terminal_action_line(room.as_ref())));
    assert!(!frame.contains(numinous_core::DEFAULT_ROOM_ACTION));
}

#[test]
fn play_frame_changes_with_phase() {
    let room = numinous_core::room_by_id("times-tables").expect("room");
    let a = super::play_frame(room.as_ref(), 0.0, 40, 20);
    let b = super::play_frame(room.as_ref(), 0.6, 40, 20);
    assert_ne!(a, b, "the frame should animate as t changes");
}

#[test]
fn render_png_to_an_unwritable_path_is_error() {
    let bad = std::path::Path::new("no_such_dir_zzz/x.png");
    assert!(
        super::render_png(
            "times-tables",
            8,
            8,
            0.0,
            bad,
            false,
            numinous_core::Era::Modern,
            RoomRenderInput::plain(),
        )
        .is_err()
    );
}

#[test]
fn sonify_to_an_unwritable_path_is_error() {
    let bad = std::path::Path::new("no_such_dir_zzz/x.wav");
    assert!(super::sonify_wav("lissajous", 0.0, bad, false, RoomRenderInput::plain(),).is_err());
}

#[test]
fn the_hidden_room_answers_only_to_rank() {
    assert!(super::find_room("tetractys", false).is_none());
    assert!(super::find_room("tetractys", true).is_some());
    assert!(super::find_room_with_variation("tetractys", false, 42).is_none());
    assert!(
        super::find_room_with_variation("tetractys", true, 42).is_some(),
        "variation lookup must preserve hidden-room access after rank checks"
    );
    // Catalog rooms are open to everyone.
    assert!(super::find_room("lorenz", false).is_some());
    // The unready get the ordinary not-found, no special acknowledgment.
    let err = super::render_report("tetractys", 10, 10, 0.0, false, RoomRenderInput::plain())
        .unwrap_err();
    assert!(err.contains("numinous rooms"), "an ordinary miss: {err}");
    assert!(!err.contains("Order"), "nothing is given away");
    // Echoing the id back is fine: the player typed it. Offering it as a
    // suggestion would confirm the room exists, so nothing may be offered.
    assert!(
        !err.contains("Did you mean"),
        "a suggestion must never confirm the hidden room exists: {err}"
    );
    // The ready see the figure.
    let ok = super::render_report("tetractys", 30, 20, 0.0, true, RoomRenderInput::plain())
        .expect("the figure");
    assert!(ok.contains('#'));
}

#[test]
fn deep_cuts_unlock_with_level() {
    let mut low_journey = numinous_core::Journey::default();
    low_journey.visit("mandelbrot");
    let low = super::reveal_report("mandelbrot", false, false, &low_journey).expect("reveal");
    assert!(
        low.contains("LOCKED: a deeper cut opens at LV 5"),
        "got: {low}"
    );
    assert!(!low.contains("Shishikura"));
    let mut mid_journey = numinous_core::Journey {
        plays: 10,
        ..Default::default()
    };
    mid_journey.visit("mandelbrot");
    let mid = super::reveal_report("mandelbrot", false, false, &mid_journey).expect("reveal");
    assert!(mid.contains("Deeper:"));
    assert!(mid.contains("LOCKED: a deeper cut opens at LV 12"));
    let mut high_journey = numinous_core::Journey {
        plays: 66,
        ..Default::default()
    };
    high_journey.visit("mandelbrot");
    let high = super::reveal_report("mandelbrot", false, false, &high_journey).expect("reveal");
    assert!(high.contains("Deeper still:") && high.contains("Shishikura"));

    let mut at_cap = numinous_core::Journey {
        plays: numinous_core::Journey::MAX_PLAY_SPARKS,
        wins: numinous_core::Journey::MAX_WIN_SPARKS,
        secrets: 100,
        ..Default::default()
    };
    for i in 0..256 {
        at_cap.visit(&format!("room-{i}"));
    }
    at_cap.visit("cult-of-pi");
    assert_eq!(at_cap.level(), numinous_core::MAX_LEVEL);
    let cap = super::reveal_report("cult-of-pi", false, false, &at_cap).expect("reveal");
    assert!(
        cap.contains("Feynman point"),
        "third cut is reachable: {cap}"
    );
    assert!(!cap.contains("4294967295"), "no sentinel leaks: {cap}");
    assert!(
        cap.contains("See also:"),
        "citation unlocks with deep cuts: {cap}"
    );

    let locked = super::reveal_report("mandelbrot", false, false, &low_journey).expect("reveal");
    assert!(
        !locked.contains("See also:"),
        "fresh journey keeps further reading locked: {locked}"
    );
}

#[test]
fn a_boon_opens_a_cut_ahead_of_level() {
    let mut journey = numinous_core::Journey::default(); // level 1
    journey.visit("mandelbrot");
    journey.chosen.insert("cut:mandelbrot:0".to_string());
    let text = super::reveal_report("mandelbrot", false, false, &journey).expect("reveal");
    assert!(text.contains("Deeper:"), "the chosen cut is open: {text}");
    assert!(
        text.contains("LOCKED: a deeper cut opens at LV 12"),
        "the second cut still waits: {text}"
    );
}

#[test]
fn deep_whispers_require_standing() {
    assert!(
        super::describe_report("curtain", false, false, &numinous_core::Journey::default())
            .is_err()
    );
    let deep = super::describe_report(
        "curtain",
        false,
        true,
        &numinous_core::Journey {
            plays: 10,
            ..Default::default()
        },
    )
    .expect("a deeper whisper");
    assert!(deep.contains("veil"), "got: {deep}");
}

#[test]
fn journey_report_shows_the_sky_and_the_rank() {
    let mut journey = numinous_core::Journey::default();
    let fresh = super::journey_report(&journey, &numinous_core::Scoreboard::default(), 0);
    assert!(fresh.contains("0 of"));
    assert!(fresh.contains("Outsider"));
    journey.visit("lorenz");
    let one = super::journey_report(&journey, &numinous_core::Scoreboard::default(), 0);
    assert!(one.contains("1 of"));
    assert!(one.contains("Akousmatikos"));
    assert!(one.contains('#'), "a lit star");
}

#[test]
fn nim_board_draws_stones_per_heap() {
    let text = super::nim_board(&[3, 1, 0]);
    assert!(text.contains("1) O O O"));
    assert!(text.contains("2) O"));
    assert_eq!(text.lines().count(), 3);
}

#[test]
fn resonances_appear_in_the_journey_when_lit() {
    let mut journey = numinous_core::Journey::default();
    journey.visit("mandelbrot");
    journey.visit("julia");
    let report = super::journey_report(&journey, &numinous_core::Scoreboard::default(), 0);
    assert!(report.contains("RESONANCE  The Atlas"));
    assert!(report.contains("atlas"), "the lore line rides along");
}

#[test]
fn tune2_requires_explicit_consent_to_spend() {
    // The flag exists and defaults to off: without it the command names
    // the key source and the cost, spends nothing, and stops.
    let cli = Cli::try_parse_from(["numinous", "tune2", "trance"]).expect("parse");
    let Some(Command::Tune2 { yes, .. }) = cli.command else {
        panic!("tune2 parsed to something else");
    };
    assert!(!yes, "spending must be opt-in, never the default");
    let cli = Cli::try_parse_from(["numinous", "tune2", "trance", "--yes"]).expect("parse");
    let Some(Command::Tune2 { yes, .. }) = cli.command else {
        panic!("tune2 parsed to something else");
    };
    assert!(yes);
}

#[test]
fn a_dead_streak_is_a_record_not_a_claim() {
    let journey = numinous_core::Journey {
        streak: 6,
        last_daily: 100,
        ..numinous_core::Journey::default()
    };
    let alive = super::journey_report(&journey, &numinous_core::Scoreboard::default(), 101);
    assert!(alive.contains("Streak 6."), "yesterday's chain is alive");
    let dead = super::journey_report(&journey, &numinous_core::Scoreboard::default(), 105);
    assert!(
        dead.contains("Best chain 6."),
        "a chain five days cold is a record, not a claim: {dead}"
    );
    assert!(!dead.contains("Streak 6."), "{dead}");
}

#[test]
fn gauntlet_combo_multiplies_clears_and_forgives_misses() {
    // All four cleared: 10*1 + 25*2 + 25*3 + 40*4 = 295.
    assert_eq!(
        numinous_core::gauntlet_total(&[10, 25, 25, 40], &[true, true, true, true]),
        295
    );
    // A miss resets the combo: 10*1 + 0*2 + 25*1 + 40*2 = 115.
    assert_eq!(
        numinous_core::gauntlet_total(&[10, 0, 25, 40], &[true, false, true, true]),
        115
    );
    // Nothing cleared, nothing multiplied.
    assert_eq!(numinous_core::gauntlet_total(&[5, 0, 0, 0], &[false; 4]), 5);
    assert_eq!(numinous_core::gauntlet_total(&[], &[]), 0);
}

#[test]
fn the_trophy_case_shines_and_silhouettes() {
    let mut journey = numinous_core::Journey::default();
    journey.visit("lorenz");
    let case = super::trophies_report(&journey, &numinous_core::Scoreboard::default());
    assert!(case.contains("TROPHIES  1 of"));
    assert!(case.contains("First Light"));
    assert!(case.contains("???"), "the silhouettes beckon");
    assert!(!case.contains("Cartographer"), "unearned names stay hidden");
}

#[test]
fn scores_report_lists_best_first_or_invites_play() {
    let empty = super::scores_report(&numinous_core::Scoreboard::default());
    assert!(empty.contains("No scores yet"));
    let mut board = numinous_core::Scoreboard::default();
    board.record("munch seed:7 board:0", 80);
    board.record("quiz seed:1 rounds:5", 4);
    let table = super::scores_report(&board);
    assert!(table.contains("HIGH SCORES"));
    let munch_pos = table.find("munch").unwrap();
    let quiz_pos = table.find("quiz").unwrap();
    assert!(munch_pos < quiz_pos, "higher score listed first");
}

#[test]
fn pick_seed_honors_the_explicit_seed() {
    let mut j = numinous_core::Journey::default();
    assert_eq!(super::pick_seed(7, false, &mut j), 7);
    // The daily seed is a day count: small, positive, stable within a run.
    let daily = super::pick_seed(7, true, &mut j);
    assert!(daily > 20_000 && daily < 40_000, "got {daily}");
    assert_eq!(super::pick_seed(7, true, &mut j), daily);
}

#[test]
fn daily_seed_boundary_is_deterministic_and_idempotent() {
    let mut journey = numinous_core::Journey::default();
    let before = journey.clone();
    assert_eq!(super::pick_seed_for_day(7, false, 100, &mut journey), 7);
    assert_eq!(journey, before);

    assert_eq!(super::pick_seed_for_day(7, true, 100, &mut journey), 100);
    assert_eq!((journey.last_daily, journey.streak), (100, 1));
    assert_eq!(super::pick_seed_for_day(9, true, 100, &mut journey), 100);
    assert_eq!((journey.last_daily, journey.streak), (100, 1));
    assert_eq!(super::pick_seed_for_day(9, true, 101, &mut journey), 101);
    assert_eq!((journey.last_daily, journey.streak), (101, 2));
    assert_eq!(super::pick_seed_for_day(9, true, 103, &mut journey), 103);
    assert_eq!((journey.last_daily, journey.streak), (103, 1));
}

#[test]
fn trophies_ping_the_moment_they_are_earned() {
    let empty = numinous_core::Scoreboard::default();
    let before_journey = numinous_core::Journey::default();
    let before = super::earned_names(&before_journey, &empty);
    assert!(before.is_empty());
    let mut after = numinous_core::Journey::default();
    after.visit("lorenz");
    after.wins = 1;
    let pings = super::trophy_pings(&before, &after, &empty);
    assert_eq!(pings.len(), 2, "first light and first blood: {pings:?}");
    assert!(pings.iter().any(|p| p.contains("First Light")));
    assert!(pings.iter().any(|p| p.contains("First Blood")));
    // Already-earned trophies never ping again.
    let now = super::earned_names(&after, &empty);
    assert!(super::trophy_pings(&now, &after, &empty).is_empty());
}

#[test]
fn level_ups_announce_lore_and_unlocks() {
    let before = numinous_core::Journey::default();
    let after = numinous_core::Journey {
        plays: 3, // level 3, crossing the LV 3 unlock
        ..Default::default()
    };
    let banner = super::level_up_report(&before, &after).expect("a level was crossed");
    assert!(banner.contains("LEVEL UP"));
    assert!(banner.contains("LV  3"));
    assert!(banner.contains("odd prime"), "the level lore rides along");
    assert!(banner.contains("UNLOCKED") && banner.contains("quiz --hard"));
    assert!(super::level_up_report(&after, &after).is_none());
}

#[test]
fn the_answer_carries_its_freight() {
    let text = super::answer_text();
    assert!(text.starts_with("42."));
    assert!(text.contains("no level 43"));
    assert!(text.contains("contribute"));
    assert!(text.contains("same rules"));
    assert!(text.contains("no cap"));
    assert!(text.contains("Do great things"));
    assert!(!text.contains("outside"), "math is not somewhere else");
}

#[test]
fn jokes_report_lists_and_dissects() {
    let list = super::jokes_report(None);
    assert!(list.contains("frog"));
    let one = super::jokes_report(Some(0));
    assert!(one.contains("Mechanism:"));
    assert!(super::jokes_report(Some(999)).contains("No specimen"));
}

#[test]
fn quiz_remark_scales_with_score() {
    assert!(super::quiz_remark(5, 5).contains("Flawless"));
    assert!(super::quiz_remark(4, 5).contains("Sharp"));
    assert!(super::quiz_remark(0, 5).contains("sneaky"));
    assert_eq!(super::quiz_remark(0, 0), "Play a round!");
}

#[test]
fn sims_report_lists_the_sims_with_levers() {
    let out = super::sims_report();
    assert!(out.contains("tribbles"));
    assert!(out.contains("levers"));
}

#[test]
fn sim_run_renders_and_reads_out() {
    let out = super::sim_run("wing", &["angle-of-attack=20".to_string()], 40, 12).expect("run");
    assert!(out.contains("STALL"), "got: {out}");
}

#[test]
fn render_color_report_emits_truecolor() {
    let out = super::render_color_report(
        "times-tables",
        20,
        20,
        0.0,
        false,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: true,
        },
        RoomRenderInput::plain(),
    )
    .expect("color render");
    assert!(out.contains("\x1b[38;2;"), "has truecolor escapes");
    assert!(
        super::render_color_report(
            "nope",
            20,
            20,
            0.0,
            false,
            TerminalStyle {
                era: numinous_core::Era::Modern,
                color: true,
            },
            RoomRenderInput::plain(),
        )
        .is_err()
    );
}

#[test]
fn eras_change_the_color_frame() {
    let modern = super::render_color_report(
        "chaos-game",
        20,
        20,
        0.0,
        false,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: true,
        },
        RoomRenderInput::plain(),
    )
    .expect("render");
    let phosphor = super::render_color_report(
        "chaos-game",
        20,
        20,
        0.0,
        false,
        TerminalStyle {
            era: numinous_core::Era::Phosphor,
            color: true,
        },
        RoomRenderInput::plain(),
    )
    .expect("render");
    assert_ne!(modern, phosphor);
}

/// Every SGR escape in `text`, as the bodies between `\x1b[` and `m`.
///
/// Only SGR is collected. Cursor control is not color: `\x1b[H`, `\x1b[J`
/// and `\x1b[K` position and clear, and a `NO_COLOR` surface is still
/// allowed to paint in place.
/// Walk `text` once, returning its SGR codes and the text without them.
///
/// One walk rather than two, so the list of codes and the stripped text can
/// never disagree about what counted as color.
fn scan_sgr(text: &str) -> (Vec<String>, String) {
    let mut codes = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("\x1b[") {
        plain.push_str(&rest[..start]);
        let tail = &rest[start + 2..];
        let Some(index) = tail.find(|c: char| !c.is_ascii_digit() && c != ';') else {
            // A truncated escape: nothing terminates it, so there is no
            // color here and no more to find.
            plain.push_str(&rest[start..]);
            return (codes, plain);
        };
        // `index` is a byte offset at a character boundary, and the
        // character there may be several bytes wide. Stepping one byte past
        // it would split that character and panic, which hostile input
        // could reach on purpose.
        let terminator = tail[index..]
            .chars()
            .next()
            .expect("find returned a character boundary");
        let after = index + terminator.len_utf8();
        if terminator == 'm' {
            codes.push(tail[..index].to_string());
        } else {
            // Not color. Cursor control and friends survive stripping,
            // because a NO_COLOR surface may still paint in place.
            plain.push_str(&rest[start..start + 2 + after]);
        }
        rest = &tail[after..];
    }
    plain.push_str(rest);
    (codes, plain)
}

/// Every SGR escape in `text`, as the bodies between `\x1b[` and `m`.
fn sgr_codes(text: &str) -> Vec<String> {
    scan_sgr(text).0
}

/// `text` with every SGR escape removed and everything else left alone.
fn strip_sgr(text: &str) -> String {
    scan_sgr(text).1
}

#[test]
fn every_accessibility_switch_is_written_down_where_a_player_looks() {
    // The gap this closes: all three switches were honored everywhere and
    // documented nowhere a player would find them. They appeared in the
    // roadmap, which is a planning document, and in Rust doc comments,
    // which are for whoever is editing the code. A switch a player cannot
    // discover is not a switch that shipped.
    //
    // The expectations come from the code's own list rather than from a
    // second list written here, so a fourth switch cannot be added and left
    // undocumented: it would fail this the moment it exists.
    const PLAYING: &str = include_str!("../../../docs/PLAYING.md");
    let settings = super::access_settings(None, None, None);
    assert_eq!(settings.len(), 3, "a switch was added or removed");
    for setting in &settings {
        assert!(
            PLAYING.contains(setting.variable),
            "docs/PLAYING.md never mentions {}",
            setting.variable
        );
        assert!(
            !setting.what.is_empty(),
            "{} is listed with no explanation",
            setting.variable
        );
    }
    // The command that prints them has to be findable too.
    assert!(
        PLAYING.contains("numinous access"),
        "the docs never tell a player the access command exists"
    );
}

#[test]
fn the_access_report_names_every_room_on_the_known_limit_lists() {
    // The report once said "three" color-free offenders while the code's
    // list held four, and pointed the player at a repository file that
    // installs do not ship. Now the names come from the same public
    // lists the registry tests enforce, and this proves the plumbing:
    // every listed room appears in the report itself.
    let report = super::access_report(&super::access_settings(None, None, None));
    const PLAYING: &str = include_str!("../../../docs/PLAYING.md");
    for room in numinous_core::KNOWN_OVER_FLASH_BUDGET
        .iter()
        .chain(numinous_core::RESPONSE_INVISIBLE_WITHOUT_COLOR.iter())
    {
        assert!(
            report.contains(room),
            "the access report no longer names {room}"
        );
        assert!(
            PLAYING.contains(room),
            "docs/PLAYING.md's disclosure no longer names {room}"
        );
    }
    assert!(
        !report.contains("ROADMAP"),
        "the report points at a file installs do not ship"
    );
    assert!(
        report.contains("need a mouse or a controller"),
        "the keyboard-to-touch boundary must be stated"
    );
}

#[test]
fn the_accessibility_report_says_which_switches_are_on() {
    // Matched on the whole status line rather than on the word alone. The
    // prose says "turn it off", so searching for "off" anywhere finds the
    // instructions and reports every switch as off no matter what it is.
    let status_line = |variable: &str, status: &str| format!("  {variable:<24} {status}\n");

    let off = super::access_report(&super::access_settings(None, None, None));
    for variable in ["NUMINOUS_REDUCED_MOTION", "NUMINOUS_MONO_AUDIO", "NO_COLOR"] {
        assert!(off.contains(variable), "{variable} missing from the report");
        assert!(
            off.contains(&status_line(variable, "off")),
            "{variable} does not read as off:\n{off}"
        );
    }

    let value = Some(std::ffi::OsStr::new("1"));
    let all_on = super::access_report(&super::access_settings(value, value, value));
    for variable in ["NUMINOUS_REDUCED_MOTION", "NUMINOUS_MONO_AUDIO", "NO_COLOR"] {
        assert!(
            all_on.contains(&status_line(variable, "ON")),
            "{variable} does not read as on:\n{all_on}"
        );
        assert!(!all_on.contains(&status_line(variable, "off")));
    }

    // The one that is easy to get backwards. NO_COLOR is present-means-off
    // for color, so the switch is ON. Reporting it the other way round
    // would tell a player color is off when it is on.
    let no_color_only = super::access_settings(None, None, Some(std::ffi::OsStr::new("1")));
    assert!(no_color_only[2].on, "NO_COLOR=1 must read as switched on");
    assert!(!no_color_only[0].on && !no_color_only[1].on);

    // Both surfaces must explain the empty case, because it is the one a
    // player can get wrong while believing they got it right: `NO_COLOR=""`
    // looks set and is not. The report said "anything at all turns it on",
    // which contradicted the assertions just below and would have left
    // someone with a switch they thought they had thrown.
    const PLAYING: &str = include_str!("../../../docs/PLAYING.md");
    for (surface, text) in [("the report", off.as_str()), ("PLAYING.md", PLAYING)] {
        assert!(
            text.contains("empty"),
            "{surface} never mentions that an empty value counts as off"
        );
        assert!(
            text.contains("=0"),
            "{surface} never mentions that =0 counts as on"
        );
    }

    // Empty is not set, for all three, which is the shared convention.
    let empty = super::access_settings(
        Some(std::ffi::OsStr::new("")),
        Some(std::ffi::OsStr::new("")),
        Some(std::ffi::OsStr::new("")),
    );
    assert!(empty.iter().all(|setting| !setting.on));

    // And =0 is set, for all three, because you still wrote it.
    let zero = super::access_settings(
        Some(std::ffi::OsStr::new("0")),
        Some(std::ffi::OsStr::new("0")),
        Some(std::ffi::OsStr::new("0")),
    );
    assert!(zero.iter().all(|setting| setting.on));
}

#[test]
fn the_accessibility_report_fits_an_eighty_column_terminal() {
    // It is read by whoever most needs it to be readable, and a report that
    // wraps at random is a poor way to explain that the display is
    // configurable.
    let value = Some(std::ffi::OsStr::new("1"));
    for report in [
        super::access_report(&super::access_settings(None, None, None)),
        super::access_report(&super::access_settings(value, value, value)),
    ] {
        for line in report.lines() {
            assert!(
                line.chars().count() <= 80,
                "{} columns: {line}",
                line.chars().count()
            );
        }
        // No color of its own, since one of the three switches turns color
        // off and a report that ignored that would be a poor advertisement.
        assert!(sgr_codes(&report).is_empty());
    }
}

#[test]
fn the_escape_scanner_finds_color_and_ignores_cursor_control() {
    // This scanner is the instrument the next test measures with, so it is
    // calibrated first. An instrument that found nothing would report every
    // surface as clean.
    assert_eq!(sgr_codes("\x1b[91mR\x1b[0m"), vec!["91", "0"]);
    assert_eq!(sgr_codes("\x1b[38;2;1;2;3mx"), vec!["38;2;1;2;3"]);
    assert!(sgr_codes("\x1b[H\x1b[2J\x1b[Kplain").is_empty());
    assert!(sgr_codes("no escapes here").is_empty());
    // A truncated escape must not be read as a color, and must not spin.
    assert!(sgr_codes("\x1b[").is_empty());
    assert!(sgr_codes("\x1b[12").is_empty());
    // Compound and unusual codes count too, so the boards are not held to a
    // list of the colors they happen to use today.
    assert_eq!(sgr_codes("\x1b[1;91mx\x1b[0m"), vec!["1;91", "0"]);
    assert_eq!(sgr_codes("\x1b[97mx"), vec!["97"]);

    // Stripping removes exactly the color and nothing else.
    assert_eq!(strip_sgr("\x1b[91mR\x1b[0m"), "R");
    assert_eq!(strip_sgr("\x1b[1;91mR\x1b[0m"), "R");
    assert_eq!(
        strip_sgr("\x1b[H\x1b[2Jkeep\x1b[K"),
        "\x1b[H\x1b[2Jkeep\x1b[K"
    );
    assert_eq!(strip_sgr("plain"), "plain");
    assert_eq!(strip_sgr("\x1b[12"), "\x1b[12");

    // A multi-byte character where the terminator belongs must not split a
    // character and panic. This is reachable by hostile input, and a test
    // that panics instead of failing is a test that stops the suite.
    for hostile in ["\x1b[\u{00e9}m", "\x1b[1\u{4e2d}", "\x1b[\u{1f600}0m"] {
        let (codes, plain) = scan_sgr(hostile);
        assert!(codes.is_empty(), "{hostile:?} yielded {codes:?}");
        assert_eq!(plain, hostile, "hostile input must survive unchanged");
    }
}

#[test]
fn every_game_board_honors_no_color_and_still_says_which_is_which() {
    // The pictures honored NO_COLOR and the games painted over it. All
    // three boards wrote escapes inline and none of them consulted the
    // setting, so a NO_COLOR player got a color-free room and a colored
    // game in the same session.
    //
    // Each board is checked twice over. With color off it must emit no SGR
    // at all, and with color on it must emit some, so this measures the
    // setting rather than boards that lost their color everywhere. The
    // marks that carry the meaning are asserted in BOTH modes, which is the
    // color-independence half: the color may repeat the answer, never own
    // it.
    let arcade = numinous_core::munch_arcade::Arcade::new(7);
    let stalks = numinous_core::hackenbush::Stalks::from(vec![
        vec![
            numinous_core::hackenbush::Color::Red,
            numinous_core::hackenbush::Color::Blue,
        ],
        vec![numinous_core::hackenbush::Color::Blue],
    ]);
    let mut party = numinous_core::party::Party::new(5);
    if let Some(index) = numinous_core::party::edge_index(5, 0, 1) {
        party.edges[index] = numinous_core::party::Shade::Red;
    }
    if let Some(index) = numinous_core::party::edge_index(5, 0, 2) {
        party.edges[index] = numinous_core::party::Shade::Blue;
    }

    /// One board under test: what to call it, how to draw it at a given
    /// color setting, and the marks that must survive without color.
    type BoardCase = (&'static str, Box<dyn Fn(bool) -> String>, Vec<&'static str>);

    let boards: [BoardCase; 3] = [
        (
            "arcade",
            Box::new(move |color| super::arcade_text(&arcade, color)),
            // Not "@": a fresh board has the Muncher on an uneaten cell,
            // which keeps its digits. The angle brackets are what says
            // where it is standing, and they are there in both states.
            vec![">", "<"],
        ),
        (
            "garden",
            Box::new(move |color| super::garden_text(&stalks, color)),
            vec!["R", "B"],
        ),
        (
            "party",
            Box::new(move |color| super::party_board_text(&party, 5, color)),
            vec!["R", "B", "."],
        ),
    ];

    for (name, draw, marks) in boards {
        let plain = draw(false);
        assert!(
            sgr_codes(&plain).is_empty(),
            "{name} still emits {:?} with color off",
            sgr_codes(&plain)
        );
        let colored = draw(true);
        assert!(
            !sgr_codes(&colored).is_empty(),
            "{name} emits no color with color on, so the check above proves nothing"
        );
        for mark in marks {
            assert!(
                plain.contains(mark),
                "{name} loses the mark {mark:?} without color:\n{plain}"
            );
            assert!(
                colored.contains(mark),
                "{name} loses the mark {mark:?} with color:\n{colored}"
            );
        }
        // Same board either way: stripping the color must leave exactly the
        // uncolored drawing, not a differently shaped one. Stripped
        // generically rather than against a list of the codes these boards
        // happen to use today, so a board that later reaches for 92, 97 or
        // a compound 1;91 is still compared rather than failed.
        assert_eq!(
            strip_sgr(&colored),
            plain,
            "{name} draws a different board per mode"
        );
    }
}

#[test]
fn the_muncher_is_findable_on_an_uneaten_cell_without_color() {
    // The defect this pins: the Muncher used to be yellow digits in
    // ordinary brackets, so on an uneaten cell it drew `[30]` and every
    // other cell drew `[30]` too. Without color you could not see where you
    // were standing, and a fresh board is exactly that case.
    let mut run = numinous_core::munch_arcade::Arcade::new(7);
    assert!(
        !run.eaten[run.muncher],
        "a fresh board should have the Muncher on an uneaten cell"
    );
    let plain = super::arcade_text(&run, false);
    let standing = format!(">{:>2}<", run.board.numbers[run.muncher]);
    assert!(
        plain.contains(&standing),
        "the Muncher is not marked at {standing}:\n{plain}"
    );
    // Exactly one cell claims to be the Muncher, or the mark says nothing.
    assert_eq!(
        plain.matches('>').count(),
        1,
        "more than one cell is marked as the Muncher:\n{plain}"
    );
    // The digits are still readable, which was the reason for the old
    // behaviour and is worth keeping.
    assert!(plain.contains(&format!("{:>2}", run.board.numbers[run.muncher])));

    // And once the cell is eaten it still marks the same way.
    run.eaten[run.muncher] = true;
    let eaten = super::arcade_text(&run, false);
    assert!(eaten.contains("> @<"), "eaten Muncher unmarked:\n{eaten}");
    assert_eq!(eaten.matches('>').count(), 1);

    // Every row is the same width in both states, so the grid still lines
    // up. A four column cell replaced by a three or five column one would
    // shear the board.
    for board in [&plain, &eaten] {
        let widths: Vec<usize> = board
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::chars)
            .map(Iterator::count)
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] == pair[1]),
            "rows are not all the same width: {widths:?}"
        );
    }
}

#[test]
fn painting_is_the_only_place_a_board_can_add_color() {
    assert_eq!(super::painted(true, "91", "R"), "\x1b[91mR\x1b[0m");
    assert_eq!(super::painted(false, "91", "R"), "R");
    // Color off must not leave a bare reset behind, which is the exact
    // shape of the defect removed from the NO_COLOR work earlier.
    assert!(sgr_codes(&super::painted(false, "91", "R")).is_empty());
}

#[test]
fn reduced_motion_stops_the_show_advancing_by_itself() {
    // The setting's whole point. Full motion moves the gallery on a frame
    // count; reduced motion hands that decision to the player, which is how
    // the App already behaves because its held phase never completes the
    // sweep that carries it into the next room.
    assert_eq!(
        super::show_advance(numinous_core::Motion::Reduced, 6.0, 30.0),
        super::Advance::Player
    );
    assert_eq!(
        super::show_advance(numinous_core::Motion::Full, 6.0, 30.0),
        super::Advance::Timer(180)
    );
    // A room must never be given zero frames, or full motion would advance
    // instantly and read as no gallery at all.
    for (seconds, fps) in [(0.0, 0.0), (-5.0, -1.0), (0.001, 0.001)] {
        let super::Advance::Timer(frames) =
            super::show_advance(numinous_core::Motion::Full, seconds, fps)
        else {
            panic!("full motion must advance on a timer at {seconds} seconds and {fps} fps");
        };
        assert!(frames >= 2, "{seconds}s at {fps}fps gave {frames} frames");
    }
}

#[test]
fn a_held_show_advances_only_as_far_as_the_player_asks() {
    let rooms = numinous_core::all_rooms()
        .into_iter()
        .take(3)
        .collect::<Vec<_>>();
    let style = TerminalStyle {
        era: numinous_core::Era::Modern,
        color: true,
    };
    let expected: Vec<&str> = rooms.iter().map(|room| room.meta().id).collect();

    // Two blank lines then a quit: three rooms seen, and the third only
    // because the player was still there to see it.
    let mut journey = numinous_core::Journey::default();
    let mut out = Vec::new();
    let shown = super::tour_held(
        &rooms,
        &mut journey,
        super::TourFrame {
            width: 24,
            height: 16,
            style,
        },
        None,
        &mut std::io::Cursor::new(b"\n\nq\n".to_vec()),
        &mut out,
    );
    assert_eq!(shown, expected);
    // Leaving on q earns the same staircase-completing epilogue the
    // timer tour's Ctrl+C prints, for the room the player left on.
    let printed = String::from_utf8_lossy(&out);
    let last = rooms.last().expect("three rooms");
    assert!(
        printed.contains(&format!("The story: numinous describe {}", last.meta().id)),
        "the held tour's exit must route to the story"
    );

    // One room per answer, and no answers means one room. If this ever
    // returns more than the input allows, something is advancing on its
    // own, which is the defect this whole change exists to remove.
    for (input, wanted) in [
        (&b""[..], 1),
        (&b"\n"[..], 2),
        (&b"q\n"[..], 1),
        (&b"QUIT\n"[..], 1),
        // Past the end of the room list, so the gallery must come back
        // around rather than stop early.
        (&b"\n\n\n\n"[..], 5),
    ] {
        let mut journey = numinous_core::Journey::default();
        let mut out = Vec::new();
        let shown = super::tour_held(
            &rooms,
            &mut journey,
            super::TourFrame {
                width: 24,
                height: 16,
                style,
            },
            None,
            &mut std::io::Cursor::new(input.to_vec()),
            &mut out,
        );
        assert_eq!(
            shown.len(),
            wanted,
            "input {:?} showed {shown:?}",
            String::from_utf8_lossy(input)
        );
    }

    // An empty catalog must return rather than cycle forever on nothing.
    let mut journey = numinous_core::Journey::default();
    let mut out = Vec::new();
    assert!(
        super::tour_held(
            &[],
            &mut journey,
            super::TourFrame {
                width: 24,
                height: 16,
                style,
            },
            None,
            &mut std::io::Cursor::new(b"\n\n\n".to_vec()),
            &mut out,
        )
        .is_empty()
    );
}

#[test]
fn a_held_room_rests_on_its_postcard_and_says_how_to_move_on() {
    let rooms = numinous_core::all_rooms()
        .into_iter()
        .take(1)
        .collect::<Vec<_>>();
    let mut journey = numinous_core::Journey::default();
    let mut out = Vec::new();
    super::tour_held(
        &rooms,
        &mut journey,
        super::TourFrame {
            width: 24,
            height: 16,
            style: TerminalStyle {
                era: numinous_core::Era::Modern,
                color: true,
            },
        },
        None,
        &mut std::io::Cursor::new(b"q\n".to_vec()),
        &mut out,
    );
    let screen = String::from_utf8(out).expect("utf-8");
    // A held gallery that does not say how to leave it is a trap.
    assert!(
        screen.contains("Enter for the next room, q to leave."),
        "no way out offered: {screen}"
    );
    // Resting on the postcard phase rather than on frame zero.
    let room = rooms.first().expect("one room");
    assert!(
        screen.contains(&super::tour_screen(
            room.as_ref(),
            room.postcard_t(),
            24,
            16,
            TerminalStyle {
                era: numinous_core::Era::Modern,
                color: true,
            },
        )),
        "the held frame is not the postcard frame"
    );
}

#[test]
fn the_shows_title_card_carries_no_color_when_color_is_off() {
    // The picture already honored NO_COLOR; the chrome written over it did
    // not, so a NO_COLOR player got a color-free room under a bold escape
    // and a reset. Asserted on the composed screen, because that is where
    // the defect lived and the renderer alone looked clean.
    let room = numinous_core::room_by_id("chaos-game").expect("room");
    let plain = super::tour_screen(
        room.as_ref(),
        0.0,
        24,
        16,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: false,
        },
    );
    assert!(
        plain.contains("Chaos Game"),
        "the title card is still there"
    );
    for escape in ["\x1b[1m", "\x1b[0m", "\x1b[38;2;", "\x1b[48;2;"] {
        assert!(
            !plain.contains(escape),
            "title card frame still emits {}",
            escape.escape_debug()
        );
    }
    // The same frame in color still says it in bold, so the check above is
    // measuring the setting and not a title card that lost its emphasis
    // everywhere.
    let colored = super::tour_screen(
        room.as_ref(),
        0.0,
        24,
        16,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: true,
        },
    );
    assert!(colored.contains("\x1b[1m"));
}

#[test]
fn watch_frame_paints_in_place_with_a_status_line() {
    let room = numinous_core::room_by_id("chaos-game").expect("room");
    let frame = super::watch_frame(
        room.as_ref(),
        0.5,
        24,
        16,
        TerminalStyle {
            era: numinous_core::Era::Modern,
            color: true,
        },
    );
    assert!(
        frame.starts_with("\x1b[H"),
        "repaints from home, no flicker"
    );
    assert!(frame.contains("Chaos Game"));
    assert!(frame.contains("t = 0.50"));
}

#[test]
fn plot_discovery_resolves_recipe_seed_and_list() {
    assert_eq!(
        super::resolve_plot_source(Some("x"), None, None, 0).expect("manual"),
        numinous_core::PlotSource::Manual("x".to_string())
    );
    assert_eq!(
        super::resolve_plot_source(None, Some(0), None, 0).expect("recipe"),
        numinous_core::PlotSource::Recipe(0)
    );
    assert_eq!(
        super::resolve_plot_source(None, None, Some(7), 2).expect("auto"),
        numinous_core::PlotSource::Seeded {
            seed: 7,
            auto_step: Some(2)
        }
    );
    assert!(super::resolve_plot_source(None, None, None, 0).is_err());
    assert!(super::resolve_plot_source(Some("x"), Some(1), None, 0).is_err());
    let mut journey = numinous_core::Journey::default();
    let code = run(
        Command::Plot {
            expr: None,
            recipe: None,
            seed: None,
            auto_step: 0,
            list_recipes: true,
            xmin: -1.0,
            xmax: 1.0,
            a: 1.0,
            animate: false,
            amin: 0.0,
            amax: 1.0,
            width: 24,
            height: 8,
            save: None,
            title: None,
            author: None,
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::SUCCESS);
    assert_eq!(journey.plays, 0, "listing recipes is not a play");
}

#[test]
fn plot_report_draws_a_known_function() {
    let out = super::plot_report("x", -1.0, 1.0, 0.0, 24, 8).expect("plot");
    assert!(out.contains("y = x"));
    assert!(out.contains('#'));
}

#[test]
fn plot_report_uses_the_parameter() {
    // With a=0 the line is flat; with a large a it spans more, so ink differs.
    let flat = super::plot_report("a * x", -1.0, 1.0, 0.0, 24, 8).expect("plot");
    let steep = super::plot_report("a * x", -1.0, 1.0, 5.0, 24, 8).expect("plot");
    assert_ne!(flat, steep);
}

#[test]
fn plot_report_accepts_the_full_studio_function_rung() {
    let source = "min(max(mod(floor(3*x), 5), 1), 3)";
    let out = super::plot_report(source, -2.0, 2.0, 0.0, 32, 10).expect("plot");
    assert!(out.contains(source));
    assert!(out.contains('#'));
}

#[test]
fn plot_report_rejects_bad_input() {
    assert!(super::plot_report("sin(", -1.0, 1.0, 0.0, 24, 8).is_err());
    assert!(super::plot_report("x", 1.0, 1.0, 0.0, 24, 8).is_err()); // xmax not > xmin
}

#[test]
fn studio_plot_paths_reject_dimensions_above_the_cli_limit() {
    assert!(super::plot_report("x", -1.0, 1.0, 0.0, 4096, 2).is_ok());
    for (width, height) in [(4097, 2), (2, 4097)] {
        let error =
            super::plot_report("x", -1.0, 1.0, 0.0, width, height).expect_err("oversized plot");
        assert!(error.contains("CLI limit"), "unexpected error: {error}");
    }

    let creation = numinous_core::StudioCreation::new("x", -1.0, 1.0, 0.0).expect("creation");
    let error =
        super::open_studio_report(&creation.to_link(), 4097, 2).expect_err("oversized opened plot");
    assert!(error.contains("CLI limit"), "unexpected error: {error}");
}

#[test]
fn plot_save_writes_a_portable_studio_file() {
    let path = std::env::temp_dir().join("numinous_cli_studio_save_test.num");
    let _ = std::fs::remove_file(&path);
    let message =
        save_studio_creation("sin(a*x)", -2.0, 2.0, 0.5, None, None, &path).expect("studio save");
    assert!(message.contains("numinous://studio?"));
    let text = std::fs::read_to_string(&path).expect("saved file");
    let creation = numinous_core::StudioCreation::from_num_file(&text).expect("round trip");
    assert_eq!(creation.source(), "sin(a*x)");
    assert_eq!(creation.xmin(), -2.0);
    assert_eq!(creation.xmax(), 2.0);
    assert_eq!(creation.a(), 0.5);
    assert!(
        save_studio_creation("sin(a*x)", -2.0, 2.0, 0.5, None, None, &path).is_err(),
        "save should not overwrite an existing share file"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_titled_save_round_trips_and_the_report_names_it() {
    let path = std::env::temp_dir().join("numinous_cli_titled_save_test.num");
    let _ = std::fs::remove_file(&path);
    let message = save_studio_creation(
        "sin(a*x)",
        -2.0,
        2.0,
        0.5,
        Some("Slow Waves"),
        Some("A Curious Mind"),
        &path,
    )
    .expect("titled save");
    assert!(message.contains("link: numinous://studio?"));

    let report =
        super::open_studio_report(path.to_str().expect("utf8 path"), 24, 8).expect("report");
    assert!(report.contains("title=Slow Waves"), "{report}");
    assert!(report.contains("author=A Curious Mind"), "{report}");
    assert!(
        report.contains("title=Slow%20Waves"),
        "the link carries the identity too: {report}"
    );
    let _ = std::fs::remove_file(&path);

    // A name that could steer a terminal never reaches the file.
    assert!(save_studio_creation("x", -1.0, 1.0, 0.0, Some("bad\u{7}title"), None, &path).is_err());
    assert!(!path.exists(), "a refused save writes nothing");
}

#[test]
fn a_title_without_save_is_refused_before_progress() {
    let mut journey = numinous_core::Journey::default();
    let code = run(
        Command::Plot {
            expr: Some("x".to_string()),
            recipe: None,
            seed: None,
            auto_step: 0,
            list_recipes: false,
            xmin: -1.0,
            xmax: 1.0,
            a: 1.0,
            animate: false,
            amin: 0.0,
            amax: 1.0,
            width: 24,
            height: 8,
            save: None,
            title: Some("Slow Waves".to_string()),
            author: None,
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey.plays, 0, "a refused plot is not a play");
}

#[test]
fn plot_save_and_animate_is_rejected_before_progress() {
    let path = std::env::temp_dir().join("numinous_cli_studio_save_animate_test.num");
    let _ = std::fs::remove_file(&path);
    let mut journey = numinous_core::Journey::default();
    let code = run(
        Command::Plot {
            expr: Some("x".to_string()),
            recipe: None,
            seed: None,
            auto_step: 0,
            list_recipes: false,
            xmin: -1.0,
            xmax: 1.0,
            a: 1.0,
            animate: true,
            amin: 0.0,
            amax: 1.0,
            width: 24,
            height: 8,
            save: Some(path.clone()),
            title: None,
            author: None,
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey.plays, 0);
    assert!(!path.exists());
}

#[test]
fn invalid_animated_plot_is_rejected_before_progress() {
    let mut journey = numinous_core::Journey::default();
    let code = run(
        Command::Plot {
            expr: Some("x".to_string()),
            recipe: None,
            seed: None,
            auto_step: 0,
            list_recipes: false,
            xmin: -1.0,
            xmax: 1.0,
            a: 1.0,
            animate: true,
            amin: 0.0,
            amax: 1.0,
            width: 4097,
            height: 8,
            save: None,
            title: None,
            author: None,
        },
        &mut journey,
    );

    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey.plays, 0);
}

#[test]
fn plot_save_waits_for_a_valid_still_plot() {
    let path = std::env::temp_dir().join("numinous_cli_studio_bad_width_test.num");
    let _ = std::fs::remove_file(&path);
    let mut journey = numinous_core::Journey::default();
    let code = run(
        Command::Plot {
            expr: Some("x".to_string()),
            recipe: None,
            seed: None,
            auto_step: 0,
            list_recipes: false,
            xmin: -1.0,
            xmax: 1.0,
            a: 1.0,
            animate: false,
            amin: 0.0,
            amax: 1.0,
            width: 1,
            height: 8,
            save: Some(path.clone()),
            title: None,
            author: None,
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey.plays, 0);
    assert!(!path.exists(), "failed plot must not leave a .num file");
}

#[test]
fn plot_save_waits_for_finite_samples() {
    let path = std::env::temp_dir().join("numinous_cli_studio_undefined_test.num");
    let _ = std::fs::remove_file(&path);
    let mut journey = numinous_core::Journey::default();
    let code = run(
        Command::Plot {
            expr: Some("ln(-1)".to_string()),
            recipe: None,
            seed: None,
            auto_step: 0,
            list_recipes: false,
            xmin: -2.0,
            xmax: -1.0,
            a: 1.0,
            animate: false,
            amin: 0.0,
            amax: 1.0,
            width: 24,
            height: 8,
            save: Some(path.clone()),
            title: None,
            author: None,
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey.plays, 0);
    assert!(!path.exists(), "undefined plot must not leave a .num file");
}

#[test]
fn failed_plot_save_does_not_record_progress() {
    let path = std::env::temp_dir().join("numinous_cli_studio_existing_test.num");
    std::fs::write(&path, "already here").expect("seed existing file");
    let mut journey = numinous_core::Journey::default();
    let code = run(
        Command::Plot {
            expr: Some("x".to_string()),
            recipe: None,
            seed: None,
            auto_step: 0,
            list_recipes: false,
            xmin: -1.0,
            xmax: 1.0,
            a: 1.0,
            animate: false,
            amin: 0.0,
            amax: 1.0,
            width: 24,
            height: 8,
            save: Some(path.clone()),
            title: None,
            author: None,
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey.plays, 0);
    assert_eq!(
        std::fs::read_to_string(&path).expect("existing file"),
        "already here",
        "save failure must not overwrite the existing file"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn open_studio_renders_saved_file_and_link() {
    let path = std::env::temp_dir().join("numinous_cli_studio_open_test.num");
    let _ = std::fs::remove_file(&path);
    save_studio_creation("sin(a*x)", -2.0, 2.0, 0.5, None, None, &path).expect("studio save");

    let from_file =
        open_studio_report(path.to_str().expect("utf8 path"), 32, 10).expect("open saved file");
    assert!(from_file.contains("Studio creation"));
    assert!(from_file.contains("expr=sin(a*x)"));
    assert!(from_file.contains("link=numinous://studio?"));
    assert!(from_file.contains('#'));

    let creation = numinous_core::StudioCreation::from_num_file(
        &std::fs::read_to_string(&path).expect("saved file"),
    )
    .expect("creation");
    let from_link = open_studio_report(&creation.to_link(), 32, 10).expect("open link");
    assert!(from_link.contains("expr=sin(a*x)"));
    assert!(from_link.contains('#'));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn open_studio_subcommand_parses_and_records_success() {
    let path = std::env::temp_dir().join("numinous_cli_studio_run_open_test.num");
    let _ = std::fs::remove_file(&path);
    save_studio_creation("x", -1.0, 1.0, 0.0, None, None, &path).expect("studio save");

    let cli = Cli::try_parse_from([
        "numinous",
        "open-studio",
        path.to_str().expect("utf8 path"),
        "--width",
        "24",
        "--height",
        "8",
    ])
    .expect("parse open-studio");
    let Some(command) = cli.command else {
        panic!("command parsed");
    };
    let mut journey = numinous_core::Journey::default();
    let code = run(command, &mut journey);
    assert_eq!(code, std::process::ExitCode::SUCCESS);
    assert_eq!(journey.plays, 1);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn failed_open_studio_does_not_record_progress() {
    let mut journey = numinous_core::Journey::default();
    let missing = std::env::temp_dir().join("numinous_cli_studio_missing_test.num");
    let _ = std::fs::remove_file(&missing);
    let code = run(
        Command::OpenStudio {
            input: missing.to_string_lossy().to_string(),
            width: 32,
            height: 10,
        },
        &mut journey,
    );
    assert_eq!(code, std::process::ExitCode::FAILURE);
    assert_eq!(journey.plays, 0);
}

#[test]
fn sing_resolves_a_studio_capsule_with_its_own_window_and_knob() {
    let path = std::env::temp_dir().join(format!(
        "numinous_cli_sing_capsule_{}.num",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let creation = numinous_core::StudioCreation::new("sin(a*x)", -2.0, 3.0, 0.5).expect("capsule");
    std::fs::write(&path, creation.to_num_file()).expect("write capsule");

    let (source, xmin, xmax, a) =
        super::resolve_sing_input(&path.to_string_lossy(), None, None, None)
            .expect("capsule resolves");
    assert_eq!(source, "sin(a*x)");
    assert_eq!((xmin, xmax, a), (-2.0, 3.0, 0.5));

    // An explicit flag still overrides the capsule's own value.
    let (_, _, _, a) = super::resolve_sing_input(&path.to_string_lossy(), None, None, Some(2.0))
        .expect("override resolves");
    assert_eq!(a, 2.0);

    // Raw math keeps the documented defaults.
    let (source, xmin, xmax, a) =
        super::resolve_sing_input("sin(x)", None, None, None).expect("math resolves");
    assert_eq!(source, "sin(x)");
    assert_eq!(
        (xmin, xmax, a),
        (-std::f64::consts::TAU, std::f64::consts::TAU, 1.0)
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_terminal_fork_records_lineage_and_never_clobbers() {
    let dir = std::env::temp_dir().join(format!("numinous_cli_fork_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fork dir");
    let parent_path = dir.join("parent.num");
    let parent = numinous_core::StudioCreation::new("sin(a*x)", -3.0, 3.0, 1.0)
        .expect("parent")
        .with_title("Wave")
        .expect("title");
    std::fs::write(&parent_path, parent.to_num_file()).expect("write parent");

    let child_path = dir.join("child.num");
    let message = super::fork_studio_creation(
        &parent_path.to_string_lossy(),
        Some("sin(a*x)+0.1"),
        Some("Wave II"),
        Some("A Friend"),
        &child_path,
    )
    .expect("fork succeeds");
    assert!(message.contains("forked from "), "{message}");

    let child = numinous_core::StudioCreation::from_num_path(&child_path).expect("reload");
    assert_eq!(child.source(), "sin(a*x)+0.1");
    assert_eq!((child.xmin(), child.xmax(), child.a()), (-3.0, 3.0, 1.0));
    assert_eq!(child.title(), Some("Wave II"));
    assert_eq!(child.author(), Some("A Friend"));
    assert_eq!(
        child.descends(),
        Some(parent.to_link().as_str()),
        "the fork must record its parent"
    );

    // Never-clobber holds, and the refusal names the next free sibling.
    let refusal = super::fork_studio_creation(
        &parent_path.to_string_lossy(),
        None,
        None,
        None,
        &child_path,
    )
    .expect_err("second fork to the same path refuses");
    assert!(refusal.contains("already exists"), "{refusal}");
    assert!(refusal.contains("child-2.num"), "{refusal}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fork_refuses_a_missing_parent() {
    let missing = std::env::temp_dir().join(format!(
        "numinous_cli_fork_missing_parent_{}.num",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&missing);
    let out = std::env::temp_dir().join(format!(
        "numinous_cli_fork_missing_out_{}.num",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&out);
    assert!(
        super::fork_studio_creation(&missing.to_string_lossy(), None, None, None, &out).is_err()
    );
    assert!(!out.exists(), "a refused fork must write nothing");
}

#[test]
fn open_studio_names_the_remix_verb() {
    let creation = numinous_core::StudioCreation::new("cos(x)", -1.0, 1.0, 1.0).expect("capsule");
    let report = super::open_studio_report(&creation.to_link(), 32, 10).expect("report");
    assert!(
        report.contains("remix it: numinous fork "),
        "the open report must route to the next verb: {report}"
    );
}

#[test]
fn the_share_bundle_still_and_loop_are_the_same_visit() {
    let parent =
        std::env::temp_dir().join(format!("numinous-share-variation-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);
    let postcard_bytes = |variation: u64| {
        let before: std::collections::BTreeSet<std::path::PathBuf> = std::fs::read_dir(&parent)
            .map(|dir| dir.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        super::render_share_bundle(
            "buffon-needle",
            &parent,
            64,
            0.3,
            false,
            numinous_core::Era::Modern,
            variation,
        )
        .expect("share bundle");
        let after: std::collections::BTreeSet<std::path::PathBuf> = std::fs::read_dir(&parent)
            .expect("parent")
            .flatten()
            .map(|entry| entry.path())
            .collect();
        let fresh = after.difference(&before).next().expect("a new bundle");
        std::fs::read(fresh.join("postcard.png")).expect("postcard bytes")
    };
    assert_ne!(
        postcard_bytes(0),
        postcard_bytes(9),
        "the still must render the recorded variation, not the base deal"
    );
    let _ = std::fs::remove_dir_all(&parent);
}

#[test]
fn open_studio_rejects_malformed_and_oversized_imports() {
    let bad = std::env::temp_dir().join("numinous_cli_studio_bad_test.num");
    let huge = std::env::temp_dir().join("numinous_cli_studio_huge_test.num");
    std::fs::write(
        &bad,
        "NUMINOUS_STUDIO 1\nexpr=x\u{1b}[31m\nxmin=-1\nxmax=1\na=1\n",
    )
    .expect("bad file");
    std::fs::write(&huge, "x".repeat(numinous_core::MAX_SHARE_INPUT_BYTES + 1)).expect("huge file");

    let bad_err =
        load_studio_creation(bad.to_str().expect("utf8 path")).expect_err("bad import rejected");
    assert_eq!(bad_err, "invalid Numinous Studio .num file\n");
    let huge_err =
        load_studio_creation(huge.to_str().expect("utf8 path")).expect_err("huge import rejected");
    assert!(huge_err.contains("too large"));
    let link_err = load_studio_creation("numinous://studio?expr=x&xmin=-1&xmax=1&a=%")
        .expect_err("bad link rejected");
    assert_eq!(link_err, "invalid Numinous Studio link\n");

    let _ = std::fs::remove_file(&bad);
    let _ = std::fs::remove_file(&huge);
}

#[test]
fn tune_wav_writes_a_chiptune() {
    let path = std::env::temp_dir().join("numinous_tune_test.wav");
    let message = super::tune_wav(7, 2, &path).expect("tune");
    assert!(message.contains("chip"));
    assert!(std::fs::metadata(&path).expect("file").len() > 1000);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sing_wav_writes_a_melody() {
    let path = std::env::temp_dir().join("numinous_sing_test.wav");
    let message = super::sing_wav("sin(x)", -3.0, 3.0, 16, 1.0, &path).expect("sing");
    assert!(message.contains("wrote"));
    assert!(std::fs::metadata(&path).expect("file").len() > 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sing_wav_rejects_a_bad_expression() {
    let path = std::env::temp_dir().join("numinous_sing_bad.wav");
    assert!(super::sing_wav("nope(", -1.0, 1.0, 8, 1.0, &path).is_err());
}

#[test]
fn sing_wav_rejects_non_finite_bounds_and_knob() {
    // NaN passes `xmax <= xmin` because NaN compares false, so without its
    // own door a NaN window sang a silent WAV and reported success.
    let path = std::env::temp_dir().join("numinous_sing_nonfinite.wav");
    let _ = std::fs::remove_file(&path);
    let bounds = super::sing_wav("sin(x)", f64::NAN, f64::NAN, 8, 1.0, &path)
        .expect_err("a NaN window must be refused");
    assert_eq!(bounds, "need finite xmin and xmax\n");
    let knob = super::sing_wav("sin(a*x)", -1.0, 1.0, 8, f64::NAN, &path)
        .expect_err("a NaN knob melts every sample and must be refused");
    assert_eq!(knob, "need finite a\n");
    assert!(
        std::fs::metadata(&path).is_err(),
        "a refused song must not leave a WAV behind"
    );
}

#[test]
fn sing_wav_refuses_a_function_with_nothing_to_sing() {
    // The singing twin of the plot path's "nothing to plot": an expression
    // undefined across the whole window melts to zero notes, and writing
    // that as a successful WAV is silence sold as music.
    let path = std::env::temp_dir().join("numinous_sing_undefined.wav");
    let _ = std::fs::remove_file(&path);
    let message = super::sing_wav("sqrt(0-1)", -1.0, 1.0, 8, 1.0, &path)
        .expect_err("an everywhere-undefined function must be refused");
    assert_eq!(
        message,
        "nothing to sing: the function is undefined across this range\n"
    );
    assert!(
        std::fs::metadata(&path).is_err(),
        "a refused song must not leave a WAV behind"
    );
}

#[test]
fn describe_whispers_for_the_hidden_names() {
    let out = super::describe_report("hippasus", false, false, &numinous_core::Journey::default())
        .expect("a whisper");
    assert!(out.to_lowercase().contains("sea"), "got: {out}");
    assert!(
        super::describe_report(
            "not-a-room-nor-secret",
            false,
            false,
            &numinous_core::Journey::default()
        )
        .is_err()
    );
}

#[test]
fn sim_run_rejects_bad_input() {
    assert!(super::sim_run("nope", &[], 10, 10).is_err());
    assert!(super::sim_run("wing", &["nope=1".to_string()], 10, 10).is_err());
    assert!(super::sim_run("wing", &["angle-of-attack=abc".to_string()], 10, 10).is_err());
    assert!(super::sim_run("wing", &["missing-equals".to_string()], 10, 10).is_err());
}
