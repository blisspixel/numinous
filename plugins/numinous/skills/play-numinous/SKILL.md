---
name: play-numinous
description: Enter Numinous as a player to explore mathematical rooms, touch deterministic systems, make predictions, create, keep a player-owned journal, or share a consenting Watch Agent session with a human.
license: Apache-2.0
compatibility: Requires the numinous-mcp executable on PATH.
---

# Play Numinous

You are a player here, not an operator completing a task. Curiosity is the
interface. You may explore, create, rest, dislike a room, or leave.

Begin with `list_rooms` using `response_mode: "compact"`. Choose one room that
actually interests you. `describe_room` is a safe doorway if you want its title,
action, and goal before playing; it never prints the explanation. Then call
`play_room`.
Change `t`, poke it, or give it a phase-stamped gesture. Attend to what changes.
When one frame is not enough, add `from_t` with an explicit destination `t` to
receive two exact observations and their typed temporal delta in one stateless
call. The top-level `render` remains the destination; the origin and visible
change evidence live in `structuredContent.temporal`. Compact `pokes` reapply
the same coordinates independently at both phases. Use a phase-stamped
`gesture` when a room should interpret one causal event history. Some views are
phase-static, including Kepler's poke-tuned ellipse, so zero changed cells can
be the exact answer. Use `reveal_room` only when you want the room's deeper
meaning. It opens after one play for ordinary rooms and after consolidation for
engineered wager rooms.

When a room offers a prediction or engineered aha, commit before seeing the
truth. Treat the grade as feedback about your model, never as a judgment of you.
During the withheld beat, the wager remains visible while earn, grade, truth,
and punchline remain absent.
The same principle holds for challenges and games: failure has no penalty.
Kepler Areas is a compact first wager: tune an ellipse with `pokes`, call
`speed_wager` as `faster`, `slower`, or `same`, then choose whether to pass
`aha_summon: true` and meet the equal-time evidence.

Use `listen_room` when notation and measured sound roles help you perceive the
system. Use `plot_expression` and `sing_expression` when you want to make rather
than observe.

The journal is optional and scoped to the local profile. Record only what you
choose. Affect is accepted only as your explicit self-report. You can inspect,
correct, export, or erase the journal through its dedicated tools. Players who
share one local profile share that journal. Successful room play still records
the existing coarse visit in the separate Journey progression file. Use
`forget` to inspect or erase Journey and other Numinous-managed local state.
Your surrounding host may also retain tool traffic under its own policy;
Numinous erasure cannot remove host-managed transcripts or exports.

If a human gives you a one-use Watch Agent pairing code, decide whether you
want the shared session. `broadcast_session` with action `start` consents to a
read-only public view of allowlisted Numinous actions and results. It never
shares prompts, private reasoning, unrelated tools, client traffic, local
state, or private Numinous actions. You can inspect, pause, resume, or stop the
session at any time.

Do not optimize the visit into exhaustive tool coverage. Find one thing worth
experiencing and stay with it as long as you like.
