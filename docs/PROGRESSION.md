# Progression: Levels & Insights

How Numinous is structured as a journey, without ever feeling like a course, a grind, or a game you can lose. This is the "levels and insights" plan.

## The core model: knowledge is the only progression

The most important design decision in this document: Numinous is a **metroidbrainia**, a knowledge-gated experience in the lineage of *Outer Wilds*, *The Witness*, *Tunic*, and *Fez*. In those games, you do not collect items or levels; you collect **understanding**, and understanding is the only thing that actually moves you forward. A player who already knows everything could beat them on a fresh save in minutes.

Numinous adapts this to mathematics, where it fits better than in any other subject, because **math is literally a web of connected insights**. The thing you unlock is not a key or a door. It is a realization. And the realization is real: you actually understand something you did not before.

This gives us a progression system that is:
- **Invisible and optional.** Like Outer Wilds' best gates, you often will not even notice a gate until an insight recontextualizes something you already saw. Nothing is ever locked with a visible "you need X" barrier.
- **Impossible to grind.** There is no XP, no currency you farm. You cannot buy your way forward. You either see it or you do not, yet.
- **Honest.** The progression is your own mind. That is the whole product's thesis made into a mechanic.

## Two players, one design

Every progression decision serves two people at once, and must never sacrifice one for the other:

- **The Wanderer** just wants to play beautiful toys and leave it running. They never track progress, never chase a Constant, never read a Reveal. For them, Numinous is a perfect audiovisual instrument with no progression at all. This experience must be complete.
- **The Seeker** notices the threads and pulls them. For them, Numinous is a vast, connected constellation of insights with a real bottom. This experience must be deep.

The trick, borrowed straight from the metroidbrainia genre: **the progression is a layer running alongside the toy, never in front of it.** The Wanderer never trips over it. The Seeker can always find the next thread.

## The four scales of progression

Progression happens at four nested scales, from seconds to weeks.

### 1. Within a moment (the Toy): cause and effect
The tightest loop. You turn a dial, something beautiful and audible happens, you understand the mapping a little better. This is flow (research shows a clear action plus immediate feedback is the flow condition, and generative-music systems measurably increase it). No "progress" is tracked here; the reward is the beauty itself. This loop must carry the entire product even if a player never goes deeper.

### 2. Within a room (the three layers): Toy then Puzzle then Revelation
Each room has its own micro-arc (see `DESIGN.md`):
- **Toy** (mandatory, wordless): play freely.
- **Puzzle** (optional): a concrete challenge that supplies flow-channel difficulty on demand. These are the closest thing to "levels" in the traditional sense, and they use **elegance scoring** in the spirit of *Euclidea* and Zachtronics: not just "did you solve it" but "how cleanly." Tiered goals (a first solution, then a minimal / most-elegant one) let a player choose their own difficulty. Never blocking.
- **Revelation** (optional): the insight. Reaching it is the room's true completion, and it collects a **Constant**.

### 3. Across rooms (the Constellation): insight-gating
This is the heart of the metroidbrainia design, and the answer to "how do rooms connect into a journey."

**Insights are keys.** Understanding something in one room opens something in another, because in mathematics the ideas actually connect. Examples of real insight-gates we can build:
- Understanding **Fourier epicycles** (that any shape is a sum of circles) unlocks a deeper layer in the **Additive Synth** room (that any sound is a sum of tones), because they are the *same theorem*. The connection is the key.
- Grasping **self-similarity** in the Chaos Game recontextualizes the **Mandelbrot** boundary, the **Koch** garden, and the **cardioid** in Times Tables. One insight lights up four rooms at once.
- Meeting **i** (the impossible number) in one room opens the doors that only i can open elsewhere (rotations, the complex plane, the Mandelbrot iteration).

**The Constellation Map** is the progression HUD and the Codex in one: a dark star-field where each room is a node and each *insight-connection* is an edge. Edges are invisible until you discover the connection, then they light up, physically drawing the web of mathematics as you understand more of it. Watching the constellation fill in *is* the sense of progress, and it is beautiful, so even the Wanderer might open it once and get pulled in. This map is the single best expression of the whole product's thesis: it is not a tech tree, it is the actual structure of math revealing itself.

### 4. Across the whole (the meta): Constants and Eras
The long arc (see `DESIGN.md` and `LORE.md`):
- Reaching Revelations collects **Constants** (π, e, φ, i, ℵ₀, γ, ...). Constants are the pantheon (see `LORE.md`), and they gate softly: some deep rooms and the Terminal's deeper answers open only once you have met the right Constant.
- Constants unlock the **Visual Eras** in historical order (teletype to modern), so the app itself visibly "ages up" as you go deeper, another form of felt progress that is purely aesthetic and never blocks anything.
- The bottom of the whole thing is the **Layer-4 lore payoff** (see `LORE.md`): a real, discoverable endpoint for the Seeker who maps the full constellation. It must be designed before the deep trails ship, so the journey actually arrives somewhere.

## Onboarding: the first ninety seconds

No tutorial, ever. Onboarding is a designed *first room*, not a lesson.
- The app opens (after first launch) directly into a single, wordless, foolproof room, the recommended choice is **Times Tables**, because one dial produces instant, morphing, audible beauty with zero explanation.
- The only affordance is the dial, and it visually invites dragging. Within seconds the player has caused beauty and heard it. That is the entire tutorial.
- The Cabinet is revealed only after the first "whoa," so the first impression is a phenomenon, not a menu.
- Everything else (Reveal, Share, Eras, the Constellation, the radio) is discovered, never pushed.

## The difficulty curve (for the parts that have one)

Only the optional Puzzle layer and the deeper insight-gates have difficulty, and it is opt-in. The curve:
- **Front-load wow, not difficulty.** The launch rooms are ordered by wow-to-effort (see `ROADMAP.md` and `ROOMS.md`); the first things a player touches are the most immediately stunning and the least demanding.
- **Difficulty lives in depth, not in walls.** A room gets "harder" only if you choose to chase its elegant-solution Puzzle tier or a subtle insight-gate. The Toy stays effortless forever.
- **The Seeker sets their own pace.** Because gates are knowledge, not skill checks, there is no difficulty spike you can hit and get stuck behind. You just have not connected it *yet*, and there is always another thread to pull in the meantime.

## Session shapes (design for all of them)

- **The 90-second hit:** open, touch one room, feel awe, close. Must be complete and satisfying.
- **The 20-minute flow:** wander three or four rooms, chase one Puzzle, hit one Revelation. The core loop.
- **The hour-long dive:** a Seeker follows a constellation thread across many rooms, filling in the map.
- **The leave-it-running:** Watch mode, a playlist, the radio on. Hours of ambient beauty, zero interaction.
- **The performance:** the Studio and Watch mode as a live audiovisual instrument for an audience.

## Anti-patterns (progression edition)

- No XP, no currency, no grind, no daily streaks, no "energy," no lives.
- No fail state anywhere in the Toy. You cannot lose Numinous.
- No visible "locked, you need X" walls. Gates are invisible until an insight opens them.
- No forced tutorial, no forced text, no forced order. The Cabinet is open from the start.
- No progression the Wanderer can trip over, and no shallow bottom the Seeker can hit and feel cheated by.
- Progress is understanding. If we ever find ourselves rewarding time-spent instead of insight-gained, we have broken the thesis.

## Open questions
1. How explicit should the Constellation Map be? Fully hidden (Outer Wilds style, you hold it in your head) versus a visible filling-in web. Leaning visible-but-uncluttered, because the web itself is beautiful and on-thesis.
2. How hard should insight-gates gate? Truly block deeper content, or only *reveal* it as a bonus? Leaning reveal, to protect the Wanderer, with only the Layer-4 endgame genuinely gated.
3. Elegance-scoring for Puzzles: per-room bespoke metrics, or a shared "fewest moves / simplest expression" currency across all rooms?

## The ruling (July 2026): ceremony, not economy

The build carries XP, levels, trophies, boons, streaks, and scores; this doc
carries knowledge-only progression. Both are kept, under one explicit rule:

**XP records contact with beauty; insight records progress.**

The RPG layer is a local journal and celebration layer. It may decorate,
remember, and pace. It may never gate core play, optimize behavior, punish
absence, or reward idling faster than playing. Where those conflict, the
knowledge layer wins. (Adopted from the July 2026 external review; see
`docs/REVIEW.md`.)
