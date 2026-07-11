# How to verify Numinous

Everything below has been built and checked on the dev laptop; this is how you
confirm it yourself efficiently. Nothing here needs a GPU or the internet after
the first dependency fetch.

## 0. Prerequisites

- **Rust** (edition 2024; pinned to 1.96.0 in `rust-toolchain.toml`). Install from
  <https://rustup.rs>. On Windows, cargo lands in `%USERPROFILE%\.cargo\bin`; if a
  fresh shell does not see `cargo`, add that to `PATH`.
- Optional, for the local coverage gate: `cargo install cargo-llvm-cov`.
- Optional, for the local supply-chain gate: `cargo install cargo-deny`.
- The Linux build of the audio crate needs ALSA headers: `sudo apt-get install -y libasound2-dev`.

## 1. One command

Run the full gate and regenerate every artifact:

- Windows: `scripts\verify.ps1`
- macOS / Linux: `bash scripts/verify.sh`

It runs format, clippy (deny warnings), tests, locked build, coverage (if
`cargo-llvm-cov` is present), supply-chain policy (if `cargo-deny` is present),
and the house-style guard, then writes images and audio into `renders/`.
If it prints "All checks passed" and exits 0, everything is green.

## 2. Or run the gates individually

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --locked
cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex '(crates[\\/](gpu|audio)[\\/]|faces[\\/]app[\\/]src[\\/]main\.rs)'
cargo deny check                         # if cargo-deny is installed; CI always runs it
bash scripts/check-style.sh                  # macOS / Linux
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-style.ps1  # Windows
```

Expected right now: **format and clippy clean, 968 tests pass, 91.28% region
cover, and 90.85% line cover** (the `gpu` and `audio` crates plus the app event-loop file are
integration-tested on real hardware and excluded from the coverage gate, see
`docs/QUALITY.md`).

## 2b. Put `numinous` on your PATH (once)

```
cargo install --path faces/cli --force
```

If a shell says numinous is not recognized, the cargo bin directory is
missing from PATH: add `%USERPROFILE%' + chr(92) + '.cargo' + chr(92) + 'bin` to the user PATH once
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
sweep, P saves the current room frame as a PNG postcard, F goes fullscreen, M mutes, B starts The Show (lean back), G deals the
quiz (name the math, right in the window), C plays today's Munch board with a
cursor (WASD moves, Space eats, Enter grades), N plays Nim against the Order
(aim with W/S and A/D, Enter takes; win and the xor secret shows), T runs the
Gauntlet (all four stages in sequence, combo and total at the end), J opens
your journey (level, rank, trophies, resonances), Tab opens the Studio (type math, watch and hear it
live). The app plays the same Journey the CLI does: entering rooms records
visits, quiz rounds record plays and wins, your level rides in the corner, and
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

**Sound:** every room is an instrument; write a WAV and play it:
```
cargo run --bin numinous -- sonify lissajous --out renders/lissajous.wav
cargo run --bin numinous -- sonify collatz  --out renders/collatz.wav
cargo run --bin numinous -- tune --seed 7 --out renders/chip.wav   # Music Engine A
```

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
cargo run --bin numinous -- forget              # see everything remembered; --confirm erases
```
Every game records plays and wins to the journey; level-ups cascade lore,
unlocks, boon banners, and trophy pings; dailies chain into streaks.

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
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"play_room","arguments":{"id":"times-tables"}}}
```
Run `cargo run --bin numinous-mcp` and paste those lines; it replies with the
tool list and an ASCII render of the room as text.

## 5. Where things are

- `crates/core` the headless engine: rooms (30 catalog rooms across 10 wings plus hidden content), sims, games
  (including nim and the chiptune composer), the Studio expression engine, the
  journey, scores, trophies, resonances, sound, eras, and the drawing surfaces.
- `crates/gpu` adaptive wgpu rendering; `crates/audio` adaptive cpal output.
- `faces/cli` the `numinous` binary; `faces/mcp` the `numinous-mcp` server.
- `docs/` the full design and plan (start at `docs/README.md`); `CHANGELOG.md` the
  running record of what shipped; `ROADMAP.md` the version-gated plan.
- `.agent/` (gitignored) the working log; `renders/` (gitignored) generated output.

## 6. What is done vs pending

Done and verifiable now: 30 catalog rooms across 10 wings plus hidden content,
6 sims, 11+ games with a shared high-score table and daily seeds, the
complete RPG spine (levels to 42 with lore, locks, 18 trophies with pings, the
Gauntlet run, boons, daily streaks, resonances), the Studio (plot, animate,
sing, in the terminal and the window), Visual Eras (including PNG output),
Music Engine A (the seeded chiptune, `numinous tune`), GPU real-time fractals,
live sound everywhere, the `forget` right for players who are minds, and
27 MCP tools (full CLI parity for the games; the challenge tool is MCP-first) so agents play the same content. Pending (see `ROADMAP.md`):
deeper room-specific pokes, human playtests, cross-platform proof, full Studio save/share beyond the first CLI `.num` save/open slice, the music visualizer, and more GPU room paths.
