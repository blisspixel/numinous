//! Case-preserving study text, shaped and rasterized with bundled fonts only.
//!
//! This is a linear text adapter, not an equation typesetter or a translation
//! service. The reader supplies its resolved content locale and distinguishes
//! prose from mathematical notation. Layouts retain the original UTF-8 source
//! so a reader can preserve a text position when its width changes.

use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use cosmic_text::{
    Align, Attrs, AttrsList, Buffer, CacheKey, Fallback, Family, FontSystem, LayoutGlyph, Metrics,
    Shaping, SwashCache, SwashContent, SwashImage, Wrap, fontdb,
};
use unicode_script::Script;
use unicode_segmentation::UnicodeSegmentation;

const MAX_TEXT_BYTES: usize = 65_536;
const MAX_SPANS: usize = 2_048;
const MAX_GLYPHS: usize = 65_536;
const MAX_LAYOUTS: usize = 4;
const MAX_IMAGE_ENTRIES: usize = 4_096;
const MAX_IMAGE_BYTES: usize = 8 * 1_024 * 1_024;
const MAX_WIDTH: u32 = 4_096;
const FONT_NAMES: [&str; 3] = ["Noto Sans", "Noto Sans JP", "Noto Sans Math"];
const FONT_BYTES: [&[u8]; 3] = [
    include_bytes!("../../../assets/fonts/noto-sans/NotoSans-Regular.ttf"),
    include_bytes!("../../../assets/fonts/noto-sans-jp/NotoSansJP-Regular.otf"),
    include_bytes!("../../../assets/fonts/noto-sans-math/NotoSansMath-Regular.ttf"),
];

/// The preferred font role of a source span, without changing its contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    /// Ordinary prose, with a Japanese script fallback.
    Prose,
    /// Linear mathematical notation, with a dedicated mathematical face.
    Math,
}

/// Borrowed source text and its preferred font role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSpan<'a> {
    /// Exact source, including its case and combining characters.
    pub text: &'a str,
    /// A font preference; it does not alter the source or locale.
    pub role: TextRole,
}

/// A pixel rectangle relative to the complete RGBA frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextViewport {
    /// Left edge, which may lie outside the frame.
    pub x: i32,
    /// Top edge, which may lie outside the frame.
    pub y: i32,
    /// Width in pixels. A zero width draws nothing.
    pub width: u32,
    /// Height in pixels. A zero height draws nothing.
    pub height: u32,
}

/// A source cluster for which none of the bundled faces supplied a glyph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingGlyph {
    /// Extended-grapheme-safe UTF-8 byte range in [`TextLayout::source`].
    pub source_range: Range<usize>,
}

/// Admission or rendering errors from the study text boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextError {
    /// The explicit locale is empty or not a bounded ASCII language tag.
    InvalidLocale,
    /// The embedded font inventory or its metrics could not be loaded.
    BundledFont,
    /// Source exceeds 65,536 bytes or the request exceeds 2,048 spans.
    TextLimit,
    /// A change of font role would divide an extended grapheme cluster.
    SplitGrapheme,
    /// Width is outside 1..=4,096, or font size is not finite in 8..=48 pixels.
    InvalidMetrics,
    /// Shaping exceeded the bounded output or returned invalid measurements.
    LayoutLimit,
    /// A layout belongs to a different [`StudyText`] instance.
    ForeignLayout,
    /// The supplied RGBA length does not match the checked frame dimensions.
    InvalidFrame,
    /// Rasterization returned an unexpected format or an oversized glyph image.
    GlyphImage,
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLocale => "study text requires an explicit bounded language tag",
            Self::BundledFont => "a bundled study font or its metrics could not be loaded",
            Self::TextLimit => "study text exceeds the source or span limit",
            Self::SplitGrapheme => "a font role change divides a grapheme cluster",
            Self::InvalidMetrics => {
                "study text dimensions or font size are outside the admitted range"
            }
            Self::LayoutLimit => "study text shaping exceeded the layout limit",
            Self::ForeignLayout => "study text layout belongs to another renderer",
            Self::InvalidFrame => "study text frame dimensions do not match its RGBA storage",
            Self::GlyphImage => "a study glyph image is outside the bundled renderer contract",
        })
    }
}

impl std::error::Error for TextError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StyledRange {
    source: Range<usize>,
    role: TextRole,
}

#[derive(Debug)]
struct TextLine {
    source_start: usize,
    top: f32,
    baseline: f32,
    ink_top: f32,
    ink_bottom: f32,
    glyphs: Vec<LayoutGlyph>,
}

/// An immutable shaped document, valid for the instance that created it.
///
/// Width measures logical advances; height measures the full stack of line
/// boxes. Font bearings and combining marks can extend outside these boxes,
/// so drawing always intersects the caller's viewport with the actual frame.
#[derive(Debug)]
pub struct TextLayout {
    owner: Arc<()>,
    source: String,
    styles: Vec<StyledRange>,
    graphemes: Vec<usize>,
    wrap_width: u32,
    font_size: f32,
    width: f32,
    height: f32,
    line_height: f32,
    lines: Vec<TextLine>,
    missing: Vec<MissingGlyph>,
}

impl TextLayout {
    /// Original source, without case conversion or Unicode normalization.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Maximum measured logical line advance in pixels.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Full document height in pixels, including empty logical lines.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Line spacing derived from the bundled faces' ascent, descent and leading.
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Number of visual lines after wrapping.
    pub fn visual_line_count(&self) -> usize {
        self.lines.len()
    }

    /// Unsupported source clusters, rather than a claim of complete Unicode coverage.
    pub fn missing_glyphs(&self) -> &[MissingGlyph] {
        &self.missing
    }

    /// Largest useful vertical scroll for this viewport height.
    pub fn max_scroll(&self, viewport_height: u32) -> f32 {
        (self.height - viewport_height as f32).max(0.0)
    }

    /// Source anchor at the topmost line touched by a document scroll position.
    ///
    /// Negative and nonfinite positions resolve to the start. Positions past
    /// the document resolve to its last line. Returned offsets are UTF-8 byte
    /// offsets at extended grapheme boundaries, obtained from shaped clusters.
    pub fn source_offset_at_scroll(&self, scroll_y: f32) -> usize {
        let scroll = finite_scroll(scroll_y);
        let index = self.lines.partition_point(|line| line.top <= scroll);
        self.lines
            .get(index.saturating_sub(1))
            .map_or(0, |line| line.source_start)
    }

    /// Top of the visual line containing a source byte offset.
    ///
    /// Out-of-range offsets clamp to the source length. Interior UTF-8 or
    /// grapheme offsets snap backward to a grapheme boundary. The reader can
    /// then clamp the result with [`Self::max_scroll`] for its new viewport.
    pub fn scroll_for_source_offset(&self, offset: usize) -> f32 {
        let offset = boundary_before(&self.graphemes, offset.min(self.source.len()));
        let index = self
            .lines
            .partition_point(|line| line.source_start <= offset);
        self.lines
            .get(index.saturating_sub(1))
            .map_or(0.0, |line| line.top)
    }
}

#[derive(Debug)]
struct BundledFallback;

impl Fallback for BundledFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &FONT_NAMES
    }

    fn forbidden_fallback(&self) -> &[&'static str] {
        &[]
    }

    fn script_fallback(&self, script: Script, _locale: &str) -> &[&'static str] {
        match script {
            Script::Han | Script::Hiragana | Script::Katakana => &["Noto Sans JP"],
            _ => &[],
        }
    }
}

#[derive(Debug)]
struct FontExtent {
    id: fontdb::ID,
    y_min: f32,
    y_max: f32,
}

/// Bundled-font shaping, four cached layouts, and a bounded glyph image cache.
///
/// This boundary never loads system fonts, accepts font paths, or discovers a
/// locale. The resolved content locale is explicit and remains fixed for the
/// instance. Callers may retain returned `Arc` layouts beyond the four cached
/// entries; those additional allocations belong to the caller.
#[derive(Debug)]
pub struct StudyText {
    owner: Arc<()>,
    fonts: FontSystem,
    extents: Vec<FontExtent>,
    line_em: f32,
    layouts: VecDeque<Arc<TextLayout>>,
    images: SwashCache,
    image_bytes: usize,
}

impl StudyText {
    /// Load the three embedded faces using an explicit resolved content locale.
    ///
    /// The tag must contain 1..=64 ASCII letters, digits or hyphens, with no
    /// empty subtag and an alphabetic first subtag. This is lexical admission,
    /// not registry validation, translation availability, or locale negotiation.
    pub fn new(resolved_locale: &str) -> Result<Self, TextError> {
        if resolved_locale.is_empty()
            || resolved_locale.len() > 64
            || resolved_locale.split('-').enumerate().any(|(index, part)| {
                part.is_empty()
                    || !part.bytes().all(|byte| {
                        if index == 0 {
                            byte.is_ascii_alphabetic()
                        } else {
                            byte.is_ascii_alphanumeric()
                        }
                    })
            })
        {
            return Err(TextError::InvalidLocale);
        }

        // The convenience constructors also load system fonts. Start from an
        // empty database and supply both the locale and fallback explicitly.
        let mut db = fontdb::Database::new();
        for bytes in FONT_BYTES {
            db.load_font_source(fontdb::Source::Binary(Arc::new(bytes)));
        }
        db.set_sans_serif_family("Noto Sans");
        let mut fonts = FontSystem::new_with_locale_and_db_and_fallback(
            resolved_locale.to_owned(),
            db,
            BundledFallback,
        );
        let faces: Vec<_> = fonts
            .db()
            .faces()
            .map(|face| {
                (
                    face.id,
                    face.families
                        .iter()
                        .map(|family| family.0.as_str())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        if faces.len() != FONT_NAMES.len()
            || FONT_NAMES
                .iter()
                .any(|name| !faces.iter().any(|(_, names)| names.contains(name)))
        {
            return Err(TextError::BundledFont);
        }
        let ids: Vec<_> = faces.iter().map(|(id, _)| *id).collect();
        let mut ascent = 0.0_f32;
        let mut descent = 0.0_f32;
        let mut leading = 0.0_f32;
        let mut extents = Vec::with_capacity(ids.len());
        for id in ids {
            let font = fonts
                .get_font(id, fontdb::Weight::NORMAL)
                .ok_or(TextError::BundledFont)?;
            let metrics = font.metrics();
            let units = f32::from(metrics.units_per_em);
            let bounds = metrics.bounds.ok_or(TextError::BundledFont)?;
            if units == 0.0 {
                return Err(TextError::BundledFont);
            }
            ascent = ascent.max(metrics.ascent / units);
            descent = descent.max(-metrics.descent / units);
            leading = leading.max(metrics.leading / units);
            extents.push(FontExtent {
                id,
                y_min: bounds.y_min / units,
                y_max: bounds.y_max / units,
            });
        }
        let line_em = ascent + descent + leading;
        if !line_em.is_finite() || line_em <= 0.0 {
            return Err(TextError::BundledFont);
        }
        Ok(Self {
            owner: Arc::new(()),
            fonts,
            extents,
            line_em,
            layouts: VecDeque::new(),
            images: SwashCache::new(),
            image_bytes: 0,
        })
    }

    /// The supplied resolved content locale. It is never inferred from the OS.
    pub fn locale(&self) -> &str {
        self.fonts.locale()
    }

    /// Shape at most 65,536 UTF-8 bytes and 2,048 spans into a reusable layout.
    ///
    /// Width is 1..=4,096 pixels and font size is finite in 8..=48 pixels.
    /// Words wrap first; an oversized word may wrap between shaped graphemes.
    /// If glyph wrapping divides a grapheme or a glyph cannot fit, word wrapping
    /// retains the indivisible source with its measured advance. That oversized
    /// word may clip at draw time. Font-role changes inside a grapheme are rejected.
    pub fn layout(
        &mut self,
        spans: &[TextSpan<'_>],
        width: u32,
        font_size: f32,
    ) -> Result<Arc<TextLayout>, TextError> {
        if width == 0
            || width > MAX_WIDTH
            || !font_size.is_finite()
            || !(8.0..=48.0).contains(&font_size)
        {
            return Err(TextError::InvalidMetrics);
        }
        let (source, styles, graphemes) = prepare_source(spans)?;
        if let Some(index) = self.layouts.iter().position(|layout| {
            layout.source == source
                && layout.styles == styles
                && layout.wrap_width == width
                && layout.font_size == font_size
        }) && let Some(layout) = self.layouts.remove(index)
        {
            self.layouts.push_back(Arc::clone(&layout));
            return Ok(layout);
        }
        // Shared baselines must accommodate ascent and descent from different
        // faces on the same line. One pixel of leading on each side absorbs
        // raster rounding without estimating text from character counts.
        let line_height = (font_size * self.line_em).ceil() + 2.0;
        let prose = Attrs::new().family(Family::Name("Noto Sans"));
        let math = Attrs::new().family(Family::Name("Noto Sans Math"));
        let mut buffer = Buffer::new_empty(Metrics::new(font_size, line_height));
        buffer.set_size(Some(width as f32), None);
        buffer.set_wrap(Wrap::WordOrGlyph);
        buffer.set_text(&source, &prose, Shaping::Advanced, Some(Align::Left));

        // Use the buffer's own line-ending parser so CRLF and empty trailing
        // lines retain their original source-byte positions across rich spans.
        let mut line_offsets = Vec::with_capacity(buffer.lines.len());
        let mut offset = 0;
        for line in &mut buffer.lines {
            line_offsets.push(offset);
            let end = offset + line.text().len();
            let mut attrs = AttrsList::new(&prose);
            let first_style = styles.partition_point(|style| style.source.end <= offset);
            for style in styles[first_style..]
                .iter()
                .take_while(|style| style.source.start < end)
                .filter(|style| style.role == TextRole::Math)
            {
                let start = style.source.start.max(offset);
                let stop = style.source.end.min(end);
                if start < stop {
                    attrs.add_span(start - offset..stop - offset, &math);
                }
            }
            line.set_attrs_list(attrs);
            offset = end + line.ending().as_str().len();
        }
        buffer.shape_until_scroll(&mut self.fonts, false);
        // The upstream emergency wrap iterates glyphs, which need not be whole
        // graphemes. Keep an oversized cluster together, even on a viewport too
        // narrow to contain it, rather than putting its marks on another line.
        if buffer.layout_runs().any(|run| run.line_w > width as f32)
            || splits_grapheme(&buffer, &line_offsets, &graphemes)
        {
            buffer.set_wrap(Wrap::Word);
            buffer.shape_until_scroll(&mut self.fonts, false);
            if splits_grapheme(&buffer, &line_offsets, &graphemes) {
                return Err(TextError::LayoutLimit);
            }
        }
        let mut lines = Vec::new();
        let mut missing = Vec::new();
        let mut measured_width = 0.0_f32;
        let mut height = 0.0_f32;
        let mut glyph_count = 0_usize;
        for run in buffer.layout_runs() {
            glyph_count += run.glyphs.len();
            if glyph_count > MAX_GLYPHS
                || lines.len() > MAX_TEXT_BYTES
                || ![run.line_top, run.line_y, run.line_height, run.line_w]
                    .iter()
                    .all(|value| value.is_finite())
            {
                return Err(TextError::LayoutLimit);
            }
            let line_offset = line_offsets[run.line_i];
            let start = run
                .glyphs
                .iter()
                .map(|glyph| glyph.start)
                .min()
                .unwrap_or(0);
            let source_start = boundary_before(&graphemes, line_offset + start);
            let mut ink_top = run.line_top;
            let mut ink_bottom = run.line_top + run.line_height;
            for glyph in run.glyphs {
                if glyph.glyph_id == 0 {
                    let start = boundary_before(&graphemes, line_offset + glyph.start);
                    let end = boundary_after(&graphemes, line_offset + glyph.end);
                    missing.push(MissingGlyph {
                        source_range: start..end,
                    });
                }
                let extent = self
                    .extents
                    .iter()
                    .find(|extent| extent.id == glyph.font_id)
                    .ok_or(TextError::BundledFont)?;
                let baseline = run.line_y + glyph.y - glyph.font_size * glyph.y_offset;
                // Font-wide outline bounds plus shaping offsets conservatively
                // cull offscreen rows, including attached combining marks.
                ink_top = ink_top.min(baseline - glyph.font_size * extent.y_max - 2.0);
                ink_bottom = ink_bottom.max(baseline - glyph.font_size * extent.y_min + 2.0);
            }
            measured_width = measured_width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
            lines.push(TextLine {
                source_start,
                top: run.line_top,
                baseline: run.line_y,
                ink_top,
                ink_bottom,
                glyphs: run.glyphs.to_vec(),
            });
        }
        missing.sort_by_key(|entry| (entry.source_range.start, entry.source_range.end));
        missing.dedup();
        let layout = Arc::new(TextLayout {
            owner: Arc::clone(&self.owner),
            source,
            styles,
            graphemes,
            wrap_width: width,
            font_size,
            width: measured_width,
            height,
            line_height,
            lines,
            missing,
        });
        if self.layouts.len() == MAX_LAYOUTS {
            self.layouts.pop_front();
        }
        self.layouts.push_back(Arc::clone(&layout));
        Ok(layout)
    }

    /// Composite a layout into straight-alpha RGBA, intersecting both clips.
    ///
    /// Scroll changes reuse shaping. Negative and nonfinite scroll resolve to
    /// zero; finite scroll is clamped to [`TextLayout::max_scroll`]. The caller's
    /// color alpha multiplies glyph coverage, including on translucent frames.
    /// An invisible viewport or zero-alpha color makes no pixel changes.
    pub fn draw(
        &mut self,
        layout: &TextLayout,
        rgba: &mut [u8],
        dimensions: (usize, usize),
        viewport: TextViewport,
        scroll_y: f32,
        color: [u8; 4],
    ) -> Result<(), TextError> {
        if !Arc::ptr_eq(&self.owner, &layout.owner) {
            return Err(TextError::ForeignLayout);
        }
        let (width, height) = dimensions;
        if width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            != Some(rgba.len())
        {
            return Err(TextError::InvalidFrame);
        }
        let frame_width = i64::try_from(width).map_err(|_| TextError::InvalidFrame)?;
        let frame_height = i64::try_from(height).map_err(|_| TextError::InvalidFrame)?;
        let clip = Clip {
            left: i64::from(viewport.x).max(0),
            top: i64::from(viewport.y).max(0),
            right: (i64::from(viewport.x) + i64::from(viewport.width)).min(frame_width),
            bottom: (i64::from(viewport.y) + i64::from(viewport.height)).min(frame_height),
        };
        if clip.left >= clip.right || clip.top >= clip.bottom || color[3] == 0 {
            return Ok(());
        }
        let scroll = finite_scroll(scroll_y).min(layout.max_scroll(viewport.height));
        for line in &layout.lines {
            if line.ink_bottom - scroll < (clip.top - i64::from(viewport.y)) as f32
                || line.ink_top - scroll >= (clip.bottom - i64::from(viewport.y)) as f32
            {
                continue;
            }
            for glyph in &line.glyphs {
                let physical = glyph.physical((0.0, line.baseline - scroll), 1.0);
                self.cache_image(physical.cache_key)?;
                if let Some(Some(image)) = self.images.image_cache.get(&physical.cache_key) {
                    paint_image(
                        rgba,
                        width,
                        clip,
                        image,
                        i64::from(viewport.x) + i64::from(physical.x),
                        i64::from(viewport.y) + i64::from(physical.y),
                        color,
                    );
                }
            }
        }
        Ok(())
    }

    fn cache_image(&mut self, key: CacheKey) -> Result<(), TextError> {
        if self.images.image_cache.contains_key(&key) {
            return Ok(());
        }
        let image = self.images.get_image_uncached(&mut self.fonts, key);
        let bytes = image.as_ref().map_or(0, |image| image.data.len());
        if let Some(image) = &image
            && (image.content != SwashContent::Mask
                || (image.placement.width as usize).checked_mul(image.placement.height as usize)
                    != Some(bytes)
                || bytes > MAX_IMAGE_BYTES)
        {
            return Err(TextError::GlyphImage);
        }
        if self.images.image_cache.len() >= MAX_IMAGE_ENTRIES
            || self.image_bytes + bytes > MAX_IMAGE_BYTES
        {
            self.images.image_cache.clear();
            self.image_bytes = 0;
        }
        self.image_bytes += bytes;
        self.images.image_cache.insert(key, image);
        Ok(())
    }
}

fn prepare_source(
    spans: &[TextSpan<'_>],
) -> Result<(String, Vec<StyledRange>, Vec<usize>), TextError> {
    if spans.len() > MAX_SPANS {
        return Err(TextError::TextLimit);
    }
    let mut source = String::new();
    let mut styles: Vec<StyledRange> = Vec::new();
    for span in spans {
        if span.text.len() > MAX_TEXT_BYTES - source.len() {
            return Err(TextError::TextLimit);
        }
        let start = source.len();
        source.push_str(span.text);
        if !span.text.is_empty() {
            if let Some(previous) = styles.last_mut()
                && previous.role == span.role
            {
                previous.source.end = source.len();
            } else {
                styles.push(StyledRange {
                    source: start..source.len(),
                    role: span.role,
                });
            }
        }
    }
    let mut graphemes: Vec<_> = source
        .grapheme_indices(true)
        .map(|(offset, _)| offset)
        .collect();
    graphemes.push(source.len());
    if styles
        .iter()
        .any(|style| graphemes.binary_search(&style.source.start).is_err())
    {
        return Err(TextError::SplitGrapheme);
    }
    Ok((source, styles, graphemes))
}

fn boundary_before(boundaries: &[usize], offset: usize) -> usize {
    boundaries[boundaries
        .partition_point(|boundary| *boundary <= offset)
        .saturating_sub(1)]
}

fn splits_grapheme(buffer: &Buffer, line_offsets: &[usize], graphemes: &[usize]) -> bool {
    let mut owners = vec![None; graphemes.len()];
    for (row, run) in buffer.layout_runs().enumerate() {
        let base = line_offsets[run.line_i];
        for glyph in run.glyphs {
            let start = graphemes
                .partition_point(|offset| *offset <= base + glyph.start)
                .saturating_sub(1);
            let end = graphemes.partition_point(|offset| *offset < base + glyph.end);
            for owner in &mut owners[start..end] {
                if owner.is_some_and(|previous| previous != row) {
                    return true;
                }
                *owner = Some(row);
            }
        }
    }
    false
}

fn boundary_after(boundaries: &[usize], offset: usize) -> usize {
    boundaries[boundaries
        .partition_point(|boundary| *boundary < offset)
        .min(boundaries.len() - 1)]
}

fn finite_scroll(scroll: f32) -> f32 {
    if scroll.is_finite() {
        scroll.max(0.0)
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug)]
struct Clip {
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

fn paint_image(
    rgba: &mut [u8],
    frame_width: usize,
    clip: Clip,
    image: &SwashImage,
    x: i64,
    y: i64,
    color: [u8; 4],
) {
    let left = x + i64::from(image.placement.left);
    let top = y - i64::from(image.placement.top);
    let right = (left + i64::from(image.placement.width)).min(clip.right);
    let bottom = (top + i64::from(image.placement.height)).min(clip.bottom);
    for destination_y in top.max(clip.top)..bottom {
        for destination_x in left.max(clip.left)..right {
            let sample = (destination_y - top) as usize * image.placement.width as usize
                + (destination_x - left) as usize;
            let coverage = u32::from(image.data[sample]);
            let alpha = (coverage * u32::from(color[3]) + 127) / 255;
            if alpha == 0 {
                continue;
            }
            let offset = (destination_y as usize * frame_width + destination_x as usize) * 4;
            let destination = &mut rgba[offset..offset + 4];
            let retained = u32::from(destination[3]) * (255 - alpha);
            let denominator = alpha * 255 + retained;
            for channel in 0..3 {
                let numerator = u32::from(color[channel]) * alpha * 255
                    + u32::from(destination[channel]) * retained;
                destination[channel] = ((numerator + denominator / 2) / denominator) as u8;
            }
            destination[3] = ((denominator + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn prose(text: &str) -> [TextSpan<'_>; 1] {
        [TextSpan {
            text,
            role: TextRole::Prose,
        }]
    }

    fn frame(width: usize, height: usize, color: [u8; 4]) -> Vec<u8> {
        color.repeat(width * height)
    }

    fn render(
        text: &mut StudyText,
        layout: &TextLayout,
        color: [u8; 4],
        background: [u8; 4],
    ) -> Vec<u8> {
        let mut rgba = frame(256, 96, background);
        text.draw(
            layout,
            &mut rgba,
            (256, 96),
            TextViewport {
                x: 0,
                y: 0,
                width: 256,
                height: 96,
            },
            0.0,
            color,
        )
        .unwrap();
        rgba
    }

    #[test]
    fn font_inventory_pins_actual_embedded_files_and_only_binary_sources() {
        let expected = [
            "f5f552c8c5edb61fe6efb824baf4d4de47b1a8689ab4925ff43f7bd6a4ebece5",
            "dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073",
            "7283c396e9b22699bb542d9631030dc804a7e5b954f193d8f8f5b5f1162fbc61",
        ];
        let inventory = include_str!("../../../assets/fonts/README.md");
        for (bytes, expected) in FONT_BYTES.into_iter().zip(expected) {
            let digest: String = Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(digest, expected);
            assert!(inventory.contains(expected));
        }
        let text = StudyText::new("ja-JP").unwrap();
        let faces: Vec<_> = text.fonts.db().faces().collect();
        assert_eq!(faces.len(), 3);
        for face in faces {
            let fontdb::Source::Binary(bytes) = &face.source else {
                panic!("a study font came from outside the bundle");
            };
            assert!(FONT_BYTES.contains(&bytes.as_ref().as_ref()));
        }
        assert_eq!(text.locale(), "ja-JP");
        assert!(text.line_em > 1.0);
    }

    #[test]
    fn mixed_study_corpus_has_glyphs_and_math_subscripts_use_bundled_fallback() {
        let mut text = StudyText::new("ja").unwrap();
        let spans = [
            TextSpan {
                text: "自由に遊ぶ。周期と位相を比べる。軌道は長方形の中で稠密になる。\n",
                role: TextRole::Prose,
            },
            TextSpan {
                text: "ʻĀāĒēĪīŌōŪū Hawaiʻi a\u{0304} e\u{0304} i\u{0304} o\u{0304} u\u{0304}\nqQ tlhIngan Hol\n",
                role: TextRole::Prose,
            },
            TextSpan {
                text: "x₂(t)=sin(t), y(t)=cos(t); π ∈ ℝ; 𝑥 ∈ ℂ; ∫₀¹ x² dx = 1/3; |x| ≤ 1",
                role: TextRole::Math,
            },
        ];
        let layout = text.layout(&spans, 320, 16.0).unwrap();
        assert!(
            layout.missing_glyphs().is_empty(),
            "{:?}",
            layout.missing_glyphs()
        );
        assert_eq!(
            layout.source(),
            spans.iter().map(|span| span.text).collect::<String>()
        );
        let subscript = text
            .layout(
                &[TextSpan {
                    text: "₂",
                    role: TextRole::Math,
                }],
                100,
                20.0,
            )
            .unwrap();
        assert!(subscript.missing_glyphs().is_empty());
        let glyph = &subscript.lines[0].glyphs[0];
        let face = text.fonts.db().face(glyph.font_id).unwrap();
        assert!(face.families.iter().any(|(name, _)| name == "Noto Sans"));
        let pixels = render(&mut text, &layout, [235, 240, 245, 255], [0, 0, 0, 255]);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[0] > 0));
    }

    #[test]
    fn unsupported_cluster_is_reported_at_its_source_bytes() {
        let mut text = StudyText::new("en").unwrap();
        let source = "q\u{10ffff}Q";
        let layout = text.layout(&prose(source), 200, 24.0).unwrap();
        assert_eq!(
            layout.missing_glyphs(),
            &[MissingGlyph { source_range: 1..5 }]
        );
        assert_eq!(
            &layout.source()[layout.missing_glyphs()[0].source_range.clone()],
            "\u{10ffff}"
        );
        assert!(
            render(&mut text, &layout, [255; 4], [0; 4])
                .iter()
                .any(|byte| *byte != 0)
        );
    }

    #[test]
    fn hawaiian_combining_macron_stays_attached_in_shaping_wrapping_and_pixels() {
        let mut text = StudyText::new("haw").unwrap();
        let composed = text.layout(&prose("ā"), 200, 32.0).unwrap();
        let decomposed = text.layout(&prose("a\u{0304}"), 200, 32.0).unwrap();
        let plain = text.layout(&prose("a"), 200, 32.0).unwrap();
        assert_eq!(decomposed.source(), "a\u{0304}");
        assert!(decomposed.missing_glyphs().is_empty());
        assert!((composed.width() - decomposed.width()).abs() < 0.001);
        let composed_pixels = render(&mut text, &composed, [255; 4], [0; 4]);
        let decomposed_pixels = render(&mut text, &decomposed, [255; 4], [0; 4]);
        assert_eq!(composed_pixels, decomposed_pixels);
        assert_ne!(
            decomposed_pixels,
            render(&mut text, &plain, [255; 4], [0; 4])
        );

        let source = "a\u{0304}".repeat(12);
        let wrapped = text.layout(&prose(&source), 20, 24.0).unwrap();
        assert_eq!(wrapped.visual_line_count(), 12);
        for line in &wrapped.lines {
            assert_eq!(line.source_start % 3, 0);
            assert_eq!(wrapped.source_offset_at_scroll(line.top), line.source_start);
        }
        for offset in 0..source.len() {
            assert_eq!(
                wrapped.scroll_for_source_offset(offset),
                wrapped.scroll_for_source_offset(offset / 3 * 3)
            );
        }
    }

    #[test]
    fn japanese_wraps_without_spaces_and_keeps_case_sensitive_klingon_distinct() {
        let mut text = StudyText::new("ja").unwrap();
        let source = "線の形を変えて遊ぶ。気になったら説明を開く。周期と位相を比べてみよう。";
        let layout = text.layout(&prose(source), 140, 18.0).unwrap();
        assert!(layout.visual_line_count() >= 4);
        assert!(layout.width() <= 140.01);
        assert!(layout.missing_glyphs().is_empty());
        let boundaries: Vec<_> = source
            .grapheme_indices(true)
            .map(|(offset, _)| offset)
            .collect();
        for line in &layout.lines {
            assert!(boundaries.contains(&line.source_start));
        }
        let lower = text.layout(&prose("q"), 200, 32.0).unwrap();
        let upper = text.layout(&prose("Q"), 200, 32.0).unwrap();
        assert_eq!(lower.source(), "q");
        assert_eq!(upper.source(), "Q");
        assert_ne!(
            lower.lines[0].glyphs[0].glyph_id,
            upper.lines[0].glyphs[0].glyph_id
        );
        assert_ne!(
            render(&mut text, &lower, [255; 4], [0; 4]),
            render(&mut text, &upper, [255; 4], [0; 4])
        );
    }

    #[test]
    fn indivisible_narrow_grapheme_keeps_its_base_and_mark_on_one_line() {
        let mut text = StudyText::new("haw").unwrap();
        // Unlike a with macron, k with macron has no precomposed Unicode scalar.
        // A one-pixel viewport forces the emergency wrap's multiple-glyph case.
        let layout = text.layout(&prose("k\u{0304}"), 1, 24.0).unwrap();
        assert!(layout.missing_glyphs().is_empty());
        assert_eq!(layout.visual_line_count(), 1);
        assert!(layout.lines[0].glyphs.len() >= 2);
        assert!(layout.width() > 1.0);
        assert_eq!(layout.source_offset_at_scroll(0.0), 0);
        assert_eq!(layout.scroll_for_source_offset(2), 0.0);
    }

    #[test]
    fn full_study_equations_and_long_urls_remain_reachable_at_reader_width() {
        let mut text = StudyText::new("ja").unwrap();
        let long_url = format!(
            "https://example.org/references/{}\n",
            "0123456789abcdef".repeat(10)
        );
        let spans = [
            TextSpan {
                text: "Each oscillator phase is a point on a circle. The full state lives on a torus.\n各振動子の位相は円周上の点です。位置が戻っても運動全体が繰り返すとは限りません。\n",
                role: TextRole::Prose,
            },
            TextSpan {
                text: "x(theta)=cos(a*theta+alpha), y(theta)=sin(b*theta+beta), a>0, b>0\nx^2+(x'/a)^2=y^2+(y'/b)^2=1\nS=(x,x'/a,y,y'/b)=(cos(u),-sin(u),sin(v),cos(v))\n(1/L) integral_0^L exp(i*(k*u(theta)+l*v(theta))) dtheta = exp(i*(k*alpha+l*beta))*(exp(i*lambda*L)-1)/(i*lambda*L)\nrho(x,y)=1/(pi^2*sqrt(1-x^2)*sqrt(1-y^2))\n",
                role: TextRole::Math,
            },
            TextSpan {
                text: "https://math.mit.edu/classes/18.353J/PSetAnswers/AnswerPSet_2024_07.pdf\n",
                role: TextRole::Prose,
            },
            TextSpan {
                text: &long_url,
                role: TextRole::Prose,
            },
        ];
        let layout = text.layout(&spans, 324, 16.0).unwrap();
        assert!(layout.height() > 240.0);
        assert!(
            layout.width() <= 324.0,
            "an entire oversized word escaped wrapping"
        );
        assert!(layout.missing_glyphs().is_empty());
        let mut reached = vec![false; layout.source().len()];
        for line in &layout.lines {
            let base = layout.source()[..line.source_start]
                .rfind('\n')
                .map_or(0, |offset| offset + 1);
            for glyph in &line.glyphs {
                assert!(glyph.x >= -0.01 && glyph.x + glyph.w <= 324.01);
                reached[base + glyph.start..base + glyph.end].fill(true);
            }
            let scroll = line.top.min(layout.max_scroll(240));
            assert!(line.top - scroll >= 0.0);
            assert!(line.top + layout.line_height() - scroll <= 240.01);
        }
        for (offset, ch) in layout.source().char_indices() {
            if !ch.is_whitespace() {
                assert!(
                    reached[offset],
                    "unreachable source character at {offset}: {ch}"
                );
            }
        }
        let mut first_page = frame(360, 240, [0; 4]);
        let mut last_page = first_page.clone();
        let viewport = TextViewport {
            x: 18,
            y: 0,
            width: 324,
            height: 240,
        };
        text.draw(
            &layout,
            &mut first_page,
            (360, 240),
            viewport,
            0.0,
            [255; 4],
        )
        .unwrap();
        text.draw(
            &layout,
            &mut last_page,
            (360, 240),
            viewport,
            layout.max_scroll(240),
            [255; 4],
        )
        .unwrap();
        assert_ne!(first_page, last_page);
        assert!(last_page.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn source_anchors_keep_line_endings_and_reading_position_through_resize() {
        let mut text = StudyText::new("ja").unwrap();
        let lines = text.layout(&prose("A\r\nā\nqQ\rあ\n"), 400, 16.0).unwrap();
        assert_eq!(lines.visual_line_count(), 5);
        for (index, offset) in [0, 3, 6, 9, 13].into_iter().enumerate() {
            let scroll = index as f32 * lines.line_height();
            assert_eq!(lines.source_offset_at_scroll(scroll), offset);
            assert_eq!(lines.scroll_for_source_offset(offset), scroll);
        }
        assert_eq!(
            lines.scroll_for_source_offset(4),
            lines.scroll_for_source_offset(3)
        );
        assert_eq!(lines.source_offset_at_scroll(f32::NAN), 0);
        assert_eq!(
            lines.scroll_for_source_offset(usize::MAX),
            4.0 * lines.line_height()
        );

        let source = "a\u{0304} 日本の周期と位相 qQ. ".repeat(30);
        let narrow = text.layout(&prose(&source), 140, 16.0).unwrap();
        let wide = text.layout(&prose(&source), 280, 16.0).unwrap();
        let anchor = narrow.source_offset_at_scroll(7.0 * narrow.line_height());
        assert!(anchor > 0);
        let destination = wide.scroll_for_source_offset(anchor);
        let new_start = wide.source_offset_at_scroll(destination);
        let next_start = wide.source_offset_at_scroll(destination + wide.line_height());
        assert!(new_start <= anchor && anchor < next_start);
        assert_eq!(wide.source(), narrow.source());
        for offset in 0..source.len() {
            let start = wide.source_offset_at_scroll(wide.scroll_for_source_offset(offset));
            assert!(start <= offset);
            assert!(wide.graphemes.contains(&start));
        }
    }

    #[test]
    fn actual_glyph_pixels_obey_intersected_clips_at_both_app_sizes() {
        let mut text = StudyText::new("ja").unwrap();
        let layout = text
            .layout(
                &prose(&"Hawaiʻi の周期 qQ 0123456789 ".repeat(20)),
                300,
                20.0,
            )
            .unwrap();
        let background = [17, 35, 53, 91];
        for (width, height) in [(360, 240), (900, 700)] {
            for (x, y) in [(19, 13), (-9, -7)] {
                let viewport = TextViewport {
                    x,
                    y,
                    width: 171,
                    height: 43,
                };
                let mut clipped = frame(width, height, background);
                let mut reference = clipped.clone();
                text.draw(
                    &layout,
                    &mut reference,
                    (width, height),
                    TextViewport {
                        x,
                        y,
                        width: width as u32 + 100,
                        height: height as u32 + 100,
                    },
                    0.0,
                    [240, 210, 180, 137],
                )
                .unwrap();
                text.draw(
                    &layout,
                    &mut clipped,
                    (width, height),
                    viewport,
                    0.0,
                    [240, 210, 180, 137],
                )
                .unwrap();
                let mut visible_ink = 0;
                let mut excluded_ink = 0;
                for (index, (actual, expected)) in clipped
                    .chunks_exact(4)
                    .zip(reference.chunks_exact(4))
                    .enumerate()
                {
                    let px = (index % width) as i32;
                    let py = (index / width) as i32;
                    let inside = px >= x && px < x + 171 && py >= y && py < y + 43;
                    if inside {
                        assert_eq!(actual, expected, "{width}x{height} at {px},{py}");
                        visible_ink += usize::from(actual != background);
                    } else {
                        assert_eq!(actual, background);
                        excluded_ink += usize::from(expected != background);
                    }
                }
                assert!(visible_ink > 100);
                assert!(excluded_ink > 100);
            }
        }
    }

    #[test]
    fn glyph_coverage_multiplies_source_alpha_and_preserves_straight_rgba() {
        let mut text = StudyText::new("en").unwrap();
        let layout = text.layout(&prose("H"), 200, 32.0).unwrap();
        let full = render(&mut text, &layout, [210, 110, 50, 255], [0; 4]);
        let half = render(&mut text, &layout, [210, 110, 50, 128], [0; 4]);
        let opaque = render(&mut text, &layout, [210, 110, 50, 128], [10, 30, 70, 255]);
        let invisible = render(&mut text, &layout, [210, 110, 50, 0], [10, 30, 70, 91]);
        assert_eq!(invisible, frame(256, 96, [10, 30, 70, 91]));
        let mut solid = 0;
        let mut antialiased = 0;
        for ((full, half), opaque) in full
            .chunks_exact(4)
            .zip(half.chunks_exact(4))
            .zip(opaque.chunks_exact(4))
        {
            assert_eq!(half[3], ((u32::from(full[3]) * 128 + 127) / 255) as u8);
            assert_eq!(opaque[3], 255);
            if half[3] > 0 {
                assert_eq!(&half[..3], &[210, 110, 50]);
            }
            if full[3] == 255 {
                solid += 1;
                for (channel, background) in [10_u32, 30, 70].into_iter().enumerate() {
                    let expected = (u32::from(full[channel]) * 128 + background * 127 + 127) / 255;
                    assert_eq!(u32::from(opaque[channel]), expected);
                }
            } else if full[3] > 0 {
                antialiased += 1;
            }
        }
        assert!(solid > 20);
        assert!(antialiased > 20);
    }

    #[test]
    fn cache_keeps_four_layouts_without_invalidating_a_retained_reading_position() {
        let mut text = StudyText::new("en").unwrap();
        let first = text.layout(&prose("Title"), 200, 16.0).unwrap();
        let body = text
            .layout(
                &prose("A body that remains readable after a cache eviction."),
                120,
                16.0,
            )
            .unwrap();
        text.layout(&prose("Tabs"), 200, 16.0).unwrap();
        text.layout(&prose("Footer"), 200, 16.0).unwrap();
        assert!(Arc::ptr_eq(
            &first,
            &text.layout(&prose("Title"), 200, 16.0).unwrap()
        ));
        let before = render(&mut text, &body, [255; 4], [0; 4]);
        text.layout(&prose("A fifth layout"), 200, 16.0).unwrap();
        assert_eq!(text.layouts.len(), 4);
        assert!(!text.layouts.iter().any(|layout| Arc::ptr_eq(layout, &body)));
        assert_eq!(before, render(&mut text, &body, [255; 4], [0; 4]));
        let reloaded = text.layout(&prose(body.source()), 120, 16.0).unwrap();
        assert!(!Arc::ptr_eq(&body, &reloaded));
        assert_eq!(
            body.source_offset_at_scroll(40.0),
            reloaded.source_offset_at_scroll(40.0)
        );
        assert_eq!(text.layouts.len(), 4);
    }

    #[test]
    fn explicit_locale_and_bundle_produce_the_same_pixels_without_shared_font_ids() {
        let mut first = StudyText::new("ja-JP").unwrap();
        let mut second = StudyText::new("ja-JP").unwrap();
        let spans = prose("Hawaiʻi qQ 周期と位相");
        let a = first.layout(&spans, 240, 20.0).unwrap();
        let b = second.layout(&spans, 240, 20.0).unwrap();
        assert_eq!(a.width(), b.width());
        assert_eq!(a.height(), b.height());
        assert_eq!(
            render(&mut first, &a, [255; 4], [0; 4]),
            render(&mut second, &b, [255; 4], [0; 4])
        );
        let mut pixels = frame(256, 96, [0; 4]);
        assert_eq!(
            second.draw(
                &a,
                &mut pixels,
                (256, 96),
                TextViewport {
                    x: 0,
                    y: 0,
                    width: 256,
                    height: 96,
                },
                0.0,
                [255; 4]
            ),
            Err(TextError::ForeignLayout)
        );
        assert!(pixels.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn admission_is_bounded_and_does_not_split_combining_characters() {
        for locale in ["", "ja_JP", " ja", "en-", "-en", "en--US", "eñ"] {
            assert!(matches!(
                StudyText::new(locale),
                Err(TextError::InvalidLocale)
            ));
        }
        assert!(matches!(
            StudyText::new(&"a".repeat(65)),
            Err(TextError::InvalidLocale)
        ));
        let mut text = StudyText::new("qaa-x-Numinous").unwrap();
        assert_eq!(text.locale(), "qaa-x-Numinous");
        for (width, size) in [
            (0, 16.0),
            (4097, 16.0),
            (200, f32::NAN),
            (200, f32::INFINITY),
            (200, 7.9),
            (200, 48.1),
        ] {
            assert!(matches!(
                text.layout(&prose("abc"), width, size),
                Err(TextError::InvalidMetrics)
            ));
        }
        assert!(matches!(
            text.layout(&prose(&"a".repeat(MAX_TEXT_BYTES + 1)), 200, 16.0),
            Err(TextError::TextLimit)
        ));
        let spans = vec![
            TextSpan {
                text: "",
                role: TextRole::Prose
            };
            MAX_SPANS + 1
        ];
        assert!(matches!(
            text.layout(&spans, 200, 16.0),
            Err(TextError::TextLimit)
        ));
        assert!(matches!(
            text.layout(
                &[
                    TextSpan {
                        text: "a",
                        role: TextRole::Prose
                    },
                    TextSpan {
                        text: "\u{0304}",
                        role: TextRole::Math
                    },
                ],
                200,
                16.0
            ),
            Err(TextError::SplitGrapheme)
        ));
        let same_role = text
            .layout(
                &[
                    TextSpan {
                        text: "a",
                        role: TextRole::Prose,
                    },
                    TextSpan {
                        text: "\u{0304}",
                        role: TextRole::Prose,
                    },
                ],
                200,
                16.0,
            )
            .unwrap();
        assert_eq!(same_role.source(), "a\u{0304}");
        assert!(same_role.missing_glyphs().is_empty());
        assert!(text.layout(&prose("Q"), 1, 8.0).is_ok());
        assert!(text.layout(&prose("Q"), 4096, 48.0).is_ok());
    }

    #[test]
    fn frame_validation_and_offscreen_rectangles_leave_storage_untouched() {
        let mut text = StudyText::new("en").unwrap();
        let layout = text.layout(&prose("Read"), 100, 16.0).unwrap();
        let viewport = TextViewport {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        };
        let mut rgba = frame(32, 32, [10, 20, 30, 40]);
        let before = rgba.clone();
        for dimensions in [(31, 32), (usize::MAX, 2)] {
            assert_eq!(
                text.draw(&layout, &mut rgba, dimensions, viewport, 0.0, [255; 4]),
                Err(TextError::InvalidFrame)
            );
            assert_eq!(rgba, before);
        }
        for viewport in [
            TextViewport {
                x: i32::MAX,
                y: i32::MAX,
                width: u32::MAX,
                height: u32::MAX,
            },
            TextViewport {
                x: i32::MIN,
                y: i32::MIN,
                width: 100,
                height: 100,
            },
            TextViewport {
                x: 0,
                y: 0,
                width: 0,
                height: 32,
            },
            TextViewport {
                x: 0,
                y: 0,
                width: 32,
                height: 0,
            },
        ] {
            text.draw(&layout, &mut rgba, (32, 32), viewport, f32::NAN, [255; 4])
                .unwrap();
            assert_eq!(rgba, before);
        }
    }
}
