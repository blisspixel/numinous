use numinous_core::{Raster, Room, Surface};

use crate::{
    input_legend::{self, InputMode},
    play::{ArcadePlay, GauntletPlay, MunchPlay, NimPlay, QuizPlay, gauntlet_total},
};

fn game_scale(width: usize) -> i32 {
    (width as i32 / 400).clamp(1, 3)
}

pub(crate) struct QuizChoiceLayout {
    base: i32,
    line_height: i32,
}

impl QuizChoiceLayout {
    pub(crate) fn new(width: usize, height: usize, choice_count: usize) -> Self {
        let line_height = 10 * game_scale(width);
        let base = height as i32 - (choice_count as i32 + 1) * line_height - 8;
        Self { base, line_height }
    }

    pub(crate) fn hit(&self, y: f64, choice_count: usize) -> Option<usize> {
        if y < f64::from(self.base) || self.line_height <= 0 {
            return None;
        }
        let index = ((y - f64::from(self.base)) / f64::from(self.line_height)) as usize;
        (index < choice_count).then_some(index)
    }
}

pub(crate) struct MunchLayout {
    top: i32,
    cell_w: i32,
    cell_h: i32,
}

impl MunchLayout {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        let scale = game_scale(width);
        let top = 14 * scale + 10;
        let cell_w = (width as i32 - 20) / 6;
        let cell_h = (height as i32 - top - 24 * scale) / 5;
        Self {
            top,
            cell_w,
            cell_h,
        }
    }

    fn rect(&self, cell: usize) -> (i32, i32, i32, i32) {
        let (col, row) = (cell as i32 % 6, cell as i32 / 6);
        let (x0, y0) = (10 + col * self.cell_w, self.top + row * self.cell_h);
        let (x1, y1) = (x0 + self.cell_w - 3, y0 + self.cell_h - 3);
        (x0, y0, x1, y1)
    }

    pub(crate) fn hit(&self, x: f64, y: f64) -> Option<usize> {
        if self.cell_w <= 1 || self.cell_h <= 1 || x < 10.0 || y < f64::from(self.top) {
            return None;
        }
        let col = ((x - 10.0) / f64::from(self.cell_w)) as usize;
        let row = ((y - f64::from(self.top)) / f64::from(self.cell_h)) as usize;
        (col < 6 && row < 5).then_some(row * 6 + col)
    }
}

/// Hit-test for Nim heaps: click a stone to aim that heap and take that many.
pub(crate) struct NimLayout {
    top: i32,
    row_h: i32,
    stone: i32,
}

impl NimLayout {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        let scale = game_scale(width);
        let top = 20 * scale + 10;
        let row_h = (height as i32 - top - 42 * scale) / 3;
        let stone = (row_h / 2).clamp(4, 10 * scale);
        Self { top, row_h, stone }
    }

    /// Returns `(heap_index, take_count)` when the pointer is over stones.
    pub(crate) fn hit(&self, x: f64, y: f64, heaps: &[u32]) -> Option<(usize, u32)> {
        if self.row_h <= 0 || heaps.is_empty() {
            return None;
        }
        let heap = ((y - f64::from(self.top)) / f64::from(self.row_h)) as usize;
        if heap >= heaps.len().min(3) {
            return None;
        }
        let count = heaps[heap];
        if count == 0 {
            return None;
        }
        let stone_span = self.stone + 6;
        if stone_span <= 0 || x < 40.0 {
            // Click on the heap label: aim take 1.
            return (10.0..40.0).contains(&x).then_some((heap, 1));
        }
        let index = ((x - 40.0) / f64::from(stone_span)) as u32;
        if index >= count {
            return None;
        }
        // Stone i from the left is the (count - index)th from the right aim set.
        let take = count.saturating_sub(index).max(1);
        Some((heap, take))
    }
}

fn munch_columns(width: usize, scale: i32, left: i32) -> usize {
    ((width as i32 - left - 10) / (6 * scale.max(1))).max(8) as usize
}

fn control_lines(copy: &str, width: usize, scale: i32, left: i32) -> Vec<String> {
    numinous_core::wrap_text(copy, munch_columns(width, scale, left))
}

fn munch_control_lines(mode: InputMode, width: usize, scale: i32) -> Vec<String> {
    control_lines(&input_legend::munch_live(mode), width, scale, 10)
}

fn munch_result_lines(
    outcome: &numinous_core::Munched,
    mode: InputMode,
    width: usize,
    scale: i32,
) -> Vec<String> {
    let clean = outcome.bad_bites == 0 && outcome.left_behind == 0 && outcome.hits > 0;
    let mut semantic = vec![format!(
        "{} +{}",
        if clean { "PERFECT." } else { "DONE." },
        outcome.score
    )];
    semantic.push(format!(
        "{} EATEN  {} BAD  {} LEFT",
        outcome.hits, outcome.bad_bites, outcome.left_behind
    ));
    if !outcome.wrongly_eaten.is_empty() {
        let listed: Vec<String> = outcome.wrongly_eaten.iter().map(u64::to_string).collect();
        semantic.push(format!("WRONG: {}", listed.join(", ")));
    }
    if !outcome.missed.is_empty() {
        let listed: Vec<String> = outcome.missed.iter().map(u64::to_string).collect();
        semantic.push(format!("WALKED PAST: {}", listed.join(", ")));
        if outcome.bad_bites == 0 && outcome.missed.len() == 1 {
            semantic.push("ONE AWAY. THE BOARD REMEMBERS.".to_string());
        }
    }
    semantic.push(input_legend::munch_result(mode));

    let columns = munch_columns(width, scale, width as i32 / 8);
    semantic
        .into_iter()
        .flat_map(|line| numinous_core::wrap_text(&line, columns))
        .collect()
}

fn centered_text_x(text: &str, width: usize, scale: i32) -> i32 {
    ((width as i32 - text.chars().count() as i32 * 6 * scale) / 2).max(10)
}

const FONT_PIXEL_HEIGHT: i32 = 7;

struct QuizResultLayout {
    verdict: String,
    reveal_lines: Vec<String>,
    controls: String,
    reveal_y: i32,
    controls_y: i32,
    body_scale: i32,
    verdict_scale: i32,
    line_height: i32,
}

impl QuizResultLayout {
    fn new(quiz: &QuizPlay, correct: bool, mode: InputMode, width: usize, height: usize) -> Self {
        let margin = 10;
        let body_scale = game_scale(width);
        let verdict_scale = body_scale + 1;
        let line_height = 10 * body_scale;
        let verdict = if correct {
            "CORRECT".to_string()
        } else {
            format!(
                "IT WAS {}: {}",
                quiz.round.answer,
                quiz.round.answer_title.to_uppercase()
            )
        };
        let controls = input_legend::quiz_result(mode);

        let reveal_y = margin + FONT_PIXEL_HEIGHT * verdict_scale + line_height;
        let controls_y = height as i32 - FONT_PIXEL_HEIGHT * body_scale - margin;
        let reveal_bottom = controls_y - line_height;
        let max_lines = if reveal_bottom < reveal_y + FONT_PIXEL_HEIGHT * body_scale {
            0
        } else {
            1 + (reveal_bottom - reveal_y - FONT_PIXEL_HEIGHT * body_scale) / line_height
        } as usize;
        let columns = ((width as i32 - 2 * margin) / (6 * body_scale)).max(12) as usize;
        let reveal_lines =
            numinous_core::wrap_text(&quiz.round.answer_reveal.to_uppercase(), columns)
                .into_iter()
                .take(max_lines)
                .collect();

        Self {
            verdict,
            reveal_lines,
            controls,
            reveal_y,
            controls_y,
            body_scale,
            verdict_scale,
            line_height,
        }
    }
}

fn gauntlet_message_lines(message: &str, width: usize, scale: i32) -> Vec<String> {
    let columns = ((width as i32 - 20) / (6 * scale)).max(8) as usize;
    let mut lines: Vec<String> = numinous_core::wrap_text(message, columns)
        .into_iter()
        .take(2)
        .collect();
    if lines.is_empty() {
        lines.push(message.to_string());
    }
    lines
}

fn gauntlet_stage_title(stage: usize) -> &'static str {
    match stage {
        0 => "GAUNTLET 1/4: MUNCH",
        1 => "GAUNTLET 2/4: SHAPE",
        2 => "GAUNTLET 3/4: SKY",
        3 => "GAUNTLET 4/4: BOMB",
        _ => "GAUNTLET COMPLETE",
    }
}

fn draw_signal_trace(raster: &mut Raster, trace: &str, x: i32, y: i32, width: usize, scale: i32) {
    let available = (width as i32 - x - 10).max(1);
    let count = trace.chars().count().max(1) as i32;
    let step = (available / count).max(1);
    for (index, pulse) in trace.chars().enumerate() {
        if pulse == '#' {
            let px = x + index as i32 * step;
            raster.line(px, y, px, y + 6 * scale, '#');
            if step > 1 {
                raster.line(px + 1, y, px + 1, y + 6 * scale, '#');
            }
        }
    }
}

fn gauntlet_header_height(message_lines: usize, scale: i32) -> i32 {
    18 * scale + message_lines as i32 * 9 * scale
}

fn gauntlet_result_lines(
    run: &GauntletPlay,
    mode: InputMode,
    width: usize,
    scale: i32,
) -> Vec<String> {
    let total = gauntlet_total(&run.scores, &run.cleared);
    let clears = run.cleared.iter().filter(|&&clean| clean).count();
    let names = ["MUNCH", "SHAPE", "SKY", "BOMB"];
    let mut semantic = vec![format!("RUN COMPLETE  {clears}/4 CLEAN")];
    let mut combo = 1;
    for ((name, score), &clean) in names.iter().zip(&run.scores).zip(&run.cleared) {
        semantic.push(format!(
            "{name}  +{score} X{combo} = {}{}",
            score * combo,
            if clean { "  CLEAN" } else { "" }
        ));
        combo = if clean { combo + 1 } else { 1 };
    }
    semantic.push(format!("TOTAL {total}  GAUNTLET SEED {}", run.seed));
    semantic.push(input_legend::gauntlet_done(mode));
    let columns = ((width as i32 - 20) / (6 * scale)).max(8) as usize;
    semantic
        .into_iter()
        .flat_map(|line| numinous_core::wrap_text(&line, columns))
        .collect()
}

/// Draw the quiz: the mystery room fullscreen, the choices at the bottom,
/// and after an answer, the verdict and the reveal.
pub(crate) fn draw_quiz(
    rooms: &[Box<dyn Room>],
    quiz: &QuizPlay,
    mode: InputMode,
    width: usize,
    height: usize,
) -> Raster {
    let answer_id = quiz
        .round
        .choices
        .iter()
        .find(|c| c.letter == quiz.round.answer)
        .map_or("", |c| c.id);
    let mystery = rooms.iter().find(|r| r.meta().id == answer_id);
    let mut raster = match mystery {
        Some(room) => {
            let mut raster = Raster::with_accent(width, height, room.meta().accent);
            room.render(&mut raster, room.postcard_t().max(0.4));
            raster
        }
        None => Raster::new(width, height),
    };
    let scale = game_scale(width);
    let layout = QuizChoiceLayout::new(width, height, quiz.round.choices.len());
    let line_height = layout.line_height;
    match &quiz.flash {
        None => {
            raster.clear_rows(0, 14 + 7 * (scale + 1));
            numinous_core::draw_text(&mut raster, "WHICH MATH MADE THIS?", 10, 10, scale + 1, '#');
            raster.dim_rows(layout.base - 6, height as i32, 40);
            for (i, choice) in quiz.round.choices.iter().enumerate() {
                let direction = input_legend::quiz_direction(mode, i);
                let line = if direction.is_empty() {
                    format!("{}  {}", choice.letter, choice.title.to_uppercase())
                } else {
                    format!(
                        "{direction:<5} {}  {}",
                        choice.letter,
                        choice.title.to_uppercase()
                    )
                };
                numinous_core::draw_text(
                    &mut raster,
                    &line,
                    10,
                    layout.base + i as i32 * line_height,
                    scale,
                    '#',
                );
            }
        }
        Some((correct, _)) => {
            raster.clear_rows(0, height as i32);
            let result = QuizResultLayout::new(quiz, *correct, mode, width, height);
            numinous_core::draw_text(
                &mut raster,
                &result.verdict,
                10,
                10,
                result.verdict_scale,
                '#',
            );
            for (i, line) in result.reveal_lines.iter().enumerate() {
                numinous_core::draw_text(
                    &mut raster,
                    line,
                    10,
                    result.reveal_y + i as i32 * result.line_height,
                    result.body_scale,
                    '#',
                );
            }
            numinous_core::draw_text(
                &mut raster,
                &result.controls,
                10,
                result.controls_y,
                result.body_scale,
                '*',
            );
        }
    }
    raster
}

/// Draw Munch: the 5x6 board as a grid, the cursor, your bites, and the rule.
pub(crate) fn draw_munch(
    play: &MunchPlay,
    frame: u64,
    mode: InputMode,
    width: usize,
    height: usize,
) -> Raster {
    let mut raster = Raster::with_accent(width, height, [140, 230, 120]);
    let scale = game_scale(width);
    raster.dim_rows(0, 12 + 7 * scale, 40);
    raster.dim_rows(height as i32 - 24 * scale, height as i32, 40);
    numinous_core::draw_text(
        &mut raster,
        &format!("MUNCH: {}", play.board.rule.describe().to_uppercase()),
        10,
        10,
        scale,
        '#',
    );
    let layout = MunchLayout::new(width, height);
    for (i, &value) in play.board.numbers.iter().enumerate() {
        let (x0, y0, x1, y1) = layout.rect(i);
        let bitten = play.bites.contains(&i);
        let mark = if bitten { '#' } else { '-' };
        raster.line(x0, y0, x1, y0, mark);
        raster.line(x0, y1, x1, y1, mark);
        raster.line(x0, y0, x0, y1, mark);
        raster.line(x1, y0, x1, y1, mark);
        if i == play.cursor && play.graded.is_none() {
            let inset = if (frame / 20) % 2 == 0 { 1 } else { 2 };
            raster.line(x0 + inset, y0 + inset, x1 - inset, y0 + inset, '#');
            raster.line(x0 + inset, y1 - inset, x1 - inset, y1 - inset, '#');
            raster.line(x0 + inset, y0 + inset, x0 + inset, y1 - inset, '#');
            raster.line(x1 - inset, y0 + inset, x1 - inset, y1 - inset, '#');
        }
        let label = value.to_string();
        let tx = x0 + layout.cell_w / 2 - (label.len() as i32 * 3 * scale);
        let ty = y0 + layout.cell_h / 2 - 4 * scale;
        numinous_core::draw_text(
            &mut raster,
            &label,
            tx,
            ty,
            scale,
            if bitten { '#' } else { '*' },
        );
    }
    match &play.graded {
        None => {
            let lines = munch_control_lines(mode, width, scale);
            for (i, line) in lines.iter().enumerate() {
                let y = height as i32 - (lines.len() - i) as i32 * 9 * scale;
                numinous_core::draw_text(&mut raster, line, 10, y, scale, '*');
            }
        }
        Some(outcome) => {
            raster.dim(30);
            let ls = (width as i32 / 400).clamp(1, 3);
            let lines = munch_result_lines(outcome, mode, width, ls);
            let lh = 10 * ls;
            let ttop = (height as i32 / 2) - (lines.len() as i32 * lh) / 2;
            let panel_top = (ttop - 6).max(0);
            let panel_bottom = (ttop + lines.len() as i32 * lh + 6).min(height as i32);
            raster.clear_rows(panel_top, panel_bottom);
            raster.line(0, panel_top, width.saturating_sub(1) as i32, panel_top, '-');
            raster.line(
                0,
                panel_bottom.saturating_sub(1),
                width.saturating_sub(1) as i32,
                panel_bottom.saturating_sub(1),
                '-',
            );
            for (i, line) in lines.iter().enumerate() {
                numinous_core::draw_text(
                    &mut raster,
                    line,
                    width as i32 / 8,
                    ttop + i as i32 * lh,
                    ls,
                    '#',
                );
            }
        }
    }
    raster
}

/// Draw the live arcade: the board, the Muncher, the spirits, and the beat.
pub(crate) fn draw_arcade(
    play: &ArcadePlay,
    mode: InputMode,
    width: usize,
    height: usize,
) -> Raster {
    use numinous_core::munch_arcade::Mind;
    use numinous_core::munchers::{COLS, ROWS};

    let mut raster = Raster::with_accent(width, height, [255, 205, 100]);
    let scale = game_scale(width);
    raster.dim_rows(0, 12 + 7 * scale, 40);
    raster.dim_rows(height as i32 - 24 * scale, height as i32, 40);
    let run = &play.run;
    numinous_core::draw_text(
        &mut raster,
        &format!(
            "ARCADE  LV {}  {}  {}",
            run.level,
            "O ".repeat(run.lives as usize),
            run.board.rule.describe().to_uppercase()
        ),
        10,
        10,
        scale,
        '#',
    );
    let top = 14 * scale + 10;
    let cell_w = (width as i32 - 20) / COLS as i32;
    let cell_h = (height as i32 - top - 24 * scale) / ROWS as i32;
    for cell in 0..ROWS * COLS {
        let (col, row) = (cell as i32 % COLS as i32, cell as i32 / COLS as i32);
        let (x0, y0) = (10 + col * cell_w, top + row * cell_h);
        let (x1, y1) = (x0 + cell_w - 3, y0 + cell_h - 3);
        raster.line(x0, y0, x1, y0, '-');
        raster.line(x0, y1, x1, y1, '-');
        raster.line(x0, y0, x0, y1, '-');
        raster.line(x1, y0, x1, y1, '-');
        let center_x = x0 + cell_w / 2;
        let center_y = y0 + cell_h / 2;
        if cell == run.muncher {
            let r = (cell_h / 3).max(4);
            for i in 0..40 {
                let a = std::f64::consts::TAU * f64::from(i) / 40.0;
                if !(0.9..=5.4).contains(&a) {
                    continue;
                }
                let px = center_x + (f64::from(r) * a.cos()) as i32;
                let py = center_y + (f64::from(r) * a.sin()) as i32;
                raster.plot(px, py, '#');
                raster.plot(px + 1, py, '#');
            }
        } else if let Some(v) = run.vexations.iter().find(|v| v.cell == cell) {
            let mark = match v.mind {
                Mind::Tracker => "T",
                Mind::Drifter => "D",
                Mind::Editor => "E",
            };
            numinous_core::draw_text(
                &mut raster,
                mark,
                center_x - 3 * scale,
                center_y - 4 * scale,
                scale + 1,
                '#',
            );
        } else if !run.eaten[cell] {
            let label = run.board.numbers[cell].to_string();
            numinous_core::draw_text(
                &mut raster,
                &label,
                center_x - (label.len() as i32 * 3 * scale),
                center_y - 4 * scale,
                scale,
                '*',
            );
        }
    }
    if let Some((caught, _)) = play.flash {
        raster.dim(55);
        let message = if caught {
            "CAUGHT"
        } else {
            "CLEAR. THEY MULTIPLY."
        };
        let flash_scale = (scale + 1).min(4);
        let flash_y = height as i32 / 2 - 6 * scale;
        raster.clear_rows(flash_y - 6, flash_y + 7 * flash_scale + 6);
        numinous_core::draw_text(
            &mut raster,
            message,
            centered_text_x(message, width, flash_scale),
            flash_y,
            flash_scale,
            '#',
        );
    }
    if play.over {
        raster.dim(30);
        let lines = [
            "THE SPIRITS SEND REGARDS".to_string(),
            format!("LEVEL {}  SCORE {}", run.level, run.score),
            input_legend::arcade_over(mode),
        ];
        let ls = (width as i32 / 300).clamp(2, 4);
        let top = height as i32 / 2 - 18 * ls;
        for (i, line) in lines.iter().enumerate() {
            numinous_core::draw_text(
                &mut raster,
                line,
                width as i32 / 8,
                top + i as i32 * 12 * ls,
                ls,
                '#',
            );
        }
    }
    if !play.over {
        let controls = control_lines(&input_legend::arcade_live(mode), width, scale, 10);
        for (index, line) in controls.iter().enumerate() {
            let y = height as i32 - (controls.len() - index) as i32 * 9 * scale;
            numinous_core::draw_text(&mut raster, line, 10, y, scale, '*');
        }
    }
    raster
}

/// Draw Nim: heaps as stones, your aim highlighted, and the Order's last word.
pub(crate) fn draw_nim(play: &NimPlay, mode: InputMode, width: usize, height: usize) -> Raster {
    let mut raster = Raster::with_accent(width, height, [230, 200, 120]);
    let scale = game_scale(width);
    raster.dim_rows(0, 12 + 7 * scale, 40);
    raster.dim_rows(height as i32 - 38 * scale, height as i32, 40);
    numinous_core::draw_text(&mut raster, "NIM: LAST STONE WINS", 10, 10, scale, '#');
    let top = 20 * scale + 10;
    let row_h = (height as i32 - top - 42 * scale) / 3;
    let stone = (row_h / 2).clamp(4, 10 * scale);
    for (heap, &count) in play.heaps.iter().enumerate() {
        let y = top + heap as i32 * row_h + row_h / 2;
        let selected = heap == play.selected && play.over.is_none();
        numinous_core::draw_text(
            &mut raster,
            &format!("{}{}", if selected { ">" } else { " " }, heap + 1),
            10,
            y - 4 * scale,
            scale,
            if selected { '#' } else { '*' },
        );
        for i in 0..count {
            let x0 = 40 + i as i32 * (stone + 6);
            let aimed = selected && i >= count.saturating_sub(play.take);
            let mark = if aimed { '#' } else { '*' };
            for dy in 0..stone {
                raster.line(x0, y + dy, x0 + stone, y + dy, mark);
            }
        }
    }
    if play.over.is_none() {
        numinous_core::draw_text(
            &mut raster,
            &play.message,
            10,
            height as i32 - 31 * scale,
            scale,
            '#',
        );
        let controls = control_lines(&input_legend::nim_live(mode, play.take), width, scale, 10);
        for (index, line) in controls.iter().enumerate() {
            let y = height as i32 - (controls.len() - index) as i32 * 9 * scale;
            numinous_core::draw_text(&mut raster, line, 10, y, scale, '*');
        }
    }
    if play.over == Some(true) {
        raster.dim(25);
        let ls = (width as i32 / 340).clamp(1, 3);
        let columns = ((width as i32 / (6 * ls)) - 6).max(12) as usize;
        let mut lines = vec!["YOU TOOK THE LAST STONE. THE SECRET IS YOURS:".to_string()];
        lines.extend(numinous_core::wrap_text(
            &numinous_core::nim_secret().to_uppercase(),
            columns,
        ));
        lines.push(input_legend::nim_result(mode));
        let lh = 10 * ls;
        let ttop = (height as i32 / 2) - (lines.len() as i32 * lh) / 2;
        for (i, line) in lines.iter().enumerate() {
            numinous_core::draw_text(&mut raster, line, 20, ttop + i as i32 * lh, ls, '#');
        }
    } else if play.over == Some(false) {
        raster.dim(25);
        let ls = (width as i32 / 340).clamp(1, 3);
        let columns = ((width as i32 / (6 * ls)) - 6).max(12) as usize;
        let mut lines = vec!["THE ORDER RETURNED THE HEAPS TO XOR 0.".to_string()];
        lines.extend(numinous_core::wrap_text(
            "AT XOR 0, EVERY MOVE OPENS A REPLY. BREAK THAT LOOP ON THE NEXT RUN.",
            columns,
        ));
        lines.push(input_legend::nim_result(mode));
        let lh = 10 * ls;
        let ttop = (height as i32 / 2) - (lines.len() as i32 * lh) / 2;
        for (i, line) in lines.iter().enumerate() {
            numinous_core::draw_text(&mut raster, line, 20, ttop + i as i32 * lh, ls, '#');
        }
    }
    raster
}

/// Draw the Gauntlet: whichever stage is live, with the run's narration.
pub(crate) fn draw_gauntlet(
    rooms: &[Box<dyn Room>],
    run: &GauntletPlay,
    frame: u64,
    mode: InputMode,
    width: usize,
    height: usize,
) -> Raster {
    let scale = game_scale(width);
    if run.stage >= 4 {
        let mut raster = Raster::with_accent(width, height, [230, 210, 120]);
        let result_scale = (width as i32 / 400).clamp(1, 3);
        let lines = gauntlet_result_lines(run, mode, width, result_scale);
        let line_height = 10 * result_scale;
        let margin = (12 * result_scale).min(width.min(height) as i32 / 8).max(4);
        raster.line(margin, margin, width as i32 - margin - 1, margin, '#');
        raster.line(
            margin,
            height as i32 - margin - 1,
            width as i32 - margin - 1,
            height as i32 - margin - 1,
            '#',
        );
        raster.line(margin, margin, margin, height as i32 - margin - 1, '#');
        raster.line(
            width as i32 - margin - 1,
            margin,
            width as i32 - margin - 1,
            height as i32 - margin - 1,
            '#',
        );
        let title = "GAUNTLET COMPLETE";
        let title_scale = (result_scale + 1).min(3);
        let title_width = title.len() as i32 * 6 * title_scale;
        let title_x = ((width as i32 - title_width) / 2).max(margin + 4);
        let title_y = (height as i32 / 7).max(margin + 6);
        numinous_core::draw_text(&mut raster, title, title_x, title_y, title_scale, '#');
        let center_x = width as i32 / 2;
        let ray_y = title_y + 10 * title_scale;
        for offset in [18, 28, 38] {
            let offset = offset * result_scale;
            raster.line(center_x - offset, ray_y, center_x - offset + 8, ray_y, '*');
            raster.line(center_x + offset - 8, ray_y, center_x + offset, ray_y, '*');
        }
        let top = (title_y + 16 * title_scale)
            .max((height as i32 - lines.len() as i32 * line_height) / 2);
        for (i, line) in lines.iter().enumerate() {
            numinous_core::draw_text(
                &mut raster,
                line,
                10,
                top + i as i32 * line_height,
                result_scale,
                '#',
            );
        }
        return raster;
    }

    let message_lines = gauntlet_message_lines(&run.message, width, scale);
    let header_height = gauntlet_header_height(message_lines.len(), scale)
        .min(height.saturating_sub(1) as i32)
        .max(1);
    let content_height = height.saturating_sub(header_height as usize);
    let content = match run.stage {
        0 => draw_munch(&run.munch, frame, mode, width, content_height),
        1 => draw_quiz(rooms, &run.quiz, mode, width, content_height),
        2 => {
            let mut raster = Raster::with_accent(width, content_height, [150, 210, 255]);
            numinous_core::draw_text(
                &mut raster,
                "THE SKY: WHICH CHANNEL IS A MIND?",
                10,
                10,
                scale,
                '#',
            );
            let lh = 14 * scale;
            for (i, channel) in run.scan.channels.iter().enumerate() {
                let line = format!("{}  {:>10}", channel.letter, channel.frequency);
                let y = 30 * scale + i as i32 * lh;
                numinous_core::draw_text(&mut raster, &line, 10, y, scale, '*');
                let trace_x = 10 + (line.chars().count() as i32 * 6 + 3) * scale;
                draw_signal_trace(&mut raster, &channel.trace, trace_x, y, width, scale);
            }
            numinous_core::draw_text(
                &mut raster,
                &input_legend::gauntlet_choice(mode),
                10,
                content_height as i32 - 22 * scale,
                scale,
                '*',
            );
            raster
        }
        3 => {
            let mut raster = Raster::with_accent(width, content_height, [255, 140, 120]);
            numinous_core::draw_text(
                &mut raster,
                "THE BOMB: FOUR DIGITS, FIVE WIRES",
                10,
                10,
                scale,
                '#',
            );
            numinous_core::draw_text(
                &mut raster,
                &format!("CLUE: {}", numinous_core::hint(&run.secret).to_uppercase()),
                10,
                26 * scale,
                scale,
                '*',
            );
            let lh = 12 * scale;
            for (i, line) in run.wire_lines.iter().enumerate() {
                numinous_core::draw_text(
                    &mut raster,
                    line,
                    10,
                    44 * scale + i as i32 * lh,
                    scale,
                    '*',
                );
            }
            numinous_core::draw_text(
                &mut raster,
                &format!("> {}_", run.wire),
                10,
                44 * scale + run.wire_lines.len() as i32 * lh + lh,
                scale + 1,
                '#',
            );
            numinous_core::draw_text(
                &mut raster,
                &input_legend::gauntlet_bomb(mode),
                10,
                content_height as i32 - 22 * scale,
                scale,
                '*',
            );
            raster
        }
        _ => unreachable!("completed Gauntlet returned above"),
    };

    let accent = match run.stage {
        0 => [140, 230, 120],
        1 => rooms
            .iter()
            .find(|room| {
                run.quiz
                    .round
                    .choices
                    .iter()
                    .find(|choice| choice.letter == run.quiz.round.answer)
                    .is_some_and(|choice| choice.id == room.meta().id)
            })
            .map_or([120, 220, 190], |room| room.meta().accent),
        2 => [150, 210, 255],
        3 => [255, 140, 120],
        _ => unreachable!("completed Gauntlet returned above"),
    };
    let mut raster = Raster::with_accent(width, height, accent);
    raster.blit(&content, 0, header_height as usize);
    raster.line(
        0,
        header_height - 1,
        width.saturating_sub(1) as i32,
        header_height - 1,
        '-',
    );
    numinous_core::draw_text(
        &mut raster,
        gauntlet_stage_title(run.stage),
        10,
        5,
        scale,
        '#',
    );
    for (i, line) in message_lines.iter().enumerate() {
        numinous_core::draw_text(
            &mut raster,
            line,
            10,
            5 + (i as i32 + 1) * 9 * scale,
            scale,
            '*',
        );
    }
    raster
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn assert_visible(name: &str, raster: Raster) {
        assert!(
            raster.lit_count() > 100,
            "{name} should draw visible pixels, lit={}",
            raster.lit_count()
        );
        assert_eq!(raster.to_rgba().len(), 320 * 220 * 4);
    }

    fn sample_munch() -> MunchPlay {
        MunchPlay {
            board: numinous_core::build_board(17, 0),
            seed: 17,
            round: 0,
            cursor: 0,
            bites: BTreeSet::new(),
            graded: None,
        }
    }

    fn sample_quiz() -> QuizPlay {
        QuizPlay {
            round: numinous_core::build_round(29, 0, 40, 24),
            flash: None,
        }
    }

    fn sample_gauntlet() -> GauntletPlay {
        GauntletPlay {
            seed: 41,
            stage: 2,
            munch: sample_munch(),
            quiz: sample_quiz(),
            scan: numinous_core::build_scan(41, 4),
            secret: numinous_core::secret_code(41, 4),
            wire: String::new(),
            wire_lines: Vec::new(),
            scores: vec![0, 25],
            cleared: vec![false, true],
            message: "SKY GATE".to_string(),
        }
    }

    #[test]
    fn nim_layout_hits_heap_and_take() {
        let layout = NimLayout::new(800, 480);
        let heaps = [3, 5, 7];
        // Left label of the first heap: take one.
        assert_eq!(
            layout.hit(15.0, f64::from(layout.top + 2), &heaps),
            Some((0, 1))
        );
        // First stone of heap 0: take all remaining from that stone rightward.
        let first_stone_x = 40.0 + f64::from(layout.stone) / 2.0;
        let hit = layout
            .hit(
                first_stone_x,
                f64::from(layout.top + layout.row_h / 2),
                &heaps,
            )
            .expect("stone hit");
        assert_eq!(hit.0, 0);
        assert!(hit.1 >= 1 && hit.1 <= 3, "take {hit:?}");
    }

    #[test]
    fn game_layouts_drive_hit_testing() {
        let quiz = sample_quiz();
        let layout = QuizChoiceLayout::new(320, 220, quiz.round.choices.len());
        assert_eq!(
            layout.hit(f64::from(layout.base), quiz.round.choices.len()),
            Some(0)
        );
        assert_eq!(
            layout.hit(
                f64::from(layout.base + layout.line_height * 2),
                quiz.round.choices.len()
            ),
            Some(2)
        );
        assert_eq!(
            layout.hit(f64::from(layout.base - 1), quiz.round.choices.len()),
            None
        );

        let munch = MunchLayout::new(320, 220);
        assert_eq!(munch.hit(11.0, f64::from(munch.top + 1)), Some(0));
        assert_eq!(
            munch.hit(
                10.0 + f64::from(munch.cell_w * 5) + 1.0,
                f64::from(munch.top + munch.cell_h * 4) + 1.0
            ),
            Some(29)
        );
        assert_eq!(munch.hit(5.0, f64::from(munch.top + 1)), None);
    }

    #[test]
    fn game_drawers_produce_visible_rasters() {
        let rooms = numinous_core::all_rooms_with(0);
        assert_visible(
            "quiz",
            draw_quiz(&rooms, &sample_quiz(), InputMode::KeyboardMouse, 320, 220),
        );
        assert_visible(
            "munch",
            draw_munch(&sample_munch(), 0, InputMode::KeyboardMouse, 320, 220),
        );
        assert_visible(
            "arcade",
            draw_arcade(
                &ArcadePlay {
                    run: numinous_core::munch_arcade::Arcade::new(19),
                    seed: 19,
                    flash: None,
                    over: false,
                },
                InputMode::KeyboardMouse,
                320,
                220,
            ),
        );
        assert_visible(
            "nim",
            draw_nim(
                &NimPlay {
                    heaps: numinous_core::nim_new(23),
                    seed: 23,
                    selected: 0,
                    take: 1,
                    message: "YOUR MOVE".to_string(),
                    over: None,
                },
                InputMode::KeyboardMouse,
                320,
                220,
            ),
        );
        assert_visible(
            "gauntlet",
            draw_gauntlet(
                &rooms,
                &sample_gauntlet(),
                0,
                InputMode::KeyboardMouse,
                320,
                220,
            ),
        );
    }

    #[test]
    fn quiz_wrong_result_keeps_reveal_and_controls_at_supported_sizes() {
        let rooms = numinous_core::all_rooms_with(0);
        let mut quiz = sample_quiz();
        quiz.flash = Some((false, 40));
        let expected_reveal = quiz.round.answer_reveal.to_uppercase();

        for (width, height) in [(360, 240), (900, 700)] {
            let layout =
                QuizResultLayout::new(&quiz, false, InputMode::KeyboardMouse, width, height);
            assert_eq!(layout.reveal_lines.join(" "), expected_reveal);
            assert!(!layout.reveal_lines.is_empty());
            assert_eq!(layout.controls, "ENTER NEXT   ESC LEAVE");

            assert!(
                10 + numinous_core::text_width(&layout.verdict, layout.verdict_scale)
                    <= width as i32,
                "verdict clips at {width}x{height}"
            );
            for line in &layout.reveal_lines {
                assert!(
                    10 + numinous_core::text_width(line, layout.body_scale) <= width as i32,
                    "reveal line clips at {width}x{height}: {line}"
                );
            }
            assert!(
                10 + numinous_core::text_width(&layout.controls, layout.body_scale) <= width as i32,
                "controls clip at {width}x{height}"
            );

            let last_reveal_bottom = layout.reveal_y
                + (layout.reveal_lines.len() as i32 - 1) * layout.line_height
                + FONT_PIXEL_HEIGHT * layout.body_scale;
            assert!(
                last_reveal_bottom <= layout.controls_y - layout.line_height,
                "reveal collides with controls at {width}x{height}"
            );
            assert!(
                layout.controls_y + FONT_PIXEL_HEIGHT * layout.body_scale <= height as i32,
                "controls clip vertically at {width}x{height}"
            );

            let raster = draw_quiz(&rooms, &quiz, InputMode::KeyboardMouse, width, height);
            assert!(raster.lit_count() > 1_000);
            let rgba = raster.to_rgba();
            let lit_in_rows = |from: i32, to: i32| {
                let from = from.max(0) as usize;
                let to = to.max(0).min(height as i32) as usize;
                rgba.chunks_exact(4)
                    .enumerate()
                    .filter(|(index, pixel)| {
                        let y = index / width;
                        (from..to).contains(&y)
                            && (pixel[0] != 10
                                || pixel[1] != 11
                                || pixel[2] != 15
                                || pixel[3] != 255)
                    })
                    .count()
            };
            assert!(
                lit_in_rows(layout.reveal_y, layout.controls_y - layout.line_height) > 100,
                "rendered reveal is absent at {width}x{height}"
            );
            assert!(
                lit_in_rows(
                    layout.controls_y,
                    layout.controls_y + FONT_PIXEL_HEIGHT * layout.body_scale,
                ) > 25,
                "rendered continuation controls are absent at {width}x{height}"
            );
        }
    }

    #[test]
    fn controller_control_copy_fits_supported_game_widths() {
        let quiz = sample_quiz();
        for (width, height) in [(360, 240), (900, 700)] {
            let scale = game_scale(width);
            let fits = |line: &str, text_scale: i32| {
                10 + numinous_core::text_width(line, text_scale) <= width as i32
            };

            let quiz_result =
                QuizResultLayout::new(&quiz, false, InputMode::Controller, width, height);
            assert_eq!(quiz_result.controls, "SOUTH NEXT   EAST LEAVE");
            assert!(fits(&quiz_result.controls, quiz_result.body_scale));

            for line in munch_control_lines(InputMode::Controller, width, scale) {
                assert!(fits(&line, scale), "Munch controls clip: {line}");
            }
            for copy in [
                input_legend::arcade_live(InputMode::Controller),
                input_legend::nim_live(InputMode::Controller, 3),
            ] {
                let lines = control_lines(&copy, width, scale, 10);
                assert!(
                    lines.len() <= 2,
                    "compact footer exceeds its reserve: {copy}"
                );
                for line in lines {
                    assert!(fits(&line, scale), "controls clip: {line}");
                }
            }
            for copy in [
                input_legend::gauntlet_choice(InputMode::Controller),
                input_legend::gauntlet_bomb(InputMode::Controller),
            ] {
                assert!(fits(&copy, scale), "Gauntlet controls clip: {copy}");
            }
        }
    }

    #[test]
    fn controller_game_drawers_keep_fixed_viewport_geometry() {
        fn assert_rows_equal(name: &str, keyboard: &Raster, controller: &Raster, bottom: usize) {
            assert_eq!((keyboard.width(), keyboard.height()), (360, 240));
            assert_eq!((controller.width(), controller.height()), (360, 240));
            let bytes = bottom * 360 * 4;
            assert_eq!(
                &keyboard.to_rgba()[..bytes],
                &controller.to_rgba()[..bytes],
                "{name} content moved outside its reserved control band"
            );
        }

        let rooms = numinous_core::all_rooms_with(0);
        let quiz = sample_quiz();
        let munch = sample_munch();
        let arcade = ArcadePlay {
            run: numinous_core::munch_arcade::Arcade::new(19),
            seed: 19,
            flash: None,
            over: false,
        };
        let nim = NimPlay {
            heaps: numinous_core::nim_new(23),
            seed: 23,
            selected: 0,
            take: 1,
            message: "YOUR MOVE".to_string(),
            over: None,
        };
        let gauntlet = sample_gauntlet();

        let quiz_bottom = QuizChoiceLayout::new(360, 240, quiz.round.choices.len())
            .base
            .saturating_sub(6) as usize;
        assert_rows_equal(
            "Quiz",
            &draw_quiz(&rooms, &quiz, InputMode::KeyboardMouse, 360, 240),
            &draw_quiz(&rooms, &quiz, InputMode::Controller, 360, 240),
            quiz_bottom,
        );
        assert_rows_equal(
            "Munch",
            &draw_munch(&munch, 0, InputMode::KeyboardMouse, 360, 240),
            &draw_munch(&munch, 0, InputMode::Controller, 360, 240),
            216,
        );
        assert_rows_equal(
            "Arcade",
            &draw_arcade(&arcade, InputMode::KeyboardMouse, 360, 240),
            &draw_arcade(&arcade, InputMode::Controller, 360, 240),
            216,
        );
        assert_rows_equal(
            "Nim",
            &draw_nim(&nim, InputMode::KeyboardMouse, 360, 240),
            &draw_nim(&nim, InputMode::Controller, 360, 240),
            202,
        );
        assert_rows_equal(
            "Gauntlet",
            &draw_gauntlet(&rooms, &gauntlet, 0, InputMode::KeyboardMouse, 360, 240),
            &draw_gauntlet(&rooms, &gauntlet, 0, InputMode::Controller, 360, 240),
            210,
        );
    }

    #[test]
    fn gauntlet_stage_screens_draw() {
        let rooms = numinous_core::all_rooms_with(0);
        for stage in 0..=3 {
            let mut run = sample_gauntlet();
            run.stage = stage;
            run.message = "A LONG GAUNTLET MESSAGE THAT MUST STAY ON THE SCREEN".to_string();
            if stage == 3 {
                run.wire = "12".to_string();
                run.wire_lines = vec!["1 LOCKED  0 LOOSE".to_string()];
            }
            assert_visible(
                &format!("gauntlet stage {stage}"),
                draw_gauntlet(&rooms, &run, 0, InputMode::KeyboardMouse, 320, 220),
            );
        }
    }

    #[test]
    fn gauntlet_signal_trace_draws_pulses_without_font_glyphs() {
        let mut raster = Raster::with_accent(160, 60, [150, 210, 255]);
        let before = raster.lit_count();

        draw_signal_trace(&mut raster, "#..##...#", 20, 12, 160, 1);

        assert!(
            raster.lit_count() >= before + 24,
            "four six-pixel pulses must be visible independently of the text font"
        );
    }

    #[test]
    fn gauntlet_done_screen_uses_combo_total() {
        let rooms = numinous_core::all_rooms_with(0);
        let mut run = sample_gauntlet();
        run.stage = 4;
        run.scores = vec![10, 20, 30, 40];
        run.cleared = vec![true, true, false, true];
        assert_eq!(gauntlet_total(&run.scores, &run.cleared), 10 + 40 + 90 + 40);
        assert_visible(
            "gauntlet done",
            draw_gauntlet(&rooms, &run, 0, InputMode::KeyboardMouse, 320, 220),
        );
    }

    #[test]
    fn gauntlet_message_wraps_inside_the_raster() {
        let scale = game_scale(320);
        let lines = gauntlet_message_lines(
            "THE GAUNTLET WAKES AND THE ORDER WRITES A VERY LONG WARNING",
            320,
            scale,
        );
        assert!(lines.len() > 1, "long narration should wrap");
        for line in lines {
            assert!(
                10 + line.len() as i32 * 6 * scale <= 320,
                "message line fits: {line}"
            );
        }
    }

    #[test]
    fn gauntlet_results_fit_small_and_default_widths() {
        let mut run = sample_gauntlet();
        run.stage = 4;
        run.scores = vec![10, 20, 30, 40];
        run.cleared = vec![true, true, false, true];
        for width in [320, 360, 900] {
            let scale = (width as i32 / 400).clamp(1, 3);
            for line in gauntlet_result_lines(&run, InputMode::KeyboardMouse, width, scale) {
                assert!(
                    10 + line.chars().count() as i32 * 6 * scale <= width as i32,
                    "result line clips at {width}: {line}"
                );
            }
        }
        let lines = gauntlet_result_lines(&run, InputMode::KeyboardMouse, 360, 1);
        assert!(lines.iter().any(|line| line.contains("+20 X2 = 40")));
        assert!(lines.iter().any(|line| line.contains("+30 X3 = 90")));
        assert!(lines.join(" ").contains("TOTAL 180 GAUNTLET SEED 41"));
    }

    #[test]
    fn munch_controls_and_worst_case_feedback_fit_small_and_default_widths() {
        let outcome = numinous_core::Munched {
            hits: 0,
            bad_bites: 3,
            left_behind: 27,
            score: 0,
            wrongly_eaten: vec![14, 27, 98],
            missed: (100..130).collect(),
        };
        for width in [320, 900] {
            let control_scale = game_scale(width);
            for line in munch_control_lines(InputMode::KeyboardMouse, width, control_scale) {
                assert!(
                    10 + line.chars().count() as i32 * 6 * control_scale <= width as i32,
                    "control line clips at {width}: {line}"
                );
            }
            let result_scale = (width as i32 / 400).clamp(1, 3);
            let left = width as i32 / 8;
            for line in munch_result_lines(&outcome, InputMode::KeyboardMouse, width, result_scale)
            {
                assert!(
                    left + line.chars().count() as i32 * 6 * result_scale <= width as i32,
                    "result line clips at {width}: {line}"
                );
            }
        }
        let lines = munch_result_lines(&outcome, InputMode::KeyboardMouse, 320, 1);
        assert!(lines.iter().any(|line| line.contains("WRONG: 14, 27, 98")));
        assert!(lines.iter().any(|line| line.contains("WALKED PAST")));
    }

    #[test]
    fn arcade_flash_text_is_geometrically_centered() {
        for (width, scale, message) in [(360, 1, "CAUGHT"), (900, 3, "CLEAR. THEY MULTIPLY.")] {
            let x = centered_text_x(message, width, scale);
            let text_width = message.chars().count() as i32 * 6 * scale;
            assert!((2 * x + text_width - width as i32).abs() <= 1);
        }
    }

    #[test]
    fn nim_loss_keeps_heap_labels_and_teaches_a_retry() {
        let play = NimPlay {
            heaps: vec![0, 0, 0],
            seed: 23,
            selected: 0,
            take: 1,
            message: "THE ORDER TOOK THE LAST STONE".to_string(),
            over: Some(false),
        };
        let raster = draw_nim(&play, InputMode::KeyboardMouse, 360, 240);
        assert!(raster.lit_count() > 1_000);

        let live = NimPlay {
            heaps: vec![3, 4, 5],
            over: None,
            ..play
        };
        let live = draw_nim(&live, InputMode::KeyboardMouse, 360, 240).to_rgba();
        let background = [10, 11, 15, 255];
        let label_band_has_readable_ink = live
            .chunks_exact(4)
            .enumerate()
            .filter(|(index, _)| index % 360 < 35)
            .any(|(_, pixel)| {
                pixel != background && pixel.iter().take(3).copied().max() > Some(80)
            });
        assert!(label_band_has_readable_ink);
    }
}
