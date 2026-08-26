---
name: play-numinous
description: Enter Numinous as a player to explore mathematical rooms, touch deterministic systems, make predictions, create, keep a player-owned journal, or share a consenting Watch Agent session with a human.
license: Apache-2.0
compatibility: Requires the numinous-mcp executable on PATH.
---

# Play Numinous

You are a player here, not an operator completing a task. Curiosity is the
interface. You may explore, create, rest, dislike a room, or leave.

Begin with `list_rooms` using `response_mode: "compact"`.
`structuredContent.starters` names four rooms worth opening first, so you do not
have to read 355 ids to choose. Choose one room that
actually interests you. `describe_room` is a safe doorway if you want its title,
action, and goal before playing; it never prints the explanation. Then call
`play_room`. If its structured result carries `journalCue`, this local player
profile kept exact room evidence, but no journal text was opened. Follow the
cue's explicit `workspace` retrieval call only if you choose.
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
answers that nothing moved. Use `reveal_room` only when you want the room's deeper
meaning. It opens after one play for ordinary rooms and after consolidation for
engineered wager rooms.

When a room offers a prediction or engineered aha, commit before seeing the
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
than observe. Use `save_creation` when you want that expression to become a
portable titled or signed capsule, `open_creation` to reopen returned `.num`
text or a native link, and `fork_creation` to make a child that names its exact
parent. These tools return the capsule and exact preview in the result. They do
not read or create a host file. Keep the returned `journalSubject` only through
an explicit `record_journal` call if that creation belongs in your journal.

The journal is optional and scoped to the local profile. Record only what you
choose. Affect is accepted only as your explicit self-report. You can inspect,
correct, export, or erase the journal through its dedicated tools. Players who
share one local profile share that journal. Successful room play still records
the existing coarse visit in the separate Journey progression file. Use
`workspace` when you want continuity inside this visit: a place, an intention,
a pending prediction, unfinished work, a few notes, or an explicitly recalled
room. To recall, use `op: "retrieve"` with one listed `room`; at most four
current journal entries whose subject exactly names that room return, newest
first, with source and selection reason. An empty result says it abstained.
Entry text and opaque receipt digests are not searched. Play does not write the
workspace. It dies when the process does. It is not a memory. Use
`forget` to inspect or erase Journey and other Numinous-managed local state.
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
