# Engineering Standards & Code Quality

The aim is code that can be defended from first principles: correctness,
clarity, and simplicity over cleverness. That is an aspiration, not a claim of
perfection. This document separates gates enforced today from hardening work
that still has to earn its place. The automated enforcement that exists lives
in `QUALITY.md` and `.github/workflows/ci.yml`.

## Toolchain and versions (verified 2026-07-18)

The current baseline is deliberate and green. A newer major release is a review
candidate, not an automatic upgrade. Patch updates within the lockfile are kept
current through Dependabot and must pass the full gate.

| Component | Enforced baseline | Notes |
| --- | --- | --- |
| Rust edition | **2024** | The current edition; use it from the first commit. |
| Rust toolchain | **1.97.1** | Exact developer and CI toolchain. CI separately checks the verified 1.88 MSRV. |
| `wgpu` | **30.0.0** | Current GPU stack. The migration preserves unbucketed adapter limits and handles mapped-range failures as typed errors. |
| `winit`, `softbuffer` | **0.30.x, 0.4.x** | Current native window and software presentation path. |
| `cpal` | **0.18.1** | Current native audio I/O. Every PCM format is converted from the shared float mix; DSD remains explicitly unsupported. |
| `png`, `pollster`, `ureq` | **0.18.1, 1.0.1, 3.3.0** | Current image, blocking-future, and synchronous HTTP baselines. HTTP redirects remain disabled for the credentialed music request and error bodies remain bounded. |
| `gilrs` | **0.11.2** | Current cross-platform gamepad input. Linux CI installs `libudev-dev`. |
| Test runner | **cargo test** | Enforced today. `cargo-nextest` is a possible speed improvement, not a current dependency. |
| Supply chain | **cargo-deny** and **cargo-audit** | Both enforced in CI. Deny covers advisories, licenses, bans, and sources; audit is the independent RustSec path with ignores in `.cargo/audit.toml`. `cargo-auditable` release binaries remain planned hardening. |
| Coverage | **cargo-llvm-cov** | Tracked, not fetishized (see Testing). |

**Version hygiene:** commit `Cargo.lock`, build with `--locked` in CI,
centralize shared versions in `[workspace.dependencies]`, and update through
reviewed changes. The 2026-07-18 audit migrated every direct dependency with a
newer general-availability line and refreshed all compatible transitive
packages. `cargo update --dry-run --verbose` reports no remaining update.
Dependabot watches Cargo and action releases without migration-era ignores.
The release evidence comes from the official
[`wgpu` 30.0.0 release](https://github.com/gfx-rs/wgpu/releases/tag/v30.0.0),
[`cpal` 0.18.1 release](https://github.com/RustAudio/cpal/releases/tag/v0.18.1),
and the published crate records for
[`png` 0.18.1](https://crates.io/crates/png/0.18.1),
[`pollster` 1.0.1](https://crates.io/crates/pollster/1.0.1), and
[`ureq` 3.3.0](https://crates.io/crates/ureq/3.3.0).

## Formatting and linting (zero-warning policy)

- **`rustfmt`** on everything; `cargo fmt --all --check` is a blocking CI gate. No unformatted code merges, ever. Style is not up for per-PR debate.
- **`clippy` at a strict level:** current CI runs `cargo clippy --workspace --all-targets -- -D warnings`. `--all-features`, selected `pedantic`, `nursery`, and `cargo` lints are hardening targets; opt out deliberately, per case, with a written reason.
- **`-D warnings` project-wide.** A warning is a bug that has not been fixed yet. CI is red on any warning.
- **Prefer `#[expect(lint, reason = "...")]` over `#[allow(...)]`** so that a suppression which is no longer needed becomes an error and gets cleaned up. Every suppression carries a `reason`.

## Language and idioms (edition 2024)

- **Make illegal states unrepresentable.** Lean on the type system: newtypes over raw primitives (no primitive obsession, especially for units, ids, seeds, and mathematical quantities), enums for state, `#[non_exhaustive]` on public enums/structs where forward-compat matters.
- **All public types derive `Debug`** (and `Clone`/`PartialEq`/`Eq`/`Hash` where sensible). Follow the Rust API Guidelines for naming and common-trait implementation.
- **Errors are typed at library boundaries, contextual at the app.** Library crates return concrete error enums (`thiserror`-style); binaries/faces use a context-carrying error (`anyhow`/`eyre`-style). 
- **Avoid `unwrap` and `expect` in production paths.** Expected failures return
  errors. A workspace-wide lint does not enforce this yet, so promoting the
  relevant Clippy lints requires an inventory and deliberate exceptions first.
- **Prefer expressions, iterators, and exhaustive matches** over imperative sprawl; small pure functions; clear ownership; borrow over clone unless clarity wins.

## Unsafe-code policy

- **`#![forbid(unsafe_code)]` by default in every crate.** Because `wgpu`, `cpal`, and friends encapsulate the FFI, the overwhelming majority of our code should be 100% safe.
- The current workspace policy permits no local exception. A future need for
  unsafe code would require changing the workspace policy in a separately
  reviewed architectural decision, with focused tests and Miri where applicable.

## Workspace and module architecture

- A **Cargo workspace** with the crate layout in `ARCHITECTURE.md`, headless `core`, `rooms/*`, and the three `faces/*`, with shared versions in `[workspace.dependencies]`.
- **Enforce the dependency rules:** `rooms/*` depend on `core` only; the faces (`app`/`cli`/`mcp`) depend on `core`, never on each other. Consider `cargo-modules`/an architecture test to keep the graph honest.
- **Minimal public surface:** `pub(crate)` by default, `pub` deliberately; small, cohesive modules; no god-modules.

## Documentation standards

- `numinous-core` denies missing documentation. The other library crates inherit
  the workspace warning today. Promoting them to deny is unfinished hardening.
- **Doctests are real tests** and run in the local gates and CI; examples must
  compile and pass.
- **`cargo doc` builds clean under `-D warnings`;** the local gates and required
  CI quality job reject broken intra-doc links and other rustdoc warnings.
- **Architecture Decision Records (ADRs)** for consequential choices (the stack, Bevy-vs-bespoke, the Studio DSL, the sandbox model). A decision without a recorded rationale is a future argument waiting to happen.
- Comments explain **why**, not what; the code says what.

## Testing (the enforcement of `QUALITY.md`)

- **Runner:** `cargo test --workspace --all-targets --locked` is the current
  enforced runner. This includes example-target tests, such as the screen
  matrix's structural-oracle regressions. Consider `cargo-nextest` only when
  measured CI time justifies another tool.
- **Layers enforced now:** unit and integration tests, deterministic fixtures,
  direct invariants, hostile-input tests, end-to-end stdio coverage, and one
  release-profile performance harness for the five 0.3 flagships. Broader
  property-test, snapshot, GPU-golden, and nightly benchmark systems remain
  roadmap work until their dependencies and workflows exist in the repository.
- **Determinism is mandatory:** seeded RNG only, never ambient randomness or wall-clock time in logic; same seed, same result (so shares and tests reproduce exactly).
- **Coverage** is tracked with `cargo-llvm-cov` and read as a *smell detector*, not a target. The real metric is meaningful assertions on real behavior, not a percentage. We do not write tests to move a number.

## Performance discipline

- **Measure before optimizing.** Use focused timings or `criterion` benchmarks
  on hot paths. The app currently enforces an adaptive 33 ms room-render target
  on the measured Windows machine. `scripts/flagship-perf.ps1` and
  `scripts/flagship-perf.sh` enforce that p95 target for five category flagships
  when run on declared reference hardware; broader CI performance and soak
  budgets are planned in `QUALITY.md`. No optimization lands without evidence
  that it helped.
- **Documented release profile** (LTO, `codegen-units`, panic strategy, opt-level) with the reasoning; a `bench`/`profiling` profile for flamegraphs.
- Prefer clear code that the compiler optimizes well over hand-rolled cleverness; reach for `unsafe`/SIMD only with a benchmark that justifies it, behind the unsafe policy above.

## Supply chain and dependency hygiene

- **`cargo-deny`** enforces advisory, license, ban, and source policy on every
  push. The local release gate runs it when installed; CI never skips it.
- **`cargo-audit`** is a second, independent RustSec advisory check. CI runs it
  on every PR. Local `scripts/verify` runs it when `cargo-audit` is installed.
  Project ignores live in `.cargo/audit.toml` and must stay aligned with the
  advisory ignores in `deny.toml` (today: build-time `quick-xml` via
  wayland-scanner only). **`cargo-auditable`** release binaries remain planned
  hardening, not a current gate.
- **Dependencies are minimal and vetted.** Each new dependency is justified in review; prefer well-maintained, widely-used crates; give extra scrutiny to unsafe-heavy deps (a CVE in an unsafe-heavy crate is more urgent than the same score in pure-safe code).

## Local threat model (security review baseline)

Numinous 0.2 targets a **local single-user desktop** deployment: the App, CLI,
and stdio MCP server run as ordinary user processes on the machine that owns
the play history. Hostile JSON on the MCP stdio boundary, oversized CLI input,
path and install provenance, and untrusted media decode are in scope. Remote
unauthenticated network MCP, multi-tenant OAuth, and cloud multi-player are
out of scope for this version unless productized later; claims about security
must name that boundary.

## CI gates (the merge bar)

Nothing merges red. On every PR, blocking:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`
4. `RUSTDOCFLAGS="-D warnings" cargo test --workspace --doc --locked`
5. `cargo test --workspace --all-targets --locked`
6. `bash scripts/check-style.sh`
7. `cargo deny check`
8. `cargo audit` (RustSec advisories; ignores in `.cargo/audit.toml`)
9. `cargo +1.88.0 check --workspace --all-targets --locked`
10. `cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex '(crates[\\/](gpu|audio)[\\/]|faces[\\/]app[\\/]src[\\/]main\.rs)'`
11. `cargo test --workspace --all-targets --locked` and
   `cargo build --workspace --locked` on macOS, Linux, and Windows
12. The native installer safety self-test on macOS, Linux, and Windows

Hardening targets not yet enforced in CI: `cargo-auditable` release binaries,
release artifact provenance, the visual and audio regression loops, and
real-hardware soak and performance jobs.

The nightly loop adds soak/perf and cross-GPU differential tests (`QUALITY.md`).

## Local enforcement (the pre-commit hook)

CI is the merge bar. The tracked pre-commit hook provides a faster local gate.
Enable it once per clone:

```
git config core.hooksPath scripts/hooks
```

`scripts/hooks/pre-commit` then blocks any commit that would fail the fast gate.
It runs the house-style guard on every commit (instant, and it applies to docs
as much as code), and the cargo gate (fmt, Clippy and rustdoc with warnings
denied, plus doctests and the full test suite) only when the commit touches
Rust, `Cargo.*`, or a shader, so a docs-only commit stays fast. Coverage, the
locked build, and artifact regeneration stay in
`scripts/verify.sh` (the release gate); they are too slow for every commit.
Emergency bypass is `git commit --no-verify`, after which you must run
`scripts/verify.sh` before pushing.

A note on why the hook matters beyond convenience: the house-style guard uses
`grep -P` with Unicode escapes, which silently aborts in a bare C/POSIX locale.
Before this was fixed (`scripts/check-style.sh` now forces a UTF-8 locale and
fails loudly if grep cannot run), the guard was a no-op in any shell with an
unset locale, so violations could land locally and only CI would catch them. A
wired, correct hook closes that gap.

## Git and review hygiene

- **Small, focused PRs**, one concern each; a clear, imperative commit subject and a body that explains the why.
- **Clean commit messages and PR descriptions with no tool/AI attribution** (project rule).
- **Review target:** math-touching changes need independent mathematical review
  before 1.0. That process is not yet staffed, so current claims remain bounded
  to tests and cited sources.
- **Branch protection target:** require green CI on the public default branch.
  Repository settings, not this document, are the evidence that it is active.

## The professor's test (the ethos, in one paragraph)

Could you stand at a whiteboard and defend every non-obvious line from first
principles? Is the simplest correct thing what shipped? Does the type system
prevent mistakes where practical? Is every public claim backed by a test,
measurement, source, or an honest label saying it is still a hypothesis? That is
the standard to keep working toward.
