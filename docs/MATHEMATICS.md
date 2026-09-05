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
The alpha 18 corrections add Henon-Heiles, Van der Pol, Duffing, and
Gray-Scott. This is a focused source and
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

The shared RK4 step also serves Lotka-Volterra, Double Pendulum, and the
alpha 18 Henon-Heiles, Van der Pol, Duffing, and Gray-Scott corrections.
Independent exponential growth/decay tests
check fourth-order convergence; a harmonic
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

### Henon-Heiles (alpha 18 correction)

The dimensionless Hamiltonian is
`H = (px^2 + py^2 + x^2 + y^2)/2 + x^2*y - y^3/3`, giving
`x' = px`, `y' = py`, `px' = -x - 2*x*y`, and
`py' = -y - x^2 + y^2`. The hand selects `E = 0.05 + 0.15*x + s`,
where the seed offset `s = 0.005*(seed % 5)`. Without a hand, bounded gallery
phase selects `E = 0.08 + 0.08*t + s`; it does not advance physical time.
The combined hand range is `[0.05,0.22]` and the ambient range is `[0.08,0.18]`.

Every orbit starts at `(x,y) = (0,0.1)` with
`px = py = sqrt(E - 7/1500)`, so its Hamiltonian equals the selected energy.
The old initialization instead gave `H = E/2 + 7/1500`: the maximum displayed
`E = 0.22` actually started at about `H = 0.115`, below the escape barrier.
One trajectory owner now supplies the samples and termination used by rendering
and readout. Repeated tuning and equivalent accepted input histories preserve
the experiment. Independent Hamiltonian evaluations check the initialization
to `1e-14`; finite differences of H check the force's sign and components.

The three saddles `(0,1)` and `(+/-sqrt(3)/2,-1/2)` all have `H = 1/6` at
zero momentum. Tests check their stationarity, height and negative Hessian
determinant. The status says `closed` below this energy, `saddle` at it, and
`open` above it. Those labels describe the central well's barriers. An open
barrier neither proves a particular orbit escapes nor classifies it as chaotic.
The printed energy uses `~` to disclose rounding; classification uses the
underlying value. The musical phrase is a metaphor, not a sampled trajectory.

Shared RK4 uses step `0.01` for at most 100 time units. The first sample is at
time zero. A step leaving `|x| <= 3, |y| <= 3`, or producing any nonfinite state
component, ends the calculation before that step is retained. The status gives
the last retained sample's time: `end` means the 100-unit horizon, `box` means
the next numerical step left that spatial domain, and `invalid` means it became
nonfinite. For example, `E = 0.17` reaches the horizon with open barriers,
whereas `E = 0.22` retains samples through time 11.37 before the next step
leaves the box. This is a numerical stopping observation, not a certified escape
time or proof of permanent escape.

Compiled regressions sample 171 energies from 0.05 to 0.22 in increments of
0.001. Their maximum retained-sample `abs(H-E)` is about `2.180e-8`, below the
`1e-7` budget; all sampled sub-barrier orbits reach the full horizon. At
`E = 0.05, 0.12, 0.16, 0.20`, refining steps `0.02, 0.01, 0.005` over four time
units gives successive endpoint-difference ratios from 15.873 to 16.170, and
the final difference is below `1e-9`. Separate full-horizon endpoint checks at
`E = 0.05, 0.08, 0.12` require a difference below `5e-8` when halving the shipped
step. RK4 is not symplectic or exactly energy preserving. These finite fixtures
do not guarantee arbitrary trajectories, parameter values, or long-time
pointwise accuracy; small energy drift alone cannot establish those claims.
Endpoint differences use the maximum absolute component difference in
`(x,y,px,py)`.

The picture fits each retained orbit independently with one physical scale,
the surface's character aspect, and symmetric margins. A unit square's actual
painted extents check equal axis units on both Raster and Canvas. Relative
shape is preserved up to pixel rounding, but different energy settings do not
share a fixed zoom. Surface and input tests check bounded work and coordinates.

Sources: [Henon and Heiles, The applicability of the third integral of motion](https://doi.org/10.1086/109234),
and [Hernandez and Bertschinger, Time-symmetric integration in astrophysics](https://academic.oup.com/mnras/article/475/4/5570/4823139),
section 4.2, equation 56, for the Hamiltonian and examples of distinct regular
and chaotic orbits. The room's error budgets above come from its own numerical
regressions, not a validation of this solver by those papers.
Implementation and regressions: `crates/core/src/rooms/henon_heiles.rs`.

### Van der Pol (alpha 18 correction)

The dimensionless equation is `x' = v`, `v' = mu*(1-x^2)*v-x`.
For `E = (x^2+v^2)/2`, differentiation gives
`E' = mu*(1-x^2)*v^2`: damping supplies energy inside `|x|<1` and removes it
outside. For positive mu, nonzero trajectories approach an attracting orbit;
the exact origin remains an unstable equilibrium. Approaching the same orbit
does not imply synchronization of phase.

The hand selects `mu = 0.2+4.5*x+s` and initial state
`(4*(x-0.5),4*(0.5-y))`, with `s = 0.15*(seed % 6)`, so
`0.2 <= mu <= 5.45`. Horizontal input therefore changes both mu and initial x;
the verb and reveal now disclose that coupling. Without a hand, gallery phase
selects `mu = 0.5+3*t+s` and the start is `(0.1,0)`. The reference always starts
at `(2.5,0)` with the same mu. Both paths share one coordinate frame, with
equal axis units and a zoom fitted to their combined extents. The selected
endpoint remains visible even when that path is the exact equilibrium.

RK4 with step `0.005` replaces Euler. Both paths include time zero and evolve
for at most 100 time units. A nonfinite next state or component magnitude above
50 stops before retaining that step. The status identifies a finite `trace`,
the exact `equilibrium`, or a numerical `limit`/`invalid` stop; its time belongs
to the last retained sample. `A` is half the entire retained x range, including
transients, rather than a certified asymptotic cycle amplitude.

At 70 hand settings, using seeds 0 and 5, seven x values, and five y values,
the largest final energy-balance residual is `4.026e-5` against a `1e-4` budget.
This compares endpoint energy change with composite Simpson quadrature of the
power law. Halving the step gives maximum endpoint component difference
`2.846e-5` against `5e-5`, and half-range difference `5.124e-6` against `1e-4`.
The largest retained component is about 8.222, below the numerical cutoff.
These are sampled endpoint and range checks, not uniform trajectory-error bounds.

Other regressions check the analytic undamped limit, equilibrium, short-time
fourth-order refinement, and five independently computed DOP853 endpoints at
time 100. Those reference calculations agree within `7e-13` when their maximum
step is halved, and a separate degree-16 Taylor solver agrees within `1e-12`.
At mu 0.2, the final 25 units give sampled half-range 2.000413600 and
crossing period 6.298876662. These reject the previous Euler-inflated motion;
they remain finite numerical observations, not exact cycle constants.

Source: [Tedrake, Underactuated Robotics, Van der Pol example and limit cycles](https://underactuated.csail.mit.edu/simple_legs.html),
for the equation, equilibrium exception, and orbital stability distinction.
Implementation and regressions: `crates/core/src/rooms/van_der_pol.rs`.

### Duffing (alpha 18 correction)

The dimensionless model is `x' = v`,
`v' = x-x^3-0.3*v+g*cos(1.2*tau)`, starting at `(0.1,0)`.
Here `tau` is physical model time. Gallery phase selects drive strength, not
elapsed time: without a hand `g = 0.2+0.5*t+s`; with a hand
`g = 0.1+0.8*x+s`, where `s = 0.02*(seed % 7)`.
The combined admitted drive range is `[0.1,1.02]`.

The spring potential `V = -x^2/2+x^4/4` has minima at x = +/-1 and a barrier
of height 1/4 above them at x = 0. For `E = v^2/2+V`, the power balance is
`E' = -0.3*v^2+g*v*cos(1.2*tau)`. Force-gradient and power-law tests independently
check these relationships. Shared RK4 includes a clock coordinate, so each
stage samples the drive at its own physical time. It uses step `0.01` over
120 units, retaining time zero and stopping before a nonfinite next state.

The previous readout evolved only 18 units while the picture evolved 120, and
large motion was labeled `chaos?`. Rendering and readout now use the same
trajectory model. The status reports sampled maximum absolute x and v over
the full retained trace and its last sample's time. At `g = 0.79`, a regression
requires the late position excursion to exceed the first 18 units' maximum by
more than 0.5; an independent calculation gives about 2.004 versus 1.353.
The portrait fits its own extent with equal axis units, so different drive
settings do not share a fixed zoom.

Amplitude is not a chaos diagnostic. At the admitted `g = 0.9`, the room has
`max|x| > 1.5`, while longer calculations approach a response repeating at the
drive period `2*pi/1.2`. Tests sample periods 98, 99, and 100 at 512 and 1,024
steps per period, requiring successive state differences below `1e-8` and
refinement difference below `1e-7`. Independent DOP853 samples also close to
within `1e-10`. This finite counterexample rejects amplitude-based labeling;
it does not prove a global attractor or classify every selected drive.

Across 93 drives spaced by 0.01, all retained states reach the full horizon;
the largest sampled `|x|` and `|v|` are 2.156018 and 2.432530. At five drives,
the largest running energy-versus-work residual is `6.070e-7`, below `2e-6`.
The work uses Simpson quadrature of physical power with a half-step midpoint.
Independent four-unit DOP853 endpoint fixtures give shipped-step errors below
`1.8e-9`; steps `0.02,0.01,0.005` give successive difference ratios
16.141 through 16.422. Long-time pointwise accuracy is not inferred from these
short-time checks, especially where nearby trajectories separate.

Source: [Moon and Holmes, A magnetoelastic strange attractor](https://doi.org/10.1016/0022-460X(79)90520-0),
for the forced double-well model context. That paper does not validate this
room's coefficients, finite numerical paths, or error budgets.
Implementation and regressions: `crates/core/src/rooms/duffing.rs`.

### Gray-Scott (alpha 18 correction)

The reactions `U+2V -> 3V` and decay of V give the model
`U' = Du*lap(U)-U*V^2+F*(1-U)`,
`V' = Dv*lap(V)+U*V^2-(F+k)*V`. The room uses a periodic 48 by 28 lattice,
unit spacing, and the five-point Laplacian, with `Du = 0.16`, `Dv = 0.08`.
Background `(U,V)=(1,0)` surrounds a disk initialized to `(0.5,0.25)`;
variation selects disk radius `4+(seed % 3)`.

The latest hand tunes only feed `F = 0.01+0.08*x` and kill `k = 0.04+0.04*y`.
Repeated tuning and returning to the same rates preserve that initial patch.
Previously the number of hand points changed the seed, so a repeated choice
silently selected a different experiment. Default rates are now fixed at
`F = 0.04`, `k = 0.06`; bounded gallery phase selects observation time
`T = floor(120*t)` for both untouched and tuned fields. Each look replays the
same start at those rates and time.

RK4 with step 1 replaces clipped Euler updates. Tests check reaction
stoichiometry, uniform equilibria, conservation of periodic diffusion's total,
and exact discrete Fourier eigenvalues. An isolated diffusion mode checks
fourth-order temporal convergence against its analytic exponential decay.
Smooth-wave stencil refinement checks second-order spatial accuracy; it is not
nonlinear pattern convergence for the discontinuous seeded patch.

Four nonlinear fixtures at T = 120 require maximum field difference below
`2e-4` when comparing the shipped step with step 0.25, and below `1e-5` for
step 0.5 against 0.25. A separate 75-case probe, five feed values, five kill
values, and all three radii, found maximum terminal difference 0.0001644762.
All saved states in that probe were finite and nonnegative, with observed U
and V in `[0,1]`. These samples neither prove positivity for every admitted
setting nor validate a continuum solution or a long-time pattern class.

The readout measures V's maximum in the same evolved lattice, before display
averaging. One physical scale preserves its unit cell spacing on pixels and
text cells. Each display footprint averages the piecewise constant field by
covered area, then applies the glyph thresholds; averages below 0.08 are hidden.
Each destination is painted once. This removes the previous aspect stretching
and false brightness from overlapping source cells when downsampling. Uniform
fields remain uniform even at exact glyph thresholds, and actual painted seed
extents check physical shape on Raster and Canvas. A visually empty snapshot
can still contain small positive concentrations. Pattern names and a claimed
bifurcation diagnostic are removed.
The small finite lattice illustrates seeded reaction-diffusion; it does not
reproduce a published pattern survey. The musical phrase is a metaphor.

Source: [Pearson, Complex Patterns in a Simple System](https://arxiv.org/pdf/patt-sol/9304003),
equation 2 and its periodic-boundary numerical experiment. Its finite-amplitude
seeded patterns should not be conflated with a demonstrated Turing instability
in this room. Implementation and regressions: `crates/core/src/rooms/gray_scott.rs`.

The four corrected rooms share their fitted coordinate mapping. Its vertical
margin leaves room for App chrome while preserving one scale for numerical
coordinates. A Duffing turning point previously fell at raster row 19 inside
the compact title band. A composed-frame regression now compares every source
curve pixel with the final App frame for six Henon-Heiles, Van der Pol, and
Duffing settings at both supported sizes. `phase_plane.rs` also checks inverse
mapping of fractional display footprints used by Gray-Scott.

## Parametric Studio's planar fit (alpha 19)

In alpha 18, Studio fitted sampled x and y ranges independently. A circle and
the path `(4*cos(t),sin(t))` consequently became the same plotted shape. The
coordinate ranges in CLI and MCP remained correct, but the App, postcards,
and Gallery did not show those ranges.

The shared `PlanarProjection` now fits a sampled planar path with one physical
scale. For an available rectangle of `W` by `H` cells, sampled spans `dx, dy`,
and cell width-to-height ratio `c`, the nondegenerate fit is

```text
s = min((W - 1) / dx, (H - 1) / (c * dy))
screen_x = left + (W - 1)/2 + s * (x - center_x)
screen_y = top  + (H - 1)/2 - c * s * (y - center_y)
```

Raster pixels use `c = 1`; the terminal assumes `c = 0.5` for cells twice as
tall as they are wide.
Actual font proportions can differ. A constant coordinate stays centered and
does not constrain the other axis; a constant pair becomes a centered point.
Undefined samples break the path, including on either side of an isolated
finite point. The expression, window, sound, and capsule remain unchanged.

The implementation centers and normalizes before scaling, so finite opposite
extremes need not overflow and tiny paths are not inflated by an arbitrary
minimum span. The viewport is clipped to the surface before projection.
Fits whose internal spans or scales lose a varying coordinate to underflow
are refused, as are nonfinite scales, instead of returning a distorted path.
Core text and App rendering use this same primitive; room portraits retain
their separately tested margins and inverse display-footprint contract.

Regressions examine actual circle and 4:1 ellipse ink, translated paths,
constant lines and points, gaps, finite extremes, and bounded viewports. App
checks include portrait and landscape rasters, composed keyboard/controller
Studio panels, saved postcards, and Gallery thumbnails. Checking coordinate
ranges alone would miss the original failure.

CLI/MCP parity also preserves the complete canvas, including blank margins.
MCP `plot_expression` returns its actual `width` and `height` beside the plot.
The black-box comparison uses those dimensions: inferring width from the
rightmost ink would incorrectly shrink a centered path before comparing it.

This preserves proportions within each fitted path. It does not establish a
common absolute magnification across separate creations, certify a continuous
curve from finite samples, or make the audio map invertible. Ordinary
`y=f(x)` graph autoscaling remains a separate presentation contract.

## Study depth and numerical readouts

The authored Lissajous treatment in `crates/core/src/study/lissajous.rs` is
shared by the App, CLI, and MCP. Its equations and reference identities are
single-sourced across English and the Japanese draft. The treatment separates
ideal continuous oscillators, their binary64 implementation, the finite drawn
trace, and the audio mapping. Worked examples distinguish a repeated position
from a full-state period, and explain rational closure, an irrational torus
orbit, occupation measure, and approximate recurrence. [Study](STUDY.md)
describes direct access; mathematical depth is not a reward for prior visits.

The source regressions check the stated numerical examples and translation
identity, including equations and reference targets. They do not constitute
formal verification or independent scholarly review of the entire treatment.
Other rooms retain their existing explanation and notes until a comparable
Mathematics treatment is authored.

Six rooms now expose finite, room-scoped numerical channels through
`NumericReadout`: Times Tables, Lissajous, Gray-Scott, Standing Wave, Bayes
Update, and Smith Chart. Grading uses channel identity rather than an English
label's position. Their existing decimal quantization, sampled challenge
ranges, seeds, tolerances, and scores are retained, including Standing Wave's
half-percent rounding. A missing or duplicate channel refuses instead of
silently selecting another number. A challenge also checks its room identity.
The compatibility parser remains for other rooms; [Rosetta](ROSETTA.md) owns
that migration boundary.

## What remains open

Independent mathematical review before 1.0 remains unstaffed. The rest of the
catalog still needs the same equation-to-experience scrutiny, especially
continuous flows using Euler steps or clamps, special-function normalization,
physical-time versus normalized-phase displays, and geometry with hardcoded
aspect factors. New quests should use audited relationships and adversarial
contrast cases. Neither the test count nor line coverage closes this review.

The concrete alpha 17 queue, Henon-Heiles, Gray-Scott, Van der Pol, and
Duffing, is addressed by the alpha 18 corrections above. These reviewed
examples still need independent mathematical review before 1.0, and the wider
catalog remains open. Better time integration alone cannot validate a room's
control design, visible geometry, sound mapping, or learning claim.

Studio's alpha 17 synchronization gap is corrected in alpha 18: fresh App
formulas start with the shared `a = 1`, and explicit quarter-step or reset
actions change one parameter used by picture, live melody, postcard, capsule,
and MIDI. Imported non-quarter values and windows remain exact until changed.
The gallery clock cannot retune the expression. Graph and paired-program App
regressions compare the live SoundSpec and actual exported MIDI with the saved
creation across gallery phases, and check edits, no-ops, preview ownership,
repair, and leave/return. Recipe morphs require identical old and new numerical
settings, so they cannot display an old formula evaluated in a different domain.

This is parameter agreement, not an invertible correspondence between music
and geometry. Parametric audio maps `y(t)`, normalizes a finite sample of its
range, and retains the MIDI limits in `STUDIO.md`. Auto uses a presentation
clock and does not establish audio phrase alignment. Continuous parameter
sweeping remains separate audio-transport work.

Parametric Studio's independent-axis distortion is addressed by the alpha 19
correction above. General graph framing, sampling density, sound-to-geometry
interpretation, and visible absolute coordinate scales remain distinct review
questions; a corrected circle does not close the Studio review.
