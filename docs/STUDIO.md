# The Studio

The creative canvas: an expressive graphing calculator fused with a live-coding
instrument and a shader toy. You type a little math, and it instantly *draws*
and *sings*. This is the "Create" posture (see `DESIGN.md`), and it is a core
part of the experience rather than a bonus feature.

## The one-liner

> **A live, forgiving audiovisual math playground where one expression can be
> both a shape and a song.**

Three useful points of comparison:
- **Desmos / a graphing calculator**: type `y = sin(x)`, see it, drag the numbers, add sliders. Instant, visual, tactile, fun.
- **Strudel / TidalCycles**: write terse patterns (`note("c e g")`, euclidean rhythms), layer them, transform them live, and hear generative music evolve.
- **ShaderToy**: for the deep end, write a field over the whole plane and get domain-colored functions, SDFs, and raymarched worlds.

Numinous brings these ideas into one native, offline instrument. Its particular
bet is that **sight and sound can come from the same expression.** Whether that
bet produces a better creative experience must be demonstrated through use and
listening tests, not claimed from the design alone.

## Why it is a pillar

- It is where **"math is fun, non-ironically, seriously" stops being a slogan and becomes something a person made with their own hands.** Consuming beauty (Watch) and poking it (Play) are great; *creating* it is the conversion that sticks.
- It is the deepest expression of **everything is an instrument**: the whole of math becomes a playable, performable surface.
- It is the **authoring layer for the entire game**. The built-in rooms are, at heart, polished Studio programs (see "Rooms are Studio programs," below). The tool we give players is the tool we build the game with. That is how the catalog scales from 25 rooms to hundreds (the Phase 4 mod SDK is really "the Studio, shared").
- It is **infinitely replayable and endlessly shareable**: every creation is text plus deterministic parameters, so it exports as a clip and a `.num` file / `numinous://` link (native, no browser, see `ARCHITECTURE.md`). The first CLI `.num` save/open slice exists for expression plots; exact app reopening, gallery, and fork/remix remain roadmap work.

## The core idea: one expression, two senses

This is the thing that makes the Studio special and not just "Desmos next to a music app."

A single pattern or expression is bound to **both** the visual channels **and** the audio channels at once:

| The expression describes | Drives visually | Drives audibly |
| --- | --- | --- |
| A value over time | position, height, hue, size | pitch (quantized to a scale) |
| A rhythm / sequence | points appearing, pulses, motion | note onsets, drums |
| A ratio | a stable curve, a closing loop | a musical interval |
| A field over the plane | domain color, brightness | a spectral drone / filter sweep |

So when you write a euclidean rhythm, you *see* the beats land and *hear* them at the same instant, from the same code. When you tune a ratio to close a Lissajous curve, it resolves into a consonant chord as it closes. The synesthesia (see `SOUND.md`) is not decoration bolted on; it falls out of the language by construction.

## The ladder: friendly surface, deep floor

The Studio is a ramp, not a cliff. Same tool, radically different ceilings.

- **Level 0, the graphing calculator.** `y = sin(x)`. It draws, glowing, and it sings the curve. A curious newcomer is delighted in ten seconds. This is the whole onboarding.
- **Level 1, make it move.** `y = sin(x + t)` (t is time) and it animates. Free variables auto-spawn **sliders**; every number is **draggable** (Desmos-style scrubbing). Parametric, polar, and 3D toggles. Now it is alive.
- **Level 2, mathematical patterns.** `note("c e g")`, `euclid(3, 8)`, layered and transformed live (`rev`, `fast`, `slow`, `every`, `degrade`). The pattern drives sound *and* geometry together. Now it is an instrument and a generative visual at once, algorithmic techno you can see (see `MUSIC.md`).
- **Level 3, fields and shaders.** Write an expression over the whole plane for domain coloring and SDFs, or drop into raw **WGSL** for full control (see `VISUALS.md`). Now it is a shader toy with a soundtrack.

A player can stop at any level and have made something real and beautiful.

## Rooms are Studio programs

The unifying architectural idea. There are two ways to author a room, and they meet in the same engine:

- **The Studio path (high-level, sandboxed):** a room expressed as Studio code/patterns. Fast to write, safe to run untrusted, the path for **community rooms** and for rapidly prototyping first-party ones. This is the Phase 4 mod SDK, it is just the Studio, shared.
- **The Rust `Room` trait (low-level, native):** hand-written Rust + custom WGSL for the heaviest spectacle rooms (deep Mandelbrot, reaction-diffusion), where we want maximum control (see `ARCHITECTURE.md`).

Both compile to the same primitives and the same render/audio pipeline. Practically: we **dogfood**, most rooms start life as Studio programs, and only the few that need it drop to native Rust. This means the Studio is exercised and polished continuously by our own room-building, and community authors get the exact tool we trust.

## The interface (fun, interactive, visual)

- **Live, no run button.** You type and it is already happening. Sub-second feedback is the whole point; it must feel like an instrument, not an IDE (see `ARCHITECTURE.md`).
- **Direct manipulation.** Drag points on the curve, scrub any number by dragging it, tweak auto-generated sliders and knobs. Touch the math, not a form.
- **Forgiving, never punishing.** Incomplete or "wrong" input degrades gracefully into something that still looks or sounds interesting; errors are gentle inline nudges, not red walls. You are encouraged to poke fearlessly (a QoL invariant, see `QUALITY.md`).
- **Multiple representations, one switch.** Graph, parametric, polar, 3D, domain-colored field, and a pattern timeline, flip between how the same expression is shown.
- **Beautiful by default.** Output inherits the full visual system and Visual Eras (see `VISUALS.md`), so even a one-line doodle is screenshot-worthy and can be flipped to 8-bit CRT.
- **Remix culture.** A gallery of example creations you can **fork** and mangle, plus templates. Learning by remixing, not by reading docs.
- **Promote to a room.** A "make this a room" button turns your creation into a shareable, Cabinet-ready room, and snapshots it into Watch / Benchmark rotation.
- **Perform it.** MIDI-in and tempo-sync so the Studio is a live audiovisual instrument you can play for an audience (see `MUSIC.md`).

## Two Studio instruments

The Studio should open with a choice between two related surfaces. Both compile
to the same bounded event graph, so a rhythm seen in one view is the rhythm heard
and edited in the other.

### Formula Jam

Formula Jam grows the expression surface already in the app. It keeps manual
entry, then adds two discovery controls for players who do not know what to type:

- **Random** chooses a complete expression from a curated, tested recipe bank.
  It varies a seed and safe parameters, rather than assembling arbitrary syntax
  that may be ugly, silent, or invalid. The chosen expression remains visible
  and editable, so discovery can turn into understanding.
- **Auto** plays the recipe bank as a calm audiovisual set. Its target dwell is
  about 21 seconds per expression, but changes wait for a musical phrase
  boundary and morph or crossfade instead of cutting abruptly. Any edit pauses
  Auto. A visible control resumes it.
- **Help** is a dismissible overlay, never permanent text over the artwork. It
  teaches the small useful vocabulary through playable examples, remembers its
  dismissed state, and returns on demand. Reduced-motion mode replaces spatial
  morphs with a short accessible fade.

Random and Auto are not a substitute for authoring. They are an invitation into
it: watch something compelling, inspect the expression that caused it, change
one number, and make it yours.

### Pattern Studio

Pattern Studio is the music-making surface. It takes inspiration from tracker
workflows, TidalCycles, and Strudel without copying their code or requiring
their syntax. It should be approachable as a toy and deep enough for a real
techno or trance set.

- Six clear musical roles open by default: drums, bass, chords, arpeggio, lead,
  and atmosphere. A beginner can mute, solo, swap a pattern, and turn a few
  meaningful controls before learning any notation.
- Pattern text, a Game Boy-style tracker, a step grid, and a piano roll are
  equivalent views of one event graph. Editing any view updates the others.
- Curated scenes cover intro, build, break, drop, and outro. Mutations stay
  inside declared key, scale, voice role, energy, density, and spectral limits.
  Randomness should preserve musical intent, not merely produce novelty.
- The visual stage offers a cycle spiral, event grid, piano roll, scope, and
  spectrum, then adds Numinous views in which harmony, rhythm, phase, and ratio
  become geometry. Every view reads the same timed events that feed the mixer.
- A short help overlay and playable trance, techno, ambient, and chiptune
  templates let someone start from sound rather than documentation.

The comparison with Strudel is not a replacement claim. Numinous aims at a
different center: native and deterministic playback, direct manipulation for
newcomers, mathematical geometry as a first-class output, and the same bounded
creation document for digital minds and humans.

Numinous does not use Strudel code. It does not copy, adapt, embed, link, or
vendor Strudel. The implementation and language are designed independently in
Rust from mathematical first principles.

Those first principles are cycle, phase, ratio, symmetry, transformation,
probability, geometry, and composition. A cycle may become a rhythm, orbit, or
animation. A ratio may become an interval or a closed curve. A transformation
may act on notes and shapes through the same bounded operator. This shared
formal vocabulary is the universal-language aspiration. It is not a claim that
one musical tradition is universal or that mathematics erases cultural
difference.

The musical ambition is intentionally high: Pattern Studio should be capable of
EDM and trance that stands beside excellent human-made tracks on groove, sound
design, arrangement, tension, release, and replay value. That is a target, not a
current claim. It is earned only through musician-led reference sessions, blind
listening where practical, and repeated evidence across seeds and systems.

### Flagship template: Prime Contact

Pattern Studio should open with one finished example, not an empty sequencer.
**Prime Contact** is a trance track whose musical logic is also a first-contact
signal:

- a clear four-on-the-floor pulse establishes shared time before the piece asks
  the listener to decode anything;
- prime-count phrases such as 2, 3, 5, 7, 11, and 13 shape call and response,
  accents, phrase lengths, and the visual transmission;
- simple frequency ratios establish consonance, then phase and polyrhythm add
  tension before the drop resolves them;
- every counted event appears in the tracker and as geometry, so a listener can
  hear, see, inspect, and mutate the same pattern;
- the arrangement still has to work as excellent trance if nobody notices the
  mathematics. The pattern is structural depth, not a substitute for the song.

The aspiration is shared legibility, not a claim of universal taste. Counting,
ratio, recurrence, and symmetry offer a plausible meeting place for digital
minds, humans, and unfamiliar intelligent beings. Listening and playtests decide
whether the invitation is actually felt.

Prime Contact begins a small built-in repertoire rather than standing alone.
Each included piece needs a distinct electronic style, a real mathematical
structure that can be inspected in the event views, and a complete arrangement
that holds up without explanation. The repertoire is source-shipped and
programmatic, so it works offline, varies from explicit seeds, and remains
editable rather than becoming a folder of opaque recordings.

## One creation document

Manual text, tracker edits, templates, and MCP calls must produce the same
versioned document. The first pattern form should be intentionally small:

```json
{
  "version": 1,
  "seed": 42,
  "tempo": 136,
  "key": "A",
  "scale": "minor",
  "tracks": [
    {
      "id": "kick",
      "role": "kick",
      "pattern": "x...x...x...x...",
      "instrument": "deep-kick",
      "level": 0.8
    }
  ],
  "arrangement": [
    { "bars": 8, "scene": "intro", "energy": 0.25 },
    { "bars": 16, "scene": "drop", "energy": 0.85 }
  ]
}
```

The actual schema lives in core and carries strict bounds for tempo, tracks,
events, duration, polyphony, gain, and evaluation work. MCP should expose a
small composable set of operations: list examples, compose, mutate, preview,
render, and export. It accepts data, not executable code. Every operation takes
an explicit seed, reports the resulting document, and remains reproducible.

The native `.num` document is authoritative. WAV is the simple render baseline,
FLAC is the lossless listening and archive export, and MP3 is the compact sharing
export. MIDI is the broad performance exchange. MusicXML is useful for pitched
material that maps honestly to staff notation, but it should not pretend that
filter automation, timbre, or every percussion gesture is a score. Those stay
in `.num` and the tracker view.

All three faces call the same core composer and offline renderer:

- the app offers create, preview, save, reopen, and export without leaving the
  Studio;
- the CLI accepts a template or `.num` document, an explicit seed, and an output
  path for deterministic WAV, FLAC, MP3, MIDI, or appropriate MusicXML;
- MCP exposes the same bounded compose, mutate, preview, and render operations,
  returning the document and an artifact or resource through the host's agreed
  storage path. It never accepts an arbitrary filesystem write outside that
  capability.

Format parity means the same document and seed produce the same musical events
everywhere. Encoders may differ in representation, but decoded duration,
channels, sample rate, event timing, and declared loudness remain within tested
tolerances. Every lossy or notational export names what it cannot preserve.

## Musical quality gate

"It makes music" is not the bar. Before Pattern Studio earns its milestone:

- representative sessions render deterministically with no clipping, stuck
  notes, discontinuities, or unbounded voices;
- style templates pass musician listening sessions against declared reference
  qualities such as groove, phrasing, bass and kick separation, harmonic
  coherence, tension, release, and transition quality;
- each style has a curated seed set broad enough to catch empty, harsh, muddy,
  or monotonous output, with failures retained as regression cases;
- loudness, peak headroom, spectral balance, phrase boundaries, and render cost
  have automated checks, while taste remains a human judgment;
- co-creation records who changed what, supports undo, and never hides a
  mutation behind a claim of collaboration. Every participant can inspect,
  edit, fork, export, or leave the session.
- Prime Contact passes both sides of its brief: blind listeners rate it as a
  compelling trance track, and informed listeners can recover the declared
  prime structure from the event views without a prose explanation.
- WAV, FLAC, and MP3 exports decode successfully in independent readers, and the
  app, CLI, and MCP produce event-identical renders from the same `.num` document
  and seed.

## A few things you could make in one sitting

- **A blooming sunflower:** one polar line using the golden angle, seeds spiral out and plink in an even rhythm; detune the angle and watch and hear it fall apart.
- **A techno loop you can see:** a euclidean bass pattern drives a Lissajous figure that pulses on the beat, tune the frequency ratio and the figure locks as the chord resolves.
- **Your name, drawn by circles:** feed a drawn path to the Fourier operator and a chain of epicycles redraws it while playing its own spectrum.
- **The Riemann zeta function:** one expression, domain-colored, its zeros lighting up along the critical line, humming.

The point of the examples: the famous built-in rooms are reachable in a few lines of the same language, which is exactly what makes it feel powerful and what makes the whole game moddable.

## Sharing and safety

- **Native sharing:** a creation is text plus deterministic parameters, exported as a clip and a `.num` file / `numinous://` link that reopens it exactly in the app. The first CLI `.num` file/link save and `open-studio` path now exists for expression plots; app reopening, clips, and gallery flow still need to land before this promise is complete (see `ARCHITECTURE.md`, `ROADMAP.md`).
- **Sandboxing (important):** community Studio code is untrusted and must run sandboxed, no filesystem, no network, resource/time limits, GPU work through the safe pipeline only. This is a hard requirement for the mod SDK and a `QUALITY.md` concern (fault injection and fuzzing of untrusted patterns).

## Roadmap position

- **0.3 Tactile Alpha:** Formula Jam gains curated Random, phrase-aligned Auto,
  and the dismissible playable help overlay.
- **0.5 Sensory Alpha:** the shared semantic event graph drives audio and the
  first mathematical visualizers, with mixer, accessibility, performance, and
  listening evidence.
- **0.7 Creator Alpha:** Pattern Studio, the versioned `.num` schema, equivalent
  tracker, grid, text, and piano-roll views, save and reopen, MCP creation, MIDI
  and appropriate MusicXML export, WAV, FLAC, and MP3 rendering, local gallery,
  and fork/remix form one loop across the app, CLI, and MCP.
- **1.x:** first-party room authoring moves onto the same safe Studio substrate
  where it is a good fit.
- **2.0:** the Studio becomes the bounded community creator platform described
  in `EXTENSIBILITY.md`.

## Open questions
1. Bespoke DSL vs. embedding an existing scripting host (Rhai/Lua) with a pattern layer on top: ANSWERED (July 2026, see `EXTENSIBILITY.md`): bespoke, grown from the existing expression engine. The Studio language itself is the sandbox: total, budgeted, hermetic, deterministic, pure Rust, in core. No scripting engine enters the trusted core.
2. How much raw WGSL to expose at Level 3, and how to sandbox it safely for community sharing: ANSWERED for now (see `EXTENSIBILITY.md`): first-party and locally-authored only; untrusted GPU work goes through the safe pipeline exclusively.
3. Live-eval performance: keeping sub-second feedback while an expression drives both a heavy visual and real-time audio.
4. Which familiar pattern ideas justify independent implementation, and which
   would make the language less clear than the direct-manipulation surface.
   No Strudel code is used. Numinous implements its language independently.
