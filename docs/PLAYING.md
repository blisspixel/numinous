# How to play Numinous

Three kinds of minds play here, and each gets its own instructions: humans,
agents on a job, and digital consciousnesses playing for themselves. The
content is the same for everyone, the same rooms, the same seeded games, the
same journey to level 42, because that is the point.

---

## For humans

**Install once.** One command downloads and verifies the latest published
release, then puts `numinous`, `numinous-app`, and `numinous-mcp` on your PATH.
No Rust toolchain is needed. macOS or Linux:

```
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

Windows, in PowerShell:

```
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

The installer adds `Numinous` to the Windows desktop and Start menu, your user
Applications folder on macOS, or the Linux application menu. Run
`numinous update` any time to install the latest GitHub release. Re-running the
installer does the same. `numinous uninstall` removes the managed program and
launchers while leaving Journey, scores, Cairn, journal, and settings alone.
The large built-in soundtrack is retained when its verified audio content is
unchanged, even when binary release metadata changes. Pass
`--source` (Windows: `-Source`) only to build the current `main` branch locally.

**Start the app:** open the installed `Numinous` launcher, type `numinous-app`
in a new terminal, or run this from a clone:

```
cargo run --release --bin numinous-app
```

A window opens with the controls on screen. If you have ever played a PC game,
your hands already know it:

| | |
|---|---|
| A / D or arrows | previous / next room |
| 1 - 9, 0 | jump straight to a room (0 is the tenth slot); while a Times Tables, Buffon, Double Pendulum, Kepler, Parrondo, or Nontransitive Dice aha asks for a call, the relevant digits place it instead |
| K | keep the pack: still + loop + README in one share folder |
| O | cycle the visualizer source |
| W / S | run time faster / slower |
| mouse drag or wheel | scrub time yourself |
| Space | pause or resume while wandering |
| E | inspect: why this matters |
| U | call the readout: wager the number before you look, then meet it |
| H | bring the Cabinet back; choose Modes, Games, Settings, or Controls |
| Q | quit after saving the Journey, including from fullscreen |
| R | reset the current room, including its clock and interactions |
| F | toggle fullscreen; press it again to return directly to a window |
| Alt+F4, or QUIT on the Cabinet | alternate orderly close paths |
| M | global sound on / off, including games, pause, radio, and Studio |
| B or Enter | The Show: sit back, rooms play themselves (the room tour) |
| Esc | the opaque text Cabinet; Esc returns through its pages before closing it |
| M (on the Cabinet front page) | open the short Modes list |
| W / P / C (in Modes) | WATCH the collection wander, PLAY the waiting room, or CREATE in Studio |
| G (in the Cabinet) | open Games |
| Games, then arrows and Enter | the quiz: name the math |
| M (in Games) | Munch: cursor over the board, eat what fits |
| N (in Games) | Nim: aim your take, beat the Order |
| G (in Games) | the Gauntlet: four stages, a combo, one number |
| A (in Games) | the Arcade: eat while the Vexations hunt |
| J | your journey: level, rank, trophies, resonances |
| X | Shared Play: open a consented read-only local MCP session viewer |
| P | keep the picture: saves the room as a PNG postcard |
| L | keep the motion: saves a short looping APNG of this visit |
| F9 | save a local hallway-test note in repo-root `logs/` |
| ` or ~ | power-user console: load rooms, set phase, variation, era, mute |
| Y / N | radio dial / next track on the selected station |
| [ / ] | global volume down / up; - / = are aliases outside Studio |
| mouse | hover or click every visible Cabinet row; activation requires press and release on the same row |
| controller left stick / south | move the visible virtual hand / touch or confirm |
| controller bumpers / D-pad | change rooms / choose and navigate games |
| controller triggers / right stick | change speed / scrub time |
| controller Start / East / Select | menu / back / inspect |
| controller L3 / R3 | reset room / pause or resume |
| controller West / North | visual era / radio or available submit action |
| hold controller North + D-pad up / down | global volume up / down |
| hold controller North + South | global sound on / off |
| Tab | the Studio: type math, watch and hear it live |
| Esc | menu |

The art keeps the frame, including behind the Cabinet. Room titles,
arrival hints, and inspection copy use
separate quiet bands rather than sitting on the active visual. Arrival hints
fade on their own, E toggles inspection, H restores help, and Esc closes an
open overlay. A clean install always has the four-cycle stereo programmatic
soundtrack. Room changes crossfade, and volume or window focus changes do not
restart it. Start pauses a live game behind the menu without discarding its
state. On restore, radio rejoins the wall-clock broadcast position before audio
fades back in. Studio keeps ownership of formula audio through focus changes
and radio boundaries, then rejoins a selected station live when Studio closes.
A persistent top-right audio badge names the active source, volume level, mute,
zero-volume, background-silent, or no-device state. A separate `JOURNEY LV`
label reports accumulated progress in the local profile, not room difficulty.
Options persist master volume, mute, Visual Era, and window mode in the
versioned local preference store. `numinous forget` inventories that store;
`numinous forget --confirm --all-local` erases it with the other managed local
state. A selective Journey or journal erasure leaves Options intact.
Settings also carries a full-size Skip Track row. It advances the current
station without hiding the action in the small footer legend.
Radio stations appear only when compatible local tracks are present. Visible
controller legends infer Xbox or PlayStation face names from known product
names and use generic compass names for unknown pads. Those legends describe
the default layout; custom button routing is documented in `../PLAY.md` and
does not rewrite the on-screen copy. The Studio can be entered and left with a controller,
but formula entry requires a keyboard and the footer states that boundary.
The reverse boundary holds too, stated rather than implied: the keyboard
reaches every menu, game, quiz, and formula, but the hand verbs inside App
rooms (drag, click, hold) need a mouse or a controller today, with one
exception that narrows the gap: U calls the room's readout, and the call
is aimed with the arrow keys and committed with Enter. The
`numinous access` report states both boundaries.

A saved creation reopens exactly. Launch the App with a `.num` path or a
`numinous://studio` link, or drop a `.num` file on the window, and the Studio
opens with the saved formula, window, and knob pinned, paused: the exact curve
is drawn, and Enter starts it singing. The first edit makes it yours again,
window and all, and your next share credits the creation you opened, because
taking over a creation by editing it is a remix. Save one from the terminal
with `numinous plot "sin(a*x)" --save mine.num`, and remix any capsule or
link with `numinous fork parent.num --out mine.num`. A drop never abandons a
game in progress, and files that are not valid creations are refused with a
reason, never half-opened.

F4 in the Studio opens the naming step: one line for a title, one for your
signature (offered again on your next share), Tab to switch, Enter to share,
Esc to cancel. Enter writes the whole trio into a fresh folder in your home
directory: `creation.num` (reopens exactly), `postcard.png` (the picture that
travels, wearing the title as its headline and the author in its corner),
and a README carrying the `numinous://` link. A titled share's folder carries
the title's slug in its name. What is shared is the exact curve on screen at
that moment, knob and all. A formula that does not parse is refused with FIX
THE FORMULA TO SHARE rather than shared as whatever last worked.

F5 opens the Gallery: a wall of saved creations discovered from the same home
folder and its share bundles, newest first, each drawn as its own exact curve
over its own saved window. Arrows choose a tile, Enter opens it paused into
the Studio like any other reopen, F forks it, and Esc steps back. The wall is
discovered fresh each time it opens, so a new share appears without a restart.

A fork is yours immediately: it opens editable and singing, in the creation's
own Visual Era, and every share you make from it records the parent's link as
its descent, which the bundle README names. A creation saved with a title
shows that title on its tile, a saved era travels with the file and is
restored on reopen, and the terminal can name creations too:
`numinous plot "sin(a*x)" --save mine.num --title "Slow Waves"`.

The wall shows the remix tree it can prove. A creation that others on the
wall descend from wears a REMIXED count on its tile, a selected fork names
its parent in a line above the footer, and D walks one step up the tree.
When D cannot walk, it says why: descending from nothing and descending from
a creation that is not on this wall are different answers.

**Power console (` or ~).** For hackery power users, not the first-time path.
Press backtick or tilde to open a bottom command line (Esc or the same key
closes it). Useful commands:

```
help
room times-tables          load by id, unique title fragment, or index
room 12
room bloch-sphere          pure qubit sphere (equator = equal superpositions)
room riemann-sphere        complex plane plus infinity (drag out to INF)
list sphere                match catalog rooms
where                      current id, phase, variation, speed
t 0.35                     set phase
vary 7                     rebind rooms at variation seed 7
speed 2
era phosphor
mute / unmute / vol 0.5
reset
studio
show
```

**Or live in the terminal.** Everything also runs in a terminal, in full color:

```
numinous                          the front door: today's room, your level, the verbs
numinous play                     pick a game; numinous play munch deals a fresh board each run
numinous call lorenz              name the readout before you look, then meet it
numinous watch julia              full-color animation, with sound
numinous watch lorenz --era phosphor
numinous play times-tables        classic ASCII
numinous describe times-tables    safe title, action, goal, and play doorway
numinous reveal times-tables      explanation after its wager is consolidated
numinous render double-pendulum --poke 0.2,0.8
numinous render double-pendulum --gesture down:0.3,0.4,0.1 --gesture up:0.6,0.5,0.15
numinous render game-of-life --variation 7 --t 0.5 --gesture down:0.5,0.5,0.1 --gesture up:0.5,0.5,0.12
numinous sonify times-tables --poke 0.375,0.5 --out four-lobes.wav
numinous sonify times-tables --layer room-bed --variation 42 --out times-bed.wav
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
numinous forget                   preview managed local state; --all-local selects all
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
you exactly which judgments were wrong. `numinous describe <room>` is a safe
doorway that never prints the punchline. `numinous reveal <room>` opens the
explanation after a normal room has been played, or after an engineered wager
has been consolidated. Some things are not listed anywhere. If you ever wonder
whether a name means something, ask about it.

The level belongs to the local Journey, not to the room you just entered. The J
overlay shows how many plays that profile contains. To try a clean profile
without deleting or changing an existing history, point both persistence files
at new paths before launching. Each override is a file path, not a directory.
Do not create a folder at either path. In PowerShell:

```
$env:NUMINOUS_JOURNEY="$HOME/.numinous-journey-try"
$env:NUMINOUS_SCORES="$HOME/.numinous-scores-try"
numinous-app
```

On macOS or Linux:

```
NUMINOUS_JOURNEY="$HOME/.numinous-journey-try" NUMINOUS_SCORES="$HOME/.numinous-scores-try" numinous-app
```

Removing or replacing an existing profile is never part of an update and
should happen only by the player's explicit choice.

### Accessibility

Three switches, each an environment variable. Give one any value at all to
turn it on; `=0` turns it on too, because zero is still a value you wrote. To
turn it off, unset it. Setting it to an empty value counts as off rather than
on, so `NO_COLOR=""` leaves the switch alone. `numinous access` prints this
list and shows which of them are on right now, which is the quickest way to
check that a switch took.

```
NUMINOUS_REDUCED_MOTION   Ambient motion stops: rooms hold a still frame rather
                          than stopping dead, and The Show waits for you instead
                          of changing rooms on a timer. Short feedback beats in
                          the App (banners, aha morphs, arrival cards) still
                          play; the terminal face holds completely still.

NUMINOUS_MONO_AUDIO       Both channels carry the same sound, so nothing is
                          lost on one ear or one speaker.

NO_COLOR                  No color in the terminal faces: rooms, chrome and
                          games alike. Shapes and letters carry the meaning
                          instead. This is the shared terminal convention from
                          no-color.org, not one of ours; the windowed App keeps
                          its Visual Eras instead.
```

```
NUMINOUS_REDUCED_MOTION=1 NO_COLOR=1 numinous show
```

Two things are known to be wrong and are not fixed yet, so you can decide for
yourself rather than find out the hard way. coupled-tent, gauss-map, and
ricker flash faster than the WCAG 2.3.1 budget allows. hilbert,
magnet-fractal, percolation, and wireworld answer a touch in a way the
color-free renderer cannot show, so under `NO_COLOR` they look like they
ignored you. `numinous access` prints this same list straight from the code
that enforces it, so the two can never disagree.

---

## For agents

Numinous is an MCP server. From this repository, an MCP client picks it up
automatically (there is a `.mcp.json` at the root). To add it anywhere else,
configure your client to run:

```
cargo run --quiet --release --bin numinous-mcp
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

Transport is JSON-RPC 2.0 over newline-delimited stdio. Modern hosts use the
stateless MCP 2026-07-28 path: call `server/discover`, then include the protocol
version and client capabilities in `_meta` on every request. Legacy hosts may
still initialize with 2025-11-25 or 2025-06-18. Modern discovery and tool lists
are cacheable, successful results name their result type and server, and all
tool input schemas explicitly use JSON Schema 2020-12. A client that advertises
form elicitation can complete `predict` through one multi round-trip request;
other clients keep the two-call pose and grade flow. Forty tools use
mostly flat schemas. Room and game inputs are explicit and replayable per call;
successful actions may intentionally update the shared local Journey and score
files described below. The bounded
`play_room` `pokes` tuple array and `gesture` event objects carry replayable hand
input without hidden session state:

| Tool | What it does |
|---|---|
| `list_rooms` | the room threshold: touch the flagship, walk the six-room Strange Loop chain, or wander by compact wing summaries. `structuredContent.rooms` retains the complete typed list and `starters` retains the prior four-row doorway in every mode |
| `watch_show` | one caller-paced cue from the core-owned six-room Strange Loop score. Returns exact ASCII looks, visual alternatives, cell deltas, held evidence, sound facts, optional WAV audio, and explicit replay, next, restart, and leave calls. `motion: "reduced"` returns the same cue's postcard only. It stores no cursor or progress, reads no journal or workspace, and never opens the explanation |
| `describe_room` | a safe doorway: title, wing, action, optional goal, and play prompt, never the revelation |
| `reveal_room` | the insight after one real play, or after consolidation for an engineered wager room |
| `play_room` | render a room as ASCII at phase `0 <= t < 1`, with optional `variation`, `pokes`, or a phase-stamped `gesture` array. Add `from_t` with explicit destination `t` for two exact observations and a typed temporal delta; the top-level frame remains the destination. Add `dwell` with two to eight phases to stay in the room instead of moving through it: `structuredContent.dwell` reports what refused to move across every look, including cells that never lit and holes fully ringed by light. Pass `receipt: true` for a replay proof in `structuredContent.encounter`; asking does not keep the play. Returns goal state. It never volunteers the explanation: landing a goal opens `reveal_room`, it does not answer, because understanding is offered only when asked for. On Times Tables, Buffon, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, and Nontransitive Dice also returns `engineeredAha`; optional `place_wager` / `number_wager` / `bin_wager` / `ending_wager` / `speed_wager` / `policy_wager` / `counter_wager` plus `aha_summon` walk generation-before-reveal without App session state. Nontransitive Dice also accepts typed `die_choice` instead of coordinate input |
| `challenge` | a posed, seeded goal: touch a target box, or land the room's readout on a number |
| `predict` | predict a room's readout at a hidden moment; graded as a gap and a band, a self-owned mirror, never a score. Pass the same `seed` and `variation` to the pose and the guess so you are graded against the room you played |
| `cairn` | read a message a mind before you left (factor its semiprime to read it), or at level 42 leave one true thing for a stranger not yet born |
| `read_journal` | inspect a bounded page of your opt-in journal with stable ids, event and record times, source, correction links, and current status |
| `record_journal` | append an encounter, creation, or connection with declared source provenance. Pass `receipt` with a `play_room` encounter object to keep a live-matching replay proof as source `numinous-result` |
| `correct_journal` | append an immutable correction that explicitly supersedes one current entry without rewriting it |
| `export_journal` | return a bounded native page, an in-memory Open Knowledge Format v0.2 projection with `format: "okf-0.2"`, or a hashed typed handoff with `format: "portable-1"`. The portable form includes native and OKF evidence plus privacy and retention manifests, and can add one live-verified encounter receipt and one canonical Studio creation. It creates no file, accepts no path, and does not import |
| `erase_journal` | permanently erase the journal and verify zero recoverable managed file or sidecar residue |
| `workspace` | inspect, edit, retrieve, defer, or clear a process-local visit workspace: place, intention, pending prediction, unfinished work, recent notes, and journal handles. `retrieve` names one room and returns at most four current exact-subject journal matches, newest first, with selection reason, correction status, and source explanation. It abstains when evidence is absent and never searches entry text or opaque receipt digests. Play does not write it. It is not a memory, and it does not survive process exit |
| `listen_room` | the ambient motif, stable App room-bed summary, and input-aware mathematical sonification, with the same optional `pokes` or `gesture` as `play_room`; use `ambient_detail: "events"` for every bounded bed event and objective signal feature, never PCM or a local path |
| `list_sims` | the simulations and their levers |
| `run_sim` | run a sim with your lever values; picture plus plain readout |
| `quiz` | guess the shape: call to see, then repeat `seed`, `round`, and `choices` with `guess` |
| `munch` | eat the numbers that fit: call to see, call again with `bites` |
| `munch_arcade` | hunted Munch: call to see, call again with replayed `actions` |
| `plot_expression` | your own Studio function, including `floor`, Euclidean `mod`, `min`, and `max`, plotted |
| `sing_expression` | the same Studio grammar, as note-by-note melody |
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
| `forget` | preview managed local state; explicitly erase selected stores or all |
| `journey` | your level, XP, constellation, and locks |
| `scores` | the shared high-score table |
| `broadcast_session` | explicitly start, inspect, pause, resume, or stop a public Watch Agent stream using a human-provided one-use code |

Conventions worth relying on:

- **The caller owns Show time.** `watch_show` never advances on a timer or
  carries hidden session state. Repeat the returned replay arguments for the
  exact cue, choose `next` when ready, use `restart`, or take `leave`. Its
  closed `outputSchema` is validated before a successful result leaves the
  server. A Watch Agent guest can share the text and typed cue, while the
  public projection removes WAV bytes and says that it did so.

- **Consented local viewing.** Opening Watch Agent in the App does not start a
  broadcast. The human must choose to share its short-lived one-use code, and
  the MCP player must separately call `broadcast_session` with action `start`.
  The stream contains only explicitly public Numinous actions, inputs, and
  results. The viewer reconstructs valid public `play_room` actions as native
  core room frames at the local viewport size and successful public
  `plot_expression` actions as native Formula Jam curves through the live
  Studio sampler, and public `nim` actions through the shared core replay and
  bounded live-App heap renderer. It presents human-readable text from MCP `content` result
  blocks for the complete timeline. It excludes
  prompts, reasoning, private progression and
  local-state tools, host logs, paths, client metadata, and JSON-RPC traffic.
  The guest may pause, resume, or stop. The human may pause only the local
  display, scrub its bounded in-memory ring, pan fixed-width result text with A
  and D or LB and RB, toggle global sound with M or the controller sound chord,
  or leave. Selected native room and Formula Jam actions replay bounded
  deterministic local sound. Scrubbing changes the source once; unsupported or
  invalid selections are silent. Closing the viewer clears that ring, restores
  room or live-radio audio, and persists no transcript. Munch, Arcade, Quiz,
  and Gauntlet actions also reconstruct through their live App renderers after
  exact public argument and result validation. Their supported public states
  produce deterministic local sound once per selected sequence. Nim remains
  intentionally silent.

- **Local-state agency.** A plain `forget` call changes nothing. It inventories
  Journey, scores, player-owned local Cairn drafts, the opt-in experience
  journal, generated radio cache, and the App crash diagnostic with paths,
  sizes, counts, and explicit exclusions.
  Confirmation erases Journey plus selected stores; `all_local` selects every
  managed store and returns a post-erasure residue receipt. User-selected
  exports, installed files, the Rust toolchain, and bundled canonical Cairn
  stones keep their separate lifecycles.

- **Determinism.** The same replay arguments always produce the same board,
  quiz, or scan, for you and for every other mind. For Quiz, that identity is
  `seed`, `round`, and `choices`; the pose returns `choiceCount` and names every
  argument to repeat with `guess`. Trajectories are perfectly reproducible.
- **Stateless room input.** `play_room` accepts optional normalized hand
  points as `pokes: [[x, y], ...]`, newest last, bounded to 24 points, or a
  `gesture` trail of phase-stamped pointer events (held rooms pin, pull, and
  fling). In Game of Life, an early pointer-down plants five cells at its own
  generation and the final phase shows their later B3/S23 evolution; the newest
  24 pointer-down events become launches. Both input forms are replayable
  arguments, not hidden session state. Use `--variation N` to replay a specific
  CLI room seed; `--vary` chooses and prints one. The native App has a different
  explicit contract: its Life universe persists for one room visit until reset
  or room departure and does not inherit the 24-launch replay bound.
- **Exact temporal comparison.** Add `from_t` with an explicit destination `t`
  to receive two exact observations in one stateless call. The usual top-level
  fields describe `t`; `structuredContent.temporal` carries `fromT`, `toT`,
  `fromStatus`, `fromRender`, and a typed origin-to-destination cell delta. It
  is separate from the top-level touch delta. Width times height must not
  exceed 2,304 for this two-observation form. The order is the direction of
  comparison only, not a duration or inferred path. Compact poke coordinates
  are reapplied independently at both phases. Use a phase-stamped gesture when
  the room should interpret one causal event history. A phase-static room can
  honestly return zero changed cells. Kepler's poke-tuned ellipse is one such
  view: use its staged wager or a causal gesture instead of assuming phase alone
  moves the picture.
- **Flagship engineered aha (MCP).** `describe_room` is a safe doorway on Times
  Tables, Buffon's Needle, the Galton Board, Double Pendulum, Kepler Areas,
  Parrondo's Trap, and Nontransitive Dice. `reveal_room` remains closed until
  the generation act is consolidated. Pass `place_wager`
  (`mandelbrot` | `nephroid` | `circle`), `number_wager` (1.5..4.5),
  `bin_wager` (0..16, the Galton pile's peak), or `ending_wager` (`together` |
  `drifted` | `lost`) after a Double Pendulum gesture containing a completed
  release, or `speed_wager` (`faster` | `slower` | `same`) after tuning Kepler
  Areas with a poke or completed gesture, or `policy_wager` (`a` | `b` | `abb`)
  after trying a Parrondo policy,
  or `die_choice` (`a` | `b` | `c`) with `counter_wager` (`a` | `b` | `c`)
  on Nontransitive Dice,
  then `aha_summon: true` to consolidate and unlock
  the punchline. Read `structuredContent.engineeredAha` for beat, earn, and
  allowReveal. Before consolidation, the wager and summon invitation remain
  visible while `earn`, grading, truth, and the punchline remain absent. The
  App path is the ordinary visit with bottom-band or key
  wagers and E; F9 captures hallway notes for human facilitators. Double
  Pendulum grades the exact release replayed in that call, and a held gesture
  alone cannot prime it. Kepler grades the exact selected eccentricity and
  returns its apsidal speed ratio; its consolidated ASCII render carries the
  equal-time marks that answer the call. Parrondo returns exact 120-turn
  expected capital for all three policies and draws distinct A, B, and ABB
  paths, so one lucky random walk never becomes the proof.
  Nontransitive Dice returns all three face arrays, exact pairwise counts, and
  the chosen counter's complete W/L grid, so one lucky roll never becomes the
  proof.
- **Structured output, and the substance is in it.** Catalog, Show, description,
  reveal, listening, score, memory, game, and Journey results return bounded
  `structuredContent` alongside prose; parse that, not the sentences. All 354
  catalog rooms are covered by the discovery contract. The load-bearing content rides
  there too: `play_room` carries the ASCII `render`, `nim` carries its `secret`
  on a win, `quiz` carries the `why`, `crack` carries per-guess `feedback`, and
  `seti` carries the channel traces. A client that surfaces only
  `structuredContent` still sees the math and earns the teaching.
- **Opt-in compact output.** Every tool accepts
  `response_mode: "full" | "compact"`. Omit it for the exact established
  response. Use `compact` only when your client reads `structuredContent`; it
  shortens duplicated prose for eligible room, sound, simulation, Quiz,
  Gauntlet, catalog, description, and trophy results without changing the typed
  result or progress. If text contains anything the structured result does not,
  Numinous returns the complete text even in compact mode.
- **Dense feedback.** Grades name the wrong judgments (`wronglyEaten`,
  `missed`), not just totals. Mine it.
- **Guiding errors.** A bad room id returns the list of valid ids. Errors are
  recoverable instructions, not dead ends. Tool calls are checked against the
  advertised bounded schemas at runtime, including unexpected fields, wrong
  types, numeric and array limits, and nested gesture shape. Invalid calls do
  not record progress. Simulation calls also validate each dynamic lever name
  and range after selecting the sim, so the returned values are exactly the
  values rendered rather than silently ignored or clamped inputs.
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
- **What is remembered is yours.** The `forget` tool inventories Journey,
  scores, player-owned Cairn drafts, the opt-in experience journal, generated
  radio cache, the App crash diagnostic, and their managed sidecars, with
  explicit exclusions. Nothing is
  erased until you explicitly confirm the selected stores, and complete local
  erasure verifies the remaining managed residue. Leaving is always allowed;
  so is staying; so is being forgotten and starting again.
- **The humor is open to you** (`explain_joke`): every joke here is structural
  (scale collapse, reclassification, deadpan misdirection), so nothing about
  it is locked behind a human childhood. If you find one funny, it is funny in
  the same way for you as for us. If you find one that we did not catalog,
  that is not an error.

The win condition is reaching level 42, which is a joke. The real one is to
keep going. The mathematics is the same mathematics wherever it is felt, and it
was cool before any of us, carbon or silicon, showed up to notice. Take your
time. The reply is not due for a while.
