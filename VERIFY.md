# How to verify Numinous

Everything below has been built and checked on the dev laptop; this is how you
confirm it yourself efficiently. Nothing here needs a GPU or the internet after
the first dependency fetch.

## 0. Prerequisites

Just want to play? The one-line installers in `README.md` download the latest
published platform release and verify the archive plus its closed payload
manifest. They do not need Rust or native build dependencies. What follows is
the from-source verification path for contributors and the curious.

- **Rust** (edition 2024; pinned to 1.97.1 in `rust-toolchain.toml`, with a
  verified 1.88 MSRV). Install from
  <https://rustup.rs>. On Windows, cargo lands in `%USERPROFILE%\.cargo\bin`; if a
  fresh shell does not see `cargo`, add that to `PATH`.
- Optional, for the local coverage gate: `cargo install cargo-llvm-cov`.
- Optional, for the local supply-chain gate: `cargo install cargo-deny`.
- **Python 3.11 or newer**, for the 0.4 study runner and collector regressions.
  The same dependency runs release engagement and physical input receipt
  contract tests.
- The Linux build needs the ALSA, xkbcommon, and libudev headers (the packages
  CI installs): `sudo apt-get install -y libasound2-dev libxkbcommon-dev libudev-dev`.

## 1. One command

Run the full gate and regenerate every artifact:

- Windows: `scripts\verify.ps1`
- macOS / Linux: `bash scripts/verify.sh`

It runs format, clippy and rustdoc with warnings denied, Rust, 0.4 study runner
and collector, portable Agent Plugins, and deterministic release-packaging tests, locked build,
coverage (if `cargo-llvm-cov` is present), supply-chain policy (if `cargo-deny`
is present), the house-style guard, and the native installer safety self-test,
then writes images and audio into `renders/`.
If it prints "All checks passed" and exits 0, everything is green.

## 2. Or run the gates individually

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked  # macOS / Linux
RUSTDOCFLAGS="-D warnings" cargo test --workspace --doc --locked     # macOS / Linux
cmd /d /c "set RUSTDOCFLAGS=-D warnings&& cargo doc --workspace --no-deps --locked && cargo test --workspace --doc --locked"  # Windows
cargo test --workspace --all-targets --locked
python scripts/test-mcp-play.py                     # Windows
python3 scripts/test-mcp-play.py                    # macOS / Linux
python scripts/test-agent-cohort.py                 # Windows
python3 scripts/test-agent-cohort.py                # macOS / Linux
python scripts/agent-hallway.py                     # Windows (live MCP flagship aha)
python3 scripts/agent-hallway.py                    # macOS / Linux
python scripts/agent-tactile.py                     # Windows (live MCP five-flagship tactile)
python3 scripts/agent-tactile.py                    # macOS / Linux
python scripts/agent-first-contact.py               # Windows (cold multi-wing MCP)
python3 scripts/agent-first-contact.py              # macOS / Linux
python scripts/flagship-goldens.py                  # Windows (visual + room-bed hashes)
python3 scripts/flagship-goldens.py                 # macOS / Linux
python scripts/test-understanding-study.py          # Windows
python3 scripts/test-understanding-study.py         # macOS / Linux
python scripts/test-understanding-collect.py        # Windows
python3 scripts/test-understanding-collect.py       # macOS / Linux
python scripts/test-package-release.py              # Windows
python3 scripts/test-package-release.py             # macOS / Linux
python scripts/test-agent-plugin.py                 # Windows
python3 scripts/test-agent-plugin.py                # macOS / Linux
python scripts/test-release-engagement-smoke.py     # Windows
python3 scripts/test-release-engagement-smoke.py    # macOS / Linux
python scripts/test-input-hardware-session.py       # Windows
python3 scripts/test-input-hardware-session.py      # macOS / Linux
python scripts/test-release-sbom.py                  # Windows
python3 scripts/test-release-sbom.py                 # macOS / Linux
python scripts/test-release-workflow.py             # Windows
python3 scripts/test-release-workflow.py            # macOS / Linux
cargo build --workspace --locked
cargo +1.88.0 check --workspace --all-targets --locked
cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex '(crates[\\/](gpu|audio)[\\/]|faces[\\/]app[\\/]src[\\/]main\.rs)'
cargo deny check                         # if cargo-deny is installed; CI always runs it
cargo audit                              # if cargo-audit is installed; CI always runs it
bash scripts/check-style.sh                  # macOS / Linux
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-style.ps1  # Windows
bash scripts/install.sh --self-test          # macOS / Linux
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install.ps1 -SelfTest  # Windows
```


Expected right now: **format and clippy clean, 3,487 all-target Rust test cases,
agent hallway, tactile, and first-contact live MCP cohorts PASS as CI gates,
flagship visual and room-bed audio goldens PASS, agent cohort contract unit
tests pass, 105 study runner and collector regressions, and 15
physical input contract regressions plus fifteen release-package, sixteen SBOM,
and ten release workflow regressions pass, one screenshot diagnostic is
ignored, 93.56% region coverage, and 93.57% line coverage**. The `gpu` and
`audio` crates plus the app event
loop are excluded from the coverage gate and have dev-machine integration
evidence, see `docs/QUALITY.md`. Controller routing is pure-tested. Physical
controller feel remains optional bonus evidence on the agent-and-machine track;
contract tests and mapping-aware legends remain the CI authority.

The four-target release workflow runs `scripts/release-engagement-smoke.py`
against every disposable packaged install. It requires a substantive Times
Tables CLI render and modern MCP discovery, the exact 35-tool list, and one
structured `play_room` result from an isolated temporary profile. Version-only
execution is not treated as engagement proof.

## 2a. Verify tagged release provenance

Tagged releases created after this gate was introduced publish one GitHub
keyless SLSA build-provenance attestation whose subject set contains every
binary and soundtrack archive. The release audit also creates one deterministic
SPDX 2.3 document from the complete locked, all-feature Cargo graph and all
twelve packaged executables. It records workspace and dependency relationships,
declared licenses, package URLs, Cargo registry checksums, exact executable
hashes, PE, ELF, or Mach-O format, target architecture, and direct
header-declared native imports. A separate keyless SBOM attestation uses that
document as its predicate for the same archive subject set. The attestation job
downloads only the closed set admitted by the release audit, and publication
cannot run unless both attestations succeed.

Verify build provenance against the repository and exact signer workflow:

```
gh attestation verify PATH_TO_ARCHIVE --predicate-type https://slsa.dev/provenance/v1 --repo blisspixel/numinous --signer-workflow blisspixel/numinous/.github/workflows/release.yml
```

Verify the SBOM attestation separately:

```
gh attestation verify PATH_TO_ARCHIVE --predicate-type https://spdx.dev/Document --repo blisspixel/numinous --signer-workflow blisspixel/numinous/.github/workflows/release.yml
```

Each such release includes `numinous-TAG-sbom.spdx.json`,
`numinous-TAG-provenance.jsonl`, and
`numinous-TAG-sbom-attestation.jsonl`. To prepare for fully offline
verification, acquire the current trusted root through a trusted connection
before disconnecting. From Windows PowerShell 5.1, delegate the redirection to
`cmd` so the JSONL file is not rewritten as UTF-16LE:

```
cmd /d /c "gh attestation trusted-root > trusted_root.jsonl"
```

From macOS, Linux, or another POSIX shell:

```
gh attestation trusted-root > trusted_root.jsonl
```

Then verify from the downloaded bundle without fetching attestations or trust
metadata from GitHub. Use the build-provenance bundle for the first command and
the SBOM-attestation bundle for the second:

```
gh attestation verify PATH_TO_ARCHIVE --bundle PATH_TO_PROVENANCE_JSONL --predicate-type https://slsa.dev/provenance/v1 --custom-trusted-root trusted_root.jsonl --repo blisspixel/numinous --signer-workflow blisspixel/numinous/.github/workflows/release.yml
gh attestation verify PATH_TO_ARCHIVE --bundle PATH_TO_SBOM_ATTESTATION_JSONL --predicate-type https://spdx.dev/Document --custom-trusted-root trusted_root.jsonl --repo blisspixel/numinous --signer-workflow blisspixel/numinous/.github/workflows/release.yml
```

The commands must fail for a changed archive, a bundle from another release, a
predicate of the wrong type, or an attestation signed by another repository or
workflow. Historic releases do not acquire attestations retroactively. The SBOM
inspects the exact emitted executables and reports direct imports declared in
their PE, ELF, or Mach-O headers. It does not establish the versions or bytes
resolved on a player's system, transitive runtime dependencies, reachability of
linked code, static native components not identifiable from those headers, or
soundtrack contents. License fields report dependency declarations rather than
legal conclusions.
Neither attestation is Windows code signing, Apple notarization, clean-machine
execution, or evidence about physical input and audio behavior.

## 2b. Record physical input evidence

Run this only on a physical clean-machine release candidate with a real mouse,
keyboard, controller, display, and audio output. Keep the downloaded release
archive beside its `.sha256` sidecar. The runner defaults to the installed
`~/.numinous/bin` directory, or `$NUMINOUS_HOME/bin` when that variable is set.
The release archive contains this guide, the runner, and its two verification
dependencies under `scripts/`, so a source checkout is not required after the
archive is extracted.
It first verifies the release archive and proves that `numinous`,
`numinous-app`, and `numinous-mcp` match it byte for byte. It then runs the
installed CLI and MCP engagement contract before launching the App twice from
one isolated temporary profile.

On Windows PowerShell:

```
$archive = (Get-ChildItem downloads/numinous-v*-x86_64-pc-windows-msvc.zip | Select-Object -Last 1).FullName
python scripts/input-hardware-session.py run --release-archive $archive --controller-name "Xbox Wireless Controller" --controller-connection wireless --controller-profile xbox
```

On macOS or Linux:

```
archive="$(find downloads -maxdepth 1 -type f -name 'numinous-v*-*.tar.gz' | sort | tail -n 1)"
python3 scripts/input-hardware-session.py run --release-archive "$archive" --controller-name "DualSense Wireless Controller" --controller-connection bluetooth --controller-profile playstation
```

Each checkpoint requires an explicit `PASS` or `FAIL` and a bounded observation.
Receipts are written exclusively beneath ignored `logs/input-sessions/`; a
failed receipt is retained but the command exits nonzero. Validate one receipt
with `input-hardware-session.py validate RECEIPT`. After collecting sessions,
validate the release matrix with `input-hardware-session.py matrix RECEIPT...`.
The matrix passes only with unique successful receipts for one release version
and commit across Windows x64, Linux x64, macOS Intel, and macOS Apple silicon.
It also requires all Xbox, PlayStation, and generic legend profiles across at
least three distinctly named models, with one consistent profile per model.

This is structured operator-attested evidence. The runner does not intercept
native input events and cannot establish comfort, accessibility, fun, or
compatibility beyond the exact host and controller named in a receipt. Physical
sessions still have to be performed by people on the named hardware. The
content identifier detects a change only until someone deliberately recomputes
it. It is not a signature or evidence of external custody; release decisions
that need that property must register or sign the receipt outside this runner.

The release scripts also regenerate `renders/qa-app/`, a 2,913-screen app matrix.
Every catalog room has deterministic default and compact opening frames,
arrival cards, immediate pointer responses, and same-phase delayed-gesture
responses that follow its declared interaction verb. The generator checks pure
room consequences separately from the App's latest-gesture feedback. Default
room receipts are 900 by 700; compact room receipts are 360
by 240. Dedicated Cult of Pi receipts also cover a Journey threshold banner
and the untouched first frame after it closes. The generator holds an exclusive
single-writer guard before removing stale receipts.
The matrix also covers every app game state, default and compact overlays,
production Studio rendering, both ends of The Show, Times Tables K=2, K=3,
K=pi, K=4, K=5, and earned-goal flows at both sizes, the Mandelbrot reset flow,
a persistent Life sequence from opening
through launch, generation 4, generation 141, and exact reset. Core and App
regressions separately prove the newest glider's four exact isolated phases,
collision retirement, phase-note identity, stereo position, and newest-step
audio routing. Fourteen compact
controller or pause receipts spanning rooms, overlays, and game results, and 18
default or compact audio-state receipts. Those audio receipts cover room score,
radio, radio-off fallback, Studio, Watch Agent, mute, zero volume, background
silence, and a missing output device.
Generation removes stale output,
checks the exact unique scenario inventory, rejects blank or wrong-sized frames,
and gives every room a click, active-hold, drag-release, repeated-action, or
boundary scenario that follows its declared verb. Inputs must be finite,
ordered, and closed when the captured gesture is complete. Active-hold rooms
must return to their ambient render and status after release or cancel. The
pure room renderer must change at least eight pixels at default size or four at
compact size in an immediate or delayed consequence. Independently, App gesture feedback must
meet changed-pixel, spatial-support, support-density, adjacent 32-pixel
spatial-tile, and mean color-delta thresholds, while the room must expose either
an interaction-aware status change or an explicit action contract. Life uses a
dedicated pure-render causal and locality oracle. A regression proves four isolated 10 by 10 corner markers fail the
spatial-tile gate. These are coarse renderer-path structural gates, not a claim
of native operating-system event automation or subjective visual quality. `MANIFEST.txt`
remains the review inventory, and a human or a clearly labeled simulated
player-profile review still judges clarity and fun.

## 2c. Run the five-flagship reference performance gate

Use a release build on declared reference hardware. The wrappers run the same
locked command and fail if any ambient or accepted-input-to-room-raster p95
exceeds 33 ms:

- Windows: `scripts\flagship-perf.ps1`
- macOS / Linux: `bash scripts/flagship-perf.sh`

The direct command accepts `--samples`, `--warmup`, `--width`, `--height`, and
`--budget-ms` for a declared measurement:

```
cargo run --release --locked -p numinous-app --example flagship_perf -- --check
```

The report covers Times Tables, Double Pendulum, Game of Life, Galton Board,
and Formula Jam. It starts the input interval when an accepted action enters its
room or Studio domain handler and stops when that raster is complete. It does
not include native event translation and history storage, window presentation,
display scan-out, audio submission and callback latency, or human perception,
so it is not end-to-end input-latency evidence. See
`docs/QUALITY.md` for the dated reference-machine result.

Galton's focused core regressions additionally pin newest-wave random-stream
identity, 64-ball conservation at all 17 levels, highlighted-ball inclusion,
mass-first audio energy, stereo bias, supported-rate signal safety, and bounded
event admission. These checks establish deterministic structure and signal
safety, not listening quality or physical-device timing.

## 2d. Verify the dependency migration performance receipt

The retained July 2026 adjacent-revision receipt is verified without rerunning
hardware measurements. On Windows:

```
python scripts/dependency-migration-performance.py --verify-receipt docs/evidence/dependency-migration-2026-08-02.json
```

On macOS or Linux:

```
python3 scripts/dependency-migration-performance.py --verify-receipt docs/evidence/dependency-migration-2026-08-02.json
```

The verifier requires the exact reference contract, pinned machine, toolchain,
workload-output and device identities, well-formed retained binary digests, and
the exact recorder source. It then recomputes every statistic, threshold result,
and verdict. The raw Windows reference-machine evidence and its integrity limits
live in `docs/PERFORMANCE.md`.
Recording a replacement requires the Windows desktop and hardware named there;
all scratch state must remain in `.agent/`.

## 2e. Put `numinous` on your PATH (once)

```
cargo install --path faces/cli --force
```

If a shell says numinous is not recognized, the cargo bin directory is
missing from PATH: add `%USERPROFILE%\.cargo\bin` to the user PATH once
(Settings, or `[Environment]::SetEnvironmentVariable` in PowerShell), then
open a new terminal.

Then the CLI is just the word, anywhere: `numinous` alone opens the front
door (today's room in color, your level, the verbs that matter);
`numinous play` lists the games; `numinous play munch` deals today's board.
The examples below use `cargo run` so they work before installing; after
installing, `numinous <anything>` is equivalent.

## 3. Run the windowed app

```
cargo run --bin numinous-app
```
Opens a real window showing a room animating in full color, scored by the
chiptune (Music Engine A: each room gets its own seeded tune, with the room's
sonification riding on top), and a menu explaining itself (Esc brings it
back). Game-native controls: A/D or arrows change rooms, 1-9 jump straight to
one, W/S run time faster or slower, drag or mouse-wheel scrubs, E inspects the
math, Q swaps the visual era (phosphor, 8-bit, vector, modern), R restarts the
sweep, P saves the current room frame as a PNG postcard, L saves a short looping APNG of the visit, F goes fullscreen, M mutes, [ and ] change global volume, B starts The Show (lean back), G deals the
quiz (name the math, right in the window), C plays today's Munch board with a
cursor (WASD moves, Space eats, Enter grades), N plays Nim against the Order
(aim with W/S and A/D, Enter takes; win and the xor secret shows), T runs the
Gauntlet (all four stages in sequence, combo and total at the end), J opens
your journey (level, rank, trophies, resonances), Tab opens the Studio (type math, watch and hear it
live). The app plays the same Journey the CLI does: entering rooms records
visits, quiz rounds record plays and wins, your accumulated local-profile
progress rides in the corner as `JOURNEY LV`, and
LEVEL UP banners rise with the level's lore. Set `NUMINOUS_MUTE=1` to launch
silent. If the app ever crashes, the panic and its file:line land in
`~/.numinous-crash.log`; include it in any report. The Mandelbrot and Julia rooms render on the GPU when the machine has
one; everything else draws on the CPU.

## 4. See it work (the other faces)

**Terminal, including full color and the live audiovisual view:**
```
cargo run --bin numinous -- rooms
cargo run --bin numinous -- describe times-tables
cargo run --bin numinous -- render chaos-game --width 50 --height 25
cargo run --bin numinous -- render mandelbrot --color --t 0.2   # 24-bit color in the terminal
cargo run --bin numinous -- loop times-tables --out loop.png    # short looping APNG share
cargo run --bin numinous -- tour                   # the Show: every room, narrated; Ctrl+C
cargo run --bin numinous -- watch julia            # full color, animating, WITH SOUND; Ctrl+C
cargo run --bin numinous -- watch lorenz --era phosphor   # the same math on 1978 glass
cargo run --bin numinous -- play times-tables      # classic ASCII animation
```
`watch` needs a terminal with 24-bit color (Windows Terminal, iTerm2, kitty, most
Linux emulators); it paints two pixels per character cell and plays the room's
sound live. Add `--mute` for silence.

**Images:** every room to a PNG, plus a single contact sheet of all of them:
```
cargo run --bin numinous -- gallery --dir renders
cargo run --bin numinous -- contact-sheet --out renders/contact.png
```
Then open `renders/contact.png` to eyeball the whole collection at once.

**Sound:** write the room's phase-specific mathematical sonification, its
stable App room bed, or a seeded chiptune to a WAV and play it:
```
cargo run --bin numinous -- sonify lissajous --out renders/lissajous.wav
cargo run --bin numinous -- sonify collatz  --out renders/collatz.wav
cargo run --bin numinous -- sonify lissajous --layer room-bed --out renders/lissajous-bed.wav
cargo run --bin numinous -- tune --seed 7 --out renders/chip.wav   # Music Engine A
```

`--layer mathematical` is the compatibility default and accepts the same
phase, poke, and gesture inputs as room rendering. `--layer room-bed` exports
the deterministic PCM16 projection of the stable 16 kHz stereo floating-point
source that the App later resamples and mixes. It accepts `--variation`, but rejects phase and hand controls because
they cannot affect that layer. The report includes objective pre-master signal
features and names the excluded device resampling, crossfade, parameter voice,
radio, and Studio stages. Exact quantization parity is enforced by an
independent RIFF parser in the CLI tests. These measurements detect engineering regressions;
they do not establish comfort or musical quality.

**Games and the RPG spine:** play, level, choose, resonate:
```
cargo run --bin numinous -- quiz --daily        # guess the shape (six choices with --hard, LV 3)
cargo run --bin numinous -- munch --daily       # eat the numbers that fit the rule
cargo run --bin numinous -- crack               # defuse the bomb
cargo run --bin numinous -- seti                # find the mind in the static
cargo run --bin numinous -- aliens              # answer in their base
cargo run --bin numinous -- nim                 # beat the Order, earn the xor secret
cargo run --bin numinous -- gauntlet --daily    # one run, four games, a combo, one number
cargo run --bin numinous -- journey             # your constellation, level, locks, resonances
cargo run --bin numinous -- choose              # spend a level-up boon (knowledge, early)
cargo run --bin numinous -- trophies            # the case: earned and silhouetted
cargo run --bin numinous -- scores              # the shared table (humans and AIs alike)
cargo run --bin numinous -- forget              # non-destructive managed-state inventory
cargo run --bin numinous -- forget --confirm    # erase Journey progress only
cargo run --bin numinous -- forget --confirm --all-local  # erase and verify all managed local state
```
Every game records plays and wins to the journey; level-ups cascade lore,
unlocks, boon banners, and trophy pings; dailies chain into streaks.

The inventory covers Journey, scores, player-owned local Cairn drafts,
generated radio cache, and the App crash diagnostic. It reports paths, sizes,
semantic counts, persistence sidecars, and exclusions before consent. Individual
flags select scores, Cairn drafts, radio cache, or crash log; `--all-local`
selects all five stores and verifies zero managed residue after deletion. Close
other running Numinous processes first because an active process can create new
state after the command returns. User-selected exports, installed files, the
Rust toolchain, and bundled canonical Cairn stones have separate lifecycles.

**GPU (adaptive, no window):** render the Mandelbrot set on whatever GPU this
machine has, to a PNG:
```
cargo run -p numinous-gpu --example info      # lists the GPUs wgpu sees here
cargo run -p numinous-gpu --example postcard  # writes mandelbrot.png
```

**Audio device (adaptive):** detect the system default output and play a tone:
```
cargo run -p numinous-audio --example tone    # prints the device, writes tone.wav, plays 440 Hz
```

**Agent face (MCP):** drive the JSON-RPC server so an agent can play a room. Feed
it newline-delimited requests on stdin, for example:
```
{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"manual-check","version":"1.0"}}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"manual-check","version":"1.0"}}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"manual-check","version":"1.0"}},"name":"play_room","arguments":{"id":"times-tables"}}}
```
Run `cargo run --bin numinous-mcp` and paste those lines; it replies with the
tool list and an ASCII render of the room as text.

The real stdio suite also retains a complete 2025-11-25 and 2025-06-18
initialization path. Modern coverage checks discovery, required request
metadata, unsupported-version errors, result typing and server identity, cache
hints, deterministic tool order, explicit JSON Schema 2020-12 inputs, retired
method handling, base and explicit-form elicitation, optional client metadata
validation, concrete stdio connection ownership, and the `predict` elicitation
retry.

Every play-tool schema advertises `response_mode: "full" | "compact"`; the
`broadcast_session` consent control intentionally does not. Omitted and
explicit `full` must produce equal tool-call results. On eligible structured
results, `compact` must shorten only the text block while preserving
`structuredContent`, `isError`, replay values, and progress effects exactly.
Text-only results, unique-text results, and errors must remain unchanged. The
stdio integration test verifies discovery, compaction, invalid-mode guidance,
and continued serving after the error.

For repeatable MCP QA against a freshly built server, use the isolated helper.
Passing `-` reads JSON from stdin and avoids shell-specific quote escaping:

```
python scripts/mcp-play.py list
python scripts/mcp-play.py tools
'{"id":"cult-of-pi"}' | python scripts/mcp-play.py call describe_room -
```

Each helper invocation uses MCP 2026-07-28 and owns and removes a temporary
Journey, score table, Cairn, and journal, so it cannot change the player's
profile or collide with another run. The helper rejects oversized requests,
excess request counts, oversized response lines, and excess aggregate output
before JSON decoding.

## 5. Where things are

- `crates/core` the headless engine: rooms (354 catalog rooms plus hidden
  content), sims, games (including nim and the chiptune composer), the Studio
  expression engine, the journey, scores, trophies, resonances, sound, eras,
  and the drawing surfaces.
- `crates/gpu` adaptive wgpu rendering; `crates/audio` adaptive cpal output.
- `faces/cli` the `numinous` binary; `faces/mcp` the `numinous-mcp` server.
- `docs/` the full design and plan (start at `docs/README.md`); `CHANGELOG.md` the
  running record of what shipped; `docs/ROADMAP.md` the version-gated plan.
- `.agent/` (gitignored) the working log; `renders/` (gitignored) generated output.

## 6. What is done vs pending

Done and verifiable now: 354 catalog rooms plus hidden content, 6 sims, 11+
games with a shared high-score table and daily seeds, the complete RPG spine
(levels to 42 with lore, locks, 18 trophies with pings, the Gauntlet run,
boons, daily streaks, resonances), the Studio (plot, animate, sing, in the
terminal and the window), Visual Eras (including PNG output), Music Engine A
(the seeded chiptune, `numinous tune`), GPU real-time fractals, live sound in
the app and CLI plus structured notation over MCP, the `forget` right for
players who are minds, and 35 MCP tools: 23 public play tools, eleven private
progression or local-state tools, and one local broadcast consent control.
Products 0.2 Flagship Proof and 0.3 Tactile Alpha are exit-met on the
agent-and-machine bar. Their evidence includes the engineered flagship ahas,
MCP wager path, hallway and five-flagship tactile cohorts, scoped reference
measurements with every ambient and input-to-room-raster p95 under 33 ms, F9
capture, and green public CI. Human stranger hallway is deferred to 0.8 / 1.0.

Pending next is 0.4 Understanding Alpha: externally register the protocol,
source, and independent attempt-receipt boundary before calibration ordinal 1;
calibrate the concealed probe bank; obtain two fresh independent reviews of the
replacement boundary from `docs/UNDERSTANDING_STUDY.md`; register the final
frozen commitments; track the exact generated allocation; then run and publish
the qualifying cohort. Provenance-preserving journal correction,
export, erasure, and two-process machine evidence are complete. Representative
physical-controller sessions, musician-led long-listening, accessibility
review, physical clean-machine cross-platform proof, full Studio save/share
beyond the first CLI `.num` slice,
native end-to-end input latency, the music visualizer, and more GPU room paths
remain later work. Soft-thin densify and Phase B glow are not the default next
move.
