# Mathematics: room truth and numerical evidence

This document owns the mathematical review of implemented rooms. Room design
lives in `ROOMS.md`, general testing in `QUALITY.md`, and priorities in
`ROADMAP.md`. A deterministic render establishes reproducibility, not the truth
of the depicted model. Catalog presence is not mathematical certification.

## What a room must make defensible

- State the governing object, equations, units or nondimensionalization, and
  admitted parameter range. Distinguish a physical model from a metaphor.
- Keep controls, initial conditions, readouts, sound mappings, and pictures
  attached to the same declared experiment. A control must not silently change
  an unrelated parameter.
- Check an independent mathematical consequence: conservation, symmetry,
  normalization, an analytic limit, an exact small case, or a known counterexample.
  A renderer compared only with itself cannot establish those consequences.
- For numerical evolution, declare the method, time step, horizon, error
  measure, and sampled domain. Check refinement. Long chaotic trajectories need
  not agree point by point when precision changes.
- Check geometry as well as equations. Axes, focus positions, aspect correction,
  sampled time, and the distinction between an arc and its chord can change the
  lesson even when an equation solver has a tiny residual.
- Keep theorem, conjecture, approximation, and empirical observation distinct
  in the reveal. Finite sampling cannot prove an infinite claim.

## September 2026 review

Scope: the Kepler Areas, Lotka-Volterra, Lorenz, and Spherical Harmonics
implementations, plus the shared numerical step used by Double Pendulum.
This is a focused source and
numerical review, not an independent sign-off of the 355-room catalog.

### Kepler Areas

For a centered ellipse, `x = a cos(E)`, `y = b sin(E)`, and
`M = E - e sin(E)` put perihelion at positive x and the sun at `+a e`.
The previous renderer put the sun at `-a e`. At `e = 0.2`, the new geometric
area test exposed a sector error of about 36 percent despite a passing solver
residual test. The focus now agrees with the timing; equal-time sectors use
orbital arcs. Surface aspect correction preserves a circle on pixels and text
cells. The speed wager treats only `e = 0` as circular, so a slightly elliptical
orbit no longer earns an incorrect SAME answer.
Small positive eccentricities retain a visible positive speed difference in
the graded text, even when floating-point addition or display rounding would
otherwise print a unit ratio.

The tests triangulate pre-rounding renderer positions about the actual sun and
compare six sectors with `pi*a*b/6` at `e = 0, 0.2, 0.6, 0.9`, with relative
quadrature error below `1e-5`. A separate finite-time displacement ratio checks
faster motion at the nearer apsis against `(1+e)/(1-e)`. Pixel rounding and
polygonal arc drawing remain visual approximations.

Source: [MIT 16.346, Lecture 3, Kepler's equation](https://ocw.mit.edu/courses/16-346-astrodynamics-fall-2008/379eae1a78cf9ad58247058dffbae3ac_lec_03.pdf),
especially the centered ellipse, focal distance, and area derivation.
Implementation and regressions: `crates/core/src/rooms/kepler_laws.rs` and
`kepler_aha.rs`.

### Lotka-Volterra

The room uses `x' = x(alpha - beta*y)` and
`y' = y(delta*x - gamma)`, with `beta = 0.5`, `delta = 0.4`,
`0.4 <= alpha <= 2.2`, and seeded `gamma` in `0.30, 0.32, 0.34, 0.36`.
Positive nonequilibrium solutions lie on closed levels of
`H = delta*x - gamma*ln(x) + beta*y - alpha*ln(y)`.

Forward Euler plus a population floor fabricated outward spirals. The first
sampled regression found an absolute H drift of about `0.0122`. RK4 in
`u = ln(x), v = ln(y)` now evolves positivity in the coordinates, with
`u' = alpha - beta*exp(v)` and `v' = delta*exp(u) - gamma`.
At step `0.01` over 48 time units, 144 sampled paths have maximum absolute
H drift about `1.25e-7`, below the `1e-6` test budget. This is a bounded
approximation, not exact conservation or a guarantee for arbitrary parameters.

All six paths now share one pair of axes. Repeating a hand action no longer
changes the predator death rate, and the equilibrium readout uses the same
`(gamma/delta, alpha/beta)` as the model. Tests also check stationarity,
positivity, and short-time fourth-order refinement.

Source: [Scholarpedia, Predator-prey model](https://www.scholarpedia.org/article/Predator-prey_model),
the model's conserved quantity and positive closed level curves.
Implementation and regressions: `crates/core/src/rooms/lotka_volterra.rs`.

### Lorenz and the shared integration method

Lorenz uses `x' = sigma*(y-x)`, `y' = x*(rho-z)-y`,
`z' = x*y-beta*z`, with `sigma = 10`, `beta = 8/3` and step `0.005`.
The background sweeps `rho` from 24 to 30; the named twin experiment uses
`rho = 28` over at most 45 time units. Replacing Euler with RK4 resolves the
analytic invariant-axis test: when `x = y = 0`, `z(t) = z(0)*exp(-beta*t)`.
The old step missed that one-second check by about `0.0247` from `z(0)=20`.
The test budget is now `1e-8`.

Other checks cover all three equilibria, half-turn symmetry, one-second step
refinement at `rho = 24, 28, 30`, the default `rho = 28, seed = 0` path bounded
after its four-unit transient, and the exact initial
twin perturbation. These checks do not certify the full chaotic trajectory or
estimate a Lyapunov exponent. The reveal distinguishes determinism from
long-range predictability and an attractor from a finite smooth path.
The status distinguishes the fixed twin parameter from the phase-selected
field parameter used by the background and clicked shadows. The picture uses
a fixed `x = [-25,25], z = [0,55]` window; legitimate clicked transients can
leave that window while integration continues. This is a disclosed display
limit, not evidence of numerical instability or a full-domain bounding proof.

The shared RK4 step also serves Lotka-Volterra and Double Pendulum. Independent
exponential growth/decay tests check fourth-order convergence; a harmonic
oscillator checks return and energy error. Double Pendulum retains its own
energy, reach, release, and refinement tests and its existing time step.

Sources: [Lorenz equations and their symmetry](https://www.scholarpedia.org/article/Equivariant_dynamical_systems),
[Sprott's Lorenz Lyapunov calculations](https://sprott.physics.wisc.edu/chaos/lorenzle.htm).
Implementation: `crates/core/src/numerics.rs` and the corresponding room files.

### Spherical Harmonics

The room exposes the 16 real spherical harmonics with `0 <= l <= 3` and
`-l <= m <= l`. The coefficient for `l = 3, |m| = 2` carried an extra factor
of one half, so those two modes had squared sphere integral `0.25` instead of
`1`. Removing it restores their normalization. Unsupported private modes now
return no value instead of fabricating an unrelated function.

The regressions check all 256 pairwise inner products and the independent
pointwise addition theorem. The reveal distinguishes the drawn angular
amplitude from a full hydrogen orbital and its probability density. No radial
wavefunction or full atomic simulation is claimed.

Sources: [NIST DLMF, spherical-harmonic definition](https://dlmf.nist.gov/14.30.E1),
[addition theorem](https://dlmf.nist.gov/14.30.E9), and
[hydrogen separation](https://dlmf.nist.gov/18.39.E24).
Implementation and regressions: `crates/core/src/rooms/harmonics.rs`.

## What remains open

Independent mathematical review before 1.0 remains unstaffed. The rest of the
catalog still needs the same equation-to-experience scrutiny, especially
continuous flows using Euler steps or clamps, special-function normalization,
physical-time versus normalized-phase displays, and geometry with hardcoded
aspect factors. New quests should use audited relationships and adversarial
contrast cases. Neither the test count nor line coverage closes this review.
