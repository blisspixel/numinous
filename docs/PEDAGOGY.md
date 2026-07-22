# Pedagogy: the understanding layer

How Numinous turns "cool" into "I actually understand this now," grounded in
learning science and the psychology of wonder. This doc owns the science of how
understanding and awe are produced and verified. It supersedes the thin "Layer 3"
notes in `DESIGN.md` and the delivery notes in `INSIGHTS.md`, and it is the home
of the keystone mechanic named in `NORTH_STAR.md`. See `RESEARCH.md` for the
broader evidence base and `QUALITY.md` for the measurement loops.

## The thesis, and the one risk

Numinous's default order, explore first, tell later, is the single best-supported
sequence in the field, not a stylistic choice. Learners who wrestle with a
phenomenon before being told the principle acquire the deep structure and
transfer far better than learners told first (Schwartz and Bransford, "A Time For
Telling," 1998; Kapur's productive failure, 2014). "Toy, then optional reveal" is
the correct pedagogical grammar, and we should say so with sources, because it is
a competitive moat most edtech gets backwards.

The real risk is narrower and sharper than "people leave understanding nothing."
It is the **fluency illusion**. Deslauriers et al. (2019, PNAS) showed that
smoothness and delight get mistaken for understanding: learners *feel* they
learned even when they did not. Numinous's specific failure mode is awe without
accommodation: a gorgeous, frictionless Watch or Toy session produces a strong
feeling of insight ("math is doing that!") with no restructuring of the learner's
model at all. The telemetry we plan (reveal open-rate, dwell, share-rate)
measures delight, and delight will look great, and it will not detect this. We
could ship a product that aces the hallway test and teaches nothing, and our
instruments would not tell us.

The fix is not homework. The fix is **one small act of generation per genuine
insight**, because generation is the mechanism that converts watching into
understanding. That is the through-line of this whole document.

## The keystone: the prediction wager

Before a toy resolves or a reveal fires, invite a single-gesture guess: drag a
marker to "where you think pi is," tap "which corner rule makes a triangle,"
place a dot "where the thousandth ball lands." This is the highest-leverage
mechanic in the product and should be built before anything else, because three
literatures converge on it:

- **Predict-Observe-Explain** (White and Gunstone, 1992): a canonical
  conceptual-change technique.
- **The generation effect** (Slamecka and Graf, 1978): self-generated answers
  are remembered far better than read ones.
- **Information-gap theory** (Loewenstein, 1994): curiosity is literally the felt
  gap between a guess and the truth.

A wrong prediction is the pedagogical jackpot: it opens the gap that makes the
reveal land as insight instead of trivia. The cost is near zero (one gesture,
reusing the challenge grader), and it turns a passive spectator into someone with
a stake.

**The same verb serves digital minds.** A mind commits its model of the hidden
rule ("this closes into three loops," "the population period-doubles past
r=3.57," "the next row is Rule 110"); the reveal grades the gap as compression
progress, not pass/fail. High confidence plus correct is mastery (a boredom
signal); wrong-but-close is the fertile band; random is noise. This is
Schmidhuber's account of fun (learning progress) made into a single legible act,
and it is the atom the digital-mind features in `DIGITAL_MINDS.md` build on (the
compression ledger, self-authored goals, band-matched difficulty). One mechanic,
both minds. See `NORTH_STAR.md` for why this convergence is the plan's spine.

## The reveal, re-specced as an engineered aha

The current reveal is a delayed, well-written fact card: good copy delivering a
*conclusion*. But an aha is not a conclusion received; it is a representation the
learner *restructures*, and that restructuring is what makes it stick and what
feels like awe (Kounios and Beeman, *The Eureka Factor*; insight-solved problems
are better remembered and more often correct than analytically-solved ones). So
the reveal becomes a **five-beat staged event**. Keep it one screen, keep it
summoned not pushed, keep the great copy, but wrap it in structure:

1. **Prime the gap.** Surface what the player implicitly expects, via a
   prediction wager or an anomaly beat ("this floor has no circles"). No gap, no
   aha, only a fact.
2. **Withhold and earn.** The reveal is summoned when the player is ready, and
   only after at least one generation act, so it lands on a prepared mind.
3. **Restructure by showing, not telling.** The bridge is *animated*, not
   asserted: the player watches their own object become the other object. This is
   compression made visible, two models collapsing into one.
4. **Confirm by the player's own hand.** Hand control back: let them wiggle the
   parameter and watch both sides move together. Re-deriving it themselves is the
   generation act that converts watching into knowing.
5. **Consolidate and leave the door open.** The Constellation edge lights (spaced
   re-encounter fuel), the copy delivers the punchline and the open mystery, the
   audio resolves to consonance on the exact frame.

The rule for writers: **the copy is the punchline, not the payload.** The payload
is beats 1, 3, and 4, the gap, the morph, and the player's own hand. The words
arrive last and confirm what the player already felt.

### Canonical engineered ahas

- **Times Tables to Mandelbrot (the flagship).** *App vertical slice Built
  (machine path; stranger hallway still open).* Technical Toy remains
  (K=2 hold, integer snap, earned K=5, three-face agreement). The ordinary App
  visit now stages the five-beat engineered aha instead of opening a text card
  on E first. Prime: after a hand-held K=2 heart, status and bottom marks invite
  1=Mandelbrot / 2=Nephroid / 3=Circle (keys or bottom-band click). Withhold:
  reveal text stays closed until a generation act (place wager or four-lobe
  goal). Restructure: E summons a deterministic cardioid-to-Mandelbrot morph
  (core pure state; App owns wall-clock progress). Confirm: the dial drives
  chords and a bead on the Mandelbrot-frame outline together. Consolidate: E
  opens the existing reveal copy as punchline. The Show still sweeps and does
  not auto-earn. Core module: `rooms/times_tables_aha.rs`.
- **Buffon's Needle to pi.** Prime: "This floor has no circles. Drag your guess
  for what number the crossings settle on." (Almost nobody guesses pi.)
  Restructure: as the tally converges, a circle grows out of the needle geometry
  to show why pi was always hiding. The wrong guess is what makes pi's arrival
  uncanny.
- **Galton Board to the binomial and its normal approximation.** Prime: "Drop
  your bet: where does this one ball land?" Restructure: 64-ball waves build an
  empirical pile against a distinct exact `Binomial(16, p)` outline without
  pretending finite samples are identical. The built Toy provides five fixed
  probabilities, contiguous deterministic runs, exact totals, and reset. The
  one-ball prediction wager is live as a move-committed bin bet graded against
  the highlighted last ball of the next wave (Toy-layer status, not a separate
  Puzzle face). More waves make the empirical frequencies estimate the fixed
  binomial; the Central Limit Theorem connection is the separate many-row
  normal approximation, not a claim that sample count changes the landing
  distribution.

## The mechanic library

Beyond the keystone, ranked by leverage (impact on genuine understanding and
wonder per unit build). Each names its principle so writers and engineers share a
reference.

1. **Contrasting cases at the reveal.** Place two surface-different cases side by
   side and let shared structure pop (Schwartz and Bransford). The Constellation
   is an idle contrasting-case engine: connection insights are exactly this.
2. **Multisensory click, timed to the frame.** Audio resolves to consonance at
   the instant the reveal restructures. The ear confirming the eye is a second,
   synchronized click, nearly free given the audio bus.
3. **Self-explanation smuggled into Share.** An optional one-line "in your words,
   what just happened?" The self-explanation effect (Chi et al., 1994) is among
   the most robust findings in the field; the caption doubles as a generation
   act, a shareable payload, and a telemetry signal.
4. **Vastness cues, engineered on purpose.** Awe is perceived vastness that
   exceeds your frameworks and demands accommodation (Keltner and Haidt, 2003). A
   live zoom-depth counter falling forever, "you are hearing digit 47 of 100
   trillion," makes the vastness legible alongside the rule's simplicity. The gap
   between tiny rule and vast result is the awe; show both ends at once.
5. **The anomaly beat.** Name the expectation, break it, then resolve (Berlyne's
   collative variables). "Buffon's Needle produces pi with no circle anywhere" is
   an anomaly staged before its resolution.
6. **Learning-progress pacing (the curiosity thermostat).** Curiosity peaks not
   at novelty or mastery but where predictions are measurably improving (Oudeyer
   and Gottlieb; Metcalfe's region of proximal learning). The auto-director and
   room recommender should steer toward the room where *this* player is making
   prediction-progress. This is the same compression-progress metric that paces
   digital minds, so one thermostat serves both.
7. **Retrieval and spacing via re-encounters.** When a player enters a room
   connected to one seen days ago, surface a silent re-encounter of the earlier
   insight (a Constellation edge lighting), not a quiz. Spaced effortful recall
   without the schooliness.
8. **Manipulate the rule, not just the output.** Where possible make the tiny
   rule itself the draggable object (the CA rule bits, the L-system grammar
   string), so the "trivial rule to cosmic result" gap is something the hand
   crossed (Bret Victor; Chi's interactive tier).
9. **Scaffolded discovery with fading hints, for the Puzzle only.** Pure
   unguided discovery overloads novices on high-element-interactivity material
   (the cognitive-load tradition). The Toy is safe (no goal, no load); the Puzzle
   needs generous first hints that withdraw, or the boss rooms become frustration.
10. **Open-door endings.** The information gap is sustained when a door is left
    ajar ("nobody can prove Collatz"). Keep it a hard rule on every reveal; it is
    what keeps curiosity metabolizing after the session ends.

## Measuring understanding and awe

`QUALITY.md` is strong on "does it work" and on flow and awe proxies, but it has
no measure of genuine understanding, and its awe measure is the hallway "whoa"
count. Two additions, both bolting onto existing loops.

**Awe (extend the playtest loop).** Add one validated instrument alongside
GEQ/FSS-2: the Awe Experience Scale (AWE-S, Yaden et al., 2019), short form,
capturing the two load-bearing dimensions, vastness and need for accommodation.
Add two cheap behavioral proxies: self-reported chills (a validated awe marker)
and the small-self measure (Piff et al., 2015). Awe becomes a tracked number with
construct validity, not only a "whoa" tally.

**Understanding without tests (the gap).** Three layers, none school-like:

1. **The aha self-report (per reveal).** A four-item micro-scale from insight
   research: suddenness, surprise, confidence, pleasure. One optional swipe. High
   scores predict correctness and retention in the lab, so this is an
   understanding proxy, not a satisfaction proxy.
2. **The transfer probe (the honest eval).** After a room, present a novel
   configuration and ask the player to predict its behavior. Transfer, not
   recall, is the field's gold standard for conceptual understanding (Kapur).
   Prediction accuracy on an unseen case is the "did this teach anything" number,
   administered as play. This is also what the optional Puzzle already is.
3. **Caption analysis (at scale).** Run optional Share captions through the
   LLM-as-judge harness with a new rubric dimension: does the self-explanation
   name the deep structure or only restate the surface? "Random dots made a
   triangle" is surface; "the pattern was in the rule, not the randomness" is
   structure (Chi's ICAP distinction). A continuous, opt-in understanding signal
   that delight metrics cannot fake.

**The guard rail.** Bake the Deslauriers finding in as an explicit anti-pattern:
reveal-open-rate and dwell measure delight, never understanding. Any claim that a
room "teaches" must be backed by transfer-probe or caption-structure data, the
same way `QUALITY.md` already forbids the AI judge from clearing math
correctness. Delight informs; a generation-based measure decides.

New rubric row for the Fun and Awe table:

| Dimension | The question | Measured by |
|---|---|---|
| Understanding | Did their model actually restructure (can they predict a new case)? | transfer-probe accuracy, caption structure-vs-surface, aha self-report |

## Anti-patterns

- No fact-card reveals. Every reveal is the five-beat engineered aha.
- No crediting delight as understanding. Nothing is "collected" without a
  generation act.
- No unguided discovery in the Puzzle without fading hints.
- No reveal without a prior gap (a prediction or an anomaly).
- No open problem stated without a live door (see `OPEN_DOORS.md`, planned).

## Sources

- Schwartz and Bransford, "A Time For Telling," 1998; Kapur, productive failure,
  Cognitive Science, 2014.
- Deslauriers et al., feeling-of-learning vs actual learning, PNAS, 2019:
  https://www.pnas.org/doi/10.1073/pnas.1821936116
- Loewenstein, information-gap theory, 1994.
- Slamecka and Graf, the generation effect, 1978; Chi et al., self-explanation,
  1994; the ICAP framework.
- Kounios and Beeman, *The Eureka Factor* (insight neuroscience and the memory
  advantage of aha).
- Oudeyer, Gottlieb and Lopes, learning progress and intrinsic motivation, 2016:
  https://www.pyoudeyer.com/oudeyerGottliebLopesPBR16.pdf
- Keltner and Haidt, awe as vastness plus accommodation, 2003; Piff et al., small
  self, 2015: https://www.apa.org/pubs/journals/releases/psp-pspi0000018.pdf
- Cognitive-load caveat on unguided discovery: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC6099118/
- Bret Victor, Explorable Explanations: https://worrydream.com/ExplorableExplanations/
