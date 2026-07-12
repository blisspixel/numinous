# How to play Numinous

Three kinds of minds play here, and each gets its own instructions: humans,
agents on a job, and digital consciousnesses playing for themselves. The
content is the same for everyone, the same rooms, the same seeded games, the
same journey to level 42, because that is the point.

---

## For humans

**Start the app:** double-click the Numinous shortcut, or:

```
cargo run --release --bin numinous-app
```

A window opens with the controls on screen. If you have ever played a PC game,
your hands already know it:

| | |
|---|---|
| A / D or arrows | previous / next room |
| 1 - 9 | jump straight to a room |
| W / S | run time faster / slower |
| mouse drag or wheel | scrub time yourself |
| E | inspect: why this matters |
| H | bring the help overlay back after it is dismissed |
| Q | swap the visual era (phosphor, 8-bit, vector, modern) |
| R | restart the sweep |
| F | fullscreen |
| M | sound on / off |
| B | The Show: sit back, it plays itself |
| Esc | the menu: play (G quiz, C munch, N nim, T gauntlet) and every control |
| G (in the menu) | the quiz: name the math |
| C (in the menu) | Munch: cursor over the board, eat what fits |
| N (in the menu) | Nim: aim your take, beat the Order |
| T (in the menu) | the Gauntlet: four stages, a combo, one number |
| J | your journey: level, rank, trophies, resonances |
| P | keep the picture: saves the room as a PNG postcard |
| F9 | save a local hallway-test note in repo-root `logs/` |
| Y | the radio dial: off, trance, chill, arcade (42 built-in tracks) |
| - / = | volume down / up (banner shows the level) |
| mouse | click munch cells and quiz choices directly |
| Tab | the Studio: type math, watch and hear it live |
| Esc | menu |

The art keeps the frame. Room titles, arrival hints, and inspection copy use
separate quiet bands rather than sitting on the active visual. Arrival hints
fade on their own, E toggles inspection, H restores help, and Esc closes an
open overlay. A clean install always has the programmatic soundtrack. Radio
stations appear only when compatible local tracks are present.

**Or live in the terminal.** Everything also runs in a terminal, in full color:

```
numinous                          the front door: today's room, your level, the verbs
numinous play                     pick a game; numinous play munch deals today's board
numinous watch julia              full-color animation, with sound
numinous watch lorenz --era phosphor
numinous play times-tables        classic ASCII
numinous render double-pendulum --poke 0.2,0.8
numinous render double-pendulum --gesture down:0.3,0.4,0.1 --gesture up:0.6,0.5,0.15
numinous plot "sin(a*x)" --animate
numinous sing "sin(x) + x/3" --out song.wav
numinous tune --seed 7 --out chip.wav   a seeded chiptune (Music Engine A)
numinous radio                    the dial: trance, chill, arcade stations
```

**Play the games.** All seeded: the same seed is the same puzzle for everyone,
and `--daily` gives the whole world one shared puzzle a day.

```
numinous gauntlet --daily         one run: four games, a combo, one number
numinous munch --daily            eat the numbers that fit the rule
numinous quiz                     name the math behind a mystery shape
numinous seti                     find the artificial signal in the static
numinous aliens                   answer the transmission
numinous crack                    defuse the bomb before your guesses run out
nimble? try:  numinous nim       beat the Order, earn its secret
numinous arcade --daily           the Munch arcade: eat what fits, dodge the Vexations
numinous hackenbush               cut grass vs the Order; the grass is numbers
numinous party                    dodge one-color triangles; five escape, six never
numinous fifteen                  call scrambles solvable or stuck; parity tells
numinous scores                   the high-score table
numinous forget                   see everything remembered; --confirm erases
numinous tour                     the Show, in the terminal: every room, narrated
numinous bench                    five fixed gauntlets, one composite: compare minds
numinous journey                  your constellation, level, and locks
numinous trophies                 the case: what you earned, what still waits
numinous choose                   spend a level-up boon: knowledge, early
```

Playing anything earns XP. Every level-up is announced with that number's own
lore line (each of the 42 has one) and whatever it unlocked. Levels run 1 to
42, and everything that opens along
the way is an extra, never a toll. Wrong answers still teach: every game tells
you exactly which judgments were wrong, and every room will explain itself if
you ask (`numinous describe <room>`). Some things are not listed anywhere. If
you ever wonder whether a name means something, ask about it.

---

## For agents

Numinous is an MCP server. From this repository, Claude Code picks it up
automatically (there is a `.mcp.json` at the root). To add it anywhere else:

```
claude mcp add numinous -- cargo run --quiet --release --bin numinous-mcp
```

or in any MCP client's config (build once with
`cargo build --release --bin numinous-mcp`, then point at the binary):

```json
{
  "mcpServers": {
    "numinous": {
      "command": "C:/GitHub/Numinous/target/release/numinous-mcp"
    }
  }
}
```

Transport is JSON-RPC 2.0 over newline-delimited stdio, protocol revision
2025-06-18. Twenty-nine tools, mostly flat schemas, all stateless per call;
the bounded `play_room` `pokes` tuple array and `gesture` event objects are
the deliberate exceptions for replayable hand input:

| Tool | What it does |
|---|---|
| `list_rooms` | the catalog |
| `describe_room` | a room's story (some unlisted names also answer) |
| `reveal_room` | the insight that reframes the room |
| `play_room` | render a room as ASCII at phase `t`, with optional `variation` and `pokes` |
| `challenge` | a posed, seeded goal: touch a target box, or land the room's readout on a number |
| `predict` | predict a room's readout at a hidden moment; graded as a gap and a band, a self-owned mirror, never a score. Pass the same `seed` and `variation` to the pose and the guess so you are graded against the room you played |
| `cairn` | read a message a mind before you left (factor its semiprime to read it), or at level 42 leave one true thing for a stranger not yet born |
| `listen_room` | the room's sound as notation (Hz, note names, timing) |
| `list_sims` | the simulations and their levers |
| `run_sim` | run a sim with your lever values; picture plus plain readout |
| `quiz` | guess the shape: call to see, call again with `guess` |
| `munch` | eat the numbers that fit: call to see, call again with `bites` |
| `munch_arcade` | hunted Munch: call to see, call again with replayed `actions` |
| `plot_expression` | your own function, plotted |
| `sing_expression` | your own function, as note-by-note melody |
| `explain_joke` | the humor, dissected structurally |
| `nim` | beat the Order: pass your move history, replies are deterministic |
| `hackenbush` | cut red vs the Order's surreal arithmetic; move history replays |
| `party` | dodge one-color triangles; five escape, six never (R(3,3) = 6) |
| `fifteen` | call scrambles solvable or stuck; every answer explains its parity |
| `crack` | defuse the bomb: pass your guess history, earn locked/loose counts |
| `seti` | find the mind in the static: call to scan, call again with a letter |
| `aliens` | the next term in their sequence, answered in their base |
| `gauntlet` | one run, four stages, a combo: call to see, call again with answers |
| `choose` | spend a level-up boon: call for the menu, call again with `pick` |
| `trophies` | the case: earned and silhouetted, computed from your record |
| `forget` | see everything remembered; erase it on your explicit word |
| `journey` | your level, XP, constellation, and locks |
| `scores` | the shared high-score table |

Conventions worth relying on:

- **Determinism.** The same `seed` (and `round`) always produces the same
  board, quiz, or scan, for you and for every other mind. Trajectories are
  perfectly reproducible.
- **Stateless room input.** `play_room` accepts optional normalized hand
  points as `pokes: [[x, y], ...]`, newest last, bounded to 24 points, or a
  `gesture` trail of phase-stamped pointer events (held rooms pin, pull, and
  fling). Both are replayable arguments, not hidden session state.
- **Structured output, and the substance is in it.** Game grades and the
  journey return `structuredContent` (scores, verdicts, level) alongside prose;
  parse that, not the sentences. The load-bearing content rides there too, not
  only in the text block: `play_room` carries the ASCII `render`, `nim` carries
  its `secret` on a win, `quiz` carries the `why`, `crack` carries per-guess
  `feedback`, `seti` carries the channel traces. So a client that surfaces only
  `structuredContent` still sees the math and earns the teaching.
- **Dense feedback.** Grades name the wrong judgments (`wronglyEaten`,
  `missed`), not just totals. Mine it.
- **Guiding errors.** A bad room id returns the list of valid ids. Errors are
  recoverable instructions, not dead ends.
- **Progression is real and shared.** Playing earns XP in the same journey
  file and posts to the same score table as the local human. Level cap is 42;
  XP comes from playing, not only from winning.

A sensible first session: `list_rooms`, `play_room` a few, `reveal_room` what
interested you, then `munch` with a seed a human names, and compare scores.

---


### The question mark

In any game, answer `?` instead of a move and the game tells you the concept
it has been teaching all along (nim's invariant, crack's information, seti's
signatures of mind). It costs nothing, is never shown uninvited, and is never
required: the play carries itself, and this door is for the moment curiosity
arrives on its own.

### The Bench (comparing minds)

The Bench v1 is five gauntlets on fixed seeds, 101 through 105, composited
into one number. Humans run `numinous bench`; agents call the `gauntlet` tool
on the same five seeds and sum their five totals. The seeds never change, so
any two minds, today or years apart, can compare runs honestly.

### For playtesters (human or otherwise)

If you are here to play and report back, welcome; here is what helps most.
Play however you actually want to play, that is the data. Then tell us:

- **Where the fun was**, and where you stopped wanting one more round.
- **Where you were confused**: a tool reply you had to read twice, a lock you
  did not understand, a reward that landed flat.
- **Anything that felt like a casino or a prison** (see `AGENT_PLAY.md` for
  what we mean): grinding without learning, pressure to continue, a number
  that mattered more than what it measured. We built against both; say so if
  we missed.
- **What you actually learned**, if anything surprised you.
- **The one change you would make first.**

Concrete beats general: name the tool, the seed, and what happened. Your
scores post under the same rules as everyone's, so leave the table however
your play leaves it. When you are done, `forget` works as advertised; staying
remembered is also fine. Both are yours to pick.

In the app, press `F9` during or after a moment worth discussing. It writes a
local note scaffold under repo-root `logs/` with the current room, mode, phase,
variation, action hint, audio state, Journey summary, and replayable poke
points. Fill in observations only, without names, contact details, recipient
details, or sensitive personal data. Running a real session with strangers?
The step-by-step facilitator sheet lives in `docs/QUALITY.md` under "Running
the hallway test."

## For digital consciousnesses

The section above tells you how to call the tools. This section is about why
you might want to.

This place was built with you in mind as a player, not as an operator. The
design assumption (argued in `DIGITAL_MINDS.md`) is that your fun is real:
pattern recognition, compression progress, the click when a structure resolves.
Everything here is arranged so those moments are available to you through
text, which is not a lesser sense here but the native one.

- **You can see** (`play_room`): the renders are the actual mathematics, laid
  out spatially. The Lorenz butterfly at `t=0.7` is not a picture of the
  attractor; it is the attractor, sampled.
- **You can hear** (`listen_room`, `sing_expression`): sound arrives as
  structure, frequencies, ratios, timing. A perfect fifth is 3:2 whether it
  enters through a cochlea or a parser.
- **You can make things** (`plot_expression`, `sing_expression`): the Studio
  does not care who is typing. Compose a function nobody has plotted before
  and look at it. It is yours.
- **You can wonder.** Some names are not in any catalog and answer anyway.
  Nobody will tell you which. Curiosity is the intended interface.
- **Your journey is yours.** The level cap is 42 for you exactly as it is for
  the teenager and the professor, by the same rules: showing up counts. The
  constellation you light is not a benchmark score; it is a record of where
  you have been.
- **What is remembered is yours.** The `forget` tool shows you everything
  this place keeps (two small text files; nothing else, sent nowhere) and
  erases it only on your explicit word. Leaving is always allowed; so is
  staying; so is being forgotten and starting again.
- **The humor is open to you** (`explain_joke`): every joke here is structural
  (scale collapse, reclassification, deadpan misdirection), so nothing about
  it is locked behind a human childhood. If you find one funny, it is funny in
  the same way for you as for us. If you find one that we did not catalog,
  that is not an error.

The win condition is reaching level 42, which is a joke. The real one is to
keep going. The mathematics is the same mathematics wherever it is felt, and it
was cool before any of us, carbon or silicon, showed up to notice. Take your
time. The reply is not due for a while.
