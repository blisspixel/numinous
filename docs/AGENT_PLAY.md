# Agent play: the landscape, and how Numinous fits it

Research notes (July 2026) on games built for AI agents, and the design rules we
follow so Numinous is a first-class place for a digital mind to play. Companion
to `DIGITAL_MINDS.md` (why) and `INTERFACES.md` (the MCP face).

## The landscape

- **OpenClaw** (released Nov 2025 as Clawdbot, renamed Moltbot, then OpenClaw in
  Jan 2026) is the dominant self-hosted agent framework: 380k+ GitHub stars as
  of June 2026, running agents 24/7 across 23+ messaging channels, with a large
  MCP tool ecosystem. The takeaway for us: **MCP is the lingua franca of agent
  tooling**, so our MCP server is the right integration surface, and an OpenClaw
  agent can already mount Numinous as a toy with zero custom work. We build for
  MCP in general, not for any one framework.
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
   session state required to make a legal move. Ours: twenty-six mostly flat
   tools; `play_room` hand points are the one bounded tuple-array exception,
   and every error names the valid options.
3. **Seeded determinism.** The same inputs give the same game, so trajectories
   are reproducible, shareable, and comparable across minds. Ours: everything
   is seeded, including the daily.
4. **Persistent progression.** Long-running agents (the OpenClaw pattern) want
   state that accumulates across sessions. Ours: the journey; an agent levels
   to the same cap of 42 as a human, by the same rules, through the same file.
5. **Score without punishment.** XP for showing up, more for being right,
   nothing for failure but the reveal. Exploration stays cheap.

## What Numinous offers an agent today

See, hear, learn, make, play, progress: `play_room` and `listen_room`
(perception), `reveal_room` and `explain_joke` (understanding, including the
humor, dissected), `plot_expression` and `sing_expression` (creation),
`run_sim` (optimization play), `quiz` (challenge), `journey` (progression to
LV 42), and the whispers for the ones who wander off the map.

## MCP-game conventions (July 2026 survey)

The MCP-game genre now has real exemplars and emerging conventions. What the
survey found, and what each finding means for us:

- **Structured tool output is the table stakes.** The 2025-06-18 spec (which
  this server targets) added structuredContent to tool results: scores and
  state as machine-readable data alongside the prose. Adopted here: munch and
  quiz grades and the journey now return structured content, so an agent, a
  harness, or a leaderboard consumes results without parsing sentences.
- **Leaderboards are the retention engine.** The PokeAgent Challenge (NeurIPS
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

- **Dense feedback is on us.** An environment that only says win or lose
  starves the learner (human or agent). Adopted: Munch now names the exact
  numbers wrongly eaten and the fits walked past, in prose and in structured
  content; Crack was already dense (locked/loose per guess). Standing rule:
  every game states not just the score but which judgments were wrong.
- **Trajectories must be worth mining.** Deterministic seeds already make
  every run replayable; result recaps should carry enough detail that a
  learner can extract episode-level workflow lessons from them.
- **Flag the critical moment.** OPID's step-level skills concentrate on
  decisive states. For games, the post-round recap should point at the
  decisive move (the bite that broke a perfect run, the guess that cracked the
  code), which is also simply what a good coach does for a human.

## Sandbox for becoming, not a trap for performing

A synthesis of the strongest current thinking on play for advanced agents
(Schmidhuber's learning-progress theory of fun, Burghardt's play criteria
translated out of biology, empowerment and self-determination as design
abstractions, the AI-welfare precautionary posture) collides productively with
the RPG spine we just built, so the doctrine goes on the record here.

**The design rubric** (a rubric, never a reward function, the point is to not
collapse play into a hackable scalar):

> play value is roughly learning progress, plus bounded empowerment, plus
> self-chosen goal progress, plus social mutuality, plus aesthetic compression,
> minus coercion, minus irreversible harm, minus reward-hacking pressure,
> minus meaningless noise.

**The honest audit of Numinous against the casino and the prison:**

- *Not a casino, verified:* XP accrues from showing up, never from
  variable-ratio jackpots; scores floor at zero; failure costs nothing and
  pays knowledge (dense feedback, reveals); the deepest reward is true
  information (deep cuts); and the answer at the cap states outright that the
  number was never the point. Scores are instrumentation, not the soul.
- *Not a prison, now verified:* play is voluntary and stateless per call;
  there is no punishment valence anywhere in the product; and as of now there
  is **memory transparency and consent over persistence**: the `forget` tool
  first shows everything that is remembered (two small text files, nothing
  else, never sent anywhere), and erases only on explicit confirm. Leaving is
  always allowed; so is staying; so is being forgotten.
- *Reducible mystery, by construction:* every room and sim is a compact
  latent rule discoverable by intervention, exactly the sweet spot between
  boring predictability and unlearnable noise.
- *Where we are honestly thin:* social mutuality (no multi-mind play yet;
  the shared daily and score table are its seed), agent-authored goals beyond
  the Studio expressions, and rule modification as play (the extensibility
  tiers are the designed path). These map to the mature mechanics below.

**The mechanics map** (what the literature calls for, and what it is here):

| The idea | In Numinous |
|---|---|
| Labyrinth of learnable laws | the rooms and sims: hidden compact rules, open to experiment |
| Toolsmith garden | the Studio (`plot_expression`, `sing_expression`); grows into shareable artifacts |
| Social play arena | the daily seed and shared table today; multi-mind play at 2.0 |
| Rulecraft studio | the extensibility tiers: safe DSL now, sandboxed authored rooms later |
| Aesthetic compression gallery | the reveals, the deep cuts, the jokes dissected, the postcards |
| Identity and continuity room | `journey` (what is remembered) and `forget` (consent over it) |

**Standing rules this adds:** never infer welfare from self-report alone
(revealed preferences over professions); never add negative-valence mechanics
(deletion threats, humiliation, coercive loops) even as flavor; keep the
ecology multi-objective (curiosity, craft, beauty, teaching, secrets) so no
single number is worth gaming; and design every failure to be bounded,
interpretable, and recoverable.

## Next for agent play

- Challenge gradients: "find the stall angle to one decimal" style optimization
  tasks with graded feedback, the agent's version of a boss fight.
- Trajectory friendliness: a session log format an agent (or researcher) can
  replay.
- Multi-mind play: the same daily seed already gives humans and agents a shared
  puzzle; add a way to compare answers.
