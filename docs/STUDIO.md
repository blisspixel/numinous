# The Studio

The creative canvas: a graphing calculator that is fun and creative as hell, fused with a Strudel-style live-coding instrument, fused with a shader toy. You type a little math, and it instantly *draws* and *sings*. This is the "Create" posture (see `DESIGN.md`), and it is a headline pillar, not a bonus feature.

## The one-liner

> **Desmos, if it made music. Strudel, if it drew pictures. A live, forgiving, audiovisual math playground where one expression is simultaneously a shape and a song.**

Three ancestors, one tool:
- **Desmos / a graphing calculator**: type `y = sin(x)`, see it, drag the numbers, add sliders. Instant, visual, tactile, fun.
- **Strudel / TidalCycles**: write terse patterns (`note("c e g")`, euclidean rhythms), layer them, transform them live, and hear generative music evolve.
- **ShaderToy**: for the deep end, write a field over the whole plane and get domain-colored functions, SDFs, and raymarched worlds.

Numinous is the first tool that is all three at once, because in it, **sight and sound are the same expression.**

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

- **Level 0, the graphing calculator.** `y = sin(x)`. It draws, glowing, and it sings the curve. A curious normie is delighted in ten seconds. This is the whole onboarding.
- **Level 1, make it move.** `y = sin(x + t)` (t is time) and it animates. Free variables auto-spawn **sliders**; every number is **draggable** (Desmos-style scrubbing). Parametric, polar, and 3D toggles. Now it is alive.
- **Level 2, patterns (the Strudel layer).** `note("c e g")`, `euclid(3, 8)`, layered and transformed live (`rev`, `fast`, `slow`, `every`, `degrade`). The pattern drives sound *and* geometry together. Now it is an instrument and a generative visual at once, "Strudel techno" you can see (see `MUSIC.md`).
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

## A few things you could make in one sitting

- **A blooming sunflower:** one polar line using the golden angle, seeds spiral out and plink in an even rhythm; detune the angle and watch and hear it fall apart.
- **A techno loop you can see:** a euclidean bass pattern drives a Lissajous figure that pulses on the beat, tune the frequency ratio and the figure locks as the chord resolves.
- **Your name, drawn by circles:** feed a drawn path to the Fourier operator and a chain of epicycles redraws it while playing its own spectrum.
- **The Riemann zeta function:** one expression, domain-colored, its zeros lighting up along the critical line, humming.

The point of the examples: the famous built-in rooms are reachable in a few lines of the same language, which is exactly what makes it feel powerful and what makes the whole game moddable.

## Sharing and safety

- **Native sharing:** a creation is text plus deterministic parameters, exported as a clip and a `.num` file / `numinous://` link that reopens it exactly in the app. The first CLI `.num` file/link save and `open-studio` path now exists for expression plots; app reopening, clips, and gallery flow still need to land before this promise is complete (see `ARCHITECTURE.md`, `ROADMAP.md`).
- **Sandboxing (important):** community Studio code is untrusted and must run sandboxed, no filesystem, no network, resource/time limits, GPU work through the safe pipeline only. This is a hard requirement for the mod SDK and a `QUALITY.md` concern (fault injection and fuzzing of untrusted patterns).

## Roadmap position (reconsidered)

The Studio is currently slated as a Phase 3 build, but its *engine* is foundational and should exist much earlier, because **rooms are Studio programs**:

- **Phase 0 to 1:** build the expression/pattern **runtime** and the one-expression-to-sight-and-sound binding as part of the engine, because the flagship room uses it. A minimal internal "expression" capability exists from the start.
- **Phase 2:** a **lite** public surface, a single expression bar / graphing-calculator mode, enough for `y = sin(x)` fun and simple patterns.
- **Phase 3:** the **full Studio**, the pattern algebra, multiple representations, the gallery/fork/remix UI, promote-to-room, MIDI performance.
- **Phase 4:** the Studio *is* the mod SDK, sandboxed community rooms.

## Open questions
1. Bespoke DSL vs. embedding an existing scripting host (Rhai/Lua) with a pattern layer on top (shared decision with `ARCHITECTURE.md`).
2. How much raw WGSL to expose at Level 3, and how to sandbox it safely for community sharing.
3. Live-eval performance: keeping sub-second feedback while an expression drives both a heavy visual and real-time audio.
4. How much of the pattern language to borrow directly from Strudel/TidalCycles (familiarity) vs. tailor to the math-first, audiovisual use case.
