//! App-local Studio input, parsing, audio, and drawing helpers.

use numinous_core::{
    Expr, MAX_STUDIO_EDITOR_CHARS, Raster, SoundSpec, StudioCreation, StudioKind, StudioProgram,
    StudioScale, Surface,
};

use crate::input_legend::{self, InputMode};

fn studio_scale(width: usize) -> i32 {
    (width as i32 / 450).clamp(1, 3)
}

/// Both Rust formats round-trip; choose the shorter one so tiny imports do
/// not fill the readout with zeros or lose their significant digits.
fn compact_number(value: f64) -> String {
    let decimal = value.to_string();
    let scientific = format!("{value:e}");
    if scientific.len() < decimal.len() {
        scientific
    } else {
        decimal
    }
}

fn fit_studio_line(text: &str, columns: usize) -> String {
    if text.chars().count() <= columns {
        return text.to_string();
    }
    let kept: String = text.chars().take(columns.saturating_sub(3)).collect();
    format!("{kept}{}", ".".repeat(columns.min(3)))
}

fn studio_footer_lines(
    mode: InputMode,
    copy: input_legend::ControllerCopy,
    columns: usize,
) -> Vec<String> {
    if columns == 0 {
        return Vec::new();
    }
    let text = input_legend::studio_controls_with_controller(mode, copy);
    let wrapped = numinous_core::wrap_text(&text, columns);
    let mut lines: Vec<_> = wrapped
        .iter()
        .take(2)
        .map(|line| fit_studio_line(line, columns))
        .collect();
    if wrapped.len() > lines.len()
        && let Some(last) = lines.last_mut()
    {
        *last = fit_studio_line(&format!("{last} ..."), columns);
    }
    lines
}

const DEFAULT_SOURCE: &str = "sin(a*x) + x/3";

/// Target seconds a recipe holds before Auto checks a presentation-clock edge.
pub(crate) const AUTO_DWELL_SECONDS: f64 = 21.0;
/// Shared visual and audio duration for a curated recipe transition.
pub(crate) const RECIPE_MORPH_SECONDS: f64 = 0.6;
/// Presentation-clock grid for recipe changes after the dwell. These edges
/// belong to gallery phase and are independent of the melody's playhead.
const AUTO_PHASE_SLICES: f64 = 8.0;
const AUTO_PHASE_EDGE: f64 = 0.06;

/// App-local alias of the shared curated bank (core owns the list).
pub(crate) const STUDIO_RECIPES: &[&str] = numinous_core::STUDIO_RECIPES;

/// Short vocabulary for the Studio help overlay (never permanent chrome).
pub(crate) const STUDIO_HELP_LINES: &[&str] = &[
    "FORMULA JAM",
    "TYPE: BUILD A CURVE  (Y = ...)",
    "PAIR: X(T)=...; Y(T)=...",
    "ONE: SIN COS TAN EXP LN ABS SQRT FLOOR",
    "TWO: MOD(V,V) MIN(V,V) MAX(V,V)",
    "F2: RANDOM RECIPE FROM THE BANK",
    "F3: AUTO SET  (~21S)",
    "F4: NAME + SHARE  .NUM + LINK + PNG + MIDI",
    "F5: GALLERY  THE SAVED WALL",
    "F6: CYCLE MUSICAL SCALE",
    "F1: TOGGLE THIS HELP",
    "TAB / ESC: CLOSE STUDIO",
    "UP/DOWN: TUNE A BY 0.25",
    "HOME: RESET A TO 1",
    "EDITING PAUSES AUTO",
];

/// A reopened `.num` creation whose saved window and knob survive editing.
///
/// While this is present the panel draws the saved window. Its parameter starts
/// at the saved value and changes only when the player adjusts it. The complete
/// capsule is kept, not just its numbers: an untouched reopen must re-share
/// with its title, author, and lineage intact, and rebuilding from the
/// window alone would silently strip them. It opens in a paused preview, the
/// hostile-input posture for shared content: the curve is drawn, the voice
/// waits for the player. An edit starts a remix through `fork_of`, so the
/// original identity becomes lineage while its numerical settings remain.
/// Only replacing the creation, such as drawing a recipe, releases them.
#[derive(Debug, Clone)]
struct OpenedCreation {
    creation: StudioCreation,
    paused: bool,
}

/// Why the Studio has nothing to share right now. Two different problems
/// must not wear one banner: telling a player to fix a formula that parses
/// fine points them at the wrong cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShareRefusal {
    /// The typed source does not parse; there is no curve to promise.
    UnparsedFormula,
    /// The formula is fine, but the recorded parent link cannot ride in a
    /// capsule's descends field.
    LineageTooLarge,
}

#[derive(Debug, Clone)]
struct CurveMorph {
    from: Expr,
    elapsed: f64,
}

impl CurveMorph {
    fn progress(&self) -> f64 {
        let linear = (self.elapsed / RECIPE_MORPH_SECONDS).clamp(0.0, 1.0);
        linear * linear * (3.0 - 2.0 * linear)
    }
}

/// The app-local Studio panel state.
#[derive(Debug, Clone)]
pub struct StudioPanel {
    source: String,
    expr: Option<Expr>,
    program: Option<StudioProgram>,
    error: Option<String>,
    scale: StudioScale,
    /// Explicit parameter shared by the picture, melody, and portable creation.
    parameter: f64,
    /// Recipe index for Random (advances each draw).
    recipe_cursor: u64,
    /// Auto set: calm recipe rotation while the player watches.
    auto_active: bool,
    /// Seconds on the current recipe under Auto.
    auto_elapsed: f64,
    /// Previous valid recipe while a curated transition is visible.
    morph: Option<CurveMorph>,
    /// Dismissible help overlay; open by default on first Studio entry.
    show_help: bool,
    /// Saved numerical settings and the original capsule for identity/lineage.
    opened: Option<OpenedCreation>,
    /// The parent link while this Studio session is a fork. Edits keep it,
    /// because edits are the remix; a fresh open or a recipe draw clears it,
    /// because those are a different creation, not a descent.
    fork_of: Option<String>,
}

impl Default for StudioPanel {
    fn default() -> Self {
        Self::new(DEFAULT_SOURCE).expect("default Studio source is within the portable limit")
    }
}

impl StudioPanel {
    /// Build a Studio panel from source text.
    pub fn new(source: &str) -> Result<Self, String> {
        if source.chars().count() > MAX_STUDIO_EDITOR_CHARS {
            return Err(format!(
                "Studio editor text is too long; limit is {MAX_STUDIO_EDITOR_CHARS} characters"
            ));
        }
        let mut panel = Self {
            source: source.to_string(),
            expr: None,
            program: None,
            error: None,
            scale: StudioScale::Continuous,
            parameter: numinous_core::DEFAULT_STUDIO_PARAMETER,
            // Start at 1 so the first Random draw is not the default recipe.
            recipe_cursor: 1,
            auto_active: false,
            auto_elapsed: 0.0,
            morph: None,
            // First contact shows Help once; F1 recalls it after dismiss.
            show_help: true,
            opened: None,
            fork_of: None,
        };
        let _ = panel.reparse();
        Ok(panel)
    }

    /// Reopen a saved creation exactly: its source, window, and knob.
    ///
    /// Opens as a paused preview. The curve draws over the saved window at the
    /// saved `a`; the voice waits for [`Self::confirm_opened`]. Help closes so
    /// the creation is what the player sees.
    pub fn open_creation(&mut self, creation: &StudioCreation) {
        self.pause_auto();
        self.morph = None;
        self.show_help = false;
        // Opening a creation is a fresh start, not a descent; the fork path
        // below sets its own parent after this clears.
        self.fork_of = None;
        self.source = creation.editor_source();
        self.scale = creation.scale();
        // A validated creation always parses. If this seatbelt branch fires,
        // clear the old program so it cannot draw under the new window.
        match creation.program() {
            Ok(program) => {
                self.parameter = creation.a();
                self.expr = Some(program.voice_expression().clone());
                self.program = Some(program);
                self.error = None;
                self.opened = Some(OpenedCreation {
                    creation: creation.clone(),
                    paused: true,
                });
            }
            Err(message) => {
                self.expr = None;
                self.program = None;
                self.error = Some(message);
                self.opened = None;
            }
        }
    }

    /// Whether a reopened creation is waiting in its paused preview.
    ///
    /// The run path reads the pin through the panel's own drawing and through
    /// [`Self::confirm_opened`]; these observers exist for the tests that
    /// prove the pin's lifecycle.
    #[cfg(test)]
    pub(crate) fn opened_paused(&self) -> bool {
        self.opened.as_ref().is_some_and(|opened| opened.paused)
    }

    /// Whether a reopened creation still pins the window and knob.
    #[cfg(test)]
    pub(crate) fn opened_active(&self) -> bool {
        self.opened.is_some()
    }

    /// Test-only: inject a parent link the production fork path cannot make,
    /// so the refusal for an unshareable lineage stays proven even while no
    /// real `to_link` can exceed the descends cap.
    #[cfg(test)]
    pub(crate) fn force_fork_of(&mut self, link: String) {
        self.fork_of = Some(link);
    }

    /// Fork a creation: open it as the player's own, editable and singing,
    /// and remember whose it was.
    ///
    /// No paused preview here: the player browsed the wall and chose the
    /// fork gesture, and fork must be as cheap as play. The descent rides
    /// every share until the Studio moves to a wholly different creation;
    /// edits keep it, because edits are the remix.
    pub fn fork_creation(&mut self, creation: &StudioCreation) -> Option<SoundSpec> {
        self.open_creation(creation);
        self.fork_of = Some(creation.to_link());
        if let Some(opened) = self.opened.as_mut() {
            opened.paused = false;
        }
        self.current_sound()
    }

    /// Confirm the paused preview: the creation starts singing.
    ///
    /// Returns the melody over the saved window at the saved knob, or `None`
    /// when nothing is waiting to be confirmed.
    pub fn confirm_opened(&mut self) -> Option<SoundSpec> {
        let opened = self.opened.as_mut()?;
        if !opened.paused {
            return None;
        }
        opened.paused = false;
        self.current_sound()
    }

    /// Whether Auto is rotating the recipe bank.
    #[must_use]
    pub fn auto_active(&self) -> bool {
        self.auto_active
    }

    /// Whether the help overlay is visible.
    #[must_use]
    pub fn help_visible(&self) -> bool {
        self.show_help
    }

    /// Pause Auto (edits and explicit discovery controls call this).
    pub fn pause_auto(&mut self) {
        self.auto_active = false;
        self.auto_elapsed = 0.0;
    }

    /// Toggle Auto set. Resuming resets the dwell clock on the current recipe.
    pub fn toggle_auto(&mut self) {
        if self.auto_active {
            self.pause_auto();
        } else {
            self.auto_active = true;
            self.auto_elapsed = 0.0;
        }
    }

    /// Toggle the help overlay. Dismissal is remembered until recalled.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Cycle the portable musical scale and return the new voice.
    pub fn cycle_scale(&mut self) -> Option<SoundSpec> {
        self.pause_auto();
        self.morph = None;
        self.begin_remix();
        self.scale = self.scale.next();
        self.current_sound()
    }

    /// Stable scale name for App status and tests.
    #[must_use]
    pub const fn scale_name(&self) -> &'static str {
        self.scale.name()
    }

    /// Move the explicit parameter in quarter steps. An admitted change is
    /// one edit: it pauses Auto, ends a morph, and returns one replacement voice.
    pub fn adjust_parameter(&mut self, steps: i32) -> Option<SoundSpec> {
        self.set_parameter(self.parameter + f64::from(steps) * 0.25)
    }

    /// Return the parameter to the shared Studio default without replacing the
    /// formula, its window, or its lineage.
    pub fn reset_parameter(&mut self) -> Option<SoundSpec> {
        self.set_parameter(numinous_core::DEFAULT_STUDIO_PARAMETER)
    }

    fn set_parameter(&mut self, parameter: f64) -> Option<SoundSpec> {
        if parameter == self.parameter {
            return None;
        }
        // Core's capsule constructor owns numerical bounds. Use the last-good
        // program so an unfinished text edit need not disable this control.
        self.creation_for_parameter(parameter).ok()?;
        self.pause_auto();
        self.morph = None;
        self.begin_remix();
        self.parameter = parameter;
        self.current_sound()
    }

    /// Load the next curated recipe. Returns a melody when the recipe parses.
    /// Does not pause Auto (bank rotation is the Auto path too).
    pub fn load_random_recipe(&mut self) -> Option<SoundSpec> {
        if self.morph.is_some() {
            return None;
        }
        let previous_numbers = self.window_and_knob();
        let previous = if self
            .program
            .as_ref()
            .is_some_and(|program| program.kind() == StudioKind::Graph)
        {
            self.expr.clone()
        } else {
            None
        };
        let len = STUDIO_RECIPES.len() as u64;
        let index = (self.recipe_cursor % len) as usize;
        self.recipe_cursor = self.recipe_cursor.saturating_add(1);
        // A recipe draw replaces the whole creation, so any fork descent
        // ends here: the bank's curve does not descend from the wall's,
        // and a still-pinned reopen drops without becoming a parent.
        self.opened = None;
        self.fork_of = None;
        self.parameter = numinous_core::DEFAULT_STUDIO_PARAMETER;
        self.source = STUDIO_RECIPES[index].to_string();
        self.auto_elapsed = 0.0;
        let spec = self.reparse();
        if spec.is_some()
            && previous_numbers == self.window_and_knob()
            && let Some(from) = previous
            && self.expr.as_ref().is_some_and(|current| current != &from)
        {
            // A different window or parameter would redraw the old formula
            // with the new numbers. Only blend curves from the same domain.
            self.morph = Some(CurveMorph { from, elapsed: 0.0 });
        }
        spec
    }

    /// Advance Auto when dwell and a presentation-clock edge are both ready.
    ///
    /// `phase` is the app gallery phase in [0, 1). After [`AUTO_DWELL_SECONDS`],
    /// the next recipe loads near a 1/8-phase edge. This is independent of the
    /// voice transport. Returns a new melody for the App to crossfade.
    pub fn tick_auto(&mut self, dt: f64, phase: f64) -> Option<SoundSpec> {
        if !(dt.is_finite() && dt > 0.0) {
            return None;
        }
        // App already bounds frame dt; cap runaway values without starving tests.
        let dt = dt.min(AUTO_DWELL_SECONDS);
        if !self.auto_active {
            return None;
        }
        self.auto_elapsed = (self.auto_elapsed + dt).min(AUTO_DWELL_SECONDS * 4.0);
        if self.auto_elapsed < AUTO_DWELL_SECONDS {
            return None;
        }
        let phase = if phase.is_finite() {
            phase.rem_euclid(1.0)
        } else {
            0.0
        };
        let edge = (phase * AUTO_PHASE_SLICES).fract();
        if edge > AUTO_PHASE_EDGE && edge < (1.0 - AUTO_PHASE_EDGE) {
            return None;
        }
        self.load_random_recipe()
    }

    /// Start a remix without changing the saved window or knob. A fork
    /// chosen on the wall keeps the parent it already set. The first edit
    /// also leaves the paused preview, including an invalid draft whose
    /// last-good expression remains available.
    fn begin_remix(&mut self) {
        if let Some(opened) = self.opened.as_mut() {
            opened.paused = false;
            if self.fork_of.is_none() {
                self.fork_of = Some(opened.creation.to_link());
            }
        }
    }

    /// Re-parse the Studio text, keeping the last good curve alive on errors.
    fn reparse(&mut self) -> Option<SoundSpec> {
        self.morph = None;
        match StudioProgram::from_editor(&self.source) {
            Ok(program) => {
                self.expr = Some(program.voice_expression().clone());
                self.program = Some(program);
                self.error = None;
                self.current_sound()
            }
            Err(message) => {
                self.error = Some(message);
                None
            }
        }
    }

    /// Remove one character and reparse. Editing pauses Auto.
    pub fn backspace(&mut self) -> Option<SoundSpec> {
        self.source.pop()?;
        self.pause_auto();
        self.begin_remix();
        self.reparse()
    }

    /// Append ordinary text and reparse. Editing pauses Auto.
    pub fn push_text(&mut self, text: &str) -> Option<SoundSpec> {
        if text.is_empty() || !self.can_append(text) {
            return None;
        }
        self.pause_auto();
        self.begin_remix();
        self.source.push_str(text);
        self.reparse()
    }

    /// Append a literal space. This preserves the current parse state, matching
    /// the old event-loop behavior. Editing pauses Auto. Returns whether the
    /// portable source bound admitted the space.
    pub fn push_space(&mut self) -> bool {
        if self.can_append(" ") {
            self.pause_auto();
            self.morph = None;
            self.begin_remix();
            self.source.push(' ');
            return true;
        }
        false
    }

    /// Render the last-good expression into the same deterministic Studio voice.
    ///
    /// The picture, voice, and portable creation use one window and parameter.
    /// A reopened creation supplies its saved window; a fresh formula uses the
    /// shared defaults. Gallery playback never changes these numbers.
    pub(crate) fn current_sound(&self) -> Option<SoundSpec> {
        let expr = self.expr.as_ref()?;
        let (xmin, xmax, a) = self.window_and_knob();
        Some(numinous_core::to_melody_with_scale(
            expr,
            xmin,
            xmax,
            numinous_core::DEFAULT_MELODY_NOTES,
            a,
            self.scale,
        ))
    }

    /// Restore audio on entry without editing or confirming the creation.
    /// A paused preview submits silence so a room's voice cannot continue
    /// underneath a capsule the player has not chosen to hear.
    pub(crate) fn entry_sound(&self) -> Option<SoundSpec> {
        if self.opened.as_ref().is_some_and(|opened| opened.paused) {
            Some(SoundSpec {
                duration: 0.12,
                notes: Vec::new(),
            })
        } else {
            self.current_sound()
        }
    }

    /// Current UTF-8 byte length, used only to detect an admitted native edit.
    pub(crate) fn source_len(&self) -> usize {
        self.source.len()
    }

    /// One numerical state for the screen, sound, share, and postcard.
    fn window_and_knob(&self) -> (f64, f64, f64) {
        match &self.opened {
            Some(opened) => (
                opened.creation.xmin(),
                opened.creation.xmax(),
                self.parameter,
            ),
            None => (
                numinous_core::DEFAULT_STUDIO_XMIN,
                numinous_core::DEFAULT_STUDIO_XMAX,
                self.parameter,
            ),
        }
    }

    fn creation_for_parameter(&self, a: f64) -> Result<StudioCreation, ShareRefusal> {
        let program = self.program.as_ref().ok_or(ShareRefusal::UnparsedFormula)?;
        let (xmin, xmax, _) = self.window_and_knob();
        let (first, second) = program.sources();
        match second {
            Some(second) => StudioCreation::new_parametric(first, second, xmin, xmax, a),
            None => StudioCreation::new(first, xmin, xmax, a),
        }
        .map(|creation| creation.with_scale(self.scale))
        .map_err(|_| ShareRefusal::UnparsedFormula)
    }

    /// The current Studio state as a shareable creation, or `None` while the
    /// typed source does not parse: an unparsed edit has no curve to promise,
    /// so it is refused rather than shared as whatever last happened to work.
    ///
    /// Numerical settings change only through an explicit edit or recipe draw.
    pub(crate) fn current_creation(&self) -> Result<StudioCreation, ShareRefusal> {
        if self.error.is_some() || self.program.is_none() {
            return Err(ShareRefusal::UnparsedFormula);
        }
        // An untouched reopen shares the very capsule that was opened,
        // identity and all: rebuilding it from the window alone would
        // silently strip the title, author, and lineage the format exists
        // to preserve. A fork deliberately does not take this path, because
        // a fork is a new creation descending from the parent, not the
        // parent wearing its own name.
        if self.fork_of.is_none()
            && let Some(opened) = &self.opened
        {
            return Ok(opened.creation.clone());
        }
        let mut creation = self.creation_for_parameter(self.parameter)?;
        if let Some(parent) = &self.fork_of {
            // A fork shares its descent, and a lineage that cannot ride is
            // its own refusal rather than a claim that the formula broke.
            creation = creation
                .with_descends(parent)
                .map_err(|_| ShareRefusal::LineageTooLarge)?;
            if let Ok(parent_creation) = StudioCreation::from_link(parent) {
                creation = match parent_creation.fork_credit_suggestion() {
                    Some(credit) => creation.clone().with_credit(&credit).unwrap_or(creation),
                    None => creation,
                };
            }
        }
        Ok(creation)
    }

    /// Render the current curve as a square postcard frame: title, formula,
    /// and the curve over the same window and knob a share would save.
    ///
    /// No footer, help, or cursor: a postcard is the creation, not the
    /// editing session around it. The postcard is the object built to
    /// escape the app, so when the creation carries a title it is the
    /// headline with the formula beneath it smaller, and an author signs
    /// the corner; identity rides the pixels, not only the capsule.
    pub(crate) fn postcard_rgba(
        &self,
        size: usize,
        era: numinous_core::Era,
        title: Option<&str>,
        author: Option<&str>,
    ) -> Vec<u8> {
        let mut raster = Raster::new(size, size);
        let scale = studio_scale(size).max(2);
        let typed = match self.program.as_ref().map(StudioProgram::kind) {
            Some(StudioKind::Parametric) => self.source.to_uppercase(),
            _ => format!("Y = {}", self.source.to_uppercase()),
        };
        if let Some(title) = title {
            numinous_core::draw_text(&mut raster, &title.to_uppercase(), 10, 10, scale + 1, '#');
            numinous_core::draw_text(&mut raster, &typed, 10, 10 + 12 * (scale + 1), scale, '#');
        } else {
            numinous_core::draw_text(&mut raster, "NUMINOUS STUDIO", 10, 10, scale, '#');
            numinous_core::draw_text(&mut raster, &typed, 10, 10 + 12 * scale, scale + 1, '#');
        }
        if let Some(author) = author {
            let credit = format!("BY {}", author.to_uppercase());
            let credit_y = (size as i32 - 12 * scale).max(0);
            numinous_core::draw_text(&mut raster, &credit, 10, credit_y, scale, '#');
        }
        if let Some(program) = &self.program {
            let (xmin, xmax, a) = self.window_and_knob();
            // The postcard must match what creation.num reopens, so it
            // evaluates the settled expression directly rather than through
            // curve_value, whose recipe-morph blend is a 600 ms presentation
            // effect the capsule does not record. A share taken mid-morph
            // stays self-consistent instead of shipping a picture no reopen
            // can reproduce.
            let layout = numinous_app::studio_render::CurveLayout {
                width: size,
                height: size,
                top: f64::from(60 * scale),
                bottom_margin: f64::from(24 * scale),
            };
            match program.kind() {
                StudioKind::Graph => {
                    let expr = program.voice_expression();
                    let _ = numinous_app::studio_render::draw_curve(
                        &mut raster,
                        layout,
                        xmin,
                        xmax,
                        |x| {
                            let value = numinous_core::eval(expr, x, a);
                            value.is_finite().then_some(value)
                        },
                    );
                }
                StudioKind::Parametric => {
                    let _ = numinous_app::studio_render::draw_parametric(
                        &mut raster,
                        layout,
                        xmin,
                        xmax,
                        |input| program.point(input, a),
                    );
                }
            }
        }
        let mut rgba = raster.to_rgba();
        era.apply(&mut rgba, size, size);
        rgba
    }

    pub(crate) fn advance_morph(&mut self, dt: f64) {
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let Some(morph) = self.morph.as_mut() else {
            return;
        };
        morph.elapsed = (morph.elapsed + dt).min(RECIPE_MORPH_SECONDS);
        if morph.elapsed >= RECIPE_MORPH_SECONDS {
            self.morph = None;
        }
    }

    fn curve_value(&self, x: f64, a: f64) -> Option<f64> {
        let current = self
            .expr
            .as_ref()
            .map(|expr| numinous_core::eval(expr, x, a))
            .filter(|value| value.is_finite());
        let Some(morph) = &self.morph else {
            return current;
        };
        let previous =
            Some(numinous_core::eval(&morph.from, x, a)).filter(|value| value.is_finite());
        match (previous, current) {
            (Some(from), Some(to)) => Some(from + (to - from) * morph.progress()),
            (Some(from), None) => Some(from),
            (None, current) => current,
        }
    }

    fn can_append(&self, text: &str) -> bool {
        let current = self.source.chars().count();
        let Some(remaining) = MAX_STUDIO_EDITOR_CHARS.checked_sub(current) else {
            return false;
        };
        text.chars().take(remaining + 1).count() <= remaining
    }

    fn status_lines(&self, mode: InputMode, columns: usize) -> [(String, char); 2] {
        let parameter = compact_number(self.parameter);
        let paused = self.opened.as_ref().is_some_and(|opened| opened.paused);
        let primary = if paused {
            format!("A {parameter}  ENTER: PLAY")
        } else {
            format!("A {parameter}  SCALE {}", self.scale_name().to_uppercase())
        };
        let context = if let Some(error) = &self.error {
            format!("DRAFT: {}", error.to_uppercase())
        } else if self.opened.is_some() {
            let (xmin, xmax, _) = self.window_and_knob();
            let domain = if self
                .program
                .as_ref()
                .is_some_and(|program| program.kind() == StudioKind::Parametric)
            {
                "T"
            } else {
                "X"
            };
            let state = if self.fork_of.is_some() {
                "REMIX"
            } else {
                "REOPENED"
            };
            let window = format!(
                "{state}  {domain} {} TO {}",
                compact_number(xmin),
                compact_number(xmax)
            );
            if paused {
                format!("SCALE {}  {window}", self.scale_name().to_uppercase())
            } else {
                window
            }
        } else {
            match mode {
                InputMode::KeyboardMouse => "UP/DOWN: A +/-0.25  HOME: A=1  F6: SCALE".to_string(),
                InputMode::Controller => "KEYBOARD F1: HELP  F6: SCALE".to_string(),
            }
        };
        [
            (fit_studio_line(&primary, columns), '*'),
            (
                fit_studio_line(&context, columns),
                if self.error.is_some() { '-' } else { '*' },
            ),
        ]
    }

    /// Draw the Studio panel into the raster.
    #[cfg(test)]
    pub(crate) fn draw(&self, raster: &mut Raster, mode: InputMode, width: usize, height: usize) {
        self.draw_with_controller(
            raster,
            mode,
            crate::input_legend::ControllerFace::Generic.into(),
            width,
            height,
        );
    }

    pub(crate) fn draw_with_controller(
        &self,
        raster: &mut Raster,
        mode: InputMode,
        copy: crate::input_legend::ControllerCopy,
        width: usize,
        height: usize,
    ) {
        let width = width.min(raster.width());
        let height = height.min(raster.height());
        let scale = studio_scale(width);
        let columns = width.saturating_sub(20) / (6 * scale as usize);
        let footer = studio_footer_lines(mode, copy, columns);
        let footer_height = (16 + 10 * footer.len().saturating_sub(1) as i32) * scale;
        let title = if self.auto_active() {
            "THE STUDIO  AUTO"
        } else {
            "THE STUDIO"
        };
        numinous_core::draw_text(raster, title, 10, 10, scale, '#');
        let typed = if self
            .program
            .as_ref()
            .is_some_and(|program| program.kind() == StudioKind::Parametric)
            || self
                .source
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("x(t)")
        {
            format!("{}_", self.source.to_uppercase())
        } else {
            format!("Y = {}_", self.source.to_uppercase())
        };
        numinous_core::draw_text(raster, &typed, 10, 10 + 12 * scale, scale + 1, '#');
        for (index, (line, mark)) in self.status_lines(mode, columns).iter().enumerate() {
            numinous_core::draw_text(
                raster,
                line,
                10,
                10 + (34 + index as i32 * 10) * scale,
                scale,
                *mark,
            );
        }

        if let Some(program) = &self.program {
            let (xmin, xmax, a) = self.window_and_knob();
            let layout = numinous_app::studio_render::CurveLayout {
                width,
                height,
                top: f64::from(10 + 56 * scale),
                bottom_margin: f64::from(footer_height + 8 * scale),
            };
            match program.kind() {
                StudioKind::Graph => {
                    let _ =
                        numinous_app::studio_render::draw_curve(raster, layout, xmin, xmax, |x| {
                            self.curve_value(x, a)
                        });
                }
                StudioKind::Parametric => {
                    let _ = numinous_app::studio_render::draw_parametric(
                        raster,
                        layout,
                        xmin,
                        xmax,
                        |input| program.point(input, a),
                    );
                }
            }
        }

        if self.help_visible() && height > 40 {
            let help_top = 10 + 58 * scale;
            raster.clear_rows(
                help_top - 4,
                help_top + STUDIO_HELP_LINES.len() as i32 * 10 * scale + 4,
            );
            for (i, line) in STUDIO_HELP_LINES.iter().enumerate() {
                numinous_core::draw_text(
                    raster,
                    line,
                    10,
                    help_top + i as i32 * 10 * scale,
                    scale,
                    '*',
                );
            }
        }
        if height >= 20 {
            raster.clear_rows(height as i32 - footer_height, height as i32);
            for (index, line) in footer.iter().enumerate() {
                let rows_below = footer.len() - index - 1;
                numinous_core::draw_text(
                    raster,
                    line,
                    10,
                    height as i32 - (11 + 10 * rows_below as i32) * scale,
                    scale,
                    '#',
                );
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn source_for_test(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AUTO_DWELL_SECONDS, MAX_STUDIO_EDITOR_CHARS, RECIPE_MORPH_SECONDS, STUDIO_HELP_LINES,
        STUDIO_RECIPES, StudioPanel, compact_number, fit_studio_line, studio_footer_lines,
        studio_scale,
    };
    use crate::input_legend::{
        self, ControllerAction, ControllerButton, ControllerCopy, ControllerFace, InputMode,
    };
    use numinous_core::{Raster, Surface};

    #[test]
    fn default_panel_has_a_curve_and_a_voice() {
        let mut panel = StudioPanel::default();
        let spec = panel.reparse().expect("melody");
        assert_eq!(panel.source, "sin(a*x) + x/3");
        assert!(panel.expr.is_some());
        assert!(panel.error.is_none());
        assert_eq!(spec.notes.len(), 32);
    }

    #[test]
    fn bad_edits_keep_the_last_good_curve_alive() {
        let mut panel = StudioPanel::new("x").expect("panel");
        assert!(panel.push_text("@").is_none());
        assert!(panel.error.is_some());
        assert!(panel.expr.is_some());
        let mut raster = Raster::new(120, 90);
        panel.draw(&mut raster, InputMode::KeyboardMouse, 120, 90);
        assert!(raster.lit_count() > 0, "last good curve should still draw");
    }

    #[test]
    fn draw_handles_tiny_and_mismatched_sizes() {
        let mut panel = StudioPanel::new("sin(x)").expect("panel");
        let mut zero = Raster::new(0, 0);
        panel.draw(&mut zero, InputMode::KeyboardMouse, 0, 0);
        assert_eq!(zero.lit_count(), 0);

        let mut one = Raster::new(1, 1);
        panel.draw(&mut one, InputMode::KeyboardMouse, 1, 1);

        let mut short = Raster::new(80, 20);
        panel.draw(&mut short, InputMode::KeyboardMouse, 500, 20);
        assert!(short.lit_count() > 0);

        let mut mismatched = Raster::new(24, 90);
        panel.draw(&mut mismatched, InputMode::KeyboardMouse, 200, 90);
        assert!(mismatched.lit_count() > 0);

        panel.toggle_auto();
        let mut auto = Raster::new(120, 90);
        panel.draw(&mut auto, InputMode::KeyboardMouse, 120, 90);
        assert!(auto.lit_count() > 0);

        panel.expr = None;
        let mut no_expression = Raster::new(120, 90);
        panel.draw(&mut no_expression, InputMode::KeyboardMouse, 120, 90);

        let non_finite = StudioPanel::new("1/0").expect("parseable non-finite expression");
        let mut no_samples = Raster::new(120, 90);
        non_finite.draw(&mut no_samples, InputMode::KeyboardMouse, 120, 90);
    }

    #[test]
    fn editing_operations_update_source_predictably() {
        let mut panel = StudioPanel::new("x").expect("panel");
        assert_eq!(panel.source_len(), 1);
        assert!(panel.push_space());
        assert_eq!(panel.source, "x ");
        assert!(panel.push_text("+ 1").is_some());
        assert_eq!(panel.source, "x + 1");
        assert!(panel.backspace().is_none());
        assert_eq!(panel.source, "x + ");
        assert!(panel.error.is_some());
        assert!(
            panel.current_sound().is_some(),
            "an invalid edit must retain a playable last-good expression"
        );
    }

    #[test]
    fn every_recipe_parses_and_random_cycles_the_bank() {
        for recipe in STUDIO_RECIPES {
            let panel = StudioPanel::new(recipe).expect("recipe must be portable");
            assert!(
                panel.error.is_none(),
                "recipe {recipe:?} must parse cleanly"
            );
            assert!(panel.expr.is_some(), "recipe {recipe:?} must yield an expr");
        }
        let mut panel = StudioPanel::default();
        let first = panel.source.clone();
        assert!(panel.load_random_recipe().is_some());
        assert_ne!(panel.source, first);
        panel.advance_morph(RECIPE_MORPH_SECONDS);
        // Cursor starts at 1 (second recipe); after remaining bank draws, wrap.
        for _ in 1..STUDIO_RECIPES.len() {
            assert!(panel.load_random_recipe().is_some());
            panel.advance_morph(RECIPE_MORPH_SECONDS);
        }
        assert_eq!(panel.source, STUDIO_RECIPES[0]);
    }

    #[test]
    fn help_names_the_complete_scalar_vocabulary() {
        let help = STUDIO_HELP_LINES.join("\n");
        for name in ["FLOOR", "MOD(V,V)", "MIN(V,V)", "MAX(V,V)"] {
            assert!(help.contains(name), "help must name {name}");
        }
        assert!(
            help.contains("MIDI"),
            "F4 help must name the MIDI file the share writes"
        );
    }

    #[test]
    fn auto_waits_for_dwell_and_presentation_clock_edge_then_advances() {
        let mut panel = StudioPanel::default();
        let start = panel.source.clone();
        assert!(panel.tick_auto(1.0, 0.3).is_none(), "Auto is inactive");
        panel.toggle_auto();
        assert!(panel.auto_active());
        assert!(
            panel.tick_auto(f64::NAN, 0.3).is_none(),
            "bad time is inert"
        );
        // Phase 0.3 sits between 1/8 edges (0.3 * 8 = 2.4).
        assert!(panel.tick_auto(1.0, 0.3).is_none(), "dwell not met");
        assert_eq!(panel.source, start);
        // Dwell complete but between presentation-clock edges: still wait.
        assert!(
            panel.tick_auto(AUTO_DWELL_SECONDS, 0.3).is_none(),
            "a presentation-clock edge is still required"
        );
        assert_eq!(panel.source, start);
        // Near a presentation-clock edge after dwell: advance.
        let advanced = panel.tick_auto(0.1, f64::NAN);
        assert!(advanced.is_some(), "clock edge after dwell advances");
        assert_ne!(panel.source, start);
        assert!(panel.morph.is_some(), "Auto begins one recipe morph");
    }

    #[test]
    fn recipe_morph_interpolates_exact_endpoints_and_finishes_on_time() {
        let mut panel = StudioPanel::default();
        let (x, a) = (0.73, 0.41);
        let old = numinous_core::eval(panel.expr.as_ref().expect("opening expression"), x, a);

        assert!(panel.load_random_recipe().is_some());
        let new = numinous_core::eval(panel.expr.as_ref().expect("new expression"), x, a);
        assert!((old - new).abs() > 1.0e-3, "fixture must expose the morph");
        assert!((panel.curve_value(x, a).expect("morph start") - old).abs() < 1.0e-12);

        panel.advance_morph(RECIPE_MORPH_SECONDS / 2.0);
        let halfway = panel.curve_value(x, a).expect("halfway morph");
        assert!((halfway - (old + new) / 2.0).abs() < 1.0e-12);

        panel.advance_morph(RECIPE_MORPH_SECONDS / 2.0);
        assert!(panel.morph.is_none());
        assert!((panel.curve_value(x, a).expect("morph end") - new).abs() < 1.0e-12);

        let invalid = numinous_core::parse("1/0").expect("parseable non-finite expression");
        let valid = panel.expr.clone().expect("valid target expression");
        panel.morph = Some(super::CurveMorph {
            from: valid.clone(),
            elapsed: RECIPE_MORPH_SECONDS / 2.0,
        });
        panel.expr = Some(invalid.clone());
        assert!((panel.curve_value(x, a).expect("finite previous") - new).abs() < 1.0e-12);
        panel.morph = Some(super::CurveMorph {
            from: invalid,
            elapsed: RECIPE_MORPH_SECONDS / 2.0,
        });
        panel.expr = Some(valid);
        assert!((panel.curve_value(x, a).expect("finite current") - new).abs() < 1.0e-12);
    }

    #[test]
    fn manual_edit_cancels_recipe_morph_and_invalid_time_cannot_advance_it() {
        let mut panel = StudioPanel::default();
        assert!(panel.load_random_recipe().is_some());
        assert!(panel.morph.is_some());

        panel.advance_morph(f64::NAN);
        assert_eq!(panel.morph.as_ref().expect("morph remains").elapsed, 0.0);

        assert!(panel.push_text("(").is_none());
        assert!(panel.morph.is_none());
        assert!(panel.current_sound().is_some());
    }

    #[test]
    fn presentation_time_advances_a_morph_without_advancing_auto() {
        let mut panel = StudioPanel::default();
        assert!(panel.load_random_recipe().is_some());
        let cursor = panel.recipe_cursor;

        panel.advance_morph(RECIPE_MORPH_SECONDS);

        assert!(panel.morph.is_none());
        panel.advance_morph(0.1);
        assert_eq!(panel.recipe_cursor, cursor);
        assert_eq!(panel.auto_elapsed, 0.0);
    }

    #[test]
    fn repeated_recipe_request_cannot_jump_an_active_morph() {
        let mut panel = StudioPanel::default();
        assert!(panel.load_random_recipe().is_some());
        let source = panel.source.clone();
        let cursor = panel.recipe_cursor;

        assert!(panel.load_random_recipe().is_none());
        assert_eq!(panel.source, source);
        assert_eq!(panel.recipe_cursor, cursor);
        assert_eq!(panel.morph.as_ref().expect("morph remains").elapsed, 0.0);
    }

    #[test]
    fn editing_pauses_auto_and_help_toggles() {
        let mut panel = StudioPanel::default();
        assert!(panel.help_visible(), "first contact shows help");
        panel.toggle_help();
        assert!(!panel.help_visible());
        panel.toggle_help();
        assert!(panel.help_visible());

        panel.toggle_auto();
        assert!(panel.auto_active());
        panel.toggle_auto();
        assert!(!panel.auto_active());
        panel.toggle_auto();
        assert!(panel.auto_active());
        let _ = panel.push_text("+0");
        assert!(
            !panel.auto_active(),
            "typing must pause Auto so the player owns the formula"
        );
        panel.toggle_auto();
        assert!(panel.auto_active());
        let _ = panel.backspace();
        assert!(!panel.auto_active());
    }

    #[test]
    fn a_reopened_creation_pins_window_and_knob_and_waits_paused() {
        use std::f64::consts::TAU;
        let creation =
            numinous_core::StudioCreation::new("sin(a*x)", 0.0, 1.0, 0.25).expect("creation");
        let mut panel = StudioPanel::default();
        panel.toggle_auto();
        assert!(panel.auto_active());

        panel.open_creation(&creation);
        assert!(!panel.auto_active(), "a reopen pauses Auto");
        assert!(
            !panel.help_visible(),
            "the creation is what the player sees"
        );
        assert!(panel.opened_active());
        assert!(panel.opened_paused());
        assert_eq!(panel.source_for_test(), "sin(a*x)");

        // Exact: the voice is the saved window at the saved knob, not the
        // ambient default of either.
        let expr = numinous_core::parse("sin(a*x)").expect("expr");
        let exact = numinous_core::to_melody(&expr, 0.0, 1.0, 32, 0.25);
        assert_ne!(
            exact,
            numinous_core::to_melody(&expr, -TAU, TAU, 32, 1.0),
            "fixture must expose the pin"
        );
        assert_eq!(panel.current_sound().expect("voice"), exact);

        // Confirm starts the singing once; a second confirm has nothing left.
        assert_eq!(panel.confirm_opened().expect("confirmed melody"), exact);
        assert!(!panel.opened_paused());
        assert!(panel.opened_active(), "confirming does not release the pin");
        assert!(panel.confirm_opened().is_none());
    }

    #[test]
    fn an_edit_preserves_the_reopened_window_and_knob() {
        let creation =
            numinous_core::StudioCreation::new("sin(a*x)", 0.0, 1.0, 0.25).expect("creation");
        let mut panel = StudioPanel::default();
        panel.open_creation(&creation);
        assert!(panel.confirm_opened().is_some());
        let curve_band = |panel: &StudioPanel| {
            let mut raster = Raster::new(200, 150);
            panel.draw(&mut raster, InputMode::KeyboardMouse, 200, 150);
            raster.to_rgba()[200 * 4 * 70..200 * 4 * 120].to_vec()
        };
        let before = curve_band(&panel);

        let spec = panel.push_text("+0").expect("still parses");
        assert!(panel.opened_active());
        assert_eq!(spec, creation.to_melody(32));
        assert_eq!(curve_band(&panel), before);
        let edited = panel.current_creation().expect("edited creation");
        assert_eq!(edited.source(), "sin(a*x)+0");
        assert_eq!((edited.xmin(), edited.xmax(), edited.a()), (0.0, 1.0, 0.25));
        assert_eq!(edited.descends(), Some(creation.to_link().as_str()));
        assert_eq!(Some(edited.to_melody(32)), panel.current_sound());
        let mut reopened = StudioPanel::default();
        reopened.open_creation(&edited);
        assert_eq!(
            panel.postcard_rgba(200, numinous_core::Era::Modern, None, None),
            reopened.postcard_rgba(200, numinous_core::Era::Modern, None, None),
            "the edited postcard must match the creation it saves"
        );
    }

    #[test]
    fn whitespace_keeps_reopened_settings_while_starting_a_remix() {
        let creation =
            numinous_core::StudioCreation::new("sin(a*x)", 0.0, 1.0, 0.25).expect("creation");
        let mut panel = StudioPanel::default();
        panel.open_creation(&creation);
        assert!(panel.opened_paused());
        assert!(panel.push_space());
        assert!(panel.opened_active());
        assert!(!panel.opened_paused());
        let edited = panel.current_creation().expect("whitespace edit");
        assert_eq!((edited.xmin(), edited.xmax(), edited.a()), (0.0, 1.0, 0.25));
        assert_eq!(edited.descends(), Some(creation.to_link().as_str()));
        assert_eq!(panel.current_sound(), Some(creation.to_melody(32)));

        // Recipe discovery intentionally replaces the complete creation.
        assert!(panel.load_random_recipe().is_some());
        assert!(!panel.opened_active());
        let recipe = panel.current_creation().expect("recipe");
        assert_eq!(recipe.xmin(), numinous_core::DEFAULT_STUDIO_XMIN);
        assert_eq!(recipe.xmax(), numinous_core::DEFAULT_STUDIO_XMAX);
        assert_eq!(recipe.a(), numinous_core::DEFAULT_STUDIO_PARAMETER);
        assert_eq!(recipe.descends(), None);
    }

    #[test]
    fn invalid_drafts_keep_the_last_good_capsule_math() {
        let creation =
            numinous_core::StudioCreation::new("sin(a*x)", 0.0, 1.0, 0.25).expect("creation");
        let mut panel = StudioPanel::default();
        panel.open_creation(&creation);
        assert!(panel.push_text("+").is_none());
        assert!(!panel.opened_paused());
        assert_eq!(panel.window_and_knob(), (0.0, 1.0, 0.25));
        assert_eq!(panel.current_sound(), Some(creation.to_melody(32)));
        assert_eq!(
            panel.current_creation(),
            Err(super::ShareRefusal::UnparsedFormula)
        );

        let scaled = creation.clone().with_scale(creation.scale().next());
        assert_eq!(panel.cycle_scale(), Some(scaled.to_melody(32)));
        assert_eq!(
            panel.current_creation(),
            Err(super::ShareRefusal::UnparsedFormula),
            "changing scale does not repair an invalid formula"
        );
        assert_eq!(panel.backspace(), Some(scaled.to_melody(32)));
        let repaired = panel.current_creation().expect("repaired formula");
        assert_eq!(
            (repaired.xmin(), repaired.xmax(), repaired.a()),
            (0.0, 1.0, 0.25)
        );
        assert_eq!(repaired.scale(), scaled.scale());
        assert_eq!(repaired.descends(), Some(creation.to_link().as_str()));
    }

    #[test]
    fn changing_a_reopened_scale_preserves_its_geometry() {
        let creation =
            numinous_core::StudioCreation::new("sin(a*x)", 0.0, 1.0, 0.25).expect("creation");
        let mut panel = StudioPanel::default();
        panel.open_creation(&creation);
        let before = panel.postcard_rgba(200, numinous_core::Era::Modern, None, None);

        let expected = creation.clone().with_scale(creation.scale().next());
        let sound = panel.cycle_scale().expect("scaled voice");
        assert_eq!(sound, expected.to_melody(32));
        assert_ne!(sound, creation.to_melody(32));
        assert!(!panel.opened_paused());
        let edited = panel.current_creation().expect("scale edit");
        assert_eq!(edited.source(), creation.source());
        assert_eq!((edited.xmin(), edited.xmax(), edited.a()), (0.0, 1.0, 0.25));
        assert_eq!(edited.scale(), expected.scale());
        assert_eq!(edited.descends(), Some(creation.to_link().as_str()));
        assert_eq!(
            panel.postcard_rgba(200, numinous_core::Era::Modern, None, None),
            before
        );
    }

    #[test]
    fn the_reopened_window_is_the_window_the_curve_draws() {
        // Same source, two saved windows: the curve band must differ, or the
        // reopen would only be echoing the numbers while drawing the ambient
        // window anyway.
        let narrow = numinous_core::StudioCreation::new("sin(x)", 0.0, 1.0, 1.0).expect("narrow");
        let wide = numinous_core::StudioCreation::new("sin(x)", -6.0, 6.0, 1.0).expect("wide");
        let curve_band = |creation: &numinous_core::StudioCreation| {
            let mut panel = StudioPanel::default();
            panel.open_creation(creation);
            let mut raster = Raster::new(200, 150);
            panel.draw(&mut raster, InputMode::KeyboardMouse, 200, 150);
            // Rows inside the curve area only, clear of chrome and footer,
            // so the difference cannot come from the reopened status line.
            raster.to_rgba()[200 * 4 * 70..200 * 4 * 120].to_vec()
        };
        assert_ne!(curve_band(&narrow), curve_band(&wide));
    }

    #[test]
    fn the_shared_creation_is_the_curve_on_screen() {
        use std::f64::consts::TAU;
        // Fresh Studio uses the shared numerical defaults for every output.
        let panel = StudioPanel::new("sin(a*x)").expect("panel");
        let creation = panel.current_creation().expect("creation");
        assert_eq!(creation.source(), "sin(a*x)");
        assert!((creation.xmin() + TAU).abs() < 1e-12);
        assert!((creation.xmax() - TAU).abs() < 1e-12);
        assert_eq!(creation.a(), numinous_core::DEFAULT_STUDIO_PARAMETER);
        assert_eq!(panel.current_sound(), Some(creation.to_melody(32)));

        // A reopened pin shares its own saved window and knob.
        let saved = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5).expect("saved");
        let mut reopened = StudioPanel::default();
        reopened.open_creation(&saved);
        let shared = reopened.current_creation().expect("shared");
        assert_eq!(shared, saved);

        // An unparsed edit has no curve to promise.
        let mut broken = StudioPanel::new("sin(a*x)").expect("panel");
        assert!(broken.push_text("(").is_none());
        assert_eq!(
            broken.current_creation(),
            Err(super::ShareRefusal::UnparsedFormula)
        );
    }

    #[test]
    fn explicit_parameter_matches_the_saved_picture_and_voice() {
        for source in ["sin(a*x)+x/3", "x(t)=cos(t); y(t)=sin(a*t)+t/3"] {
            let mut panel = StudioPanel::new(source).expect("panel");
            let original = panel.current_sound().expect("voice");
            let mut voices = Vec::new();
            for a in [0.0, 0.25, 1.25, -0.5] {
                let voice = panel.set_parameter(a).expect("changed parameter");
                let creation = panel.current_creation().expect("creation");
                assert_eq!(creation.a(), a);
                assert_eq!(creation.xmin(), numinous_core::DEFAULT_STUDIO_XMIN);
                assert_eq!(creation.xmax(), numinous_core::DEFAULT_STUDIO_XMAX);
                assert_eq!(voice, creation.to_melody(32));
                assert_eq!(voice.midi(), creation.to_melody(32).midi());
                let mut reopened = StudioPanel::default();
                reopened.open_creation(&creation);
                assert_eq!(
                    panel.postcard_rgba(240, numinous_core::Era::Modern, None, None),
                    reopened.postcard_rgba(240, numinous_core::Era::Modern, None, None),
                );
                voices.push(voice);
            }
            assert!(voices.iter().all(|voice| voice != &original));
            assert!(voices.windows(2).all(|pair| pair[0] != pair[1]));
        }
    }

    #[test]
    fn parameter_edits_retain_exact_imports_and_refuse_out_of_bounds_noops() {
        for a in [1e12, -1e12, 1.0, 0.123_456_789_012_345_66] {
            let saved = numinous_core::StudioCreation::new("sin(a*x)+x/3", -2.0, 3.0, a)
                .expect("saved")
                .with_title("A kept experiment")
                .expect("title");
            let mut panel = StudioPanel::default();
            panel.open_creation(&saved);
            panel.toggle_auto();
            assert_eq!(panel.window_and_knob(), (-2.0, 3.0, a));
            for invalid in [
                f64::NAN,
                f64::INFINITY,
                -f64::INFINITY,
                1e12 + 0.25,
                -1e12 - 0.25,
            ] {
                assert!(panel.set_parameter(invalid).is_none());
            }
            assert!(panel.adjust_parameter(0).is_none());
            if a == 1.0 {
                assert!(panel.reset_parameter().is_none());
            } else if a.abs() == 1e12 {
                assert!(
                    panel
                        .adjust_parameter(if a > 0.0 { 1 } else { -1 })
                        .is_none()
                );
            }
            assert!(panel.opened_paused());
            assert!(panel.auto_active());
            assert_eq!(panel.current_creation().expect("unchanged"), saved);

            let steps = if a > 0.0 { -1 } else { 1 };
            let voice = panel.adjust_parameter(steps).expect("one admitted step");
            let edited = panel.current_creation().expect("remix");
            assert_eq!(edited.a(), a + f64::from(steps) * 0.25);
            assert_eq!((edited.xmin(), edited.xmax()), (-2.0, 3.0));
            assert_eq!(edited.descends(), Some(saved.to_link().as_str()));
            assert_eq!(edited.title(), None);
            assert!(!panel.opened_paused());
            assert!(!panel.auto_active());
            assert_eq!(voice, edited.to_melody(32));
        }
    }

    #[test]
    fn parameter_edits_work_through_invalid_drafts_and_cancel_recipe_morphs() {
        let mut panel = StudioPanel::default();
        assert!(panel.load_random_recipe().is_some());
        assert!(panel.morph.is_some());
        assert!(panel.adjust_parameter(1).is_some());
        assert!(panel.morph.is_none());
        let valid_source = panel.source.clone();
        assert!(panel.push_text("+").is_none());
        let voice = panel.adjust_parameter(-2).expect("last-good voice");
        assert!(panel.error.is_some());
        assert_eq!(panel.parameter, 0.75);
        assert!(panel.current_creation().is_err());
        assert_eq!(panel.backspace(), Some(voice));
        assert_eq!(panel.source, valid_source);
        assert_eq!(panel.current_creation().expect("repaired").a(), 0.75);
        let scaled = panel.cycle_scale().expect("scale");
        assert_eq!(
            scaled,
            panel.current_creation().expect("scaled").to_melody(32)
        );
        assert_eq!(panel.current_creation().expect("scaled").a(), 0.75);
    }

    #[test]
    fn recipe_changes_blend_only_when_the_old_numbers_still_apply() {
        for (xmin, xmax, a, should_blend) in [
            (
                numinous_core::DEFAULT_STUDIO_XMIN,
                numinous_core::DEFAULT_STUDIO_XMAX,
                1.0,
                true,
            ),
            (
                numinous_core::DEFAULT_STUDIO_XMIN,
                numinous_core::DEFAULT_STUDIO_XMAX,
                1.25,
                false,
            ),
            (0.0, 1.0, 1.0, false),
        ] {
            let saved = numinous_core::StudioCreation::new("x^3", xmin, xmax, a).expect("saved");
            let mut panel = StudioPanel::default();
            panel.open_creation(&saved);
            let sound = panel.load_random_recipe().expect("recipe");
            let next = panel.current_creation().expect("next");
            assert_eq!(panel.morph.is_some(), should_blend);
            assert_eq!(next.a(), 1.0);
            assert_eq!(sound, next.to_melody(32));
        }
    }

    #[test]
    fn a_parametric_pair_draws_sings_and_shares_as_one_creation() {
        let mut panel = StudioPanel::new("x(t)=cos(3*t); y(t)=sin(2*t)").expect("pair");
        assert!(panel.error.is_none());
        assert_eq!(
            panel
                .program
                .as_ref()
                .map(numinous_core::StudioProgram::kind),
            Some(numinous_core::StudioKind::Parametric)
        );
        assert_eq!(panel.current_sound().expect("voice").notes.len(), 32);

        let shared = panel.current_creation().expect("shared pair");
        assert_eq!(shared.kind(), numinous_core::StudioKind::Parametric);
        assert_eq!(shared.source(), "cos(3*t)");
        assert_eq!(shared.second_source(), Some("sin(2*t)"));
        assert_eq!(shared.scale(), numinous_core::StudioScale::Continuous);
        assert!(shared.to_num_file().starts_with("NUMINOUS_STUDIO 3\n"));

        let mut raster = Raster::new(240, 180);
        panel.draw(&mut raster, InputMode::KeyboardMouse, 240, 180);
        assert!(raster.lit_count() > 100, "the pair has a visible path");

        assert_eq!(panel.scale_name(), "continuous");
        let continuous = panel.current_sound().expect("continuous voice");
        let pentatonic = (0..4).fold(None, |_, _| panel.cycle_scale());
        assert_eq!(panel.scale_name(), "pentatonic");
        assert_ne!(pentatonic.expect("scaled voice"), continuous);
        assert_eq!(
            panel.current_creation().expect("scaled pair").scale(),
            numinous_core::StudioScale::Pentatonic
        );
    }

    #[test]
    fn a_parametric_reopen_keeps_pair_scale_and_lineage_on_takeover() {
        let saved = numinous_core::StudioCreation::new_parametric(
            "cos(3*t+a)",
            "sin(2*t+a)",
            0.0,
            std::f64::consts::TAU,
            0.5,
        )
        .expect("pair")
        .with_scale(numinous_core::StudioScale::Major)
        .with_title("Orbit")
        .expect("title");
        let mut panel = StudioPanel::default();
        panel.open_creation(&saved);

        assert!(panel.opened_paused());
        assert_eq!(panel.scale_name(), "major");
        assert_eq!(panel.current_creation().expect("untouched"), saved);
        assert_eq!(panel.current_sound(), Some(saved.to_melody(32)));

        assert_eq!(panel.push_text("+0"), Some(saved.to_melody(32)));
        let edited = panel.current_creation().expect("edited pair");
        assert_eq!(edited.source(), "cos(3*t+a)");
        assert_eq!(edited.second_source(), Some("sin(2*t+a)+0"));
        assert_eq!(edited.xmin(), saved.xmin());
        assert_eq!(edited.xmax(), saved.xmax());
        assert_eq!(edited.a(), saved.a());
        assert_eq!(edited.scale(), numinous_core::StudioScale::Major);
        assert_eq!(edited.descends(), Some(saved.to_link().as_str()));
        assert_eq!(edited.title(), None);
        assert_eq!(panel.current_sound(), Some(edited.to_melody(32)));
    }

    #[test]
    fn a_fork_sings_at_once_and_its_shares_record_the_descent() {
        let parent = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
            .expect("parent")
            .with_title("Parent Wave")
            .expect("title");
        let mut panel = StudioPanel::default();

        let spec = panel.fork_creation(&parent);
        assert!(spec.is_some(), "a chosen fork sings without a preview");
        assert!(!panel.opened_paused());
        assert!(
            panel.opened_active(),
            "the parent window and knob still pin"
        );

        let shared = panel.current_creation().expect("shared");
        assert_eq!(shared.descends(), Some(parent.to_link().as_str()));

        // Edits keep the descent, because edits are the remix.
        assert!(panel.push_text("+0").is_some());
        let edited = panel.current_creation().expect("edited");
        assert_eq!(edited.descends(), Some(parent.to_link().as_str()));

        // A recipe draw is a different creation, not a descent.
        assert!(panel.load_random_recipe().is_some());
        let drawn = panel.current_creation().expect("recipe");
        assert_eq!(drawn.descends(), None);

        // So is opening something else.
        assert!(panel.fork_creation(&parent).is_some());
        let other = numinous_core::StudioCreation::new("x*x", -1.0, 1.0, 0.0).expect("other");
        panel.open_creation(&other);
        assert_eq!(panel.current_creation().expect("opened").descends(), None);
    }

    #[test]
    fn an_untouched_reopen_reshares_its_whole_identity() {
        // The capsule format exists to carry title, author, and lineage;
        // re-sharing an unedited reopen must not silently strip them by
        // rebuilding the creation from its numbers alone.
        let grandparent =
            numinous_core::StudioCreation::new("sin(x)", -1.0, 1.0, 0.0).expect("grandparent");
        let full = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
            .expect("creation")
            .with_title("Slow Waves")
            .expect("title")
            .with_author("A Curious Mind")
            .expect("author")
            .with_descends(&grandparent.to_link())
            .expect("descends");
        let mut panel = StudioPanel::default();
        panel.open_creation(&full);
        assert_eq!(
            panel.current_creation().expect("reshared"),
            full,
            "identity and lineage survive an untouched re-share"
        );

        assert!(panel.push_text("").is_none());
        assert!(panel.opened_paused());
        assert_eq!(panel.current_creation().expect("no edit"), full);

        // The first edit starts a remix. The parent's identity becomes
        // lineage while its numerical settings remain. The edited share
        // descends from what was opened, not from the
        // grandparent the opened capsule itself descends from.
        assert!(panel.push_text("+0").is_some());
        let taken_over = panel.current_creation().expect("taken over");
        assert_eq!(taken_over.title(), None);
        assert_eq!(
            taken_over.credit(),
            Some("After Slow Waves by A Curious Mind")
        );
        assert_eq!(taken_over.descends(), Some(full.to_link().as_str()));
    }

    #[test]
    fn a_recipe_draw_after_a_reopen_does_not_descend() {
        // The bank's curve replaces the whole creation; a still-pinned
        // reopen drops without becoming a parent.
        let parent = numinous_core::StudioCreation::new("sin(x)", -1.0, 1.0, 0.0).expect("parent");
        let mut panel = StudioPanel::default();
        panel.open_creation(&parent);
        assert!(panel.load_random_recipe().is_some());
        let drawn = panel.current_creation().expect("recipe");
        assert_eq!(drawn.descends(), None);
    }

    #[test]
    fn a_fork_descends_from_its_parent_but_does_not_wear_its_name() {
        let parent = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
            .expect("parent")
            .with_title("Parent Wave")
            .expect("title");
        let mut panel = StudioPanel::default();
        assert!(panel.fork_creation(&parent).is_some());
        let fork = panel.current_creation().expect("fork");
        assert_eq!(fork.descends(), Some(parent.to_link().as_str()));
        assert_eq!(
            fork.title(),
            None,
            "a fork is a new creation descending from the parent, not the \
             parent wearing its own name"
        );
        assert_eq!(
            fork.credit(),
            Some("After Parent Wave"),
            "a fork offers prose credit the player can edit before sharing"
        );
    }

    #[test]
    fn an_unshareable_lineage_is_named_not_blamed_on_the_formula() {
        let mut panel = StudioPanel::default();
        // No real to_link can exceed the descends cap today, so the panel is
        // handed an oversized parent directly: the refusal path must stay
        // honest even for states nothing currently produces.
        panel.force_fork_of(format!(
            "numinous://studio?expr=x&xmin=-1&xmax=1&a=0{}",
            "&".repeat(5000)
        ));
        assert_eq!(
            panel.current_creation(),
            Err(super::ShareRefusal::LineageTooLarge),
            "a lineage that cannot ride is its own refusal"
        );
    }

    #[test]
    fn a_mid_morph_postcard_matches_the_capsule_not_the_blend() {
        // The bundle's promise is that postcard.png shows what creation.num
        // reopens. A recipe morph is a 600 ms presentation blend the capsule
        // does not record, so the postcard must ignore it.
        let mut morphing = StudioPanel::default();
        assert!(morphing.load_random_recipe().is_some());
        let mid_morph = morphing.postcard_rgba(240, numinous_core::Era::Modern, None, None);

        let mut settled = StudioPanel::default();
        assert!(settled.load_random_recipe().is_some());
        settled.advance_morph(RECIPE_MORPH_SECONDS);
        let after = settled.postcard_rgba(240, numinous_core::Era::Modern, None, None);

        assert_eq!(
            mid_morph, after,
            "the postcard is the settled curve whether or not a morph is \
             mid-flight on screen"
        );
    }

    #[test]
    fn the_postcard_draws_the_shared_window() {
        // Two saved windows, one source: the postcards must differ, or the
        // postcard would be drawing some other window than the one the
        // bundle's creation.num promises to reopen.
        let narrow = numinous_core::StudioCreation::new("sin(x)", 0.0, 1.0, 1.0).expect("narrow");
        let wide = numinous_core::StudioCreation::new("sin(x)", -6.0, 6.0, 1.0).expect("wide");
        let postcard = |creation: &numinous_core::StudioCreation| {
            let mut panel = StudioPanel::default();
            panel.open_creation(creation);
            panel.postcard_rgba(300, numinous_core::Era::Modern, None, None)
        };
        let narrow_rgba = postcard(&narrow);
        assert!(
            narrow_rgba.iter().any(|&byte| byte > 32),
            "a postcard has ink"
        );
        assert_ne!(narrow_rgba, postcard(&wide));
    }

    #[test]
    fn the_postcard_carries_title_and_author_when_the_capsule_has_them() {
        // The postcard is the object built to escape the app; identity must
        // ride the pixels, not only the capsule. Title changes the headline,
        // author signs the corner, and an anonymous card stays exactly the
        // card it always was.
        let panel = StudioPanel::default();
        let plain = panel.postcard_rgba(300, numinous_core::Era::Modern, None, None);
        let titled =
            panel.postcard_rgba(300, numinous_core::Era::Modern, Some("Fading Wave"), None);
        let signed = panel.postcard_rgba(
            300,
            numinous_core::Era::Modern,
            Some("Fading Wave"),
            Some("A Curious Mind"),
        );
        assert_ne!(plain, titled, "a title must change the headline");
        assert_ne!(titled, signed, "an author must sign the card");
        assert_ne!(plain, signed);
    }

    #[test]
    fn editing_stops_at_the_portable_source_limit() {
        let mut panel = StudioPanel::new("x").expect("panel");
        for _ in 1..numinous_core::MAX_STUDIO_EDITOR_CHARS {
            panel.push_space();
        }
        assert_eq!(
            panel.source.chars().count(),
            numinous_core::MAX_STUDIO_EDITOR_CHARS
        );

        panel.push_space();
        assert_eq!(
            panel.source.chars().count(),
            numinous_core::MAX_STUDIO_EDITOR_CHARS
        );
        panel.source = "x".repeat(numinous_core::MAX_STUDIO_EDITOR_CHARS + 1);
        assert!(!panel.push_space());
    }

    #[test]
    fn over_limit_character_events_are_rejected_atomically() {
        let source = format!(
            "{}x",
            " ".repeat(numinous_core::MAX_STUDIO_EDITOR_CHARS - 1)
        );
        let mut panel = StudioPanel::new(&source).expect("panel");

        assert!(panel.push_text("+1").is_none());
        assert_eq!(panel.source, source);
        assert!(panel.expr.is_some());
        assert!(panel.error.is_none());
    }

    #[test]
    fn construction_rejects_over_limit_unicode_source() {
        let source = "π".repeat(MAX_STUDIO_EDITOR_CHARS + 1);
        assert!(StudioPanel::new(&source).is_err());
    }

    #[test]
    fn controller_footer_names_the_keyboard_requirement_and_fits() {
        let copy = input_legend::studio_controls(InputMode::Controller);
        assert_eq!(copy, "UP A+  DOWN A-  L3 A=1  EAST MENU  KEYBOARD TYPES");
        assert!(copy.contains("KEYBOARD TYPES"));

        for (width, height) in [(360, 240), (900, 700)] {
            let scale = studio_scale(width);
            assert!(
                10 + numinous_core::text_width(&copy, scale) <= width as i32,
                "Studio controls clip at {width}x{height}"
            );
            let panel = StudioPanel::default();
            let mut raster = Raster::new(width, height);
            panel.draw(&mut raster, InputMode::Controller, width, height);
            assert!(raster.lit_count() > 100);
        }
    }

    #[test]
    fn compact_parameter_labels_preserve_every_significant_digit() {
        for value in [
            0.0,
            -0.0,
            1.0,
            0.123_456_789_012_345_66,
            1.234_567_890_123_456_7e-100,
            -f64::MIN_POSITIVE,
            f64::from_bits(1),
            -f64::from_bits(1),
            1e12,
            -1e12,
        ] {
            let label = compact_number(value);
            assert_eq!(
                label.parse::<f64>().expect("displayed number").to_bits(),
                value.to_bits(),
                "the label must retain the exact imported value: {label}"
            );
            assert!(label.len() <= 24, "the complete label must fit: {label}");
        }
        assert_eq!(compact_number(1.25), "1.25");
        assert_eq!(compact_number(1e-300), "1e-300");
        assert_eq!(compact_number(1e12), "1e12");
    }

    fn assert_composed_text_line(raster: &Raster, text: &str, y: i32, scale: i32, mark: char) {
        assert!(
            10 + numinous_core::text_width(text, scale) <= raster.width() as i32 - 10,
            "the entire text must fit with both margins: {text}"
        );
        let mut expected = Raster::new(raster.width(), raster.height());
        numinous_core::draw_text(&mut expected, text, 10, y, scale, mark);
        assert!(
            expected.lit_count() > 0,
            "the expected text must be visible"
        );
        let band = y as usize * raster.width() * 4..(y + 7 * scale) as usize * raster.width() * 4;
        assert_eq!(
            &raster.to_rgba()[band.clone()],
            &expected.to_rgba()[band],
            "all text pixels must survive the composed panel: {text}"
        );
    }

    #[test]
    fn exact_parameter_and_preview_action_survive_compact_and_large_panels() {
        for (width, height) in [(360, 240), (900, 700)] {
            let scale = studio_scale(width);
            for mode in [InputMode::KeyboardMouse, InputMode::Controller] {
                for value in [
                    0.123_456_789_012_345_66,
                    1e-300,
                    -f64::MIN_POSITIVE,
                    f64::from_bits(1),
                    1e12,
                    -1e12,
                ] {
                    let saved =
                        numinous_core::StudioCreation::new("sin(a*x)+x/3", -1e12, 1e12, value)
                            .expect("admitted values");
                    let mut panel = StudioPanel::default();
                    panel.open_creation(&saved);
                    let mut raster = Raster::new(width, height);
                    panel.draw(&mut raster, mode, width, height);
                    assert_composed_text_line(
                        &raster,
                        &format!("A {}  ENTER: PLAY", compact_number(value)),
                        10 + 34 * scale,
                        scale,
                        '*',
                    );
                    assert_eq!(panel.current_creation().expect("untouched"), saved);

                    assert!(panel.push_text("(").is_none());
                    let mut draft = Raster::new(width, height);
                    panel.draw(&mut draft, mode, width, height);
                    assert_composed_text_line(
                        &draft,
                        &format!("A {}  SCALE CONTINUOUS", compact_number(value)),
                        10 + 34 * scale,
                        scale,
                        '*',
                    );
                    let columns = width.saturating_sub(20) / (6 * scale as usize);
                    let [_, (error, mark)] = panel.status_lines(mode, columns);
                    assert!(error.starts_with("DRAFT: "));
                    assert_composed_text_line(&draft, &error, 10 + 44 * scale, scale, mark);
                    assert!(panel.current_creation().is_err());
                }
            }
        }
    }

    #[test]
    fn secondary_text_marks_truncation_and_hostile_footer_widths_stay_bounded() {
        assert_eq!(fit_studio_line("REOPENED X -1 TO 2", 14), "REOPENED X ...");
        assert_eq!(fit_studio_line("DRAFT: unexpected character", 2), "..");
        assert_eq!(fit_studio_line("DRAFT", 0), "");
        for columns in 0..58 {
            let lines = studio_footer_lines(
                InputMode::Controller,
                ControllerCopy::empty(ControllerFace::Generic),
                columns,
            );
            assert!(lines.len() <= 2);
            assert!(lines.iter().all(|line| line.chars().count() <= columns));
            if (1..20).contains(&columns) {
                assert!(lines.last().expect("bounded footer").ends_with('.'));
            }
        }
    }

    #[test]
    fn mapped_and_unbound_controls_survive_the_composed_footer() {
        let mut mapped = ControllerCopy::empty(ControllerFace::PlayStation);
        for (action, buttons) in [
            (
                ControllerAction::Up,
                [
                    ControllerButton::North,
                    ControllerButton::Start,
                    ControllerButton::LeftThumb,
                    ControllerButton::RightThumb,
                ],
            ),
            (
                ControllerAction::Down,
                [
                    ControllerButton::West,
                    ControllerButton::RightTrigger,
                    ControllerButton::LeftTrigger2,
                    ControllerButton::RightTrigger2,
                ],
            ),
            (
                ControllerAction::Reset,
                [
                    ControllerButton::East,
                    ControllerButton::LeftTrigger,
                    ControllerButton::DPadLeft,
                    ControllerButton::DPadRight,
                ],
            ),
            (
                ControllerAction::Back,
                [
                    ControllerButton::South,
                    ControllerButton::Select,
                    ControllerButton::DPadUp,
                    ControllerButton::DPadDown,
                ],
            ),
        ] {
            for button in buttons {
                mapped.bind(action, button);
            }
        }
        for copy in [
            ControllerFace::Generic.into(),
            ControllerFace::Xbox.into(),
            ControllerFace::PlayStation.into(),
            ControllerCopy::empty(ControllerFace::Generic),
            mapped,
        ] {
            for (width, height) in [(360, 240), (900, 700)] {
                let scale = studio_scale(width);
                let columns = width.saturating_sub(20) / (6 * scale as usize);
                let lines = studio_footer_lines(InputMode::Controller, copy, columns);
                let complete =
                    input_legend::studio_controls_with_controller(InputMode::Controller, copy);
                assert_eq!(
                    lines.join(" ").split_whitespace().collect::<Vec<_>>(),
                    complete.split_whitespace().collect::<Vec<_>>(),
                    "every effective control must fit at {width}x{height}"
                );
                let panel = StudioPanel::default();
                let mut raster = Raster::new(width, height);
                panel.draw_with_controller(&mut raster, InputMode::Controller, copy, width, height);
                for (index, line) in lines.iter().enumerate() {
                    let rows_below = lines.len() - index - 1;
                    assert_composed_text_line(
                        &raster,
                        line,
                        height as i32 - (11 + 10 * rows_below as i32) * scale,
                        scale,
                        '#',
                    );
                }
            }
        }
    }
}
