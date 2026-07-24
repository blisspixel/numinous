# Visuals: The Rendering & Look Bible

How Numinous is drawn. The rule above all others: **every single frame is
screenshot-worthy.** If you pause at a random instant and it is not beautiful,
that is a bug. This document owns both the current rendering boundary and the
target visual system.

**Implementation boundary, 2026-07-13:** 0.2.0-alpha.1 renders every room
deterministically through CPU `Surface` implementations and presents app frames
with `softbuffer`. Mandelbrot and Julia alone have targeted `wgpu` paths. Four
CPU-styled Eras ship: phosphor, 8-bit, vector, and modern. PNG room renders,
gallery sheets, and app postcards ship. HDR, bloom, feedback persistence, a
universal GPU pipeline, 16-bit and blueprint Eras, audio voice swaps, loop or
video export, and native link reopening are targets, not current evidence.

## Philosophy

- **The math draws itself.** We render the mathematical object rather than a
  prerecorded texture. Most rooms compute on the CPU today; the two shipped
  fractal GPU paths evaluate escape-time fields in WGSL.
- **Lit from within, not lit from above.** The aesthetic is additive light on a near-black stage (see `DESIGN.md`), not flat UI and not photorealism. Think glowing lines and points, HDR bloom, phosphor. The image looks *emissive*.
- **Restraint is the style.** One idea per screen, one accent color per room, generous negative space. Beauty comes from precision and motion, not from clutter or spectacle.
- **Beauty in stillness and in motion.** Both the paused frame and the animation must be gorgeous. Much of the magic lives in smooth, eased, continuous motion at a locked 60fps (120 where the display allows).

## Current and target render pipelines

The current shared seam is `Surface`: each room emits deterministic drawing
operations that can become terminal cells or RGBA pixels. The app presents the
RGBA raster, adaptively reducing live resolution when a room exceeds its 33 ms
budget. The GPU adapter can replace the fractal raster for Mandelbrot and Julia
while preserving CPU fallback and deterministic exports. Mandelbrot uses a
smooth escape-time field with a dark interior and a high-energy cyan, lime,
violet, and magenta cosine palette; its native camera keeps advancing after a
click rather than snapping back at a normalized phase boundary. Julia retains
its separate palette and interaction identity.
Times Tables uses five fixed spectral chord families on the shared additive
raster. Their hue identifies source-circle regions, while crossings brighten
naturally. A resolution-aware sample count preserves negative space in ASCII
without changing the 240-point mathematical circle used by full-size raster
frames. Its in-scene dial draws explicit ticks and a bright current marker.

The target systemic GPU post-stack has five stages:

1. **Compute pass:** run only measured simulations or fields that benefit from
   GPU parallelism.
2. **Scene pass:** draw line, point, field, or SDF primitives into an HDR target.
3. **Post pass:** apply bright-pass bloom, tone mapping, and a restrained grade.
4. **Era filter:** express an Era as a shared post-process where that can replace
   per-face duplication without erasing room meaning.
5. **Capture tap:** export deterministic stills and, later, bounded loops from a
   defined pre-grade or post-grade surface.

## Target technique toolbox

These techniques are candidates for the staged GPU system, not a list of
already shipped room implementations.

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
- **One accent per room by default.** Each room owns a signature accent that
  glows. A deliberate spectral mapping may add a small shared palette when hue
  carries real state, as the source-circle families do in Times Tables.
- **Color carries meaning.** Hue maps to a real quantity, pitch, phase, iteration-to-escape, curvature, so the color *is* information you can read, not styling.
- **Perceptually uniform ramps.** Use perceptually-uniform colormaps (viridis-family) for scalar fields so equal steps in value look like equal steps in color, and so it stays honest.
- **HDR for emission, planned.** Accent values above 1.0 will drive the future
  bloom pass. The current raster approximates glow in 8-bit color.
- **Accessible by construction, required.** Validate every palette for contrast
  and color-vision deficiencies, and pair hue with brightness or shape. That
  complete validation has not happened yet and remains in the 0.5 gate.

## Target motion design

Rooms currently animate deterministically from phase, and the app can reduce
live render resolution to protect its 33 ms room budget. The rules below are the
remaining product bar, not claims that every room already satisfies it.

- **Everything eases.** Nothing snaps. Physical, continuous, momentum-based. Dials have inertia; values glide.
- **Idle "breathing."** A room left alone never freezes; it drifts in a slow, generative, gorgeous idle loop (this is also what makes Watch mode and the Cabinet's live tile-previews work, see `DESIGN.md`).
- **Transitions are dissolves through black.** Room-to-room is a soft cross-dissolve, never a hard cut, always in the near-black.
- **Frame budget.** Work toward smooth display pacing on representative
  hardware. The current evidence is the adaptive 33 ms room-render budget on
  one Windows machine, not a universal 60 or 120 fps guarantee.
- **Reduce-motion is real.** A genuine reduced-motion mode (calmer idles, no fast strobing, no aggressive zoom) that stays beautiful, not a degraded fallback.

## Rendering the Visual Eras

Four Eras ship as deterministic CPU styling in the app, CLI, and PNG paths:

- **Phosphor:** a green character-display treatment.
- **8-bit:** a small-palette, chunky-pixel treatment.
- **Vector:** a sparse line-forward treatment inspired by an oscilloscope.
- **Modern:** the native near-black and single-accent raster look.

The fuller CRT effects, feedback persistence, 16-bit and blueprint treatments,
HDR modern pass, and per-Era audio voices are planned. Era progression must not
be described as complete until those visual and sonic variants are both built
and tested.

## Per-wing visual identity (so 353 rooms feel varied but unified)

The shared pipeline guarantees coherence; these keep the wings distinct:
- **Emergence:** dense fields and grids, particle clouds, feedback trails. Cellular, alive.
- **Waves & Sound:** flowing lines, oscilloscope-native, waveforms and phase. Fluid.
- **Infinity & Fractals:** deep zoom, domain coloring, raymarched recursion. Vertiginous.
- **Number & Pattern:** points on circles and spirals, chords of light, discrete dots. Crystalline.
- **Shape & Space:** raymarched SDF solids, clean geometry, 3D/4D. Architectural.
- **Chance & Order:** many small particles accumulating into a whole. Statistical, granular.

## Export & capture

- **Shipped:** deterministic CPU PNG renders, full catalog galleries and contact
  sheets, and app postcards of the live room state.
- **Separate shipped artifacts:** Studio `.num` files and matching links round
  trip through the CLI, but do not reopen in the app yet.
- **Planned:** HDR still capture after that pipeline exists, bounded loop or
  video export, and native application reopening for files and links.

## Open questions
1. Bloom approach: physically-based HDR bloom vs. a cheaper stylized glow, per performance budget on integrated GPUs.
2. Deep-zoom precision for Mandelbrot: when to switch from f32 to emulated double-double / perturbation, and whether that forces a dedicated render path.
3. How aggressively the CRT/dither Eras can run on low-end hardware without dropping the 60fps floor.
4. One global grade vs. per-wing grades: how much color identity each wing gets before coherence suffers.
