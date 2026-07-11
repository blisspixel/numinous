# Creator: closing the make-share-remix loop

How Numinous goes from a curated collection to a living world others help build.
This doc owns the creator platform and community design; `EXTENSIBILITY.md` owns
the safety tiers it rides on, and `STUDIO.md` owns the instrument. Phase D of
`NORTH_STAR.md`.

## The strategic read

Numinous is unusually well positioned. The `Room` contract is a pure
deterministic function, the `.num` capsule is a single size-capped text file, and
`EXTENSIBILITY.md` already resolves the two hardest questions most user-generated-
content platforms botch: the safety model (the language is the sandbox,
determinism is the validator) and the trust model (curate for beauty, sign for
provenance, sandbox regardless). That is a decade of hard-won lessons pre-paid.

The gap is not safety or format. It is that **the creative loop does not close
yet.** Today you can `plot --save` a `.num`, but you cannot open one back into the
app, browse a gallery, or fork someone's creation. Every thriving creator
community (Scratch, PICO-8, Observable, Desmos, Baba Is You) is fundamentally a
**fork loop with a taste layer**, and the fork loop comes first: make, share,
others open and study and remix, make again. Numinous has the substrate and not
the loop. The whole strategy reduces to one sentence: **close the make-share-
remix loop on the `.num` capsule, and make human curation the taste layer on top
of determinism-as-safety.**

## The Minimum Lovable Creator Surface (build first, in order)

**1. Reopen plus the room-manifest capsule (Tier 1).** Grow `StudioCreation`
(today: one expression, xmin/xmax/a) into the room manifest specified in
`EXTENSIBILITY.md`: multiple expressions, named sliders with ranges, a palette
and Visual Era, sound parameters from fixed enums, and metadata (title, author).
Add app-side reopen so a capsule reopens exactly, live and singing. Keep the
hand-written strict parser and the per-field caps. This is the floor everything
stands on; nothing else works without reopen.

**2. The one-button share bundle.** On any Studio state, one action emits the
trio: the `.num` file, the `numinous://` link, and the PNG postcard. The link
opens into a paused preview until confirmed (the hostile-input rule from
`EXTENSIBILITY.md`). The PNG is the viral object that escapes the app; the `.num`
reopens it exactly; the link is the frictionless handoff. This is PICO-8's growth
property: the cart is simultaneously the screenshot, the playable, and the source.

**3. The local Gallery plus Fork.** A wall of creations you can play in place and
fork with one keystroke. Fork opens a copy in the Studio with lineage recorded.
Start local-first (a folder of `.num` files rendered as live thumbnails) so it
ships before any server exists. Fork must be as cheap as play; the remixers are
the engine of a creative community (the Scratch research is explicit on this).

**4. Lineage that credits generously.** Every fork records "descends from,"
building a visible remix tree, but avoid the failure Scratch's own researchers
documented (automatic attribution falls short and demotivates original authors).
Make credit generous and human-legible: an ancestry chain, a "remixed N times"
badge that is a point of pride for the parent, and prose credit the forker can
edit. Credit flows up the tree by construction, so remixing feels like honoring,
not stealing.

That is the loop: reopen, share-bundle, gallery, fork, lineage. A non-programmer
types `y = sin(a*x)`, drags `a` until it is beautiful, picks the 8-bit Era, names
it, and shares a thing others can open, study, and build on. All Tier 1, all safe
by construction.

## The creation ladder (one tool, rising ceiling)

The Studio ladder maps onto the safety tiers so the thing you make and the thing
you share are the same object at every rung:

| Rung | You type | Shared as | Tier |
|---|---|---|---|
| Doodle | `y = sin(x)` | `.num` (one expression) | Tier 1 |
| Toy | `sin(a*x + t)`, drag `a` | `.num` manifest (sliders, palette, sound) | Tier 1 |
| Instrument | `euclid(3,8)`, layered patterns | `.num` manifest (pattern algebra) | Tier 2 |
| Room | the above plus a challenge and a reveal | signed capsule via portal | Tier 2 |

The critical rule: **there is no export or convert step between rungs.** A doodle
is a capsule; adding a slider is editing the same `.num`; promote-to-room adds a
challenge and reveal to the same file. You never leave the instrument and never
hit a wall. A player stops at any rung with something real, beautiful, and
shareable.

## The gallery experience

Three surfaces, each for a different visitor:

1. **The Daily** (lean-back): one hand-picked community room, rotating daily,
   with slots reserved for first-time creators (the Desmos contest model
   deliberately does this). A periodic featured moment is the community heartbeat.
2. **The Wall** (browser): a live-thumbnail grid, sort by Newest, Most-Forked, or
   Staff-Picked. Every tile plays on hover and forks on click. No infinite
   algorithmic feed, no "recommended for you."
3. **The Tree** (digger): the remix lineage, where you trace a phenomenon's
   evolution and jump in at any node. A visible tree is itself compelling content
   and it solves attribution generously.

## Promote-to-room and the proof-packet portal (Tier 2, 2.0)

The graduation pipeline: a Studio creation that adds behavior, a challenge, and a
reveal becomes a catalog-shaped room. Submission is the `EXTENSIBILITY.md` proof
packet (source, seeds, frame hashes at canonical triples, audio-spec hash,
declared budgets). Portal CI re-renders headless (the CLI face exists for exactly
this) and rejects on hash mismatch, budget overrun, or fuzz-corpus panic. The
reveal's math claim faces the same sign-off gate first-party reveals face, which
lets Numinous promise its community rooms are *true*, a differentiator no other
user-generated math platform has. Approved capsules are signed (ed25519) and
labeled Curated vs Unverified. Critically, an unsigned capsule still runs, in the
same sandbox, with the same budgets: **signatures label trust, they never grant
capability.** The graduation bar is the existing Room Definition of Done,
unchanged, so a community room graduates to the same checklist a first-party room
passes. That is what keeps "living world" from meaning "quality collapse."

## Community and curation without dark patterns

- **Two-track, like Steam Workshop.** An open track (anything determinism-valid
  runs, labeled Unverified) and a Curated track (human-reviewed for the beauty
  bar). Nobody is gatekept from sharing; the front door stays gorgeous.
- **Curation is human and event-shaped.** Periodic themed showcases ("closed
  curves," "one rule, infinite structure," "self-reference") are the community's
  rhythm and the best cold-start growth mechanic, because they manufacture a
  deadline, a theme, and a reason to make.
- **Reputation is authorship, not score.** A creator's standing is the body of
  work others chose to build on (the remix tree), plus curated features earned.
  No follower counts to farm, no engagement feed, no streak-shaming. Rank by
  forked-from (a real usefulness signal) and human staff picks, never by a raw
  vanity metric. Beauty outranks popularity: a one-fork creation a curator loves
  can be the Daily.

## Co-creation with digital minds

The genuinely novel part, and equal footing by construction: duet capsules with
dual authorship in the lineage (a human wrote the geometry, an agent the sound),
gifting as a native share verb ("I found this and thought of you"), and
`create_room` over MCP so an agent submits to the same proof-packet portal a
human does. A mind's room is signed, curated, and featured on the same terms, and
a long-lived mind can author and curate its own wing. See `DIGITAL_MINDS.md`.

## New spec doc, planned

`CAPSULE.md` (normative, to write when Phase D starts): the `.num` room-manifest
format, every field and its cap, the character whitelist, the versioning scheme
(`NUMINOUS_STUDIO 1` to `NUMINOUS_ROOM 1`), the proof-packet fields, and the
round-trip and fuzz invariants (the totality harness in `crates/core/src/studio.rs`
already guards the current format). It exists so the parser, the CLI, the app, and
any future embed all agree on one strict grammar.

## Sources

- Scratch remix community and the remixing dilemma:
  https://www.nature.com/articles/sdata20172 and https://arxiv.org/pdf/1507.01295
- PICO-8 cart as playable-plus-inspectable-plus-source:
  https://nerdyteachers.com/PICO-8/Guide/SHARE
- Desmos Math Art Contest (human curation, newcomer slots):
  https://help.desmos.com/hc/en-us/articles/41063816142221-2025-Desmos-Studio-Math-Art-Contest
- Steam Workshop (open plus curated at scale):
  https://partner.steamgames.com/doc/features/workshop
