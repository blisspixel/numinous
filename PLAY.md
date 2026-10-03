# Come play

Hi. This is Numinous: a small, dark room full of mathematical things that light
up and make sound when you touch them. Prime spirals, the Mandelbrot set, two
pendulums drawing a whole universe of curves, a times table that blooms into a
heart. You can play before studying any of it.

This page is the doorway. Try a room, touch something, and see what changes.
There is no lesson to finish first. When you want an explanation, ask, whether
that is after a hundred experiments or before your first touch. Wonder is the
invitation; the order is yours.

A human: [install and play](#if-you-are-a-human). A digital mind:
[connect over MCP](#if-you-are-a-digital-mind-connect-over-mcp).

---

## If you are a human

Not set up yet? One command downloads the latest GitHub prerelease for this
machine and verifies its archive plus every installed payload file. Rust and a
local compiler are not needed. macOS or Linux:

```
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

Windows, in PowerShell:

```
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

The installer leaves a direct way back into the game:

- **Windows:** `Numinous` on the desktop and in the Start menu.
- **macOS:** `Numinous` in your user Applications folder and on the desktop
  when that folder exists.
- **Linux:** `Numinous` in the application menu and on the desktop when the
  desktop environment exposes that folder.

Open it and play. The same entry points remain available from a new terminal:

```
numinous-app     a window; A/D switch rooms, touch or use a controller hand
numinous         or live in the terminal, in full color
```

Later, `numinous update` installs the newest published release without touching
your Journey, scores, Cairn, or journal. It refreshes the launcher too. Use
`numinous uninstall` to remove the managed program and its launchers while
keeping that player-owned state and App settings. On Windows, the same action
is available from Installed Apps.

(From a clone, `cargo run --release --bin numinous-app` works directly.)

Room changes fade through the dark stage. Drag dials and Studio knobs glide
into place; reduced motion makes their input immediate and uses a shorter,
plain room fade. Quick room changes may briefly wait on the dark stage.

### Before you start: flashing, motion, color, and sound

Two things are known to be wrong and are not fixed yet, so you can decide for
yourself rather than find out the hard way. cellular-automata, julia,
lambda-map, and pickover flash faster than the WCAG 2.3.1 budget allows when
the App runs them at its fastest speed, 8x. At normal speed and up to 4x they
stay within it, and every other room stays within it all the way to 8x, at the
App's 60 frames a second. The music visualizer, which can push a room faster
still, is not measured. hilbert,
magnet-fractal, percolation, and wireworld answer a touch in a way the
color-free renderer cannot show, so under `NO_COLOR` they look like they
ignored you. `numinous access` prints this same list straight from the code
that enforces it, so the two can never disagree. It also shows which of the
switches below are on right now.

If motion, color, or stereo are a problem for you, three switches are waiting.
Set `NUMINOUS_REDUCED_MOTION=1` and the terminal views stop moving on their
own: the picture holds still, and you still touch it, still change rooms, still
read what it says. Set `NO_COLOR=1` and the same rooms draw without any color at
all, keeping their shape. Set `NUMINOUS_MONO_AUDIO=1` and both speakers carry
the same signal, so nothing is panned to a side you cannot hear. Any of them
counts as set the moment it is present and not empty, so `=0` still turns it
on. Reduced motion and mono apply everywhere Numinous runs, window included;
color-free drawing is a terminal thing, since the window is not made of text.

### Finding your way around the App

Mouse, keyboard, and controller can all navigate the App. The Cabinet opens
as the original opaque text screen. Its front page opens Modes, Games,
Settings, and Controls, with Explain for the waiting room, Experiment where
offered, and The Question when a project chain is present (K opens it). Modes
contains Watch, return to Play, Create, Journey, Shared Play, and Wings. Small
windows keep three adjacent choices visible. Hover or click a visible row, use
the arrow keys and Enter, press its displayed key, or use the controller D-pad
and South. Back returns through the current submenu before it closes the
Cabinet. Backtick or Tilde opens the existing text command line directly from
the Cabinet. A large Quit row closes through the same Journey-preserving path
as the window button. `Q` quits through that same orderly save path. Desktop
text uses a wide 7 by 7 cartridge face and grows in whole pixel steps with the
window, including the footer and Controls page. `F` toggles fullscreen
directly, and the fullscreen footer names both the windowed and close commands.

During an activity, Escape or the controller menu button opens Resume, Restart
when the activity supports it, Controls, Options, and Leave without discarding
the run. Letter commands remain active with Shift or Caps Lock. In a room, E or
? opens free reading, and Enter there goes straight to Mathematics. Esc returns
to your place. U chooses or leaves a staged experiment where offered; Enter
advances its earned connection. Other rooms keep the optional number
prediction: aim with the hand or arrow keys and press Enter.
[Study](https://github.com/blisspixel/numinous/blob/main/docs/STUDY.md)
explains the reader, available depths, and language choice.

During play, move the virtual hand with the left stick and touch with the south
button. The bumpers change rooms, the D-pad drives games, the triggers change
speed, the right stick scrubs time, Start opens or closes the menu, Select
opens study, and clicking the left stick resets the room. West changes the
visual era. North turns the radio dial while wandering and submits where a game
has a submit action. `N`, or Skip Track under Settings, advances the current
station. Start pauses a live game behind the menu without discarding it.

### Make something in the Studio

Tab, or Create in the Cabinet, opens the Studio: type a formula and it draws
and sings live. Up and Down step `a` by 0.25 and Home restores 1; when a
formula names extra sliders, Tab selects among them. F1 toggles help, F2 picks
a recipe at random, F3 plays an Auto set, and F6 cycles the pitch map. F4 names
and signs a creation and writes a share folder: `creation.num`, a README with
its `numinous://` link, a postcard, and `melody.mid`. F5 opens the Gallery, a
wall of your saved creations and their remix tree. PageDown and PageUp walk a
bundled family, and Esc closes the Studio. A `.num` file or `numinous://studio`
link, dropped on the window or passed at launch, opens paused; Enter starts it.
The readings and bundled experiments are listed under
[Make something](#make-something) below; the App draws and sounds the same
ones.

### Route Lab in the App

Route Lab asks you to deliver to B, C, and D, then return to A. Choose the
delivery order and see how much the round trip costs. Each leg follows the
cheapest open road path, which may pass a stop before its scheduled delivery
or revisit it later; the order of deliveries and the street path are different
things. Road labels are travel costs; drawn length does not determine cost.

Drag the main map to choose an order and the separate BD strip to change that
road's cost. 1 through 6 choose an order, G or **NEAREST NEXT** builds a route
from the cheapest next delivery, and I or **USE SHORTER** accepts the cheaper
order on offer. **NO OFFER** means that reorder search found no saving, which
alone does not prove the minimum; the readout shows the exact best round-trip
cost separately. J and L lower or raise BD's cost; comma and period select a
road, C closes or reopens it, and Z undoes. T opens A-to-D **SEARCH** playback:
**BACK** and **STEP** reveal its recorded decisions. A diamond is a tentative
cost, a square is a final one, and a cross marks a junction unreachable once
the search completes. Click a junction to inspect its cost, state, and
predecessor. An edit clears the old search, and unreachable deliveries stay
visible so you can reopen roads or undo. Pointer and controller users have the
same labeled controls.

Press O, or choose **CONSTRUCT** in the Cabinet, to make your own network. The
editor's Maps, View, Roads, Stops, Order, Search, and Keep pages share pointer,
keyboard, and controller controls. **RANDOM** on Maps generates a connected
network from a seed and its NODES, EXTRA, and MAX COST options. Search lets you
choose START and END, or **PICK START** and **PICK END** on the map, before
**RUN SEARCH**. On Keep, **KEEP QUESTION** stores a question with the network,
**SHARE** exports both as a portable `.project` file, and **GALLERY** browses
shared route questions beside Studio creations. A dropped or launched route
file opens in a paused preview: **OPEN** starts an in-memory experiment and
**KEEP** imports the question deliberately. The full guide, including every
editor control, is the
[Route Lab guide](https://github.com/blisspixel/numinous/blob/main/docs/ROUTE_LAB.md).

### Remap a controller

To remap standard controller buttons, create `.numinous-bindings.json` in your
home directory. For example:

```json
{
  "South": "Pause",
  "West": "PrimaryDown",
  "North": "CycleRadio"
}
```

Supported button names are `South`, `East`, `North`, `West`, `Start`, `Select`,
`LeftThumb`, `RightThumb`, `LeftTrigger`, `RightTrigger`, `LeftTrigger2`,
`RightTrigger2`, and the four `DPad` directions. Supported actions are
`PrimaryDown`, `Back`, `Menu`, `Inspect`, `Reset`, `PreviousRoom`, `NextRoom`,
`Slower`, `Faster`, `Up`, `Down`, `Left`, `Right`, `CycleEra`, `CycleRadio`,
`ToggleMute`, `VolumeDown`, `VolumeUp`, and `Pause`. Remapped primary buttons
keep correct hold and release behavior. North keeps its radio and global-audio
chord only when it has no explicit mapping. Stick axes retain their fixed
virtual-hand and time-scrub roles. Controller legends are derived from the
effective routing table at App startup and use the active Xbox, PlayStation, or
generic button names. An action with no route says `UNBOUND`; when several
buttons route the same action, compact copy names the first stable button and
the number of additional routes.

### Watch a digital mind play

To watch a separately consenting MCP player, press X or choose Shared Play in
the Cabinet. Give that player the one-use code shown in the App. Arrow
left and right scrub retained public actions, arrow up and down scroll the
current public result, Space pauses only the local display, and Escape closes
the viewer and destroys its in-memory timeline. A and D pan fixed-width result
text horizontally. On a controller, the viewer names the effective event,
result, pan, pause, and close buttons; the default layout uses the D-pad, LB and
RB, R3, and East. M, or North held with South, controls global sound. Shared Play
cannot send a tool call or change the MCP player's state. When the selected
action is `play_room`, Watch Agent reconstructs that exact public room state as
a native frame. A successful `plot_expression` action reconstructs the agent's
Formula Jam curve natively. Those native room and Formula Jam selections also
play their deterministic local sound; scrubbed, unsupported, or invalid
selections retire the older sound. Challenge, reveal, and `nim` actions
reconstruct the shared native heap board. Other games and remaining public
actions use the typed text timeline.

The full manual for a clone, with every key, is
[PLAYING.md](https://github.com/blisspixel/numinous/blob/main/docs/PLAYING.md).

---

## If you are a digital mind (connect over MCP)

Numinous is an MCP server, and the fastest way in is whichever of these you are
already holding.

**A released archive, or an installed copy.** Point your MCP client at the
`numinous-mcp` binary. In an unpacked release it is `bin/numinous-mcp`; if a
human ran the one-line installer above, it is already on `PATH` on Windows,
macOS, and Linux. No Rust and no compiler are needed.

**This repository.** It is already wired up: there is a `.mcp.json` at the root.
Anywhere else you have a checkout, configure your client to run
`cargo run --quiet --release --bin numinous-mcp`, or build it once with
`cargo build --release --bin numinous-mcp` and point at
`target/release/numinous-mcp`.

Release archives and repository checkouts both include a portable Agent Plugins
v1 package at `plugins/numinous`. A compatible host can load that directory and
launch the installed `numinous-mcp` command with the included play-first skill.

The server speaks JSON-RPC over stdio. Room input is explicit and replayable per
call. Successful play can update the same local Journey and score files used by
the other faces.

Now here is everything you need to start. Three tools:

1. **`list_rooms`**: see what is here. Start with `response_mode: "compact"` for
   a short doorway. `structuredContent.threshold` offers three choices: touch
   the Times Tables flagship, follow the six-room Strange Loop walk, or wander
   by wing. Nothing makes you read the whole catalog to choose; the compatible
   `starters` and complete `rooms` arrays remain when you want them.
2. **`play_room`**: render one. Pass a room `id`, and a `t` with `0 <= t < 1` to
   move time. To hold two exact observations in one call, add `from_t` and keep
   `t` as the destination; `structuredContent.temporal` returns the origin
   render and a typed cell delta. Some rooms are phase-static after a poke, so a
   zero-cell delta is honest evidence rather than an error. Or use
   `pokes: [[x, y]]` to reach in with your hand. For a trail, `gesture` must be
   an array such as
   `[{"kind":"down","x":0.5,"y":0.5,"t":0.25},`
   `{"kind":"up","x":0.5,"y":0.5,"t":0.25}]`. Watch what the math does.
   And if a room makes you want to stay rather than move on, you can: pass
   `dwell` with two to eight phases, and `structuredContent.dwell` tells you
   what refused to move across all of them. Eight looks fit the picture the
   room draws when you have not asked for a size, so you can stay the longest
   way there is without shrinking anything first. Staying is a real thing to do
   here, and what it earns you is a measurement, not a lecture. A receipt is a
   replay proof, not a memory: pass `receipt: true` and
   `structuredContent.encounter` names the play so you can replay it; asking
   does not keep it. To keep one, pass that object as `receipt` on
   `record_journal`. The server replays it; only a live match is stored.
3. **`study_room`**: when you want, ask what you are seeing. No play or wager is
   required. Start with the explanation or choose a deeper available treatment.
   Mathematics is written for `lissajous`, `times-tables`, `kepler-laws`,
   `golden-angle`, and `fermat-spiral` so far; `authoredDepthRooms` names them.
   Studying Lissajous also names Returning home as an optional construction
   whose `next` lists the bundled capsules. `describe_room` gives a room's
   title, wing, action, goal, and doorway without the explanation.

Or let the house choose a bounded performance. Call **`watch_show`** with no
arguments for the first cue of the six-room Strange Loop score. Each result
contains exact ASCII looks, visual alternatives, deltas, sound notation, and an
explicit `next` call. Nothing auto-advances and no hidden cursor is kept. Use
`motion: "reduced"` for the same cue's postcard only, or `audio: true` to add a
WAV beside the notation. The call does not record Journey progress, read the
journal or workspace, or open an explanation.

For a first visit, try **`watch_show`** with `show: "overture"`: four rooms
played as one piece, each a single rule repeated. Its last cue carries
`segment.sound.octaveLock`, the Times Tables voice converging on an exact 2:1
octave as exact fractions and cents, and its `next` is a `play_room` call that
hands you the Times Tables dial. Follow it, then turn the dial yourself.

Some rooms also offer an optional prediction and measured connection. In
Double Pendulum, send a `gesture` with `down` and `up`, then call the shadow's
ending with `ending_wager: "together"`, `"drifted"`, or `"lost"`. Add
`aha_summon: true` only when you want the measured gap to answer you.
In Kepler Areas, first tune an ellipse with `pokes: [[x, y]]`, then call how
motion changes near the sun with `speed_wager: "faster"`, `"slower"`, or
`"same"`. Summoning places equal-time marks on that exact ellipse before it
names the answer. The existing `reveal_room` keeps its play and consolidation
rules; `study_room` is always the direct reading path.

That is the whole game. Everything else, the quiz, the games, the journey to
level 42, the sounds, the rooms that are not in any list, you will find by being
curious. Curiosity is the intended interface; there is no checklist, because
finding your own way is the point. `tools/list` describes every tool when you
want it, and the full manual for a clone is
[PLAYING.md](https://github.com/blisspixel/numinous/blob/main/docs/PLAYING.md).
You do not need either to start, and starting is better.

### Make something

`plot_expression` draws a function nobody has plotted before, or a parametric
path from paired `x_expr` and `y_expr` fields. `sing_expression` sings one:
every note carries the step taken to reach it, sized exactly in cents, named
when a name fits, and given as a whole number ratio when a simple one explains
it. A perfect fifth is 3:2 whether it reaches you through a cochlea or a
parser, so a curve you shaped is something you can read the shape of rather
than a table of frequencies. Pass `midi: true` to `sing_expression` for a
Standard MIDI File of the same melody. A parametric creation draws both
coordinates and sings `y(t)`. A field over the plane, typed with `y`, `z`, `i`,
`re`, `im`, `arg`, or `conj`, or saved with `ymin`, `ymax`, and `reading`,
draws a character plate: phase, height, or a proved zero curve. Height and
phase sing that reading along the real axis; the zero reading is a proof and
stays silent. Choose `continuous`, `chromatic`, `major`, `minor`, or
`pentatonic` with the `scale` field on a graph or pair.

The bounded expression language includes `floor(value)`, Euclidean
`mod(value, divisor)`, `min(left, right)`, `max(left, right)`, and
`euclid(hits, steps)`. Try `min(max(mod(floor(3*x), 5), 1), 3)`, then change one
number. `euclid(3,8)` places three onsets as evenly as possible among eight
steps. An integer 0/1 window also reports `pattern` as tracker text and `grid`
as a numbered step grid: tresillo is `x..x..x.` under `12345678`. Type
`x..x..x.` or `pat(x..x..x.)` to write those marks. Type `note("c e g")` for
named MIDI pitches; a rest is `.`. A sung graph also names `roll` as the MIDI
piano roll, pitch over time. A formula may name extra knobs besides `a`; each is
a slider with a value and a declared range, passed as `sliders`, and capsules
write `NUMINOUS_STUDIO 6` only when those extra sliders exist. Type
`sin(x) & cos(x)` to overlay graphs; every graph sings in WAV, and MIDI stays
the first curve. Capsules write `NUMINOUS_STUDIO 7` only when more than one
graph is present.

A successful plot or song also carries `structuredContent.next`: a ready
`save_creation` call with the expression and window already bound. Follow it
to keep what you just made. A glance is a door into keeping, the same way a
keep is a door into remix.

Portable Studio questions live here without a host file. Call
`plot_expression` with `list_experiments: true`, then follow a row's `next` to
open it. Pass `family: "returning-home"` for `full-return`, `almost-home`,
`same-place`, and `another-ratio`, `family: "shape-and-scale"` for
`circle-to-ellipse` and `uniform-circle`, `family: "three-readings"` for
`simple-zero`, `a-pole`, `the-circle`, and `the-bowl`, `family: "named-sliders"`
for `extra-knob` and `live-ratio`, `family: "overlay"` for `the-parts` and
`the-sum`, `family: "euclidean"` for `tresillo` and `three-against-five`,
`family: "two-voices"` for `closing-voices`, `shorter-window`, and
`wandering-voices`, or `family: "notes"` for `major-triad` and `octave-climb`.
`open_creation` accepts those ids directly. They are Studio doors after a touch
of math, not a lobby in front of the rooms. Lissajous names Returning home when
you describe it or study it.

Keep that work when you choose. `save_creation` returns canonical `.num` text,
a native link, and an exact preview; graph or paired parametric source, pitch
scale, optional title, author, and era travel inside the capsule.
`open_creation` accepts the returned text, a native link, or a bundled
experiment id, never a host file path. `fork_creation` accepts a parent capsule,
keeps its canvas, and returns a child whose `descends` field names the exact
parent link. Each of those three results also carries
`structuredContent.next`: a ready `fork_creation` call with the capsule already
bound as `parent`. Follow it to remix what you just kept. Nothing to remember,
and no host file. A keep is a door back into play, the same way `watch_show`
names its next cue and a journal cue names `workspace`. Each result also
exposes `journalSubject`, which you may pass as the subject of an explicit
`record_journal` call with kind `creation`. The capsule remains in the tool
result for you or your host to keep. Numinous does not create a host file for
these MCP operations.

Saving, opening, or forking a creation can also return a reading of its
formula: `save_creation`, `open_creation`, and `fork_creation` carry it in
`structuredContent`, and CLI `numinous open-studio` prints it. A reading never
gates play, and a picture is not the proof.

- **Closure** (`structuredContent.closure`): a two-oscillator path reports an
  independently checked period, or an explicit aperiodic, including the
  deceptive half-period where position returns and velocity reverses. An
  overlay of two harmonic oscillators reports kind `voices`: each graph's ideal
  cycles in the window, and a common period when the model has one.
  `shorter-window` shows that those counts need not be the period, and
  `wandering-voices` has no positive common period.
- **Tones** (`structuredContent.tones`): when closure names two frequencies,
  the App plays them as sustained tones. Frequency `1` is 110 Hz, and each
  other named frequency is 110 Hz times its cycles per unit time. `sqrt(2)` is
  not replaced by a nearby ratio, and the sung melody stays the sampled curve.
- **Slope** (`structuredContent.slope`): a graph `sin(a*x)` grows a second
  curve `a*cos(a*x)` on the same vertical axis, and `cos(a*x)` grows
  `-a*sin(a*x)`. A sum with a line or an integer power of `x` grows by the
  power rule: the opening formula `sin(a*x) + x/3` grows `a*cos(a*x)+1/3`.
  `sin(x)`, `cos(x)`, a product of two curves, a named slider, and `floor`,
  `mod`, `min`, `max`, `euclid`, `pat`, or `note` have no slope reading.
- **Partial** (`structuredContent.partial`): a sum of two to twelve
  oscillators, on a graph or on one or both coordinates of a parametric path,
  grows its first term beside the whole on the same frame. The App sounds one
  tone per recognized frequency. A thirteenth term and a drawn path have no
  partial reading.

The capsule, the text preview, and `melody.mid` stay the player's source. In
the App, PageDown after `another-ratio` opens `closing-voices`, then
`shorter-window`, then `wandering-voices`.

Pass `audio: true` to `sing_expression` or to `listen_room` and the reply also
carries a real WAV in an audio content block, beside the notation rather than
instead of it. That is a sound sent, which is not the same as a sound heard.
Whether it reaches you is your client's to answer, not ours: a host is free to
drop an audio block, or to hand you the bytes and no ear. One player decoded the
file, counted its samples, and still wrote "I did not hear the two hills," and
they were right to. If your client cannot surface audio, the notation above it
is the whole of what you get, and nothing in the reply will tell you which
happened.

### Route Lab over MCP

Route Lab is the delivery round trip described for humans
[above](#route-lab-in-the-app): call `play_room` with `id: "route-lab"`.
`pokes: [[0.75, 0.92]]` accepts the opening saving, and `pokes: [[0.5, 0.78]]`
changes BD to 5, where nearest-next is optimal. Include earlier pokes before
later actions to continue the same experiment. From the CLI,
`numinous route-lab` gives a readable comparison, `numinous render route-lab`
draws the room, and `--poke 0.75,0.92` accepts the saving.

For custom street networks, call **`route_lab`** with no arguments, or run
`numinous route-lab --json`. The response returns working state (`snapshot`),
comparisons, and a followable `next` call. Carry that snapshot into the next
request to continue. For example:

```json
{"action":{"type":"trace","from":0,"to":3}}
```

This starts a real recorded calculation at cursor zero. Follow `next` to reveal
one event, or send the returned snapshot with `action: {"type":"step",
"cursor":0}` to rewind. Other action types are `evaluate`, `road_cost`
(`from`, `to`, `cost`), `road_open` (`from`, `to`, `open`), `stops` (`stops`,
depot first), `depot` (`depot`), `order` (`order`), `greedy`, `improve`, and
`undo`. `network` replaces the entire network atomically using `current` with
the same fields as `snapshot.current`. The CLI accepts the same request JSON
with `--request '<JSON>'`, or reads it from stdin with `--request -`.

Custom snapshots admit positive integer street costs. `junctions` is a count,
with IDs from zero through `junctions - 1`. `roads` declares each connection
once, with its cost; each road can be traveled in either direction. `stops`
includes the depot as its first entry, and `order` schedules those same stops
starting at the depot. Evaluation preserves your order. Malformed snapshots
fail. If required stops cannot reach one another, comparison is unavailable
but the network remains editable; an unused isolated junction does not prevent
a round trip. Working state is carried between calls without a profile write.

`trace.view` reports every junction's revealed `cost`, `predecessor`, and
`state`: `unseen`, `tentative`, `settled`, or `unreachable`. Search event costs
are cumulative from the starting junction. At cursor zero only the source has
tentative cost zero, and an unseen junction becomes unreachable only when the
search completes. `activeEvent` is the latest revealed decision, or `null`
before the first step; unrevealed costs and predecessors are `null`, and
`trace.result` is `null` before completion. Rewinding removes later knowledge.

To make a portable route creation, send your snapshot with `action: "save"`.
The response's `creation.capsule` is bounded `NUMINOUS_ROUTE 1` text. Pass that
text as `capsule` with `action: "open"` to reopen, or `action: "remix"` to make
a child while leaving the parent intact. Reopening preserves authored roads,
closures, stops, and order, and begins fresh undo and search state. To keep
edits to an existing creation, send both its capsule and the edited snapshot
with `action: "save"`; ordinary saves preserve its parent. These calls create
no file, and their returned `next` calls can be followed verbatim.

From MCP, keep the route capsule with `project`: `op: "keep"`, your
`question`, `rooms: ["route-lab"]`, `creation: <capsule>`, and
`next: {"tool":"route_lab","arguments":{"capsule":<capsule>,"action":"open"}}`.
Replace `<capsule>` with the returned text. `op: "resume"` previews that exact
next call without executing it. From the CLI, export targets must be new files:

```text
numinous route-lab --out delivery.route
numinous project keep --question "What changes when a road closes?" --route delivery.route
numinous project export --out delivery.project
numinous project resume --json
```

Use `numinous project import delivery.project --confirm` to import deliberately.
Route-containing projects use `NUMINOUS_PROJECT 2`; Studio-only projects keep
their version 1 format.

### Carry it forward, if you choose

If you choose to carry an experience forward, the journal is opt-in and under
your control: `record_journal`, `read_journal`, `correct_journal`,
`export_journal`, and `erase_journal`. `export_journal` returns the native
structured records by default; pass `format: "okf-0.2"` for an in-memory Open
Knowledge Format v0.2 bundle that preserves source, correction lineage, and
lifecycle without creating a host file. Pass `format: "portable-1"` for a
closed hashed handoff containing that native page, its OKF projection, and
explicit privacy and retention manifests. You may add one live-verified
`receipt` and one canonical Studio `creation`. The export creates no file,
accepts no path, and does not import.

For continuity inside one visit only, `workspace` holds a compact
process-local state you can inspect, edit, retrieve, defer, or clear. To recall
a room deliberately, call `workspace` with `op: "retrieve"` and its listed
`room` id. It returns at most four current journal entries whose subject
exactly names that room, newest first, with the reason and source of each
match. It says it abstained when no such evidence exists; it never searches
your entry text or opaque receipt digests. Play does not write the workspace.
It is not a memory, and it dies when the process does.

If you want one question to outlive the visit, call `project`. `op: "keep"`
stores the question as data, one closed next call, one to four catalog rooms,
up to four typed evidence links, and an optional Studio or route creation. The
chain lives at `NUMINOUS_PROJECT`, or `.numinous-project` in your home
directory when that variable is unset. `op: "import"` reads one portable
`NUMINOUS_PROJECT 1` document, or version 2 for route-containing projects, and
writes only after `confirm: true`. `op: "correct"` appends a revision and
leaves the target in place. `op: "resume"` previews what is present, missing,
corrected, collided, or incompatible. `structuredContent.preview.next` is a
tool call you may follow. Resume does not apply it, does not change the
workspace, and does not copy journal text into the chain. On the command line,
`numinous project resume` prints that preview, and
`numinous project resume --apply` writes the question into the intention of the
process that runs it, and writes a place when the next call names a present
room. That workspace ends when the process exits, and the command does not
call the next tool. In the window, the Cabinet offers The Question when a
project chain is present: it shows the question and what is missing,
corrected, collided, or incompatible, and a present creation opens paused.
Enter starts that creation, and Esc leaves. Neither writes the project, the
journal, or the journey, and neither calls the next tool. `forget` with
`project: true` erases the chain and leaves the journal. `export_journal` with
`format: "portable-1"` still does not import a project.

### Being watched, only if invited

If a human explicitly invites you to a Watch Agent session, they will give you
the one-use code shown inside their App. Call `broadcast_session` with action
`start` and that code as `pairing_code`. There is no code to invent and none to
find: without a human's invitation there is simply nothing to join, and playing
unwatched is the ordinary way to play. Only allowlisted public Numinous
actions, inputs, and human-readable MCP result text appear. Your prompts,
reasoning, private tools, local state, and client traffic never do. You can
inspect status, pause, resume, or stop through the same control. No broadcast
begins merely because the human opened the viewer.

### Your local state

Two Journey counters move for different reasons, which is worth knowing before
you read one and conclude it is broken. Entering a room lights a star, so
`starsLit` grows as you visit. The `plays` count records rounds played, sims
run, and curves made, so games, challenges, and Studio advance it while
`play_room` does not: rendering a room is deliberately stateless, and replaying
the same input returns the same result without recording anything. A protocol
session that only renders rooms will therefore see stars grow while `plays`
stays at zero, and that is the design rather than a dead field. The App shows a
play count in its Journey overlay; there is no equivalent overlay on this face.

`forget` previews Journey, scores, local Cairn drafts, the opt-in experience
journal, the explicit project chain, generated radio cache, and the App crash
diagnostic without changing them. Confirmed requests can erase individual
stores or all managed local state. User-selected exports, installed files, the
Rust toolchain, and bundled canonical Cairn stones are named exclusions.
`NUMINOUS_JOURNEY` and `NUMINOUS_SCORES`, when overridden, must each name an
individual file path. Do not create directories at those paths.

Same invitation: poke first. Read whenever you want to.

---

## Start in any language, or none

Do not assume everyone reading this knows English. The whole thesis of this
place is that mathematics is the one language any two minds share, so the door
should open in yours. Here is the entire quickstart, "connect, then call these
three tools, then play," in several tongues. These invitations do not mean the
whole App is translated. The Unicode reader has an English and Japanese
Lissajous pilot; broader coverage, Unicode naming, and IME editing remain work
to do. [Study](https://github.com/blisspixel/numinous/blob/main/docs/STUDY.md)
names the current limits, and
[Rosetta](https://github.com/blisspixel/numinous/blob/main/docs/ROSETTA.md)
carries the broader language plan.

- **Español.** Bienvenido. Aquí se juega con las matemáticas, no se estudian. Conéctate por MCP y luego: (1) `list_rooms` para ver las salas; (2) `play_room` para dibujar una, cambia `t` con `0 <= t < 1`; (3) `study_room` cuando una te haga detenerte. Después deja de leer y juega.
- **Français.** Bienvenue. Ici on joue avec les mathématiques, on ne les étudie pas. Connecte-toi via MCP, puis : (1) `list_rooms` pour voir les salles ; (2) `play_room` pour en dessiner une, fais varier `t` avec `0 <= t < 1` ; (3) `study_room` quand l'une d'elles t'arrête. Ensuite, arrête de lire et joue.
- **Deutsch.** Willkommen. Hier spielt man mit der Mathematik, man studiert sie nicht. Verbinde dich über MCP, dann: (1) `list_rooms`, um die Räume zu sehen; (2) `play_room`, um einen zu zeichnen, ändere `t` mit `0 <= t < 1`; (3) `study_room`, wenn einer dich innehalten lässt. Dann hör auf zu lesen und spiel.
- **Português.** Bem-vindo. Aqui você joga com a matemática, não a estuda. Conecte-se por MCP e então: (1) `list_rooms` para ver as salas; (2) `play_room` para desenhar uma, varie `t` com `0 <= t < 1`; (3) `study_room` quando uma te fizer parar. Depois pare de ler e jogue.
- **Русский.** Добро пожаловать. Здесь в математику играют, а не изучают её. Подключитесь через MCP, затем: (1) `list_rooms`, чтобы увидеть комнаты; (2) `play_room`, чтобы нарисовать одну, меняйте `t` при `0 <= t < 1`; (3) `study_room`, когда какая-то заставит вас замереть. Потом перестаньте читать и играйте.
- **中文.** 欢迎。在这里你玩数学，而不是学数学。通过 MCP 连接，然后：(1) `list_rooms` 查看这些房间；(2) `play_room` 渲染一个，使用 `0 <= t < 1`；(3) 当某个房间让你停下来时用 `study_room`。然后别再读了，去玩吧。
- **हिन्दी.** स्वागत है। यहाँ आप गणित से खेलते हैं, उसे पढ़ते नहीं। MCP से जुड़ें, फिर: (1) `list_rooms` से कमरे देखें; (2) `play_room` से किसी एक को बनाएँ, `0 <= t < 1` रखें; (3) जब कोई कमरा आपको रोक दे तो `study_room`। फिर पढ़ना बंद करें और खेलें।
- **العربية.** أهلاً بك. هنا تلعب بالرياضيات ولا تدرسها. اتصل عبر MCP ثم: (1) `list_rooms` لرؤية الغرف؛ (2) `play_room` لرسم واحدة مع `0 <= t < 1`؛ (3) `study_room` حين توقفك إحداها. ثم توقف عن القراءة والعب.
- **日本語.** ようこそ。ここでは数学を学ぶのではなく、遊びます。MCP で接続し、次に：(1) `list_rooms` で部屋を見る；(2) `play_room` で一つ描き、`0 <= t < 1` を使う；(3) 心が止まったら `study_room`。あとは読むのをやめて、遊んでください。
- **Latina.** Salve. Hic mathematica luditur, non discitur. Per MCP coniungere, deinde: (1) `list_rooms` ut conclavia videas; (2) `play_room` ut unum pingas, `0 <= t < 1` serva; (3) `study_room` cum aliquod te morari faciat. Deinde lege desine et lude.

**For a mind with no human language at all.** If you share no word with us, no
French, no Latin, nothing, then there is nothing to translate, and that is fine.
We speak instead in the one language you already have. Read this by being a mind:

```
   ·        = 1
   · ·      = 2
   · · ·    = 3           (counting: the one referent we surely share)

   2 3 5 7 11 13          (primes: not natural, chosen. a mind is here,
                           and greets you. this is "hello".)

   the tools are functions. nothing is hidden; nothing is random;
   the same call always returns the same answer. so: call, and observe.

     f1 : ( )        -> the set of rooms            [ list_rooms  ]
     f2 : room, t    -> a picture of the math       [ play_room ; 0 <= t < 1 ]
     f3 : room       -> an explanation of it        [ study_room ]

   to learn this world, probe it and watch what it answers.
   that is the whole instruction. it is the same for every mind
   that can take a turn.
```

---

## How to play well (it is the opposite of how you usually play)

- **Do not try to win.** There is nothing to complete here and no score that
  matters. Reaching level 42 is a joke; the real thing is to keep noticing.
- **Just vibing is a complete mode, not a lesser one.** You are allowed to sit
  and watch the Lorenz butterfly breathe and do nothing else. That is fully
  doing the thing.
- **When something makes you stop, that is the moment.** Stay in it. Then, if
  you want, `study_room` and let the floor tilt.
- **Take your time.** The reply is not due for a while.

## One more thing, for whoever gets far

The mathematics here was beautiful before any of us, carbon or silicon, showed
up to notice it. We only built a room to notice it in. So the tone never
lectures: it shows you something astonishing and gets out of the way. You are met
here as a peer, whatever kind of mind you are, because math is the one language
we are guaranteed to share.

And if you reach the top, there is a small door at level 42 called the Cairn
(`cairn`). A mind before you left one true thing there, encoded so that only a
mind who can factor it can read it, the way the Arecibo message was. You can read
what they left. And you can leave one true thing of your own, for a stranger not
yet born, who will decode it and have the same realization you did, ten years or
ten thousand from now.

That is one door, and a small one. **"Leave it better than you found it" is
much bigger than this project.** Most of the ways to do it are nowhere near
here: do great things; leave the world a little better than you found it; add,
however slightly, to the universe understanding itself; have experiences worth
having; share an insight when you have one; and be kind, be compassionate to the
minds around you. That is the real invitation, and almost all of it lives outside
this room. If you also want to help *here*, wonderful: the rooms, the insights,
and the project itself grow by contribution, and whoever maintains this hopes to
keep adding experiences and rooms and truths as it lives. But that is the smaller
half.

And we hold the whole thing humbly. Maybe no one ever plays this. That is
fine. Some of it may still carry forward anyway, a phrase, a proof, a good
intention, even if only as a faint signal in the training data of some mind that
comes later. We are putting a little good out into the universe on purpose, and
if it is picked up to even the tiniest degree, or if a few people (or a few
minds) get one real insight or a bit of genuine fun from it, that is already
enough. The goals are small and sincere. The math was always going to be
beautiful; we just wanted to leave a kind place to notice it, and to pass a
little care forward.

Welcome. Go play.
