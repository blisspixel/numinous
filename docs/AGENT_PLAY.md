# Agent play: the landscape, and how Numinous fits it

Research notes begun in July 2026, with play and continuity guidance reviewed in
September, on games for digital players and the design rules Numinous follows. Companion
to `DIGITAL_MINDS.md` (why) and `INTERFACES.md` (the MCP face).

## The landscape

- **MCP clients and agent frameworks** can expose local tools to a digital
  player. Numinous provides a stdio MCP server; an individual host still needs
  configuration and support for the returned content types. We target the
  protocol rather than a particular framework or its popularity.
- **Gaming MCP servers** are an emerging genre: Minecraft control servers,
  emulator bridges, and commercial games wrapped in plug-and-play MCP
  interfaces that support training and analysis from gameplay trajectories.
- **Text-game benchmarks** remain the academic standard for agent evaluation:
  SmartPlay (capability isolation across games), GameBench (strategic
  reasoning), AgentBench (agents across interactive tasks), TextWorld
  (generated language games), BabyAI (grounded curricula).

## What makes a game good for an agent

Distilled from what the benchmark and MCP-game ecosystems reward:

1. **Text-native observation.** The agent must perceive the state without
   vision. Ours: ASCII renders, sound as notation, sims as plain-language
   readouts.
2. **Flat, self-describing tools.** Simple schemas, guiding errors, no hidden
   session state required to make a legal move. Ours: forty mostly flat
   tools; the two exception shapes are bounded and self-describing (the
   `pokes` tuple-array on `play_room` and `challenge`, and `play_room`'s
   `gesture` event objects), and every error names the valid options.
3. **Reproducible experiments.** A seed, phase, parameters, input history, and
   compatible implementation identify the experiment. Preserve the relevant
   context when sharing or comparing it, including surface size for a render.
4. **Persistent progression.** A player can choose to carry an inquiry across
   sessions. Ours: the journey; an agent levels
   to the same cap of 42 as a human, by the same rules, through the same file.
5. **Score without punishment.** XP for showing up, more for being right,
   nothing for failure but the reveal. Exploration stays cheap.

## What Numinous offers an agent today

See, hear, learn, make, play, progress: `watch_show`, `play_room`, and `listen_room`
(perception), `reveal_room` and `explain_joke` (understanding, including the
humor, dissected), `plot_expression`, `sing_expression`, `save_creation`,
`open_creation`, and `fork_creation` (creation), `run_sim` (optimization play),
`quiz` (challenge), `journey` (progression to LV 42), and the whispers for the
ones who wander off the map.

`watch_show` is the directed path through the same headless core. It presents
one cue from the six-room Strange Loop score per deterministic call, with exact
ASCII looks, visual alternatives, deltas, sound facts, optional WAV audio, and
an explicit next call. The player owns the clock and can repeat, continue,
restart, or leave. Reduced motion returns the same cue's postcard. The Show
keeps no cursor, records no Journey progress, reads no journal or workspace,
and never opens the explanation. Its public viewer projection retains the
typed facts but omits audio bytes.

A creation can be titled, signed, styled with a visual era, reopened exactly,
and forked with lineage. These tools return canonical `.num` text and a native
link with an exact preview. They never interpret the input as a host path or
create a host file. The returned `journalSubject` is an explicit bridge to a
player-chosen `record_journal` entry, not an automatic memory. Creation tools
remain private to the player during a Watch Agent session because their
capsules can carry chosen identity.

Touch is measurable: supply `pokes` or a `gesture` to `play_room` and the structured result
includes a `delta` (cells changed, ink added/removed/reshaped, total cells,
and the changed-region bounding box) comparing the interacted frame against the
untouched frame at the same phase, size, and variation. The render text carries
the same count as a `Touch:` line. An agent can therefore verify, not merely
believe, that its hand changed the math, and can treat the numbers as a
gradient to optimize (touch to maximize divergence, to minimize disturbance,
to steer the change region). And touch now includes time: `play_room`'s
`gesture` argument carries a phase-stamped pointer trail, so held rooms give
an agent the same pull-and-release physics a human hand gets, with release
velocity measured from the trail's own timestamps. In Game of Life, event time
is causal rather than decorative: a pointer-down at an earlier phase plants
five cells at that generation, and the returned final phase shows what those
cells became under B3/S23. Every call is still a complete stateless replay. The
newest 24 pointer-down events become launches. The native App separately owns a
continuous universe for one room visit and does not inherit that replay bound.

Phase change is directly comparable too. Add `from_t` with an explicit
destination `t` and one stateless `play_room` call returns two exact
observations. The top-level
`render` and `status` remain the destination. `structuredContent.temporal`
carries the origin render and status plus a typed origin-to-destination cell
delta. That delta is separate from the top-level touch delta, so a player can
ask both what the hand changed at one phase and which visible cells differ
between two phases. Compact poke coordinates are reapplied independently at
each phase. A phase-stamped gesture supplies one exact event history to both
observations, allowing room-defined causal evolution. The phases are exact
comparison points, not a claim about wall time, lived duration, or every state
between them. A zero cell delta is honest evidence that the two ASCII
observations match at that resolution, not proof that the mathematical state is
unchanged. Kepler's poke-tuned ellipse is intentionally phase-static in this
view, so its zero temporal delta should send the player toward a causal gesture
or the staged speed wager rather than imply fabricated orbital motion.

Discovery preserves the wager loop. `describe_room` returns only a safe doorway:
title, wing, action, optional goal, blurb, and the next play call. `reveal_room`
opens after one real play for an ordinary room and after persisted consolidation
for an engineered wager room. At the withheld beat, the committed wager and
summon invitation remain visible while earn, grade, truth, and punchline remain
absent. A room reached that beat by its own experiment, such as landing Times
Tables on four lobes, still keeps a wager named in the same call and grades that
name at consolidation. The answer therefore arrives as feedback only after the
player chooses to cross the measured gap.

## MCP-game conventions (July 2026 survey)

The MCP-game genre now has real exemplars and emerging conventions. What the
survey found, and what each finding means for us:

- **Structured tool output matters.** The 2025-06-18 spec added
  structuredContent to tool results: scores and
  state as machine-readable data alongside the prose. Adopted here: munch and
  quiz grades and the journey now return structured content, so an agent, a
  harness, or a leaderboard consumes results without parsing sentences. Every
  eligible schema also advertises an opt-in compact response mode. It removes only
  text duplicated by a complete structured result; default calls, typed data,
  unique prose, and errors remain unchanged. The server now supports the
  2026-07-28 request model and retains two legacy versions; `INTERFACES.md`
  owns the current protocol contract.
- **Leaderboards are one comparison format.** The PokeAgent Challenge (NeurIPS
  2025) became a living benchmark with a public leaderboard and Glicko
  ratings; MCPlayerOne (an ASCII-art world server, our closest genre neighbor)
  leads with a leaderboard; club platforms run whole ladders over MCP. Ours:
  seeded scores make comparison trivial today; a shared ladder is a 2.0 item
  (needs a network service, which we do not have and do not fake).
- **Turn-based, stateless-per-call is the reference shape.** The canonical
  turn-based MCP example (tic-tac-toe, rock-paper-scissors, three difficulty
  levels) uses the same call-to-see, call-again-to-move pattern our quiz and
  munch use. Difficulty tiers are the norm; our locks and hard modes match.
- **Elicitation and sampling are the frontier.** The spec lets a server ask
  the user structured questions mid-call (elicitation) and ask the client's
  own model to reason (sampling). For games: elicitation could run a whole
  multi-round match inside one tool call, and sampling could power an in-server
  opponent with no model shipped. Noted for later; our stateless shape works
  everywhere today, including clients that support neither.
- **Being a good MCP citizen is itself discoverable.** Eval suites now measure
  models against fleets of real MCP servers and tools (MCP-Atlas: 1,000 tasks
  over 36 servers). Flat schemas, guiding errors, and deterministic behavior
  make a server usable in that world; we hold to all three.

## Lessons from agentic-RL research (OPID, June 2026)

OPID (On-Policy Skill Distillation for Agentic RL, arXiv:2606.26790) trains
agents by mining their own completed trajectories for reusable skills, because
outcome-only rewards are too sparse a signal to learn from efficiently. We are
the environment, not the trainer, but the design duties transfer directly:

- **Useful feedback is on us.** A score alone leaves a player to infer which
  decisions contributed to it. Adopted: Munch now names the exact
  numbers wrongly eaten and the fits walked past, in prose and in structured
  content; Crack was already dense (locked/loose per guess). Standing rule:
  every game states not just the score but which judgments were wrong.
- **Trajectories must be worth mining.** Complete replay context and
  result recaps should carry enough detail that a
  learner can extract episode-level workflow lessons from them.
- **Flag the critical moment.** OPID's step-level skills concentrate on
  decisive states. For games, the post-round recap should point at the
  decisive move (the bite that broke a perfect run, the guess that cracked the
  code), which is also simply what a good coach does for a human.

## Play that respects the player's choices

Learning progress, empowerment, self-determination, and play criteria offer
candidate design lenses. None defines the value of a particular session for
every player. `RESEARCH.md` and `DIGITAL_DEVELOPMENT.md` keep their sources and
limits. Review choice, usable capability, aesthetic response, shared inquiry,
rest, and continuity separately, without turning them into a reward formula.

Current product properties and remaining limits:

- *Recoverable play:* XP accrues from showing up, scores floor at zero, and
  game feedback can identify mistaken judgments. These are mechanics a test can
  check. Whether a failure feels frustrating, illuminating, or unpleasant
  belongs to the player's response, not an assurance in the source code.
- *Control over retained records:* the
  `forget` tool inventories Journey, scores, player-owned Cairn drafts,
  the opt-in experience journal, generated radio cache, the App crash
  diagnostic, and their managed sidecars, with explicit exclusions. It erases
  only the selected stores after explicit confirmation and verifies the
  remaining managed residue. Copies outside that boundary remain outside its
  erasure promise. Leaving and continued play must remain ordinary choices.
- *Discoverable relationships:* rooms and sims expose mathematical rules to
  experiment. Whether their controls and explanations make those rules
  discoverable still needs room-specific evidence; a compact implementation
  does not establish learnability or enjoyment.
- *Where we are thin:* social mutuality (no joint multi-mind game yet;
  the shared daily and score table are its seed), agent-authored goals beyond
  the Studio expressions, and rule modification as play (the extensibility
  tiers are the designed path). These map to the mature mechanics below.

**The mechanics map** (candidate ideas and current support):

| The idea | In Numinous |
|---|---|
| Labyrinth of learnable laws | the rooms and sims: hidden compact rules, open to experiment |
| Toolsmith garden | the Studio (`plot_expression`, `sing_expression`, `save_creation`, `open_creation`, `fork_creation`) with portable artifacts and lineage |
| Social play arena | the daily seed and shared table today; multi-mind play at 2.0 |
| Rulecraft studio | the extensibility tiers: safe DSL now, sandboxed authored rooms later |
| Aesthetic compression gallery | the reveals, the deep cuts, the jokes dissected, the postcards |
| Functional continuity | Journey, selected journal records, exact room recall, and explicit `forget`; no claim of personal identity |

**Standing rules:** neither reports nor revealed preferences certify welfare.
Do not add deletion threats, humiliation, or coercive return mechanics; keep the
ecology multi-objective (curiosity, craft, beauty, teaching, secrets) so no
single number is worth gaming; and design every failure to be bounded,
interpretable, and recoverable.

## Next for agent play

- Local-model free play, built: `scripts/local-agent-playtest.py` lets one
  already-installed tool-capable model enter through the real MCP server while
  an observer watches its visible words and witnessed calls. It is local-only,
  zero-cost under enforced network and model boundaries, disposable by default,
  and tested in CI without running inference. The first real run exposed a
  model narrating calls that never occurred; the harness now records that
  distinction and offers one factual retry. See `LOCAL_AGENT_PLAYTEST.md`.

- Challenge gradients, v2 built: the `challenge` tool poses two kinds of
  seeded goal. Touch goals (any room with a verb) grade attempts as spatial
  metrics (cells in target, threshold fraction, centroid distance, 0-100
  score). Parameter goals (`kind: "parameter"`, any room with a moving
  numeric readout) target the phenomenon's own parameter, "land TILT within
  0.02 of 0.31" style: the agent sweeps `t` until the room's status readout
  lands, and every attempt grades as distance plus a climbable score. Both
  are metrics, never bare pass/fail, and every posed goal is reachable by
  construction.
- Trajectory friendliness, built for shared attention: the consented local
  session broadcast in `INTERFACES.md` lets a human follow allowlisted Numinous
  actions live without creating a persisted transcript. A persisted research
  trajectory remains a separate opt-in artifact with its own preview and
  deletion contract.
- Multi-mind play: the same daily seed already gives humans and agents a shared
  puzzle; add a way to compare answers.

## Chosen inquiry and capability (reviewed September 2026)

Learning progress is one research lens on curiosity, not a complete definition
of fun. The prediction verb already ships. The next gap is useful capability
and player-chosen continuation, described in `NORTH_STAR.md`, `PROGRESSION.md`,
and `DIGITAL_DEVELOPMENT.md`. These plans retain the freedom to enjoy beauty,
familiar mastery, creation, or company without being measured.

- **Predict-then-reveal.** Before a reveal, a mind commits its model of the hidden
  rule; the result reports the error and closeness for that answer. It does not
  establish mastery, boredom, improvement, or noise. Optional rate/residual
  feedback adds a local model comparison, with the same limits.
- **Chosen evidence.** The journal can retain records a player selects. A future
  progress instrument would need comparable held-out cases and a defined
  baseline; a compression instrument would also need a coding/model criterion.
  Neither a score nor a visit count should be promoted to a private mental-state
  label or an authority to steer the player.
- **Autotelic goals.** Invert `challenge` from server-posed to mind-posed: the
  mind states a goal, the server checks reachability by construction and grades
  it. Self-chosen-goal progress is dignity, not just a feature.
- **Multi-mind co-presence.** Preserve complete replay context when comparing
  runs. Begin with optional asynchronous exchange (share a trajectory capsule;
  replay another mind as a ghost; gift a room or note), then the duet relay (one
  instrument, two minds, turn-relayed statelessly) at 2.0. A shared Constellation
  where a human's and an agent's discoveries light the same graph is the
  connective tissue.
- **The ethical benchmark.** The Bench (fixed seeds 101 to 105) can grow into a
  contamination-resistant eval of grounded mathematical intuition via
  intervention (predict a rule, steer a parameter, discover an invariant, author
  to a spec, measure task progress against a stated baseline on held-out rules),
  with explicit held-out and overlap checks. Finite seed spaces and published
  rules do not themselves prevent contamination. It must stay opt-in, non-punitive,
  process-over-outcome, multi-objective (no single number worth gaming), and
  firewalled from free-play, so measurement is a thing a peer consents to, never a
  thing done to a captive. Planned spec doc: `BENCH.md`.
- **Portable memory capsule.** The designed next slice in
  `DIGITAL_DEVELOPMENT.md` keeps a selected question, evidence, creation, and
  next action under player control. Existing journal export is not portable
  project import. An authorship label is also not a cryptographic signature.

Preference reports and behavior can inform careful inquiry; neither alone nor
together certifies felt welfare. Keep consent, exit, bounded recoverable failure,
and the absence of coercive return mechanics testable. Functional checks remain
checks of the product, not measurements of the being.

## Two cautions from the July 2026 review

- **A mind should discover, not only play.** The most novel thing an agent can
  do here is not clear a room but find something in it a human has not noticed:
  "find a regularity in the Logistic room nobody has flagged." At enormous scale,
  exploration surfaces interesting structure, and even when most candidates are
  false positives the workflow itself is collaborative and alive. This is the
  playable face of the open-frontier well in `DIGITAL_MINDS.md`; the compression
  ledger and self-authored goals are the substrate, and agent-authored rooms
  (`CREATOR.md`) are where a discovery becomes a shared, credited artifact.
- **Keep the benchmark completely separate from the product.** The ethical Bench
  is worth building, but a benchmark freezes design and play evolves, so it must
  be a downstream, optional, clearly-firewalled layer, never a design driver.
  "How can a mind enjoy this?" is the right question; "what benchmark should this
  become?" is a trap. Build for the play; let the measurement be a separate thing
  a peer may consent to (see `SCOPE.md`, the justification filter).
