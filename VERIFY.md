# How to verify Numinous

Everything below has been built and checked on the dev laptop; this is how you
confirm it yourself efficiently. Nothing here needs a GPU or the internet after
the first dependency fetch.

## 0. Prerequisites

Just want to play? The one-line installers in `README.md` check the native
prerequisites, explain any missing platform package, install Rust when needed,
and build Numinous. What follows is the from-source verification path for
contributors and the curious.

- **Rust** (edition 2024; pinned to 1.97.1 in `rust-toolchain.toml`, with a
  verified 1.88 MSRV). Install from
  <https://rustup.rs>. On Windows, cargo lands in `%USERPROFILE%\.cargo\bin`; if a
  fresh shell does not see `cargo`, add that to `PATH`.
- Optional, for the local coverage gate: `cargo install cargo-llvm-cov`.
- Optional, for the local supply-chain gate: `cargo install cargo-deny`.
- The Linux build needs the ALSA, xkbcommon, and libudev headers (the packages
  CI installs): `sudo apt-get install -y libasound2-dev libxkbcommon-dev libudev-dev`.

## 1. One command

Run the full gate and regenerate every artifact:

- Windows: `scripts\verify.ps1`
- macOS / Linux: `bash scripts/verify.sh`

It runs format, clippy and rustdoc with warnings denied, tests, locked build,
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


Expected right now: **format and clippy clean, 2,985 all-target test cases pass,
one screenshot diagnostic is ignored, 95.44% region coverage, and 95.55% line
coverage**. The `gpu` and `audio` crates plus the app event
loop are excluded from the coverage gate and have dev-machine integration
evidence, see `docs/QUALITY.md`. Controller routing is pure-tested. Sessions
with representative physical controller models remain open.

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

## 2a. Run the five-flagship reference performance gate

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

## 2b. Put `numinous` on your PATH (once)

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
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"manual-check","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"play_room","arguments":{"id":"times-tables"}}}
```
Run `cargo run --bin numinous-mcp` and paste those lines; it replies with the
tool list and an ASCII render of the room as text.

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

Each helper invocation owns and removes a temporary Journey, score table, and
Cairn, so it cannot change the player's profile or collide with another run.

## 5. Where things are

- `crates/core` the headless engine: rooms (352 catalog rooms plus hidden
  content), sims, games (including nim and the chiptune composer), the Studio
  expression engine, the journey, scores, trophies, resonances, sound, eras,
  and the drawing surfaces.
- `crates/gpu` adaptive wgpu rendering; `crates/audio` adaptive cpal output.
- `faces/cli` the `numinous` binary; `faces/mcp` the `numinous-mcp` server.
- `docs/` the full design and plan (start at `docs/README.md`); `CHANGELOG.md` the
  running record of what shipped; `docs/ROADMAP.md` the version-gated plan.
- `.agent/` (gitignored) the working log; `renders/` (gitignored) generated output.

## 6. What is done vs pending

Done and verifiable now: 352 catalog rooms plus hidden content, 6 sims, 11+
games with a shared high-score table and daily seeds, the complete RPG spine
(levels to 42 with lore, locks, 18 trophies with pings, the Gauntlet run,
boons, daily streaks, resonances), the Studio (plot, animate, sing, in the
terminal and the window), Visual Eras (including PNG output), Music Engine A
(the seeded chiptune, `numinous tune`), GPU real-time fractals, live sound in
the app and CLI plus structured notation over MCP, the `forget` right for
players who are minds, and 30 MCP tools (29 play tools with full CLI parity for the games,
plus one local broadcast consent control;
challenge, predict, and cairn are MCP-first) so agents play the same content.
Pending for product 0.2 (critical path; see **Critical path right now** in
`docs/ROADMAP.md`): stranger hallway on Times Tables and Buffon engineered
ahas, arrival-card clarity with real humans, and plate beauty under real
window sizes. Soft-thin densify, bulk new rooms, and Phase B glow are not
substitutes for that human gate.

Also pending later gates: deeper held and causal interactions in other rooms,
representative physical-controller sessions, musician-led long-listening
review, physical clean-machine cross-platform proof, full Studio save/share
beyond the first CLI `.num` save/open slice, the music visualizer, and more
GPU room paths.
