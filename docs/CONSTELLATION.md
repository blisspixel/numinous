# The Constellation: the meta-map as discovery, not checklist

The redesign of Numinous's meta-progression from "collect a Constant on
Revelation" (a progress bar drawn as stars) into an Outer-Wilds Rumor-Mode map: a
per-player web where a node lights only after real understanding, edges are
insights you actively confirm, and the daily route runs across it. Phase C of
`NORTH_STAR.md`, alongside `CONSTRUCTIONS.md`. `PROGRESSION.md` remains the
philosophy; this is the spec.

## The diagnosis

Today the Constellation is "collect a Constant on Revelation." That is a
checklist, which is exactly the XP treadmill the thesis forbids, wearing a
prettier hat. A checklist tells you chores remaining. A discovery map shows you
mysteries open. The whole redesign is that swap.

## The node model

Three states, borrowed from Outer Wilds' "you are locked only by your own
ignorance" and its Rumor Mode's per-player layout:

- **Dark** (unseen): you know the room exists, nothing more. No "you need X" wall;
  gating basics is banned.
- **Touched** (Toy played): the node glows dim. You have felt it, not understood
  it.
- **Lit** (insight confirmed): the node burns, and its edges to related rooms
  appear, drawn only now, physically extending the web.

The transition to Lit is gated on the generation act from `PEDAGOGY.md`: a node
lights only after a prediction, a construction, or a self-explanation, never from
having opened a reveal card. This is what makes the map measure understanding, as
`PROGRESSION.md` already claims it does, rather than measuring exposure.

## Edges are insights you confirm

An edge is not decoration; it is a discovered theorem, and confirming it is the
act. When you reach a connection insight, do not just name it, hand the player a
tiny confirmation ("you said the Times Tables cardioid is the Mandelbrot body,
now find it in the Mandelbrot room," one click in the right place). Only then
does the edge light. The Chaos-Game self-similarity insight, once confirmed,
draws four edges at once (Mandelbrot, Koch, Times Tables cardioid, Sierpinski):
the player sees the theorem propagate across the sky. This is "one insight lights
up four rooms" made into a visible, earned event instead of a sentence.

## Constants are regions, not trophies

Each fundamental constant (pi, e, phi, i, aleph-null, gamma) owns a colored
region of the sky. Meeting a constant tints its region; understanding the rooms
in that region fills it in. The Visual Era unlock rides this progression
(historical order), so the sky and the whole app age up together, the era grain
and the meta-map advancing as one.

## Per-player layout

Because edges appear in discovery order, no two players' skies are identical.
Your map is a portrait of how *you* came to understand mathematics, not a
standardized completion grid. This is the single property that makes the meta
feel like a universe you explored rather than a form you filled, and it is why
the redesign is worth the build cost.

## The daily route plays on this board

The Daily Traverse (the run structure) is literally a path drawn across your
current sky, preferring edges that lead from a lit node to an adjacent dark one,
so it takes you to the frontier of your own understanding. One seeded run per
day threads a handful of rooms along real edges; your elegance at each stop
(from `CONSTRUCTIONS.md`) earns a charge you spend on the next as a lens (an
epistemic power: reveal a spectrum, mirror a symmetry, relax a par), never a
statistical buff. It ends; tomorrow's seed is a different route. The meta-map and
the daily loop are therefore the same object seen at two speeds, which is the
composition that makes both exceptional instead of parallel.

Bounding the run to one per day borrows Wordle's discipline: the pause between
plays is what builds a healthy craving rather than a compulsion. The lenses are
knowledge arriving early (the existing boon rule), not stat buffs, and the run
cannot be farmed, which is what keeps it clear of the treadmill.

## The bottom is real

The Layer-4 lore payoff sits at the deepest cluster: a region that only lights
when enough of the sky connects, the Strange-Loop insight-chain terminating in
the discoverable endpoint. It is the one true gate in the whole map (everything
else reveals rather than blocks), and it must be designed now so the deepest
diggers arrive somewhere worthy.

## Resolving the open questions in PROGRESSION.md

1. **How explicit should progression be?** Visible but earned: the sky is
   beautiful and openable by anyone, but shows only what you have discovered, so
   it is never a cluttered checklist.
2. **How hard to gate?** Reveal, do not block, to protect the Wanderer; only the
   Layer-4 bottom is truly gated.
3. **Shared vs bespoke elegance metric?** Bespoke per room, unified only by
   presentation (see `CONSTRUCTIONS.md`).

## Rendering bar

A janky web undercuts the thesis worse than no web. Ship it incrementally (the
data model and edge events first, then the animated sky), gated by the same
beauty QA every room faces, and rendered through the glow pipeline in
`SYNESTHESIA.md` so the sky is lit, not flat.

## Sources

- Outer Wilds knowledge-gating and Rumor Mode:
  https://steamcommunity.com/sharedfiles/filedetails/?id=2160452487
- Wordle one-per-day habit psychology:
  https://uxmag.com/articles/the-fascinating-psychology-tricks-that-make-wordle-so-addictive
