# Design

How Numinous feels, moment to moment, and the rules that keep it feeling that way.

## The design pillars (and how to obey them)

### 1. Awe before instruction
The visitor must experience beauty **before** they read a single word of math. No room opens on an equation. No room requires reading to start playing. Text is earned: you tap **Reveal** when *you're* ready, and never before. If a room can't create a "whoa" in its first 10 seconds with zero explanation, it isn't done.

### 2. Everything is an instrument
Nothing on screen is silent. Every visual element maps to sound, and the sound is **musical**, tuned to scales, harmonically aware, never a beep. The entire app can be played like an instrument and performed. Sound is not decoration; it's a second channel for the same math, and often the channel that lands the point (dissonance = the numbers don't line up; consonance = they do).

### 3. Toy → puzzle → revelation (the three layers)
Every room is built in three concentric layers. You can stop at any layer.

- **Layer 1, TOY (mandatory).** Pure sandbox. Grab things, turn dials, no goal, no fail state, no words. Just cause and effect that's fun to poke. This layer alone must justify the room.
- **Layer 2, PUZZLE (optional).** A small, self-contained challenge that gives the flow-state hit: "make the shape close on itself," "find the rule that draws a triangle," "tune it to a perfect fifth." Concrete goal, instant feedback, a clean win. Never mandatory, never blocking.
- **Layer 3, REVELATION (optional).** One card. One or two sentences. The 3Blue1Brown gut-punch, the connection or fact that recontextualizes everything you just played with. ("That heart-shape? It's the exact boundary of the Mandelbrot set's main body. You've been drawing a fractal with a ruler.") Links out to go deeper for the truly hooked.

### 4. Emergence is the star
Prefer rooms where a **stupidly simple rule** produces **stunning complexity**, and make the simplicity *legible*, the visitor must be able to see/feel how little input created how much output. That gap is the product. Show the rule plainly (a single slider, a single equation-free statement) so the output feels impossible.

### 5. Beautiful by default
- Every frame is screenshot-worthy. If you pause at a random moment and it isn't gorgeous, fix it.
- Motion is always smooth (target 60fps; 120 where the display allows). Beauty lives in the animation, not just the still.
- Restraint over spectacle. Negative space. One idea per screen.

### 6. Made to be shared
Every session can leave the app. One-tap **Share** captures a loop (MP4/GIF) or a `numinous://` link / `.num` seed file that reopens *your exact configuration* in the installed app (native, no browser). Watermark is a single tasteful glyph. The dream: a Numinous clip goes viral on its own aesthetic merits, math smuggled inside.

## The aesthetic direction

**One sentence:** Ryoji Ikeda's restraint × Teenage Engineering's playfulness × 3Blue1Brown's clarity, on a near-black stage.

- **Canvas:** deep near-black (`#0a0b0f`-ish), never pure black. The math glows *on* the dark.
- **Color:** each room owns **one** signature accent that glows; supporting values stay monochrome. Color carries meaning (e.g., pitch, phase, iteration count), never decoration. A shared palette across rooms keeps the whole product coherent. (Palette to be validated for contrast + colorblind-safety per the `dataviz` guidance when we build the design system.)
- **Type:** a precise technical monospace for numbers/parameters (the "computational" voice) paired with a refined humanist sans for the rare prose (revelation cards). Numbers are first-class typography.
- **Line & glow:** additive blending, subtle bloom, anti-aliased everything. Think "lit from within," not "flat UI."
- **UI:** near-invisible until needed. Controls fade in on hover/approach and recede while you watch. The math is the interface.
- **Motion:** eased, physical, continuous. Nothing snaps. Dials have momentum. Transitions between rooms are dissolves through black, never hard cuts.

## Visual Eras: the look *progresses* (retro → modern)

The minimalist glow above is Numinous's **native** look, but it is not the *only* look. Numinous carries a set of **Visual Eras**, skins that re-render every room (and re-voice its audio) in a different graphics epoch. This does three jobs at once: it gives the app **variety** so it never feels same-y across 20 rooms; it delivers pure **retro joy** (8-bit, CRT, chiptune, catnip for exactly our audience); and it quietly tells a story, *the history of computer graphics is the history of humans trying to make math visible*, from teletype to GPU. That lineage is the Wolfram/computational-universe thesis, felt.

**The eras (roughly chronological):**

| Era | Look | Audio voice |
| --- | --- | --- |
| **Teletype** | Green-phosphor terminal, ASCII/character-cell rendering, cursor blink | Bleeps / modem tones |
| **8-bit** | Chunky pixels, a strict ~4-color palette, CRT scanlines + curvature + glow | Chiptune (square/triangle/noise) |
| **16-bit** | Richer pixel palette, dithering, sprite-era polish | FM synth (Genesis/SNES flavor) |
| **Vector / Oscilloscope** | Glowing wireframe lines on black, phosphor persistence, no fills | Pure analog sine/saw tones |
| **Blueprint** | Graph-paper grid, drafting lines, annotations, ink-on-cyan | Soft mechanical pencil/pen |
| **Modern (native)** | The minimalist additive-glow system above | The tuned house synth |

**Three ways they're used:**

1. **Skins (player choice).** Flip the whole app into any unlocked era anytime, including the audio. "Numinous in 8-bit with chiptune" is its own delightful mode, and a *distinct set of shareable clips* from the same rooms.
2. **Progression (the meta-thread).** Collecting **Constants** (see the Cabinet section) unlocks eras **in historical order**, so the app literally *ages up* from teletype to modern glow as you go deeper. Reaching the modern era feels earned, and the journey re-tells the history of visualizing math.
3. **Native era per room/wing (variety by default).** Some phenomena have an obvious home era, so the collection has built-in visual variety even before you touch a skin: *Game of Life* and *Cellular Automata* are gorgeous in **8-bit**; *Lissajous* and *Fourier* belong on the **oscilloscope**; *Straightedge & Compass* wants **Blueprint**; *Mandelbrot* sings in **modern glow**. Each room ships with a "native" era and inherits the rest for free.

**The rule that makes this cheap:** rooms never hardcode colors, line styles, or synth voices. They draw and sound in **theme-relative terms** (`gfx.stroke(accent)`, `bus.note(...)`), and the `engine/theme` layer swaps the entire era, pixels vs. glow vs. scanlines, chiptune vs. house synth, underneath them. One room, every era, no per-room work. (See `ARCHITECTURE.md`.)

> Design guardrail: an era is a *lens on the same beauty*, never an excuse for an ugly frame. Every era gets the same beauty-QA bar. Retro means lovingly-crafted CRT, not lazy pixels.

## The audio direction

Sound is a first-class citizen with its own art direction, not an afterthought.

- **Musical, not sonified-raw.** Map math to **tuned** pitch (quantize to scales/just intonation) so exploration sounds like music, not a Geiger counter. The default scale should make "wrong" inputs sound *interesting*, not painful.
- **Consonance carries truth.** When numbers align (integer ratios, closed curves, resonance) it should sound *resolved*; when they don't, gently tense. The ear learns the math.
- **One coherent sonic palette** across rooms (a shared synth voice, reverb, master bus) so the whole app sounds like one instrument, the way it looks like one place.
- **Ambient by default, expressive on touch.** Left alone, a room breathes a calm generative drone. Touched, it becomes an instrument you're playing.
- **Always mutable.** A prominent, respectful mute. Beauty must survive silence (for the library, the office, the 2am room where someone's asleep).

## UX & interaction principles

- **Zero-friction entry.** No account, no tutorial wall, no settings gauntlet. Open → Cabinet → tap a tile → you're playing in under 3 seconds.
- **Discovery over instruction.** Like *The Witness*: you learn what a control does by using it, not by reading a tooltip. Affordances are visual (a dial *looks* draggable).
- **Direct manipulation.** You touch the math itself (drag the point, bend the curve), not an abstract slider elsewhere, wherever possible.
- **No dead ends, no fail.** In toy mode you can't lose or break anything. A **reset** is always one tap and always graceful.
- **Reversible everything.** Undo/scrub where it makes sense. Encourage fearless poking.
- **Respect the flow.** Interruptions (dialogs, popups, "did you know?") are banned during play. The Reveal card is the *only* text, and it's summoned, never pushed.
- **Progressive depth.** A curious visitor can always go one level deeper (Reveal → "the math" → external link), but the surface stays clean for everyone else.

## Modes: Watch, Play, Create

The same room meets you in three postures. A player slides between them freely; each is a complete way to be here, and each targets a different person (and a different moment).

- **Watch (lean back, minimal interaction).** A room, or a playlist of rooms, runs itself as generative art with a soundtrack. Zero input required. This is the "put it on the big screen and just look" mode: a live math visualizer, a screensaver that is actually beautiful, a VJ backdrop, an ambient companion while you work. **This is the mode that opens the door.** A newcomer glances at a friend's screen, sees this, and says, with no irony at all, "wait, *math* is doing that?" It is also the primary sharing engine (every Watch session is a clip waiting to happen) and where the ElevenLabs radio (see `MUSIC.md`) does its best work.

- **Play (grab the dials).** The default posture and the three-layer model (Toy → puzzle → revelation). You touch the phenomenon directly and it responds in sight and sound. Described throughout this doc.

- **Create (make your own).** **The Studio**, below. For the person who stops consuming and starts building.

Design consequence: every room must be gorgeous and self-sustaining with **no** input (Watch), delightful *with* input (Play), and *expressible as a pattern* so it can live in the Studio (Create). Build all three affordances into the Room contract from the start (see `ARCHITECTURE.md`).

### Benchmark / "The Show" (the maxed-out Watch)

Watch mode has a headline form: a full-screen, self-directing, never-repeating audiovisual **performance** designed to be left running for hours. This is the "lava lamp for math lovers," the thing you put on the big screen at a party and lose track of time watching. It earns the name "Benchmark" in two senses, and we lean into both:

- **It is a show.** An internal auto-director (a VJ with taste) moves through rooms and transitions on its own: it settles into a phenomenon, finds its most beautiful configuration, slowly explores the parameter space, dives into a fractal, morphs a curve, lets a Game-of-Life colony bloom, then dissolves to the next, all beat-matched to the current radio station or the generative score (see `MUSIC.md`). Never the same twice (seeded, generative), never a hard cut, never a dull stretch. Pacing is engineered like a DJ set: builds, drops, breathers.
- **It is a benchmark.** In the demoscene spirit, it deliberately flexes the machine, layering the heaviest, most gorgeous GPU work the hardware can sustain, and it can display an optional, tasteful on-screen readout of what it is actually computing (iterations/sec, particle count, GPU headroom, current phenomenon, the live equation). That readout is both a genuine "look what your rig can do" flex and a subtle lore/insight surface. It auto-scales quality to hold the 60/120fps floor (see `VISUALS.md`), so it looks maximal on a beast and still smooth on a laptop.

Design requirements it imposes on everything else: every room must have an **auto-director profile** (what "beautiful, evolving, hands-off" means for it, what to sweep and how slowly), and must degrade quality gracefully under the benchmark's load balancer. It is also the single best sharing and marketing engine in the product, a Benchmark session is an endless supply of clips, and the readout makes those clips legibly *about math*.

## The Studio (the creative canvas)

The creator tier, and the thing that makes Numinous a tool people *live in*, not just a gallery they visit. Think **an expressive graphing calculator**, crossed with a **Strudel / TidalCycles live-coding environment**, crossed with a shader toy.

- **Live-code sight and sound at once.** You write terse **patterns** (see `MUSIC.md`, Engine A3) that drive geometry and audio from the *same* expression. Change a number, the visual and the music both shift, instantly, no recompile. The feedback loop is sub-second, which is what makes it feel like an instrument and not an IDE.
- **A ladder, not a cliff.** The surface is a friendly expression box ("type `sin(x)` and watch it sing") that a curious newcomer can enjoy in ten seconds. Underneath, it goes as deep as raw WGSL shaders and full pattern algebra for people who want it. Same tool, radically different ceilings.
- **Everything is shareable and reproducible.** A Studio creation is just text + a seed, so it exports as a deep-link that reopens *exactly* what you made, and a loop you can post. The best community creations become candidate **rooms** (the on-ramp to the Phase 4 mod SDK).
- **The point of it all:** this is where "math is fun, non-ironically, seriously" stops being a slogan and becomes something a person *did with their own hands*. Consuming beauty is good; making it is the conversion that sticks.

The full Studio is a Phase 3 build, but its *runtime* is engine-foundational and built from day one, because **rooms are Studio programs** (rooms are patterns, patterns drive both channels, every configuration serializes to a shareable link). See **`STUDIO.md`** for the complete design.

## The Cabinet (the shell / hub)

The connective tissue between rooms. It must feel like a *place*, not a menu.

- A dark hall of glowing tiles, each a **live, animated preview** of its room (a looping micro-visual, the room breathing).
- Rooms grouped into **Wings** by theme (see `ROOMS.md`): *Emergence, Waves & Sound, Infinity & Fractals, Number & Pattern, Shape & Space, Chance & Order.*
- Gentle **meta-progression**: playing a room to its Revelation "collects a Constant" (π, e, φ, i, ℵ₀, …), a light, optional completionist thread, purely for the joy of the set, never a gate.
- An **Ambient / Performance mode**: pick rooms into a playlist and let Numinous run itself as generative art / a screensaver / a VJ backdrop, the "just leave it on, it's beautiful" mode that also seeds sharing.

## The anti-patterns list (paste this above your monitor)

- No equation before wonder.
- No silent interaction.
- No fail states in the toy.
- No ugly frame.
- No forced text.
- No feature that feels like school.
- No hard cut, no snap, no beep.
- If in doubt, make it more beautiful and less explained.
