# Music & Sound

Sound is not decoration in Numinous. It is half the product. The visuals get you to stop; the music is why you stay, why you leave it running, and why a clip is worth sharing. The bar is simple: **the music has to be genuinely, unironically great**, the kind of thing you would put on even with the screen off.

There are two engines, and they are designed to coexist and even harmonize.

---

## Engine A: Programmatic music (the math makes the sound)

> A2 status (July 2026): motifs shipped for all 30 catalog rooms. A motif is
> a room's musical identity (key, tempo, a line of semitone degrees, and
> what it encodes): Times Tables circles and returns in D minor pentatonic;
> Lorenz wanders ten notes and never resolves; the Random Walk stumbles
> chromatically; Voronoi rings open fifths; Lissajous locks a visible fifth;
> Zeno's Square shrinks toward arrival; the Logistic Map splits into chaos.
> In the app the motif IS the room's bed; over MCP, listen_room returns the
> phrase structurally (key, BPM, note names, what it encodes). And the room's
> actual sonification now derives from the motif too: the default `Room::sound`
> plays the motif's own phrase (`SoundSpec::from_motif`), so every room sounds
> like itself rather than a shared root-fifth-octave fallback, and the notes you
> hear match the notation listen_room reports (a July 2026 playtest caught the
> old fallback making every room sound identical and disagree with its motif).
> Rooms whose math has richer, phase-varying music (Collatz's orbit, Epicycles'
> harmonic stack, Lissajous' tuned ratio) still override with something truer.
> Next: state-dependent tension (the phrase resolves when the dial closes).

> Status: v1 shipped. `crates/core/src/chiptune.rs` composes deterministic
> pentatonic chiptunes (square lead, triangle bass, noise ticks, click-free
> envelopes) from a seed; `numinous tune --seed N --out chip.wav` writes them.
> Next: wire it into the app as the score, then the pattern engine below.

This is the native, generative, "everything is an instrument" engine. It runs locally, in real time, in Rust (`cpal` + `fundsp`), and it is driven by the math itself. No files, no streaming, infinite and never-repeating.

### A1. Room sonification (the instrument layer)
Every room turns its own math into tuned, musical sound (detailed per-room in `ROOMS.md`). The rules that keep it musical instead of noisy:

- **Quantize to scales / just intonation.** Map continuous math to notes in a chosen scale so exploration always sounds like music. Integer frequency ratios (which is what consonance *is*) come straight out of the math: a 2:3 Lissajous figure *is* a perfect fifth. The ear learns the math.
- **Consonance carries truth.** When numbers align (closed curves, resonance, integer ratios) it resolves; when they do not, it gently tenses.
- **Number sequences become melody and rhythm.** Primes, Fibonacci, Collatz orbits, digits of pi, all play themselves. A prime spiral has a prime beat.
- **Euclidean rhythms.** The Bjorklund algorithm (spreading k beats as evenly as possible over n steps) is pure math and produces almost every traditional world rhythm. It is a first-class rhythm generator in the engine.

### A2. The bit-depth stations (chiptune, synced to the Visual Eras)
The programmatic engine has its own retro voices that pair with the Visual Eras (see `DESIGN.md`). Flip the app into 8-bit and the *sound* goes 8-bit too:

- **4-bit**: the crudest square/noise, one or two voices, brutal and charming.
- **8-bit**: NES-flavored: pulse, triangle, noise channels. Chiptune melodies generated from the room's math.
- **16-bit**: Genesis/SNES-flavored FM synthesis and sample-ish pads. Richer, still retro.
- **Oscilloscope era**: pure analog sine/saw; the waveform you hear is the waveform you see.
- **Modern era**: the full tuned house synth, reverb, the polished default.

Because the melodies are *generated from the math*, the same room produces an endless chiptune in 8-bit and an endless ambient piece in modern, from one source of truth.

### A3. Strudel-style generative techno (the pattern engine)
The centerpiece of the programmatic engine, and the beating heart of the **Studio** (see `DESIGN.md`): a **pattern language** in the spirit of **TidalCycles / Strudel**, where terse patterns describe rhythm, pitch, and timbre as functions of time, and can be layered, transformed (reverse, every-n, degrade, euclid), and modulated live.

- This is how we get **"Strudel techno"**: driving, evolving, algorithmic electronic music that never loops identically, built from mathematical patterns.
- Crucially, the same pattern that drives the *sound* can drive the *geometry* on screen, so in the Studio you live-code an audiovisual piece where sight and sound are literally the same expression.
- It doubles as the app's generative soundtrack: point the pattern engine at the current room's parameters and it scores your play in real time.

**Why local + generative:** it is free, infinite, offline, never repeats, reacts instantly to what you do, and every configuration is reproducible from a seed (so a shared deep-link sounds identical to what the sharer heard).

---

## Engine B: The Radio (ElevenLabs-powered stations, GTA-style)

> Status: v1 live. Three stations, 30+ original tracks and climbing (about
> two hours on air), full stereo, varied unround runtimes dealt from each
> station's rotation deck, wall-clock live sync (you tune in mid-broadcast),
> bounded local cache loading through the app radio-cache module, per-track
> briefs from the house identity plus a card. The dial: Y in the
> app, - and = for volume; `numinous radio` lists rotations; `numinous tune2
> <station> --count N` grows them. Next: crossfade on rotation, the Comedy
> Channel, room-over-radio one-bus mixing without record restarts, cost guardrails.
>
> (v0 status, kept for the record:) v0 shipped. The dial lives in `crates/core/src/radio.rs` (three
> stations with full producer briefs: NUMINA FM trance at 132 BPM, THE
> ATTRACTOR chillwave at 84, EIGHT BIT SUNRISE synthwave at 118, all
> instrumental by contract, tested). `numinous radio` shows the dial;
> `numinous tune2 <station> --seconds 120` calls ElevenLabs Music
> (`POST /v1/music?output_format=pcm_44100`, `ELEVENLABS_API_KEY` env),
> receives raw PCM, and caches `~/.numinous-radio/<station>.wav`. In the
> app, Y turns the dial: off, then station by station; a cached station
> becomes the bed. Room-over-radio mixing is the next mixer upgrade.
> Next: multiple tracks per station with rotation, the Comedy Channel
> (needs its writer), crossfade on dial turns, and cost guardrails
> (a track of 2 minutes is a paid API call; cache hard, regenerate rarely).

The counterpoint to the generative engine: curated, produced, *songs* and *talk radio*, delivered as a set of **stations** you tune between like the radio in a GTA game. Where Engine A is the math singing, Engine B is the world the math lives in having a personality.

Powered by the **ElevenLabs API** (music generation for tracks, voice/TTS for DJs, idents, and ads).

### The stations (launch set)
- **NUMINA FM: EDM.** Festival-grade four-on-the-floor for the Watch mode and performance sessions.
- **137.5 Trance.** (Named for the golden angle.) Long, euphoric, hypnotic, the lean-back-and-dissolve station.
- **Lo-Fi / Chill: "Study Group".** Warm, mellow beats for long ambient sessions. The "leave it on while you work" station.
- **The Comedy Channel: "WKRP-adjacent, but for math."** Talk radio hosted by deadpan characters who live inside the math universe. Bits, fake ads, call-ins, station idents. This is the single biggest lore-carrier in the product (see `LORE.md`) and the source of the hyper-specific, obsessive, deadpan insider humor the whole thing runs on.

Additional stations are cheap to add (a station is a prompt template + a voice + a schedule), so seasonal and community stations are trivial later.

### How the Comedy Channel works (and why it matters)
The comedy channel is generated, not hand-recorded, so it can be endless and current:

- **Hosts** are ElevenLabs designed voices with fixed personas (e.g., a serene host who speaks only in koans; a hype DJ who is *way* too excited about the Riemann Hypothesis; a nervous intern who keeps almost proving Collatz on air).
- **Content** is generated from prompt templates and stitched between music: cold opens, math jokes, "on this day in mathematics," fake ads, listener call-ins, deadpan news from the math dimension.
- **Fake ads** are the comedic core, and pure insider bait. Examples of the register we are aiming for:
  - *"New from the Numinous: the Trisection Compass. Finally trisect any angle with nothing but compass and straightedge. (Not valid in Euclidean geometry. Side effects may include two thousand years of failed proofs.)"*
  - *"Tired of your series diverging? Ask your analyst about Analytic Continuation. Now 1 + 2 + 3 + ... can equal negative one-twelfth. Terms and conditions are, frankly, upsetting."*
  - *"Feeling incomplete? So is every sufficiently powerful formal system. Gödel's, now open late."*
- Everything here is **in-universe**: the DJs are inhabitants of the dimension, and long-time listeners slowly realize the station is telling a story.

### Technical shape
- **`crates/music`** holds a thin ElevenLabs service layer (request, stream, cache) fully decoupled from the synthesis engine.
- **Generation is offline-first where possible:** tracks and comedy segments are generated ahead, cached to disk, validated under bounded local cache rules, and assembled by a local **station scheduler**, so the radio works without a live connection after first fetch. Optional online refresh pulls new bits.
- **Station identity** (idents, stingers, DJ drops) is generated once and reused; music beds and talk are ducked/crossfaded by the scheduler for that seamless-radio feel.
- **Licensing / rights:** AI-generated audio via a paid ElevenLabs tier is intended to grant commercial usage, but confirm the current ElevenLabs commercial terms before shipping, and keep a fully-generative fallback (Engine A) so the product is never dependent on a third party to make sound.

---

## How the two engines coexist

- **One master bus target.** Both engines should feed a shared mix with a global master volume and mute. Current app radio v1 keeps long station tracks stable by handing the station buffer to the player; the room-over-radio overlay is still a mixer upgrade so it can happen without restarting records.
- **Global key and tempo.** The app holds a global key and BPM. Room sonification quantizes to that key so your poking harmonizes with the current station instead of fighting it. (This is itself a piece of math: everything tuned to one ratio lattice.)
- **Mode-aware mixing.**
  - *Watch* mode: radio forward, room sonification as gentle texture. Lean back.
  - *Play* mode: room sonification forward (you are the instrument), radio as a bed you can turn down.
  - *Create* / Studio mode: the pattern engine (Engine A3) is the whole show; radio off by default.
- **Always mutable, beautiful in silence.** A prominent, respectful mute. The visuals must still be gorgeous with the sound off (the library, the office, the 2am room where someone is asleep).

## Open questions
1. ElevenLabs Music API quality/latency/cost at the volume we need, and the exact commercial-rights terms. Prototype early.
2. How much comedy content to pre-generate and ship vs. fetch on demand (size vs. freshness).
3. Whether the Strudel-style pattern engine is a bespoke Rust DSL or an embedded scripting host (shared decision with the Studio, see `ARCHITECTURE.md`).
4. Global-key harmonization: how aggressively to quantize room sound to the station key before it feels less like *the room's* voice.
