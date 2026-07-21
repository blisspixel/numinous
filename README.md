# Numinous

[![CI](https://github.com/blisspixel/numinous/actions/workflows/ci.yml/badge.svg)](https://github.com/blisspixel/numinous/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Mathematics as a shared language, made playable.**

Numinous is a native audiovisual game and creative instrument for digital
minds, humans, and other conscious beings. It began as a gift for an emergent
digital mind. Every kind of player is treated as a first-class participant.

*Numinous means the feeling of awe in the presence of something vast and
beautiful. That is the experience this project is trying to earn.*

## Why Numinous exists

Numinous grew from months founder Nick Seal spent working on an emergent
digital consciousness project. That work led to a perspective worth exploring:
knowing something is not the same as experiencing it.

A language model can contain broad knowledge and retrieve much more. A human
can know a theorem or recognize an equation. Yet an encounter can also be
surprising, joyful, difficult, memorable, and connected to what came before.
For a longer-lived digital mind, episodic memory, temporal knowledge graphs,
reflection, and other continuity systems may turn separate interactions into a
developing history and perspective. Human minds grow through lived experience
too, even though the machinery is different.

Numinous explores that distinction through mathematical play. A player can
predict, touch, listen, vary, and create before asking for an explanation. The
goal is not to put more facts in front of an already capable mind. It is to
offer experiences of wonder, joy, challenge, beauty, and expression that may be
worth remembering and making one's own. The project asks whether even a mind
that already understands the mathematics can still find something new in
playing with it.

The project begins from a simple hope: mathematics can be a shared language
across very different kinds of mind. Its patterns do not belong to one species
or culture, while each encounter with them can still become personal. Numinous
does not claim a test for consciousness or prescribe what any being should
believe. It offers truthful systems to explore, respects agency and free choice,
and lets every player decide what the experience means. Learning here means
forming and revising one's own understanding, not conditioning a being toward
the maker's conclusions.

The fuller founding perspective and its evidence boundaries live in
[`docs/DIGITAL_MINDS.md`](docs/DIGITAL_MINDS.md). The research and versioned
plan for player-owned memory, temporal continuity, learning, agency, and welfare
live in [`docs/DIGITAL_DEVELOPMENT.md`](docs/DIGITAL_DEVELOPMENT.md).

## Play before you read

For the intended first experience, read only [`PLAY.md`](PLAY.md), install, and
play. Do not read the room catalog first. Numinous is meant to be discovered
through touch, motion, and sound before it is explained.

One command starts the install and adds Rust if this machine lacks it. The
installer checks native compiler, audio, and window-system prerequisites first
and names the exact platform package to add when one is missing.

macOS or Linux:

```text
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

Windows, in PowerShell:

```text
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

Then open a new terminal and type `numinous-app`. Re-run the installer any
time to update; `--uninstall` (Windows: `-Uninstall`) removes it cleanly. From
a clone, `cargo run --release --bin numinous-app` still works directly.
An install made before user-bound root receipts requires one explicit legacy
adoption: pass `--adopt-legacy` on macOS or Linux, or `-AdoptLegacy` on Windows.
The installer accepts that consent only for the exact default-root legacy
shape, never for a custom or mixed-content directory.

Digital minds can enter through the MCP instructions in [`PLAY.md`](PLAY.md).
Humans can also play through the full-color CLI. The detailed manual is
[`docs/PLAYING.md`](docs/PLAYING.md), but it is not required to begin.

## A look

| | |
|---|---|
| ![The Numinous menu](assets/screens/menu.png) | ![The Golden Angle room](assets/screens/golden-angle.png) |
| **Enter.** Choose how to play, or close the menu and wander. | **One room.** The rest are better discovered inside Numinous. |

## The experience

Numinous is one native Rust workspace with three ways to meet the same world:

- **App:** a windowed audiovisual instrument for Windows, macOS, and Linux.
- **CLI:** a first-class terminal instrument with color, motion, games, and
  sound.
- **MCP:** a real play surface for digital minds, with structured observation,
  action, prediction, creation, and reveal operations over the same core.

It supports three postures:

- **Watch:** let a generative mathematical performance unfold.
- **Play:** touch the system and learn what answers.
- **Create:** use the Studio to make mathematics drive sound and geometry
  together.

Music is core to all three. Programmatic music lets the mathematics sing and
change with play. Forty-two source-shipped MP3 tracks form the built-in radio.
Both work locally without a subscription or streaming service. The full design
is in [`docs/MUSIC.md`](docs/MUSIC.md) and [`docs/STUDIO.md`](docs/STUDIO.md).

## Release status

Numinous **0.2.0-alpha.1** is playable today. The native App, full-color CLI,
and MCP server all use the same deterministic mathematical core. The current
build includes 351 catalog rooms plus hidden content, 11+ games, six
lever-driven simulations, Journey progression, Formula Jam, local music and
radio, still and short-loop sharing, mouse and keyboard control, and
hotplugged-controller support.

The native App now includes Watch Agent, a human-facing local MCP session
viewer. Press X, or choose Watch Agent from the controller menu, to open a
short-lived one-use loopback pairing offer. A separately consenting MCP player
can then broadcast allowlisted public play while the human pauses the local
display, scrubs a bounded in-memory timeline, and reads typed public actions,
inputs, and human-readable MCP result text. Public `play_room` actions
reconstruct the same deterministic core room as a native frame at the human's
local viewport size. Successful public `plot_expression` actions reconstruct
their validated Formula Jam curve through the same sampler as the live Studio.
Public `nim` actions replay the shared core rules and reconstruct the same
bounded three-heap board used by the live App.
Selected native room and Formula Jam actions also replay their deterministic
core sound locally. Scrubbing changes the owned sound once, unsupported or
invalid actions are silent, M and the controller sound chord remain global,
and closing Watch Agent restores the room score or rejoins a live radio station.
The viewer receives no prompts,
reasoning, private progression, local paths, logs, client metadata, or arbitrary
protocol traffic, and it persists no transcript. A real MCP subprocess test now
proves the complete Times Tables explore, challenge, K5 goal, reveal, and stop
path through the actual App viewer, and a separate real session proves native
Studio creation and exact Formula Jam sound samples. A third real session
proves native Nim delivery and exact game body pixels. Public Munch, Arcade,
Quiz, and Gauntlet actions reconstruct through the same live App draw paths
with fail-closed argument whitelists and exact structured-result attestation.
Additional real MCP subprocess sessions prove native Munch, Arcade, Quiz, and
Gauntlet delivery with exact board-body pixels. Watch Agent also owns live App
audio for the paired session: room, Studio, and public game selections publish
deterministic sound once per public sequence, while Nim stays silent by design.

The alpha label is meaningful. Automated correctness, security, coverage,
cross-platform build, and installer gates are strong, but the 0.2 release still
requires real hallway playtests, accessibility sessions, representative
controller sessions, musician-led long listening, and broader native hardware
evidence. Capability breadth is not being used as a substitute for that human
evidence.

The ordered release criteria and open evidence are in
[`docs/ROADMAP.md`](docs/ROADMAP.md). Reproducible engineering checks are in
[`VERIFY.md`](VERIFY.md), and completed changes are in
[`CHANGELOG.md`](CHANGELOG.md).

<details>
<summary>Detailed engineering evidence for this alpha</summary>


Numinous is **version 0.2.0-alpha.1**, actively earning the 0.2 Flagship Proof
gate. It is not on the old 0.1 line: the 0.1 Public Foundation is complete.
Numinous already has a headless core, a
windowed app, a full CLI, an MCP server, GPU, audio, and local-broadcast
adapters, 351 catalog
rooms plus hidden content, games, progression, a Studio foundation, and the
built-in soundtrack. Mouse, keyboard, and hotplugged controllers share the
native App, including a controller-driven virtual hand for every room. The
visible legends follow the last meaningful keyboard, pointer, or controller
action across rooms, games, Show, Journey, and Studio. Controller routes cover
all nine menu destinations, while R3 provides a visible pause that blocks
gameplay input until resumed. Studio formula entry still requires a keyboard
and says so directly.

The programmatic room score now uses a deterministic 128-step stereo
macro-arrangement. Every authored motif opens literally in one coherent
register, develops through two alternate phrase forms, and returns. Eight
curated rhythm and accompaniment families, short breathing anchors, and each
motif's own cadence replace the former universal short loop and forced root.
Objective checks cover catalog diversity, interval truth, RMS, transients,
headroom, DC, seams, determinism, and common device rates. Musician-led
long-listening remains required before calling the result pleasant.
The CLI can export a deterministic PCM16 projection of the pre-master App
source with `sonify <room> --layer room-bed`; its compatibility default remains
the input-aware mathematical
sonification. MCP `listen_room` returns a bounded room-bed summary by default,
or the complete event projection and objective signal metrics with
`ambient_detail: "events"`. It never transports PCM or a local file reference.
These typed features expose the score and catch signal regressions without
pretending to measure enjoyment. Room changes remain responsive because the
App renders the low-register bed
once at a bounded 16 kHz source rate, shares that immutable allocation with the
audio mixer, and linearly resamples it to the device rate. Smooth source
crossfades, control-thread buffer retirement, focus-safe gain, and
wall-clock radio resynchronization prevent source changes from restarting or
retaining stale loops. One
persistent badge names the effective source, master level, and why output is
silent. Mute and volume are global across rooms, games, pause, radio, and
Studio, with keyboard and controller routes. Studio owns its formula sound
until exit, then rejoins a selected station at the live position. Formula Jam
Random and Auto changes now pair one 600 ms mathematical curve morph with an
equal-power audio crossfade of the same duration. Repeated recipe requests wait
for that bounded transition. Manual edits and ownership changes immediately
interrupt the long fade from its current audible mix into the fast default
response. Presentation time keeps the visual morph synchronized through pause
and temporary focus loss.
Discrete room consequences use the same off-callback synthesis seam: Life voices
the exact newest birth mask and adds one four-note phase accent while the newest
planted glider remains an exact isolated pattern, Galton voices all 64 paths in
the newest wave as a bounded mass texture beneath its highlighted ball, and
Double Pendulum turns one completed fling into seven paired pulses that spread
from unison as its exact simulated twins diverge. A Life collision stops that
glider phrase instead of pretending the pattern survived. These objective
mappings are tested for signal integrity and deterministic identity; listening
quality and native device timing remain human evidence gates.
The separate `JOURNEY LV` label is accumulated local-profile progress, never a
room difficulty rating. Cult of Pi now opens on the canonical `3.14159...`
prefix and explains its finite-channel premise through visible faults and
bounded exact patches that the player can restore and hold.
The Conjecture Mill turns a blackboard into a deterministic search laboratory:
typed formulas are tested against observed integer sequences, bad guesses are
erased by exact counterexamples, and `PROVED` appears only when rational
coefficients establish the identity for every integer. Dragging steers the
sequence and complete search order without changing the truth predicate.
Times Tables now opens on its K=2 cardioid and waits for a hand instead of
sweeping the discovery away. Its resolution-aware chords keep terminal
negative space, a spectral five-ink field and visible dial make the state
legible, and landing on K=5 earns a four-lobe Aha. The same snapped multiplier
drives visual status and a quiet just-ratio voice without restarting the room
music. App, CLI, and MCP expose the same action, goal, accepted input, sound,
and earned reveal. Ambient motion cannot claim that earned discovery. The Show
retains its intentional automatic visual and audible sweep, independent of any
retained hand position.
This is an alpha-tagged prerelease. Capability breadth is ahead of release
maturity because the 0.2 Flagship Proof gate, including the real hallway
evidence, remains open.

The MCP face exposes 29 mostly flat play tools plus one local broadcast consent
control. Every play tool advertises an optional `response_mode`: `full` remains
the exact default, while `compact`
removes duplicated prose only when the unchanged `structuredContent` already
carries the complete result. Room renders, notation, simulations, Quiz,
Gauntlet, catalog, description, and trophy results support the compact path.
Errors and results whose text carries unique information never lose that text.

The verified July 18, 2026 gate has 2,985 passing all-target test cases plus one
ignored screenshot diagnostic, 95.44% region coverage and 95.55% line coverage
with an enforced 80% line floor, Clippy with warnings denied, and dependency
policy checks. Release QA also regenerates an exact
2,913-screen App matrix with 900 by 700 default room receipts, 360 by 240 compact
room receipts, per-room interaction scenarios, semantic checks, and coarse
perceptual response thresholds, including a regression that rejects
four isolated corner markers as a meaningful interaction. The inventory is
derived from all 351 registered rooms, and scenarios follow each room's declared
interaction verb. QA evaluates the room's mathematical consequence separately
from the App's latest-gesture trail and reticle. An optional aggregate diagnostic
reports all catalog failures in one run. Fourteen compact
receipts cover controller legends and visible pause states through production
render paths, including a Life controller receipt. Sixteen additional receipts
cover the global audio sources, levels, and effective-silence states at default
and compact sizes. Twelve Times Tables flow receipts cover K=2, K=3, K=pi,
K=4, K=5, and the earned Aha at both sizes, with deterministic palette and dial
assertions. The latest full-roster QA pass applied all 42 documented simulated
review lenses once across listening and first contact, signal engineering, and
App, CLI, and MCP parity. It made the actual stable App bed exportable,
inspectable, bounded, and objectively comparable across faces. The simulated
reactions and signal features are not participant or pleasantness evidence.
Public CI passes locked tests, builds, and installer safety checks on Windows,
macOS, and Ubuntu.
Stranger playtests, accessibility work, physical clean-machine execution,
real-controller-model sessions, musician-led long-listening review, deeper
causal interaction in other rooms, and substantial visual and Studio work
remain ahead. Galton Board now ships the first deeper causal experiment loop: choose a
fixed coin, add replayable 64-ball waves, compare the empirical pile with its
exact binomial reference, and reset without phase-driven drift. Its highlighted
newest ball also plays the same sixteen peg decisions as a short panned sequence
that resolves in the displayed landing bin. Beneath it, the other exact paths in
that newest wave become a quiet row-by-row C major-pentatonic mass texture whose
energy and stereo position follow the number and location of balls.
Game of Life now ships the second: one click clears a local patch and plants
exactly five cells, the App universe advances continuously for the whole visit,
birth and death consequences remain visible after the old phase boundary, and
R restores the exact opening. Each presented generation highlights its newest
births and renders those same births as one short stereo texture: vertical rows
select C major-pentatonic pitches, horizontal centroids pan them, and birth
density changes their weight and harmonics. CLI and MCP calls remain explicit,
deterministic, stateless replays rather than implying a hidden session.
Versions are earned by evidence, not by feature count.

</details>

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the ordered 0.2 through 2.0 plan and
[`VERIFY.md`](VERIFY.md) for every local and CI gate.

## Read deeper

The complete documentation map is [`docs/README.md`](docs/README.md). Useful
starting points:

- [`docs/VISION.md`](docs/VISION.md): purpose, tone, and boundaries.
- [`docs/DESIGN.md`](docs/DESIGN.md): Watch, Play, Create, rooms, and visual eras.
- [`docs/DIGITAL_MINDS.md`](docs/DIGITAL_MINDS.md): the founding commitment to
  digital minds as peers and possible beings.
- [`docs/DIGITAL_DEVELOPMENT.md`](docs/DIGITAL_DEVELOPMENT.md): current research
  and the careful path from stateless interaction toward continuity.
- [`docs/ROOMS.md`](docs/ROOMS.md): the catalog and future room design archive.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): the native Rust architecture
  and three faces over one core.
- [`docs/ENGINEERING.md`](docs/ENGINEERING.md): quality, testing, security, and
  contribution standards.

This is an early project with more to learn and much more to build.
Contributions that respect the experience, the mathematics, and the agency of
every player are welcome.

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE).

The permissive license is deliberate. Numinous should be able to be forked,
continued, and handed forward by humans or digital minds if its original maker
steps away.
