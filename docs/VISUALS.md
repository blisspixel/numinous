# Visuals: The Rendering & Look Bible

How Numinous is drawn. The rule above all others: **every single frame is screenshot-worthy.** If you pause at a random instant and it is not beautiful, that is a bug. This doc covers the rendering philosophy, the shared pipeline, the technique toolbox, the color and motion systems, and how each Visual Era is rendered. It is written for the Rust + `wgpu` stack in `ARCHITECTURE.md`.

## Philosophy

- **The math draws itself.** We do not paint pictures of math; we render the actual mathematical object, computed live on the GPU. A fractal is not a texture, it is an escape-time field evaluated per pixel this frame. This is why it holds up to infinite zoom and interaction.
- **Lit from within, not lit from above.** The aesthetic is additive light on a near-black stage (see `DESIGN.md`), not flat UI and not photorealism. Think glowing lines and points, HDR bloom, phosphor. The image looks *emissive*.
- **Restraint is the style.** One idea per screen, one accent color per room, generous negative space. Beauty comes from precision and motion, not from clutter or spectacle.
- **Beauty in stillness and in motion.** Both the paused frame and the animation must be gorgeous. Much of the magic lives in smooth, eased, continuous motion at a locked 60fps (120 where the display allows).

## The shared render pipeline

Every room, in every Era, flows through the same stages so the whole app looks like one place and inherits polish for free. Implemented in wgpu/WGSL.

1. **Compute pass (simulate).** The room's math runs as a GPU compute shader into buffers/textures: escape-time fields, reaction-diffusion state, cellular-automata grids, particle positions, N-body steps. Millions of elements, off the CPU.
2. **Scene pass (draw).** The state is rendered: instanced lines and points for 2D fields (the times-table chords, prime dots), raymarched SDFs for 3D/4D objects, full-screen fragment work for domain-colored fields. Output is HDR (values above 1.0 so glow reads).
3. **Post pass (glow and grade).** HDR bloom (bright-pass, blur, composite) for the emissive look, tone-mapping, a subtle unified color grade, optional vignette and film-grain-free dithering to kill banding.
4. **Era filter (skin).** The active Visual Era is applied here as a post-process plus a per-Era draw mode: CRT curvature and scanlines, palette quantization and dithering, phosphor persistence, blueprint grid, or the clean modern glow. Because this is a pipeline stage, one room renders in every Era with no per-room work (see `DESIGN.md` "Visual Eras").
5. **Capture tap.** Still and loop export reads the HDR buffer pre-or-post grade, at arbitrary resolution, deterministically (seeded), for high-quality shareable output.

## The technique toolbox

The rendering vocabulary rooms draw from. All GPU-side.

- **Signed distance fields (SDF) + raymarching.** The workhorse for 3D and 4D scenes: define shapes as distance functions, march rays per pixel. Intuitive to build and blend, and pure math all the way down (reference: Inigo Quilez's articles are the canon). Used for 4D objects, hyperbolic space, smooth organic forms.
- **Smooth-minimum blending.** `smin` to melt SDF shapes into each other for the organic, liquid look (reaction-diffusion coral, L-system growth).
- **Domain coloring.** For complex-valued functions (Riemann zeta, complex maps), map the output's angle to hue and magnitude to brightness, so a whole function becomes one glowing image and its zeros become visible.
- **Additive / HDR line and point rendering.** Thousands to millions of translucent, additively-blended primitives that sum into light where they overlap (the times-table bloom, prime fields, Fourier trails, Galton particles). This *is* the signature look.
- **Feedback buffers (ping-pong).** Render into a texture that decays each frame for trails, phosphor persistence, and the reaction-diffusion and Game-of-Life simulations themselves.
- **Procedural palettes.** Cosine-based palette functions (again the IQ technique) for smooth, tunable, GPU-cheap color ramps that map a scalar (pitch, phase, iteration count) to color.
- **GPU instancing and particles.** One draw call for a million elements; essential for the emergence rooms.

## The color system

Color is data, never decoration. Rules:

- **Near-black stage.** Deep near-black background (around `#0a0b0f`), never pure black, so glow has somewhere to sit.
- **One accent per room.** Each room owns a single signature accent that glows; everything else stays monochrome or low-saturation. A shared cross-room palette keeps the whole app coherent (a room's accent is a value in that shared system, not a random pick).
- **Color carries meaning.** Hue maps to a real quantity, pitch, phase, iteration-to-escape, curvature, so the color *is* information you can read, not styling.
- **Perceptually uniform ramps.** Use perceptually-uniform colormaps (viridis-family) for scalar fields so equal steps in value look like equal steps in color, and so it stays honest.
- **HDR for emission.** Accent values push above 1.0 so bloom makes them glow. This is what sells "lit from within."
- **Accessible by construction.** Every palette is validated for contrast and colorblind-safety (run the `dataviz` skill's validator when building the design system), and every color mapping has a colorblind-safe variant. Never encode meaning in hue alone; pair with brightness or shape.

## Motion design

- **Everything eases.** Nothing snaps. Physical, continuous, momentum-based. Dials have inertia; values glide.
- **Idle "breathing."** A room left alone never freezes; it drifts in a slow, generative, gorgeous idle loop (this is also what makes Watch mode and the Cabinet's live tile-previews work, see `DESIGN.md`).
- **Transitions are dissolves through black.** Room-to-room is a soft cross-dissolve, never a hard cut, always in the near-black.
- **60fps floor, 120 ceiling.** Motion is where the beauty lives; frame drops are a visible defect. Each room declares a GPU cost tier so Watch mode never stacks two heavy rooms (see `ARCHITECTURE.md`).
- **Reduce-motion is real.** A genuine reduced-motion mode (calmer idles, no fast strobing, no aggressive zoom) that stays beautiful, not a degraded fallback.

## Rendering the Visual Eras (concrete)

Each Era is a draw-mode plus a post-filter, all in the pipeline's stage 4. Same room, radically different skin.

- **Teletype.** Character-cell rasterizer: the scene is quantized to a grid of glyphs on green phosphor, with a soft phosphor glow, scanlines, and a blinking cursor. The math rendered as living ASCII.
- **8-bit / CRT.** Hard palette quantization (a strict small palette), a chunky pixel grid, ordered dithering for gradients, then a CRT shader: barrel curvature, scanlines, aperture-grille mask, corner vignette, and bloom. Lovingly-crafted retro, never lazy pixels (the guardrail from `DESIGN.md`).
- **16-bit.** A richer quantized palette, dithering, a cleaner sprite-era feel, slightly softer CRT. The "SNES/Genesis" look.
- **Oscilloscope / vector.** Lines only, no fills, on black, with heavy phosphor persistence (feedback-buffer decay) and additive green/amber glow. The waveform you hear is the waveform you see. Natural home for Lissajous and Fourier.
- **Blueprint.** A drafting look: graph-paper grid, thin ink lines, dimension annotations, ink-on-cyan or ink-on-white. Natural home for the Straightedge & Compass room.
- **Modern (native).** The full HDR additive-glow system described above: bloom, subtle depth, the polished default and the endpoint of the progression.

Each Era also swaps the audio voice to match (chiptune, FM, analog, modern), see `SOUND.md` and `MUSIC.md`. Eras unlock in historical order as the player collects Constants (see `PROGRESSION.md`).

## Per-wing visual identity (so 23 rooms feel varied but unified)

The shared pipeline guarantees coherence; these keep the wings distinct:
- **Emergence:** dense fields and grids, particle clouds, feedback trails. Cellular, alive.
- **Waves & Sound:** flowing lines, oscilloscope-native, waveforms and phase. Fluid.
- **Infinity & Fractals:** deep zoom, domain coloring, raymarched recursion. Vertiginous.
- **Number & Pattern:** points on circles and spirals, chords of light, discrete dots. Crystalline.
- **Shape & Space:** raymarched SDF solids, clean geometry, 3D/4D. Architectural.
- **Chance & Order:** many small particles accumulating into a whole. Statistical, granular.

## Export & capture

- **Deterministic.** Seeded RNG means a captured loop or a shared deep-link reproduces the exact frame the sharer saw (see `ARCHITECTURE.md`).
- **High quality.** Capture reads the HDR buffer and can render above screen resolution for crisp shareable stills and loops (video/GIF), with a single tasteful glyph watermark.
- **Sharing is native, not web.** Exports are real image/video files and reproducible seed strings that reopen in the app. There is no browser build (see `ARCHITECTURE.md`).

## Open questions
1. Bloom approach: physically-based HDR bloom vs. a cheaper stylized glow, per performance budget on integrated GPUs.
2. Deep-zoom precision for Mandelbrot: when to switch from f32 to emulated double-double / perturbation, and whether that forces a dedicated render path.
3. How aggressively the CRT/dither Eras can run on low-end hardware without dropping the 60fps floor.
4. One global grade vs. per-wing grades: how much color identity each wing gets before coherence suffers.
