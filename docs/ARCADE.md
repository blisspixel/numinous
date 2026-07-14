# The Arcade build: hands on the math

The founder's directive, designed to be built. Two halves, one principle:
every screen answers your hands, and the fun ramps under pressure.

## Half one: Munch becomes an arcade game

Legal ground (settled): game mechanics are not protectable; names, characters,
and art are. Ours: the game stays **Munch**, the player character is **the
Muncher** (our own design: a small bright ring with a bite taken out, the
accent color of hunger), and the enemies are **Vexations**, the Order's lesser
spirits, deadpan wrong answers given legs. MECC's names and Troggles stay
theirs; nothing is borrowed.

### Core (`crates/core/src/munch_arcade.rs`), pure and tested
- State: the existing `Board` (numbers + rule), muncher cell, `Vec<Vexation>`
  (cell + kind), eaten set, lives (3), level number.
- Turn discipline (deterministic, testable, MCP-replayable): the player acts
  (move one cell or eat), then every Vexation steps. No wall clock in core.
- Vexation kinds, one behavior each, all seeded:
  - **Drifter**: random legal step (SplitMix64).
  - **Tracker**: steps to reduce Manhattan distance; ties broken by seed.
  - **Editor**: does not chase; REWRITES the number it stands on (new seeded
    value), so the board decays if you stall. The anti-camping spirit.
- Contact = lose a life, muncher respawns at a corner, Vexations scatter.
  Zero lives ends the run. Clearing all fits advances the level: one more
  Vexation, deeper rule band (the existing ramp), fresh board.
- Score: existing munch scoring plus a per-level clear bonus times level.
- Tests: pursuit reduces distance; drifters stay legal; editors change only
  their cell; respawn scatters; a scripted full level clears; determinism.

### App (real time, the fun half)
- WASD moves cell to cell (repeat-rate limited), Space eats, Vexations step
  on a beat (every N frames, N shrinks per level).
- Juice: bite flash on eat, screen nudge on a wrong bite, Vexation contact
  flashes red and thins the lives row, per-level chiptune tempo up.
- The arrival line names the verb: "WASD: RUN. SPACE: EAT. DON'T BE CAUGHT."

### CLI (turn-based twin)
- Same core, board redrawn per turn, moves `w a s d` and `e`. Slower, same
  math, same scores table (`arcade seed:N`), daily-able.

### MCP (parity from day one)
- Stateless replay like nim: pass the full action list; Vexation steps are
  deterministic, so the same actions give the same run. Tool: `munch_arcade`;
  replayed action lists post to the shared score table as `arcade seed:N`.

## Half two: the poke (every room answers)

- `Room` trait gains `verb() -> Option<&'static str>` (the arrival card's
  action line) and `poke(x, y, variation)` where meaningful; the registry
  threads a per-visit `variation` seed (default 0 pins all current tests
  and postcards exactly).
- Expanded pokes and drags (touch verbs on arrival cards for playable rooms): first wave (Lorenz: DROP A STORM, Life: LAUNCH A GLIDER, Voronoi: DROP A WELL, Double: RE-DROP from the hand's point, Chaos: MOVE A CORNER, Random: PLANT A WALKER) plus many more including Golden (PLANT A SEED), Langton (FLIP A CELL), Barnsley (PLANT A NEW POINT), Buffon (THROW A NEEDLE), Galton (DROP A BALL: x chooses the lane, y tilts its coin), Mandelbrot (DIVE), Julia (MORPH C), Times Tables (TURN THE DIAL), Epicycles (PERTURB), Goldbach (TEST THIS EVEN: x chooses an even target, y chooses a prime-pair witness), L-System (PLANT), Quine (PLACE A COPY), StrangeLoop (SHIFT), and Cult of Pi (REPAIR THE SIGNAL). 31 total: the whole catalog.
- The app maps clicks to normalized coordinates; R resets the current visit,
  while moving to another room deals the next replayable variation.
  The CLI gets `watch --vary`; MCP `play_room` gains `variation`.
- MCP `munch_arcade` tool added for full parity (stateless action-list replay + state + score posting through progress).

## Order of work (one session each, built to the bar)
1. `munch_arcade` core + CLI twin + tests (the game exists end to end).
2. App real-time Munch with juice (the fun lands).
3. The poke trait + first six rooms + arrival verbs (expanded to all 31 rooms).
4. MCP `munch_arcade` + variation parity and docs. **DONE** (MCP tool + `play_room` variation + score posting).
5. Run the documented human playtest, including a younger participant, before claiming the experience passes for either audience. **OPEN**.

Every law holds: rewards stay knowledge, `?` explains the concept (chase
adds: greedy pursuit IS gradient descent; you are outrunning an optimizer),
no scolding, no toll, determinism everywhere a seed can reach.
