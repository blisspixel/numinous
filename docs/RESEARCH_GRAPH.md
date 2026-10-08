# A research graph that helps us build Numinous

**Reviewed 2026-10-08.** The offline atlas is a research tool. Its records,
Markdown sidecars, typed links, relationship lenses and evidence inspector are
built by the Rust atlas example. The richer mathematical model and workflows
below are Designed. Their usefulness needs evaluation; the existence or size
of a graph is not evidence that it improves our decisions.

[MATHEMATICAL_PLAY.md](MATHEMATICAL_PLAY.md) owns the product selection rules.
[RESEARCH.md](RESEARCH.md#mathematical-play-and-useful-instruments) owns the paper
survey. [CONSTELLATION.md](CONSTELLATION.md) is a separate proposed player
experience. A research graph must not silently become a progression gate.

## Start with a note, then connect the notes

The unit of research is an individual manuscript record, readable as a
[Markdown sidecar](evidence/math-atlas/papers/p175.md). A note preserves the
mathematical question, source sections inspected, constructions, visual or
sonic possibilities, play loop, useful expert task, limits, open questions and
roadmap disposition. It links to related notes and to its graph neighborhood.
The [register](evidence/math-atlas/coverage.md) links every indexed manuscript.
Open the atlas inside its research folder to follow local sidecar links. The
HTML contains its interactive data, but copying that file alone does not copy
the separate Markdown notes.

The canonical record remains in `evidence/math-atlas/papers.json`; source
identity and hashes live in `sources.json`, and work-item scope lives in
`opportunities.json`. Edit those records and rebuild with
`cargo run -p numinous-cli --example math_atlas --locked`. The Markdown files,
graph, register and viewer are derived views. `-- --check` rejects drift,
including a stale sidecar. This avoids maintaining conflicting research in
JSON and Markdown. A future authoring UI could edit one record at a time
without changing this ownership boundary.

The graph helps when it answers a question we care about:

- Which constructions could deepen this room, and what would a player do?
- Which useful operation would serve several otherwise distant papers?
- Can this saved object move between those rooms without changing its meaning?
- What assumption blocks the proposed transfer, and can a tiny example expose it?
- What remains unknown before selecting this prototype?
- If a source claim changes, which notes and proposals need another review?

The current graph supports lookup, explicit transfers and work-item grouping.
It does not automatically answer all these questions. Shared membership is a
lead to inspect, not a demonstrated mathematical relationship.

## What the research suggests

These observations come from primary project documentation and research. The
Numinous responses are design inferences, not features inherited from those
systems or evidence that they will work for our players.

| Reference | Useful observation | Numinous response |
| --- | --- | --- |
| [LMFDB development guide](https://github.com/LMFDB/lmfdb/blob/main/Development.md#adding-material-to-the-lmfdb) | Novices can browse concrete objects while experts search directly; object pages join properties, related objects, downloads and expandable definitions | Give a mathematical object an inviting example and precise inspection on the same page. Let a player follow a connection without first learning a taxonomy |
| [OMDoc](https://kwarc.info/systems/omdoc/) and [MMT theory graphs](https://kwarc.info/people/mkohlhase/papers/ems13.pdf) | Mathematical knowledge includes statements in contexts and explicit mappings between theories | Name the representation and assumptions a transfer needs. A shared word such as duality or median is insufficient; an analogy is not a theorem-preserving map |
| [Mathlib import graph](https://leanprover-community.github.io/mathlib4_docs/mathlib.html) | A navigable graph can reveal imports and directory groupings | Keep an import or source dependency layer distinct from mathematical prerequisites and suggested play routes |
| [The Network Structure of Mathlib](https://arxiv.org/abs/2604.24797) | Its reported analysis distinguishes mathematical, logical and infrastructure structure; centrality can emphasize language infrastructure | Do not rank product value by degree or centrality. Rank a proposed use against a concrete task, then expose the reason for that ranking |
| [MatrixExplorer](https://aviz.fr/~fekete/matrixexplorer/) | Matrix and node-link views support different graph-reading tasks; tracing paths in a matrix is awkward | Use small node neighborhoods for following a path, and tables or matrices for comparing many papers against tools. A single animated network is not the only useful view |
| [PROV-O](https://www.w3.org/TR/prov-o/) | Provenance can qualify derivations and distinguish source entities from the activities that produce later entities | Keep source revision, inspected section, extraction, design inference and later experiment result distinct. Adopt the useful structure without requiring an RDF service |
| [MathAlgoDB ontology](https://mathalgodb.mardi4nfdi.de/static/widoco/v1/index-en.html) | Problems, algorithms, software, benchmarks and publications have distinct roles and relations | A manuscript can discuss an operation without a room implementing it. Link an implementation and a finite check explicitly when they exist |
| [OpenMath technical overview](https://openmath.org/technical/) | Symbol meanings and supported content dictionaries are explicit; aligning different mathematical dialects is a separate problem | Preserve coefficient domain, basis, ordering, precision and exactness in portable objects. A matching label alone does not make two representations compatible |

## Improve meaning before adding density

The present graph contains papers, catalog families, concept tags, existing
rooms, capability groups and scoped work items. Paper-to-paper transfers retain
a rationale and a boundary. The viewer can isolate transfers, rooms, roadmap
relationships, concepts or catalog grouping, and inspect every direct edge's
direction and evidence label. Its limited drawing is accompanied by the full
direct-edge list. These are navigation aids, not proof search.

The next model should add first-class records only where they answer a real
question. A useful small schema would distinguish:

| Record | Example | Why it deserves identity |
| --- | --- | --- |
| Mathematical object | A finite weighted graph with declared directedness and positive conductances | Different papers can operate on the same exact input |
| Construction or operation | Build a Laplacian, glue separator marginals, compute finite autocorrelation | Reveals reusable implementation below broad topic labels |
| Claim | A specified bound under named hypotheses | Keeps manuscript claims separate from our proposed application |
| Question | Whether a small certificate can be replayed with bounded arithmetic | Gives research an explicit unresolved state and possible next experiment |
| Experiment or witness | Input, seed, operation, result, residual or certificate | Connects research to something inspectable and reproducible |

Use controlled relation kinds such as `constructs`, `requires-assumption`,
`uses-operation`, `has-counterexample`, `approximates`, `tests-question`, and
`proposes-room-use`. A flexible explanatory sentence belongs alongside the
kind. Existing free-text transfer descriptions should be reviewed into this
vocabulary, not relabeled automatically. Catalog membership, citation,
analogy, a formally checked dependency and an executable object conversion
must remain distinguishable.

For a proposed conversion, record source and target representations, required
conditions, the operation, preserved quantities, discarded information and a
check. Conditions involving several inputs belong on the conversion record;
separate pairwise edges must not imply that any one input is sufficient. A
path through analogies is not a proof. Two individually meaningful conversions
may fail to compose when the first loses an assumption the second needs.

Give curated insights and questions stable IDs before using them as dependency
targets. Attach a transfer to the particular insight and source locator that
supports it, and give a question an open, investigated, resolved or deferred
state with evidence. Keep failed transfers and negative results. Whole-paper
links are too coarse to determine which proposals a repaired lemma affects.
These fields are a designed extension; the current graph does not yet perform
that change-impact analysis.

Use stable semantic identifiers and aliases for curated objects and operations.
Retain pinned source paths for manuscripts. The current `pNNN` IDs identify
records within this snapshot; they are not permanent mathematical identities.
Similarity search may suggest a candidate link for review. It must not silently
create a prerequisite, merge homonyms, or attach a mathematical truth label.

## A concrete route through the graph

Start with [p177's cut laws](evidence/math-atlas/papers/p177.md). The note names
small probability tables on graph bags and matching separator marginals.
Those suggest an operation: check overlap consistency and glue a joint law.
The same law defines distances by measuring which cuts separate two vertices.
Now a probability puzzle and a graph-distance inspector share one object.

The source paper's global embedding result remains separate. The first useful
prototype could use a tiny tree decomposition, exact rational probabilities,
an intentionally inconsistent overlap, and an independently recomputed distance
table. A child can repair the overlap; a specialist can inspect or export the
law. That is a reason to follow a graph edge, and a bounded engineering task.

Other candidate trails include coefficients to correlation to a noisy message;
convex polygons to polar bodies to metric balls; and finite painted mass to
power cells to a weighted Laplacian. Each trail must name actual representations
and the conversions still missing. A visible route is not a promise that its
operations are implemented.

## Views for different questions

Keep an object or note at the center, with a small, stable neighborhood and
the question visible. Offer optional expansion rather than showing the entire
corpus at once. Preserve a breadcrumb when following a route, show why each
edge was chosen, and make it easy to return to the original question.

A comparison table is better for asking which proposed tool serves named
papers and rooms. A source view is better for reviewing a claim. A short path
is better for imagining a play sequence. Expert filters can select coefficient
domain, finite versus continuum model, exact versus approximate operation,
dimension, prerequisites, and evidence status once those fields are curated.
Unknown fields must remain unknown.

For a player, the eventual door should offer a concrete action such as changing
a weight, hearing a coefficient pattern, or carrying a polygon to its polar.
For research, the door can lead to a sidecar, an unresolved question or a
prototype task. CLI and MCP should receive structured records and operations
when a runtime graph slice is selected; the offline attachment does not add
those endpoints now. Browsing, study and useful tools stay freely accessible.

## A bounded implementation order

1. **Readable notes and inspectable edges.** Complete source-anchored sidecars,
   relationship lenses and explicit edge evidence. Keep the Rust build and
   drift checks authoritative. This is the current offline research slice.
2. **Curate one object trail.** Use a concrete example such as cut laws. Give
   its objects, operations and unresolved questions identities, with explicit
   assumptions and transfer checks. Review misleading near-matches as well as
   valid links before broadening the schema.
3. **Add a question-driven comparison.** Compare the graph against plain
   keyword search and the sidecars alone. Add path explanations and a work-item
   matrix only if they help with the selected tasks.
4. **Connect a verified operation.** Reuse the existing core object and saved
   creation boundary. A graph action must invoke the owned operation and
   preserve its actual result, not implement mathematics in the viewer.
5. **Evaluate a player-facing route.** Reuse the same reviewed relationships
   in a freely browsable experience, with personal discovery annotations
   separate from mathematical truth and access permissions.

No graph database, embedding service, new symbolic backend or stack migration
is required for the first steps. Rust owns maintained extraction, validation
and future core operations. The offline page presents the records.

## How to tell whether it helps

Build a small reviewed task set before tuning navigation. It should include
finding a reusable operation across distant papers, choosing a finite example
for a room, detecting an invalid transfer, finding a blocking assumption, and
tracking proposals affected by a source correction. Preserve the expected
evidence and known misleading paths for each task.

Compare search alone, notes alone, and notes plus graph on those same tasks.
Record supported answers, missed useful connections, false connections,
inspected evidence and time to a justified decision. A correct refusal to
connect incompatible objects matters as much as a surprising valid connection.
Measure prototype selection and useful reuse separately from node count,
clicks, session duration or visual appeal.

Examples of regression cases include cosmological expansion versus norm
distortion, integer partitions versus set partitions, metric medians versus
coordinate medians, and finite sampling versus a continuum theorem. A source
repair should flag dependent proposals for review, without declaring unrelated
experiments invalid. None of these automated checks establishes delight or
human understanding; those remain separate questions under the product's
existing evidence policy.
