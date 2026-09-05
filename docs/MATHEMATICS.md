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

Scope: the Kepler Areas, Lotka-Volterra, Lorenz, Spherical Harmonics,
Lissajous, Standing Wave, Simple Pendulum, and Braess implementations, plus the
shared numerical step used by Double Pendulum and the claims in Wet Oracle.
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

### Lissajous

The room uses unit-amplitude oscillators
`x(theta) = cos(fx*theta + phase_x)` and
`y(theta) = sin(fy*theta + phase_y)`. With no hand, `fx = 3` and
`fy = 2 + 3*t` for a bounded gallery phase `0 <= t <= 1`. A hand chooses
integer frequencies from 1 through 8 in eight equal-width intervals; the
gallery then changes relative phase
without changing that tuning. The oscillator parameter and gallery phase are
distinct. All displayed hand ratios are rational.

The previous seeded x perturbation added another sinusoid of the same frequency
and clamped the sum, flattening peaks for seeds with amplitude above one.
Seed variation now changes phase, and one spatial scale preserves circles on
both pixels and text cells. A common margin keeps the full curve clear of the
App title and controls. A composed-frame regression checks every source curve
pixel at both supported sizes for a circle, a sweep, and the public 2:5 tuning;
the same test rejects the previous clipped framing.
Readout, rendered path, one-shot sound, and live
sound targets use the same accepted frequency pair. The sound is the ordered
pair `110*fx` and `110*fy` Hz; it does not reproduce the displayed phases.
The previous one-shot sound ignored the hand and rounded the continuous sweep:
at `t = 0.5`, a visible `3:3.5` ratio became an audible `3:4` ratio.

The live mixer now smooths both absolute frequencies, retaining each oscillator's
phase. Smoothing root and ratio independently used to create an unintended pitch
excursion when their changes should cancel: a fixed 880 Hz second voice could
pass 2 kHz. An independent phase-increment regression checks both directions,
frequency containment, and settling in `crates/audio/src/lib.rs`.

Independent checks use the sinusoidal recurrence, unit mean square, geometric
circle radius, and actual renderer axis crossings. They cover all 64 hand
tunings at four seeds and sampled continuous phases. A separate counterexample
uses `fx = 3, fy = 3.5`: the position returns at `theta = 2*pi`, but y velocity
reverses, and the full motion's least positive period is `4*pi`. Endpoint
coincidence alone does not prove periodicity.

For positive frequencies a full period requires both oscillator phases to
advance by integer multiples of `2*pi`; such a period exists exactly when
their ratio is rational. Floating-point frequency values approximate an ideal
parameter model. Neither a finite trace nor an apparent gap can classify an
arbitrary expression as rational or irrational. Pleasantness of a musical
interval is also not a mathematical consequence of periodicity.

Source: [MIT 18.353, quasiperiodic functions and Lissajous figures, section 3](https://math.mit.edu/classes/18.353J/PSetAnswers/AnswerPSet_2024_07.pdf).
Implementation and regressions: `crates/core/src/rooms/lissajous.rs`.
[Returning home](experiments/returning-home.md) supplies three portable Studio
contrasts with exact formulas, including an ideal irrational ratio.

### Standing Wave

For unit length and dimensionless speed `c = 2`, the room uses
`u(x,t) = A*sin(n*pi*x)*cos(2*pi*n*t + phase)`, with `1 <= n <= 8`.
It satisfies fixed endpoints and `u_tt = c^2*u_xx`; every mode shares the same
time coordinate and has period `1/n`. The previous renderer changed wavelength
but kept every mode's temporal frequency equal, silently changing wave speed.
Repeated selection also changed phase through the input-event count. Both
effects are removed. The reveal explicitly distinguishes the model's time from
wall-clock playback.

Central finite differences check the wave equation across all eight modes,
three phases, and two interior points, with residual budget
`1e-5*(1 + abs(u_tt))`. Separate tests check full and half periods and unchanged
rendering after repeated identical selection. These are model checks, not a
measurement of a physical instrument's tone or an audio-visual phase lock.
Source: [MIT 8.03, wave equation and normal-mode derivation](https://ocw.mit.edu/courses/8-03sc-physics-iii-vibrations-and-waves-fall-2016/e4278fad9bdd90bc05f163fd7f670f65_MIT8_03SCF16_Lec9.pdf).
Implementation and regressions: `crates/core/src/rooms/standing_wave.rs`.

### Simple Pendulum

This is an analytic phase portrait of `E = omega^2/2 - cos(theta)`, with
`-1 <= E <= 3`. Gallery phase selects energy; it is not elapsed pendulum time.
The hand covers the full interval independently of seed. `E = -1` is stable
rest, `-1 < E < 1` gives librations, `E = 1` is the separatrix, and `E > 1`
gives rotations. The status classifies the actual energy and marks its rounded
printed value with `~`.

Contours use `omega = +/- sqrt(2*(E + cos(theta)))`. Libration angles run from
`-acos(-E)` to `acos(-E)`, including both exact zero-speed endpoints. The two
separatrix branches match `+/- 2*cos(theta/2)` on `[-pi,pi]` and are drawn
behind the selected orbit. The fixed speed window `[-3,3]` contains the largest
admitted speed `sqrt(8)`. The left and right angle boundaries are the same point
on the phase cylinder.

The old portrait could leave librations disconnected, clip every rotation,
omit the lower reference branch, classify an actual rotation as a separatrix,
and draw a different energy from its readout. Its smallest available swing was
already over 100 degrees. The arbitrary distorted bob is removed. Tests check
Hamiltonian residuals, known turning angles, both separatrix branches, exact
regime boundaries, reachable small swings, actual rendered connectivity, and
bounded input and surface behavior. Drawing sampled analytic contours does not
assert a physical time integration or estimate the nonlinear period.

Source: [MIT Underactuated Robotics, pendulum orbit calculations](https://underactuated.mit.edu/pend.html).
Implementation and regressions: `crates/core/src/rooms/simple_pendulum.rs`.

### Braess Trap

The directed network has routes `S-A-T`, `S-B-T`, and an optional `S-A-B-T`.
Edges `S-A` and `B-T` cost their flow; `A-T` and `S-B` cost 1; the shortcut
`A-B` costs 0. Demand is a divisible flow, not an integer number of drivers.
Without the bridge, equal outer-route flows give travel time `1 + d/2`.
With it, all demand follows the shortcut only when `d <= 1`. For `1 < d <= 2`,
outer-route flows are `d-1` each and shortcut flow is `2-d`; every used route
then costs 2. These satisfy Wardrop's condition: no infinitesimal driver can
improve by changing route.

The previous formula `2*d` incorrectly extended the all-shortcut regime to
higher demand. At `d = 1.6` it reported 3.2, even though an unused outer route
would cost only 2.6 under that flow. The corrected equilibrium costs 2.
The bridge helps below `d = 2/3`, harms between `2/3` and 2, and ties at the
two boundary demands. This is an equilibrium comparison, not an optimization
of total travel time or a model of live traffic.

Tests independently reconstruct edge loads and all route costs, checking
nonnegative flows, demand conservation, equal costs on used routes, and no
cheaper unused route over the admitted interval. Drawing and status resolve one
seeded scenario; the display compares both bridge alternatives. `~TIE` denotes
a difference below 0.005 at the displayed hundredth precision, not exact equality.
The helper admits `0.5 <= d <= 2`; current hand controls cover `[0.6,1.6]`,
and seeded ambient demand stays within `[0.72,1.2]`.

Source: [Braess, Nagurney and Wakolbinger, On a Paradox of Traffic Planning](https://doi.org/10.1287/trsc.1050.0127).
Implementation and regressions: `crates/core/src/rooms/braess.rs`.

### Wet Oracle: what the picture actually computes

This room evolves and draws a Physarum-inspired scent field and reports its
mass. It does not extract a route, measure route cost, compare a player's route,
or prove optimality. Its copy no longer promises a shortest-path race or a win.
The Tokyo network experiment motivates a biological-network metaphor; it
studied tradeoffs among cost, transport efficiency, and fault tolerance.
It does not validate this field simulation as an optimal-route solver.

Source: [Tero et al., Rules for Biologically Inspired Adaptive Network Design](https://doi.org/10.1126/science.1177894).
The correction is a claims review in `wet_oracle.rs` and catalog copy, not a
validation of biological fidelity. The proposed computational route experience
has its own explicit contract in [Route Lab](ROUTE_LAB.md).

## What remains open

Independent mathematical review before 1.0 remains unstaffed. The rest of the
catalog still needs the same equation-to-experience scrutiny, especially
continuous flows using Euler steps or clamps, special-function normalization,
physical-time versus normalized-phase displays, and geometry with hardcoded
aspect factors. New quests should use audited relationships and adversarial
contrast cases. Neither the test count nor line coverage closes this review.

The next concrete queue follows a read-only source and equation review. These
issues remain in alpha 17 and still need corrections and compiled regressions:

| Room | Next correction and evidence to require |
| --- | --- |
| [Henon-Heiles](../crates/core/src/rooms/henon_heiles.rs) | Its initial Hamiltonian is `E/2 + 7/1500`, rather than the displayed `E`. Derive initial momenta from the available energy, share the trajectory with the readout, and distinguish an open escape channel from observed escape or chaos. |
| [Gray-Scott](../crates/core/src/rooms/gray_scott.rs) | Repeated identical tuning changes an unrelated seed. Make the experiment stable under repeated selection, and replace unsupported pattern and bifurcation labels with observations or justified diagnostics. |
| [Van der Pol](../crates/core/src/rooms/van_der_pol.rs) | Bound the Euler step's amplitude error against refinement, recognize the exact equilibrium, and make the readout's observation window explicit. |
| [Duffing](../crates/core/src/rooms/duffing.rs) | Motion amplitude does not establish chaos. Review the suggestion against periodic counterexamples and align the plotted and reported horizons. |

Studio has a separate synchronization gap: a fresh unsaved App expression can
use a gallery-phase-dependent `a` visually while its live melody uses `a = 1`.
Saved creations and their edits preserve their chosen window and `a` across
picture, share, and melody. Closing the fresh-animation gap needs a shared
transport-aware parameter update, not repeated sample-buffer restarts.
