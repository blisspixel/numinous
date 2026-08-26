# Performance Evidence

Performance claims in Numinous are attached to a named workload, machine,
revision, and measurement boundary. A dependency update is not called neutral
because it compiles. Comparable evidence must exercise the exact revisions on
one machine and retain the raw samples.

## August 2026 Sensory Lift post comparison

The disabled-by-default `gpu-post` feature tests the proposed post stack before
it changes the App. Revision `5282956ab6b55e5b99f704892b640fb36c9e2dc3`
uploads a deterministic sRGB frame, samples it into a full-resolution linear
`Rgba16Float` target, performs a half-resolution bright pass and separable
Gaussian bloom, then tone maps into an sRGB output. An equivalent
single-threaded CPU reference performs the same stages in reusable `f32`
buffers. The emissive input includes sharp beams, and optional PNG previews
make the claimed neighboring glow inspectable. Both paths reuse their
frame-sized resources.

The reference run used the release profile and locked dependencies on an AMD
Radeon 780M through Vulkan. It retained three warmups and twenty samples at
both target sizes. The device boundary covers the five GPU render passes. The
GPU wall boundary additionally covers the prebuilt host frame upload, final
texture copy, map, and tight RGBA output. The CPU boundary covers its complete
post stack through a tight RGBA output.

| Size | GPU device p50 / p95 ms | GPU wall p50 / p95 ms | CPU p50 / p95 ms | Result |
| --- | ---: | ---: | ---: | --- |
| 1920 x 1080 | 2.437 / 2.558 | 8.011 / 9.961 | 36.505 / 38.151 | GPU pass, CPU fail |
| 2560 x 1440 | 3.259 / 4.431 | 12.260 / 13.709 | 74.449 / 83.955 | GPU pass, CPU fail |

The p95 budgets are 8 ms GPU device and 33 ms full boundary at 1080p,
then 12 ms and 50 ms at 1440p. The CPU reference uses the corresponding full
boundary budget.

The full receipt is
[`evidence/sensory-post-spike-2026-08-25.json`](evidence/sensory-post-spike-2026-08-25.json).
It records every sample, output identities, the exact implementation revision
and binary digest, format capabilities, driver, toolchain, power scheme,
budgets, and scope limits. Reproduce the workload with:

```
cargo run --release --locked -p numinous-gpu --all-features --example post_spike -- --warmups 3 --samples 20 --check
```

This clears GPU feasibility on the reference integrated adapter and rejects
the measured single-threaded CPU implementation for production. It does not
claim that every possible CPU implementation fails. The direct presentation
decision is measured in the next section. Display scan-out, cross-platform
behavior, catalog-room aesthetics, accessibility, and preference remain
outside this receipt.

## August 2026 Sensory Lift direct presentation

Revision `b1dd42e9fa50ecd29362b96a3a2f6d7fd52575dc` adds a second consumer of the
same five-pass post stack. It safely creates a real window surface, selects a
compatible adapter and sRGB format, requests FIFO pacing with one frame in
flight, and writes the final tone-map pass directly into the acquired surface
texture. The direct path does not allocate the validator's offscreen output
texture or readback buffer.

The locked release example ran on the same Windows AMD Radeon 780M Vulkan
reference. Each workload used 30 warmups and 120 retained samples. The acquire
boundary reports swapchain wait alone. The render-and-present boundary starts
after acquisition and covers host upload, command encoding, queue submission,
and the queue presentation request. The combined boundary covers both.

| Size | Acquire p50 / p95 ms | Render and present p50 / p95 ms | Combined p50 / p95 ms | Result |
| --- | ---: | ---: | ---: | --- |
| 1920 x 1080 | 13.891 / 15.251 | 2.309 / 2.834 | 16.196 / 17.447 | Pass |
| 2560 x 1440 | 13.142 / 15.339 | 2.984 / 3.671 | 16.021 / 18.378 | Pass |

The combined p95 budgets remain 33 ms at 1080p and 50 ms at 1440p. Neither run
encountered a transient acquisition or suboptimal frame. FIFO acquisition
dominates the combined result, so this is a paced presentation boundary, not an
unpaced throughput claim.

The full receipt is
[`evidence/sensory-surface-spike-2026-08-25.json`](evidence/sensory-surface-spike-2026-08-25.json).
It retains every segment sample, exact implementation and binary identity,
driver, toolchain, window-size contract, budgets, decision, and scope limits.
Reproduce each workload with the commands recorded in that receipt. The general
form is:

```
cargo run --release --locked -p numinous-gpu --features gpu-post --example surface_spike -- --width 1920 --height 1080 --warmups 30 --samples 120 --budget-ms 33 --check
```

This selects direct `wgpu` surface output as the production candidate for the
full Sensory Lift. The disabled App `gpu-post` feature now feeds its real room
rasters through that renderer, while explicit recovery retains the default
software path on failure. The next gate is Windows, macOS, and Linux correctness
and pacing evidence before promotion. The receipt stops when the queue
presentation request returns and does not include compositor work, display
scanout, input latency, human perception, or room aesthetics. The 1440p client
area was exact but extended beyond the reference machine's 2256 x 1504 desktop.

## App platform proof contract

The `sensory_platform` example closes the portable runtime half of that next
gate without overstating hosted runner timing. It renders the same deterministic
Times Tables room, input feedback, room chrome, audio badge, spectrum meter, and
Modern Era transform used by the App, then passes those bytes through the
production frame fitter, recovery state machine, and direct surface presenter.
The receipt binds the source frame twice by SHA-256, the running executable,
revision when available, adapter class and driver, negotiated sRGB format and
FIFO mode, every presentation outcome, and raw acquire, render-and-present, and
combined samples.

The existing Windows, macOS, and Linux build matrix runs the portable class and
retains one JSON artifact per operating system. That class proves only that the
exact production boundary completed on the recorded runtime. Its timing field
is always `informational-only`; a hosted software adapter is useful portability
evidence and is not physical GPU performance evidence.

Run that same correctness boundary locally with:

```
cargo run --locked -p numinous-app --features gpu-post --example sensory_platform -- --out sensory-app-platform.json --check
```

The physical class is deliberately harder to invoke. It requires a locked
release build outside GitHub Actions, an integrated or discrete GPU, a full
revision, named machine and OS version, AC power, at least 30 warmups and 120
retained samples, zero skipped and suboptimal frames, and an explicit p95
budget. Only the declared 1920 by 1080 at 33 ms and 2560 by 1440 at 50 ms
contracts qualify, and the window manager must grant the exact requested client
area. A 1080p reference command has this form:

```
cargo run --release --locked -p numinous-app --features gpu-post --example sensory_platform -- --out sensory-app-windows-1080p.json --width 1920 --height 1080 --warmups 30 --samples 120 --physical --machine "MACHINE" --os-version "OS VERSION" --power-state ac --revision FULL_COMMIT_SHA --budget-ms 33 --check
```

Each passing physical receipt is still a candidate for one named reference,
not a universal performance result. Promotion needs a reviewed physical set at
both target sizes for Windows, macOS, and Linux. Compositor completion, scanout,
input latency, and perceptual quality remain outside both classes.

## July 2026 dependency migration

The migration is the adjacent revision pair below. The after revision includes
the dependency updates and the compatibility changes required by those updates,
so this evidence measures the complete migration commit. It does not isolate a
single library as the cause of a timing change.

- Before: `b47303d742c795540eb08a9c0e70a7e391a47978`
- After: `301eac6943fb44ff00316c7b0994e8d8cc505455`
- Machine: AMD Ryzen 7 7840U class Windows laptop, 64 GiB, AC power
- GPU path: AMD Radeon 780M through Vulkan
- Audio path: Realtek stereo output at 48 kHz
- Build: each revision's locked dependencies and pinned Rust toolchain, release
  profile, incremental compilation disabled
- Sampling: three warmups and twenty retained samples per revision, in
  alternating AB and BA order

The full canonical receipt is
[`evidence/dependency-migration-2026-08-02.json`](evidence/dependency-migration-2026-08-02.json).
It retains every sample, binary digest, output identity, environment field,
threshold, and recomputed verdict. The receipt also binds the exact recorder and
verifier source by SHA256.

| Workload | Before p50 / p95 ms | After p50 / p95 ms | Median change | Result |
| --- | ---: | ---: | ---: | --- |
| CLI request | 17.240 / 21.839 | 17.640 / 28.052 | +0.400 ms, 1.023x | Pass |
| GPU postcard | 545.612 / 633.035 | 636.489 / 813.576 | +90.876 ms, 1.167x | Pass |
| Audio device discovery | 10.413 / 12.289 | 17.530 / 20.456 | +7.117 ms, 1.683x | Pass |
| App visible window | 46.689 / 86.706 | 45.171 / 65.609 | -1.518 ms, 0.967x | Pass |

The comparison uses median gates because desktop scheduling produced isolated
tail outliers that are visible in the retained p95 and maximum samples. A
workload fails only when its after median exceeds both its relative guard and
its small absolute guard. This prevents a few milliseconds of fixed device
discovery cost from masquerading as a product-scale regression while still
blocking material slowdowns.

| Workload | Relative guard | Absolute guard |
| --- | ---: | ---: |
| CLI request | 1.25x | +5 ms |
| GPU postcard | 1.25x | +50 ms |
| Audio device discovery | 1.50x | +10 ms |
| App visible window | 1.35x | +20 ms |

The GPU migration added 16.7 percent to this complete one-shot postcard path,
which remains inside the declared 25 percent comparison guard. Audio device
discovery added 7.1 ms. The muted App process-to-visible-window boundary
remained flat, as did the byte-identical CLI render. The window appears before
the App initializes audio and later subsystems, so these measurements do not
establish the migration's effect on end-to-end App readiness.

## Verification and reproduction

Receipt verification is fast, deterministic, cross-platform, and runs in the
local and CI gates. On Windows:

```
python scripts/dependency-migration-performance.py --verify-receipt docs/evidence/dependency-migration-2026-08-02.json
```

On macOS or Linux:

```
python3 scripts/dependency-migration-performance.py --verify-receipt docs/evidence/dependency-migration-2026-08-02.json
```

Recording new historical evidence requires a Windows desktop, default audio
output, working GPU adapter, AC power, Git, Python 3.11 or newer, and both
commits' Rust toolchains. All temporary checkouts, targets, profiles, and output
stay under the ignored `.agent/` directory:

```
python scripts/dependency-migration-performance.py --record .agent/dependency-migration-performance.json --work-dir .agent/dependency-migration-performance --warmup 3 --samples 20
```

Generation checks out the two exact commits as detached worktrees, injects one
identical audio discovery probe, builds four locked release targets per
revision, alternates executions, closes each App window cleanly, writes the
receipt outside cleanup-owned scratch, and removes only its marked worktrees.
The verifier requires the exact three-warmup, twenty-sample Windows reference
contract, strict per-workload fields, pinned machine, toolchain, output, and
device identities, well-formed retained binary digests, and the exact runner
bytes. It recomputes every summary, guard, and verdict from the retained samples.

## Evidence limits

This is one Windows reference-machine comparison, not a cross-platform or
population benchmark. The GPU path includes process startup, adapter discovery,
dispatch, readback, and PNG encoding, not steady-state frame pacing. The audio
path stops after default output configuration discovery and does not measure
stream startup, callback latency, drift, glitches, or listening quality. The App
boundary stops when its native top-level window becomes visible, before audio
initialization, and does not claim end-to-end readiness, first paint, display
scan-out, sound, input latency, or human perception. The CLI boundary includes
process startup and one deterministic render.

The receipt is tracked evidence, not a signed laboratory attestation. Its
verifier detects malformed structure, pinned environment or output identity
drift, and inconsistent derived results. It cannot independently prove that a
coherent set of timing samples or binary digests was observed rather than
replaced. Review and Git history provide the durable integrity record. A
stronger provenance claim requires an external signed timestamp or measurement
witness.

Current `main` contains substantial work after the measured migration commit,
so these numbers close the historical migration evidence item rather than
describing current release performance. Current flagship raster evidence lives
in `QUALITY.md`. Physical Windows, macOS, and Linux performance, device soak,
and sensory latency remain separate roadmap work.

## Standing migration rule

Every future direct major dependency update must name adjacent before and after
revisions, preserve equivalent workloads and outputs, use release builds with
locked dependencies, alternate enough samples on one declared machine, retain
raw samples and binary identities, explain every material change, and keep the
receipt verifier in CI. Platform-specific dependencies additionally require
the appropriate native hardware run before a release claim expands to that
platform.
