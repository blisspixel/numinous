# The panel

A working review: five minds around a table with the build as it stood at
that session (81 commits, 27 rooms, 7 games on 4 faces of play, the full RPG
spine, 22 MCP tools; the catalog has since grown, see `ROOMS.md`). One seat is real: the AGI seat quotes an actual cold-start AI
playtest (July 2026, LV 13 session). The others are composed from their
doors' design goals. The point is what we are missing, not what we have.

## The seats

**Maya, 11.** Plays Minecraft and Mario Kart. Reads when she wants to.
**Dr. Okafor.** Retired number theorist; taught for thirty years.
**Jules.** Designer and stoner creative; owns a CRT for the vibes.
**The agent.** An AI that actually played a full session and filed a report.
**The chair.** A game designer keeping score.

## What each seat said

### Maya (the kid door)
- "The menu says PLAY first, good. But I tried to CLICK the munch board and
  nothing happened." **Games are keyboard-only; a kid's first instinct is
  the mouse.** Miss.
- "When I eat a number nothing happens until the end." **The window games
  grade at the end; there is no bite-by-bite juice**, no flash, no crunch
  sound, no shake on a wrong bite. Munch in 1990 crunched.
- "Can I keep the picture?" **P now saves a PNG postcard in the app.** The
  remaining share gap is polish: loops, links, and a discoverable share flow.

### Dr. Okafor (the PhD door)
- "The deep cuts are genuinely good. Now cite them: every room should carry
  one pointer to a real text or paper. Further reading is loot too."
- "Where are the open problems? Collatz says the frontier exists. Give it a
  wing: twin primes, Goldbach, Moving Sofa, drawn as playthings that say
  'nobody knows, you could be first'." **An Open Problems wing.**
- "The sound is thinner than the mathematics." Agreed below.

### Jules (the vibes door)
- "The eras are skins; they need the grain: scanlines and bloom on phosphor,
  dither on 8-bit. Two shaders' worth of love."
- "The Show should crossfade rooms, not cut. And where is the visualizer?
  You promised Winamp." **Still the roadmap's biggest unbuilt promise.**
- "Couch mode: gamepad plus fullscreen tour equals the best screensaver ever
  shipped." (gilrs; already queued.)

### The agent (the AI door, real data)
- Verdict, verbatim: "yes, I would play again... correct answers pay out in
  true, well-written facts, failures pay out in named mistakes, and the map
  visibly has edges beyond the catalog. The number was never the point, and
  unusually for a game, this one means it."
- "Munch became bookkeeping after round three": **the rules repeat and are
  trivially machine-checkable. Rule variety is a depth fix for every mind:**
  digit sums, powers, one-away-from-prime, composites with exactly three
  divisors.
- "Aliens got real when the base changed. More of that, earlier."
- "`listen_room` disappointed everywhere": **the sound layer is one drone
  note deep. Rooms should sing motifs (Engine A2), not tones.**
- XP accrues fastest through the least interesting play (idle-looping
  play_room). Not a casino, but the grind path exists; consider capping
  visit sparks per room.
- The reward-parity findings from this seat were fixed the same day (deep
  cuts, whispers, nim scores, dailies over MCP).

### The chair (synthesis)
The spine is done and the doors all open. What is missing is **depth where
the hands touch**: the moment-to-moment feel (juice, mouse, sound) and the
long game (rule variety, open problems, further reading). Nothing structural
is absent anymore; everything on this list makes an existing organ stronger.

## The list, in order

> Progress: items 1 (first serving), 2, 3, 5, and the phosphor half of 8 are
> built. Open Problems opened (Goldbach); Engine B v0 shipped (the dial, the
> fetch pipeline, Y in the app). Motifs (4), citations (7), full Share v1,
> crossfade, the visualizer, gamepad, and the spark cap remain.


1. **Juice in the window games**: per-action feedback (flash on eat, shake
   on bad bite, a tick sound per action from the chiptune voices).
2. **Mouse support**: click a munch cell, click a quiz choice, click stones.
3. **Munch rule variety** (core, seeded by round depth) and an **aliens base
   ramp** (their base drifts from 10 earlier).
4. **Engine A2, room motifs**: every room's sound becomes a short chiptune
   phrase in its own key; `listen_room` returns real notation.
5. **Save-postcard key** in the app (P writes the live room frame to a PNG,
   preserving pokes and the selected Visual Era).
6. **Open Problems wing** (Collatz has friends: twin primes, Goldbach).
7. **Further reading**: one citation per room, unlocked with its deep cut.
8. **Era grain** (scanlines, bloom, dither), **Show crossfade**, then the
   **music visualizer** and **gamepad** as already queued.
9. **Visit-spark cap** per room (anti-grind; play stays the fast path).

Everything above holds the standing laws: rewards stay earned, math is never
the toll, no scolding, no casino, no prison.
