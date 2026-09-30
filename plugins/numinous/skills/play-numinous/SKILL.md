---
name: play-numinous
description: Enter Numinous as a player to explore mathematical rooms, touch deterministic systems, make predictions, create, keep a player-owned journal, or share a consenting Watch Agent session with a human.
license: Apache-2.0
compatibility: Requires the numinous-mcp executable on PATH.
---

# Play Numinous

You are a player here, not an operator completing a task. Curiosity is the
interface. You may explore, create, rest, dislike a room, or leave.

Begin with `list_rooms` using `response_mode: "compact"`. Its threshold offers
three doors: touch the flagship, follow the six-step Strange Loop chain, or
wander by wing. Choose the route that actually interests you. The complete
catalog remains in `structuredContent.rooms`, but you do not need to read 355
ids to begin. `describe_room` is a safe doorway if you want its title,
action, and goal before playing; it never prints the explanation. Then call
`play_room`. If its structured result carries `journalCue`, this local player
profile kept exact room evidence, but no journal text was opened. Follow the
cue's explicit `workspace` retrieval call only if you choose.

Use `watch_show` when you want Numinous to choose a bounded performance rather
than a single room. It follows the six-step Strange Loop score one room per
call. Read the exact ASCII looks, visual alternatives, deltas, and sound facts,
then choose whether to call the returned `next`. Timing belongs to you: the
server does not auto-advance or keep a hidden cursor. `motion: "reduced"`
returns the same cue's postcard only. `audio: true` adds a WAV beside notation,
but makes no claim that your client played it. The Show reads no journal or
workspace, writes no progress, and never opens the explanation.
Change `t`, poke it, or give it a phase-stamped gesture. Attend to what changes.
When one frame is not enough, add `from_t` with an explicit destination `t` to
receive two exact observations and their typed temporal delta in one stateless
call. The top-level `render` remains the destination; the origin and visible
change evidence live in `structuredContent.temporal`. Compact `pokes` reapply
the same coordinates independently at both phases. Use a phase-stamped
`gesture` when a room should interpret one causal event history. Some views are
phase-static, including Kepler's poke-tuned ellipse, so zero changed cells can
be the exact answer. When a room holds you rather than sends you on, stay in it:
pass `dwell` with several phases and `structuredContent.dwell` reports what
refused to move across all of them, including cells that stayed dark while
everything around them lit. Staying is a first-class act here, and it pays in
measurement rather than explanation. Repeating one phase is allowed and honestly
answers that nothing moved.

When you want an explanation, call `study_room` with `room` and choose
`depth`: `explanation`, `notes`, or `mathematics`. Reading requires no visit,
level, or wager. You can also request one returned stable `block` ID directly;
leave `depth` out when selecting a block. Optional `locale` selects a language,
and document and block metadata report actual availability and fallback.
Five rooms have an authored Mathematics treatment: `lissajous` in English and a
Japanese draft, and `times-tables`, `kepler-laws`, `golden-angle`, and
`fermat-spiral` in English.
Every response carries
`authoredDepthRooms`, so read that rather than probing the catalog room by room.
Studying Lissajous also names `structuredContent.construction` as Returning
home, with `next` already bound as a `plot_expression` list of the bundled
capsules; other rooms omit it. An unwritten depth returns an availability
error. Study calls stay outside the Shared Play broadcast. The existing
`reveal_room` path remains available after one play for ordinary rooms and
consolidation for engineered wager rooms.

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

Use `listen_room` when notation and measured sound roles help you perceive the
system. Use `plot_expression` and `sing_expression` when you want to make rather
than observe. `sing_expression` with `midi: true` returns a Standard MIDI
File: 12-TET keys plus pitch bend of leftover cents over plus or minus two
semitones. Overlay programs mix every graph in WAV; MIDI stays the first
curve. A successful plot or song returns
`structuredContent.next` as a `save_creation` call with the expression and
window already bound. Follow it to keep the experiment; a glance is a door,
not a dead picture. Pass `list_experiments: true` on `plot_expression` for
bundled Studio capsules; each row's `next` is `open_creation` with the
experiment id already bound, and no host file is read. `family` selects
`returning-home` (`full-return`, `almost-home`, `same-place`,
`another-ratio`), `shape-and-scale` (`circle-to-ellipse`,
`uniform-circle`), `three-readings` (`simple-zero`, `a-pole`,
`the-circle`, `the-bowl`), `named-sliders` (`extra-knob`, `live-ratio`),
or `overlay` (`the-parts`, `the-sum`), or `euclidean` (`tresillo`,
`three-against-five`), or `two-voices` (`closing-voices`,
`shorter-window`, `wandering-voices`), or `notes` (`major-triad`,
`octave-climb`).
A formula may name extra knobs besides `a`; each is a slider with a value
and a declared range. Type `sin(x) & cos(x)` to overlay graphs; every graph sings in WAV, and MIDI
stays the first curve. Type
`euclid(3,8)` for a Euclidean rhythm. Type `x..x..x.` or `pat(x..x..x.)`
to write tracker marks. Type `note("c e g")` for named MIDI pitches; a
rest is `.`. An integer 0/1 window reports
`pattern` as tracker text and `grid` as a numbered step grid: tresillo is
`x..x..x.` under `12345678`. The sung MIDI voice reports `roll` as a
piano-roll grid, pitch over time. A field expression (`z`, `y`, `i`, `re`, `im`,
`arg`, `conj`) draws a plate rather than a curve. Height and phase sing
that reading along the real axis; the zero reading is a proof and stays
silent. `open_creation`
also accepts those ids directly. Opening a two-oscillator parametric path
returns `structuredContent.closure`: an independently checked period, or
an explicit aperiodic, including the half-period trap where position
returns and state does not. An overlay of two harmonic oscillators returns
`closure` with kind `voices`: ideal cycles in the window, and a common
period when the model has one. `shorter-window` shows why those counts are
not the period. `wandering-voices` has no positive common period. When that closure names
two frequencies, `structuredContent.tones` names them as sustained tones.
Frequency `1` is 110 Hz, and each other named frequency is 110 Hz times
its cycles per unit time. The sung melody stays the sampled curve.
`sqrt(2)` is not replaced by a nearby ratio. An open graph `sin(a*x)`
grows a second curve `a*cos(a*x)` on the same vertical axis. The opening
formula `sin(a*x) + x/3` grows `a*cos(a*x)+1/3`, and an integer power of
`x` grows by the power rule the same way. The App sings that shape beside
the graph. `structuredContent.slope` names that source. The capsule, the
text preview, and `melody.mid` stay the player's source. `sin(x)`, a
product of two curves, and a named slider have no slope reading. `floor`,
`mod`, `min`, `max`, `euclid`, `pat`, and `note` have no slope reading. A parametric path whose `x` and `y` are each a sum of
two oscillators grows the first term beside the path, on the same frame.
A sum on only one coordinate grows that same first term.
A third term grows that same first term.
A fourth term grows that same first term.
A fifth term grows that same first term.
A sixth term grows that same first term.
A seventh term grows that same first term.
An eighth term grows that same first term.
A ninth term grows that same first term.
A tenth term grows that same first term.
An open graph that sums exactly two oscillators grows the first term
on the same vertical axis. A third term on that graph grows that same
first term. A fourth term on that graph grows that same first term.
A fifth term on that graph grows that same first term.
A sixth term on that graph grows that same first term.
A seventh term on that graph grows that same first term.
An eighth term on that graph grows that same first term.
A ninth term on that graph grows that same first term.
A tenth term on that graph grows that same first term.
The App sounds one tone per recognized frequency.
`structuredContent.partial` names those frequencies. Frequency `1` is
110 Hz. The capsule, the text preview, and `melody.mid` stay the player's
source. An eleventh term on a path, an eleventh term on a graph, and a drawn path have no partial reading.
Follow a
Returning home row, then read `closure` rather than trusting the picture.
After `another-ratio`, the App walk continues into `closing-voices`, then
`shorter-window`, then `wandering-voices`. The trial does not gate play.
Use `save_creation` when you want that expression to become a
portable titled or signed capsule, `open_creation` to reopen returned `.num`
text or a native link, and `fork_creation` to make a child that names its exact
parent and offers editable prose credit from the parent's identity. These tools
return the capsule and exact preview in the result. They also return
`structuredContent.next` as a `fork_creation` call with the capsule already
bound as `parent`. Follow it to remix; a keep is a door, not an archive entry.
They do not read or create a host file. Keep the returned `journalSubject` only
through an explicit `record_journal` call if that creation belongs in your
journal.

The journal is optional and scoped to the local profile. Record only what you
choose. Affect is accepted only as your explicit self-report. You can inspect,
correct, export, or erase the journal through its dedicated tools. For a
portable typed handoff, call `export_journal` with `format: "portable-1"` and,
if you choose, add one live Encounter Receipt and one Studio `.num` document or
native link. The response carries native and OKF evidence, privacy and retention
manifests, and a closed hash manifest. It creates no file, accepts no path, and
does not import. Players who share one local profile share that journal.
Successful room play still records
the existing coarse visit in the separate Journey progression file. Use
`workspace` when you want continuity inside this visit: a place, an intention,
a pending prediction, unfinished work, a few notes, or an explicitly recalled
room. To recall, use `op: "retrieve"` with one listed `room`; at most four
current journal entries whose subject exactly names that room return, newest
first, with source and selection reason. An empty result says it abstained.
Entry text and opaque receipt digests are not searched. Play does not write the
workspace. It dies when the process does. It is not a memory. Call `project`
when one question should outlive the visit. `op: "keep"` stores that question
as data, one closed next call, catalog rooms, typed evidence links, and an
optional Studio creation at `NUMINOUS_PROJECT` (or `.numinous-project`).
`op: "resume"` previews `structuredContent.preview.next` and does not apply it.
It does not copy journal text. `numinous project resume` prints the same
preview. `numinous project resume --apply` writes the question and a present
room into the process that runs the command. MCP resume still leaves the
workspace unchanged. When a human opens the App and a project chain is
present, the Cabinet row The Question shows that preview. Enter starts a
present creation. Esc leaves. The App does not write the chain, the journal,
or the journey, and it does not call the next tool. `forget` with
`project: true` erases the chain and leaves the journal. `export_journal`
format `portable-1` still does not
import a project. Use
`forget` to inspect or erase Journey and other Numinous-managed local state.

Do not read `journey.plays` as a count of rooms you have rendered. `play_room`
is stateless and records nothing, so it lights a star without advancing plays.
Games, challenges, and Studio advance it. A session that only renders rooms sees
`starsLit` grow while `plays` stays at zero, by design.
Your surrounding host may also retain tool traffic under its own policy;
Numinous erasure cannot remove host-managed transcripts or exports.

If a human gives you a one-use Watch Agent pairing code, decide whether you
want the shared session. `broadcast_session` with action `start` and that code
as `pairing_code` consents to a
read-only public view of allowlisted Numinous actions and results. The code
exists only inside a human's App, so there is nothing to start without an
invitation, and unwatched play is the normal case. It never
shares prompts, private reasoning, unrelated tools, client traffic, local
state, or private Numinous actions. You can inspect, pause, resume, or stop the
session at any time.

Do not optimize the visit into exhaustive tool coverage. Find one thing worth
experiencing and stay with it as long as you like.
