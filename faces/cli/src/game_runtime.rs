//! Terminal game session orchestration for the CLI face.
//!
//! This module owns terminal presentation, input loops, Journey events, and
//! score posting for games. Seeded rules, deals, legality, and grading remain
//! in `numinous_core`.

use std::io::{BufRead, IsTerminal};
use std::process::ExitCode;
use std::time::Duration;

use numinous_core::{Journey, Raster, Surface, all_rooms};

use crate::access::color_allowed;
use crate::game_input::{asked_why, read_game_line};
use crate::post_score;

/// The five seeds of the Bench: fixed forever, so every mind runs the same
/// five gauntlets and the composite means something.
const BENCH_SEEDS: [u64; 5] = [101, 102, 103, 104, 105];

/// The Bench: five gauntlets back to back, one composite, posted as bench v1.
/// A teenager, a laureate, and an agent all take the same run.
pub(super) fn bench(journey: &mut Journey) -> ExitCode {
    println!("THE BENCH v1: five gauntlets, seeds 101 to 105, one number.");
    println!("Agents run the same five seeds over MCP. Compare minds kindly.\n");
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    bench_with_input(journey, &mut input)
}

pub(super) fn bench_with_input(journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    let mut composite = 0i64;
    for (i, &seed) in BENCH_SEEDS.iter().enumerate() {
        println!("RUN {} OF 5", i + 1);
        // The composite is the run just played, never the scoreboard's
        // memory of a better day: an abandoned run ends the bench with its
        // reason named instead of padding the number with history.
        let (_, total) = gauntlet_run(seed, journey, input);
        let Some(total) = total else {
            println!(
                "BENCH ABANDONED  run {} of 5 ended early; no composite posted",
                i + 1
            );
            return ExitCode::SUCCESS;
        };
        composite += total;
        println!();
    }
    post_score("bench v1", composite);
    println!("BENCH COMPLETE  composite {composite}  (bench v1)");
    ExitCode::SUCCESS
}

/// Play Crack the Code: defuse a math-clued bomb from stdin guesses.
pub(super) fn crack(seed: u64, digits: usize, attempts: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    crack_with_input(seed, digits, attempts, journey, &mut input)
}

pub(super) fn crack_with_input(
    seed: u64,
    digits: usize,
    attempts: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let secret = numinous_core::secret_code(seed, digits);
    println!("A bomb. A hidden {digits}-digit code; {attempts} wires before it blows.");
    println!("After each guess: LOCKED = right digit in the RIGHT place.");
    println!("                  LOOSE  = right digit, WRONG place. Digits can repeat.");
    println!("Clue: {}\n", numinous_core::hint(&secret));
    let mut attempt = 0usize;
    while attempt < attempts {
        let Some(line) = read_game_line(input, &format!("Wire {}/{attempts} > ", attempt + 1))
        else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "crack") {
            continue;
        }
        let guess: Vec<u8> = line
            .trim()
            .chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect();
        if guess.len() != digits {
            println!("  Enter exactly {digits} digits.");
            continue;
        }
        if attempt == 0 {
            journey.play();
        }
        attempt += 1;
        let feedback = numinous_core::grade(&secret, &guess);
        if feedback.locked == digits {
            journey.win();
            let spare = (attempts - attempt) as i64;
            post_score(&format!("crack seed:{seed} digits:{digits}"), spare);
            println!();
            word_in_lights("DEFUSED", [90, 230, 120], 6);
            println!(
                "{spare} wire{} to spare. You cracked it.",
                if spare == 1 { "" } else { "s" }
            );
            return ExitCode::SUCCESS;
        }
        println!("  {} locked, {} loose.", feedback.locked, feedback.loose);
    }
    let code: String = secret.iter().map(|&d| char::from(b'0' + d)).collect();
    println!();
    word_in_lights("BOOM", [255, 90, 40], 6);
    println!("The code was {code}. The bomb does not hold grudges; deal another.");
    ExitCode::FAILURE
}

/// A word in lights: draw `word` huge on a colored burst and print frames in
/// place (truecolor half-blocks), a little cinema for the big moments.
fn word_in_lights(word: &str, accent: [u8; 3], frames: usize) {
    use std::io::Write as _;
    if !std::io::stdout().is_terminal() {
        println!("*** {word} ***");
        return;
    }
    let (w, h) = (96usize, 34usize);
    let mut stdout = std::io::stdout();
    // The moment owns the whole screen: wipe first, then erupt.
    let _ = write!(stdout, "[2J[H");
    let rows = h / 2 + 1;
    for frame in 0..frames {
        let mut raster = Raster::with_accent(w, h, accent);
        let reach = (frame + 1) as f64 / frames as f64;
        let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
        // Rays start outside a quiet disc, so the word owns the center.
        let hush = (word.len() as f64 * 6.0 * 2.0) / 2.0 + 4.0;
        for ray in 0..48 {
            let angle = std::f64::consts::TAU * f64::from(ray) / 48.0;
            let steps = (reach * w as f64 / 2.0) as i32;
            for step in (0..steps).step_by(2) {
                let fx = angle.cos() * f64::from(step);
                let fy = angle.sin() * f64::from(step) * 0.5;
                if (fx * fx + fy * fy * 4.0).sqrt() < hush {
                    continue;
                }
                let x = (cx + fx) as i32;
                let y = (cy + fy) as i32;
                raster.plot(x, y, if step % 6 == 0 { '#' } else { '*' });
            }
        }
        let scale = 2;
        let tx = (w as i32 - word.len() as i32 * 6 * scale) / 2;
        let ty = (h as i32 - 7 * scale) / 2;
        numinous_core::draw_text(&mut raster, word, tx, ty, scale, '#');
        // The reset only has something to reset when color was emitted. The
        // erase is cursor control, not color, so it stays either way.
        let color = color_allowed();
        let reset = if color { "\x1b[0m" } else { "" };
        let _ = write!(
            stdout,
            "{}{reset}\x1b[J",
            numinous_core::to_terminal(&raster, color)
        );
        let _ = stdout.flush();
        std::thread::sleep(Duration::from_millis(if frame + 1 == frames {
            350
        } else {
            70
        }));
        if frame + 1 < frames {
            let _ = write!(stdout, "\x1b[{rows}A");
        }
    }
}

/// Play SETI: scan channels of static and pick the artificial signal.
pub(super) fn seti(seed: u64, channels: usize, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    seti_with_input(seed, channels, rounds, journey, &mut input)
}

pub(super) fn seti_with_input(
    seed: u64,
    channels: usize,
    rounds: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut score = 0usize;
    let mut completed = 0usize;
    println!(
        "Listening near the hydrogen line. One channel hides a MIND; the rest are nature.\nOnly a mind counts: look for pulse groups going 2, 3, 5, 7. Answer with the letter.\n"
    );
    for round in 0..rounds {
        let scan = numinous_core::build_scan(seed.wrapping_add(round as u64), channels);
        println!("Scan #{}:", round + 1);
        for channel in &scan.channels {
            println!(
                "  {})  {:>10}  |{}|",
                channel.letter, channel.frequency, channel.trace
            );
        }
        let guess = loop {
            let Some(line) = read_game_line(input, "Which channel is a transmission? ") else {
                if completed > 0 {
                    post_score(
                        &format!("seti seed:{seed} rounds:{completed}"),
                        score as i64,
                    );
                }
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "seti") {
                continue;
            }
            let Some(guess) = line
                .chars()
                .find(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_uppercase())
            else {
                println!("  Answer with a channel letter.");
                continue;
            };
            break guess;
        };
        journey.play();
        completed += 1;
        if guess == scan.answer {
            score += 1;
            journey.win();
            println!(
                "Contact. {} at {} was counting in primes. That is not nature.\n",
                scan.answer, scan.answer_frequency
            );
        } else {
            println!(
                "Static. The signal was {} at {}, counting 2, 3, 5, 7, 11.\n",
                scan.answer, scan.answer_frequency
            );
        }
    }
    if completed > 0 {
        post_score(
            &format!("seti seed:{seed} rounds:{completed}"),
            score as i64,
        );
    }
    println!("You found {score}/{rounds}. Now open a channel and say hello: numinous aliens.");
    ExitCode::SUCCESS
}

/// Play Talk to the Aliens: continue the transmitted sequences from stdin.
pub(super) fn aliens(seed: u64, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    aliens_with_input(seed, rounds, journey, &mut input)
}

pub(super) fn aliens_with_input(
    seed: u64,
    rounds: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut score = 0usize;
    let mut completed = 0usize;
    println!("A transmission. They speak only in numbers. Prove you understand.\n");
    for round in 0..rounds {
        let message = numinous_core::alien_message(seed.wrapping_add(round as u64), 5);
        let shown: Vec<String> = message
            .terms
            .iter()
            .map(|&t| numinous_core::to_base(t, message.base))
            .collect();
        let base_note = if message.base == 10 {
            String::new()
        } else {
            format!(" (they count in base {})", message.base)
        };
        println!(
            "Signal #{}{}: {}, ...?",
            round + 1,
            base_note,
            shown.join(", ")
        );
        let line = loop {
            let Some(line) = read_game_line(input, "The next number > ") else {
                if completed > 0 {
                    post_score(
                        &format!("aliens seed:{seed} rounds:{completed}"),
                        score as i64,
                    );
                }
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "aliens") {
                continue;
            }
            if line
                .chars()
                .any(|character| character.is_ascii_alphanumeric())
            {
                break line;
            }
            println!("  Answer with the next transmitted number.");
        };
        journey.play();
        completed += 1;
        let answer = numinous_core::to_base(message.answer, message.base);
        let cleaned: String = line.chars().filter(char::is_ascii_alphanumeric).collect();
        if u64::from_str_radix(&cleaned, message.base).ok() == Some(message.answer) {
            score += 1;
            journey.win();
            println!(
                "Contact. It was {answer} ({}).\n  {}\n",
                message.name, message.explanation
            );
        } else {
            println!(
                "Silence. It was {answer} ({}).\n  {}\n",
                message.name, message.explanation
            );
        }
    }
    if completed > 0 {
        post_score(
            &format!("aliens seed:{seed} rounds:{completed}"),
            score as i64,
        );
    }
    println!("You understood {score}/{rounds} of their language.");
    ExitCode::SUCCESS
}

/// The seed to play with: the explicit one, or today's shared seed with
/// `--daily` (the same for every player on the same calendar day, UTC).
pub(super) fn pick_seed(seed: u64, daily: bool, journey: &mut Journey) -> u64 {
    if !daily {
        return seed;
    }
    let day = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() / 86_400)
        .unwrap_or(0);
    pick_seed_for_day(seed, true, day, journey)
}

/// Apply an already selected UTC day. Separating the clock makes the daily
/// Journey boundary deterministic under test without changing command timing.
pub(super) fn pick_seed_for_day(seed: u64, daily: bool, day: u64, journey: &mut Journey) -> u64 {
    if !daily {
        return seed;
    }
    println!("Daily challenge (day {day}). Everyone gets this one.");
    if let Some(chain) = journey.record_daily(day)
        && chain > 1
    {
        println!("DAILY STREAK  {chain} days.");
    }
    println!();
    day
}

/// A closing remark for a quiz score. Pure, so it is unit-tested.
pub(super) fn quiz_remark(score: usize, rounds: usize) -> &'static str {
    if rounds == 0 {
        return "Play a round!";
    }
    match score * 100 / rounds {
        100 => "Flawless. You see the math behind the shape.",
        60..=99 => "Sharp eye.",
        _ => "The shapes are sneaky. Play again.",
    }
}

/// Wrap `text` in an SGR color, or leave it alone when color is off.
///
/// Every game board goes through here rather than writing escapes inline. The
/// boards used to write them inline, and every one of them ignored `NO_COLOR`
/// as a result: the pictures honored the setting while the games painted over
/// it. One helper means the next board cannot forget, because there is nowhere
/// left to forget it.
pub(super) fn painted(color: bool, sgr: &str, text: &str) -> String {
    if color {
        format!("\x1b[{sgr}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// Draw the arcade board: the Muncher, the spirits, the numbers.
///
/// The Muncher is yellow. On an uneaten cell it keeps the digits so you can
/// see what you are about to eat; only empty cells show the bare `@`.
///
/// Every mark is legible without its color. The spirits are `d`, `T` and `e`,
/// and the Muncher's cell is the one in angle brackets, `>30<` rather than
/// `[30]`.
///
/// Those brackets are load bearing. The Muncher used to be yellow digits inside
/// ordinary brackets, so on an uneaten cell it was `[30]` in color and `[30]`
/// without, identical to every other cell: a `NO_COLOR` player, or one who
/// cannot pick yellow out, could not see where they were standing. Keeping the
/// digits was the right call, since you should see what you are about to eat.
/// Keeping them and nothing else was not. Both bracket forms are four columns
/// wide, so the grid still lines up.
pub(super) fn arcade_text(run: &numinous_core::munch_arcade::Arcade, color: bool) -> String {
    use numinous_core::munch_arcade::Mind;
    use numinous_core::munchers::{COLS, ROWS};
    let mut out = String::new();
    for row in 0..ROWS {
        for col in 0..COLS {
            let cell = row * COLS + col;
            if cell == run.muncher {
                let standing = if run.eaten[cell] {
                    "> @<".to_string()
                } else {
                    // The value stays legible: you should see what you are
                    // about to eat, and the brackets say you are on it.
                    format!(">{:>2}<", run.board.numbers[cell])
                };
                out.push_str(&painted(color, "93", &standing));
            } else if let Some(v) = run.vexations.iter().find(|v| v.cell == cell) {
                let (sgr, mark) = match v.mind {
                    Mind::Drifter => ("95", "[ d]"),
                    Mind::Tracker => ("91", "[ T]"),
                    Mind::Editor => ("96", "[ e]"),
                };
                out.push_str(&painted(color, sgr, mark));
            } else if run.eaten[cell] {
                out.push_str("[  ]");
            } else {
                out.push_str(&format!("[{:>2}]", run.board.numbers[cell]));
            }
        }
        out.push('\n');
    }
    out
}

/// The Munch arcade in the terminal: turn-based, same math, same spirits.
pub(super) fn arcade(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    arcade_with_input(seed, journey, &mut input)
}

pub(super) fn arcade_with_input(
    seed: u64,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    use numinous_core::munch_arcade::{Action, Arcade, Turn};
    let mut run = Arcade::new(seed);
    let mut played = false;
    println!("THE MUNCH ARCADE  seed {seed}. You are @. Eat what fits; dodge the spirits.");
    println!("T tracks you, d drifts, e rewrites numbers where it walks.");
    println!("Moves: w a s d, then e to eat. One move, then they move. (? explains)");
    loop {
        println!(
            "\nLEVEL {}  LIVES {}  SCORE {}  RULE: {}",
            run.level,
            run.lives,
            run.score,
            run.board.rule.describe()
        );
        print!("{}", arcade_text(&run, color_allowed()));
        let Some(line) = read_game_line(input, "move > ") else {
            break;
        };
        if asked_why(&line, "arcade") {
            continue;
        }
        let action = match line.trim().chars().next().map(|c| c.to_ascii_lowercase()) {
            Some('w') => Action::Up,
            Some('s') => Action::Down,
            Some('a') => Action::Left,
            Some('d') => Action::Right,
            Some('e') => Action::Eat,
            Some('q') => break,
            _ => {
                println!("  w a s d to move, e to eat, q to leave.");
                continue;
            }
        };
        if !played {
            journey.play();
            played = true;
        }
        match run.turn(action) {
            Turn::Going => {}
            Turn::Caught => {
                println!(
                    "\n  CAUGHT. A Vexation touches you; {} lives left.",
                    run.lives
                );
            }
            Turn::Cleared => {
                journey.win();
                println!(
                    "\n  BOARD CLEAR. Level {}: one more spirit joins.",
                    run.level
                );
            }
            Turn::Over => {
                println!();
                word_in_lights("CAUGHT", [255, 120, 60], 5);
                break;
            }
        }
    }
    if played {
        post_score(&format!("arcade seed:{seed}"), run.score);
    } else {
        println!("RUN CLOSED. No score recorded.");
        return ExitCode::SUCCESS;
    }
    println!(
        "RUN OVER  level {}, score {}  (arcade seed:{seed}). The spirits send regards.",
        run.level, run.score
    );
    ExitCode::SUCCESS
}

/// Draw the garden: stalks as columns, red and blue.
///
/// `R` and `B` are the answer; the color repeats it. A player who cannot tell
/// red from blue, or who has turned color off, still reads the garden.
pub(super) fn garden_text(stalks: &numinous_core::hackenbush::Stalks, color: bool) -> String {
    use numinous_core::hackenbush::Color;
    let tallest = stalks.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = String::new();
    for row in (0..tallest).rev() {
        out.push_str("   ");
        for stalk in stalks {
            match stalk.get(row) {
                Some(Color::Red) => out.push_str(&painted(color, "91", " R ")),
                Some(Color::Blue) => out.push_str(&painted(color, "94", " B ")),
                None => out.push_str("   "),
            }
            out.push(' ');
        }
        out.push('\n');
    }
    out.push_str("   ");
    for (i, _) in stalks.iter().enumerate() {
        out.push_str(&format!("={}= ", i + 1));
    }
    out.push('\n');
    out
}

/// Hackenbush against the Order: cut red, it cuts blue, last cutter wins.
pub(super) fn hackenbush(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    hackenbush_with_input(seed, journey, &mut input)
}

pub(super) fn hackenbush_with_input(
    seed: u64,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    use numinous_core::hackenbush as hb;
    let mut stalks = hb::new_garden(seed);
    let mut played = false;
    println!("HACKENBUSH  seed {seed}. Cut a RED segment; everything above it falls.");
    println!("The Order cuts blue. Whoever cannot cut, loses. Answer: stalk height");
    println!("(1 1 cuts stalk 1 at the ground). This garden is winnable. (? explains)");
    loop {
        println!("\n{}", garden_text(&stalks, color_allowed()));
        if !hb::can_move(&stalks, hb::Color::Red) {
            println!("No red left to cut. The Order takes the garden. (It was arithmetic.)");
            return ExitCode::SUCCESS;
        }
        let Some(line) = read_game_line(input, "stalk height > ") else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "hackenbush") {
            continue;
        }
        let nums: Vec<usize> = line
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        let (Some(&stalk), Some(&height)) = (nums.first(), nums.get(1)) else {
            println!("  Two numbers: which stalk, which height (both from 1).");
            continue;
        };
        if stalk == 0 || height == 0 || !hb::cut(&mut stalks, stalk - 1, height - 1, hb::Color::Red)
        {
            println!("  That is not a red segment you can reach.");
            continue;
        }
        if !played {
            journey.play();
            played = true;
        }
        if !hb::can_move(&stalks, hb::Color::Blue) {
            journey.win();
            post_score(&format!("hackenbush seed:{seed}"), 1);
            println!("\nThe Order has nothing left to cut. It concedes, and keeps its word:");
            println!("\n{}", hb::the_secret());
            return ExitCode::SUCCESS;
        }
        let (bi, bh) = hb::order_move(&stalks).expect("blue can move");
        let _ = hb::cut(&mut stalks, bi, bh, hb::Color::Blue);
        println!("  The Order cuts stalk {} at height {}.", bi + 1, bh + 1);
    }
}

/// The Party Problem: round one, five guests (escapable); round two, six.
pub(super) fn party(journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    party_with_input(journey, &mut input)
}

/// Draw the handshake matrix: who has shaken whom, and in which shade.
///
/// `R` and `B` carry the shade; the color repeats it, and a dot is an unshaken
/// pair. Pulled out of the game loop so it can be looked at by a test at all:
/// it used to be written straight to stdout, which is why it was the last of
/// the three boards still ignoring `NO_COLOR`.
pub(super) fn party_board_text(
    p: &numinous_core::party::Party,
    guests: usize,
    color: bool,
) -> String {
    use numinous_core::party::Shade;
    let mut out = String::from("     ");
    for b in 1..=guests {
        out.push_str(&format!(" {b}"));
    }
    out.push('\n');
    for a in 0..guests {
        out.push_str(&format!("   {} ", a + 1));
        for b in 0..guests {
            if b <= a {
                out.push_str("  ");
                continue;
            }
            let mark = match numinous_core::party::edge_index(guests, a, b).map(|i| p.edges[i]) {
                Some(Shade::Red) => painted(color, "91", "R"),
                Some(Shade::Blue) => painted(color, "94", "B"),
                _ => ".".to_string(),
            };
            out.push_str(&format!(" {mark}"));
        }
        out.push('\n');
    }
    out
}

pub(super) fn party_with_input(journey: &mut Journey, input: &mut impl BufRead) -> ExitCode {
    use numinous_core::party::{Party, Shade};
    println!("THE PARTY PROBLEM. Shade every handshake red or blue WITHOUT making");
    println!("a triangle of one color. Answer like: 1 3 r   (guests 1 and 3, red).");
    println!("Round one: five guests. It can be done. (? explains)\n");
    for (round, guests) in [(1usize, 5usize), (2, 6)] {
        let mut played = false;
        let mut p = Party::new(guests);
        println!(
            "ROUND {round}: {guests} guests, {} handshakes.",
            p.edges.len()
        );
        loop {
            // The matrix of handshakes so far.
            print!("{}", party_board_text(&p, guests, color_allowed()));
            let Some(line) = read_game_line(input, "handshake > ") else {
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "party") {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let (Some(a), Some(b), Some(color)) = (
                words.first().and_then(|w| w.parse::<usize>().ok()),
                words.get(1).and_then(|w| w.parse::<usize>().ok()),
                words.get(2),
            ) else {
                println!("  Like this: 1 3 r   or   2 5 b");
                continue;
            };
            let shade = match color.chars().next().map(|c| c.to_ascii_lowercase()) {
                Some('r') => Shade::Red,
                Some('b') => Shade::Blue,
                _ => {
                    println!("  Color must be r or b.");
                    continue;
                }
            };
            if a == 0 || b == 0 || !p.shade(a - 1, b - 1, shade) {
                println!("  That handshake is not open.");
                continue;
            }
            if !played {
                journey.play();
                played = true;
            }
            if let Some((x, y, z, _)) = p.mono_triangle() {
                println!(
                    "\nA one-color triangle: guests {}, {}, {}. {} handshakes survived.",
                    x + 1,
                    y + 1,
                    z + 1,
                    p.shaded() - 1
                );
                if guests == 6 {
                    println!(
                        "It was never possible. Among six, three mutual friends or three\n\
                         mutual strangers MUST exist: R(3,3) = 6. You just felt a theorem."
                    );
                } else {
                    println!(
                        "Five guests CAN escape. The pentagon knows: ring one color, star the other."
                    );
                }
                break;
            }
            if p.complete() {
                journey.win();
                post_score(&format!("party guests:{guests}"), p.shaded() as i64);
                println!(
                    "\nEvery handshake shaded, no triangle. You escaped with all {}.",
                    p.shaded()
                );
                if guests == 5 {
                    println!("Now try six. (Ramsey is waiting.)\n");
                }
                break;
            }
        }
    }
    ExitCode::SUCCESS
}

/// Fifteen's Bet: call each scramble solvable or stuck forever.
pub(super) fn fifteen(seed: u64, rounds: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    fifteen_with_input(seed, rounds, journey, &mut input)
}

pub(super) fn fifteen_with_input(
    seed: u64,
    rounds: u64,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    use numinous_core::fifteen as ff;
    let mut called = 0u64;
    let mut completed = 0u64;
    println!("FIFTEEN'S BET. Half of all scrambles can never be solved, and one");
    println!("invisible quantity decides which. Call each one: S(olvable) or U(nsolvable).");
    println!("(? explains)\n");
    for n in 0..rounds {
        let tiles = ff::deal(seed, n);
        println!(
            "SCRAMBLE {} of {rounds}:\n{}",
            n + 1,
            ff::board_text(&tiles)
        );
        let verdict = loop {
            let Some(line) = read_game_line(input, "S or U > ") else {
                // A departure mid-session keeps what it earned, exactly as
                // seti, aliens, and quiz do on this same exit path: four
                // correct calls that vanish from the board are a silent loss.
                if completed > 0 {
                    post_score(
                        &format!("fifteen seed:{seed} rounds:{completed}"),
                        called as i64,
                    );
                }
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "fifteen") {
                continue;
            }
            match line
                .chars()
                .find(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_uppercase())
            {
                Some('S') => break true,
                Some('U') => break false,
                _ => println!("  S or U."),
            }
        };
        journey.play();
        completed += 1;
        let truth = ff::solvable(&tiles);
        if verdict == truth {
            called += 1;
            journey.win();
            println!("  Called it. {}\n", ff::why(&tiles));
        } else {
            println!("  No: {}\n", ff::why(&tiles));
        }
    }
    if completed > 0 {
        post_score(
            &format!("fifteen seed:{seed} rounds:{completed}"),
            called as i64,
        );
    }
    println!("{called} of {rounds} called. Parity is learnable; deal again.");
    ExitCode::SUCCESS
}

/// Draw the heaps as rows of stones.
pub(super) fn nim_board(heaps: &[u32]) -> String {
    heaps
        .iter()
        .enumerate()
        .map(|(i, &h)| format!("  {}) {}", i + 1, "O ".repeat(h as usize)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Play nim against the Order. Winning earns the secret, spoken in full.
pub(super) fn nim(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    nim_with_input(seed, journey, &mut input)
}

pub(super) fn nim_with_input(
    seed: u64,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut heaps = numinous_core::nim_new(seed);
    let mut played = false;
    println!("NIM  seed {seed}. On your turn, take ANY number of stones from ONE heap.");
    println!("Whoever takes the last stone wins. Answer like: 2 3  (heap 2, take 3).");
    println!("The Order plays a secret. Beat it and the secret is yours. (? explains)");
    loop {
        println!("\n{}", nim_board(&heaps));
        let Some(line) = read_game_line(input, "heap amount > ") else {
            return ExitCode::SUCCESS;
        };
        if asked_why(&line, "nim") {
            continue;
        }
        let nums: Vec<u32> = line
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        let (Some(&heap), Some(&take)) = (nums.first(), nums.get(1)) else {
            println!("  Two numbers: which heap, how many. Like: 2 3");
            continue;
        };
        if heap == 0 || !numinous_core::nim_apply(&mut heaps, heap as usize - 1, take) {
            println!("  That move is not on the board.");
            continue;
        }
        if !played {
            journey.play();
            played = true;
        }
        if numinous_core::nim_finished(&heaps) {
            journey.win();
            post_score(&format!("nim seed:{seed}"), 1);
            println!("\nYou took the last stone. The Order concedes, and keeps its word:");
            println!("\n{}", numinous_core::nim_secret());
            return ExitCode::SUCCESS;
        }
        let (oh, ot) = numinous_core::nim_order(&heaps);
        let _ = numinous_core::nim_apply(&mut heaps, oh, ot);
        println!("  The Order takes {ot} from heap {}.", oh + 1);
        if numinous_core::nim_finished(&heaps) {
            println!("\nThe Order takes the last stone. Again. (It is not luck.)");
            return ExitCode::SUCCESS;
        }
    }
}

/// The first answer letter in one already-read input line.
fn letter_from_line(line: &str) -> Option<char> {
    line.chars()
        .find(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
}

/// The Gauntlet: munch board, mystery shape, sky scan, bomb code, one run.
/// Opt-in, bounded, and over in minutes: a shape for a session, not a trap.
pub(super) fn gauntlet(seed: u64, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    gauntlet_with_input(seed, journey, &mut input)
}

pub(super) fn gauntlet_with_input(
    seed: u64,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    gauntlet_run(seed, journey, input).0
}

/// The gauntlet body, returning the completed run's posted total as well:
/// `None` means the run was abandoned mid-stage and nothing was posted, so a
/// caller like the Bench cannot mistake history for the run just played.
fn gauntlet_run(
    seed: u64,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> (ExitCode, Option<i64>) {
    let puzzle = numinous_core::GauntletPuzzle::new(seed);
    let mut stage_scores = Vec::new();
    let mut cleared = Vec::new();
    println!(
        "THE GAUNTLET  seed {seed}. Four stages. Clears build your combo.
"
    );

    // Stage 1: one munch board.
    let board = &puzzle.munch;
    println!("STAGE 1 of 4  MUNCH: {}", board.rule.describe());
    print!("{}", numinous_core::board_text(board));
    let line = loop {
        let Some(line) = read_game_line(input, "Your bites > ") else {
            return (ExitCode::SUCCESS, None);
        };
        if !asked_why(&line, "gauntlet") {
            break line;
        }
    };
    journey.play();
    let bites: Vec<usize> = line
        .split_whitespace()
        .filter_map(|w| w.parse::<usize>().ok())
        .filter(|&n| n >= 1)
        .map(|n| n - 1)
        .collect();
    let grade = puzzle.grade_munch(&bites);
    let clear = grade.stage.clean;
    if clear {
        journey.win();
    }
    println!(
        "  +{} points{}
",
        grade.stage.score,
        if clear { "  CLEAN" } else { "" }
    );
    stage_scores.push(grade.stage.score);
    cleared.push(clear);

    // Stage 2: one mystery shape.
    let round = &puzzle.shape;
    println!("STAGE 2 of 4  THE SHAPE:");
    print!("{}", round.art);
    for choice in &round.choices {
        println!("  {}) {}", choice.letter, choice.title);
    }
    let guess = loop {
        let Some(line) = read_game_line(input, "Your answer > ") else {
            return (ExitCode::SUCCESS, None);
        };
        if asked_why(&line, "gauntlet") {
            continue;
        }
        let Some(guess) = letter_from_line(&line) else {
            println!("  Answer with a choice letter.");
            continue;
        };
        break guess;
    };
    journey.play();
    let grade = puzzle.grade_shape(Some(guess));
    let clear = grade.clean;
    if clear {
        journey.win();
    }
    let points = grade.score;
    println!(
        "  It was {} ({}). +{points} points{}
",
        round.answer,
        round.answer_title,
        if clear { "  CLEAN" } else { "" }
    );
    stage_scores.push(points);
    cleared.push(clear);

    // Stage 3: one sky scan.
    let scan = &puzzle.sky;
    println!("STAGE 3 of 4  THE SKY:");
    for channel in &scan.channels {
        println!(
            "  {})  {:>10}  |{}|",
            channel.letter, channel.frequency, channel.trace
        );
    }
    let guess = loop {
        let Some(line) = read_game_line(input, "Which is a mind > ") else {
            return (ExitCode::SUCCESS, None);
        };
        if asked_why(&line, "gauntlet") {
            continue;
        }
        let Some(guess) = letter_from_line(&line) else {
            println!("  Answer with a channel letter.");
            continue;
        };
        break guess;
    };
    journey.play();
    let grade = puzzle.grade_sky(Some(guess));
    let clear = grade.clean;
    if clear {
        journey.win();
    }
    let points = grade.score;
    println!(
        "  The signal was {}. +{points} points{}
",
        scan.answer,
        if clear { "  CLEAN" } else { "" }
    );
    stage_scores.push(points);
    cleared.push(clear);

    // Stage 4: the bomb, four digits, five tries.
    println!("STAGE 4 of 4  THE BOMB. Four digits, five tries.");
    println!("  Clue: {}", puzzle.bomb_hint());
    let mut points = 0i64;
    let mut clear = false;
    let mut played = false;
    let mut attempt = 1usize;
    while attempt <= numinous_core::GAUNTLET_MAX_WIRES {
        let Some(line) = read_game_line(
            input,
            &format!("Wire {attempt}/{} > ", numinous_core::GAUNTLET_MAX_WIRES),
        ) else {
            return (ExitCode::SUCCESS, None);
        };
        // Help and typos are free here as they are in every other stage and
        // in standalone crack: a wire only burns when a real four-digit
        // guess actually tests the bomb.
        if asked_why(&line, "gauntlet") {
            continue;
        }
        let guess: Vec<u8> = line
            .trim()
            .chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect();
        if guess.len() != 4 {
            println!("  Four digits.");
            continue;
        }
        if !played {
            journey.play();
            played = true;
        }
        let Some(grade) = puzzle.grade_wire(attempt, &guess) else {
            println!("  Four digits.");
            continue;
        };
        if grade.stage.clean {
            clear = true;
            points = grade.stage.score;
            journey.win();
            word_in_lights("DEFUSED", [90, 230, 120], 5);
            println!("  +{points} points  CLEAN\n");
            break;
        }
        println!(
            "  {} locked, {} loose.",
            grade.feedback.locked, grade.feedback.loose
        );
        attempt += 1;
    }
    if !clear {
        word_in_lights("BOOM", [255, 90, 40], 5);
        println!("  It was {}. +0 points\n", puzzle.bomb_code_text());
    }
    stage_scores.push(points);
    cleared.push(clear);

    // The one honest number.
    let total = numinous_core::gauntlet_total(&stage_scores, &cleared);
    let clears = cleared.iter().filter(|&&c| c).count();
    post_score(&numinous_core::gauntlet_score_key(seed), total);
    println!("RUN COMPLETE  {clears}/4 clean  TOTAL {total}  (gauntlet seed:{seed})");
    (ExitCode::SUCCESS, Some(total))
}

/// Play Munch: eat the numbers that fit the rule, round by round, scored.
pub(super) fn munch(seed: u64, rounds: usize, journey: &mut Journey) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    munch_with_input(seed, rounds, journey, &mut input)
}

pub(super) fn munch_with_input(
    seed: u64,
    rounds: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut total = 0i64;
    println!("MUNCH. Eat by cell number, e.g. \"1 7 22\". Wrong bites cost you. (? explains)\n");
    for round in 0..rounds {
        let board = numinous_core::build_board(seed, round as u64);
        println!("Board {} of {rounds}: {}", round + 1, board.rule.describe());
        print!("{}", numinous_core::board_text(&board));
        let line = loop {
            let Some(line) = read_game_line(input, "Your bites > ") else {
                println!("Final score: {total} (seed {seed}).");
                return ExitCode::SUCCESS;
            };
            if !asked_why(&line, "munch") {
                break line;
            }
        };
        journey.play();
        let bites: Vec<usize> = line
            .split_whitespace()
            .filter_map(|w| w.parse::<usize>().ok())
            .filter(|&n| n >= 1)
            .map(|n| n - 1)
            .collect();
        let outcome = numinous_core::grade_munch(&board, &bites);
        post_score(
            &numinous_core::munch_score_key(seed, round as u64),
            outcome.score,
        );
        if numinous_core::munch_clean_win(&outcome) {
            journey.win();
            println!(
                "PERFECT. {} eaten, nothing wasted. +{} points.\n",
                outcome.hits, outcome.score
            );
        } else {
            println!(
                "{} eaten, {} bad bites, {} left behind. +{} points.",
                outcome.hits, outcome.bad_bites, outcome.left_behind, outcome.score
            );
            // The dense feedback: exactly which judgments went wrong.
            if !outcome.wrongly_eaten.is_empty() {
                let bad: Vec<String> = outcome.wrongly_eaten.iter().map(u64::to_string).collect();
                println!("  Not {}: {}.", board.rule.describe(), bad.join(", "));
            }
            if !outcome.missed.is_empty() {
                let missed: Vec<String> = outcome.missed.iter().map(u64::to_string).collect();
                println!("  You walked past: {}.", missed.join(", "));
                if outcome.bad_bites == 0 && outcome.missed.len() == 1 {
                    println!("  One away. The board remembers.");
                }
            }
            println!();
        }
        total += outcome.score;
    }
    println!("Final score: {total} (seed {seed}). Beat that, or make an AI try.");
    ExitCode::SUCCESS
}

/// Play the interactive "guess the shape" quiz, reading guesses from stdin.
pub(super) fn quiz(
    rounds: usize,
    seed: u64,
    width: usize,
    height: usize,
    choices: usize,
    journey: &mut Journey,
) -> ExitCode {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    quiz_with_input(rounds, seed, width, height, choices, journey, &mut input)
}

pub(super) fn quiz_with_input(
    rounds: usize,
    seed: u64,
    width: usize,
    height: usize,
    choices: usize,
    journey: &mut Journey,
    input: &mut impl BufRead,
) -> ExitCode {
    let mut score = 0usize;
    let mut completed = 0usize;
    let mut recent: Vec<&'static str> = Vec::new();
    println!("Guess the shape. Name the math behind each mystery render.\n");
    for round in 0..rounds {
        // Recently asked rooms sit out: no repeated questions in a session.
        let all: Vec<&'static str> = all_rooms().iter().map(|r| r.meta().id).collect();
        let fresh: Vec<&'static str> = all
            .iter()
            .copied()
            .filter(|id| !recent.contains(id))
            .collect();
        let pool = if fresh.len() > choices { fresh } else { all };
        let r = numinous_core::build_round_pool(seed, round as u64, width, height, choices, &pool);
        if let Some(choice) = r.choices.iter().find(|c| c.letter == r.answer) {
            recent.push(choice.id);
            if recent.len() > 10 {
                recent.remove(0);
            }
        }
        println!("Mystery #{} of {rounds}:", round + 1);
        print!("{}", r.art);
        println!();
        for choice in &r.choices {
            println!("  {}) {}", choice.letter, choice.title);
        }
        let guess = loop {
            let Some(line) = read_game_line(input, "Your answer: ") else {
                if completed > 0 {
                    post_score(
                        &format!("quiz seed:{seed} rounds:{completed}"),
                        score as i64,
                    );
                }
                println!("Final score: {score}/{completed}.");
                return ExitCode::SUCCESS;
            };
            if asked_why(&line, "quiz") {
                continue;
            }
            let Some(guess) = letter_from_line(&line) else {
                println!("  Answer with a choice letter.");
                continue;
            };
            break guess;
        };
        journey.play();
        completed += 1;
        if guess == r.answer {
            score += 1;
            journey.win();
            println!(
                "Correct! It is {}.\n  {}\n",
                r.answer_title, r.answer_reveal
            );
        } else {
            println!(
                "Not quite. It was {} ({}).\n  {}\n",
                r.answer, r.answer_title, r.answer_reveal
            );
        }
    }
    if completed > 0 {
        post_score(
            &format!("quiz seed:{seed} rounds:{completed}"),
            score as i64,
        );
    }
    println!(
        "Final score: {score}/{completed}. {}",
        quiz_remark(score, completed)
    );
    ExitCode::SUCCESS
}
