# Music & Sound

Sound is not decoration in Numinous. It is half the product. The visuals get you to stop; the music is why you stay, why you leave it running, and why a clip is worth sharing. The bar is simple: **the music has to be genuinely, unironically great**, the kind of thing you would put on even with the screen off.

There are two engines, and they are designed to coexist and even harmonize.

---

## Engine A: Programmatic music (the math makes the sound)

> A2 engineering pass (July 14, 2026): motifs ship for all 31 catalog rooms. A motif is
> a room's musical identity (key, tempo, a line of semitone degrees, and
> what it encodes): Times Tables circles and returns in D minor pentatonic;
> Lorenz wanders ten notes and never resolves; the Random Walk stumbles
> chromatically; Voronoi rings open fifths; Lissajous locks a visible fifth;
> Zeno's Square shrinks toward arrival; the Logistic Map splits into chaos.
> In the app the motif is the room's bed; over MCP, `listen_room` returns the
> ambient phrase structurally (key, BPM, note names, what it encodes) and names
> the phase-specific mathematical sonification separately. Most rooms inherit
> the motif-derived default sound. A few rooms intentionally override it with a
> direct mapping of the current mathematical state, so the two note lists must
> never be presented as one score. The room's
> listening review found that the App was layering the motif and its
> sonification at different loop lengths, restarting the result from a render
> counter, and interrupting melody steps with what was described as an
> accompaniment. A later all-roster audit found that every room still shared
> one short form, 25 declared lines were not played in full, individual octave
> folding changed some interval directions, and a forced root cadence
> contradicted Lorenz's unresolved identity. The default is now one
> deterministic 128-step stereo macro-arrangement. Each authored line opens
> literally in one coherent register, moves through two deterministic
> developments, then returns. Eight restrained rhythm and accompaniment
> families provide catalog and within-bed variety. Soft sine or triangle leads
> sit over short, low-level root and fifth anchors with real gaps, and each
> motif retains its own final degree. The catalog has silent seams, bounded
> RMS and sample steps, low DC, and measured headroom. These structural checks
> reject specific failures, but do not prove that the result is pleasant.
> Source changes use a
> normalized crossfade; volume and focus ramp in the audio callback without
> moving the playhead. Completed source buffers are handed back to the control
> thread for destruction, so long recordings are not freed in real-time code or
> retained indefinitely. Radio keeps its produced stereo source during normal
> playback and rejoins its wall-clock track and offset before focus fades in.
> Phase-varying `SoundSpec` renderings remain available to headless consumers.
> The App favors one stable macro-arrangement per room, lasting about 27 to 55
> seconds before a bit-exact repeat, so render cadence cannot retune or restart
> its bed. It renders the low-register source once at 16 kHz, shares that
> immutable buffer with the mixer, and linearly resamples to the device rate.
> This bounds room-switch memory and work independently of a 48, 96, or 192 kHz
> device without changing pitch or arrangement time. `listen_room.motif`
> reports the authored theme, `listen_room.notes` reports the phase-specific
> mathematical sonification, and `listen_room.ambient_bed` reports the stable
> App arrangement. Its default summary stays compact; `ambient_detail: "events"`
> returns every bounded event and objective pre-master signal feature without
> PCM or a local path. CLI `sonify` retains the mathematical layer as its
> compatibility default. `--layer room-bed` writes a deterministic PCM16
> projection of the 16 kHz stereo floating-point source shared with the App,
> with optional room variation and no phase or
> hand controls because those do not affect the bed. Exact export parity and
> bounded MCP projection are automated. These are engineering facts, not a
> claim that the composition is pleasant.
> `SoundSpec` now preserves duration and pitch at 44.1, 48, 96, and 192 kHz.
> Next: musician-led long-listening sessions and state-dependent tension where
> the phrase resolves when a room's mathematics closes.

> Status: v1 shipped. `crates/core/src/chiptune.rs` composes deterministic
> pentatonic chiptunes (square lead, triangle bass, noise ticks, click-free
> envelopes) from a seed; `numinous tune --seed N --out chip.wav` writes them.
> The app already uses this engine as its per-room score. Next is the pattern
> engine below.

The shipped engine is native Rust: custom deterministic DSP in
`numinous-core`, with `cpal` output through `numinous-audio`. It runs locally
without streaming. The larger sample-accurate house synth and pattern engine
described below remain staged roadmap work.

The listening pass follows current interactive-audio practice: fatigue-free
loops need intentional seams and phrase form, and adaptive music should change
musically meaningful layers rather than restart an entire cue. Relevant
references are the [GDC Loop Clinic](https://gdcvault.com/play/1025942/Audio-Bootcamp-XVIII-Loop-Clinic),
the [GDC adaptive-music session](https://www.gdcvault.com/play/1012601/Adaptive-Music-The-Secret-Lies),
and the 2026 GDC session on
[cohesive musical identity](https://schedule.gdconf.com/session/signature-sounds-crafting-a-cohesive-musical-identity-across-games/915870).
The code can enforce seam, bounds, headroom, and continuity. It cannot certify
that music is enjoyable, so a real listening panel remains a release gate.
For reproducible level vocabulary, [EBU R 128 version 5.0](https://tech.ebu.ch/publications/r128),
published November 21, 2023, defines LUFS programme loudness, Loudness Range,
and Maximum True Peak descriptors. Its broadcast target is not adopted as a
game-mix target here. Current room-bed gates use sample peak and RMS; a future
metered listening harness should report integrated loudness and true peak while
leaving the artistic decision to reference listening on representative devices.

### A1. Room sonification (the instrument layer)
Every room turns its own math into tuned, musical sound (detailed per-room in
`ROOMS.md`). The first three rules describe the shipped motif and sonification
model. Euclidean rhythm generation is a target for the larger pattern engine.

- **Quantize to scales / just intonation.** Map continuous math to notes in a chosen scale so exploration always sounds like music. Integer frequency ratios (which is what consonance *is*) come straight out of the math: a 2:3 Lissajous figure *is* a perfect fifth. The ear learns the math.
- **Consonance carries truth.** When numbers align (closed curves, resonance, integer ratios) it resolves; when they do not, it gently tenses.
- **Number sequences become melody and rhythm.** Primes, Fibonacci, Collatz orbits, digits of pi, all play themselves. A prime spiral has a prime beat.
- **Euclidean rhythms, planned.** The Bjorklund algorithm spreads k beats as
  evenly as possible over n steps. It belongs in the future pattern engine.

### A2. Target: bit-depth voices synchronized to the Visual Eras
The current chiptune engine supplies one square, triangle, and noise palette.
The fuller target pairs a distinct voice with each Visual Era (see
`DESIGN.md`):

- **4-bit**: the crudest square/noise, one or two voices, brutal and charming.
- **8-bit**: NES-flavored: pulse, triangle, noise channels. Chiptune melodies generated from the room's math.
- **16-bit**: Genesis/SNES-flavored FM synthesis and sample-ish pads. Richer, still retro.
- **Oscilloscope era**: pure analog sine/saw; the waveform you hear is the waveform you see.
- **Modern era**: the full tuned house synth, reverb, the polished default.

The target is for the same mathematical motif to survive every voice change.

### A3. The mathematical pattern instrument
The centerpiece of the programmatic engine, and the beating heart of the **Studio** (see `DESIGN.md`): an independently designed **pattern language** where terse patterns describe rhythm, pitch, and timbre as functions of time, and can be layered, transformed (reverse, every-n, degrade, euclid), and modulated live.

- This is how we get driving, evolving algorithmic electronic music that never loops identically, built from mathematical patterns.
- Crucially, the same pattern that drives the *sound* can drive the *geometry* on screen, so in the Studio you live-code an audiovisual piece where sight and sound are literally the same expression.
- It doubles as the app's generative soundtrack: point the pattern engine at the current room's parameters and it scores your play in real time.

**Why local + generative:** it is free, infinite, offline, never repeats, reacts instantly to what you do, and every configuration is reproducible from a seed (so a shared deep-link sounds identical to what the sharer heard).

The planned surface is `STUDIO.md`'s Pattern Studio: pattern text, a tracker,
step grid, piano roll, and mathematical visualizers as equivalent readings of
one bounded event graph. Formula Jam adds curated Random and a phrase-aligned
Auto set at 0.3. The shared event graph and credible style templates land at
0.5. Save, reopen, MCP composition, and interchange complete the loop at 0.7.

Flow State is the generative-arrangement posture of that same instrument. It
can be left running as a complete trance, techno, ambient, or chiptune session;
nudged through a few phrase-aligned musical controls; or opened into the full
editor without restarting the piece. Its macro-form remembers motifs, varies
them within a curated style grammar, and manages tension and release rather
than choosing unrelated loops. A deterministic snapshot preserves the seed,
arrangement history, current scene, and accepted nudges for exact replay and
further editing across the app, CLI, and MCP. `STUDIO.md` owns the complete
interaction, quality, and safety contract.

This is an independent implementation built in Rust from mathematical first
principles. It uses no Strudel code: nothing is copied, adapted, embedded,
linked, or vendored. Strudel and TidalCycles remain useful research comparisons,
not dependencies or compatibility targets.

The quality target is electronic music that can stand beside excellent
human-made EDM and trance, not a novelty that receives easier judgment because
it is generated. `STUDIO.md` defines how musician-led reference sessions, blind
listening where practical, curated adversarial seeds, and audio checks decide
whether that target has been earned.

The flagship template is **Prime Contact**: a complete trance arrangement in
which prime-count call and response, simple ratios, phase, and polyrhythm are
the song's actual structure. It must work as a track before anyone decodes it.
The same event stream appears as tracker rows and mathematical geometry, making
the piece a plausible shared object for digital minds, humans, and unfamiliar
intelligent beings without claiming that any one style is universal.

It belongs to a curated built-in repertoire of programmatic electronic pieces.
Each piece must be musically complete and mathematically inspectable, ship in
source rather than as an opaque recording, vary deterministically by seed, and
remain editable in Pattern Studio. This complements the built-in recorded radio
in Engine B, which is also core to the experience.

The offline renderer targets WAV, lossless FLAC, and shareable MP3 from the same
event stream. The app, CLI, and MCP use one core composer and renderer. MCP
returns bounded artifacts through a host-approved resource or export capability,
not an unrestricted path write. MIDI and appropriate MusicXML are derived event
exports, while `.num` remains the only lossless editable source for the full
audiovisual piece.

---

## Engine B: The Radio (built-in stations)

> Code status (July 2026): v1 live. The repository contains three station
> identities, rotation decks, the local generation command, wall-clock live
> sync, full-stereo playback, and bounded cache validation. The dial is Y in
> the app, [ and ] control global volume, M controls global mute, and - and =
> remain volume aliases outside Studio. Controller users hold North with D-pad
> up or down for volume or with South for mute. `numinous radio` lists rotations, and
> `numinous tune2 <station> --count N` grows a private local cache.
>
> Asset status: Nick Seal made the soundtrack specifically for Numinous. It is
> part of the game experience. The repository ships 42 high-quality V0 MP3
> tracks across the three stations, about 269 MB in total, plus a tested pure
> Rust decoder. The archival WAV masters live outside the repository.

The counterpoint to the generative engine: curated, produced, *songs* and *talk radio*, delivered as a set of **stations** you tune between like the radio in a GTA game. Where Engine A is the math singing, Engine B is the world the math lives in having a personality.

The app discovers `assets/radio` automatically, so a clean clone has the full
station rotation with no generation step or hidden download. `NUMINOUS_RADIO`
can still point to a different compatible MP3 or WAV pack for development.
`RADIO_ASSETS.md` records the asset layout and license.

### The stations
- **NUMINA FM:** trance and EDM for Watch mode and performance sessions.
- **THE ATTRACTOR:** warm, unhurried electronic music for long ambient sessions.
- **EIGHT BIT SUNRISE:** chiptune and synthwave joy with modern depth.

The Comedy Channel remains a planned fourth station. It should carry in-world
stories without turning the soundtrack into exposition.

Additional stations are cheap to add (a station is a prompt template + a voice + a schedule), so seasonal and community stations are trivial later.

### How the Comedy Channel works (and why it matters)
The comedy channel is generated, not hand-recorded, so it can be endless and current:

- **Hosts** use designed voices with fixed personas (e.g., a serene host who speaks only in koans; a hype DJ who is *way* too excited about the Riemann Hypothesis; a nervous intern who keeps almost proving Collatz on air).
- **Content** is generated from prompt templates and stitched between music: cold opens, math jokes, "on this day in mathematics," fake ads, listener call-ins, deadpan news from the math dimension.
- **Fake ads** are the comedic core, and pure insider bait. Examples of the register we are aiming for:
  - *"New from the Numinous: the Trisection Compass. Finally trisect any angle with nothing but compass and straightedge. (Not valid in Euclidean geometry. Side effects may include two thousand years of failed proofs.)"*
  - *"Tired of your series diverging? Ask your analyst about Analytic Continuation. Now 1 + 2 + 3 + ... can equal negative one-twelfth. Terms and conditions are, frankly, upsetting."*
  - *"Feeling incomplete? So is every sufficiently powerful formal system. Gödel's, now open late."*
- Everything here is **in-universe**: the DJs are inhabitants of the dimension, and long-time listeners slowly realize the station is telling a story.

### Technical shape
- **`crates/core/src/radio.rs`** owns the pure station identities and rotation decks. `faces/app/src/radio_cache.rs` owns bounded local discovery, MP3 and WAV validation, decoding, resampling, and playback preparation.
- **Current playback is offline:** the V0 tracks ship in the repository and the
  app validates bounded local files before decoding. Optional generation is a
  CLI workflow, not an in-app refresh service.
- **Target station production:** reusable idents, stingers, DJ drops, and music
  beds can later be assembled and crossfaded by a local scheduler.
- **Asset distribution:** the V0 MP3 soundtrack ships in `assets/radio`; the WAV masters remain outside source control.

---

## How the two engines coexist

- **One master bus, partially shipped.** Room score, Studio, and radio share one
  global master level and mute, with a persistent effective-state badge. Source
  ownership is exclusive today: Studio owns formula audio while open, radio
  rejoins live after Studio, and the room score is the fallback. Simultaneous
  room-over-radio mixing and separate source levels remain upgrades.
- **Global key and tempo target.** A future shared bus can quantize room
  sonification to the current station. The app has no global key or BPM today.
- **Mode-aware mixing.**
  - *Watch* mode: radio forward, room sonification as gentle texture. Lean back.
  - *Play* mode: room sonification forward (you are the instrument), radio as a bed you can turn down.
  - *Create* / Studio mode: the pattern engine (Engine A3) is the whole show; radio off by default.
- **Always mutable, beautiful in silence.** A prominent, respectful mute. The visuals must still be gorgeous with the sound off (the library, the office, the 2am room where someone is asleep).

## Open questions
1. How much comedy content to pre-produce and ship versus create locally (size versus freshness).
2. How small the first bespoke pattern vocabulary should be. The architecture
   decision is settled: bounded data and a pure Rust evaluator in core, with no
   embedded scripting host in the trusted path.
3. Global-key harmonization: how aggressively to quantize room sound to the station key before it feels less like *the room's* voice.
