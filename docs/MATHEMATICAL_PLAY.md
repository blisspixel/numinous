# Mathematical play and useful instruments

**Designed direction, reviewed 2026-10-08.** This document owns how the paper
survey becomes a buildable research backlog. [RESEARCH.md](RESEARCH.md#mathematical-play-and-useful-instruments)
owns the research synthesis and evidence limits. [ROADMAP.md](ROADMAP.md) owns
release order. The [generated work register](evidence/math-atlas/coverage.md)
accounts for every indexed manuscript and records each proposed slice's scope,
dependencies, checks and reason to revisit or reject it. Its canonical input is
[opportunities.json](evidence/math-atlas/opportunities.json); paper-specific
assignments live alongside the reading evidence in
[papers.json](evidence/math-atlas/papers.json).
Each manuscript also has a generated Markdown sidecar. [RESEARCH_GRAPH.md](RESEARCH_GRAPH.md)
owns how those notes and their connections help investigate useful next steps.

## One world, many reasons to enter

Numinous is a place to play with mathematics, make things, and do useful work.
A child might drag a point because the resulting curve is lovely. A
mathematician might drag the same point to explore a bifurcation, then ask for
a parameter table. A digital player might construct an unusual example and
give another player a reproducible challenge. These are different intentions
around the same object. Age, credentials, embodiment and progression never
select a restricted mathematical world.

The first action should be legible and satisfying without a lesson. Depth
should be available immediately: definitions, exact input, domains, assumptions,
methods, residuals, references and export. A specialist should be able to skip
the challenge and solve a problem directly. A player should be able to ignore
the inspector and remain with a beautiful, responsive experience indefinitely.
Neither route is a lesser use of the instrument.

An optional game comes from the object's structure: make a counterexample,
preserve an invariant, improve a route, recover a message, hide an interior,
construct a shape, or trade a puzzle. Undo, reset, copying and personal
constraints make experimentation cheap. A surprising failed conjecture can
become a kept creation. Points, rewards and exposition cannot rescue an action
that is dull in free play.

Beauty must convey the mathematics. A current carries conserved mass; a quiet
line identifies the chosen mode; a color change exposes a residual or selected
quantity. Sound mappings name what changes and can be disabled. Stillness,
silence, repetition and simply watching remain complete choices. Reduced
motion, color-free readings, bounded flashes and the existing shared audio
chain apply to every new instrument.

## Patterns and relationships are the play

The central pleasure to design for is noticing a relationship, changing it,
and discovering what follows. Repetition, symmetry, interference, recurrence,
invariants, and a small change with a large consequence can sustain play
before a score or explanation appears. This is a design hypothesis about the
experience, not a claim that every player enjoys the same patterns.

A game gives that curiosity an optional aim: make two rhythms meet, preserve
a shape while changing its parts, find where an apparent rule breaks, or make
a puzzle for someone else. The moves must change the mathematical object.
The visible or audible response must follow that change, and a result should
be inspectable and reproducible. Free exploration remains available alongside
the challenge, including after its target has been reached.

These are Designed game sketches within the existing work items:

| Game invitation | What the player changes | Visual and auditory response | Research or useful mathematical task |
| --- | --- | --- | --- |
| Make the voices meet | Oscillator frequencies and observation window | Traces approach or miss recurrence; tones expose the chosen frequency relationship | Compare exact periods, approximate returns, and the half-period trap; extend the existing Returning home construction |
| Tame the echoes | A finite sequence's signs or coefficients | Shifted copies and correlation peaks update; an optional sound mapping follows the selected lag and value | Search for low sidelobes under declared periodic or aperiodic correlation conventions; inspect exact sums in `wave-code-instrument` |
| Fool the route finder | Road costs, closures, and delivery order | The chosen street walk changes; an optional cost sonification accompanies the numeric comparison | Construct a heuristic counterexample and compare with the bounded exact optimum in `graph-workbench`; moving a drawn point alone does not change road cost |
| Paint a moving balance | Finite masses, sites, and target amounts | Weighted cells reorganize; an optional mapped pulse exposes marginal error | Inspect mass conservation, cost, and the cell-mass Jacobian, using the [power-cell construction](evidence/math-atlas/papers/p708.md) |
| Find the hidden order | Point arrangement and interaction width | Spatial and frequency views change together; selected spectral quantities can drive declared sound mappings | Compare finite energies and truncation effects using the [reciprocal-energy construction](evidence/math-atlas/papers/p178.md) |

Distinguish a discovery new to this player, a verified result for a bounded
instance, and an advance on an open research question. A saved example can be
real mathematical work without proving the source paper's theorem. Each
challenge must name the model, legal moves, objective, assumptions, verification
method, and what remains unknown. A solver result and a sound cue have different
evidential roles; musical resolution alone cannot certify optimality or truth.

For digital players, expose the same relationships through precise actions,
compact observations, comparable states, and optional sensory attachments.
A client that receives only text still needs room to experiment and create.
[Digital play research](DIGITAL_DEVELOPMENT.md#october-2026-interest-and-freely-chosen-play)
owns the evidence, current gaps, and optional evaluation of what players choose.

## From a manuscript to a candidate

A source theorem is not a product specification. For each paper, retain the
claimed result and then look inside its construction for a finite object,
algorithm, example, recurrence, certificate, obstruction or useful failure.
Record the actual source file and section inspected. A proof mechanism can
suggest a useful instrument even when the final theorem is too abstract or
its constants are impractical. Conversely, an appealing picture may only
illustrate a prerequisite and must be labeled that way.

Each source-anchored record answers:

- What mechanism was found, where is it in the source, and what was read?
- What can someone touch, hear, make, change or challenge?
- What result would justify a specialist returning with their own input?
- What exact finite model, bounded computation or approximation is proposed?
- What object could move to another room, and what survives that transfer?
- Which hypotheses, dimensions, coefficient domains and numerical limitations
  prevent the idea from becoming a false demonstration?
- What should be investigated next, and why is the current disposition sensible?

The audit is a selected-construction opportunity review, not line-by-line
proof verification. Source retrieval, source reading, mathematical verification,
implementation correctness, and evidence of enjoyment remain separate claims.
All manuscripts receiving a record establishes coverage of the pinned catalog;
it cannot establish that every useful insight has been found. New questions,
independent mathematical review and source corrections should keep changing it.

## Selection and order

Keep the Sensory Alpha work and the existing release gates in their current
order. The research backlog feeds room depth and creator work as their
foundations become ready. A new paper does not silently create a release gate
or justify a new room.

The first selection round compares existing-object slices:

| Candidate | First playable action | Concrete useful job | Why consider it early |
| --- | --- | --- | --- |
| `graph-workbench` | Change roads or costs to defeat a route heuristic, then repair the route | Export a replayable comparison with a checked finite baseline | Route Lab already owns maps, algorithms and comparisons |
| `wave-code-instrument` | Flip coefficients and hear/see a pattern change | Inspect finite autocorrelation with explicit conventions | Studio already keeps coefficient-based sound creations |
| `curve-inspector` | Move a bracket and follow a zero as a slider changes | Export a bounded root/sweep result with domain and stopping status | Studio already owns the expression and parameters |

The shared foundation, `object-readings`, starts inside the selected slice.
It should not become an abstraction project before a real operation needs it.
The default first candidate is the route comparison because its exact bounded
baseline already exists. Select a different candidate only with a concrete
advantage in the paired play and expert tasks, not because its paper sounds
more prestigious. The work register specifies what each candidate deliberately
leaves out.

Next, evaluate one reusable object family, such as small matrices, exact
polygons/norms or finite complexes. Then consider a more expressive prototype,
such as transport painting, periodic bubbles, domain modes or reversible
circuits. An inverse-probe experience first needs a tractable forward model.
Persistent terminal editing follows useful core operations; it stays within
the CLI face. [INTERFACES.md](INTERFACES.md#native-mathematical-tools) owns the
computational comparison and terminal contract.

The paper-level register uses these dispositions:

- **Candidate:** a possible destination in a scoped work item. It remains
  Designed and unselected until its slice is promoted into the active roadmap.
- **Research lens:** preserve the construction, prerequisite, connection and
  open question; a faithful near-term instrument is still unresolved. This can
  be valuable specialist content without becoming a new room.
- **Deferred:** retain a specific obstacle and the condition that would change
  the decision. Do not disguise a missing model with a decorative animation.

The atlas's prototype/explore/background labels describe research promise.
They do not override these execution dispositions. A research-promising idea
can remain unselected because its required object or solver is not yet owned.

## Carry something through the door

Selected constructions make the proposed experience more concrete. Each link
opens the pinned manuscript's record, including its source sections and limits:

| Construction | Play | Useful work and next door | Boundary |
| --- | --- | --- | --- |
| [Complex lens map, p173](evidence/math-atlas/index.html#p173) | Paint a grid and bend it through a lens; trace a point backward | Inspect series error and inverse conditioning, connecting function painting to curve inspection | The auxiliary planar map is not the complete higher-dimensional embedding |
| [Projection bodies, p175](evidence/math-atlas/index.html#p175) | Assemble a shape from its shadows, then hunt for a failed bound | Export an exact dimension-product inequality, connecting convex geometry to certificate arithmetic | The decisive counterexample is high-dimensional; the low-dimensional picture is a prerequisite |
| [Glued cut laws, p177](evidence/math-atlas/index.html#p177) | Patch small probability puzzles together and find incompatible overlaps | Keep separator marginals and compare the induced cut distances with graph paths | Factorized local laws do not make unrestricted global table enumeration cheap |
| [Reciprocal Gaussian energy, p178](evidence/math-atlas/index.html#p178) | Change interaction width and watch real-space and frequency-space patterns respond | Export pair energies, scale conventions and tails, connecting lattices to spectral inspection | A finite pattern does not certify the manuscript's infinite optimality claim |
| [Rational quadrature, p180](evidence/math-atlas/index.html#p180) | Refine a calculation until an uncertain matrix reading becomes decisive | Replay a small certificate and inspect each error contribution, connecting matrix work to exact arithmetic | The finite block leaves the full analytic certificate unverified |
| [Finite mass and power cells, p708](evidence/math-atlas/index.html#p708) | Move painted mass and watch cell boundaries reorganize | Inspect marginal errors and the cell-mass Jacobian, connecting transport painting to a weighted graph | A smooth animation is not a certified continuum transport map |

These examples are design inferences from selected source constructions. They
show why the same object can invite casual experimentation and sustained
technical work. They do not add another priority queue beside the work register.

Connections earn priority when a creation survives them. These are proposed
contracts, not existing arbitrary conversions:

| Starting object | Next interpretation | What must travel with it | What must not be implied |
| --- | --- | --- | --- |
| Route graph | Random walk, matrix, robust smoothing | Vertex identity, edge semantics, weights, normalization and any stationary law | Every weighted graph already defines the intended reversible chain |
| Coefficient sequence | Waveform, correlation, noisy message | Exact coefficients, sample convention, periodicity, conjugation, normalization and seed | A pleasing audio rendering proves communication capacity |
| Convex polygon | Polar body, unit ball, distances, balancing game | Coordinates, origin, scale and metric convention | A two-dimensional example proves a high-dimensional extremal theorem |
| Tile or arrow configuration | Height surface and sampled ensemble | Local rules, boundary data, winding and move law | Legal moves automatically sample the claimed distribution |
| Painted finite mass | Coupling, morph and transport cost | Marginals, total mass, cost, plan/map distinction and regularization | A visually smooth morph is a continuum optimal map |
| Domain or finite complex | Modes or algebraic invariants | Geometry/incidence, operator, boundary conditions or coefficient domain | Different operators share a spectrum, or equal homology means equivalent spaces |
| Finite program | Reversible motion or circuit puzzle | State space, gates, inverse and preserved quantity | A finite simulation establishes continuum universality or solves halting |

The graph distinguishes these proposed transfers from catalog membership.
Paper-to-paper edges include a reason and a transfer boundary. A shared word
alone is insufficient: cosmological expansion is not norm distortion, integer
partitions are not set partitions, and category arrows are not modular forms.
Some papers correctly have no direct existing-room edge.

## Acceptance: delight and utility both matter

Before implementing a selected slice, write a bounded task for each relevant
kind of use. For example, make and exchange a route that fools a heuristic;
inspect the exact comparison and keep a witness; then reopen the same state
over MCP and ask a different question. The mathematical state and available
operations should agree even when presentation differs.

| Dimension | Evidence required before the corresponding claim |
| --- | --- |
| Mathematical trust | Independent finite fixtures or references; domain and degeneracy handling; exact/approximate/heuristic status; appropriate residuals or certificates; resource limits |
| Useful work | A concrete task completed with inspectable output and export, including failure cases; no manual transcription from pixels required |
| Play | A reversible action with meaningful choices, an optional challenge arising from the model, and a creation worth keeping or sharing; scripted success alone does not establish enjoyment |
| Beauty and awe | A deliberate visual or sonic relationship to the modeled quantity, coherent pacing, and observations of response when available; image polish alone does not establish awe |
| Peer access | Equivalent mathematical agency through App, CLI and MCP; immediate study and exact input; no puzzle or progression requirement to use the instrument |
| Continuity | Create, change, compare, keep, reopen, export and remix preserve the actual object and its assumptions through the existing Studio/project boundaries |

Do not infer child enjoyment from expert success, specialist usefulness from
a citation, or a digital player's experience from a scripted transcript.
Human feedback remains optional under the release policy; unobserved claims
stay Hypothesis. Mathematical and interface regressions can still be required
without pretending they measure delight. Keep the strongest failure or
counterexample alongside favorable evidence.

## Engineering boundary and maintenance

Rust is the default for product code and new maintained tooling. Core owns
objects, validation, bounded operations and result semantics. Faces translate
actions and presentation. Reuse the existing expression grammar, persistence,
creation lineage and project chain before introducing another abstraction.
The pinned stack stays closed until a concrete requirement justifies a change.

Every operation needs inputs with declared bounds, cancellation where work is
long, and honest failure output. A useful small instrument can expose exact
rational results, numerical residuals or finite witnesses without pretending
to be a universal symbolic system. Broader algebra, PDE solvers, arbitrary
natural-language queries and external factual databases require separate
decisions and evidence.

The Rust atlas builder validates source identity, cited-source provenance,
paper and work-item references, live room IDs, dependency cycles and generated
outputs. Run `cargo run -p numinous-cli --example math_atlas --locked` to rebuild,
or append `-- --check` to reject drift. Source text is linked at the pinned
revision, not copied into player content. Local source caches remain ignored.
When reviewing a later source snapshot, reconcile by source path, inspect
withdrawals and repairs, and reassess the affected claims and designs. The
snapshot-local paper number is a navigation aid, not a permanent research ID.
