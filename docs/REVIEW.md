# External review, July 2026

A full product review received during the build; kept verbatim in spirit and
condensed here to its 25 findings, with the build's responses. The review's
own summary of the product stands as the best short description yet written:

> Numinous is a native audiovisual math game where mathematical phenomena
> are playable instruments. You do not study equations; you touch simple
> rules, watch them explode into beauty, hear them as music, and optionally
> uncover the revelation underneath.

The mantra it set, now the roadmap's first line: **every screen answers
your hand; every answer reveals the math.**

## Findings and responses

1. Doctrine conflict (metroidbrainia vs RPG spine): RULED, Option B,
   ceremony not economy; the ruling lives in PROGRESSION.md.
2. The poke must become a real input substrate (RoomInput enum: pointer
   down/move/up, wheel, keys, params), not one-shot clicks: ACCEPTED, on
   the priority stack as item 2.
3. Rooms as deterministic dynamical systems (S, P, V, I, T, R, A, C, E)
   with rich .num artifacts: ACCEPTED as the target Room contract.
4. Semantic rendering roles (StrokeRole, VoiceRole) to prevent aesthetic
   drift: ACCEPTED for the era/accessibility pipeline.
5. Studio as layered runtime (L0 expressions to L4 sandboxed WGSL): the
   authoring model; L0/L1 exist today.
6. Sonification graduates from mapping to composition: three layers per
   room (ambient bed, interaction voice, motif); listen_room returns the
   motif structurally: ACCEPTED as Engine A2's definition.
7. Operational beauty constraints per room plus beauty sampling: ACCEPTED;
   contact-sheet QA is the seed of it.
8. Open math claims are release-blocking: reveals audited (prime spirals
   tightened; times tables already careful); open-problem cards need a
   last-verified date mechanism: QUEUED.
9. Agent play: structured deltas, challenge gradients, replayable
   trajectories: SHIPPED v1 for deltas (play_room `delta`) and gradients
   (the `challenge` tool); replayable trajectory logs QUEUED.
10. Digital-mind stance stays non-manipulative; taglines stay literal:
    ADOPTED ("Math you can feel." / "A playable math world for human and
    digital minds.").
11. Casino/prison: active play must outearn idling; XP shaping rules
    ADOPTED under the Option B ruling.
12. Munch as real arcade: SHIPPED v1 (Muncher, Vexations); enemy-as-
    failure-mode roster (Mirror, Jammer, Proof) QUEUED.
13. Challenge specs per room (metrics, not binary): SHIPPED v1 (the seeded
    spatial-response challenge, graded as metrics); room-specific parameter
    goals SHIPPED v2 (the challenge tool's parameter kind targets the room's
    own status readout, "land TILT within 0.02 of 0.31" style, graded as
    distance and a climbable score).
14. Lore stays invisible at the surface: standing law, reaffirmed.
15. First ninety seconds: open on the flagship phenomenon, not the menu:
    OVERRULED by the founder (July 2026): the menu stays; it leads with
    PLAY and is part of the identity. The flagship gets its polish inside
    the room instead.
16. Times Tables as production template: priority 1.
17. Performance as aesthetic (audio never waits; render degrades
    gracefully): standing engineering law.
18. Cross-platform proof early: priority 7.
19. Sandbox before community creation: standing architecture law; the
    design ruling elaborating it (three content tiers, the Studio language
    as the sandbox, the trust and distribution model) is `EXTENSIBILITY.md`
    (July 2026, founder-directed).
20. Quality loops must gate real decisions, not perform: standing law;
    the hallway test is sovereign.
21. Room proof packets: ACCEPTED as the room template.
22. Docs status reconciliation with truth headers and a linter: QUEUED.
23. Name stays Numinous; subtitle "Math you can feel.": ADOPTED.
24. Priority stack: ADOPTED verbatim into ROADMAP.md.
25. The final bar (open cold, touch, say "wait", share; no lies; every
    frame respectable; a coherent instrument; agents as peers; dignity):
    this is the 1.0 exit criterion restated, and it stands.
