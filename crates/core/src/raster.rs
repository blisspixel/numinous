//! An RGBA pixel raster: the image (PNG) surface.
//!
//! Rooms draw into a [`Raster`] through the same [`Surface`] trait they use for
//! ASCII, so one `render` method produces both the terminal view and a real
//! image. Rendering is on the CPU, deterministic, and needs no GPU, so it is
//! fully testable. Marks are drawn additively on a near-black stage, so
//! overlapping strokes glow (see `docs/VISUALS.md`).

use crate::surface::{MAX_DIM, Surface};

/// The near-black background (the Numinous stage).
const BACKGROUND: [u8; 3] = [10, 11, 15];

/// The accent used when a room does not specify one.
const DEFAULT_ACCENT: [u8; 3] = [36, 120, 180];

/// Scale a color by `factor`, clamping each channel to 255.
fn scale(color: [u8; 3], factor: f32) -> [u8; 3] {
    let ch = |c: u8| (f32::from(c) * factor).round().clamp(0.0, 255.0) as u8;
    [ch(color[0]), ch(color[1]), ch(color[2])]
}

/// A fixed-size RGB pixel buffer that rooms draw into, in a room's accent color.
#[derive(Debug, Clone)]
pub struct Raster {
    width: usize,
    height: usize,
    accent: [u8; 3],
    pixels: Vec<[u8; 3]>,
}

impl Raster {
    /// Create a raster filled with the background color, using the default accent.
    ///
    /// Each dimension is clamped to a safe maximum so any request is safe.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self::with_accent(width, height, DEFAULT_ACCENT)
    }

    /// Create a raster that draws in the given accent color.
    #[must_use]
    pub fn with_accent(width: usize, height: usize, accent: [u8; 3]) -> Self {
        let width = width.min(MAX_DIM);
        let height = height.min(MAX_DIM);
        Self {
            width,
            height,
            accent,
            pixels: vec![BACKGROUND; width * height],
        }
    }

    /// Import a tightly packed RGBA frame while retaining an accent for later
    /// interface drawing. Alpha is ignored because Numinous frames are opaque.
    ///
    /// Returns `None` when dimensions exceed the shared surface bound, their
    /// byte size overflows, or the slice length does not exactly match.
    #[must_use]
    pub fn from_rgba(width: usize, height: usize, accent: [u8; 3], rgba: &[u8]) -> Option<Self> {
        if width > MAX_DIM || height > MAX_DIM {
            return None;
        }
        let expected = width.checked_mul(height)?.checked_mul(4)?;
        if rgba.len() != expected {
            return None;
        }
        let pixels = rgba
            .chunks_exact(4)
            .map(|pixel| [pixel[0], pixel[1], pixel[2]])
            .collect();
        Some(Self {
            width,
            height,
            accent,
            pixels,
        })
    }

    /// The color added for a mark: semantic interface colors plus four
    /// spectral inks that rooms can combine additively for prismatic light.
    fn ink(&self, mark: char) -> [u8; 3] {
        match mark {
            '#' => scale(self.accent, 1.7),
            '!' => [230, 72, 72],
            '-' => [16, 20, 34],
            '@' => [216, 40, 190],
            '%' => [56, 224, 132],
            '&' => [242, 148, 36],
            '~' => [116, 72, 232],
            _ => self.accent,
        }
    }

    /// The pixels as a tightly packed RGBA byte buffer (`width * height * 4`),
    /// suitable for PNG encoding. Alpha is always fully opaque.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for p in &self.pixels {
            out.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        out
    }

    /// The number of pixels brighter than the background. Useful for tests.
    #[must_use]
    pub fn lit_count(&self) -> usize {
        self.pixels.iter().filter(|&&p| p != BACKGROUND).count()
    }

    /// Dim every pixel to `keep` percent of its brightness, a backdrop for
    /// overlay text so menus stay legible over busy rooms.
    pub fn dim(&mut self, keep: u32) {
        let keep = keep.min(100);
        for pixel in &mut self.pixels {
            for channel in pixel.iter_mut() {
                *channel = ((u32::from(*channel) * keep) / 100) as u8;
            }
        }
    }

    /// Dim only the rows from `y0` to `y1` (clamped): a legibility band
    /// behind HUD text, so words stay readable over bright rooms.
    pub fn dim_rows(&mut self, y0: i32, y1: i32, keep: u32) {
        let keep = keep.min(100);
        let from = y0.max(0) as usize;
        let to = (y1.max(0) as usize).min(self.height);
        for y in from..to {
            for x in 0..self.width {
                let pixel = &mut self.pixels[y * self.width + x];
                for channel in pixel.iter_mut() {
                    *channel = ((u32::from(*channel) * keep) / 100) as u8;
                }
            }
        }
    }

    /// Reset a horizontal band to the stage background.
    ///
    /// This gives dense interface copy a quiet surface instead of asking it to
    /// compete with a bright room. Bounds are clamped to the raster.
    pub fn clear_rows(&mut self, y0: i32, y1: i32) {
        let from = y0.max(0) as usize;
        let to = (y1.max(0) as usize).min(self.height);
        for y in from..to {
            for x in 0..self.width {
                self.pixels[y * self.width + x] = BACKGROUND;
            }
        }
    }

    /// Replace this raster's pixels from an RGBA byte buffer (alpha ignored;
    /// extra or missing bytes are tolerated). Brings a post-processed frame,
    /// for example a visual era, back onto a raster.
    pub fn set_rgba(&mut self, rgba: &[u8]) {
        for (pixel, bytes) in self.pixels.iter_mut().zip(rgba.chunks_exact(4)) {
            *pixel = [bytes[0], bytes[1], bytes[2]];
        }
    }

    /// A `width` x `height` copy of this raster where each source pixel
    /// covers a `factor` x `factor` block (nearest neighbor, never blended).
    ///
    /// The live app view renders heavy rooms below window resolution and
    /// expands them with this before the HUD draws, so interface text stays
    /// window-crisp while only room pixels trade sharpness for motion. The
    /// accent carries over so chrome drawn on the result matches chrome drawn
    /// on a full-resolution render.
    ///
    /// The output is exactly the requested size, whatever the source size:
    /// blocks at the right and bottom edges are partial when the dimensions
    /// are not factor multiples, output beyond the scaled source repeats the
    /// nearest edge pixel, and output smaller than the scaled source is a
    /// top-left crop. Requested dimensions are clamped to the same safe
    /// maximum as every raster; a zero-size source stays background.
    #[must_use]
    pub fn upscaled(&self, factor: usize, width: usize, height: usize) -> Raster {
        let factor = factor.max(1);
        let mut out = Raster::with_accent(width, height, self.accent);
        if self.width == 0 || self.height == 0 {
            return out;
        }
        for y in 0..out.height {
            let sy = (y / factor).min(self.height - 1);
            let row = y * out.width;
            let same_source_row = y > 0 && sy == ((y - 1) / factor).min(self.height - 1);
            if same_source_row {
                out.pixels.copy_within(row - out.width..row, row);
                continue;
            }
            for x in 0..out.width {
                let sx = (x / factor).min(self.width - 1);
                out.pixels[row + x] = self.pixels[sy * self.width + sx];
            }
        }
        out
    }

    /// Copy another raster's pixels into this one with its top-left at `(x, y)`,
    /// clipping anything that falls outside. Used to tile rooms into a sheet.
    pub fn blit(&mut self, other: &Raster, x: usize, y: usize) {
        for oy in 0..other.height {
            let ty = y + oy;
            if ty >= self.height {
                break;
            }
            for ox in 0..other.width {
                let tx = x + ox;
                if tx >= self.width {
                    break;
                }
                self.pixels[ty * self.width + tx] = other.pixels[oy * other.width + ox];
            }
        }
    }
}

impl Surface for Raster {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn plot(&mut self, x: i32, y: i32, mark: char) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x < self.width && y < self.height {
            let add = self.ink(mark);
            let pixel = &mut self.pixels[y * self.width + x];
            for i in 0..3 {
                pixel[i] = pixel[i].saturating_add(add[i]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BACKGROUND, Raster};
    use crate::surface::Surface;

    #[test]
    fn new_raster_is_background() {
        let r = Raster::new(4, 4);
        assert_eq!(r.width(), 4);
        assert_eq!(r.lit_count(), 0);
    }

    #[test]
    fn plot_brightens_a_pixel_additively() {
        let mut r = Raster::new(4, 4);
        r.plot(1, 1, '*');
        assert_eq!(r.lit_count(), 1);
        r.plot(1, 1, '*'); // additive: brighter, still one lit pixel
        assert_eq!(r.lit_count(), 1);
    }

    #[test]
    fn semantic_warning_ink_is_distinct_from_structure_and_accent() {
        let raster = Raster::with_accent(4, 4, [40, 210, 90]);

        assert_eq!(raster.ink('!'), [230, 72, 72]);
        assert_ne!(raster.ink('!'), raster.ink('.'));
        assert_ne!(raster.ink('!'), raster.ink('-'));
        assert_ne!(raster.ink('!'), raster.ink('#'));
    }

    /// The block character `to_mono` gives a cell filled with one flat color.
    ///
    /// Asked of the renderer rather than worked out from its thresholds, so
    /// this cannot drift away from what a `NO_COLOR` player is actually shown.
    fn mono_glyph(color: [u8; 3]) -> char {
        let mut raster = Raster::new(1, 2);
        raster.set_rgba(&[
            color[0], color[1], color[2], 255, color[0], color[1], color[2], 255,
        ]);
        crate::ansi::to_mono(&raster)
            .chars()
            .next()
            .expect("to_mono draws at least one cell")
    }

    /// Every room whose source draws with a given mark, as `(id, accent)`.
    ///
    /// Read from the room sources rather than kept as a list here. A list would
    /// be a second copy of something the code already knows, and second copies
    /// drift: a room could start using the warning ink and this check would go
    /// on testing the four rooms that used it when the list was written.
    /// A source that draws with the mark but yields no room id is a failure of
    /// this scan, not a file to pass over. Skipping it quietly would be the
    /// exact shape of hole the scan exists to close: a room could start using
    /// the ink, go unparsed, and the catalog would still look clean.
    fn rooms_drawing_with(mark: char) -> Vec<(String, [u8; 3])> {
        rooms_drawing_with_all(&[mark])
    }

    /// Sources that draw on another room's raster, and the room they draw on.
    ///
    /// These are the engineered-aha overlays. They declare no id of their own
    /// because they are not rooms; they paint over one, in that room's accent,
    /// so that is the accent their marks have to be legible against.
    ///
    /// Written down rather than guessed from the file name, and checked below:
    /// an entry naming a room that does not exist fails, and a new helper that
    /// reaches for a mark fails the scan until it is listed here.
    const HELPER_ROOMS: [(&str, &str); 2] = [
        ("buffon_aha", "buffon-needle"),
        ("times_tables_aha", "times-tables"),
    ];

    fn helper_parent(stem: &str) -> Option<&'static str> {
        HELPER_ROOMS
            .iter()
            .find(|(helper, _)| *helper == stem)
            .map(|(_, room)| *room)
    }

    #[test]
    fn every_helper_names_a_room_that_exists() {
        for (helper, room) in HELPER_ROOMS {
            assert!(
                crate::registry::room_by_id(room).is_some(),
                "{helper} is recorded as drawing on {room}, which is not a room"
            );
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/rooms")
                .join(format!("{helper}.rs"));
            assert!(
                path.is_file(),
                "{helper} is listed but {path:?} does not exist"
            );
        }
    }

    /// Every room whose source draws with all of `marks`, as `(id, accent)`.
    ///
    /// Same scan, same refusal to pass over what it cannot read. Asking for
    /// several marks at once answers the question that matters for a ramp:
    /// which rooms draw two levels that a player has to tell apart.
    fn rooms_drawing_with_all(marks: &[char]) -> Vec<(String, [u8; 3])> {
        let literals: Vec<String> = marks.iter().map(|mark| format!("'{mark}'")).collect();
        let literal = literals.join(" and ");
        let mut pending = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rooms")];
        let mut found = Vec::new();
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("the rooms directory is readable") {
                let path = entry.expect("a readable directory entry").path();
                // Rooms with several files keep them in a subdirectory. Walking
                // only the top level would leave those permanently unchecked.
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().is_none_or(|extension| extension != "rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).expect("a readable room source");
                if !literals.iter().all(|wanted| source.contains(wanted)) {
                    continue;
                }
                // The id the room declares, so the mapping never depends on a
                // file name happening to match it.
                let stem = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .expect("a room source has a readable name");
                let id = source
                    .split_once("id: \"")
                    .and_then(|(_, rest)| rest.split_once('"'))
                    .map(|(id, _)| id)
                    .or_else(|| helper_parent(stem))
                    .unwrap_or_else(|| {
                        panic!(
                            "{path:?} draws with {literal} and declares no room id, so this scan \
                             cannot tell which accent it is drawn against. If it draws on behalf \
                             of another room, add it to HELPER_ROOMS naming that room; if it \
                             draws nowhere, it should not be reaching for a mark."
                        )
                    });
                let room = crate::registry::room_by_id(id)
                    .unwrap_or_else(|| panic!("{id} is declared in {path:?} but not registered"));
                found.push((id.to_string(), room.meta().accent));
            }
        }
        found.sort();
        found
    }

    #[test]
    fn the_warning_ink_survives_the_color_free_renderer_in_every_room_that_uses_it() {
        // `'!'` is the one ink that carries meaning rather than beauty: it says
        // this cell is wrong. The test above proves it is a different color
        // from the accent, which is not the same as being a different thing to
        // look at. Accents vary per room, and 81 of the 354 sit within 12
        // luminance of the warning ink, so "distinct color" can quite easily
        // mean "identical grey".
        //
        // What has to hold is that a player who cannot use color still sees the
        // warning, and the renderer that player is given is `to_mono`. So the
        // question is put to `to_mono` directly: does the warning cell come out
        // as a different block character than the ordinary one?
        let rooms = rooms_drawing_with('!');
        assert!(
            rooms.len() >= 4,
            "only {} rooms found drawing with the warning ink, so the scan is broken \
             rather than the catalog being clean",
            rooms.len()
        );

        let warning = mono_glyph(Raster::new(1, 1).ink('!'));
        for (id, accent) in rooms {
            let raster = Raster::with_accent(1, 1, accent);
            for ordinary in ['.', '#'] {
                let against = mono_glyph(raster.ink(ordinary));
                assert_ne!(
                    warning, against,
                    "in {id} a warning cell and an ordinary {ordinary:?} cell are the same \
                     character without color, so the warning is carried by hue alone"
                );
            }
        }
    }

    /// Rooms that draw both `'#'` and `'*'` and whose accent makes the two the
    /// same character once color is gone.
    ///
    /// `'#'` is the accent at 1.7, and every other ordinary mark is the accent
    /// itself, so a room drawing both is drawing two levels. Rooms use that as
    /// a depth: in `burning-ship` `'#'` is the interior of the set and `'*'` is
    /// a point that escaped late, and in `josephus` it is how far through the
    /// elimination a seat was. That is the picture's own information, not
    /// decoration.
    ///
    /// It survives `to_mono` in most rooms and not in these, for two reasons
    /// that pull in opposite directions. A bright accent multiplied by 1.7
    /// clamps, so both levels arrive at full and both read as a solid block. A
    /// dark accent multiplied by 1.7 is still dark, so both land in the
    /// faintest shade. Either way a player without color sees one level where
    /// the room drew two.
    ///
    /// This is a record of a real limitation, not a permission slip. The test
    /// below fails if the list grows, if an entry stops colliding and is not
    /// removed, or if a room outside it starts colliding. Fixing it means
    /// changing either the ink scale or the shade thresholds, and both change
    /// what all 354 rooms look like, so it is a decision about the product
    /// rather than a defect to patch. Tracked in `docs/ROADMAP.md` under 0.5
    /// Sensory.
    const MARK_LEVELS_COLLAPSE_WITHOUT_COLOR: [&str; 18] = [
        "attention",
        "burning-ship",
        "dla-frost",
        "gamblers-ruin",
        "goldbach",
        "henon-heiles",
        "hofstadter-q",
        "josephus",
        "kepler-laws",
        "liouville",
        "magnet-fractal",
        "moser-debruijn",
        "rabi",
        "ruler-function",
        "seifert",
        "sinai-billiard",
        "twin-primes",
        "zipf",
    ];

    #[test]
    fn two_drawn_levels_stay_two_levels_without_color_outside_the_known_list() {
        // Scanned once and used twice. Reading the sources again for the count
        // would be the same question asked of the disk a second time, and two
        // answers that are meant to agree are two answers that can differ.
        let drawing_both = rooms_drawing_with_all(&['#', '*']);

        // Proof the scan looked at something. A scan that found no room drawing
        // both marks would report a clean catalog by never having looked.
        assert!(
            drawing_both.len() > 50,
            "only {} rooms found drawing both marks, so the scan is broken \
             rather than the catalog being simple",
            drawing_both.len()
        );

        let mut colliding: Vec<String> = drawing_both
            .into_iter()
            .filter(|(_, accent)| {
                let raster = Raster::with_accent(1, 1, *accent);
                mono_glyph(raster.ink('#')) == mono_glyph(raster.ink('*'))
            })
            .map(|(id, _)| id)
            .collect();
        colliding.sort();

        let known: Vec<String> = MARK_LEVELS_COLLAPSE_WITHOUT_COLOR
            .iter()
            .map(|id| (*id).to_string())
            .collect();
        let newly: Vec<&String> = colliding.iter().filter(|id| !known.contains(id)).collect();
        assert!(
            newly.is_empty(),
            "these rooms newly lose a level without color and must be fixed or tracked: {newly:?}"
        );
        let fixed: Vec<&String> = known.iter().filter(|id| !colliding.contains(id)).collect();
        assert!(
            fixed.is_empty(),
            "these no longer collide and must leave MARK_LEVELS_COLLAPSE_WITHOUT_COLOR: {fixed:?}"
        );
    }

    #[test]
    fn an_accent_can_collide_with_the_warning_ink_and_the_check_would_catch_it() {
        // Proof that the check above can fail. Without this it might be passing
        // because `mono_glyph` returns the same character for everything, and a
        // check that cannot fail is not a check.
        let warning = mono_glyph(Raster::new(1, 1).ink('!'));
        let colliding = Raster::with_accent(1, 1, [230, 72, 72]);
        assert_eq!(
            warning,
            mono_glyph(colliding.ink('.')),
            "an accent equal to the warning ink must be indistinguishable from it"
        );
        // And that it is not returning one character for every input.
        assert_ne!(warning, mono_glyph([255, 255, 255]));
        assert_ne!(warning, mono_glyph([0, 0, 0]));
    }

    #[test]
    fn spectral_inks_are_distinct_and_preserve_semantic_marks() {
        let raster = Raster::with_accent(4, 4, [40, 210, 90]);
        let spectral = ['@', '%', '&', '~'].map(|mark| raster.ink(mark));

        for (index, color) in spectral.iter().enumerate() {
            assert_ne!(*color, raster.ink('.'));
            assert_ne!(*color, raster.ink('!'));
            assert!(spectral[index + 1..].iter().all(|other| other != color));
        }
        assert_eq!(raster.ink('!'), [230, 72, 72]);
        assert_eq!(raster.ink('-'), [16, 20, 34]);
    }

    #[test]
    fn plot_clips_out_of_bounds() {
        let mut r = Raster::new(4, 4);
        r.plot(-1, 0, '*');
        r.plot(0, 99, '*');
        assert_eq!(r.lit_count(), 0);
    }

    #[test]
    fn to_rgba_has_four_bytes_per_pixel_and_opaque_alpha() {
        let r = Raster::new(3, 2);
        let bytes = r.to_rgba();
        assert_eq!(bytes.len(), 3 * 2 * 4);
        assert_eq!(bytes[0..3], BACKGROUND);
        assert_eq!(bytes[3], 255);
    }

    #[test]
    fn rgba_import_round_trips_rgb_and_rejects_bad_shapes() {
        let source = [1, 2, 3, 4, 5, 6, 7, 8];
        let raster = Raster::from_rgba(2, 1, [9, 10, 11], &source).expect("valid frame");
        assert_eq!(raster.to_rgba(), [1, 2, 3, 255, 5, 6, 7, 255]);
        assert!(Raster::from_rgba(2, 1, [0; 3], &source[..7]).is_none());
        assert!(Raster::from_rgba(usize::MAX, 1, [0; 3], &[]).is_none());
    }

    #[test]
    fn line_lights_pixels_via_the_shared_bresenham() {
        let mut r = Raster::new(10, 10);
        r.line(0, 0, 9, 9, '#');
        assert!(r.lit_count() >= 10);
    }

    #[test]
    fn with_accent_draws_in_the_given_color() {
        let mut r = Raster::with_accent(2, 2, [200, 0, 0]);
        r.plot(0, 0, '*');
        let bytes = r.to_rgba();
        assert!(bytes[0] > BACKGROUND[0] + 100, "red channel should be lit");
        assert!(
            bytes[2] <= BACKGROUND[2] + 1,
            "blue channel should stay dark"
        );
    }

    #[test]
    fn pixels_have_square_aspect() {
        assert!((Raster::new(4, 4).char_aspect() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn dim_darkens_and_clamps() {
        let mut raster = Raster::new(2, 2);
        raster.plot(0, 0, '#');
        let bright = raster.to_rgba()[0];
        raster.dim(25);
        let dimmed = raster.to_rgba()[0];
        assert!(
            dimmed < bright / 2,
            "should darken hard: {bright} -> {dimmed}"
        );
        raster.dim(150); // clamped to 100: no brightening, no overflow
    }

    #[test]
    fn dim_rows_darkens_only_the_band() {
        let mut raster = Raster::new(2, 4);
        for y in 0..4 {
            raster.plot(0, y, '#');
        }
        let before = raster.to_rgba();
        raster.dim_rows(1, 3, 20);
        let after = raster.to_rgba();
        let px = |buf: &Vec<u8>, y: usize| buf[y * 2 * 4];
        assert_eq!(px(&before, 0), px(&after, 0), "above the band untouched");
        assert!(px(&after, 1) < px(&before, 1), "inside the band darker");
        assert!(px(&after, 2) < px(&before, 2));
        assert_eq!(px(&before, 3), px(&after, 3), "below the band untouched");
        raster.dim_rows(-5, 99, 50); // clamps, never panics
    }

    #[test]
    fn clear_rows_restores_only_the_requested_band() {
        let mut raster = Raster::with_accent(3, 4, [100, 80, 60]);
        for y in 0..4 {
            for x in 0..3 {
                raster.plot(x, y, '#');
            }
        }
        let before = raster.to_rgba();

        raster.clear_rows(1, 3);
        let after = raster.to_rgba();

        assert_eq!(&after[0..12], &before[0..12]);
        assert_eq!(&after[36..48], &before[36..48]);
        for pixel in after[12..36].chunks_exact(4) {
            assert_eq!(pixel, [10, 11, 15, 255]);
        }
    }

    #[test]
    fn upscaled_expands_each_source_pixel_into_a_block() {
        let mut small = Raster::with_accent(2, 2, [200, 40, 40]);
        small.plot(0, 0, '*'); // only the top-left source pixel is lit
        let big = small.upscaled(3, 7, 5);
        assert_eq!(big.width(), 7);
        assert_eq!(big.height(), 5);
        let rgba = big.to_rgba();
        let lit = |x: usize, y: usize| rgba[(y * 7 + x) * 4] > BACKGROUND[0];
        // The lit source pixel covers exactly the 3x3 block at the origin.
        assert!(lit(0, 0) && lit(2, 2), "block interior is lit");
        assert!(!lit(3, 0) && !lit(0, 3), "neighboring blocks stay dark");
        // The partial right/bottom edge repeats the nearest source pixel
        // (source x=1 dark, so the edge is dark) instead of reading out of
        // bounds.
        assert!(!lit(6, 0) && !lit(0, 4));
    }

    #[test]
    fn upscaled_smaller_than_the_scaled_source_is_a_top_left_crop() {
        let mut small = Raster::new(3, 3);
        small.plot(0, 0, '*');
        small.plot(2, 2, '*'); // outside the crop below
        let cropped = small.upscaled(2, 4, 4); // scaled source is 6x6
        assert_eq!(cropped.width(), 4);
        assert_eq!(cropped.height(), 4);
        let rgba = cropped.to_rgba();
        let lit = |x: usize, y: usize| rgba[(y * 4 + x) * 4] > BACKGROUND[0];
        assert!(lit(0, 0) && lit(1, 1), "top-left block survives the crop");
        assert_eq!(
            cropped.lit_count(),
            4,
            "the (2,2) source pixel's block falls wholly outside"
        );
    }

    #[test]
    fn upscaled_factor_one_matches_the_source() {
        let mut small = Raster::new(3, 2);
        small.plot(1, 1, '#');
        let copy = small.upscaled(1, 3, 2);
        assert_eq!(copy.to_rgba(), small.to_rgba());
    }

    #[test]
    fn upscaled_keeps_the_accent_and_survives_degenerate_input() {
        let small = Raster::with_accent(2, 2, [10, 200, 10]);
        let mut big = small.upscaled(0, 4, 4); // factor 0 behaves as 1
        big.plot(0, 0, '*');
        let rgba = big.to_rgba();
        assert!(rgba[1] > 100, "accent green carried to the upscaled raster");
        let empty = Raster::new(0, 3);
        let out = empty.upscaled(2, 4, 4);
        assert_eq!(
            out.lit_count(),
            0,
            "zero-size source upscales to background"
        );
    }

    #[test]
    fn blit_copies_a_tile_and_clips() {
        let mut tile = Raster::new(2, 2);
        tile.plot(0, 0, '*');
        tile.plot(1, 1, '*');
        let mut sheet = Raster::new(4, 4);
        sheet.blit(&tile, 1, 1); // places the two lit pixels at (1,1) and (2,2)
        assert_eq!(sheet.lit_count(), 2);
        sheet.blit(&tile, 3, 3); // partly off the edge: only (3,3) lands
        assert_eq!(sheet.lit_count(), 3);
    }
}
