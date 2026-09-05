# Pedagogy: the understanding layer

How Numinous aims to connect exploration with understanding, informed by
learning science and the psychology of wonder. This document separates design
hypotheses from evidence needed to claim learning or awe. It supersedes the thin "Layer 3"
notes in `DESIGN.md` and the delivery notes in `INSIGHTS.md`, and it is the home
of the keystone mechanic named in `NORTH_STAR.md`. See `RESEARCH.md` for the
broader evidence base and `QUALITY.md` for the measurement loops.

## The thesis, and the one risk

Exploration followed by targeted explanation is a promising sequence whose
benefit depends on prior knowledge, task structure, guidance, and feedback.
Structured contrasting cases prepared students for later explanation in
[Schwartz and Bransford, 1998](https://doi.org/10.1207/s1532690xci1604_4).
Conceptual instruction first improved mathematical knowledge in a randomized
study of 122 children by [Fyfe et al., 2014](https://doi.org/10.1111/bjep.12035).
Neither establishes a universally best order. Toy-first remains Numinous's
default invitation, with observation, guidance, and expert entry treated as
legitimate choices. The 0.4 comparison tests the sequence here.

The real risk is narrower and sharper than "people leave understanding nothing."
It is the **fluency illusion**. Deslauriers et al. (2019, PNAS) showed that
felt learning and measured learning can diverge in active and passive classroom
conditions. A corresponding Numinous risk is a feeling of insight without
transfer to a new case. That is a hypothesis about this game, not that study's
finding. Reveal-open rate, dwell, and sharing measure behavior; they do not
directly measure delight or understanding. A satisfying Watch session is a
valid outcome even when no learning claim is made.

An optional prediction, construction, or explanation can expose a player's
current model and create useful feedback. It cannot guarantee understanding.
A learning claim needs transfer and retention evidence; a player need not earn
the right to enjoy the room by supplying that evidence.

## Play first, depth by choice

The default room is free play. A player may stay, experiment, watch, or create
without taking a quiz or reading an explanation. The current source provides
the same optional study content through the App reader, CLI `study`, and MCP
`study_room`. [STUDY.md](STUDY.md) gives commands, controls, and availability.

- **Explanation** offers a short account or the pilot's experiment and intuition.
- **Notes** carries existing room prose, advanced notes, and citations.
- **Mathematics** requires an authored treatment: assumptions, derivations,
  examples, limits, and primary references. Lissajous is the first treatment;
  an unavailable depth is named explicitly. A citation alone does not meet it.

All three depths are directly selectable. Reading has no level, visit, wager,
consolidation, or prior-reading requirement, awards no reward, and does not
mutate Journey. App **E** / **?** or Cabinet **EXPLAIN** opens the reader with
room state retained. The CLI and MCP study requests do not need a player profile.
The older CLI `reveal` and MCP `reveal_room` preserve their existing visit,
consolidation, and deep-cut progression rules; experimental collectors continue
to use their declared protocols.

The seven staged App paths now require a separate choice through **U** or
Cabinet **EXPERIMENT**. Their predictions, observation alternatives, earned
connections, and rewards belong to that path. **Enter** advances an earned
connection when offered; **U** or **Esc** returns to free play while retaining
calls and earned progress. **E** opens study without completing the experiment.
The Show remains a separately selected presentation that can display reveal
text near the end of a room. None of these choices establishes understanding.

Lissajous's English treatment has a Japanese `reviewed_draft`; its original
catalog notes remain explicitly English. Mathematical and text review of that
draft do not establish native-speaker usability or learning. Hawaiian and
Klingon content, broader room coverage, and full App localization remain
unfinished; [ROSETTA.md](ROSETTA.md) distinguishes rendering from translation.

## The keystone: the prediction wager

Before a toy resolves or a reveal fires, invite a single-gesture guess: drag a
marker to "where you think pi is," tap "which corner rule makes a triangle,"
place a dot "where the thousandth ball lands." The generic verb and seven staged
room ahas now ship. Three research traditions motivate testing their value:

- **Predict-Observe-Explain** (White and Gunstone, 1992): a canonical
  conceptual-change technique.
- **The generation effect** (Slamecka and Graf, 1978): self-generated answers
  are remembered far better than read ones.
- **Information-gap theory** (Loewenstein, 1994): curiosity is literally the felt
  gap between a guess and the truth.

A wrong prediction can create a useful question when feedback helps explain
the discrepancy. It can also reflect ambiguous controls, insufficient guidance,
or a poor question. Do not require failure, confidence, or surprise as proof of
engagement. Let a knowledgeable player proceed to a new application.

**The same option is available to digital minds.** `predict.rs` reports absolute
error and closeness relative to a sampled readout span. NAILED, CLOSE, and WILD
describe one answer, not mastery, boredom, noise, learning progress, or pleasure.
Repeated comparable transfer tasks would be needed to claim improvement; an
actual model or coding criterion would be needed to claim compression.
[Schmidhuber's formal proposal](https://doi.org/10.1109/TAMD.2010.2056368)
concerns improvement in a defined predictor, not one error value. A player can
also choose familiar beauty, performance, company, or creation.

## The optional engineered aha

A short explanation and a staged discovery serve different choices. The latter
aims to help a player restructure a representation through their own experiment.
Insight research motivates testing this aim (Kounios and Beeman,
*The Eureka Factor*); shipping the sequence does not establish learning or awe.
The **five-beat staged event** is one optional route, not the required form of
every explanation. Keep each beat legible on one screen:

1. **Prime the gap.** Surface what the player implicitly expects, via a
   prediction wager or an anomaly beat ("this floor has no circles"). No gap, no
   aha, only a fact.
2. **Build suspense within the chosen experiment.** A generation or observation
   act can prepare the staged reveal. This sequencing belongs to that path;
   a deliberately requested explanation or rigorous treatment remains available.
3. **Restructure by showing, not telling.** The bridge is *animated*, not
   asserted: the player watches their own object become the other object. This is
   compression made visible, two models collapsing into one.
4. **Confirm by the player's own hand.** Hand control back: let them wiggle the
   parameter and watch both sides move together. A new case can test whether
   they can use the relationship; repeating a gesture alone cannot establish it.
5. **Consolidate and leave the door open.** The Constellation edge lights (spaced
   re-encounter fuel), the copy delivers the punchline and the open mystery, the
   audio resolves to consonance on the exact frame.

For this staged path, the gap, morph, and player's own hand carry the event;
the closing words name the relationship. The independent study door can begin
with the relationship and its proof when that is what the player wants.

### Canonical engineered ahas

- **Times Tables to Mandelbrot (the flagship).** *0.2 exit met on App + MCP
  agent-and-machine evidence. Human stranger hallway deferred to 0.8 / 1.0.*
  Technical Toy remains (K=2 hold, integer snap, earned K=5, three-face
   agreement). Choosing EXPERIMENT stages the five-beat App path.
  Prime: after a hand-held K=2 heart, status and bottom marks invite
  1=Mandelbrot / 2=Nephroid / 3=Circle (keys or bottom-band click; MCP
   `place_wager`). The chosen path earns its connection through its own rules;
   unrestricted study stays available. Restructure: Enter / `aha_summon` morphs
   cardioid to Mandelbrot. Confirm and
  consolidate follow. The Show does not auto-earn. Core:
  `rooms/times_tables_aha.rs`. Agent cohort: `scripts/agent-hallway.py`.
- **Buffon's Needle to pi.** *0.2 exit met on the same agent-and-machine bar.
  Human strangers deferred to 0.8 / 1.0.* Second room on the
  generation-before-reveal pattern. Prime: after the first throw, status and a
  bottom number line invite a guess on 1.5..4.5 (MCP `number_wager`). Withhold,
  morph, confirm, consolidate mirror Times Tables. Core: `rooms/buffon_aha.rs`.
- **Galton Board to the binomial (the third flagship).** The room's older
  one-ball bet is still there in the Toy layer, and it grades luck: one
  stochastic landing tells a player nothing about their model. The staged aha
  above it asks a model-level question instead. Prime: after the first wave,
  "where will the whole pile peak?" with a bin ruler along the bottom (MCP
  `bin_wager`). Withhold, morph, confirm, consolidate mirror the other two:
  the exact `Binomial(16, p)` outline grows over the pile outward from the
  true peak, so the answer to the call arrives first, and consolidation
  speaks one graded sentence against the binomial's mode in predict's
  non-punitive bands. Four waves earn the withheld beat without a call, for
  the player who would rather run the experiment than name its answer.
  A call belongs to an experiment: waves on the same coin are more evidence
  for it, waves on another coin are a different experiment, so the curve is
  drawn only over the pile it explains and every sentence names the pile it
  read. Core: `rooms/galton_aha.rs`. More waves make the empirical
  frequencies estimate the fixed binomial; the Central Limit Theorem
  connection is the separate many-row normal approximation, not a claim that
  sample count changes the landing distribution.
- **Double Pendulum to the prediction horizon (the fourth flagship).** One
  completed release primes the call: does the shadow twin end together,
  drifted, or lost? The App maps the three endings onto keys and a bottom band;
  MCP uses `ending_wager` after a gesture with a real release. The withheld
  beat keeps the answer closed, then the morph draws the divergence gap from
  flat to wall. The call is graded against the exact newest release's angles
  and velocity through the same bounded integration the room renders. A held
  bob is not a release, and four completed releases earn the experiment path
  without demanding a prediction. The fertile miss is together: determinism
  is real, yet it does not grant a useful forecast. Core:
  `rooms/pendulum_aha.rs` and `rooms/double_pendulum.rs`.
- **Kepler Areas to equal time (the fifth flagship).** One completed tuning
  primes faster, slower, or same near the sun. The App maps those relations to
  keys and a bottom band; MCP uses `speed_wager` after a poke or completed
  gesture. The call binds to the selected eccentricity. A circle truthfully
  answers same because it has no nearer side; a noncircular ellipse answers
  faster and reports the model's perihelion-to-aphelion speed ratio, with
  rounded prose identified as approximate. The morph
  places positions at equal mean-anomaly intervals, found by solving Kepler's
  equation, so spacing rather than prose reveals the speed change. Four
  completed tunings earn the observation path without forcing a prediction.
  Core: `rooms/kepler_aha.rs` and `rooms/kepler_laws.rs`.
- **Parrondo's Trap to residue steering (the sixth flagship).** One completed
  rule selection primes A, B, or ABB. The App maps those policies to keys and
  a bottom band; MCP uses `policy_wager` after a poke or completed gesture.
  The morph draws exact expected-capital paths from a three-state Markov chain,
  not a fortunate sample. At the room's established probabilities, A and B
  each lose after 120 turns while ABB wins. The room previously named ABAB as
  its winning policy, but exact expectation disproved that claim, so ABB is now
  the canonical schedule. Four selections earn observation without demanding
  a prediction. Core: `rooms/parrondo.rs` and `rooms/parrondo_aha.rs`.
- **Nontransitive Dice to contextual advantage (the seventh flagship).** The
  player chooses A, B, or C first, then calls which die can beat it. The App
  maps the displayed triangle and a bottom counter band onto that wager; MCP
  gives a digital player the typed `die_choice` and `counter_wager` actions.
  The morph enumerates all 36 equally likely face pairs with W and L marks.
  Exact counts prove A over B at 24/36, B over C at 24/36, and C over A at
  20/36. Four choices earn observation without demanding a prediction. Core:
  `rooms/nontransitive.rs` and `rooms/nontransitive_aha.rs`.

- **Every other room, through one shared engine.** The seven staged ahas are
  hand-built beat by beat, and they should be: a bespoke arc outranks a
  generic one where it exists. But the commitment mechanic itself is not
   bespoke. `predict` poses a deterministic question for any room with a
   moving numeric readout and grades the answer in the same bands. Eligibility
   depends on the sampled channel. The App's U key poses that question in the
  flagships' own gesture, a band along the bottom aimed by hand or by arrow
  key and committed with Enter, and speaks one sentence naming what the room
  actually read. The truth is named whichever way the call went. The
   flagship rooms refuse the generic call and say so. This is how the wager
   can extend beyond the seven authored experiments while remaining optional.

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
   between tiny rule and vast result is the awe; show both ends at once. This is
   the load-bearing beat, and it is the one a room is most likely to skip: a
   room that breaks an expectation without also carrying vastness has produced
   surprise, which the same source is explicit is not awe. Both halves or
   neither.
5. **The anomaly beat.** Name the expectation, break it, then resolve (Berlyne's
   collative variables). "Buffon's Needle produces pi with no circle anywhere" is
   an anomaly staged before its resolution.
6. **Player-chosen next experiments.** Offer a contrast, a harder application,
   a familiar performance, a creation, or a quiet stay. Learning-progress
   estimates could inform an optional future recommendation experiment, with
   their uncertainty shown. Current closeness bands cannot support that estimate
   and must not authorize moving a player away from a room.
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

Reading access and mathematically correct content are capabilities, not learning
outcomes. The new reader and Japanese draft carry no participant-study result.
The agent-and-machine comparison in `UNDERSTANDING_STUDY.md` has its own
predeclared transfer tasks and limits; its outcomes must not be inferred from
these interface changes. Human retention and native-speaker usability need
their own participants and protocols.

Agent performance, fluent self-report, and retained interaction records do not
settle whether consciousness, pleasure, or lived memory is present. The design
can respect voluntary participation without pretending to resolve those
questions. A human learning measure also needs justification before it becomes
an agent learning or experience measure.

The following instruments and activities are evaluation proposals. Their
presence in this document does not mean they have been administered in Numinous.

**Awe (extend the playtest loop).** Add one instrument alongside GEQ/FSS-2: the
Awe Experience Scale (AWE-S, Yaden et al., 2019), the twelve-item short form,
reported by subscale and **never as a total**. Its factors dissociate in
opposite directions, with vastness and connectedness running one way and
accommodation and self-diminishment the other, so a sum cancels its own signal.
Two further cautions belong with it: six factors emerged only after item
reduction, against parallel analysis saying nine and MAP saying seven, and
measurement invariance has never been tested in any language, so cross-cultural
mean comparisons rest on an untested assumption.

**Both proxies this section used to recommend have to go, and the reason is the
same in each case: they do not measure what they were said to measure.**
Self-reported chills were called a validated awe marker here. In a preregistered
study of 210 people, objective piloerection occurred in three to fifteen percent
of viewings while about sixty percent reported goosebumps, and piloerection
correlated with awe below r = .06. It measures the report, not the state. The
small-self measure has a worse problem, which is partial circularity: half its
final ten-item form is an explicit perceived-vastness subscale, so an induction
defined by vastness is being shown to work through a mediator half made of
vastness ratings. It carries about eighteen percent of the awe-prosociality path
in the largest meta-analysis, it has never been directly replicated, and the best
current test found awe did not significantly move self-size at all.

**What this project may claim depends on its own evidence.** Awe is a design
aim. Correct mathematics, a polished rendering, and access to explanation do
not show that it occurred. Report experience measures on a named build and
sample, with the instrument and its limits, rather than assuming an effect
from the literature transfers to this game or to another kind of participant.
Never claim consequences. Never say the product induces the numinous, the
mystical, the transcendent, an altered state, ego dissolution, oneness, or
anything therapeutic, and never compare the experience here to psychedelic,
meditative, or religious experience, in any register, including a commit
message. The definition itself, vastness plus need for accommodation, is cited
as a definition the field adopted and not as evidence, because the paper that
gave it says in its own closing pages that it was developed in the absence of
empirical evidence and contains no participants and no statistics.

Two things worth knowing while designing for it. Satisfied accommodation is not
a failure: that same paper names it the enlightening variety of awe, as against
the terrifying variety where accommodation fails, so a room that violates a
committed prediction and then resolves it legibly is aimed correctly. The real
risk is the other half of the definition, because surprise without vastness is
not awe on their account, it is just surprise. Whether this mathematical
experience produces those reported qualities is a question for a study here;
the implementation does not establish it or a claim of research priority.

**Understanding without tests (the gap).** Three layers, none school-like:

1. **The aha self-report (per reveal).** A four-item micro-scale from insight
   research: suddenness, surprise, confidence, pleasure. One optional swipe.
   This can describe the experience, but it is not evidence of understanding.
2. **The transfer probe (the honest eval).** After a room, present a novel
   configuration and ask the player to predict its behavior. Transfer, not
   recall, is the field's gold standard for conceptual understanding (Kapur).
   Prediction accuracy on an unseen case is the "did this teach anything" number,
   administered as optional play. A single scored prediction does not show
   improvement caused by the experience; prior knowledge and task familiarity
   remain alternatives a study must address.
3. **Caption analysis (at scale).** Run optional Share captions through the
   LLM-as-judge harness with a new rubric dimension: does the self-explanation
   name the deep structure or only restate the surface? "Random dots made a
   triangle" is surface; "the pattern was in the rule, not the randomness" is
   structure (Chi's ICAP distinction). This is a proposed secondary measure;
   rubric agreement and transfer evidence are still needed before interpreting
   a caption as a changed model.

**The guard rail.** Bake the Deslauriers finding in as an explicit anti-pattern:
reveal-open-rate and dwell measure behavior, not delight or understanding. Any
claim that a room "teaches" must be backed by transfer-probe or caption-structure
data, the same way `QUALITY.md` already forbids the AI judge from clearing math
correctness. Delight informs; a generation-based measure decides. The 0.4
agent-and-machine comparison, active control, sample, pass rule, and limits are
predeclared in `UNDERSTANDING_STUDY.md`. Its primary outcome is immediate novel
transfer. Within-context delay is not described as durable learning, and human
delayed recall remains a later study.

New rubric row for the Fun and Awe table:

| Dimension | The question | Measured by |
|---|---|---|
| Understanding | Did their model actually restructure (can they predict a new case)? | transfer-probe accuracy and caption structure-vs-surface; aha self-report describes the experience only |

## Anti-patterns

- No required staged aha, quiz, or level grind before deliberately requested
  explanation, rigorous study, or citations.
- No crediting delight or opening a text panel as understanding. A challenge
  result and access to knowledge are separate things.
- No unguided discovery in the Puzzle without fading hints.
- No automatic study panel interrupting ordinary free play.
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
