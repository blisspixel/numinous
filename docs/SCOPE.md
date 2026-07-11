# Scope: the definition of no

The hardest discipline for this project, and the one an external review (July
2026) graded lowest. The design is unusually coherent around one thesis, and
the risk is no longer "not enough ideas," it is whether every subsystem serves
the thesis or slowly buries it. This doc is the filter that says no. It sits
above the roadmap: `ROADMAP.md` sequences what we build, this decides what earns
a place at all.

## The thesis, stated so it can be defended

> Numinous is not about teaching mathematics. It is about making mathematics
> feel inexhaustibly alive.

Everything is measured against that. "Teach" is a side effect we are proud of;
it is never the goal, and the moment a feature optimizes for instruction over
aliveness, it is working against us (see `PEDAGOGY.md`, the fluency-illusion
risk, for why felt aliveness and real understanding are the same target
approached from the awe side).

## Three products, not equal

The review named something true: we are, if we are not careful, building three
products at once. They are not equally important, and naming the hierarchy is
what keeps the build honest.

1. **The playable mathematical instrument (the rooms).** This is *the thing*.
   Everything else exists to make someone spend one more minute inside it, never
   the reverse. The nearest kin are not games; they are Ableton, Blender,
   Desmos, TouchDesigner, the OP-1, Dreams. You never "beat" a piano; you become
   more expressive. That is where Numinous lives.
2. **The Studio (the multiplier).** How experts stay forever and how the ceiling
   goes to infinity. Excellent, and subordinate to product 1: it is the room
   authoring itself (see `STUDIO.md`, `CREATOR.md`).
3. **Progression (levels, XP, trophies, gauntlets, ranks, dailies, lore).** Kept
   deliberately subordinate. It is built and the founder wanted it, so the move
   is not to delete it; the move is to hold a hard line that it never outshines
   the math, and to convert progression-shaped systems into discovery-shaped
   ones wherever possible (the Constellation redesign in `CONSTELLATION.md` is
   exactly this: a checklist became a map you explore).

## The daily test

Ask this of every mechanic, every day:

> If I removed this tomorrow, would the player experience **more math** or
> **more progression**?

When progression wins, cut it. This is the operational form of the anti-checklist
invariant in `NORTH_STAR.md` ("nothing counts as learned or won without an act
of generation"), pointed at the whole feature set rather than one loop.

## The justification filter

A feature earns a place only by answering yes to at least one, honestly:

- Does it create more **awe**?
- More **agency** (the player acts on the math, not around it)?
- More **beauty**?
- More **mastery** (a real skill ceiling, see below)?
- More **surprise**?

If it is justified only by "it is a game convention," or "it aids retention," or
"it would make a good benchmark," the answer is no. Retention is downstream of
awe, not a thing to engineer directly.

## Mastery is math's, not XP's

People love mastery, and mathematics has mastery built in. The design must not
replace it with a level number. The test is not "can the player understand this
room" but "can the player get **good** at it": improvise, perform, develop a
recognizable style. "My best Lorenz solo" should be a coherent, shareable thing.
This is where the elegance-and-performance layer in `CONSTRUCTIONS.md` earns its
place, and it is the honest, uncapped alternative to an XP treadmill.

## The unit of growth is the moment

What becomes legendary is not the app, the Studio, or the catalog. It is the
five-second moment: someone sees something mathematically impossible, stops,
watches, and sends it to a friend. Every room should hold dozens of those. Design
for the clip. "Shareable by design" (`VISION.md`) is not a distribution feature
bolted on at the end; it is a per-room quality bar (see the beauty-QA and Show
work in `SYNESTHESIA.md`).

## The fan-out docs are a menu, not a build list

The July 2026 research fan-out (`NORTH_STAR.md` and its lane docs, the Awe Engine
wave in `ROOMS.md`) produced a large menu of possibilities. **Writing a planning
doc is not a commitment to build the thing.** A doc's job is to hold a good idea
at rest so a future decision can pick it up or leave it. The discipline is to
build few of them, deeply, and let the rest wait. When in doubt, ship one room
to jaw-dropping quality rather than five to good.

## Architecture must not become identity

The engineering rigor (Rust, wgpu, the coverage gate, CI, MCP) is real and worth
keeping, but it is invisible to the player, who experiences only latency, beauty,
feel, and clarity. A green gate at high coverage is not evidence of product
excellence; it is evidence the code does what it says. The cautionary example is
live: we held 90-plus percent coverage while the signature glow pipeline was not
built at all (`SYNESTHESIA.md`). So the player-experienced qualities get
first-class standing alongside the code gates, and "it passes the gate" never
substitutes for "it feels alive."

## Beloved, not indispensable

Most ambitious software tries to become indispensable: habitual use because it
solves a problem. Numinous aims at something rarer, to be **beloved**: repeated
use because people want to return to the experience. Every decision should favor
the latter, even when it means saying no to an otherwise clever feature. This is
the same instinct as "would this make a math nerd grab their friend's shoulder"
(`VISION.md`), stated as a competitive posture.
