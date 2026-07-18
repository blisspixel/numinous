# Synesthesia and the glow: the sensory seam

`VISUALS.md` owns the eye and `SOUND.md`/`MUSIC.md` own the ear, but nothing owns
the seam where they become one thing. This doc is the seam. It also owns the
honest infrastructure finding from the July 2026 fan-out: the documented HDR glow
pipeline is not yet built, and building it is the single highest-leverage
aesthetic investment. Phase B of `NORTH_STAR.md`.

## The honest starting point

`VISUALS.md` describes a gorgeous HDR additive-glow pipeline (stages: compute,
scene, bright-pass bloom, Era filter, tonemap, capture). The running app does not
have it. Most rooms draw additive 8-bit marks on near-black through the CPU
raster (`crates/core/src/raster.rs`); overlapping strokes brighten, but there is
no true HDR bloom, no phosphor persistence, no tonemap, and the `wgpu` path is
wired only to the Mandelbrot and Julia escape-time fractals. "Lit from within" is
a promise on paper, not a look on screen. This is not a flaw in the design; the
design is right. It is the most important fact for the aesthetics roadmap,
because the fix is systemic: one pipeline lifts all 351 rooms and every Era at
once.

## The signature identity, in one sentence

Numinous is math drawn as light emitted from a lattice of discrete luminous
samples on a near-black stage, where every moving thing writes a decaying
phosphor trail, and a live number is always burned into the frame.

Four locked ingredients. The first ships; the other three are the differentiators
that make a still frame unmistakable:

1. **Near-black stage, one accent, additive HDR emission.** The canvas (shipped
   in spirit, faked in the raster). The upgrade is making it truly HDR: accent
   values push above 1.0 and a real bright-pass bloom makes them glow, rather
   than drawing bright pixels.
2. **The luminous sample lattice (the DNA move).** Promote the character-cell
   heritage from "the Teletype Era" to permanent structural DNA in every Era,
   including Modern. Every render is a field of discrete glowing cells sampled on
   a grid; Modern uses a fine grid with soft bloom, Teletype a coarse grid of
   glyphs, 8-bit a chunky grid with a CRT mask. A quantized, cellular glow reads
   as nobody-but-Numinous, and it is the literal truth of the product (the
   terminal face is cells; the raster is a pixel grid). Keep the lattice faintly
   perceptible even in Modern; that faint grid under the glow is the fingerprint
   that survives cropping and compression.
3. **Phosphor persistence: motion writes light.** A ping-pong feedback buffer in
   every room, not just the vector Era. Everything that moves leaves a decaying
   luminous wake. This makes every still frame screenshot-worthy because the
   still encodes time (the Lorenz butterfly is drawn by its own history), it is
   the cheapest route to beauty-in-stillness, and it is the visual rhyme of
   reverb in the audio. Highest beauty per line of code in the whole plan.
4. **The instrument readout.** A small, precise monospace number always on the
   frame (the `Room::status` line already exists). The frame always looks like
   the readout of a beautiful scientific instrument, and shared clips are legibly
   about math. Treat the readout typography as a brand asset, present in every
   Era.

Together these pass a blind test: near-black, one glowing hue, a cellular sampled
texture, a comet-tail of history, and a number. That is not generic glow; that is
a place.

## The glow pipeline (the highest-leverage build)

Build the GPU post-stack as one systemic pipeline every room renders through:

- HDR offscreen target (Rgba16Float), bright-pass threshold, separable Gaussian
  or dual-Kawase blur, additive composite, ACES or Reinhard tonemap. Standard and
  cheap.
- Feedback persistence: two textures, each frame drawing new content additively
  over a `prev * decay` copy, then swapping. Decay is per room (long for an
  oscilloscope look, short for a cellular one). This is the same ping-pong a
  reaction-diffusion room needs, so building it once serves both.
- Cosine palettes for all scalar-to-color mapping (`color(t) = a + b*cos(2*pi*(c*t
  + d))`, the IQ palette formula): GPU-cheap, perceptually smooth, and each room's
  identity becomes its four-vector color signature, which doubles as the
  Era-relative palette hook.
- The sample-lattice as a post pass: quantize the composited HDR image onto the
  active grid (dot-mask or glyph-atlas lookup), so one shader stage delivers the
  DNA in every Era.

This is portable across the wgpu backends the repo already targets (Vulkan,
Metal, DX12, with CPU fallback), and it unblocks the particle-field rooms, the
reaction-diffusion room, and the CRT Eras that all depend on it.

## The Sensory Bus: one event, two renderings

The synesthesia promise is "sight and sound from the same math." Today a room
implements `render` and `sound` separately and they happen to agree. Make them
structurally incapable of disagreeing: a room emits a single stream of typed
sensory events per frame or step, and both the renderer and the synth consume the
same stream.

- Event vocabulary (small, shared): `Onset { pos, pitch, intensity }`,
  `Drone { value, timbre }`, `Value { field, magnitude }`, `Alignment { closure }`.
- A Lorenz lobe-crossing is one `Onset` that simultaneously drops a bright
  particle (which joins the persistence trail) and triggers a note. They cannot
  drift because they are the same object. A Game-of-Life birth is one event that
  lights a cell and plays its grid-pitched note.
- A shared modulation lattice: a global key, BPM, and a sensory LFO clock that
  both the bloom's idle "breathing" and the audio tremolo read from, so the whole
  app breathes in one time. Pair parameters across channels: hue maps to pitch
  (already a rule), persistence-decay maps to reverb-tail, brightness maps to
  filter cutoff. Sight and sound share not just events but parameters.

Game of Life now ships a narrow room-local precursor, not the general Sensory
Bus. Its exact transition loop writes one birth mask consumed by recent-cell
highlighting and a fixed twelve-row stereo reduction. This closes visual and
sonic source agreement for the presented generation while leaving the shared
event vocabulary, sample-accurate scheduling, independent per-cell onsets, and
cross-room modulation lattice unbuilt.

The rule for every room: find its structural duality and give it to both
channels. Lissajous's ratio is one interval you see and hear (shipped). The
worked example to build first is the Lorenz attractor: the two wings become two
stereo hemispheres and two tonal centers, the z-height drives a filter you see as
trail brightness and hear as timbre, and poking it drops a second detuned voice
that beats apart from the first, the butterfly effect felt in the ears. When
someone can hum the Lorenz attractor, the promise is delivered.

## Render like a cinematographer, not only a graphics engineer

The glow pipeline is the how; this is the why. Beauty is necessary but not
sufficient. Every room should answer one question before its treatment is
chosen: **what emotion is this room?** Wonder, fragility, violence, infinity,
loneliness, chaos, stillness. The shaders, palette, persistence length, motion
cadence, and sound are then selected to communicate that emotion, not merely to
look good. Lorenz is chaos held in a strange, beautiful cage; the double
pendulum is violence; Golden Angle is serene inevitability; Sizes of Infinity
is vertigo. Two rooms with identical glow settings but different emotional
intent are a failure of direction. This is a per-room design decision recorded
alongside the room's palette signature, and it is what turns "a gorgeous
generative-art frame" into "a place with a feeling." The same principle governs
sound: a room's emotion is carried by both channels or neither.

## The Visual Eras as complete worlds

The Eras are a top-tier idea and are currently palette-deep. Make each a coherent
world with eight properties, not a skin: render primitive, palette and
quantization, post chain, synth voice, type and chrome, motion signature (the
missing one: Teletype snaps to discrete cell updates, vector eases with long
persistence, Modern is buttery), transition grammar, and idle behavior. Only
phosphor exists as a real post chain today; 16-bit, blueprint, a distinct
Teletype-vs-vector, and a lovingly-crafted CRT mask are owed, and Plotter (a
visible traveling pen drawing curves in real time) and Demoscene (the Benchmark's
native flex skin) are new complete worlds worth adding cheaply. Adding an Era is
one post chain plus one voice plus one motion profile, not touching 30 rooms,
because rooms already draw in Era-relative terms.

Detailed per-Era specs and The Show (the auto-director, transitions, match-cut
composition, the load-balancer) extend `VISUALS.md` rather than splitting into
their own files, to respect the anti-redundancy map. Note them there as they are
built.

## Roadmap position

- **The Glow Pipeline** (before more rooms): the GPU post-stack, so a random
  paused frame passes the blind "is this Numinous" test. Retires the risk "does
  the documented look actually exist in the product."
- **The Sensory Bus**: rooms emit the shared event stream; ship state-dependent
  motif tension and the hue/pitch and decay/reverb pairings as first payloads;
  Lorenz becomes hummable and never desyncs.
- **GPU field-room port** (Chaos Game, Barnsley Fern, Galton, prime fields,
  Fourier trails) to instanced additive particles, and the reaction-diffusion
  signature room on the ping-pong buffer.
- **Era grain build-out** and the music visualizer (a scorecard gap), rendered
  in the sample lattice with persistence.

## Sources

- IQ cosine palettes: https://iquilezles.org/articles/palettes/
- Reaction-diffusion on WebGPU (ping-pong):
  https://tympanus.net/codrops/2024/05/01/reaction-diffusion-compute-shader-in-webgpu/
- crt-royale CRT shader reference: https://docs.libretro.com/shader/crt_royale/
- Ryoji Ikeda, data as a discrete luminous lattice:
  https://www.ryojiikeda.com/project/testpattern/
- Tetris Effect synesthesia design: https://www.nicholassinger.com/blog/tetriseffect
