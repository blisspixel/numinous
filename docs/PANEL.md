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
  nothing happened." **Resolved for machine path:** every window game now
  accepts left-click (Munch, Quiz, Nim, Arcade, Gauntlet stages).
- "When I eat a number nothing happens until the end." **Machine juice path:**
  Munch flashes the toggled cell and plays a soft crunch one-shot on each
  bite. Wrong-bite shake after grading remains open.
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
  shipped." The first standard-controller path now ships through `gilrs`, with
  a virtual hand, room travel, time control, and all current games. Remapping,
  platform certification, and adaptive button glyphs remain.

### The agent (the AI door, real data)
- Verdict, verbatim: "yes, I would play again... correct answers pay out in
  true, well-written facts, failures pay out in named mistakes, and the map
  visibly has edges beyond the catalog. The number was never the point, and
  unusually for a game, this one means it."
- "Munch became bookkeeping after round three": **the original shallow app
  loop repeated one opening board.** The standalone game now starts in the full
  seeded rule deck, advances continuously, and avoids adjacent repeats from the
  same rule family. Target-density balancing now prefers a playable fit band
  (about 3 to two-thirds of cells) with deterministic re-rolls; score formulas
  stay unchanged so existing seeds remain comparable when the board settles.
- "Aliens got real when the base changed. More of that, earlier."
- "`listen_room` disappointed everywhere": **resolved in Engine A2.** Every
  catalog room now exposes a structured phrase, and automatic room beds use a
  softer triangle voice at a conservative default level.
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

> Progress: items 1, 2, 3, 4, 5, 7 (citations table + deep-cut unlock + CLI/MCP
> parity), 8 (era grain including phosphor bloom + Show crossfade + spectrum
> substrate plus output-mix/loopback meters), 9 (play/win spark soft caps),
> controller input, remapping, and adaptive face glyphs (Xbox / PlayStation /
> generic) across room chrome and every window game HUD are built. Aliens base
> ramp softens earlier for denser seeds. Room-bed spectrum meter and MCP
> spectrum bands ship as the offline visualizer path. Open Problems opened
> (Goldbach); Engine B v0 shipped. Output-mix and optional OS loopback spectrum
> sources ship (key O). Spectrum levers now soft-drive room time scale, phase
> nudge, and beat pokes on output-mix/loopback. Share sidecars and CLI share
> bundles (postcard + loop + README) ship; GIF/MP4 and physical pad certification
> remain. Munch generator prefers a playable target-density band without
> rewriting score identity.


1. **Juice in the window games**: per-action feedback (flash on eat, shake
   on bad bite, a tick sound per action from the chiptune voices).
2. **Mouse support**: click a munch cell, click a quiz choice, click stones.
3. **Munch rule variety** (core, seeded by round depth, with the standalone app
   now starting in the full deck and avoiding adjacent rule-family repeats) and
   an **aliens base ramp** (their base drifts from 10 earlier).
4. **Engine A2, room motifs**: every room's sound becomes a short chiptune
   phrase in its own key; `listen_room` returns real notation.
5. **Save-postcard key** in the app (P writes the live room frame to a PNG,
   preserving pokes and the selected Visual Era).
6. **Open Problems wing** (Collatz has friends: twin primes, Goldbach).
7. **Further reading**: one citation per room, unlocked with its deep cut.
8. **Era grain** (scanlines, bloom, dither), **Show crossfade**, then the
   **music visualizer**. Standard-controller play is built; remapping, adaptive
   glyphs, and cross-platform hardware certification remain.
9. **Visit-spark cap** per room (anti-grind; play stays the fast path).

Everything above holds the standing laws: rewards stay earned, math is never
the toll, no scolding, no casino, no prison.
