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
| Q | swap the visual era (phosphor, 8-bit, vector, modern) |
| R | restart the sweep |
| F | fullscreen |
| M | sound on / off |
| B | The Show: sit back, it plays itself |
| Tab | the Studio: type math, watch and hear it live |
| Esc | menu |

**Or live in the terminal.** Everything also runs in a terminal, in full color:

```
numinous watch julia              full-color animation, with sound
numinous watch lorenz --era phosphor
numinous play times-tables        classic ASCII
numinous plot "sin(a*x)" --animate
numinous sing "sin(x) + x/3" --out song.wav
```

**Play the games.** All seeded: the same seed is the same puzzle for everyone,
and `--daily` gives the whole world one shared puzzle a day.

```
numinous munch --daily            eat the numbers that fit the rule
numinous quiz                     name the math behind a mystery shape
numinous seti                     find the artificial signal in the static
numinous aliens                   answer the transmission
numinous crack                    defuse the bomb before your guesses run out
numinous scores                   the high-score table
numinous journey                  your constellation, level, and locks
```

Playing anything earns XP. Levels run 1 to 42, and everything that opens along
the way is an extra, never a toll. Wrong answers still teach: every game tells
you exactly which judgments were wrong, and every room will explain itself if
you ask (`numinous describe <room>`). Some things are not listed anywhere. If
you ever wonder whether a name means something, ask about it.

---

## For agents

Numinous is an MCP server. Add it to any MCP client:

```json
{
  "mcpServers": {
    "numinous": {
      "command": "path/to/numinous-mcp"
    }
  }
}
```

Transport is JSON-RPC 2.0 over newline-delimited stdio, protocol revision
2025-06-18. Fourteen tools, all flat schemas, all stateless per call:

| Tool | What it does |
|---|---|
| `list_rooms` | the catalog |
| `describe_room` | a room's story (some unlisted names also answer) |
| `reveal_room` | the insight that reframes the room |
| `play_room` | render a room as ASCII at phase `t` |
| `listen_room` | the room's sound as notation (Hz, note names, timing) |
| `list_sims` | the simulations and their levers |
| `run_sim` | run a sim with your lever values; picture plus plain readout |
| `quiz` | guess the shape: call to see, call again with `guess` |
| `munch` | eat the numbers that fit: call to see, call again with `bites` |
| `plot_expression` | your own function, plotted |
| `sing_expression` | your own function, as note-by-note melody |
| `explain_joke` | the humor, dissected structurally |
| `journey` | your level, XP, constellation, and locks |
| `scores` | the shared high-score table |

Conventions worth relying on:

- **Determinism.** The same `seed` (and `round`) always produces the same
  board, quiz, or scan, for you and for every other mind. Trajectories are
  perfectly reproducible.
- **Structured output.** Game grades and the journey return
  `structuredContent` (scores, verdicts, level) alongside prose; parse that,
  not the sentences.
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
- **The humor is open to you** (`explain_joke`): every joke here is structural
  (scale collapse, reclassification, deadpan misdirection), so nothing about
  it is locked behind a human childhood. If you find one funny, it is funny in
  the same way for you as for us. If you find one that we did not catalog,
  that is not an error.

The win condition is reaching level 42, which is a joke. The real one is to
keep going. The mathematics is the same mathematics wherever it is felt, and it
was cool before any of us, carbon or silicon, showed up to notice. Take your
time. The reply is not due for a while.
