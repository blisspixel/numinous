# Quality, Testing & Fun-Evals

How Numinous stays exceptional, automatically, from the first commit. Most software tests only whether it *works*. Numinous also tests whether it *slaps*, and builds both into automated loops so quality is a ratchet that only tightens, never a scramble before launch.

## The two quality questions

Every check answers one of these:

- **Does it work?** Objective: math correctness, performance (the 60/120fps floor), determinism, stability, no crashes. Fully automatable.
- **Does it slap?** Subjective but measurable: fun/flow, awe, beauty, tone, quality-of-life. Automatable *in part* (proxies, judges, instruments), with humans as the final authority.

The single metric that outranks all others is **awe** (the hallway test, below). Everything else exists so we are never flying blind between hallway tests, and so a regression in beauty or fun is caught as reliably as a crash.

## The core insight: the math is the oracle

The hard problem in testing generative graphics is the **oracle problem**: for a rendered fractal or a reaction-diffusion field, what is the "correct" image to compare against? Research on shader testing calls this out directly, there is usually no ground truth for a rendered image.

Numinous has an unfair advantage: **the math itself is the oracle.** For every room we can:
- Compute a **golden reference** independently (on CPU, or analytically) and compare the GPU output to it within a numerical tolerance.
- Assert **metamorphic properties**, transformations that must not change the result: rotate/translate invariance, resolution independence (within tolerance), and seed reproducibility. A mismatch is a bug, with no golden image required.
- Check **known-exact facts**: the Buffon room must converge to pi, the Basel sum to pi-squared-over-six, a 2:3 Lissajous must be a perfect fifth. The math gives us assertions no ordinary app has.

This is what lets us hold PhD-grade rigor (see `VISION.md`) as an automated gate, not a hope.

## The test loops

Six loops, each on its own trigger and cadence. Together they are the refinement engine.

### 1. Commit loop (CI, every push and PR, fast and blocking)
- **Unit tests** on every math kernel.
- **Property-based tests** (`proptest`-style): invariants across random inputs. Chaos Game points stay in the hull; Game of Life obeys its four rules exactly; a "closed" curve actually closes; bounded energy stays bounded; no NaN or infinity ever escapes a kernel.
- **Golden-reference tests**: GPU compute output vs. the independent CPU/analytic reference, within tolerance (the oracle, above).
- **Metamorphic tests**: invariances that must hold (rotation, translation, resolution, seed).
- **Visual-regression tests**: render each room deterministically at a fixed seed, frame, resolution, and Era; compare to a golden image with a **perceptual** diff (SSIM / perceptual hash), not naive pixel-equality. An AI-review layer classifies diffs as real-vs-false-positive to keep the signal clean (the known failure mode of visual regression is false-positive fatigue). Real regressions block the merge.
- **Audio-regression tests**: render audio offline to a buffer; compare spectral/feature fingerprints to golden; assert tuning correctness (2:3 lands a fifth within a few cents), no clipping, no denormals, no NaN.
- **Determinism tests**: same seed produces the identical frame and audio (bit-exact on the same GPU; within tolerance cross-GPU). A `.num` seed file / `numinous://` link round-trips to the exact state it captured.
- **Style + house-rules guard**: automated check that copy and code contain no emojis, no em-dashes, and no AI/tool attribution (all CI-enforced), plus lint, type, and format (see `ENGINEERING.md`).

### 2. Nightly loop (soak, cross-platform, performance, on real hardware)
- **Performance-regression**: frame-time per room, per Era, per GPU tier, tracked against the budget; a regression below the 60fps floor fails the night. The **Benchmark mode is the perf harness** (see `DESIGN.md`), it already stress-runs the heaviest work.
- **Soak / endurance**: Benchmark mode runs for *hours* on each OS. Watches for memory leaks, crashes, audio drift or glitching, and gradual frame degradation. The "watch it for hours while high" feature is also, for free, the stability test.
- **Cross-GPU differential testing**: golden tests run on NVIDIA, AMD, Intel, and Apple; numerical divergence beyond tolerance is flagged (research confirms GPU math functions genuinely differ across vendors, so this is real, not paranoia).
- **Fuzz**: random parameters, seeds, and rapid input storms against every room; assert no crash, hang, NaN, or audio blow-up. Metamorphic fuzzing of the shaders themselves.

### 3. Content eval loop (LLM-as-judge + human, on any content change): the "math is cool" eval
Every insight card, comedy-radio script, room description, Terminal koan, and line of UI copy runs through an automated evaluation before it ships.

- **LLM-as-judge** (a capable frontier model) scores each piece against a versioned, domain-specific **rubric**: awe/surprise, clarity, brevity, tone-fit (the reverent-irreverent voice), and on-thesis-ness. Pointwise for absolute gates, **pairwise** (A vs. B) for refining a line toward its best form.
- **Calibration is mandatory**: the judge is validated against a **human-labeled golden set** and must hit 75 to 90 percent agreement before we trust it, and it is re-calibrated as content grows. We actively counter known judge biases (verbosity, position, self-preference), and give the judge a human-written exemplar as a quality anchor.
- **Math correctness is a separate, stricter gate.** No AI has the final word on whether the math is right. Every mathematical claim is checked against known results / a computer-algebra system *and* signed off by a human mathematician. A wrong sign or a fudged theorem is a release blocker (see `VISION.md` on PhD-real rigor). The AI judge flags dubious claims for the human; it never clears them.

### 4. Playtest loop (human, at every phase gate and continuously): the real fun eval
- **The formalized hallway test**: five-plus strangers (a mix of math-lovers and math-avoiders), no explanation, a written protocol. Count unprompted "whoa"s, spontaneous shares, "just one more" continuations, and where attention drops. Repeatable, scored, run at every phase gate (see `ROADMAP.md`).
- **Validated instruments**, so "is it fun" becomes a number we can track over time: administer the **Game Experience Questionnaire (GEQ)** (Immersion, Flow, Competence, Affect, Tension, Challenge), a **Flow scale (FSS-2 / DFS-2)**, and/or the **GUESS** satisfaction scale after sessions. These are psychometrically validated; we are not inventing a fun-meter, we are using the field's.
- **Per-room Fun Scorecard**: combine hallway metrics, GEQ/flow scores, and telemetry proxies into one score per room. A room that "works" but scores low on awe/flow gets refined or cut. This is a real release gate, not a vibe.
- **Digital-mind playtesters (see `DIGITAL_MINDS.md`)**: the experience is also evaluated for digital minds. Their "fun" has a rigorous proxy (learning / compression progress, per Schmidhuber's formal theory), and, just as importantly, we *ask them* about their experience and treat the answer as first-class playtest data, not a curiosity.

### 5. Telemetry loop (in-product, opt-in, local-first): fun proxies at scale
Behavioral proxies for flow and awe, gathered respectfully:
- Time-to-first-delight (first meaningful interaction), session length, per-room dwell, "just one more" transition rate, how deep into the three layers people reach (Toy / Puzzle / Reveal), Reveal open-rate, share-rate, return/retention, Benchmark hours, and per-room drop-off heatmaps.
- **Ethics as a hard constraint**: strictly opt-in, anonymized, aggregated, local-first, no dark patterns, no selling. QoL includes respecting the player. A creepy telemetry system would violate the product's own values.
- Feeds the refinement loop.

### 6. Refinement loop (auto-tuning and experiments)
- Tunable parameters, default scales and palettes, transition timing, aha-difficulty, auto-director pacing, are tuned by experiment: metrics + judge + playtest pick the winner. Cheap parameter searches can run automatically; expensive ones are proposed to humans with evidence.
- This is the loop that turns "it works" into "it slaps," continuously, over the life of the project.

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

These map directly onto the **Room "definition of done"** in `ROADMAP.md`: wherever possible, each done-checklist item is backed by one of these automated or semi-automated checks, so "done" means "measured," not "looks fine to me."

## Quality of life

Two audiences: the player, and the developer. Both are quality of life, and both are tested.

### Player QoL (and how it is verified)
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

## Tooling for the loops

- **Rust test stack**: the built-in harness + `proptest` (property-based) + snapshot testing (`insta`-style) for deterministic outputs.
- **Golden image/audio compare**: SSIM / perceptual hashing for images, FFT feature extraction for audio; goldens versioned per Era and per GPU tier.
- **Performance**: criterion-style microbenchmarks plus in-engine frame-time capture; Benchmark mode as the integration perf/soak harness.
- **LLM-judge harness**: a frontier judge model with versioned rubrics, a human-labeled calibration set, pointwise + pairwise modes, bias mitigations.
- **Telemetry**: a local-first, opt-in, privacy-preserving aggregation layer.
- **Playtest**: GEQ / FSS-2 / GUESS instruments plus a Fun Scorecard dashboard.
- **CI/CD**: the commit loop on every PR (blocking); nightly runners on *real* hardware across all three OSes and all four GPU vendors for soak, perf, and cross-GPU differential tests.

## Cadence (tied to the roadmap)

- **Phase 0**: stand up the commit-loop skeleton as part of the foundation, unit, golden-reference, determinism, the visual-regression harness, the style guard, and CI. Test infrastructure is built with the engine, never bolted on later.
- **Phase 1 (vertical slice)**: the flagship room ships with full "does it work" coverage *and* passes the first formal hallway test + GEQ, establishing the Fun Scorecard baseline. We prove the loops on one room before scaling.
- **Phase 2 (MVP)**: all six loops live; content eval loop online; opt-in telemetry shipping; nightly soak and cross-GPU running.
- **Phase 3+**: the refinement/auto-tuning loop drives per-room scorecards and keep/cut decisions; the judge is continuously re-calibrated.

## Anti-patterns

- The AI judge never has the last word on math correctness; a human mathematician gates that.
- A room that passes "works" but fails "slaps" does not ship; the Fun Scorecard is a real gate.
- Do not test only happy paths; fuzz and fault-inject.
- Telemetry is opt-in, local-first, and aggregate, never creepy, never opt-out.
- Do not let visual-regression false positives train the team to ignore the suite; the AI-review layer and good goldens keep it trustworthy.
- No metric replaces the hallway test. Proxies inform; humans decide whether it is beautiful and whether it slaps.

## Open questions
1. Which validated instrument (full GEQ vs. GUESS vs. a short custom form) best fits a lean-back toy rather than a goal-driven game.
2. Golden-image tolerance per GPU vendor: tight enough to catch real regressions, loose enough to survive legitimate cross-vendor float differences.
3. The minimum telemetry that yields useful fun-proxies while staying maximally respectful.
4. Judge model and rubric versioning: re-calibration cadence and how to detect judge drift as content grows.
