//! MCP parsing and projection for the seven engineered flagship Aha arcs.
//!
//! Core owns each staged state machine, its mathematical truth, grading, and
//! overlay drawing. This adapter translates keyless protocol arguments, replays
//! one bounded request, and projects only the state the mind has earned.

use numinous_core::Canvas;
use serde_json::{Value, json};

/// Optional engineered-aha arguments for the staged flagship rooms.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct FlagshipAhaRequest {
    place_wager: Option<numinous_core::rooms::times_tables_aha::CardioidHome>,
    number_wager: Option<f64>,
    bin_wager: Option<usize>,
    ending_wager: Option<numinous_core::rooms::pendulum_aha::Ending>,
    speed_wager: Option<numinous_core::rooms::kepler_aha::SpeedRelation>,
    policy_wager: Option<numinous_core::rooms::parrondo::Policy>,
    die_choice: Option<numinous_core::rooms::nontransitive::Die>,
    counter_wager: Option<numinous_core::rooms::nontransitive::Die>,
    pub(super) summon: bool,
}

impl FlagshipAhaRequest {
    pub(super) fn uses_generation_args(self) -> bool {
        self.place_wager.is_some()
            || self.number_wager.is_some()
            || self.bin_wager.is_some()
            || self.ending_wager.is_some()
            || self.speed_wager.is_some()
            || self.policy_wager.is_some()
            || self.die_choice.is_some()
            || self.counter_wager.is_some()
            || self.summon
    }
}

pub(super) fn parse_flagship_aha_request(
    args: &Value,
    room_id: &str,
) -> Result<FlagshipAhaRequest, String> {
    let place_raw = args.get("place_wager");
    let number_raw = args.get("number_wager");
    let ending_raw = args.get("ending_wager");
    let speed_raw = args.get("speed_wager");
    let policy_raw = args.get("policy_wager");
    let die_choice_raw = args.get("die_choice");
    let counter_raw = args.get("counter_wager");
    let summon = args
        .get("aha_summon")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let place_wager = if let Some(value) = place_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'place_wager' must be a string.".to_string());
        };
        if room_id != "times-tables" {
            return Err("place_wager is only valid on Times Tables (id times-tables).".to_string());
        }
        Some(match name {
            "mandelbrot" => numinous_core::rooms::times_tables_aha::CardioidHome::Mandelbrot,
            "nephroid" => numinous_core::rooms::times_tables_aha::CardioidHome::Nephroid,
            "circle" => numinous_core::rooms::times_tables_aha::CardioidHome::Circle,
            other => {
                return Err(format!(
                    "place_wager must be mandelbrot, nephroid, or circle; got '{other}'."
                ));
            }
        })
    } else {
        None
    };

    let number_wager = if let Some(value) = number_raw {
        let Some(guess) = value.as_f64() else {
            return Err("Argument 'number_wager' must be a finite number.".to_string());
        };
        if !guess.is_finite() {
            return Err("Argument 'number_wager' must be a finite number.".to_string());
        }
        if room_id != "buffon-needle" {
            return Err(
                "number_wager is only valid on Buffon's Needle (id buffon-needle).".to_string(),
            );
        }
        if !(numinous_core::rooms::buffon_aha::GUESS_MIN
            ..=numinous_core::rooms::buffon_aha::GUESS_MAX)
            .contains(&guess)
        {
            return Err(format!(
                "number_wager must be in [{}, {}].",
                numinous_core::rooms::buffon_aha::GUESS_MIN,
                numinous_core::rooms::buffon_aha::GUESS_MAX
            ));
        }
        Some(guess)
    } else {
        None
    };

    let bin_raw = args.get("bin_wager");
    let bin_wager = if let Some(value) = bin_raw {
        let Some(bin) = value.as_u64() else {
            return Err("Argument 'bin_wager' must be a whole number.".to_string());
        };
        if room_id != "galton-board" {
            return Err(
                "bin_wager is only valid on the Galton Board (id galton-board).".to_string(),
            );
        }
        let last = numinous_core::rooms::galton_board::BOARD_ROWS as u64;
        if bin > last {
            return Err(format!("bin_wager must be in [0, {last}]."));
        }
        Some(bin as usize)
    } else {
        None
    };

    let ending_wager = if let Some(value) = ending_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'ending_wager' must be a string.".to_string());
        };
        if room_id != "double-pendulum" {
            return Err(
                "ending_wager is only valid on Double Pendulum (id double-pendulum).".to_string(),
            );
        }
        Some(match name {
            "together" => numinous_core::rooms::pendulum_aha::Ending::Together,
            "drifted" => numinous_core::rooms::pendulum_aha::Ending::Drifted,
            "lost" => numinous_core::rooms::pendulum_aha::Ending::Lost,
            other => {
                return Err(format!(
                    "ending_wager must be together, drifted, or lost; got '{other}'."
                ));
            }
        })
    } else {
        None
    };

    let speed_wager = if let Some(value) = speed_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'speed_wager' must be a string.".to_string());
        };
        if room_id != "kepler-laws" {
            return Err("speed_wager is only valid on Kepler Areas (id kepler-laws).".to_string());
        }
        Some(match name {
            "faster" => numinous_core::rooms::kepler_aha::SpeedRelation::Faster,
            "slower" => numinous_core::rooms::kepler_aha::SpeedRelation::Slower,
            "same" => numinous_core::rooms::kepler_aha::SpeedRelation::Same,
            other => {
                return Err(format!(
                    "speed_wager must be faster, slower, or same; got '{other}'."
                ));
            }
        })
    } else {
        None
    };

    let policy_wager = if let Some(value) = policy_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'policy_wager' must be a string.".to_string());
        };
        if room_id != "parrondo" {
            return Err("policy_wager is only valid on Parrondo's Trap (id parrondo).".to_string());
        }
        Some(match name {
            "a" => numinous_core::rooms::parrondo::Policy::OnlyA,
            "b" => numinous_core::rooms::parrondo::Policy::OnlyB,
            "abb" => numinous_core::rooms::parrondo::Policy::CycleAbb,
            other => {
                return Err(format!("policy_wager must be a, b, or abb; got '{other}'."));
            }
        })
    } else {
        None
    };

    let parse_die = |name: &str, field: &str| match name {
        "a" => Ok(numinous_core::rooms::nontransitive::Die::A),
        "b" => Ok(numinous_core::rooms::nontransitive::Die::B),
        "c" => Ok(numinous_core::rooms::nontransitive::Die::C),
        other => Err(format!("{field} must be a, b, or c; got '{other}'.")),
    };
    let die_choice = if let Some(value) = die_choice_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'die_choice' must be a string.".to_string());
        };
        if room_id != "nontransitive" {
            return Err(
                "die_choice is only valid on Nontransitive Dice (id nontransitive).".to_string(),
            );
        }
        Some(parse_die(name, "die_choice")?)
    } else {
        None
    };
    let counter_wager = if let Some(value) = counter_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'counter_wager' must be a string.".to_string());
        };
        if room_id != "nontransitive" {
            return Err(
                "counter_wager is only valid on Nontransitive Dice (id nontransitive).".to_string(),
            );
        }
        Some(parse_die(name, "counter_wager")?)
    } else {
        None
    };

    let wagers = usize::from(place_wager.is_some())
        + usize::from(number_wager.is_some())
        + usize::from(bin_wager.is_some())
        + usize::from(ending_wager.is_some())
        + usize::from(speed_wager.is_some())
        + usize::from(policy_wager.is_some())
        + usize::from(counter_wager.is_some());
    if wagers > 1 {
        return Err(
            "Pass one wager: place_wager, number_wager, bin_wager, ending_wager, speed_wager, policy_wager, or counter_wager."
                .to_string(),
        );
    }
    if summon
        && room_id != "times-tables"
        && room_id != "buffon-needle"
        && room_id != "galton-board"
        && room_id != "double-pendulum"
        && room_id != "kepler-laws"
        && room_id != "parrondo"
        && room_id != "nontransitive"
    {
        return Err(
            "aha_summon is only valid on Times Tables, Buffon's Needle, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, or Nontransitive Dice."
                .to_string(),
        );
    }

    Ok(FlagshipAhaRequest {
        place_wager,
        number_wager,
        bin_wager,
        ending_wager,
        speed_wager,
        policy_wager,
        die_choice,
        counter_wager,
        summon,
    })
}

/// The stdio face has no keyboard. The aha chrome's key prompts translate
/// into this face's own verbs before they reach a mind that cannot press
/// them: E becomes aha_summon, and wager digits become the named values in
/// the room's wager field.
fn keyless_aha_status(status: String) -> String {
    status
        .replace(" (from 1e-4)", "")
        .replace(
            "WHERE? 1=M 2=N 3=C",
            "WHERE? place_wager: mandelbrot, nephroid, or circle",
        )
        .replace(
            "AT THE END? 1=TOGETHER 2=DRIFTED 3=LOST",
            "END? use ending_wager",
        )
        .replace(
            "NEAR SUN? 1=FASTER 2=SLOWER 3=SAME",
            "NEAR SUN? use speed_wager",
        )
        .replace("WHICH WINS? 1=A 2=B 3=ABB", "WHICH WINS? use policy_wager")
        .replace("1=A 2=B 3=C", "use counter_wager: a, b, or c")
        .replace("E:WHY", "aha_summon:true opens why")
        .replace("PRESS E", "SUMMON: aha_summon:true")
}

fn expose_consolidated_aha_fields<const N: usize>(
    projection: &mut Value,
    consolidated: bool,
    fields: [(&str, Value); N],
) {
    if !consolidated {
        return;
    }
    let object = projection
        .as_object_mut()
        .expect("engineered Aha projection is an object");
    for (key, value) in fields {
        object.insert(key.to_string(), value);
    }
}

pub(super) fn project_flagship_aha(
    room_id: &str,
    variation: u64,
    t: f64,
    inputs: &[numinous_core::RoomInput],
    completed_actions: usize,
    goal_met: bool,
    request: FlagshipAhaRequest,
) -> Result<Option<Value>, String> {
    match room_id {
        "times-tables" => {
            use numinous_core::rooms::times_tables_aha::{AhaBeat, TimesTablesAha};
            let room = numinous_core::rooms::times_tables::TimesTables::new_with(variation);
            let mut aha = TimesTablesAha::new();
            // Match the App: only a real hand primes the K=2 gap. Ambient open
            // phase sits on closed K=2, and must not steal the dial invite with
            // WHERE? chrome before anyone touches the instrument.
            let hand_controls_dial = inputs.iter().any(|input| match *input {
                numinous_core::RoomInput::PointerDown { x, y, .. }
                | numinous_core::RoomInput::PointerMove { x, y, .. }
                | numinous_core::RoomInput::PointerUp { x, y, .. } => {
                    x.is_finite() && y.is_finite()
                }
                _ => false,
            });
            if hand_controls_dial {
                aha.note_hand_multiplier(room.live_multiplier(t, inputs));
            }
            if goal_met {
                let _ = aha.note_four_lobes();
            }
            if let Some(place) = request.place_wager {
                // Generation may open from Explore; a second wager is a no-op.
                let _ = aha.commit_wager(place);
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a place_wager or the four-lobe goal first."
                            .to_string(),
                    );
                }
                advance_aha_to_consolidated(&mut aha);
            }
            // Footer aha status uses its own compact lines; dial detail is optional.
            let dial = None::<String>;
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "place",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(dial.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "placeOptions": ["mandelbrot", "nephroid", "circle"],
                "wager": aha.wager().map(|home| home.label().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!("mandelbrot")),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "buffon-needle" => {
            use numinous_core::rooms::buffon_aha::{AhaBeat, BuffonAha};
            let mut aha = BuffonAha::new();
            let throws = numinous_core::rooms::buffon_needle::BuffonNeedle::throw_count(inputs);
            aha.note_throws(throws);
            if let Some(guess) = request.number_wager {
                let _ = aha.commit_wager(guess);
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a number_wager or eight throws first.".to_string()
                    );
                }
                advance_buffon_to_consolidated(&mut aha);
            }
            let throw_status = None::<String>;
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "number",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(throw_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "guessMin": numinous_core::rooms::buffon_aha::GUESS_MIN,
                "guessMax": numinous_core::rooms::buffon_aha::GUESS_MAX,
                "wager": aha.wager().map(|(guess, _)| guess),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    (
                        "band",
                        json!(
                            aha.wager()
                                .map(|(_, band)| band.name().to_ascii_lowercase())
                        ),
                    ),
                    ("truth", json!(std::f64::consts::PI)),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "galton-board" => {
            use numinous_core::rooms::galton_aha::{AhaBeat, GaltonAha, peak_bin_for_coin};
            let mut aha = GaltonAha::new();
            let waves = numinous_core::rooms::galton_board::wave_count_from_inputs(inputs);
            // Both the earn and the call belong to the pile these pokes
            // build; with no waves yet the fair coin is the honest default.
            let coin =
                numinous_core::rooms::galton_board::selected_coin_from_inputs(inputs).unwrap_or(2);
            aha.note_waves(waves, coin);
            if let Some(bin) = request.bin_wager
                && !aha.commit_wager(bin, coin)
                && !aha.earned()
            {
                return Err(
                    "bin_wager needs a pile to bet on: drop at least one wave of pokes first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err("aha_summon requires a bin_wager or four waves first.".to_string());
                }
                advance_galton_to_consolidated(&mut aha);
            }
            // Unlike Buffon's compact footer, the pile readout must ride
            // every beat: an interaction's status keeps showing what the
            // interaction did (the frozen understanding-study contract
            // checks exactly this), and the invite appends to it instead
            // of replacing it.
            let galton: Box<dyn numinous_core::Room> = Box::new(
                numinous_core::rooms::galton_board::GaltonBoard::new_with(variation),
            );
            let room_status = galton.status_input(t, inputs).or_else(|| galton.status(t));
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "bin",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "binMin": 0,
                "binMax": numinous_core::rooms::galton_board::BOARD_ROWS,
                "coin": coin,
                "wager": aha.wager().map(|(bin, _, _)| bin),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    (
                        "band",
                        json!(
                            aha.wager()
                                .map(|(_, _, band)| band.name().to_ascii_lowercase())
                        ),
                    ),
                    ("truth", json!(peak_bin_for_coin(coin))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "double-pendulum" => {
            use numinous_core::rooms::pendulum_aha::{AhaBeat, PendulumAha};
            let mut aha = PendulumAha::new(variation);
            // A full gesture counts only releases, so a held bob cannot prime
            // or earn the question before the player lets it go. Compact
            // pokes remain static hand points and do not pretend to be lifts.
            let drops = inputs
                .iter()
                .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
                .count();
            let pendulum =
                numinous_core::rooms::double_pendulum::DoublePendulum::new_with(variation);
            if let Some(gap) = pendulum.divergence_at_full_sweep_for_inputs(inputs) {
                let _ = aha.bind_truth_gap(gap);
            }
            aha.note_drops(drops);
            if let Some(ending) = request.ending_wager
                && !aha.commit_call(ending)
                && !aha.earned()
            {
                return Err(
                    "ending_wager needs a completed run: release the pendulum at least once first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires an ending_wager or four completed releases first."
                            .to_string(),
                    );
                }
                advance_pendulum_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> = Box::new(pendulum);
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let (gap, truth) = aha.truth();
            let wager = aha.call();
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "ending",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "endingOptions": ["together", "drifted", "lost"],
                "drops": drops,
                "wager": wager.map(|ending| ending.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!(truth.name().to_ascii_lowercase())),
                    ("gap", json!(gap)),
                    ("right", json!(wager.map(|ending| ending == truth))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "kepler-laws" => {
            use numinous_core::rooms::kepler_aha::{AhaBeat, KeplerAha};
            let eccentricity =
                numinous_core::rooms::kepler_laws::eccentricity_for_inputs(t, inputs, variation);
            let mut aha = KeplerAha::new(eccentricity);
            aha.note_tunings(completed_actions);
            if let Some(relation) = request.speed_wager
                && !aha.commit_call(relation)
                && !aha.earned()
            {
                return Err(
                    "speed_wager needs a chosen orbit: tune the eccentricity with at least one poke or completed gesture first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a speed_wager or four completed tunings first."
                            .to_string(),
                    );
                }
                advance_kepler_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> = Box::new(
                numinous_core::rooms::kepler_laws::KeplerLaws::new_with(variation),
            );
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let truth = aha.truth();
            let wager = aha.call();
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "speed",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "speedOptions": ["faster", "slower", "same"],
                "tunings": completed_actions,
                "eccentricity": aha.eccentricity(),
                "wager": wager.map(|relation| relation.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    (
                        "apsidalSpeedRatio",
                        json!(numinous_core::rooms::kepler_aha::apsidal_speed_ratio(
                            aha.eccentricity()
                        )),
                    ),
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!(truth.name().to_ascii_lowercase())),
                    ("right", json!(wager.map(|relation| relation == truth))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "parrondo" => {
            use numinous_core::rooms::parrondo::{DEMONSTRATION_STEPS, Policy};
            use numinous_core::rooms::parrondo_aha::{AhaBeat, ParrondoAha};
            let mut aha = ParrondoAha::new();
            aha.note_selections(completed_actions);
            if let Some(policy) = request.policy_wager
                && !aha.commit_call(policy)
                && !aha.earned()
            {
                return Err(
                    "policy_wager needs a tried policy: select A, B, or ABB with at least one poke or completed gesture first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a policy_wager or four completed selections first."
                            .to_string(),
                    );
                }
                advance_parrondo_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> = Box::new(
                numinous_core::rooms::parrondo::Parrondo::new_with(variation),
            );
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let truth = aha.truth();
            let wager = aha.call();
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "policy",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "policyOptions": ["a", "b", "abb"],
                "selections": completed_actions,
                "turns": DEMONSTRATION_STEPS,
                "wager": wager.map(|policy| policy.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    (
                        "expectedEnd",
                        json!({
                            "a": aha.expected_end(Policy::OnlyA),
                            "b": aha.expected_end(Policy::OnlyB),
                            "abb": aha.expected_end(Policy::CycleAbb),
                        }),
                    ),
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!(truth.name().to_ascii_lowercase())),
                    ("right", json!(wager.map(|policy| policy == truth))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "nontransitive" => {
            use numinous_core::rooms::nontransitive::{Die, exact_wins, win_rate};
            use numinous_core::rooms::nontransitive_aha::{AhaBeat, NontransitiveAha};
            if request.die_choice.is_some() && completed_actions > 0 {
                return Err(
                    "Choose the first die with either die_choice or room hand inputs, not both."
                        .to_string(),
                );
            }
            let chosen = request
                .die_choice
                .or_else(|| numinous_core::rooms::nontransitive::selected_die_from_inputs(inputs));
            let choices = if request.die_choice.is_some() {
                1
            } else {
                completed_actions
            };
            let mut aha = NontransitiveAha::new();
            aha.note_choices(chosen, choices);
            if let Some(counter) = request.counter_wager
                && !aha.commit_call(counter)
                && !aha.earned()
            {
                return Err(
                    "counter_wager needs a chosen die: pass die_choice a, b, or c, or select one with at least one poke or completed gesture first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a counter_wager or four completed die choices first."
                            .to_string(),
                    );
                }
                advance_nontransitive_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> =
                Box::new(numinous_core::rooms::nontransitive::Nontransitive::new_with(variation));
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let truth = aha.truth();
            let wager = aha.call();
            let chosen_wins = |left: Die, right: Die| exact_wins(left, right);
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "counter",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "dieOptions": ["a", "b", "c"],
                "counterOptions": ["a", "b", "c"],
                "choices": choices,
                "chosen": chosen.map(|die| die.name().to_ascii_lowercase()),
                "faces": {
                    "a": Die::A.faces(),
                    "b": Die::B.faces(),
                    "c": Die::C.faces(),
                },
                "wager": wager.map(|die| die.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    (
                        "exactCycle",
                        json!({
                            "aOverB": chosen_wins(Die::A, Die::B),
                            "bOverC": chosen_wins(Die::B, Die::C),
                            "cOverA": chosen_wins(Die::C, Die::A),
                            "outcomesPerPair": 36,
                        }),
                    ),
                    (
                        "counterWins",
                        json!(
                            chosen
                                .zip(truth)
                                .map(|(chosen, truth)| exact_wins(truth, chosen))
                        ),
                    ),
                    (
                        "counterLosses",
                        json!(
                            chosen
                                .zip(truth)
                                .map(|(chosen, truth)| 36 - exact_wins(truth, chosen))
                        ),
                    ),
                    (
                        "counterRate",
                        json!(
                            chosen
                                .zip(truth)
                                .map(|(chosen, truth)| win_rate(truth, chosen))
                        ),
                    ),
                    ("punchline", json!(aha.punchline())),
                    (
                        "wagerWins",
                        json!(
                            chosen
                                .zip(wager)
                                .map(|(chosen, wager)| exact_wins(wager, chosen))
                        ),
                    ),
                    (
                        "truth",
                        json!(truth.map(|die| die.name().to_ascii_lowercase())),
                    ),
                    (
                        "right",
                        json!(wager.zip(truth).map(|(wager, truth)| wager == truth)),
                    ),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        _ => {
            if request.uses_generation_args() {
                return Err(
                    "Engineered aha arguments are only valid on Times Tables, Buffon's Needle, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, or Nontransitive Dice."
                        .to_string(),
                );
            }
            Ok(None)
        }
    }
}

pub(super) fn render_engineered_aha_overlay(
    room_id: &str,
    aha: Option<&Value>,
    canvas: &mut Canvas,
) {
    let Some(aha) = aha else {
        return;
    };
    let beat = aha.get("beat").and_then(Value::as_str);
    match room_id {
        "kepler-laws" => {
            let eccentricity = aha
                .get("eccentricity")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            match beat {
                Some("prime") => {
                    numinous_core::rooms::kepler_aha::render_speed_band(canvas, None);
                }
                Some("confirm" | "consolidated") => {
                    numinous_core::rooms::kepler_aha::render_equal_time_overlay(
                        canvas,
                        1.0,
                        eccentricity,
                    );
                }
                _ => {}
            }
        }
        "parrondo" => match beat {
            Some("prime") => {
                numinous_core::rooms::parrondo_aha::render_policy_band(canvas, None);
            }
            Some("confirm" | "consolidated") => {
                numinous_core::rooms::parrondo_aha::render_expectation_overlay(canvas, 1.0);
            }
            _ => {}
        },
        "nontransitive" => match beat {
            Some("prime") => {
                numinous_core::rooms::nontransitive_aha::render_counter_band(canvas, None);
            }
            Some("confirm" | "consolidated") => {
                if let Some(chosen) =
                    aha.get("chosen")
                        .and_then(Value::as_str)
                        .and_then(|name| match name {
                            "a" => Some(numinous_core::rooms::nontransitive::Die::A),
                            "b" => Some(numinous_core::rooms::nontransitive::Die::B),
                            "c" => Some(numinous_core::rooms::nontransitive::Die::C),
                            _ => None,
                        })
                {
                    numinous_core::rooms::nontransitive_aha::render_outcome_grid(
                        canvas, 1.0, chosen,
                    );
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn advance_parrondo_to_consolidated(aha: &mut numinous_core::rooms::parrondo_aha::ParrondoAha) {
    if aha.summon() {
        aha.set_morph_progress(1.0);
        let _ = aha.summon();
    }
}

fn advance_nontransitive_to_consolidated(
    aha: &mut numinous_core::rooms::nontransitive_aha::NontransitiveAha,
) {
    if aha.summon() {
        aha.set_morph_progress(1.0);
        let _ = aha.summon();
    }
}

fn advance_galton_to_consolidated(aha: &mut numinous_core::rooms::galton_aha::GaltonAha) {
    use numinous_core::rooms::galton_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_pendulum_to_consolidated(aha: &mut numinous_core::rooms::pendulum_aha::PendulumAha) {
    use numinous_core::rooms::pendulum_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_kepler_to_consolidated(aha: &mut numinous_core::rooms::kepler_aha::KeplerAha) {
    use numinous_core::rooms::kepler_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_aha_to_consolidated(aha: &mut numinous_core::rooms::times_tables_aha::TimesTablesAha) {
    use numinous_core::rooms::times_tables_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_buffon_to_consolidated(aha: &mut numinous_core::rooms::buffon_aha::BuffonAha) {
    use numinous_core::rooms::buffon_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}
