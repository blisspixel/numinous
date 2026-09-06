//! A directly accessible, case-preserving reader for shared room study.
//!
//! This component owns only reading state. It has no Journey, room input,
//! simulation clock, audio transport, or reward capability.

use std::sync::Arc;

use numinous_core::{
    Room, RoomStudy, StudyDepth, StudyInline, StudyLocale, StudyPart, StudyTranslationStatus,
    rooms_with_authored_depth,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::input_legend::{ControllerAction, ControllerCopy, InputMode};
use crate::study_text::{StudyText, TextError, TextLayout, TextRole, TextSpan, TextViewport};

const BACKGROUND: [u8; 4] = [10, 11, 15, 255];
const FOREGROUND: [u8; 4] = [233, 237, 240, 255];
const SECONDARY: [u8; 4] = [169, 181, 191, 255];
const ACCENT: [u8; 4] = [78, 255, 255, 255];

/// Semantic navigation, shared by keyboard, pointer, and controller routing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReaderCommand {
    /// Move by a signed number of text lines, including fractional wheel input.
    Lines(f32),
    /// Move by pages with one line of overlap.
    Pages(i8),
    /// Move to the beginning of the current depth.
    Start,
    /// Move to the end of the current depth.
    End,
    /// Move between adjacent depths without wrapping at either end.
    Depth(i8),
    /// Open a depth directly, whether or not it has authored content.
    Select(StudyDepth),
    /// Open mathematics directly, without requiring earlier reading.
    Mathematics,
    /// Return to the exact activity or Cabinet that opened this reader.
    Back,
    /// Request the other pilot reading language.
    Language,
}

/// A reader action requiring its App owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReaderIntent {
    /// The reading action has been handled locally.
    None,
    /// Close the reader without changing the room.
    Close,
    /// Replace the document using the requested content language.
    Language(StudyLocale),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Back,
    Language,
    Depth(StudyDepth),
}

#[derive(Debug, Clone, Copy)]
struct Geometry {
    back: TextViewport,
    title: TextViewport,
    language: TextViewport,
    tabs: [TextViewport; 3],
    body: TextViewport,
    footer: TextViewport,
}

impl Geometry {
    fn content_width(width: u32, compact: bool) -> u32 {
        let inset = if compact { 12 } else { 28 };
        width.saturating_sub(inset * 2).clamp(1, 780)
    }

    fn new(width: u32, height: u32, compact: bool, line_height: f32, footer_height: u32) -> Self {
        let inset = if compact { 12 } else { 28 };
        let available = Self::content_width(width, compact);
        let x = width.saturating_sub(available) / 2;
        let line = line_height.ceil() as u32;
        let button = line + if compact { 4 } else { 8 };
        let gap = if compact { 6 } else { 8 };
        let back_width = if compact { 124 } else { 68 };
        let language_width = if compact { 80 } else { 116 };
        let tab_y = inset + button + gap;
        let body_y = tab_y + button + gap;
        let footer_y = height.saturating_sub(inset + footer_height);
        let rect = |left, top, width, height| TextViewport {
            x: left as i32,
            y: top as i32,
            width,
            height,
        };
        let tabs = std::array::from_fn(|index| {
            let start = available * index as u32 / 3;
            let end = available * (index as u32 + 1) / 3;
            rect(x + start, tab_y, end - start, button)
        });
        Self {
            back: rect(x, inset, back_width, button),
            title: rect(
                x + back_width + 8,
                inset,
                available
                    .saturating_sub(back_width + language_width + 16)
                    .max(1),
                button,
            ),
            language: rect(
                x + available.saturating_sub(language_width),
                inset,
                language_width,
                button,
            ),
            tabs,
            body: rect(
                x,
                body_y,
                available.saturating_sub(12).max(1),
                footer_y.saturating_sub(body_y + gap).max(1),
            ),
            footer: rect(x, footer_y, available, footer_height),
        }
    }

    fn hit(self, point: (f64, f64)) -> Option<Target> {
        let inside = |rect: TextViewport| {
            point.0.is_finite()
                && point.1.is_finite()
                && point.0 >= f64::from(rect.x)
                && point.1 >= f64::from(rect.y)
                && point.0 < f64::from(rect.x) + f64::from(rect.width)
                && point.1 < f64::from(rect.y) + f64::from(rect.height)
        };
        if inside(self.back) {
            return Some(Target::Back);
        }
        if inside(self.language) {
            return Some(Target::Language);
        }
        self.tabs
            .into_iter()
            .zip(StudyDepth::ALL)
            .find_map(|(rect, depth)| inside(rect).then_some(Target::Depth(depth)))
    }
}

#[derive(Debug)]
struct Composition {
    size: (u32, u32),
    mode: InputMode,
    controller: ControllerCopy,
    geometry: Geometry,
    title: Arc<TextLayout>,
    back: Arc<TextLayout>,
    language: Arc<TextLayout>,
    tabs: [Arc<TextLayout>; 3],
    body: Arc<TextLayout>,
    footer: Arc<TextLayout>,
}

/// An optional room document and its independent per-depth reading positions.
#[derive(Debug)]
pub struct StudyReader {
    document: RoomStudy,
    title: String,
    depth: StudyDepth,
    scroll: [f32; 3],
    anchors: [usize; 3],
    depth_layouts: [Option<(u32, u32)>; 3],
    text: StudyText,
    composition: Option<Composition>,
    pressed: Option<Target>,
    notice: Option<&'static str>,
}

impl StudyReader {
    /// Read a room without visiting, scoring, or mutating it.
    pub fn new(room: &dyn Room, locale: &StudyLocale) -> Result<Self, TextError> {
        let document = numinous_core::room_study_for_locale(room, locale);
        let text = StudyText::new(document.locale.resolved)?;
        Ok(Self {
            document,
            title: room.meta().title.to_string(),
            depth: StudyDepth::Explanation,
            scroll: [0.0; 3],
            anchors: [0; 3],
            depth_layouts: [None; 3],
            text,
            composition: None,
            pressed: None,
            notice: None,
        })
    }

    /// The complete shared document, including actual translation availability.
    pub fn document(&self) -> &RoomStudy {
        &self.document
    }

    /// The depth currently selected; absence of content is never an unlock.
    pub fn depth(&self) -> StudyDepth {
        self.depth
    }

    /// Pixel position within the current shaped body.
    pub fn scroll(&self) -> f32 {
        self.scroll[self.depth_index()]
    }

    fn depth_index(&self) -> usize {
        StudyDepth::ALL
            .iter()
            .position(|depth| *depth == self.depth)
            .unwrap_or(0)
    }

    fn remember_anchor(&mut self) {
        let index = self.depth_index();
        if let Some(composition) = &self.composition {
            self.anchors[index] = composition.body.source_offset_at_scroll(self.scroll[index]);
        }
    }

    /// Handle a reading command; this never returns a gameplay action.
    pub fn navigate(&mut self, command: ReaderCommand) -> ReaderIntent {
        let index = self.depth_index();
        let metrics = self.composition.as_ref().map(|view| {
            (
                view.body.line_height(),
                view.geometry.body.height,
                view.body.max_scroll(view.geometry.body.height),
            )
        });
        let target = match command {
            ReaderCommand::Back => return ReaderIntent::Close,
            ReaderCommand::Language => {
                let tag = if self.document.locale.requested.language() == "ja" {
                    "en"
                } else {
                    "ja"
                };
                if let Ok(locale) = StudyLocale::parse(tag) {
                    return ReaderIntent::Language(locale);
                }
                return ReaderIntent::None;
            }
            ReaderCommand::Select(depth) => Some(depth),
            ReaderCommand::Mathematics => Some(StudyDepth::Mathematics),
            ReaderCommand::Depth(delta) => {
                let next = (index as i16 + i16::from(delta)).clamp(0, 2) as usize;
                Some(StudyDepth::ALL[next])
            }
            ReaderCommand::Start => {
                self.scroll[index] = 0.0;
                None
            }
            ReaderCommand::End => {
                if let Some((_, _, maximum)) = metrics {
                    self.scroll[index] = maximum;
                }
                None
            }
            ReaderCommand::Lines(lines) => {
                if let Some((line, _, maximum)) = metrics
                    && lines.is_finite()
                {
                    self.scroll[index] = (self.scroll[index] + lines.clamp(-100.0, 100.0) * line)
                        .clamp(0.0, maximum);
                }
                None
            }
            ReaderCommand::Pages(pages) => {
                if let Some((line, height, maximum)) = metrics {
                    let page = (height as f32 - line).max(line);
                    self.scroll[index] =
                        (self.scroll[index] + f32::from(pages) * page).clamp(0.0, maximum);
                }
                None
            }
        };
        if let Some(depth) = target
            && StudyDepth::ALL.contains(&depth)
            && depth != self.depth
        {
            self.remember_anchor();
            self.depth = depth;
            self.composition = None;
            self.pressed = None;
        }
        ReaderIntent::None
    }

    /// Capture only a fresh press on a visible reader button.
    pub fn pointer_down(&mut self, point: (f64, f64)) {
        self.pressed = self
            .composition
            .as_ref()
            .and_then(|view| view.geometry.hit(point));
    }

    /// Activate only when the release matches this reader's own press.
    pub fn pointer_up(&mut self, point: (f64, f64)) -> ReaderIntent {
        let target = self
            .composition
            .as_ref()
            .and_then(|view| view.geometry.hit(point));
        let pressed = self.pressed.take();
        if pressed.is_none() || target != pressed {
            return ReaderIntent::None;
        }
        match target {
            Some(Target::Back) => ReaderIntent::Close,
            Some(Target::Language) => self.navigate(ReaderCommand::Language),
            Some(Target::Depth(depth)) => self.navigate(ReaderCommand::Select(depth)),
            None => ReaderIntent::None,
        }
    }

    /// Forget pointer capture on focus loss or resizing, without a synthetic release.
    pub fn clear_pointer(&mut self) {
        self.pressed = None;
    }

    /// Explain why a dropped creation does not replace the room while reading.
    pub fn show_file_drop_notice(&mut self) {
        self.notice = Some(if self.japanese() {
            "ファイルを開くには、プレイ画面に戻ってください。"
        } else {
            "Return to play to open a file."
        });
        let index = self.depth_index();
        self.anchors[index] = 0;
        self.scroll[index] = 0.0;
        self.composition = None;
    }

    fn japanese(&self) -> bool {
        self.document.locale.requested.language() == "ja"
    }

    fn body_spans(&self) -> Vec<(String, TextRole)> {
        let ja = self.japanese();
        // The pinned header already names the room. Keep reading metadata short
        // enough that a compact first page reaches the selected content.
        let mut spans = Vec::new();
        if let Some(notice) = self.notice {
            spans.push((format!("{notice}\n"), TextRole::Prose));
        }
        if self.document.locale.resolved != self.document.locale.requested.language() {
            spans.push((
                if ja {
                    "日本語訳がないため、英語で表示します。\n".to_string()
                } else {
                    format!(
                        "Text unavailable in {}; showing English.\n",
                        self.document.locale.requested
                    )
                },
                TextRole::Prose,
            ));
        } else if self.document.locale.resolved == "ja"
            && self
                .document
                .blocks_at(self.depth)
                .any(|block| block.translation == StudyTranslationStatus::ReviewedDraft)
        {
            spans.push(("日本語訳: 試作版\n".to_string(), TextRole::Prose));
        }
        if !self.document.has_depth(self.depth) {
            spans.push((if ja {
                "この深さの解説はまだありません。補足や説明は自由に読むことができます。プレイや得点で解放するものではありません。".to_string()
            } else {
                "This depth has not been written for this room yet. Explanation and notes are freely available when present. Reading has no play or score requirement.".to_string()
            }, TextRole::Prose));
            // Naming where it is written keeps an unwritten depth from reading
            // as a broken one, and saves hunting the catalog room by room.
            let authored = rooms_with_authored_depth(self.depth);
            if !authored.is_empty() {
                spans.push((
                    if ja {
                        format!(
                            "この深さが書かれているルーム: {}
",
                            authored.join(", ")
                        )
                    } else {
                        format!(
                            "Written so far for: {}
",
                            authored.join(", ")
                        )
                    },
                    TextRole::Prose,
                ));
            }
        }
        for (index, block) in self.document.blocks_at(self.depth).enumerate() {
            // Measured line spacing separates parts without blank rows. A blank
            // row between blocks still makes each new section easy to find.
            if index > 0 {
                spans.push(("\n".to_string(), TextRole::Prose));
            }
            spans.push((format!("{}\n", block.title), TextRole::Prose));
            if block.locale.resolved != self.document.locale.requested.language()
                && block.locale.resolved != self.document.locale.resolved
            {
                spans.push((
                    if ja {
                        "この節は英語です。\n"
                    } else {
                        "This section is in English.\n"
                    }
                    .to_string(),
                    TextRole::Prose,
                ));
            }
            for part in &block.parts {
                match part {
                    StudyPart::Paragraph(runs) => {
                        for run in runs {
                            match run {
                                StudyInline::Text(text) => {
                                    spans.push((text.to_string(), TextRole::Prose))
                                }
                                StudyInline::Math(text) => {
                                    spans.push((text.to_string(), TextRole::Math))
                                }
                                _ => spans.push((run.as_str().to_string(), TextRole::Prose)),
                            }
                        }
                    }
                    StudyPart::Equation(text) => spans.push((text.to_string(), TextRole::Math)),
                    StudyPart::Reference {
                        source,
                        description,
                    } => spans.push((
                        format!("{}\n{description}\n{}", source.title, source.url),
                        TextRole::Prose,
                    )),
                    _ => spans.push((part.plain_text(), TextRole::Prose)),
                }
                spans.push(("\n".to_string(), TextRole::Prose));
            }
        }
        spans
    }

    fn prose(&mut self, text: &str, width: u32, size: f32) -> Result<Arc<TextLayout>, TextError> {
        self.text.layout(
            &[TextSpan {
                text,
                role: TextRole::Prose,
            }],
            width,
            size,
        )
    }

    fn footer_text(&self, mode: InputMode, controller: ControllerCopy, compact: bool) -> String {
        let ja = self.japanese();
        match mode {
            InputMode::KeyboardMouse if compact => {
                if ja {
                    "上下: スクロール  左右: 深さ".to_string()
                } else {
                    "Up/Down: scroll | Left/Right: depth".to_string()
                }
            }
            InputMode::KeyboardMouse => {
                if ja {
                    "上下: スクロール  左右: 深さ\nEsc: 戻る  L: 言語  Enter: 数学".to_string()
                } else {
                    "Up/Down: scroll | Left/Right: depth\nEsc: back | L: language | Enter: math"
                        .to_string()
                }
            }
            InputMode::Controller if compact => format!(
                "{}/{}: {} | {}/{}: {}",
                controller.action_token(ControllerAction::Up),
                controller.action_token(ControllerAction::Down),
                if ja { "スクロール" } else { "scroll" },
                controller.action_token(ControllerAction::Left),
                controller.action_token(ControllerAction::Right),
                if ja { "深さ" } else { "depth" }
            ),
            InputMode::Controller => format!(
                "{}/{}: {}\n{}: {} | {}/{}: {}",
                controller.action_token(ControllerAction::Up),
                controller.action_token(ControllerAction::Down),
                if ja { "スクロール" } else { "scroll" },
                controller.action_token(ControllerAction::Back),
                if ja { "戻る" } else { "back" },
                controller.action_token(ControllerAction::Left),
                controller.action_token(ControllerAction::Right),
                if ja { "深さ" } else { "depth" }
            ),
        }
    }

    fn compose(
        &mut self,
        width: u32,
        height: u32,
        mode: InputMode,
        controller: ControllerCopy,
    ) -> Result<(), TextError> {
        if self.composition.as_ref().is_some_and(|view| {
            view.size == (width, height) && view.mode == mode && view.controller == controller
        }) {
            return Ok(());
        }
        let resized = self
            .composition
            .as_ref()
            .is_none_or(|view| view.size != (width, height));
        self.remember_anchor();
        if resized {
            self.clear_pointer();
        }
        let compact = width < 600 || height < 500;
        let size = if compact { 14.0 } else { 20.0 };
        let chrome_size = if compact { 12.0 } else { 20.0 };
        // Scientific text keeps its own measured line spacing. Compact chrome
        // has fewer jobs and smaller type, so it need not consume body-sized rows.
        let line = self.prose("Ag", 100, chrome_size)?.line_height();
        let footer_text = self.footer_text(mode, controller, compact);
        let footer = self.prose(
            &footer_text,
            Geometry::content_width(width, compact),
            chrome_size,
        )?;
        let geometry = Geometry::new(width, height, compact, line, footer.height().ceil() as u32);
        let ja = self.japanese();
        let back_label = if ja { "戻る" } else { "Back" };
        let back_text = if compact {
            let key = match mode {
                InputMode::KeyboardMouse => "Esc".to_string(),
                InputMode::Controller => controller.action_token(ControllerAction::Back),
            };
            format!("{key}: {back_label}")
        } else {
            back_label.to_string()
        };
        let back = self.prose(&back_text, geometry.back.width, chrome_size)?;
        let language_label = if ja { "English" } else { "日本語" };
        let language_text = if compact && mode == InputMode::KeyboardMouse {
            format!("L: {language_label}")
        } else {
            language_label.to_string()
        };
        let language = self.prose(&language_text, geometry.language.width, chrome_size)?;
        let mut title_text = self.title.clone();
        let title = loop {
            let layout = self.prose(&title_text, geometry.title.width, chrome_size)?;
            if layout.visual_line_count() <= 1 {
                break layout;
            }
            let stem = title_text.trim_end_matches('.');
            let mut graphemes: Vec<_> = stem.graphemes(true).collect();
            if graphemes.len() < 2 {
                break self.prose("", geometry.title.width, chrome_size)?;
            }
            graphemes.pop();
            title_text = format!("{}...", graphemes.concat());
        };
        let labels = if ja {
            ["説明", "補足", "数学"]
        } else {
            ["Explain", "Notes", "Mathematics"]
        };
        let tabs = [
            self.prose(labels[0], geometry.tabs[0].width, chrome_size)?,
            self.prose(labels[1], geometry.tabs[1].width, chrome_size)?,
            self.prose(labels[2], geometry.tabs[2].width, chrome_size)?,
        ];
        let owned = self.body_spans();
        let spans: Vec<_> = owned
            .iter()
            .map(|(text, role)| TextSpan { text, role: *role })
            .collect();
        let body = self.text.layout(&spans, geometry.body.width, size)?;
        let index = self.depth_index();
        let body_geometry = (geometry.body.width, size as u32);
        if self.depth_layouts[index] != Some(body_geometry) {
            self.scroll[index] = body.scroll_for_source_offset(self.anchors[index]);
        }
        self.depth_layouts[index] = Some(body_geometry);
        self.scroll[index] = self.scroll[index].min(body.max_scroll(geometry.body.height));
        self.composition = Some(Composition {
            size: (width, height),
            mode,
            controller,
            geometry,
            title,
            back,
            language,
            tabs,
            body,
            footer,
        });
        Ok(())
    }

    /// Draw the complete opaque reader at the App's actual pixel dimensions.
    /// Only body text scrolls; every button uses its drawn rectangle for input.
    pub fn render(
        &mut self,
        width: u32,
        height: u32,
        mode: InputMode,
        controller: ControllerCopy,
    ) -> Result<Vec<u8>, TextError> {
        let pixels = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .filter(|bytes| *bytes <= 256 * 1024 * 1024)
            .ok_or(TextError::InvalidFrame)?;
        if width == 0 || height == 0 {
            return Err(TextError::InvalidFrame);
        }
        self.compose(width, height, mode, controller)?;
        let mut rgba = Vec::with_capacity(pixels);
        rgba.extend(BACKGROUND.into_iter().cycle().take(pixels));
        let index = self.depth_index();
        let Some(view) = &self.composition else {
            return Err(TextError::InvalidFrame);
        };
        let geometry = view.geometry;
        let dims = (width as usize, height as usize);
        for (layout, rect, color) in [
            (&view.title, geometry.title, SECONDARY),
            (&view.back, geometry.back, FOREGROUND),
            (&view.language, geometry.language, FOREGROUND),
            (&view.footer, geometry.footer, SECONDARY),
        ] {
            self.text.draw(layout, &mut rgba, dims, rect, 0.0, color)?;
        }
        for (i, (layout, rect)) in view.tabs.iter().zip(geometry.tabs).enumerate() {
            self.text.draw(
                layout,
                &mut rgba,
                dims,
                rect,
                0.0,
                if i == index { FOREGROUND } else { SECONDARY },
            )?;
            if i == index {
                fill(
                    &mut rgba,
                    dims,
                    TextViewport {
                        y: rect.y + rect.height as i32 - 3,
                        height: 2,
                        ..rect
                    },
                    ACCENT,
                );
            }
        }
        self.text.draw(
            &view.body,
            &mut rgba,
            dims,
            geometry.body,
            self.scroll[index],
            FOREGROUND,
        )?;
        let maximum = view.body.max_scroll(geometry.body.height);
        if maximum > 0.0 {
            let track = TextViewport {
                x: geometry.body.x + geometry.body.width as i32 + 8,
                width: 2,
                ..geometry.body
            };
            fill(&mut rgba, dims, track, [58, 69, 78, 255]);
            let thumb = ((geometry.body.height as f32 / view.body.height())
                * geometry.body.height as f32)
                .clamp(
                    8.0_f32.min(geometry.body.height as f32),
                    geometry.body.height as f32,
                ) as u32;
            let offset = ((geometry.body.height - thumb) as f32 * self.scroll[index] / maximum)
                .round() as i32;
            fill(
                &mut rgba,
                dims,
                TextViewport {
                    y: track.y + offset,
                    height: thumb,
                    ..track
                },
                SECONDARY,
            );
        }
        Ok(rgba)
    }
}

fn fill(rgba: &mut [u8], dimensions: (usize, usize), rect: TextViewport, color: [u8; 4]) {
    let (width, height) = dimensions;
    let left = i64::from(rect.x).clamp(0, width as i64) as usize;
    let top = i64::from(rect.y).clamp(0, height as i64) as usize;
    let right = (i64::from(rect.x) + i64::from(rect.width)).clamp(0, width as i64) as usize;
    let bottom = (i64::from(rect.y) + i64::from(rect.height)).clamp(0, height as i64) as usize;
    for y in top..bottom {
        for x in left..right {
            let index = (y * width + x) * 4;
            rgba[index..index + 4].copy_from_slice(&color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader(room: &str, locale: &str) -> StudyReader {
        let room = numinous_core::room_by_id(room).expect("catalog fixture");
        StudyReader::new(room.as_ref(), &StudyLocale::parse(locale).unwrap()).unwrap()
    }

    fn draw(reader: &mut StudyReader, size: (u32, u32), mode: InputMode) -> Vec<u8> {
        reader
            .render(size.0, size.1, mode, ControllerCopy::default())
            .unwrap()
    }

    fn center(rect: TextViewport) -> (f64, f64) {
        (
            f64::from(rect.x) + f64::from(rect.width) / 2.0,
            f64::from(rect.y) + f64::from(rect.height) / 2.0,
        )
    }

    #[test]
    fn pilot_text_and_notation_remain_reachable_at_both_supported_sizes() {
        for locale in ["en", "ja"] {
            for size in [(360, 240), (900, 700)] {
                let mut reader = reader("lissajous", locale);
                for depth in StudyDepth::ALL {
                    reader.navigate(ReaderCommand::Select(depth));
                    let before = draw(&mut reader, size, InputMode::KeyboardMouse);
                    let view = reader.composition.as_ref().unwrap();
                    assert!(
                        view.body.missing_glyphs().is_empty(),
                        "{locale} {depth:?}: {:?}",
                        view.body.missing_glyphs()
                    );
                    assert!(
                        view.body.width() <= view.geometry.body.width as f32 + 1.0,
                        "unreachable horizontal text: {locale} {depth:?}"
                    );
                    assert!(
                        view.footer.height() <= view.geometry.footer.height as f32,
                        "clipped navigation: {locale}"
                    );
                    for block in reader.document.blocks_at(depth) {
                        for part in &block.parts {
                            assert!(
                                view.body.source().contains(&part.plain_text()),
                                "missing exact source in {locale} {}",
                                block.id
                            );
                        }
                    }
                    let body = view.geometry.body;
                    let maximum = view.body.max_scroll(body.height);
                    reader.navigate(ReaderCommand::End);
                    let after = draw(&mut reader, size, InputMode::KeyboardMouse);
                    assert_eq!(reader.scroll(), maximum);
                    // Actual pixels outside the body and scrollbar stay fixed.
                    for y in 0..size.1 {
                        if y < body.y as u32 || y >= body.y as u32 + body.height {
                            let start = (y * size.0 * 4) as usize;
                            let end = start + size.0 as usize * 4;
                            assert_eq!(
                                before[start..end],
                                after[start..end],
                                "scroll moved pinned chrome"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_first_page_reaches_selected_content_and_a_complete_initial_equation() {
        for locale in ["en", "ja"] {
            for size in [(360, 240), (900, 700)] {
                let mut reader = reader("lissajous", locale);
                for depth in StudyDepth::ALL {
                    reader.navigate(ReaderCommand::Select(depth));
                    let rgba = draw(&mut reader, size, InputMode::KeyboardMouse);
                    let view = reader.composition.as_ref().unwrap();
                    let block = reader.document.blocks_at(depth).next().unwrap();
                    let source = view.body.source();
                    let first_part = block.parts.first().unwrap().plain_text();
                    let start = source.find(&first_part).expect("canonical opening content");
                    let line = view.body.line_height();
                    assert_eq!(reader.scroll(), 0.0);
                    assert!(
                        view.body.scroll_for_source_offset(start) + line
                            <= view.geometry.body.height as f32,
                        "metadata hides the opening content: {locale} {depth:?} {size:?}"
                    );
                    if depth != StudyDepth::Mathematics {
                        continue;
                    }
                    let equation = block
                        .parts
                        .iter()
                        .find_map(|part| match part {
                            StudyPart::Equation(text) => Some(*text),
                            _ => None,
                        })
                        .unwrap();
                    let first_line = equation.lines().next().unwrap();
                    let start = source.find(equation).expect("unchanged equation source");
                    let last = start + first_line.char_indices().last().unwrap().0;
                    let top = view.body.scroll_for_source_offset(start);
                    let bottom = view.body.scroll_for_source_offset(last) + line;
                    assert!(
                        bottom <= view.geometry.body.height as f32,
                        "the first equation needs a full visible line: {locale} {size:?}, bottom {bottom}"
                    );
                    let rect = view.geometry.body;
                    let first_row = rect.y as u32 + top.floor() as u32;
                    let last_row = rect.y as u32 + bottom.ceil() as u32;
                    assert!(
                        (first_row..last_row).any(|y| {
                            (rect.x as u32..rect.x as u32 + rect.width).any(|x| {
                                let pixel = ((y * size.0 + x) * 4) as usize;
                                rgba[pixel..pixel + 4] != BACKGROUND
                            })
                        }),
                        "the shaped first equation must paint visible ink: {locale} {size:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn brief_metadata_distinguishes_a_translation_draft_from_english_notes_and_fallback() {
        let mut pilot = reader("lissajous", "ja");
        pilot.navigate(ReaderCommand::Mathematics);
        draw(&mut pilot, (360, 240), InputMode::KeyboardMouse);
        let body = pilot.composition.as_ref().unwrap().body.source();
        assert!(body.starts_with("日本語訳: 試作版\n"));
        pilot.navigate(ReaderCommand::Select(StudyDepth::Notes));
        draw(&mut pilot, (360, 240), InputMode::KeyboardMouse);
        let body = pilot.composition.as_ref().unwrap().body.source();
        assert!(body.contains("この節は英語です。"));
        assert!(!body.contains("試作版"));

        let mut fallback = reader("times-tables", "ja");
        draw(&mut fallback, (360, 240), InputMode::KeyboardMouse);
        let body = fallback.composition.as_ref().unwrap().body.source();
        assert!(body.starts_with("日本語訳がないため、英語で表示します。\n"));
        assert!(!body.contains("この節は英語です。"));
        assert!(!body.contains("試作版"));
    }

    #[test]
    fn a_first_mouse_click_survives_the_controller_footer_redraw() {
        let mut reader = reader("lissajous", "en");
        draw(&mut reader, (360, 240), InputMode::Controller);
        let target = center(reader.composition.as_ref().unwrap().geometry.tabs[2]);
        reader.pointer_down(target);
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        assert_eq!(reader.pointer_up(target), ReaderIntent::None);
        assert_eq!(reader.depth(), StudyDepth::Mathematics);
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        assert!(
            !reader
                .composition
                .as_ref()
                .unwrap()
                .body
                .source()
                .contains("Written so far for"),
            "a room that has the depth must not be told where else to look"
        );
        draw(&mut reader, (360, 240), InputMode::Controller);
        let target = center(reader.composition.as_ref().unwrap().geometry.back);
        reader.pointer_down(target);
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        assert_eq!(reader.pointer_up(target), ReaderIntent::Close);
    }

    #[test]
    fn opening_releases_and_changed_geometry_cannot_activate_a_reader_button() {
        let mut reader = reader("lissajous", "en");
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        let target = center(reader.composition.as_ref().unwrap().geometry.tabs[2]);
        assert_eq!(reader.pointer_up(target), ReaderIntent::None);
        assert_eq!(reader.depth(), StudyDepth::Explanation);
        reader.pointer_down(target);
        draw(&mut reader, (900, 700), InputMode::KeyboardMouse);
        assert_eq!(reader.pointer_up(target), ReaderIntent::None);
        assert_eq!(reader.depth(), StudyDepth::Explanation);
    }

    #[test]
    fn depth_and_resize_preserve_a_case_sensitive_source_position() {
        let mut reader = reader("lissajous", "en");
        reader.navigate(ReaderCommand::Mathematics);
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        reader.navigate(ReaderCommand::Pages(4));
        let view = reader.composition.as_ref().unwrap();
        let anchor = view.body.source_offset_at_scroll(reader.scroll());
        let source = view.body.source().to_string();
        assert!(anchor > 0);
        reader.navigate(ReaderCommand::Select(StudyDepth::Explanation));
        draw(&mut reader, (900, 700), InputMode::KeyboardMouse);
        reader.navigate(ReaderCommand::Mathematics);
        draw(&mut reader, (900, 700), InputMode::KeyboardMouse);
        let view = reader.composition.as_ref().unwrap();
        assert_eq!(view.body.source(), source);
        assert_eq!(reader.scroll(), view.body.scroll_for_source_offset(anchor));
        let before = reader.scroll();
        reader.navigate(ReaderCommand::Mathematics);
        draw(&mut reader, (900, 700), InputMode::KeyboardMouse);
        assert_eq!(
            reader.scroll(),
            before,
            "reselecting the depth must not restart it"
        );
    }

    #[test]
    fn unavailable_mathematics_and_languages_are_explicit_without_locking_notes() {
        // A room with no authored treatment, so the reader has to say so. Times
        // Tables used to be that example and now has one of its own.
        let mut reader = reader("golden-angle", "haw");
        assert_eq!(reader.document.locale.requested.as_str(), "haw");
        assert_eq!(reader.document.locale.resolved, "en");
        reader.navigate(ReaderCommand::Mathematics);
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        let body = reader.composition.as_ref().unwrap().body.source();
        assert!(body.contains("unavailable in haw"));
        assert!(body.contains("has not been written"));
        assert!(!body.to_lowercase().contains("unlock"));
        // An unwritten depth must read as unwritten, not as broken, so the
        // reader names where the treatment does exist instead of leaving the
        // player to open every room in the catalog looking for one.
        for named in numinous_core::AUTHORED_MATHEMATICS_ROOMS {
            assert!(body.contains(named), "the notice must name {named}: {body}");
        }
        reader.navigate(ReaderCommand::Select(StudyDepth::Notes));
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        assert!(reader.document.has_depth(StudyDepth::Notes));
        assert!(
            !reader
                .composition
                .as_ref()
                .unwrap()
                .body
                .source()
                .contains("has not been written")
        );
    }

    #[test]
    fn fractional_wheel_steps_accumulate_and_hostile_values_cannot_escape_bounds() {
        let mut reader = reader("lissajous", "en");
        reader.navigate(ReaderCommand::Mathematics);
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        let line = reader.composition.as_ref().unwrap().body.line_height();
        for _ in 0..8 {
            reader.navigate(ReaderCommand::Lines(0.125));
        }
        assert!((reader.scroll() - line).abs() < 0.001);
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            reader.navigate(ReaderCommand::Lines(invalid));
            assert!((reader.scroll() - line).abs() < 0.001);
        }
        reader.navigate(ReaderCommand::Pages(i8::MAX));
        let view = reader.composition.as_ref().unwrap();
        assert_eq!(
            reader.scroll(),
            view.body.max_scroll(view.geometry.body.height)
        );
        reader.navigate(ReaderCommand::Pages(i8::MIN));
        assert_eq!(reader.scroll(), 0.0);
    }

    #[test]
    fn revisiting_a_depth_preserves_fractional_scroll_and_the_exact_end() {
        let mut reader = reader("lissajous", "en");
        reader.navigate(ReaderCommand::Mathematics);
        draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
        for position in [ReaderCommand::Lines(1.5), ReaderCommand::End] {
            reader.navigate(position);
            let before = reader.scroll();
            reader.navigate(ReaderCommand::Select(StudyDepth::Notes));
            draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
            reader.navigate(ReaderCommand::Mathematics);
            draw(&mut reader, (360, 240), InputMode::KeyboardMouse);
            assert_eq!(reader.scroll(), before);
        }
    }

    #[test]
    fn compact_chrome_leaves_five_readable_body_lines_and_visible_depth_controls() {
        for locale in ["en", "ja"] {
            for mode in [InputMode::KeyboardMouse, InputMode::Controller] {
                let mut reader = reader("lissajous", locale);
                draw(&mut reader, (360, 240), mode);
                let view = reader.composition.as_ref().unwrap();
                assert_eq!(
                    view.body.line_height(),
                    25.0,
                    "the 14 px scientific body must not shrink"
                );
                assert_eq!(
                    view.back.line_height(),
                    21.0,
                    "compact chrome uses separate 12 px metrics"
                );
                assert!(view.geometry.body.height as f32 >= 5.0 * view.body.line_height());
                assert_eq!(view.footer.visual_line_count(), 1);
                assert_eq!(
                    view.tabs.iter().map(|tab| tab.source()).collect::<Vec<_>>(),
                    if locale == "ja" {
                        vec!["説明", "補足", "数学"]
                    } else {
                        vec!["Explain", "Notes", "Mathematics"]
                    }
                );
                for (layout, rect) in [
                    (&view.back, view.geometry.back),
                    (&view.language, view.geometry.language),
                    (&view.title, view.geometry.title),
                    (&view.tabs[0], view.geometry.tabs[0]),
                    (&view.tabs[1], view.geometry.tabs[1]),
                    (&view.tabs[2], view.geometry.tabs[2]),
                ] {
                    assert_eq!(layout.visual_line_count(), 1);
                    assert!(layout.width() <= rect.width as f32);
                    assert!(layout.height() <= rect.height as f32);
                    assert!(layout.missing_glyphs().is_empty());
                    assert!(
                        rect.y >= 0 && rect.y as u32 + rect.height <= view.geometry.body.y as u32
                    );
                }
                if mode == InputMode::KeyboardMouse {
                    assert!(view.back.source().contains("Esc:"));
                    assert!(view.language.source().starts_with("L:"));
                } else {
                    assert!(
                        view.back.source().contains(
                            &ControllerCopy::default().action_token(ControllerAction::Back)
                        )
                    );
                }
            }
        }
    }

    #[test]
    fn compact_remapped_controls_fit_without_sacrificing_body_type_or_pointer_capture() {
        use crate::input_legend::{ControllerButton, ControllerFace};

        let mut mapped = ControllerCopy::empty(ControllerFace::PlayStation);
        for (action, buttons) in [
            (
                ControllerAction::Up,
                [ControllerButton::North, ControllerButton::LeftTrigger],
            ),
            (
                ControllerAction::Down,
                [ControllerButton::East, ControllerButton::RightTrigger],
            ),
            (
                ControllerAction::Left,
                [ControllerButton::LeftThumb, ControllerButton::LeftTrigger2],
            ),
            (
                ControllerAction::Right,
                [
                    ControllerButton::RightThumb,
                    ControllerButton::RightTrigger2,
                ],
            ),
            (
                ControllerAction::Back,
                [ControllerButton::West, ControllerButton::Select],
            ),
        ] {
            for button in buttons {
                mapped.bind(action, button);
            }
        }
        let mut long_back = ControllerCopy::empty(ControllerFace::PlayStation);
        for button in ControllerButton::ALL.into_iter().skip(2) {
            long_back.bind(ControllerAction::Back, button);
        }
        for copy in [
            mapped,
            long_back,
            ControllerCopy::empty(ControllerFace::Generic),
        ] {
            for locale in ["en", "ja"] {
                let mut reader = reader("lissajous", locale);
                reader.navigate(ReaderCommand::Mathematics);
                reader
                    .render(360, 240, InputMode::Controller, copy)
                    .unwrap();
                reader.navigate(ReaderCommand::Lines(1.5));
                let before = reader.scroll();
                let view = reader.composition.as_ref().unwrap();
                assert_eq!(view.body.line_height(), 25.0);
                assert!(view.geometry.body.height >= 100);
                assert!(view.footer.visual_line_count() <= 2);
                assert!(view.footer.width() <= view.geometry.footer.width as f32);
                assert_eq!(view.back.visual_line_count(), 1);
                assert!(view.back.width() <= view.geometry.back.width as f32);
                assert!(
                    view.back
                        .source()
                        .contains(&copy.action_token(ControllerAction::Back))
                );
                for action in [
                    ControllerAction::Up,
                    ControllerAction::Down,
                    ControllerAction::Left,
                    ControllerAction::Right,
                ] {
                    assert!(view.footer.source().contains(&copy.action_token(action)));
                }
                let back = center(view.geometry.back);
                reader.pointer_down(back);
                reader
                    .render(360, 240, InputMode::KeyboardMouse, copy)
                    .unwrap();
                assert_eq!(
                    reader.scroll(),
                    before,
                    "a control hint change must retain fractional scroll"
                );
                assert_eq!(reader.pointer_up(back), ReaderIntent::Close);
            }
        }
    }
}
