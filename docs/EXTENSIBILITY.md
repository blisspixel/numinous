# Extensibility: community content with a hard safety boundary

How Numinous becomes an open, community-extensible game (people add rooms,
levels, and creations) without ever letting untrusted content compromise a
player's machine. This document is the design ruling for REVIEW.md's standing
law 19, "sandbox before community creation," and it answers STUDIO.md open
questions 1 and 2. Researched July 2026 against the current state of the art;
sources at the end.

## The one invariant

**Untrusted content is data or budgeted interpretation, never ambient
authority.** Safety failures in moddable games almost never come from the
content model; they come from the edges: archive extraction, URL handlers,
bytecode loaders, and installers that execute code. Numinous is unusually
well positioned because the `Room` contract is already a pure function
(deterministic render from seed and phase, `unsafe_code` forbidden, no I/O in
core). Community content keeps that shape.

## The three tiers

### Tier 1: data-only capsules (the sharing substrate; extend what exists)

The `.num` format is already Tier 1: a size-capped text file, a hand-written
strict parser, no code. It grows from "one expression plus range parameters"
into a **room manifest**: one or more expressions, named sliders with ranges,
a palette and Era choice, sound parameters drawn from fixed enums, and
metadata (title, author). Trusted engine code interprets everything; content
can only recombine primitives we already ship. This is the Baba Is You and
Doom-WAD model, and its safety record is perfect for a structural reason:
there is no code to escape with.

Where the ceiling bites: pure data cannot express new behavior, only new
parameterizations. The long-term goal is that Studio programs can become
rooms, so that wall arrives fast. Tier 1 is therefore the link-safe sharing
format, not the creative ceiling.

Hardening rules, all cheap, all in force from now:
- Keep the hand-written parser. No general deserializer for untrusted input.
- Byte-cap the whole file, cap every field, whitelist characters, reject
  control bytes.
- Treat `numinous://` links as hostile input of the highest severity: any web
  page can feed bytes into that parser with one click. Fuzz the parser
  (cargo-fuzz on the file and link paths), and never trigger side effects
  (sound, file writes) before an explicit in-app confirmation for linked
  content.
- Installation of any capsule is: copy one file into a directory. Nothing
  ever executes at install time.

### Tier 2: the Studio language is the mod language (the 2.0 centerpiece)

STUDIO.md's open question, bespoke DSL versus embedded scripting host, is
answered: **bespoke, grown from the existing expression engine in
`crates/core/src/studio.rs`.** The Studio pattern language becomes the way a
creator gives a room behavior, and the language itself is the sandbox. This
is the Starlark philosophy (hermetic, deterministic, no ambient I/O, safe by
construction) applied to a creative instrument instead of a config language.

Why bespoke wins:
1. **The language is the product.** The Studio ladder (expression bar up to
   pattern algebra) is a UX artifact a curious teenager types into a live
   bar. Lua or WASM cannot be that surface. We build this interpreter for
   first-party rooms anyway; the mod SDK being "the Studio, shared" means
   there is no second language to sandbox.
2. **Safety by construction, not by fencing.** The evaluator has no
   filesystem, no network, no clock (phase is a host-supplied argument), no
   FFI, and lives under forbidden `unsafe_code`. There is nothing to escape
   to. Subtractive sandboxing of a general language is the alternative, and
   its residue bites even the most disciplined: Factorio, the gold standard
   of Lua modding, shipped a client RCE through its Lua bytecode verifier,
   fixed only in 1.1.101.
3. **Determinism is provable.** Pure math plus a seeded PRNG as host
   functions, so the validation story below applies to community content for
   free.

The evaluator requirements ARE the sandbox:
- **Totality:** `eval` never panics and never diverges; every operation is
  defined on all inputs; NaN and infinity are tolerated everywhere
  downstream (renderer clamps, melody quantizer clamps). Fuzzed continuously.
- **Budgets, checked at the door:** AST node cap at parse time (defeats
  expression blowup), a decrementing per-frame operation counter at eval
  time, a recursion/depth cap, and caps on any string or collection the
  pattern layer introduces. A tripped budget degrades gracefully: freeze the
  last good frame, gentle inline nudge, never a crash and never punishment.
- **Cross-platform bit-exactness:** platform `libm` differs between OSes, so
  the evaluator routes transcendentals through the pure-Rust `libm` crate.
  Without this, frame-hash validation across CI and player machines is a
  lie. This is the most commonly missed determinism landmine in the field.

### Tier 3: WASM component rooms (2.0+, portal-only, the pressure valve)

For the rare author who outgrows the pattern language: WebAssembly components
on **wasmtime**, never native code. The `Room` trait is nearly a WIT
interface already (render from seed and phase, sound spec, pokes); Zed proves
the exact pattern in production (wasmtime, WIT-defined versioned API), and
Veloren proves WASM plugins in a game.

Non-negotiable configuration:
- **No WASI at all.** The module's only imports are the room interface.
  Absence of imports is the capability system; there is no filesystem to
  forget to deny.
- **Fuel metering** for deterministic validation runs, **epoch interruption**
  for cheap live deadlines, and a **memory cap** on the store. All three are
  first-class wasmtime features.
- **NaN canonicalization on**, threads off, so Tier 3 rooms are
  hash-verifiable like Tier 2.
- Tier 3 capsules arrive only through the curated portal, never via
  `numinous://` links.
- Only standard `.wasm` that wasmtime validates and compiles itself. No
  precompiled artifacts from users, ever: accepting precompiled bytecode
  moves the verifier into the trusted base, and verifiers fail (the Factorio
  lesson).

### What never ships

- Native code plugins (dlopen/DLL). Once allowed, never revocable; every
  real mod-malware incident traces to this pattern.
- Installers that execute anything.
- Raw WGSL from untrusted sources. First-party and locally-authored shaders
  only; untrusted GPU work goes through the safe pipeline exclusively.
- Precompiled bytecode of any kind from users.

## Trust and distribution

The sandbox is the security boundary; everything else is provenance and
quality. Never the other way around.

1. **Capsule = one text file** for Tiers 1 and 2. A single size-capped text
   file has no zip-slip surface, is diffable, and is inspectable by anyone.
   If a pack format ever becomes necessary, extraction sanitizes entry names
   (no absolute paths, no `..`, no symlinks); even Zed shipped a zip slip in
   extension extraction, the class is evergreen.
2. **Determinism is the validation engine.** A submission is a proof packet:
   source, seeds, frame hashes at canonical (seed, phase, size) triples, an
   audio-spec hash, and declared budgets. Portal CI re-renders headlessly
   (the CLI face exists for exactly this) and rejects on hash mismatch,
   budget overrun (fuel-metered, so reproducible), panic under the fuzz
   corpus, or a perf-floor failure. Same seed, same output turns "is this
   content well behaved" from a trust question into a computation.
3. **Curation for beauty, signatures for provenance.** Human curation
   protects the beauty bar (already the 2.0 plan). The portal signs approved
   capsules (ed25519); the client verifies and labels ("Curated" vs
   "Unverified"). Critically, an unsigned capsule still runs in the same
   sandbox with the same budgets. Signatures label trust; they never grant
   capability, because the moment a signature unlocks a weaker sandbox, the
   signing key becomes a remote-code-execution key.
4. **Links are the lowest tier.** `numinous://` carries Tiers 1 and 2 only,
   size-capped, opening into a paused preview until confirmed.
5. **Any future shared context** pins content sets by checksum so every
   participant runs byte-identical content (the Factorio multiplayer model).

## Crate choices (when each tier builds)

| Concern | Choice | Why |
|---|---|---|
| Tier 2 interpreter | our own, extending `studio.rs` in core | hermetic by construction; zero new trusted dependencies |
| Float determinism | `libm` | bit-exact transcendentals across OSes |
| Tier 3 runtime | `wasmtime` + `wit-bindgen` | fuel + epochs + memory limits, component model, Zed precedent; behind a feature flag in a face-adjacent crate, never in core |
| Signing | `ed25519-dalek` | boring, audited, tiny |
| Fuzzing | in-suite seeded totality harness now (stable gate), `cargo-fuzz` on parse/file/link/eval as the nightly deepening | the parser edge is the real attack surface today; a deterministic stress harness over `parse`/`from_num_file`/`from_link` runs every commit (never panic, always terminate, caps always bite, valid creations round-trip losslessly), and a nightly cargo-fuzz run needs a nightly toolchain the project does not pin |
| Rejected: Rhai/Lua/Starlark in core | | a second language to sandbox, subtractive fencing, and a UX we would fight; Luau via mlua reconsidered only if courting existing modder muscle memory ever matters |

## Sequencing (mirrors ROADMAP.md)

- **Now / 1.x:** Tier 1 hardening. The parser budgets (token and depth caps),
  the import byte and magnitude caps, and the in-suite totality stress harness
  are done and gate every commit; the link-preview confirmation, a nightly
  cargo-fuzz run, and the `.num` room-manifest extension remain. These protect
  surfaces that already exist.
- **2.0:** Tier 2, the Studio pattern language as the creation surface, with
  the gallery, fork/remix, promote-to-room, the proof-packet portal CI, and
  signing. This is "The Living World" milestone's spine.
- **2.0+:** Tier 3 WASM rooms, designed as a WIT mirror of the `Room` trait
  now, built when author demand proves out.

## Sources

- Factorio Lua bytecode RCE: https://memorycorruption.net/posts/rce-lua-factorio/
- Factorio determinism and mod portal: https://wiki.factorio.com/Desynchronization and https://www.factorio.com/blog/post/fff-141
- wasmtime fuel/epochs/limits: https://docs.wasmtime.dev/api/wasmtime/struct.Config.html
- Zed extensions (wasmtime + WIT): https://zed.dev/blog/zed-decoded-extensions
- Zed extension zip slip advisory: https://github.com/zed-industries/zed/security/advisories/GHSA-v385-xh3h-rrfr
- Veloren WASM plugins: https://book.veloren.net/contributors/modders/writing-a-plugin.html
- Rhai safety limits: https://rhai.rs/book/safety/
- starlark-rust (principles borrowed, crate rejected): https://github.com/facebook/starlark-rust
- Baba Is You data-only levels: https://babaiswiki.fandom.com/wiki/Levelpack_Data
- Screeps metered isolation (background knowledge, not re-verified): https://docs.screeps.com/architecture.html
