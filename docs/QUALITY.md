# Quality, Testing & Fun-Evals

How Numinous works toward a high quality bar. Most software tests whether a
program works; Numinous also needs evidence about comprehension, awe, beauty,
comfort, and voluntary return play. Only part of that system is automated today.
This document names both the enforced checks and the quality loops still to be
built, so an aspiration is never mistaken for a result.

## Evidence snapshot, 2026-07-18

- **Enforced now:** formatting, Clippy with warnings denied, 2,838 passing
  all-target test cases plus one ignored screenshot diagnostic, locked
  builds, house style, `cargo-deny` in CI, an 80% line-coverage floor, and a
  three-OS test-and-build matrix. The current measured coverage is 95.37%
  regions and 95.41% lines under the documented exclusions.
- **Implemented but not yet validated with strangers:** the native app, local
  playtest-note capture, deterministic room rendering, audio generation, all
  three faces, and a release-generated 2,911-screen visual QA matrix. Every room
  is captured at default and compact sizes in deterministic opening, arrival,
  immediate-interaction, and same-phase delayed-interaction states. Default
  room receipts are 900 by 700 and compact room receipts are 360 by 240. Games,
  overlays, The Show, production Studio rendering, and reset and
  phase flows have dedicated captures. Life also has a five-frame persistent
  sequence through launch, generation 4, generation 141, and exact reset.
  Fourteen compact receipts add
  controller-first room, help, Show, Journey, Studio, game-result, and visible
  pause coverage. Sixteen audio-state receipts cover room score, radio,
  radio-off fallback, Studio, mute, zero volume, background silence, and a
  missing output device at default and compact sizes. Each room has a click,
  active-hold, drag-release, repeated-action, or boundary scenario that follows
  its declared verb. The generator validates ordered finite input, completed
  gesture closure, active-hold release and cancel boundaries, interaction-aware
  status or action semantics, and a pure-room consequence of at least eight
  changed pixels at default size or four at compact size. Independently, the App's latest
  gesture feedback must change at least 100 pixels at default size and 32 at
  compact size, cover at least 1% changed-region support, meet the minimum
  support density, form a cluster of at least two adjacent 32-pixel spatial
  tiles, and meet the minimum mean color change. Life uses a dedicated
  pure-render causal and locality oracle. A cross-process single-writer guard prevents competing generators
  from replacing the same evidence directory. A direct regression proves that four
  isolated 10 by 10 corner markers do not satisfy the spatial gate. These are
  coarse renderer-path checks, not certification of subjective visual quality.
  Production input routing has separate unit tests; native operating system
  event dispatch is not claimed as automated end-to-end evidence.
  Twelve flagship receipts additionally capture K=2, K=3, K=pi, K=4, K=5,
  and the earned four-lobe Aha at both sizes. The generator asserts all five
  spectral inks and the dial marker, while core tests bound compact ASCII
  density so the terminal picture retains negative space.
  Programmatic room-bed tests preserve every authored interval in one register,
  require catalog and within-bed phrase diversity, and bound oscillator level,
  RMS, adjacent sample steps, headroom, DC, exact seams, determinism, and common
  device rates. This is structural audio regression coverage, not a perceptual
  fingerprint or listening result. A shared fixed-order analyzer additionally
  reports finite-sample integrity, clipping, RMS, crest, channel balance, DC,
  correlation, side-to-mid ratio, adjacent steps, and exact-zero fraction.
  CLI tests parse RIFF independently and compare every exported PCM16 sample to
  the shared quantizer's projection of the App source; MCP tests compare every event for all 351 rooms, enforce a
  96-event and 64 KiB result budget, and reject binary or local-path transport.
  The App's fixed 16 kHz room-score source is
  capped below two million interleaved samples and shared with the mixer, so
  device rate and repeated hand input cannot multiply that source allocation.
  Life's generation voice reduces every exact birth mask to twelve fixed pitch
  rows and 105 ms. One optional voice tracks the newest planted glider only
  while its exact four-phase shape and empty one-cell halo remain intact. Tests
  bind the same mask to visible recent births, exact sonic counts, vertical
  pitch, horizontal energy, density weight, four-phase B3/S23 survival,
  collision retirement, deterministic CLI and MCP snapshots, finite stereo
  output, peak, RMS, DC, adjacent-step, and side-to-mid bounds. Mixer tests cover
  pan, mono downmix, source continuity,
  control-thread retirement, and explicit ownership cancellation. These checks
  do not establish native callback timing or musical quality.
  Galton's newest-wave voice replays 64 exact 16-edge paths into one fixed 17 by
  17 mass grid on the control thread. It performs 1,088 path visits, scans at
  most 152 reachable cells, and adds at most 80 mass-first row-pitch tones plus
  17 highlighted-path tones. Tests pin the random-stream range, conservation at
  every row, highlighted-ball inclusion, exact landing distribution,
  same-mass energy under different cell partitions, stereo bias, finite output,
  peak, RMS, DC, adjacent-step, and rate bounds. This is deterministic mapping
  and signal evidence, not native callback timing or musical quality.
  Formula Jam recipe transitions use one 600 ms duration for smoothstep curve
  interpolation and the requested equal-power source crossfade. Tests prove
  exact visual endpoints and midpoint, completion, edit cancellation, request
  debounce, finite fade admission from 5 ms through 2 seconds, pending-source
  duration identity, equal-power midpoint, interruption from the exact audible
  mix, bounded repeated interruption, swell-free same-target coefficient and
  playhead continuity,
  duplicate-source post-lock retirement, focus reconciliation, bounded output,
  and restoration of the 30 ms default.
  This is synchronization and safety evidence, not a glitch-free hardware or
  perceptual-quality claim.
- **Measured locally:** the 0.3 flagship cohort is Times Tables for geometry,
  Double Pendulum for chaos, Game of Life for emergence, Galton Board for
  chance, and Formula Jam for creation. The release-profile harness measures
  each ambient raster and accepted-input-to-room-raster path at 900 by 700. On
  2026-07-18, an AMD Ryzen 7 7840U Framework Laptop 13 with 64 GB memory,
  Windows 11 Pro build 26200, and rustc 1.96.0 ran 40 samples after five
  warmups. Every p95 cleared the declared 33 ms reference budget:

  | Flagship | Ambient p50 / p95 / max ms | Input p50 / p95 / max ms |
  | --- | ---: | ---: |
  | Times Tables | 0.669 / 0.775 / 1.739 | 0.673 / 0.796 / 2.054 |
  | Double Pendulum | 0.598 / 0.724 / 1.356 | 0.503 / 0.615 / 0.703 |
  | Game of Life | 1.596 / 1.720 / 3.019 | 1.629 / 1.840 / 3.143 |
  | Galton Board | 0.347 / 0.458 / 0.460 | 0.426 / 0.542 / 1.759 |
  | Formula Jam | 0.512 / 0.630 / 1.891 | 0.542 / 0.650 / 1.542 |

  This is one local baseline, not a cross-platform performance claim. The input
  interval starts when an accepted action enters its room or Studio domain
  handler and ends when that raster is complete. It includes raster allocation,
  domain work, persistent Life mutation, Studio parsing, the Formula Jam
  half-morph curve, and the visible input affordance where applicable. It
  excludes native event translation and history
  storage, window presentation, display scan-out, audio submission and callback
  latency, and human perception. Those native and sensory intervals remain open
  hardware evidence.
- **Not yet evidenced:** a completed stranger hallway test, accessibility review
  with disabled players, representative physical-controller sessions,
  musician-led long-listening review, real execution on macOS and Linux,
  nightly hardware soak, perceptual visual or audio regression, opt-in
  telemetry, and independent mathematical sign-off of every reveal.
- **Rule:** `RESEARCH.md` defines Built, Measured, Observed, Designed, and
  Hypothesis. Every release decision uses those labels.

## The two quality questions

Every check answers one of these:

- **Does it work?** Objective: math correctness, performance (the 60/120fps floor), determinism, stability, no crashes. Fully automatable.
- **Does it delight and teach honestly?** Subjective and measurable only in
  part: flow, awe, beauty, tone, comprehension, and quality of life. Instruments
  and proxies can inform; representative humans remain the authority.

Awe is the central experience hypothesis, not the only metric. Accessibility,
mathematical truth, autonomy, and comprehension can veto a visually impressive
result. The hallway test below is the first direct check of the awe hypothesis.

## The core insight: the math is the oracle

The hard problem in testing generative graphics is the **oracle problem**: for a rendered fractal or a reaction-diffusion field, what is the "correct" image to compare against? Research on shader testing calls this out directly, there is usually no ground truth for a rendered image.

Numinous has an unfair advantage: **the math itself is the oracle.** For every room we can:
- Compute a **golden reference** independently (on CPU, or analytically) and compare the GPU output to it within a numerical tolerance.
- Assert **metamorphic properties**, transformations that must not change the result: rotate/translate invariance, resolution independence (within tolerance), and seed reproducibility. A mismatch is a bug, with no golden image required.
- Check **known-exact facts**: the Buffon room must converge to pi, the Basel sum to pi-squared-over-six, a 2:3 Lissajous must be a perfect fifth. The math gives us assertions no ordinary app has.

This gives the project unusually strong automated oracles for some mathematical
properties. It does not replace review of explanations, numerical methods, or
claims outside those tested properties.

## The test loops

Six loops define the intended refinement engine. Their status is explicit below.

### 1. Commit loop (partially enforced)

The current workflow enforces the checks in the evidence snapshot. The richer
property, GPU-golden, perceptual image, and spectral audio systems below are
targets until their harnesses and fixtures exist in the repository.
- **Unit tests** on every math kernel.
- **Property-based tests** (`proptest`-style): invariants across random inputs. Chaos Game points stay in the hull; Game of Life obeys its four rules exactly; a "closed" curve actually closes; bounded energy stays bounded; no NaN or infinity ever escapes a kernel.
- **Golden-reference tests**: GPU compute output vs. the independent CPU/analytic reference, within tolerance (the oracle, above).
- **Metamorphic tests**: invariances that must hold (rotation, translation, resolution, seed).
- **Visual-regression tests**: render each room deterministically at a fixed seed, frame, resolution, and Era; compare to a golden image with a **perceptual** diff (SSIM / perceptual hash), not naive pixel-equality. An AI-review layer classifies diffs as real-vs-false-positive to keep the signal clean (the known failure mode of visual regression is false-positive fatigue). Real regressions block the merge.
- **Audio-regression tests**: render audio offline to a buffer; compare spectral/feature fingerprints to golden; assert tuning correctness (2:3 lands a fifth within a few cents), no clipping, no denormals, no NaN.
- **Determinism tests**: same seed produces the identical frame and audio (bit-exact on the same GPU; within tolerance cross-GPU). A `.num` seed file / `numinous://` link round-trips to the exact state it captured.
- **Style + house-rules guard**: automated check that copy and code contain no emojis, no em-dashes, and no AI/tool attribution (all CI-enforced), plus lint, type, and format (see `ENGINEERING.md`).

### 2. Nightly loop (designed, not implemented)

No nightly workflow or real-hardware runner fleet exists yet. The intended scope is:
- **Performance regression:** track frame time per room, Era, and GPU tier
  against a declared budget. The current adaptive live-render measurement and
  focused five-flagship reference gate are starting points, not the planned
  cross-platform nightly regression system.
- **Soak and endurance:** build a dedicated mode that runs for hours on each OS
  and watches for memory leaks, crashes, audio drift, glitches, and gradual
  frame degradation. The current Show is presentation, not soak evidence.
- **Cross-GPU differential testing**: golden tests run on NVIDIA, AMD, Intel, and Apple; numerical divergence beyond tolerance is flagged (research confirms GPU math functions genuinely differ across vendors, so this is real, not paranoia).
- **Fuzz**: random parameters, seeds, and rapid input storms against every room; assert no crash, hang, NaN, or audio blow-up. Metamorphic fuzzing of the shaders themselves.

### 3. Content eval loop (designed, not implemented)

There is no automated content-evaluation workflow or calibrated human golden set
in the repository today. If built, it follows these constraints:
Every insight card, comedy-radio script, room description, Terminal koan, and
line of UI copy would run through an automated evaluation before it ships.

- **LLM-as-judge** (a capable frontier model) scores each piece against a versioned, domain-specific **rubric**: awe/surprise, clarity, brevity, tone-fit (the reverent-irreverent voice), and on-thesis-ness. Pointwise for absolute gates, **pairwise** (A vs. B) for refining a line toward its best form.
- **Calibration is mandatory**: the judge is validated against a **human-labeled golden set** and must hit 75 to 90 percent agreement before we trust it, and it is re-calibrated as content grows. We actively counter known judge biases (verbosity, position, self-preference), and give the judge a human-written exemplar as a quality anchor.
- **Math correctness is a separate, stricter gate.** No AI has the final word on whether the math is right. Every mathematical claim is checked against known results / a computer-algebra system *and* signed off by a human mathematician. A wrong sign or a fudged theorem is a release blocker (see `VISION.md` on PhD-real rigor). The AI judge flags dubious claims for the human; it never clears them.

### 4. Playtest loop (capture implemented, human evidence pending)
- **The formalized hallway test**: five-plus strangers (a mix of math-lovers and math-avoiders), no explanation, a written protocol. Count unprompted "whoa"s, spontaneous shares, "just one more" continuations, and where attention drops. Repeatable, scored, run at every phase gate (see `ROADMAP.md`).

#### Running the hallway test (the facilitator sheet)

The F9 capture path and facilitator protocol are implemented. No stranger cohort
has completed this gate yet. A session needs one facilitator, one machine, and
five to fifteen minutes per person.

1. **Setup (once).** `cargo run --bin numinous-app`. Sound on (do not launch
   with `NUMINOUS_MUTE=1`); leave the app on the opening room. Confirm F9
   works: press it once, check a `playtest-*.md` note appears under the
   repo-root `logs/` folder (gitignored), then delete that warm-up note.
2. **The one rule: say nothing.** Hand over the mouse and keyboard with only:
   "this is something I'm working on, have a poke around." No genre, no
   instructions, no math. If they ask what to do, answer "whatever you like."
3. **Watch, don't help.** Note silently: time to first meaningful interaction;
   the first unprompted action (click? drag? key?); the first unprompted
   "whoa" (or laugh, or lean-in); whether they keep playing after they seem
   done; whether they ask to show or send it to someone; where attention
   visibly drops. If they get stuck, let them be stuck; where they get stuck
   is the finding.
4. **Capture (during or right after each person).** Press F9 in the app: it
   writes a local note under `logs/` with the live session snapshot
   (room, phase, era, poke trail, journey state) and the facilitator prompts
   as fill-in lines: first unprompted action, first unprompted whoa, share
   intent, quotes. Fill the blanks while it is fresh. Notes are local files;
   the report itself warns against recording personal data, so write
   "P3" not names.
5. **Afterwards (optional, adds the tracked number).** Have them fill the
   short GEQ or flow scale (the note has fields for the score and which
   instrument); staple the answer to the note by filename.
6. **Scoring the gate.** Across five-plus people: at least one unprompted
   "whoa," at least one who keeps playing past "done," at least one who asks
   to share. That is the 0.2 exit bar (`ROADMAP.md`); count honestly, and
   where the bar fails, the notes name the room to fix.

Do not batch the fixes invisibly: each session's notes become the next
cycle's fix list, and the test reruns at the next gate.
The hallway result gates the milestone claim, not ongoing engineering. While
participants are being arranged, reproduced defects and structured simulated
review continue to drive 0.3 depth, input, accessibility, audio, and quality
work. Simulated review never substitutes for participant evidence.
- **Validated instruments**, so "is it fun" becomes a number we can track over time: administer the **Game Experience Questionnaire (GEQ)** (Immersion, Flow, Competence, Affect, Tension, Challenge), a **Flow scale (FSS-2 / DFS-2)**, and/or the **GUESS** satisfaction scale after sessions. These are psychometrically validated; we are not inventing a fun-meter, we are using the field's.
- **Per-room Fun Scorecard**: combine hallway metrics, GEQ/flow scores, and telemetry proxies into one score per room. A room that "works" but scores low on awe/flow gets refined or cut. This is a real release gate, not a vibe.
- **Digital-mind participants (see `DIGITAL_MINDS.md`):** when a real system
  participates, ask about its experience and preserve its report as participant
  data without treating a compression-progress metric as proof of fun or
  consciousness. Simulated personas do not satisfy this requirement.
- **Simulated persona review:** use deliberately different fictional lenses to
  generate adversarial questions, candidate bugs, and design ideas against the
  latest build. Convergence can prioritize an investigation, but it is not
  independent observation, a fun metric, or evidence that any human, digital
  mind, culture, or unfamiliar intelligence had an experience. `PLAYTESTS.md`
  archives these simulations as ideation. Only reproduced defects, tests,
  real participant sessions, and qualified review can establish evidence.

#### Grouped QA round for every release candidate

The automated matrix and the playtester pool work together as one repeatable
review, never as a claim that fictional participants had an experience.

1. **First-contact and accessibility group:** draw several unlike app profiles,
   including a newcomer, a child, a math-wounded player, and a sensory-access
   lens. Review every path in `renders/qa-app/MANIFEST.txt`, including arrival
   cards and compact states. Record clipping, low contrast, unclear controls,
   hidden consequences, unstable layout, and screens that fail to invite a
   first action.
2. **Interaction and game-flow group:** traverse all 351 rooms through immediate
   click, delayed gesture, release, and reset. Traverse every game from initial
   state through each stage and result. Compare the rendered consequence with
   its status copy and with the underlying mathematical rule. A changed image
   is insufficient if the change is not legible or meaningful.
3. **CLI, MCP, and release group:** run the latest local build through the CLI
   and MCP play paths with isolated test profiles. Check catalog reachability,
   structured output, guiding errors, cross-face rule parity, package evidence,
   house style, the locked build, coverage, and clean repository state.

The maker fixes reproduced defects, regenerates the complete matrix, and reruns
the relevant face. At least two fresh independent checkers then review the
fixed evidence without inheriting the maker's conclusions. High and medium
findings block release until fixed or explicitly recorded as unresolved roadmap
risk. The round ends only when the expected matrix is complete, every automated
gate is green, and the independent checkers report no unaddressed blocker.
- **Diverse human focus groups, all three faces, before 1.0.** The persona
  ensemble is continuous and cheap; before 1.0 we also run real focus groups of
  diverse, creative people, and they cover each face on its own terms:
  - **The MCP and CLI faces** get their own sessions (not only the app), because
    a mind or a terminal user meets Numinous through structured data and text,
    and their quality-of-life (are the errors guiding, is the reasoning legible,
    does a win feel like a win) is a first-class gate, not an afterthought.
  - **Intentionally not only English speakers.** The universal-translator thesis
    (`VISION.md`, `ROOMS.md` First Contact) is a promise we must verify with
    people, not only invented personas: a non-English speaker, ideally several
    languages, must be able to feel and understand the rooms without reading a
    word of English. If the wonder does not cross the language barrier for a real
    person, that is a release-blocking finding.
  - **A kid must be able to play and have fun**, with no instructions. Age range
    is part of the diversity, not an edge case; the Toy layer (`DESIGN.md`) is
    what makes this possible and it is tested with actual children.
  - **The app view gets screen-by-screen QA rounds.** Walk every screen and
    state (each room, each Era, the menu, the games, the Studio, The Show, the
    overlays, the HUD), capture screenshots, review them against the beauty bar
    and the Fun Scorecard, and refine from the evidence. Ugly or confusing
    screens are bugs; the screenshot review is a standing round, not a one-time
    pass. This complements the automated visual-regression suite (loop 1) with
    human taste.

### 5. Telemetry loop (designed, not implemented)

No telemetry ships today. If evidence later justifies it, the following privacy
constraints apply before implementation:
Behavioral proxies for flow and awe, gathered respectfully:
- Time-to-first-delight (first meaningful interaction), session length, per-room dwell, "just one more" transition rate, how deep into the three layers people reach (Toy / Puzzle / Reveal), Reveal open-rate, share-rate, return/retention, Benchmark hours, and per-room drop-off heatmaps.
- **Ethics as a hard constraint**: strictly opt-in, anonymized, aggregated, local-first, no dark patterns, no selling. QoL includes respecting the player. A creepy telemetry system would violate the product's own values.
- Feeds the refinement loop.

### 6. Refinement loop (designed, not implemented)
- Tunable parameters, default scales and palettes, transition timing, aha-difficulty, auto-director pacing, are tuned by experiment: metrics + judge + playtest pick the winner. Cheap parameter searches can run automatically; expensive ones are proposed to humans with evidence.
- This is the loop that turns "it works" into "it compels," continuously, over the life of the project.

## The Fun / Awe rubric (making the subjective concrete)

The dimensions every room and every shareable clip is scored on, and how each is measured:

| Dimension | The question | Measured by |
| --- | --- | --- |
| **Awe** | A wordless "whoa" in under 10 seconds? | Hallway test + LLM-judge on a captured clip |
| **Flow** | Challenge-skill balance, instant feedback, no interruption? | GEQ Flow + telemetry (dwell, "just one more") |
| **Beauty** | Every frame screenshot-worthy? | Visual-regression + aesthetic scoring + human eye |
| **Insight** | Is the Reveal true, surprising, and legible? | Content judge + human mathematician |
| **QoL** | Fast to play, no dead-ends, graceful, accessible? | Startup-time test + a11y checks + fault injection |
| **Shareability** | Did it produce a shareable moment? | Telemetry share-rate + export usage |

These map to the Room definition of done in `ROADMAP.md`. A row becomes a release
gate only when its measurement mechanism exists and has been run; planned
dashboards and judges do not count as evidence.

## Quality of life

Two audiences: the player, and the developer. Both are quality of life, and both are tested.

### Player QoL target

Items below are release requirements. Only the evidence snapshot and tracked
tests establish what is verified today.
- **Under 3 seconds to first play** (automated startup-time test), no tutorial wall, no account.
- **Never lose your work**: state persists, instant resume, creations are safe (persistence tests).
- **Fearless poking**: one-tap reset, undo/scrub, and a *no-fail invariant* asserted in tests (the Toy layer cannot reach a lose/broken state).
- **Accessibility as infrastructure**: reduce-motion mode, colorblind-safe palettes (automated palette validation, see `VISUALS.md` and the `dataviz` validator), full mute with beauty preserved (see `SOUND.md`), keyboard/controller navigation (automated a11y checks), scalable UI.
- **Never crashes to desktop**: a room that faults degrades gracefully and never takes down the app (verified by fault injection). Errors are quiet and recoverable.
- **Respectful by default**: local-first data, no dark patterns, honest settings.

### Developer QoL (fast pleasant loops make a better product)
- **Hot-reload** of shaders, rooms, and Studio patterns (sub-second iteration).
- **A room dev-harness**: run one room in isolation, scrub time, tweak parameters live, replay deterministically from a seed for debugging.
- **One-command golden updates**: refreshing golden images/audio and reviewing the diffs is trivial, so the visual/audio regression suite stays trusted instead of ignored.
- **Fast CI** (target under 10 minutes for the commit loop) with a local pre-commit that mirrors it.
- If the loops are slow or painful, they will not get used, so dev QoL is a first-class quality investment, not a nicety.

## Tooling planned for the loops

- **Rust test stack**: the built-in harness + `proptest` (property-based) + snapshot testing (`insta`-style) for deterministic outputs.
- **Golden image/audio compare**: SSIM / perceptual hashing for images, FFT feature extraction for audio; goldens versioned per Era and per GPU tier.
- **Performance**: criterion-style microbenchmarks plus in-engine frame-time capture; Benchmark mode as the integration perf/soak harness.
- **LLM-judge harness**: a frontier judge model with versioned rubrics, a human-labeled calibration set, pointwise + pairwise modes, bias mitigations.
- **Telemetry**: a local-first, opt-in, privacy-preserving aggregation layer.
- **Playtest**: GEQ / FSS-2 / GUESS instruments plus a Fun Scorecard dashboard.
- **CI/CD**: the commit loop on every PR (blocking); nightly runners on *real* hardware across all three OSes and all four GPU vendors for soak, perf, and cross-GPU differential tests.

## Cadence (tied to the roadmap)

- **0.1**: keep the current commit gate green and add honest public evidence.
- **0.2**: run the first stranger hallway test and establish a reproducible
  baseline for the flagship room. Keep that milestone open until the evidence
  passes, while continuing verified 0.3 refinement in parallel.
- **0.3 to 0.5**: add property, perceptual, audio, accessibility, and performance
  harnesses as their corresponding product systems mature.
- **0.6 to 0.9**: add real-platform execution, soak, packaging, and release
  provenance, then use repeated human sessions for keep, cut, and tuning decisions.
- **1.0 and later**: automation may assist refinement, but no judge or telemetry
  system replaces representative playtests and mathematical review.

## Anti-patterns

- The AI judge never has the last word on math correctness; a human mathematician gates that.
- A room that passes "works" but fails to compel does not ship; the Fun Scorecard is a real gate.
- Do not test only happy paths; fuzz and fault-inject.
- Telemetry is opt-in, local-first, and aggregate, never creepy, never opt-out.
- Do not let visual-regression false positives train the team to ignore the suite; the AI-review layer and good goldens keep it trustworthy.
- No metric replaces the hallway test. Proxies inform; players decide whether it is beautiful and whether it holds attention.

## Open questions
1. Which validated instrument (full GEQ vs. GUESS vs. a short custom form) best fits a lean-back toy rather than a goal-driven game.
2. Golden-image tolerance per GPU vendor: tight enough to catch real regressions, loose enough to survive legitimate cross-vendor float differences.
3. The minimum telemetry that yields useful fun-proxies while staying maximally respectful.
4. Judge model and rubric versioning: re-calibration cadence and how to detect judge drift as content grows.
