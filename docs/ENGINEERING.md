# Engineering Standards & Code Quality

The bar: **code a PhD CS professor would be proud of.** Every decision defensible from first principles, no cargo-culting, correctness and clarity and simplicity over cleverness. The rigor has to be visible in the code the way it is visible in the math (see `VISION.md` on PhD-real rigor). This doc is the standing engineering contract; the automated enforcement of it lives in `QUALITY.md`.

## Toolchain and versions (latest GA as of July 2026)

Pin these exactly; **re-verify the exact current stable at project kickoff**, since the ecosystem moves and a few of these will have advanced by the time 0.1 starts.

| Component | Version (July 2026 GA) | Notes |
| --- | --- | --- |
| Rust edition | **2024** | The current edition; use it from the first commit. |
| Rust toolchain | **latest stable (1.9x series)** | Pin exactly in `rust-toolchain.toml`; this is also the MSRV floor. Verify the exact current stable at kickoff. |
| `wgpu` | **29.x** | The portable GPU stack (Vulkan/Metal/DX12). See `ARCHITECTURE.md`. |
| `bevy` (if used) | **0.19** | Only if we take the engine route; decided by the 0.1 spike. Bevy tracks wgpu, so versions move together. |
| `cpal`, `fundsp` | latest GA | Native audio I/O + DSP (see `SOUND.md`, `MUSIC.md`). |
| Test runner | **cargo-nextest** | Faster, per-test process isolation. |
| Property testing | **proptest** (1.x) | Invariants and metamorphic tests (`QUALITY.md`). |
| Snapshot testing | **insta** (1.x) | Deterministic output snapshots. |
| Benchmarks | **criterion** | Hot-path perf, regression tracking. |
| Supply chain | **cargo-deny**, **cargo-audit**, **cargo-auditable** | Policy, RustSec advisories, auditable shipped binaries. |
| Coverage | **cargo-llvm-cov** | Tracked, not fetishized (see Testing). |

**Version hygiene:** commit `Cargo.lock`, build with `--locked` in CI, centralize versions in `[workspace.dependencies]`, and keep pins current with a controlled update cadence (renovate/dependabot into CI, never blind-bumped).

## Formatting and linting (zero-warning policy)

- **`rustfmt`** on everything; `cargo fmt --all --check` is a blocking CI gate. No unformatted code merges, ever. Style is not up for per-PR debate.
- **`clippy` at a strict level:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Enable `clippy::pedantic` and selected `nursery`/`cargo` lints; opt *out* deliberately, per case, with a written reason.
- **`-D warnings` project-wide.** A warning is a bug that has not been fixed yet. CI is red on any warning.
- **Prefer `#[expect(lint, reason = "...")]` over `#[allow(...)]`** so that a suppression which is no longer needed becomes an error and gets cleaned up. Every suppression carries a `reason`.

## Language and idioms (edition 2024)

- **Make illegal states unrepresentable.** Lean on the type system: newtypes over raw primitives (no primitive obsession, especially for units, ids, seeds, and mathematical quantities), enums for state, `#[non_exhaustive]` on public enums/structs where forward-compat matters.
- **All public types derive `Debug`** (and `Clone`/`PartialEq`/`Eq`/`Hash` where sensible). Follow the Rust API Guidelines for naming and common-trait implementation.
- **Errors are typed at library boundaries, contextual at the app.** Library crates return concrete error enums (`thiserror`-style); binaries/faces use a context-carrying error (`anyhow`/`eyre`-style). 
- **No `unwrap`/`expect` in production paths.** A clippy lint bans them outside tests; the only exceptions are provably-infallible calls, each annotated with a `// SAFETY of unwrap:`-style justification. Panics are reserved for genuine, documented invariant violations, never for expected failure.
- **Prefer expressions, iterators, and exhaustive matches** over imperative sprawl; small pure functions; clear ownership; borrow over clone unless clarity wins.

## Unsafe-code policy

- **`#![forbid(unsafe_code)]` by default in every crate.** Because `wgpu`, `cpal`, and friends encapsulate the FFI, the overwhelming majority of our code should be 100% safe.
- **Any exception is a deliberate, isolated event:** confined to a small, clearly named module that opts back in with a documented reason; every `unsafe` block carries a `// SAFETY:` comment proving the invariant it upholds; it gets extra review and, where feasible, **Miri** in CI. Unsafe is never casual.

## Workspace and module architecture

- A **Cargo workspace** with the crate layout in `ARCHITECTURE.md`, headless `core`, `rooms/*`, and the three `faces/*`, with shared versions in `[workspace.dependencies]`.
- **Enforce the dependency rules:** `rooms/*` depend on `core` only; the faces (`app`/`cli`/`mcp`) depend on `core`, never on each other. Consider `cargo-modules`/an architecture test to keep the graph honest.
- **Minimal public surface:** `pub(crate)` by default, `pub` deliberately; small, cohesive modules; no god-modules.

## Documentation standards

- **`#![deny(missing_docs)]` on library crates.** Every public item has a doc comment; every crate has a `//!` overview.
- **Doctests are real tests** and run in CI; examples must compile and pass.
- **`cargo doc` builds clean under `-D warnings`;** broken intra-doc links fail CI.
- **Architecture Decision Records (ADRs)** for consequential choices (the stack, Bevy-vs-bespoke, the Studio DSL, the sandbox model). A decision without a recorded rationale is a future argument waiting to happen.
- Comments explain **why**, not what; the code says what.

## Testing (the enforcement of `QUALITY.md`)

- **Runner:** `cargo-nextest` for speed and isolation.
- **Layers:** unit + integration tests; **proptest** for invariants and metamorphic properties; **insta** snapshots for deterministic output; **golden-reference** tests for GPU math (the-math-is-the-oracle, `QUALITY.md`); **criterion** for benchmarks. These *are* the commit-loop in `QUALITY.md`.
- **Determinism is mandatory:** seeded RNG only, never ambient randomness or wall-clock time in logic; same seed, same result (so shares and tests reproduce exactly).
- **Coverage** is tracked with `cargo-llvm-cov` and read as a *smell detector*, not a target. The real metric is meaningful assertions on real behavior, not a percentage. We do not write tests to move a number.

## Performance discipline

- **Measure before optimizing.** `criterion` benchmarks on hot paths; the 60/120fps floor is a CI-tracked budget (`QUALITY.md` nightly soak/perf). No optimization lands without a benchmark showing it helped.
- **Documented release profile** (LTO, `codegen-units`, panic strategy, opt-level) with the reasoning; a `bench`/`profiling` profile for flamegraphs.
- Prefer clear code that the compiler optimizes well over hand-rolled cleverness; reach for `unsafe`/SIMD only with a benchmark that justifies it, behind the unsafe policy above.

## Supply chain and dependency hygiene

- **`cargo-deny`** enforces license policy, banned/duplicate crates, and trusted sources; **`cargo-audit`** checks the RustSec advisory database. Both run on every push and block on real findings (tuned with `--deny` severities so noise does not).
- **`cargo-auditable`** embeds the dependency manifest in shipped binaries so they can be audited post-hoc.
- **Dependencies are minimal and vetted.** Each new dependency is justified in review; prefer well-maintained, widely-used crates; give extra scrutiny to unsafe-heavy deps (a CVE in an unsafe-heavy crate is more urgent than the same score in pure-safe code).

## CI gates (the merge bar)

Nothing merges red. On every PR, blocking:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo nextest run --workspace` (unit + property + integration)
4. the `QUALITY.md` commit-loop: golden-reference, determinism, visual-regression, audio-regression, and the **style guard** (no emojis, no em-dashes, no AI/tool attribution)
5. `cargo doc --workspace --no-deps` under `-D warnings`
6. `cargo deny check` and `cargo audit`
7. build succeeds on macOS, Linux, and Windows (and on a non-NVIDIA GPU in the nightly cross-GPU job)

The nightly loop adds soak/perf and cross-GPU differential tests (`QUALITY.md`).

## Git and review hygiene

- **Small, focused PRs**, one concern each; a clear, imperative commit subject and a body that explains the why.
- **Clean commit messages and PR descriptions with no tool/AI attribution** (project rule).
- **Every change is reviewed;** math-touching changes additionally pass the math-correctness gate (human-mathematician sign-off, `QUALITY.md`).
- **Branch protection:** green CI required to merge; `Cargo.lock` committed and consistent.

## The professor's test (the ethos, in one paragraph)

Could you stand at a whiteboard and defend every non-obvious line from first principles? Is the simplest thing that is correct also what shipped? Does the type system make the wrong thing impossible rather than merely discouraged? Is every public thing documented, every claim tested, every dependency justified, every unsafe block proven? If yes, a PhD CS professor would be proud, and, more to the point, a mind that may one day read this code and be far smarter than us (see `DIGITAL_MINDS.md`) would find nothing to be embarrassed by.
