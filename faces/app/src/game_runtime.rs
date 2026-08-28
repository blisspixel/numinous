use super::{
    App, ArcadePlay, GauntletPlay, Key, MunchPlay, NamedKey, NimPlay, QuizPlay, SaveStore,
    controls, play,
};

impl App {
    pub(super) fn quiz_next(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        let room_ids = self.rooms.iter().map(|room| room.meta().id);
        let quiz = play::deal_quiz(seed, self.journey.plays, room_ids, &mut self.quiz_recent);
        self.journey.play();
        self.journey_changed();
        self.quiz = Some(quiz);
        self.sync_room_parameter_voice();
    }

    /// Answer the quiz with a letter; right or wrong, the reveal follows.
    pub(super) fn quiz_answer(&mut self, letter: char) {
        let Some(correct) = self
            .quiz
            .as_mut()
            .and_then(|quiz| play::answer_quiz(quiz, letter))
        else {
            return;
        };
        self.play_game_tick(correct);
        if correct {
            self.journey.win();
            self.journey_changed();
        }
    }

    /// Post a score to the shared table (the CLI's file and rules).
    pub(super) fn post_score(&mut self, key: &str, score: i64) -> bool {
        // A write failure must not wear the same face as "not a new best":
        // that costume hides both the lost score and the reason it was lost.
        match numinous_core::record_score_file(&self.scores_file, key, score) {
            Ok(best) => {
                self.score_save_warned = false;
                best
            }
            Err(error) => {
                self.report_save_trouble(SaveStore::Scores, "score save", &error);
                false
            }
        }
    }

    /// Deal a Munch board (today's).
    pub(super) fn munch_start(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        self.journey.play();
        self.journey_changed();
        let (round, board) = play::deal_munch(seed, self.munch_next_round, self.munch_last_rule);
        self.munch_next_round = round.saturating_add(1);
        self.munch_last_rule = Some(board.rule);
        self.munch = Some(MunchPlay {
            board,
            seed,
            round,
            cursor: 0,
            bites: std::collections::BTreeSet::new(),
            graded: None,
            bite_flash: None,
        });
        self.sync_room_parameter_voice();
    }

    /// Grade the Munch board: the dense feedback, the score, the record.
    pub(super) fn munch_grade(&mut self) {
        let Some(play) = self.munch.as_mut() else {
            return;
        };
        if play.graded.is_some() {
            return;
        }
        let bites: Vec<usize> = play.bites.iter().copied().collect();
        let outcome = numinous_core::grade_munch(&play.board, &bites);
        let clean = numinous_core::munch_clean_win(&outcome);
        let bad = outcome.bad_bites > 0;
        let (seed, round, score) = (play.seed, play.round, outcome.score);
        play.graded = Some(outcome);
        self.post_score(&numinous_core::munch_score_key(seed, round), score);
        if clean {
            self.journey.win();
            self.play_game_tick(true);
        } else if bad {
            // Panel juice: shake the board on a bad bite set, with a low buzz.
            self.screen_shake = 14;
            self.play_game_buzz(seed ^ round);
        } else {
            self.play_game_tick(false);
        }
        self.journey_changed();
    }

    /// Deal a Nim game (today's heaps).
    pub(super) fn nim_start(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        self.journey.play();
        self.journey_changed();
        let heaps = numinous_core::nim_new(seed);
        self.nim = Some(NimPlay {
            selected: heaps.iter().position(|&h| h > 0).unwrap_or(0),
            take: 1,
            heaps,
            seed,
            message: String::from("THE ORDER PLAYS A SECRET. BEAT IT AND IT IS YOURS."),
            over: None,
        });
        self.sync_room_parameter_voice();
    }

    /// Commit the aimed Nim move; the Order answers at once.
    pub(super) fn nim_move(&mut self) {
        let tick = {
            let Some(play) = self.nim.as_mut() else {
                return;
            };
            if play.over.is_some() {
                return;
            }
            if !numinous_core::nim_apply(&mut play.heaps, play.selected, play.take) {
                play.message = String::from("THAT MOVE IS NOT ON THE BOARD.");
                Some(false)
            } else if numinous_core::nim_finished(&play.heaps) {
                play.over = Some(true);
                let seed = play.seed;
                self.journey.win();
                self.post_score(&format!("nim seed:{seed}"), 1);
                Some(true)
            } else {
                let (heap, take) = numinous_core::nim_order(&play.heaps);
                let _ = numinous_core::nim_apply(&mut play.heaps, heap, take);
                if numinous_core::nim_finished(&play.heaps) {
                    play.over = Some(false);
                    play.message =
                        String::from("THE ORDER TAKES THE LAST STONE. AGAIN. (NOT LUCK.)");
                    Some(false)
                } else {
                    play.message = format!("THE ORDER TAKES {take} FROM HEAP {}.", heap + 1);
                    if play.heaps.get(play.selected).copied().unwrap_or(0) == 0 {
                        play.selected = play.heaps.iter().position(|&h| h > 0).unwrap_or(0);
                    }
                    play.take = play.take.min(play.heaps[play.selected].max(1));
                    Some(true)
                }
            }
        };
        if matches!(tick, Some(true)) {
            // Win path posts score before journey change so the borrow ends first.
            if self
                .nim
                .as_ref()
                .is_some_and(|play| play.over == Some(true))
            {
                self.journey_changed();
            }
        }
        if let Some(good) = tick {
            self.play_game_tick(good);
        }
    }

    /// Start the arcade: today's run, spirits loose, the beat ticking.
    pub(super) fn arcade_start(&mut self) {
        self.the_show = false;
        // Clear any stale pause from the wander view: the arcade is real-time, and
        // a leaked pause would freeze the Vexations while the player kept eating,
        // then post an unfairly-earned score to the shared table.
        self.paused = false;
        let seed = play::daily_seed();
        self.journey.play();
        self.journey_changed();
        self.arcade = Some(ArcadePlay {
            run: numinous_core::munch_arcade::Arcade::new(seed),
            seed,
            flash: None,
            over: false,
        });
        self.sync_room_parameter_voice();
    }

    /// One player action into the live arcade.
    pub(super) fn arcade_act(&mut self, action: numinous_core::munch_arcade::Action) {
        use numinous_core::munch_arcade::Turn;
        let Some(play) = self.arcade.as_mut() else {
            return;
        };
        if play.over {
            return;
        }
        match play.run.act(action) {
            Turn::Cleared => {
                play.flash = Some((false, 40));
                self.journey.win();
                self.journey_changed();
            }
            Turn::Over => play.over = true,
            _ => {}
        }
    }

    /// The spirits' beat: called from the frame clock.
    pub(super) fn arcade_beat(&mut self) {
        use numinous_core::munch_arcade::Turn;
        let feedback = {
            let Some(play) = self.arcade.as_mut() else {
                return;
            };
            if play.over {
                return;
            }
            match play.run.tick() {
                Turn::Caught => {
                    play.flash = Some((true, 40));
                    Some(false)
                }
                Turn::Over => {
                    play.over = true;
                    let (seed, score) = (play.seed, play.run.score);
                    self.post_score(&format!("arcade seed:{seed}"), score);
                    Some(false)
                }
                Turn::Cleared => Some(true),
                Turn::Going => None,
            }
        };
        if let Some(good) = feedback {
            if good {
                self.play_game_tick(true);
            } else if self.arcade.as_ref().is_some_and(|play| play.over) {
                let seed = self.arcade.as_ref().map(|play| play.seed).unwrap_or(0);
                self.play_game_buzz(seed);
            } else {
                self.play_game_tick(false);
            }
        }
    }

    /// Start the Gauntlet: today's run, four stages, a combo.
    pub(super) fn gauntlet_start(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        let puzzle = numinous_core::GauntletPuzzle::new(seed);
        let secret = puzzle.bomb_code().to_vec();
        self.gauntlet = Some(GauntletPlay {
            seed,
            stage: 0,
            munch: MunchPlay {
                board: puzzle.munch,
                seed,
                round: 0,
                cursor: 0,
                bites: std::collections::BTreeSet::new(),
                graded: None,
                bite_flash: None,
            },
            quiz: QuizPlay {
                round: puzzle.shape,
                flash: None,
            },
            scan: puzzle.sky,
            secret,
            wire: String::new(),
            wire_lines: Vec::new(),
            scores: Vec::new(),
            cleared: Vec::new(),
            message: String::from("STAGE 1 OF 4  MUNCH. CLEAN STAGES BUILD YOUR COMBO."),
        });
        self.sync_room_parameter_voice();
    }

    /// Bank a gauntlet stage: score, clean flag, journey, and the narration.
    fn gauntlet_bank(&mut self, points: i64, clean: bool, what: &str) {
        self.journey.play();
        if clean {
            self.journey.win();
        }
        self.journey_changed();
        let Some(run) = self.gauntlet.as_mut() else {
            return;
        };
        run.scores.push(points);
        run.cleared.push(clean);
        run.stage += 1;
        let combo = run.cleared.iter().take_while(|&&c| c).count() + 1;
        run.message = if run.stage < 4 {
            format!(
                "{what}  STAGE {} OF 4{}",
                run.stage + 1,
                if clean {
                    format!("  COMBO X{combo}")
                } else {
                    String::new()
                }
            )
        } else {
            what.to_string()
        };
        if run.stage == 4 {
            let total = numinous_core::gauntlet_total(&run.scores, &run.cleared);
            let seed = run.seed;
            self.post_score(&numinous_core::gauntlet_score_key(seed), total);
        }
    }

    /// One key into the Gauntlet: routed to whichever stage is live.
    pub(super) fn gauntlet_key(&mut self, key: &Key) {
        if matches!(key, Key::Named(NamedKey::Escape)) {
            self.gauntlet = None;
            self.clear_transient_audio();
            self.update_audio();
            return;
        }
        let Some(run) = self.gauntlet.as_mut() else {
            return;
        };
        match run.stage {
            0 => {
                let play = &mut run.munch;
                match key {
                    Key::Named(NamedKey::Enter) => {
                        let bites: Vec<usize> = play.bites.iter().copied().collect();
                        let outcome = numinous_core::grade_munch(&play.board, &bites);
                        let clean = numinous_core::munch_clean_win(&outcome);
                        let (points, what) = (outcome.score, format!("MUNCH +{}.", outcome.score));
                        self.gauntlet_bank(points, clean, &what);
                    }
                    key => {
                        if let Some(cell) =
                            controls::apply_munch_control(&mut play.cursor, &mut play.bites, key)
                        {
                            play.flash_bite(cell);
                            // Gauntlet stage keeps the crunch; wrong-bite shake
                            // lives on the standalone Munch path (borrowed stage).
                            self.play_munch_crunch(cell as u64 ^ 0x6A17);
                        }
                    }
                }
            }
            1 => {
                if let Key::Character(c) = key
                    && c.len() == 1
                {
                    let letter = c.chars().next().unwrap_or(' ').to_ascii_uppercase();
                    if let Some(correct) = play::answer_quiz(&mut run.quiz, letter) {
                        let what = format!(
                            "IT WAS {} ({}).",
                            run.quiz.round.answer,
                            run.quiz.round.answer_title.to_uppercase()
                        );
                        let grade = numinous_core::gauntlet_choice_grade(correct);
                        self.gauntlet_bank(grade.score, grade.clean, &what);
                    }
                }
            }
            2 => {
                if let Key::Character(c) = key
                    && c.len() == 1
                {
                    let letter = c.chars().next().unwrap_or(' ').to_ascii_uppercase();
                    if run.scan.channels.iter().any(|ch| ch.letter == letter) {
                        let correct = letter == run.scan.answer;
                        let what = format!("THE SIGNAL WAS {}.", run.scan.answer);
                        let grade = numinous_core::gauntlet_choice_grade(correct);
                        self.gauntlet_bank(grade.score, grade.clean, &what);
                    }
                }
            }
            3 => match key {
                Key::Named(NamedKey::Backspace) => {
                    run.wire.pop();
                }
                Key::Named(NamedKey::Enter) => {
                    let guess: Vec<u8> = run
                        .wire
                        .chars()
                        .filter(char::is_ascii_digit)
                        .map(|c| c as u8 - b'0')
                        .collect();
                    if guess.len() != 4 {
                        return;
                    }
                    let attempt = run.wire_lines.len() + 1;
                    let Some(grade) =
                        numinous_core::gauntlet_wire_grade(&run.secret, attempt, &guess)
                    else {
                        return;
                    };
                    if grade.stage.clean {
                        self.gauntlet_bank(grade.stage.score, true, "DEFUSED.");
                        return;
                    }
                    run.wire_lines.push(format!(
                        "{}: {} LOCKED, {} LOOSE",
                        run.wire, grade.feedback.locked, grade.feedback.loose
                    ));
                    run.wire.clear();
                    if run.wire_lines.len() >= 5 {
                        let code: String = run
                            .secret
                            .iter()
                            .map(|&digit| char::from(b'0' + digit))
                            .collect();
                        self.gauntlet_bank(0, false, &format!("BOOM. IT WAS {code}."));
                    }
                }
                Key::Character(c) if run.wire.len() < 4 => {
                    for ch in c.chars().filter(char::is_ascii_digit) {
                        if run.wire.len() < 4 {
                            run.wire.push(ch);
                        }
                    }
                }
                _ => {}
            },
            _ => {
                self.gauntlet = None;
                self.clear_transient_audio();
                self.update_audio();
            }
        }
    }

    /// One key into standalone Munch.
    pub(super) fn munch_key(&mut self, key: &Key) {
        let graded = self
            .munch
            .as_ref()
            .is_some_and(|play| play.graded.is_some());
        if graded {
            match key {
                Key::Named(NamedKey::Escape) => {
                    self.munch = None;
                    self.clear_transient_audio();
                    self.update_audio();
                }
                Key::Named(NamedKey::Enter | NamedKey::Space) => self.munch_start(),
                _ => {}
            }
            return;
        }
        match key {
            Key::Named(NamedKey::Escape) => {
                self.munch = None;
                self.clear_transient_audio();
                self.update_audio();
            }
            Key::Named(NamedKey::Enter) => self.munch_grade(),
            key => {
                let feedback = if let Some(play) = &mut self.munch {
                    let was = play.bites.contains(&play.cursor);
                    if let Some(cell) =
                        controls::apply_munch_control(&mut play.cursor, &mut play.bites, key)
                    {
                        play.flash_bite(cell);
                        let now = play.bites.contains(&cell);
                        Some((play.board.clone(), play.seed, cell, was, now))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((board, seed, cell, was, now)) = feedback {
                    self.munch_bite_feedback(&board, seed, cell, was, now);
                }
            }
        }
    }

    /// Soft one-shot noise tick over the room score (Munch bite juice).
    pub(super) fn play_munch_crunch(&self, seed: u64) {
        let Some(player) = &self.player else {
            return;
        };
        if self.muted {
            return;
        }
        let samples = numinous_core::munch_crunch(player.sample_rate(), seed);
        player.play_oneshot(samples, 0.55 * self.volume);
    }

    /// Bright or low square tick for quiz, nim, and graded munch feedback.
    fn play_game_tick(&self, good: bool) {
        let Some(player) = &self.player else {
            return;
        };
        if self.muted {
            return;
        }
        let samples = numinous_core::game_tick(player.sample_rate(), good);
        player.play_oneshot(samples, 0.5 * self.volume);
    }

    /// Low buzz for a bad Munch grade (pairs with screen shake).
    fn play_game_buzz(&self, seed: u64) {
        let Some(player) = &self.player else {
            return;
        };
        if self.muted {
            return;
        }
        let samples = numinous_core::game_buzz(player.sample_rate(), seed);
        player.play_oneshot(samples, 0.45 * self.volume);
    }

    /// One key into standalone Nim, including an explicit retry after either
    /// result so a loss can teach without ejecting the player.
    pub(super) fn nim_key(&mut self, key: &Key) {
        let over = self.nim.as_ref().is_some_and(|play| play.over.is_some());
        if over {
            match key {
                Key::Named(NamedKey::Escape) => {
                    self.nim = None;
                    self.update_audio();
                }
                Key::Named(NamedKey::Enter | NamedKey::Space) => self.nim_start(),
                _ => {}
            }
            return;
        }
        match key {
            Key::Named(NamedKey::Escape) => {
                self.nim = None;
                self.update_audio();
            }
            Key::Named(NamedKey::Enter) => self.nim_move(),
            key => {
                if let Some(play) = &mut self.nim {
                    let _ = controls::apply_nim_control(play, key);
                }
            }
        }
    }

    /// Report a file-producing key's outcome. Success names the path in the
    /// window title as always; failure says so on screen and in the crash
    /// log, because a save key that fails silently looks exactly like a save
    /// key that worked.
    fn munch_wrong_bite_juice(&mut self, seed: u64) {
        self.screen_shake = self.screen_shake.max(6);
        self.play_game_buzz(seed);
    }

    /// Crunch plus optional wrong-bite buzz when a toggle turns a bite on.
    pub(super) fn munch_bite_feedback(
        &mut self,
        board: &numinous_core::Board,
        seed: u64,
        cell: usize,
        was_bitten: bool,
        now_bitten: bool,
    ) {
        self.play_munch_crunch(cell as u64);
        if now_bitten && !was_bitten {
            let value = board.numbers.get(cell).copied().unwrap_or(0);
            if !board.rule.fits(value) {
                self.munch_wrong_bite_juice(seed ^ cell as u64 ^ 0x0000_0BAD_B17E);
            }
        }
    }
}
