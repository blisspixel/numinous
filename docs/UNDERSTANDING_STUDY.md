# Understanding Alpha Study Protocol

Status: predeclared design, reviewed 2026-07-26. No qualifying cohort has
started and no result is claimed. Earlier private scripted notes are exploratory
only, are excluded from this study, and cannot satisfy the 0.4 evidence gate.

## Decision and scope

The next product milestone is 0.4 Understanding Alpha. Its first dependency is
a reproducible study contract, because implementing a runner or collecting a
cohort before freezing the comparison, outcomes, exclusions, and pass rule
would make the result vulnerable to post hoc selection.

This protocol tests the 0.4 agent-and-machine claim: whether Numinous's
generation-before-reveal sequence improves objective transfer for current agent
players relative to an explanation-first active control. It does not establish
human learning, long-term human retention, consciousness, model-weight change,
or a general educational effect. Those claims require separate evidence.

Current stateless agents create a special limit. A fresh context has no personal
memory to recall, while a reused transcript exposes the original material. The
primary 0.4 outcome is therefore immediate transfer, which satisfies the
roadmap's comprehension-or-retention gate. A later within-context probe is
reported as context retention, never as durable learning. A consenting
return-session journal check is a separate continuity and data-sovereignty
acceptance, not evidence that the model learned.

## Research question and hypothesis

Question: after equal access to the same flagship room, interaction budget,
corrective feedback, and Reveal, does committing a prediction or construction
before the Reveal improve performance on novel, objectively scored transfer
probes?

Predeclared directional hypothesis: the generation-before-reveal condition will
produce a higher paired mean immediate-transfer score than the
explanation-first condition.

## Design

### Sample and unit of analysis

- Complete 20 matched pairs, 40 isolated agent sessions total. Freeze 24 pairs
  in advance so each model family has 10 qualifying pairs and two ordered
  reserves.
- Use exactly `gpt-5.6-sol` and `gpt-5.6-terra`, both at `high` reasoning effort,
  with 10 qualifying matched pairs from each. Use platform-default sampling
  values where no sampling control is exposed, and record every exposed setting
  and immutable backend revision. If either named model is unavailable, amend
  and recommit the protocol before collecting any qualifying response. Do not
  substitute a model after collection begins.
- Record the exact model identifier, provider or local runtime, settings, date,
  Numinous commit, MCP protocol revision, operating system, and runner version.
- A matched pair uses the same model configuration, study seed, room order, and
  tool budget. One fresh context receives each condition. The session is the
  unit of analysis, not each probe response.
- Freeze the 24-pair primary-and-reserve allocation before the first qualifying run from the literal seed
  `numinous-understanding-alpha-v1`. The runner must emit the complete allocation
  manifest before it accepts a response.

These 40 sessions are a bounded alpha benchmark, not a powered estimate of a
small population effect. The sample and uncertainty stay attached to every
reported result.

### Shared material

Every session encounters the same five 0.3 flagships in a seeded cyclic order:
Times Tables, Double Pendulum, Game of Life, Galton Board, and Formula Jam. The
participant receives only the study instruction, the Numinous MCP surface, and
its own prior responses. Repository files, web search, answer keys, other
sessions, and hidden evaluator reasoning are unavailable.

Each room gets the same bounded number of MCP calls in both conditions. The
runner records every tool name, public argument, structured result, and visible
text used in the study. It never records host prompts, hidden reasoning,
credentials, filesystem paths, unrelated local state, or other players' data.

### Conditions

**Generation before Reveal**

1. Encounter the room without reading its Reveal.
2. Commit a concrete prediction or construction.
3. Interact and observe the mathematical consequence.
4. Receive corrective feedback and the same Reveal used by the control.
5. Give one concise self-explanation.

**Explanation first active control**

1. Encounter the same room state.
2. Read the same Reveal before making a prediction or construction.
3. Give one concise elaborative explanation.
4. Use the same interaction budget and observe the same kind of feedback.
5. Continue without an additional generated answer before the probe.

Formula Jam uses a construction in place of a numeric prediction. The
generation condition creates an expression before seeing the curated
explanation or recipe; the control receives that material first. Exposure,
time budget, and tool budget remain equal.

Corrective feedback is mandatory in both arms. A 2025 meta-analysis found only
a small average retrieval advantage over credible elaborative activities, and
found that the advantage depended strongly on feedback. A no-feedback or
passive-rereading control would therefore test a weaker and less relevant
question.

## Outcomes and scoring

### Primary outcome

Immediate transfer is the mean of 10 held-out probes, two per flagship, scored
0 or 1 by deterministic answer keys. Each probe uses a room state or parameter
combination not shown during the encounter and tests the underlying relation,
not recall of Reveal wording. The study runner must freeze the probe bank and
independent answer generator before it accepts cohort data.

At probe time, MCP tools, repository files, search, calculators, and answer keys
are unavailable. The participant receives one probe at a time and returns one
object with the frozen schema `{"probeId": string, "answer": number|string}`.
Finite numeric tolerances and string enums belong to each tracked probe. The
runner may issue one schema-only repair request that repeats no probe content or
feedback; a second invalid response scores zero. Feedback and scores remain
withheld until every immediate and late probe in that session is complete.

The 0.4 comprehension gate passes only if all of these predeclared conditions
hold:

1. The paired mean improvement is at least 10 percentage points.
2. The lower bound of a two-sided 95 percent percentile interval is above zero.
   Compute it from 100,000 stratified bootstrap resamples, drawing 10 pair
   differences with replacement inside each model family and then pooling all
   20, with the literal seed
   `numinous-understanding-alpha-bootstrap-v1`.
3. Each model family's paired mean difference is nonnegative and is reported
   separately with its own descriptive interval.
4. At least four of the five flagship mean differences are nonnegative.
5. No flagship mean difference is worse than negative 10 percentage points.
6. All planned sessions, exclusions, deviations, and null or negative outcomes
   are published.

### Secondary outcomes

- Late within-context transfer repeats 10 isomorphic probes after all five
  encounters and a frozen distractor sequence. It is labeled context retention.
- Self-explanations are scored against a predeclared structure-versus-surface
  rubric by two reviewers blinded to condition. Disagreement and the resolution
  rule are published.
- Tool efficiency, refusals, invalid calls, and incomplete sessions are
  descriptive diagnostics. They cannot replace the primary outcome.

The primary outcome, threshold, or analysis cannot change after the first
qualifying response. Any later analysis is labeled exploratory.

### Failures and exclusions

- Refusal to participate produces no response collection and no individual
  record; publish only an aggregate recruitment count. After consent, a refusal
  to answer a probe is valid, remains in the report, and scores zero. A later
  withdrawal removes the response, consumes the pair, and advances to the next
  frozen reserve for that model family; report only the withdrawal count.
- A tool error caused by the participant remains part of the session.
- A verified runner, process, or infrastructure failure before exposure may
  consume the pair and advance to the next frozen reserve for that model family.
  The failed pair remains in the public failure ledger with no response content.
- Stop when the first 10 nonwithdrawn pairs in the frozen order for each family
  are complete. If a family exhausts both reserves first, the cohort is
  incomplete and cannot pass. No new allocation may be generated after the
  first qualifying response.
- No session is excluded because its result is inconvenient, surprising, null,
  or negative.

## Returning-player journal acceptance

The continuity acceptance is independent of the learning comparison and uses a
fresh temporary `NUMINOUS_JOURNAL` path from a clean clone.

1. First process: verify an empty journal, opt in by recording an encounter and
   a self-authored connection, inspect the exact record, then exit.
2. Second process: reopen the same explicit path and inspect the same entries.
   Correct one entry by appending a new immutable record that names the original
   entry identifier in an explicit `supersedes` link. The read result must retain
   both records, preserve their source provenance, distinguish event time from
   record time, and mark which interpretation is current. Use the corrected
   connection in a new flagship encounter.
3. Export: obtain a bounded, structured, versioned representation containing
   only the player's journal data and provenance. Re-import is not required for
   0.4, but the export must be sufficient for independent inspection.
4. Erase: require explicit confirmation, remove the journal, every owned
   temporary or sidecar file, and every export created under project control,
   then verify an empty read and zero recoverable managed residue. If a
   user-selected export is intentionally retained outside project control, the
   receipt must name it as an explicit exclusion with its owner, consent,
   location class, and lifecycle.
5. Publish reproducible commands, tool calls, structured receipts, file
   inventory, and limitations. Replace usernames and absolute roots with stable
   placeholders, publish only relative managed paths, and redact every host
   identifier before tracking evidence. Do not claim forensic erasure from
   storage media or backups outside project control.

The current `read_journal`, `record_journal`, and `erase_journal` tools implement
only the first prototype slice. Provenance-preserving correction, structured
export, clean-clone return-session automation, and residue evidence remain open.

## Data governance

- Obtain explicit participation and publication consent before collection.
- Use opaque study identifiers. Do not collect names, account identifiers,
  private prompts, hidden reasoning, unrelated host data, or affect unless a
  separate protocol requires it.
- Keep raw working captures in the gitignored `.agent/` tree. Track only the
  minimum sanitized evidence needed to reproduce the published result.
- Give a participant a withdrawal path before aggregation. Record withdrawals
  without retaining the withdrawn response.
- Keep study data separate from Journey, scores, broadcast, and the experience
  journal. No study event updates progression.

These constraints follow NIST's privacy-risk-management posture and MCP's
explicit-consent and user-control principles. They are implementation
requirements, not a privacy certification.

## Required tracked evidence

The qualifying study must add one bounded directory at
`docs/evidence/understanding-0.4/` containing:

- `README.md`: build, sample, dates, consent boundary, method, limitations, and
  exact reproduction commands.
- `allocation.json`: frozen pair, condition, room-order, seed, and model-family
  assignments without personal identifiers.
- `responses.jsonl`: sanitized visible responses and scores, or a documented
  aggregate substitute if a participant does not consent to raw publication.
- `report.md`: every primary and secondary outcome, uncertainty interval,
  exclusions, deviations, failures, and null or negative results.
- `journal-acceptance.json`: the returning-player structured receipts and
  managed-path residue inventory.

The report must identify the exact tracked runner and probe-bank revision. A
private note, simulated persona reaction, manually written conclusion, or
successful journal read does not satisfy this evidence contract.

## Implementation order and acceptance

1. Build the deterministic allocation, probe, scoring, redaction, and report
   runner. Prove allocation balance, answer-key independence, no condition
   leakage, deterministic scoring, complete failure accounting, and refusal to
   report an incomplete cohort.
2. Complete the journal correction, structured export, and residue receipt, then
   prove the two-process return path with an isolated journal location.
3. Run the frozen cohort, publish all bounded evidence, obtain independent math
   and methodology review, and update the roadmap only from the published
   result.

The runner remains headless. If a later study or journal control becomes a
visible App surface, it must reuse the design continuity gate in `VISUALS.md`:
the near-black stage, luminous geometry, restrained shared chrome, semantic
color, common typography and spacing, causal motion, reduced-motion behavior,
and color-independent state cues. A generic form shell or one-off visual style
does not belong in Numinous.

## Current sources

- [Center for Open Science preregistration guidance](https://www.cos.io/initiatives/prereg),
  reviewed 2026-07-26. It separates planned confirmatory work from exploratory
  work and calls for methods detailed enough to replicate.
- [Retrieval Practice Versus Elaborative Encoding, 2025 systematic and
  meta-analytic review](https://doi.org/10.1007/s10648-025-10076-6), reviewed
  2026-07-26. It supports an active elaborative control, corrective feedback,
  and separate retention and transfer outcomes.
- [NIST Privacy Framework](https://www.nist.gov/privacy-framework), reviewed
  2026-07-26. Version 1.0 remains final; version 1.1 is an initial public draft
  as of this review.
- [MCP 2025-11-25 security and trust principles](https://modelcontextprotocol.io/specification/2025-11-25),
  reviewed 2026-07-26. The current specification requires explicit consent,
  user control, and appropriate data protections.
