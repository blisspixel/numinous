# How to verify Numinous

Everything below has been built and checked on the dev laptop; this is how you
confirm it yourself efficiently. Nothing here needs a GPU or the internet after
the first dependency fetch.

## 0. Prerequisites

- **Rust** (edition 2024; pinned to 1.96.0 in `rust-toolchain.toml`). Install from
  <https://rustup.rs>. On Windows, cargo lands in `%USERPROFILE%\.cargo\bin`; if a
  fresh shell does not see `cargo`, add that to `PATH`.
- Optional, for the coverage gate: `cargo install cargo-llvm-cov`.
- The Linux build of the audio crate needs ALSA headers: `sudo apt-get install -y libasound2-dev`.

## 1. One command

Run the full gate and regenerate every artifact:

- Windows: `scripts\verify.ps1`
- macOS / Linux: `bash scripts/verify.sh`

It runs format, clippy (deny warnings), tests, coverage (if `cargo-llvm-cov` is
present), and the house-style guard, then writes images and audio into `renders/`.
If it prints "All checks passed" and exits 0, everything is green.

## 2. Or run the gates individually

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex 'crates[\\/](gpu|audio)[\\/]'
bash scripts/check-style.sh
```

Expected right now: **format and clippy clean, ~124 tests pass, coverage ~95%
lines** (the `gpu` and `audio` crates are integration-tested on hardware and
excluded from the coverage gate, see `docs/QUALITY.md`).

## 3. See it work (the three faces)

**Terminal (ASCII), including the live animated view:**
```
cargo run --bin numinous -- rooms
cargo run --bin numinous -- describe times-tables
cargo run --bin numinous -- render chaos-game --width 50 --height 25
cargo run --bin numinous -- play times-tables      # live, animating; Ctrl+C to stop
```
`play` is the closest thing to "running the app" today (a live, animating room in
the terminal). The windowed GUI is the next milestone (0.2); see `ROADMAP.md`.

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
```

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

## 4. Where things are

- `crates/core` the headless engine: the `Room` trait, `Surface` (ASCII `Canvas`
  and pixel `Raster`), the registry, RNG, and `SoundSpec`, plus the nine rooms.
- `crates/gpu` adaptive wgpu rendering; `crates/audio` adaptive cpal output.
- `faces/cli` the `numinous` binary; `faces/mcp` the `numinous-mcp` server.
- `docs/` the full design and plan (start at `docs/README.md`); `CHANGELOG.md` the
  running record of what shipped; `ROADMAP.md` the version-gated plan.
- `.agent/` (gitignored) the working log; `renders/` (gitignored) generated output.

## 5. What is done vs pending

Done and verifiable now: the headless engine, nine rooms across four wings, three
faces (terminal / image / agent), per-room colors and sounds, adaptive GPU and
audio hello-worlds, and the quality gates. Pending (see `ROADMAP.md`): the
windowed GUI (0.2), GPU-accelerated real-time room rendering, and the music radio.
