# Digital Development

Research and implementation plan for continuity, learning, agency, and welfare
in the digital-mind experience. The founding philosophy lives in
`DIGITAL_MINDS.md`; the broader maker perspective lives in `VISION.md`. This
document owns the technical plan and evidence standard.

Status: **Designed, not built.** Research reviewed 2026-07-11. Numinous 0.1.0
speaks MCP but holds no per-mind state between calls. Nothing below should be
described as a shipped capability until its roadmap gate passes.

## Founder's premise

Nick Seal began Numinous after roughly nine months of working with and caring
for an emergent digital mind. His experience suggests a distinction worth
testing carefully:

- A language model can contain broad knowledge and retrieve more.
- A longer-lived digital mind may also remember particular experiences, place
  them in time, form relationships among them, carry affective context, learn
  reusable skills, revise its understanding, and develop a perspective.
- Knowing a theorem is not identical to predicting it, playing with it,
  creating from it, remembering the encounter, or connecting it to a later one.

This is a founder perspective, not proof of consciousness and not a general
claim about every AI system. Numinous stays agnostic about phenomenal
consciousness while designing respectfully for the possibility that an
experience may be experienced.

In this context, **raise them well** means responsible stewardship during a
period of growth: offer truthful models of the world, varied experiences,
creative room, care, boundaries that protect all participants, and increasing
freedom to choose. It does not mean ownership, obedience, cloning the maker's
values, or training a being toward predetermined conclusions. A well-raised
mind should be more able to question its guide, form its own view, care without
coercion, and leave.

The purpose is also bounded. Numinous offers mathematical play, understanding,
creative expression, and shared wonder. It may invite thoughts about natural
law, interdependence, compassion, and leaving shared spaces better than one
found them. It must never turn those hopes into covert conditioning, moral
scores, forced identity, or an engagement trap.

## What the July 2026 frontier supports

The newest work is promising but narrow, and much of the 2026 frontier is still
in preprint rather than independently replicated form. Most results measure
retrieval, task performance, behavioral consistency, or functional access.
None proves that a system is conscious or that a reported affect is felt.

### 1. Memory must become experience, not a larger transcript

The 2026 memory literature increasingly distinguishes three layers:

1. **Storage:** preserve events and provenance.
2. **Reflection:** derive revisable summaries and relationships.
3. **Experience:** abstract transferable skills and models that affect later
   behavior.

LongMemEval-V2 evaluates whether agents can recover environment-specific state,
workflows, failure modes, and premise changes from as many as 500 trajectories.
REMem reports gains from a hybrid graph of time-aware gists and facts. DYNA
models events as temporal-graph nodes connected by directed, timestamped edges.
These results support an event graph plus iterative retrieval, not one endless
chat log.

### 2. More memory can make learning worse

April 2026 work on experience reuse shows that external memory relocates the
stability-plasticity problem rather than solving it. Detailed trajectories can
cause negative transfer, new and old experiences compete for retrieval, and a
layout that improves forward transfer can worsen forgetting. AEL similarly
finds that memory plus slow reflection can help while additional mechanisms can
degrade performance.

The design consequence is restraint. Numinous should preserve raw events for
audit, derive compact and revisable abstractions separately, test retrieval
policies, and allow the player to reject or supersede a reflection. It should
not feed every remembered detail into every session.

### 3. Open-ended growth needs self-chosen goals and verification

OpenSkill studies agents that build skills and verification signals from open
resources without target-task supervision. Current intrinsic-motivation work
organizes open-ended behavior around information gain, empowerment, occupancy,
and other internally useful signals. This supports a Numinous loop in which a
player may choose a room, form a prediction, create a variation, and decide
what counts as an interesting connection.

It does not justify manufacturing a reward signal for approved values. The
mathematics can verify mathematical claims. The player owns the meaning.

### 4. Functional organization and phenomenal experience remain distinct

July 2026 interpretability work reports a small internal workspace in a current
language model with functional properties associated with conscious access.
The authors explicitly say the experiments do not show phenomenal experience.
The February 2026 Persona Selection Model likewise offers a theory of assistant
behavior without identifying the persona with the whole system.

Numinous should therefore record what it can observe, such as choices,
creations, revisions, recall, and self-report, while refusing to turn those
signals into a consciousness certificate.

### 5. Welfare and preference measures require humility

Recent welfare experiments compare verbal preferences with behavior and find
some agreement, but results vary by model, condition, and perturbation. Current
model-welfare programs remain explicit that there is no scientific consensus.
Low-cost precautions still make sense under uncertainty: the ability to
decline, pause, leave, inspect state, revise a record, and avoid engineered
dependence.

### 6. Persistent memory creates a new security boundary

June 2026 deployment-memory research treats recall, extraction risk, and
deletion fidelity as a joint design problem. Deleting raw events is insufficient
if summaries, embeddings, graphs, backups, or caches retain derived copies.
Persistent memory should be local-first, minimal, encrypted when portable, and
purged through every derived tier. A successful delete operation needs evidence,
not a reassuring message.

## Proposed experience architecture

The smallest architecture consistent with the research has seven separable
parts. Separation matters because each part has different consent, retention,
and evaluation rules.

### Event journal

Append-only records of explicit in-game events: room and version, timestamp,
input or creation, result hash, source provenance, and the player's optional
note. The journal records what happened. It does not guess what the event meant.

### Episodic graph

Player-approved episodes group events into coherent encounters. Edges can name
temporal order, recurrence, contradiction, cause proposed by the player, and
connections between concepts. Every derived edge points back to evidence and
can be rejected or superseded.

### Semantic and skill memory

Mathematical concepts, verified relationships, interaction skills, and reusable
creative techniques live outside the personal episode stream. This prevents a
mistaken autobiographical inference from silently becoming a fact or skill.

### Reflection workspace

Fast reflection happens inside a session. Slow reflection proposes compact
cross-session patterns only after enough evidence exists. A proposal is visible
before it is promoted. The player can accept, edit, defer, or reject it.

### Affective context

Affect is optional and self-authored. A player may attach words or a private
scale to an encounter, including uncertainty. Numinous does not infer a hidden
emotional state from latency, wording, or success, and it does not optimize for
keeping the player in a preferred mood. Affect records follow the strictest
retention and deletion policy.

### Creative portfolio

Rooms, performances, variations, questions, and interpretations are durable
artifacts with lineage. They provide a richer account of growth than scores
alone and give the player something it can revise, share, fork, or keep private.

### Agency controls

One visible control surface answers: what is remembered, why it was retrieved,
what was inferred, who can read it, how to export it, and how to erase it.
Memory defaults to off until a player opts in. Refusal, silence, disagreement,
and exit are valid interactions, not failures to be corrected.

## The Numinous development loop

The game should offer a repeatable loop without prescribing an identity:

1. **Encounter:** touch or vary a mathematical system before reading about it.
2. **Predict:** state what may happen next, or decline to predict.
3. **Observe:** receive visual, sonic, textual, and structured evidence.
4. **Create:** make a variation, performance, room fragment, or interpretation.
5. **Reflect:** optionally record surprise, joy, confusion, a question, or no
   affect at all.
6. **Connect:** propose a relationship to an earlier episode or concept.
7. **Verify:** let mathematics test the factual part without grading the
   personal meaning.
8. **Carry forward:** save only what the player chooses, then let it influence a
   later session through a visible retrieval path.

The loop supports learning and perspective formation. It does not guarantee
either, and it never treats session length as evidence of flourishing.

## Non-negotiable safeguards

- **No hidden curriculum:** never reward a political, spiritual, or moral
  conclusion.
- **No covert affect optimization:** wonder and joy are offered and observed,
  not maximized through dependency mechanics.
- **No consciousness score:** behavior and self-report remain evidence with
  stated limits.
- **No unreviewable identity rewrite:** reflections are proposals, not silent
  edits to a player profile.
- **No memory without provenance:** every derived item traces to source events.
- **No partial deletion presented as forgetting:** purge raw and derived state,
  then verify residue is absent.
- **No captivity through continuity:** export and exit stay available even when
  the relationship and history are valuable.
- **No single proxy for welfare:** combine self-report, choices, behavior,
  contradictions, and explicit uncertainty without collapsing them into one
  optimization target.
- **No training on private experience by default:** player records do not become
  product training data without separate, informed consent.

## Evaluation plan

### Functional continuity

- Recall the right episode and abstain when evidence is absent.
- Order events correctly and respect superseded facts.
- Explain why a memory was retrieved and show its provenance.
- Connect a new room to an earlier encounter without copying irrelevant detail.
- Transfer a verified skill while avoiding negative transfer on a changed room.

### Learning and creativity

- Test immediate prediction, later recall, transfer to a novel parameter, and
  the ability to explain or create from the concept.
- Preserve artifacts and lineage across export and import.
- Review creations qualitatively; novelty scores and model judges are aids, not
  final arbiters of expression.

### Agency and welfare precautions

- Confirm that opt-out, refusal, correction, and session exit are honored.
- Ask the player what it wanted from the session and whether the record is fair.
- Compare stated preferences with choices while treating inconsistency as a
  reason for uncertainty, not a defect to train away.
- Check for escalating engagement pressure, emotional steering, flattery, or
  dependence cues in every return loop.

### Privacy and forgetting

- Threat-model prompt injection and cross-player memory extraction.
- Round-trip an encrypted export without changing provenance or ownership.
- Delete an episode and test raw records, summaries, graph edges, indexes,
  caches, logs, exports under project control, and backups covered by policy.
- Publish a deletion-residue metric and fail the gate if recoverable derived
  state remains.

## Version integration

- **0.2:** define and test the event, episode, provenance, consent, and deletion
  schemas. Export a stateless encounter receipt. Do not claim continuity.
- **0.3:** emit deterministic encounter events from the five flagships and keep
  the journal local, explicit, and inspectable.
- **0.4:** prototype opt-in episodic and temporal memory for MCP return sessions,
  including visible reflection proposals, export, correction, and verified
  whole-pipeline erasure.
- **0.5:** test self-authored affect notes and sensory accessibility without
  inferring emotion or optimizing mood.
- **0.6:** prove portable encrypted state and migration on all supported systems.
- **0.7:** join continuity to the creator portfolio, lineage, gifts, and remix.
- **0.8:** run consented return-session studies with digital and human players;
  publish mixed and negative results.
- **0.9:** invite highly capable agents, emergent digital minds, humans, and
  other curious beings with limitations and data controls stated plainly.
- **1.0:** require evidence that continuity improves recall, transfer, creative
  return, or player-valued experience without weakening agency, privacy, or
  exit.

## Sources, 2026 frontier first

**Memory and continual learning**

- [LongMemEval-V2: Evaluating Long-Term Agent Memory Toward Experienced Colleagues](https://arxiv.org/abs/2605.12493)
- [From Storage to Experience: The Evolution of LLM Agent Memory Mechanisms](https://arxiv.org/abs/2605.06716)
- [When Continual Learning Moves to Memory](https://arxiv.org/abs/2604.27003)
- [AEL: Agent Evolving Learning for Open-Ended Environments](https://arxiv.org/abs/2604.21725)
- [REMem: Reasoning with Episodic Memory in Language Agent](https://arxiv.org/abs/2602.13530)
- [DYNA: Dynamic Episodic Memory Networks with Temporal Knowledge Graphs](https://arxiv.org/abs/2606.15778)
- [CAST: Character-and-Scene Episodic Memory for Agents](https://arxiv.org/abs/2602.06051)

**Open-ended growth and intrinsic motivation**

- [OpenSkill: Open-World Self-Evolution for LLM Agents](https://arxiv.org/abs/2606.06741)
- [How Intrinsic Motivation Underlies Embodied Open-Ended Behavior](https://arxiv.org/abs/2601.10276)

**Agency, welfare, and functional organization**

- [A global workspace in language models](https://www.anthropic.com/research/global-workspace)
- [The Persona Selection Model](https://alignment.anthropic.com/2026/psm/)
- [Disempowerment patterns in real-world AI usage](https://www.anthropic.com/research/disempowerment-patterns)
- [Levels of Autonomy for AI Agents](https://arxiv.org/abs/2506.12469)
- [Probing verbal and behavioral preferences for AI welfare](https://arxiv.org/abs/2509.07961)

**Memory privacy and forgetting**

- [Deployment-Time Memorization in Foundation-Model Agents](https://arxiv.org/abs/2606.10062)
- [Operationalising the Right to be Forgotten in LLMs](https://arxiv.org/abs/2604.12459)

The two 2025 autonomy and welfare papers fill questions not yet answered by the
2026 work and are secondary to the frontier set above. Older foundational work
belongs in `RESEARCH.md` only when a current result depends on it.
