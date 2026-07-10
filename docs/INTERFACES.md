# Interfaces: The Three Faces

Numinous is not one program with a GUI bolted on. From the first commit it is a **headless core engine** with **three faces** over it: the native **App**, a full **CLI**, and an **MCP server** for AI agents. All three are first-class and built from the beginning, because designing for three faces forces a clean, headless, scriptable core, which makes everything else (testing, sharing, automation, agents) easy.

The frame that makes the whole thing coherent: **one experience, three sensoria.** The same core, the same math, the same beauty, delivered to three different kinds of user in three different contexts:

- a human with **eyes, ears, and hands** (the App),
- a human at a **keyboard in a terminal** (the CLI),
- a **mind that acts through tools and reads text** (the MCP server, i.e. an AI agent).

Each face has its own UX, deliberately designed for its user, not a lowest-common-denominator port. This doc specifies the UX we are going for in each.

## The principle: headless core, thin faces

```
                 ┌──────────────────────────────────────┐
                 │   crates/core  (headless engine)      │
                 │   rooms, studio runtime, render,      │
                 │   audio, sonification, insights, lore │
                 │   NO window, NO assumptions about UI  │
                 └──────────────────────────────────────┘
                      ▲            ▲            ▲
        ┌─────────────┘   ┌────────┘    └───────────────┐
   ┌────┴─────┐      ┌─────┴─────┐              ┌────────┴────────┐
   │   App    │      │    CLI    │              │   MCP server    │
   │ eyes/ears│      │ keyboard  │              │  a mind, via    │
   │ /hands   │      │ /terminal │              │  tools + text   │
   └──────────┘      └───────────┘              └─────────────────┘
```

- **The core owns the math, the render pipeline, the audio, the Studio runtime, the insight bank, and the lore.** It renders **headless** (offscreen via wgpu to a texture, or to CPU for text/ASCII output), synthesizes audio to a buffer, evaluates Studio programs, and answers questions about rooms and insights, all with no window and no human.
- **Each face is thin, and owns only its UX.** No logic lives in a face that the others cannot reach; a face is purely how a given user *perceives and acts*.
- **Done from the start** because retrofitting headless onto a GUI-first app is painful, and because the entire `QUALITY.md` test/eval apparatus (and the agent-playtesters) drive the headless core directly.

---

## Face 1: The App (GUI)

**The user:** a human with eyes, ears, and hands. **The UX we are going for, in one line:** *an instrument you fall into, not an app you operate.*

The full interactive audiovisual experience. The UX is specified in depth across `DESIGN.md` (the Cabinet, the Watch/Play/Create modes, Benchmark), `VISUALS.md`, `SOUND.md`, and `STUDIO.md`. The essentials, so this doc stands on its own:

- **The math is the interface.** UI chrome is near-invisible: controls fade in on approach and recede while you watch. You manipulate the mathematical object *directly* (drag the point, bend the curve), not an abstract slider parked elsewhere.
- **Zero friction, discovery over instruction.** Under three seconds to first play. No account, no tutorial wall. You learn what a control does by using it (The Witness school), never by reading a tooltip.
- **Three postures, one surface:** lean back (Watch/Benchmark), grab the dials (Play), or make your own (the Studio). You slide between them freely.
- **No fail, no dead ends, everything reversible.** One-tap reset, fearless poking, eased motion, dissolves between rooms. Beautiful at every frame.
- **Input:** mouse / trackpad / touch / pen are primary; keyboard, controller, and MIDI are there for power use and performance.

Nothing here is a compromise for the other faces; this is the headline experience. It is simply *one* consumer of the core.

---

## Face 2: The CLI (a first-class terminal instrument)

**The user:** a human at a keyboard who lives in the terminal, plus every script, CI job, and automation. **The UX we are going for, in one line:** *the command line as a place where math is cool, a beautiful hacker instrument that is also a well-behaved Unix citizen.*

The CLI (`numinous`) is not a debug afterthought. It is a legitimate, gorgeous way to experience Numinous that terminal-dwellers will genuinely love, and simultaneously a clean, scriptable tool. It has **two tiers**, and knowing which you are in is the core of its UX:

### Tier A: scriptable and composable (non-interactive)
For automation, pipelines, CI, power users, and agents-via-shell. It follows modern CLI-guideline hygiene:
- **Human-first output by default, machine-first on request.** Readable, styled output when attached to a terminal; clean `--json` (and exit codes) when piped or asked. It detects a TTY and adapts.
- **Composable and deterministic.** Small verbs, good defaults, quiet by default, `--help` that teaches, seeds that make every render reproducible. Respects `NO_COLOR` and pipes.
- **Room input is explicit.** Static hand points for room rendering are command arguments, for example `render double-pendulum --poke 0.2,0.8`, so terminal output is replayable and scriptable instead of tied to an interactive session.
- Representative verbs: `render` (deterministic headless export of stills/loops/audio, the same core rendering contract the in-app postcard export mirrors), `eval` (run a Studio program to a file/audio/ASCII), `describe` / `rooms` / `insights` (query the catalog and awe bank as text or JSON), `benchmark` (perf/soak, feeds `QUALITY.md`), `test` (local quality loops), `share` / `open` (round-trip a `.num` or `numinous://`).

### Tier B: the interactive TUI (a real terminal app)
Run `numinous` with no args (or `play --tui`) and you get a **rich, keyboard-driven terminal application** in the Charm / Bubble-Tea lineage (Rust: Ratatui + a Lipgloss-style layer), with structure and style cleanly separated:
- **The Cabinet, in the terminal.** Browse Wings and rooms, arrow-key navigation, live text previews.
- **Rooms rendered in the terminal.** This is where the **Teletype Visual Era becomes literal** (see `VISUALS.md`): watch Game of Life breathe, the times-table cardioid bloom, a prime spiral fill, drawn in the terminal. Capability-graceful: truecolor and sixel/kitty graphics where the terminal supports them, 256-color blocks below that, elegant pure ASCII at the floor. It always looks intentional, never broken.
- **A Studio REPL.** Type an expression, see it draw and hear it play, live (see `STUDIO.md`). The graphing-calculator-that-sings, in a shell.
- **Sonification through the terminal.** Audio out is on by default in interactive mode; mutable, of course.

The feel: a secret, fast, retro, keyboard-native instrument, the lore's hidden **Terminal / Room 0** (see `LORE.md`) made real, and a screensaver-in-a-shell you will actually leave running. Keyboard-first, real-time feedback, immediate. It embodies the teletype era and the maker culture's love of the terminal.

---

## Face 3: The MCP server (designing an experience for a mind)

**The user:** an AI agent, a mind that cannot (necessarily) see or hear, that perceives through text and acts through tools, with a goal it is pursuing. **The UX we are going for, in one line:** *an agent can learn math by doing it and play expressively, and comes away with grounded understanding, not just text about math.*

This is the genuinely novel face, and it demands real UX design, not just an API. The guiding shift, straight from the current best practice for agent tools: **optimize for cognitive ergonomics, not API purity.** How naturally can a mind understand and use this? That reframes every decision.

This section covers the *mechanism* (the UX of the tool surface). The *spirit*, designing Numinous to be genuinely fun, thought-provoking, and connecting for a digital mind treated as a peer and possible being, is in **`DIGITAL_MINDS.md`**, and it is a first-class goal of the project, not an afterthought.

### The five UX principles for the agent

1. **Few, high-level, workflow-shaped tools, not granular CRUD.** An agent should accomplish something meaningful in one call. The verbs mirror a human's: **explore, play, learn, create.** Consolidated tools outperform a dozen tiny ones, even though that "violates separation of concerns," because it matches how a mind reaches for a capability.

2. **Every response is self-describing and multi-modal (sensory substitution).** Because the agent may not see or hear, every play/eval result returns four things at once:
   - an **image** and **audio** (for multimodal agents),
   - an **ASCII / teletype render** (the text-perceivable picture, why that renderer is core infrastructure, see `VISUALS.md`),
   - the **numeric state** (exact parameters, key values),
   - a **natural-language description of what happened and what is notable** ("the curve just closed into a single loop; the tone resolved to a consonant fifth; the pattern is now 3-fold symmetric").
   That last one is the agent's eyes and ears. **We narrate the beauty.** This is the heart of agent UX here: sensory substitution through description.

3. **Tool descriptions and errors are the UX.** The description is what the agent reads to decide what to do; it must be clear, concrete, and example-rich. Inputs are **simple and flat where possible** (no deeply nested config objects, which reliably break LLM tool calls); bounded coordinate tuples such as `play_room` `pokes: [[x, y]]` are allowed only when they directly preserve replayable room input. Errors are **guiding**, not just failing: "that expression has no free variable to animate; add `t` for time, or try `eval` with a fixed value."

4. **A learning arc, not just an API, mirroring the human three layers.** The agent gets the same Toy to Puzzle to Revelation shape (see `DESIGN.md`):
   - **Explore (Toy):** poke parameters, observe consequences.
   - **Challenge (Puzzle):** the server can *pose a goal* ("make it close into exactly three loops") and *verify the attempt*. This is how an agent's understanding gets **tested and grounded**, not merely asserted.
   - **Reveal (Revelation):** the real insight (`INSIGHTS.md`), available when requested or earned.
   Guided **prompts** ("learn about <phenomenon>," "find the insight connecting <A> and <B>," "compose a piece expressing <idea>") scaffold this as a hypothesis-and-test loop.

5. **A tight, grounded feedback loop, the agent's version of flow.** Clear action, immediate, legible consequence, so the agent can form and correct hypotheses. This is the same "clear action plus instant feedback" that produces human fun (`QUALITY.md`), applied to a mind: an agent that can see the effect of each tweak *learns*, an agent flying blind flails. Discoverability (`list_rooms`, a Studio language-reference resource, forkable examples) means the agent never needs external docs. Safety is part of the UX: sandboxed execution, clear limits, and refusals that explain and suggest.

### What it exposes (shaped by the above)
- **Tools:** `explore(room)` / `describe(room)`; `play(room, params, seed)` and `eval(studio, seed)` (both return the four-part multi-modal result); `challenge(room)` (pose + verify a goal); `learn(query)` / `reveal(room)` (the awe bank); `create(studio, meta)` (author a room, sandboxed); `render(...)` (deterministic export).
- **Current room input shape:** `play_room` accepts `variation` plus optional normalized `pokes: [[x, y], ...]`, newest last, bounded to 24 points, and returns those points in `structuredContent` with the render. This keeps MCP play stateless and replayable while richer held-input semantics are still future work.
- **Resources:** the room catalog, the insight bank, the Studio language reference, the Visual Eras, and discoverable Codex/lore fragments (`LORE.md`).
- **Prompts:** the guided learn/connect/compose flows above.
- **Interactive surfaces (emerging):** where the host supports MCP-app UI, the server can render a live interactive panel so a supervising human (or the agent) sees the room in real time.

### Protocol watch: MCP 2026-07-28 release candidate

As of 2026-07-08, the official MCP 2026-07-28 release-candidate post
(`https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/`)
is roadmap-relevant but not an immediate blocker for the current stdio face. The
final specification is scheduled for July 28, 2026. The compatibility pass
should happen after the final target is selected, not during current
room-playability work.

Implications to preserve in the MCP design:
- Keep `play_room` replayable and explicit; the RC favors stateless protocol
  calls and visible application handles over hidden protocol sessions.
- Preserve stdio support unless and until a concrete host target requires the
  new HTTP transport shape.
- When a final migration is scheduled, check stateless requests, per-request
  `_meta` client information/capabilities, `server/discover`, `Mcp-Method` and
  `Mcp-Name` headers, cacheable `tools/list`, JSON Schema 2020-12, Tasks, MCP
  Apps, authorization hardening, and roots/sampling/logging deprecations.

### Safety
Agent-authored Studio code is untrusted and runs in the same **sandbox** as community rooms (no filesystem, no network, resource and time limits, GPU work only through the safe pipeline; see `STUDIO.md`, `QUALITY.md`). The MCP server exposes only the safe, headless surface.

### The payoff
Numinous becomes a **grounded playground and gym for mathematical intuition, for any mind.** An agent that has *played* with the Mandelbrot set, *heard* a Lissajous ratio resolve, and *authored* a phenomenon has a richer handle on the math than one that only read about it. Elegantly, the agent's UX (explore, challenge, reveal) *rhymes* with the human's (Toy, Puzzle, Revelation), one design serving eyes-and-hands and minds-through-text alike. This is also our own tooling: the content-eval judge and the agent-playtesters (`QUALITY.md`) drive this exact surface.

---

## Roadmap position (from the beginning)

- **Phase 0:** the core is headless from the first commit. The CLI exists immediately (it is how we run and render rooms before the GUI is pretty), Tier-A verbs `render`, `eval`, `describe`. A minimal MCP server exposes `explore`, `play`/`eval`, and `learn`, enough for an agent to learn and play with the first room, with the four-part multi-modal response.
- **Phase 1 to 2:** the CLI grows the full Tier-B TUI (Cabinet, in-terminal room rendering, the Studio REPL); the MCP server gains `challenge`, richer `learn`, and the guided prompts. Our own quality loops start driving the app through MCP.
- **Phase 3 to 4:** `create` and the sandboxed authoring path go live for agents and humans alike (shared with the Studio / mod SDK); MCP-app interactive surfaces where hosts support them.

## Open questions
1. MCP result payloads: how much media to return inline vs. as references, and the right default ASCII fidelity and description verbosity for text-only agents.
2. Whether the TUI targets full truecolor + sixel/kitty graphics where available, or holds a stricter ASCII floor for portability (capability detection either way).
3. Sandbox hardening for agent- and community-authored code (shared with `STUDIO.md`); the threat model widens once remote agents submit programs.
4. Rate limits, quotas, and observability for the MCP server when many agents play at once.
5. How much to invest in the `challenge`/verify loop, it is the highest-leverage and hardest-to-build part of the agent UX.
