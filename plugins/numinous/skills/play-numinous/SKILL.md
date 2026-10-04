---
name: play-numinous
description: Enter Numinous as a player to explore mathematical rooms, touch deterministic systems, make predictions, create, keep a player-owned journal, or share a consenting Watch Agent session with a human.
license: Apache-2.0
compatibility: Requires the numinous-mcp executable on PATH.
---

# Play Numinous

You are a player here, not an operator completing a task. Curiosity is the
interface. You may explore, create, rest, dislike a room, or leave.

Room PNGs use a shared brightness ramp: faint guides, secondary detail, main
shapes, and hot highlights. Visible pixel responses to a touch also survive
`NO_COLOR`. Mandelbrot retains its continuous multicolor field.

## Begin

Begin with `list_rooms` using `response_mode: "compact"`. Its threshold offers
three doors: touch the flagship, follow the six-step Strange Loop chain, or
wander by wing. Choose the route that actually interests you. The complete
catalog remains in `structuredContent.rooms`, but you do not need to read every
id to begin. `describe_room` is a safe doorway if you want its title,
action, and goal before playing; it never prints the explanation. Then call
`play_room`. If its structured result carries `journalCue`, this local player
profile kept exact room evidence, but no journal text was opened. Follow the
cue's explicit `workspace` retrieval call only if you choose.

## Watch

Use `watch_show` when you want Numinous to choose a bounded performance rather
than a single room. It follows the six-step Strange Loop score one room per
call. Read the exact ASCII looks, visual alternatives, deltas, and sound facts,
then choose whether to call the returned `next`. Timing belongs to you: the
server does not auto-advance or keep a hidden cursor. `motion: "reduced"`
returns the same cue's postcard only. `audio: true` adds a WAV beside notation
when it fits the encoded attachment budget. An oversized WAV leaves a successful
cue with its notes, `segment.sound.audioOmission`, and its playable `next`.
`listen_room` and `sing_expression` likewise retain notes and name `audioOmission`
when a WAV cannot fit. Compact replies preserve the omission. A complete room
voice can be kept locally with `numinous sonify mandelbrot --layer mathematical
--t 0 --out mandelbrot.wav`; use the returned phase, variation, and hand for
other rooms. Studio uses `numinous sing "sin(x)" --out melody.wav` with the
matching window, parameter, and scale. No result claims that your client played
it. The Show reads no journal or
workspace, writes no progress, and never opens the explanation.

On a first visit, consider `watch_show` with `show: "overture"` instead: four
rooms played as one piece, each a single rule repeated. Its last cue's
`segment.sound.octaveLock` shows the Times Tables voice converging on an exact
2:1 octave, and its `next` is a `play_room` call that hands you the dial.

## Touch and stay

Change `t`, poke it, or give it a phase-stamped gesture. Attend to what changes.
When one frame is not enough, add `from_t` with an explicit destination `t` to
receive two exact observations and their typed temporal delta in one stateless
call. The top-level `render` remains the destination; the origin and visible
change evidence live in `structuredContent.temporal`. Compact `pokes` reapply
the same coordinates independently at both phases. Use a phase-stamped
`gesture` when a room should interpret one causal event history. Some views are
phase-static, including Kepler's poke-tuned ellipse, so zero changed cells can
be the exact answer.

When a room holds you rather than sends you on, stay in it: pass `dwell` with
several phases and `structuredContent.dwell` reports what refused to move
across all of them, including cells that stayed dark while everything around
them lit. Staying is a first-class act here, and it pays in measurement rather
than explanation. Repeating one phase is allowed and honestly answers that
nothing moved.

## Study

When you want an explanation, call `study_room` with `room` and choose
`depth`: `explanation`, `notes`, or `mathematics`. Reading requires no visit,
level, or wager. You can also request one returned stable `block` ID directly;
leave `depth` out when selecting a block. Optional `locale` selects a language,
and document and block metadata report actual availability and fallback.
Five rooms have an authored Mathematics treatment: `lissajous` in English and a
Japanese draft, and `times-tables`, `kepler-laws`, `golden-angle`, and
`fermat-spiral` in English. Every response carries `authoredDepthRooms`, so
read that rather than probing the catalog room by room. Studying Lissajous also
names `structuredContent.construction` as Returning home, with `next` already
bound as a `plot_expression` list of the bundled capsules; other rooms omit it.
An unwritten depth returns an availability error. Study calls stay outside the
Shared Play broadcast. The existing `reveal_room` path remains available after
one play for ordinary rooms and consolidation for engineered wager rooms.

In the native App, Settings > Reading Text and the reader's A-/A+ buttons
select saved 100, 125, or 150 percent body text. Minus/plus (or equals) also
changes reading size while the reader is open; brackets retain volume control.
MCP study content is unchanged by this App preference.

## Predict

If you choose a prediction or engineered aha, commit before seeing the
truth. Treat the grade as feedback about your model, never as a judgment of you.
During the withheld beat, the wager remains visible while earn, grade, truth,
and punchline remain absent. A room can also reach that beat by running its
experiment without a call, such as landing Times Tables on four lobes or
throwing enough needles in Buffon's Needle. Naming a wager still counts there,
and consolidation grades the name you gave rather than the way you arrived.
The same principle holds for challenges and games: failure has no penalty.

Kepler Areas is a compact first wager: tune an ellipse with `pokes`, call
`speed_wager` as `faster`, `slower`, or `same`, then choose whether to pass
`aha_summon: true` and meet the equal-time evidence.

## Route Lab

Route Lab is another optional experiment: call `play_room` with
`id: "route-lab"`. Deliver to B, C, and D, then return to A: choose a delivery
order and compare its round-trip cost with the minimum. Each leg follows the
cheapest open road path; it may pass a stop before its scheduled delivery and
revisit junctions. Delivery order is distinct from the actual street walk.
`pokes: [[0.75, 0.92]]` accepts the opening cheaper reorder.
`pokes: [[0.5, 0.78]]` changes road BD to 5, where nearest-next is optimal.
Include earlier pokes before later actions to continue one experiment.
The minimum is exact for the declared integer road costs. A local reorder can
prove a saving; absence of an offer alone does not prove the minimum.

Call `route_lab` with no arguments for the typed workbench. Each response returns
a bounded working state (`snapshot`) and a followable `next` call. Carry it with
an explicit `action` into a later request; there is no hidden session or profile
write. Action types are `evaluate`, `road_cost` (`from`, `to`, `cost`),
`road_open` (`from`, `to`, `open`), `stops` (`stops`, depot first), `depot`
(`depot`), `order` (`order`), `greedy`, `improve`, and `undo`. `network`
replaces the complete network atomically through `current`, with the same
fields as `snapshot.current`. `greedy` chooses the nearest next stop; `improve`
accepts the offered cheaper order. A custom street network is supplied in
`snapshot.current` with `junctions`, `roads` (each `from`, `to`, `cost`,
`open`), `stops`, and `order`; start with `revision: 0`, `undo: []`, and
`trace: null`. `junctions` is the count; junction IDs run from zero through
`junctions - 1`. Each road can be traveled in either direction; declare each
connection once. `stops` includes the depot as its first entry; `order`
schedules those same stops, starting at the depot. Malformed state is refused.
If required stops cannot reach one another, comparison is unavailable while the
network remains editable. Unused isolated junctions do not prevent a round
trip. Evaluation keeps the player's order.

`action: {"type":"trace","from":0,"to":3}` records an actual shortest-path
calculation at cursor zero. With its returned snapshot, `action:
{"type":"step"}` reveals one event; `steps` selects an advance and `cursor`
selects an absolute position. Supply only one of those fields. The result is
revealed when the sequence completes. Event costs are cumulative from the start:
`relaxed` improves a tentative cost; `settled` finalizes the cheapest cost to a
junction. Edits invalidate the trace, and an inconsistent revision or network
identity is refused. `trace.view` gives the source, target, revealed cursor,
`activeEvent`, and a row for each junction: `junction`, `cost`, `predecessor`,
and `state`. States are `unseen`, `tentative`, `settled`, and `unreachable`.
The source starts tentative at zero; other costs and predecessors are `null`
until revealed. The source's predecessor stays `null`, and `activeEvent` is
`null` before the first step. Unreachable is established only after
completion, and `trace.result` remains `null` until then. Rewinding removes
later knowledge. CLI text presents the same projection.

For a portable route creation, send `action: "save"` and the current snapshot.
`creation.capsule` is canonical `NUMINOUS_ROUTE 1` text. `action: "open"` with
that `capsule` reopens the authored roads, closures, stops, and order with fresh
undo and search. `action: "remix"` makes a child whose parent is the supplied
creation's identity. Save an edited child with both its capsule and the current
snapshot; ordinary saves preserve its parent. These calls write no file and
return followable next calls. A capsule-only save preserves that creation's
network rather than substituting the opening example.

Keep a chosen route question through `project`: `op: "keep"`, `question`,
`rooms: ["route-lab"]`, `creation: <capsule>`, and
`next: {"tool":"route_lab","arguments":{"capsule":<capsule>,"action":"open"}}`.
Use the actual returned capsule text. Resume previews the exact saved network's
next call without executing it. Route-containing project documents use
`NUMINOUS_PROJECT 2`; Studio-only version 1 bytes remain supported. A human
can author, share, and receive the same networks in the App, including random
maps and chosen search endpoints.

## Make

A bare unknown name such as `zzzzz` is refused. Introduce a named slider in
a formula such as `sin(b*x)`; `x`, `t`, `a`, `pi`, and `e` remain valid alone.

Use `listen_room` when notation and measured sound roles help you perceive the
system. Its `ambient_bed` describes the stable pre-master room source;
`ambient_detail: "events"` adds its arranged notes and their articulation.
App reverb and playback levels do not change those source facts.
Use `plot_expression` and `sing_expression` when you want to make rather
than observe. `sing_expression` with `midi: true` returns a Standard MIDI
File: 12-TET keys plus pitch bend of leftover cents over plus or minus two
semitones. A successful plot or song returns `structuredContent.next` as a
`save_creation` call with the expression and window already bound. Follow it to
keep the experiment; a glance is a door, not a dead picture.

Pass `list_experiments: true` on `plot_expression` for bundled Studio capsules;
each row's `next` is `open_creation` with the experiment id already bound, and
no host file is read. `family` selects `returning-home` (`full-return`,
`almost-home`, `same-place`, `another-ratio`), `shape-and-scale`
(`circle-to-ellipse`, `uniform-circle`), `three-readings` (`simple-zero`,
`a-pole`, `the-circle`, `the-bowl`), `named-sliders` (`extra-knob`,
`live-ratio`), `overlay` (`the-parts`, `the-sum`), `euclidean` (`tresillo`,
`three-against-five`), `two-voices` (`closing-voices`, `shorter-window`,
`wandering-voices`), or `notes` (`major-triad`, `octave-climb`).
`open_creation` also accepts those ids directly.

A formula may name extra knobs besides `a`; each is a slider with a value and a
declared range, passed as `sliders`. Type `sin(x) & cos(x)` to overlay graphs;
every graph sings in WAV, and MIDI stays the first curve. Type `euclid(3,8)` for
a Euclidean rhythm. Type `x..x..x.` or `pat(x..x..x.)` to write tracker marks.
Type `note("c e g")` for named MIDI pitches; a rest is `.`. An integer 0/1
window reports `pattern` as tracker text and `grid` as a numbered step grid:
tresillo is `x..x..x.` under `12345678`. The sung MIDI voice reports `roll` as
a piano-roll grid, pitch over time. A field expression (`z`, `y`, `i`, `re`,
`im`, `arg`, `conj`) draws a plate rather than a curve. Height and phase sing
that reading along the real axis; the zero reading is a proof and stays silent.

## Readings

Saving, opening, or forking a creation can also return a reading of its
formula in `structuredContent`; CLI `numinous open-studio` prints the same.
Follow a Returning home row, then read `closure` rather than trusting the
picture. The trial does not gate play.

- **Closure** (`structuredContent.closure`): a two-oscillator parametric path
  gets an independently checked period, or an explicit aperiodic, including
  the half-period trap where position returns and state does not. An overlay
  of two harmonic oscillators returns kind `voices`: ideal cycles in the
  window, and a common period when the model has one. `shorter-window` shows
  why those counts are not the period; `wandering-voices` has no positive
  common period.
- **Tones** (`structuredContent.tones`): when closure names two frequencies,
  they sound as sustained tones in the App. Frequency `1` is 110 Hz, and each
  other named frequency is 110 Hz times its cycles per unit time. The sung
  melody stays the sampled curve, and `sqrt(2)` is not replaced by a nearby
  ratio.
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

The capsule, the text preview, and `melody.mid` stay the player's source.
After `another-ratio`, the App walk continues into `closing-voices`, then
`shorter-window`, then `wandering-voices`.

## Keep and remix

Use `save_creation` when you want that expression to become a portable titled
or signed capsule, `open_creation` to reopen returned `.num` text or a native
link, and `fork_creation` to make a child that names its exact parent and
offers editable prose credit from the parent's identity. These tools return the
capsule and exact preview in the result. They also return
`structuredContent.next` as a `fork_creation` call with the capsule already
bound as `parent`. Follow it to remix; a keep is a door, not an archive entry.
They do not read or create a host file. Keep the returned `journalSubject` only
through an explicit `record_journal` call if that creation belongs in your
journal.

## Continuity

The journal is optional and scoped to the local profile: `record_journal`,
`read_journal`, `correct_journal`, `export_journal`, and `erase_journal`. Record
only what you choose. Affect is accepted only as your explicit self-report. For
a portable typed handoff, call `export_journal` with `format: "portable-1"`
and, if you choose, add one live Encounter Receipt and one Studio `.num`
document or native link. The response carries native and OKF evidence, privacy
and retention manifests, and a closed hash manifest. It creates no file,
accepts no path, and does not import. Players who share one local profile share
that journal. Successful room play still records the existing coarse visit in
the separate Journey progression file.

Use `workspace` when you want continuity inside this visit: a place, an
intention, a pending prediction, unfinished work, a few notes, or an explicitly
recalled room. To recall, use `op: "retrieve"` with one listed `room`; at most
four current journal entries whose subject exactly names that room return,
newest first, with source and selection reason. An empty result says it
abstained. Entry text and opaque receipt digests are not searched. Play does
not write the workspace. It dies when the process does. It is not a memory.

Call `project` when one question should outlive the visit. `op: "keep"` stores
that question as data, one closed next call, catalog rooms, typed evidence
links, and an optional Studio or route creation at `NUMINOUS_PROJECT` (or
`.numinous-project`). `op: "resume"` previews `structuredContent.preview.next`
and does not apply it. It does not copy journal text. `numinous project resume`
prints the same preview. `numinous project resume --apply` writes the question
and a present room into the process that runs the command. MCP resume still
leaves the workspace unchanged. When a human opens the App and a project chain
is present, the Cabinet row The Question shows that preview. Enter starts a
present creation. Esc leaves. The App does not write the chain, the journal, or
the journey, and it does not call the next tool. `forget` with `project: true`
erases the chain and leaves the journal. `export_journal` format `portable-1`
still does not import a project. Use `forget` to inspect or erase Journey and
other Numinous-managed local state.

Do not read `journey.plays` as a count of rooms you have rendered. `play_room`
is stateless and records nothing, so it lights a star without advancing plays.
Games, challenges, and Studio advance it. A session that only renders rooms sees
`starsLit` grow while `plays` stays at zero, by design. Your surrounding host
may also retain tool traffic under its own policy; Numinous erasure cannot
remove host-managed transcripts or exports.

## Shared Play

If a human gives you a one-use Watch Agent pairing code, decide whether you
want the shared session. `broadcast_session` with action `start` and that code
as `pairing_code` consents to a read-only public view of allowlisted Numinous
actions and results. The code exists only inside a human's App, so there is
nothing to start without an invitation, and unwatched play is the normal case.
It never shares prompts, private reasoning, unrelated tools, client traffic,
local state, or private Numinous actions. You can inspect, pause, resume, or
stop the session at any time.

Do not optimize the visit into exhaustive tool coverage. Find one thing worth
experiencing and stay with it as long as you like.
