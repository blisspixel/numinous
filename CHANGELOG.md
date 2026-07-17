# Changelog

All notable changes to Numinous. The format follows Keep a Changelog, and the
project uses version-gated milestones (see ROADMAP.md), not dates.

## [Unreleased]

### Added
- Double Pendulum now makes its physical gesture state audible through the
  existing continuous parameter voice. The same reduced state drives pixels,
  the twin-divergence status, and sound: the first-arm drop selects one of five
  minor-pentatonic roots, second-arm bend opens a symmetric interval from 1:1
  through 3:2, and real release speed raises the quiet gain from 0.03 toward a
  capped 0.05. A pin stays low, a flick becomes more present, and cancel drops
  gently. Core tests cover compact rendering, bare release, cancellation,
  phase-wrapped fling, invalid tails, ordered roots, bend, and gain. App tests
  cover voice ownership and time-stamped focus-loss cancellation, CLI tests
  cover compact and full deterministic replay plus wrapped phase parsing, and
  MCP tests cover pin, fling, and wrapped replay through the protocol. Human
  musical judgment remains open.
- Galton Board now makes the selected fixed coin audible through the same
  continuous parameter-voice seam as its visible experiment. The five coin
  regions climb through C major-pentatonic roots, while bias strength uses the
  exact larger-to-smaller Bernoulli odds ratio: 7:3, 3:2, 1:1, 3:2, or 7:3.
  The App glides that quiet voice over the stable room bed; CLI `sonify` and MCP
  `listen_room` render the same accepted poke or gesture as a deterministic
  two-note snapshot. Focused tests cover all five coins, no parameter voice
  before a drop, ownership routing, exact ratios, ordered roots, and three-face
  replay parity.
  Planned peg ticks, pile texture, spatialization, and human listening evidence
  remain explicitly unclaimed.
- A release-profile App performance harness now measures the five 0.3
  flagships across geometry, chaos, emergence, chance, and creation. It reports
  ambient-raster and accepted-input-to-room-raster p50, p95, and maximum time
  at a declared viewport and sample count. The explicit reference-machine gate
  fails above a 33 ms p95 budget, rejects hostile arguments, and checks that
  every measured path produces visible output. Wrapper scripts provide the
  same locked command on Windows, macOS, and Linux. The report explicitly
  excludes native event translation and history storage, window presentation,
  display scan-out, audio submission and callbacks, and human perception rather
  than mislabeling a headless duration as end-to-end latency.
- The Conjecture Mill joins the Number & Pattern wing as room 351. Its finite
  typed grammar enumerates primitive rational quadratic formulas against six
  observed integer sequences. Every wrong candidate exposes an exact
  counterexample, while `PROVED` requires cross-multiplied coefficient equality
  rather than a sample threshold. Phase advances the complete search; bounded
  drag paths select the laboratory and replayable search permutation without
  changing data or truth. The shared blackboard renders the observed sequence,
  current chalk, best survivor, rejected ledger, trial counts, and proof stamp
  at raster and ASCII sizes. Complete-permutation, proof-separation, witness,
  hostile-input, variation, registry, and catalog visual-oracle tests cover it.

### Changed
- App release QA now derives eight room receipts from all 351 registry entries
  and validates an exact 2,909-screen inventory instead of a fixed 31-room
  table. Scenarios follow each room's declared interaction verb. The generator
  checks mathematical consequences independently from the App's latest-gesture
  trail and reticle, while a catalog-wide diagnostic aggregates every failure.
  A one-pixel regression proves that raw room responses must clear the separate
  perceptibility floor. Held actions now return to their ambient render and
  status on release or cancel without collapsing compact CLI or MCP multi-point
  input, and Laplace's Clockwork gains a visible detune gauge.
  Audioactive Decay renders its spoken digits as a scalable signal, Busy Beaver
  renders written tape cells as visible bands, and Phantom Jam marks the active
  brake immediately at both supported viewport sizes.
- Consequence-depth pass (cycle 120 physics and geometry): Berry Phase now
  keeps its loop on the Bloch sphere and reports phase magnitude without an
  invented sign; Bragg Diffraction shares seeded plane spacing between its
  detector and order-detune readout; Capillary Meniscus crosses neutral contact
  continuously and distinguishes rise from depression; Sphere Geodesics now
  rotates the actual great-circle plane, bounds seeded tilt, and distinguishes
  local geodesics from globally minimizing arcs; Polarization samples the full
  unit interval and grades Malus-law transmission. Duplicate hand history no
  longer changes any of the five rendered experiments.
- Consequence-depth pass (cycles 118-119): 22 rooms with domain-true action
  status. Menagerie/Clifford span, Henon-Heiles span and escape energy regime,
  Brusselator Hopf margin, coupled-oscillator mode split, elliptical and Sinai
  billiard launch geometry, Feigenbaum period 2^g, Weierstrass ab roughness,
  baker Lyapunov, horseshoe strips, Hopf link/fibration/base, unknot cr=0,
  Seifert half-twists, Cantor and Menger Hausdorff dims, percolation vs pc,
  rule-30 class labels, Kaplan-Yorke dimension, Manneville laminar scale,
  Buddhabrot escape probe. Footer and digit contracts hold.
- Consequence-depth pass (cycles 116-117 exceptional depth): 49 rooms deepened
  with domain-true action status. Maps and attractors (Hopalong, Tinkerbell,
  Svensson, Pickover, Sprott, Gingerbread, tent, coupled tent, van der Pol,
  standard map, lambda, filled Julia) report orbit spans, Lyapunov, sync
  residual, limit-cycle amplitude, KAM flip rate, escape, and fill fraction.
  Classical and surfaces (tractrix, helicoid, involute, Fermat, evolute,
  Boy, cross-cap, Roman, Whitney, pseudosphere, Klein, trefoil, figure-eight,
  Viviani) report areas, pitch, cusp scale, triple-point phase, curvature K,
  crossings, and volume. Number, epidemic, and spin rooms report density,
  growth, attack size, Onsager M, Foucault period, heat peak, Rabi Pmax, and
  class-IV bias. Burn-in plus finite guards on spans. Footer and digit
  contracts hold.
- Consequence-depth pass (cycles 114-115 bulk depth): 51 rooms across classical
  curves, knots and surfaces, maps and attractors, fractals, special functions,
  and escape-time portraits. Action status reports domain measures (areas,
  dimensions, crossing numbers, Lyapunov-ready spans, escape iters, Farey
  counts, Hermite energies, Koch perimeter growth) instead of knob echoes.
  Lightweight span samples use burn-in plus finite/magnitude guards. Compact
  footer and digit contracts hold.
- Consequence-depth pass (cycle 113 classical curves and flows batch):
  Superellipse shape class, Agnesi peak and area, Reuleaux constant-width area,
  log spiral pitch and growth per turn, hypotrochoid cusp class, Poisson E[N]
  and realized n, diffraction first zero, dual-cobweb logistic band, folium
  loop area, tautochrone bead gap, catenoid neck radius, conchoid gap, piriform
  height, kappa cot arms, three-scroll and Rabinovich-Fabrikant spans.
  Compact footer and digit contracts hold.
- Consequence-depth pass (cycle 112 dynamics/geometry/fractal batch): Zipf P1
  share, doubling-map Lyapunov and bit density, prism spectral spread, Coriolis
  frame rotation, Manneville laminar scale and burst %, Multibrot/Nova/Phoenix
  escape probes, cochleoid r(pi/2), epitrochoid petals, devil b-a shape,
  hyperbolic verts, Poincare step angle, witch nephroid light angle, Collatz
  tree node count, Halvorsen span. Compact footer and digit contracts hold.
- Consequence-depth pass (cycle 111 waves/number/fractal batch): triangle and
  sawtooth harmonic energy, standing-wave nodes and spacing, Young fringe scale,
  Zeckendorf ones-count, Gaussian prime count, quadratic residue half, Vicsek
  dim and cells, Delaunay Euler estimates, Ricker orbit band, Thomas span,
  Mexican-hat zero width, Gumowski-Mira span, Fresnel distance to asymptote.
  Compact footer and digit contracts hold.
- Consequence-depth pass (cycle 110 attractors/curves batch): Aizawa span,
  Astroid P=6a and area, Bedhead span, Bifolium leaf area, Blancmange roughness,
  Bogdanov soft radius band, Cardioid P=8a and area, Catenary sag and span,
  Chua scroll flips, Henon |det| and span, Duffing amplitude band, Deltoid
  perimeter and area, Cassini loop shape at b/a, Lemniscate area and half-width.
  Compact footer and digit contracts hold.
- Consequence-depth pass (cycle 109 waves/dynamics/shape batch): Bessel nodal
  ring count, Airy dark-ring count, Arnold winding drift and lock band,
  coupled-logistic mean |dx| and SYNC, damped half-life and end amp,
  Cauchy FWHM and peak, AM depth and carrier share, bifurcation mid/span band,
  Beatty |r-phi| and unique hits, Chebyshev max Runge error, Clifford span,
  clothoid dkappa/ds, cycloid path in r units, Archimedean gap per turn.
  Compact footer and digit contracts hold.
- Consequence-depth pass (cycle 108 classical/prob batch): Basel err vs pi^2/6,
  Birthday pair count and half-threshold, Blackbody Wien peak, CLT SE shrink,
  Coupon H_n and last-wait, Brownian rms scale, Brewster d-to-iB and p-pol zero,
  Wallis pi error, Benford P1/P9 ratio, beat period, pendulum libration vs
  rotation, escape ve/vc ratio, Kepler ra/rp, Stirling relative error. Builds
  on the cycle 107 catalog-tail pass. Compact footer and digit contracts hold.
- Consequence-depth pass on the catalog tail: action status now reports domain
  measures after a poke (twin last pair, perfect n or digit scale, AGM iters and
  |a-g|, Bayes prior/post delta, Huffman avg vs H gap, Napoleon side spread,
  erf with Phi, Erdos-Renyi edge count, Markov visit peak, eta series drift,
  Pell fundamental solution, Egyptian unit range, mutual-info residual,
  Gamma Stirling error, Shannon gap to fair). Compact footer and digit
  contracts still hold.
- Current-state docs name the live catalog size (350 rooms): root README,
  `docs/README.md`, `docs/ROOMS.md`, `docs/MUSIC.md`, `docs/ROADMAP.md` progress
  and Where we stand, and `VERIFY.md`. Historical 0.1 foundation notes that
  recorded the original 31-room baseline stay as history.

### Added
- Four closing inventions (catalog 350): AGM Mean, Twin Primes, Perfect Numbers,
  Napoleon Theorem.
- Six analysis inventions (catalog 346): Gamma Function, Error Function,
  Fresnel Integrals, Lambert W, Sinc Interpolation, Dirichlet Eta.
- Six topology inventions (catalog 340): Klein Bottle, Cross-Cap, Boy Surface,
  Solid Torus, Hopf Link, Unknot.
- Six probability and information inventions (catalog 334): Shannon Entropy,
  Bayes Update, Erdos-Renyi Graph, Markov Chain, Huffman Tree, Mutual Info.
- Six number-theory inventions (catalog 328): Lucky Numbers, Gaussian Primes,
  Quadratic Residues, Zeckendorf, Egyptian Fractions, Pell Path.
- Six dynamics inventions (catalog 322): Simple Pendulum, Blackbody Spectrum,
  Kepler Areas, Escape Velocity, Coupled Oscillators, Prism Dispersion.
- Six classical curve inventions (catalog 316): Superellipse, Cochleoid,
  Serpentine, Bifolium, Butterfly Curve, Piriform.
- Six analysis inventions (catalog 310): Harmonic Series, Basel Problem,
  Stirling Approx, Benford Law, Central Limit, Wallis Product.
- Six stochastic inventions (catalog 304): Poisson Process, Brownian Motion,
  Birthday Paradox, Coupon Collector, Zipf Law, Gambler's Ruin.
- Six arithmetic inventions (catalog 298): Mertens Function, Liouville Function,
  Euler Totient, Partition Function, Paperfold Sequence, Sylvester Sequence.
- Six sequence inventions (catalog 292): Kolakoski Sequence, Beatty Sequence,
  Wythoff Array, Minkowski Question Mark, Ruler Function, Moser-de Bruijn.
- Six topology inventions (catalog 286): Viviani Curve, Torus Knot, Whitney
  Umbrella, Roman Surface, Spherical Harmonic, Lissajous 3D.
- Six topology inventions (catalog 280): Seifert Film, Trefoil Knot, Hopf
  Fibration, Filled Julia, Figure-Eight Knot, Borromean Rings.
- Six special-function inventions (catalog 274): Bessel J0, Hermite Wave,
  Legendre P_n, Heat Kernel, Cauchy Lorentz, Mexican Hat wavelet.
- Six classical inventions (catalog 268): Kampyle of Eudoxus, Hippopede,
  Cartesian Oval, Berry Phase, Runge Phenomenon, Chebyshev Nodes.
- Six classical inventions (catalog 262): Maclaurin Trisectrix, Watt Curve,
  Devil Curve, Capillary Meniscus, Rabi Flopping, Sphere Geodesics.
- Six classical inventions (catalog 256): Tautochrone, Catenoid, Helicoid,
  Pseudosphere, Airy Disk, Bragg Diffraction.
- Six classical inventions (catalog 250): Reuleaux Triangle, Logarithmic Spiral,
  Archimedean Spiral, Cassini Ovals, Foucault Pendulum, Coriolis Path.
- Six optics inventions (catalog 244): Doppler, Interference, Diffraction,
  Snell's Law, Polarization, Brewster Angle.
- Six wave inventions (catalog 238): Gibbs Square, Sawtooth, Triangle Wave, AM
  Modulation, FM Modulation, Standing Wave.
- Six classical inventions (catalog 232): Involute, Ellipse Evolute, Pedal
  Curve, Roulette Gallery, Damped Sine, Beats.
- Six classical curve inventions (catalog 226): Semicubical, Kappa, Circular
  Caustic, Trochoid, Hypotrochoid, Epitrochoid.
- Six classical curve inventions (catalog 220): Gerono Eight, Cissoid, Strophoid,
  Conchoid, Limacon, Folium of Descartes.
- Six classical curve inventions (catalog 214): Dual Cobweb, Beverton-Holt,
  Witch of Agnesi, Tractrix, Catenary, Clothoid.
- Six classical inventions (catalog 208): Hyperbolic Tiling, Mackey-Glass,
  Fermat Spiral, Euclid Algorithm, Oregonator, Hofstadter Q.
- Six classical inventions (catalog 202): Cardioid, Deltoid, Coupled Logistic,
  Menger Sponge, Spiral of Theodorus, Rule 110.
- Six curve and system inventions (catalog 196): Sprott Attractor, Delaunay Mesh,
  Astroid, SIR Epidemic, Nephroid, Lemniscate.
- Six physics and geometry inventions (catalog 190): Percolation, Ising Lattice,
  Lotka-Volterra, Poincare Disc, Cycloid, Brusselator.
- Six chaos and geometry inventions (catalog 184): Chua Circuit, Arnold Cat
  Map, Blancmange Curve, Rose Curve, Kuramoto Sync, H-Tree.
- Six fractal and cascade inventions (catalog 178): Nova, Magnet Fractal,
  Lambda Map, Feigenbaum Ladder, Menger Carpet, Vicsek Fractal.
- Six fractal escape inventions (catalog 172): Burning Ship, Tricorn, Multibrot,
  Phoenix, Lyapunov Weather, Collatz Tree.
- Six map and gasket inventions (catalog 166): Bogdanov, Kaplan-Yorke, Ricker,
  Farey Sequence, Gosper Curve, Sierpinski Triangle.
- Six fractal and intermittency inventions (catalog 160): Gauss Map, Manneville,
  Coupled Tents, Koch Snowflake, Cesaro Fractal, Minkowski Sausage.
- Six chaos-geometry inventions (catalog 154): Smale Horseshoe, Logistic Orbit,
  Sinai Billiard, Henon-Heiles, Quadratic Map, Angle Doubling.
- Six classical map inventions (catalog 148): Lozi, Baker's Map, Tent Map,
  Arnold Circle Map, Chirikov Standard Map, Elliptical Billiard.
- Six continuous chaos inventions (catalog 142): Pickover, Aizawa, Thomas,
  Halvorsen, Rabinovich-Fabrikant, Three-Scroll.
- Six classical inventions (catalog 136): Clifford, Peter de Jong, Svensson,
  Bedhead, Hopalong, Gumowski-Mira attractors.
- Six classical inventions (catalog 130): Bifurcation Weather, Stern-Brocot Tree,
  Josephus Circle, Calkin-Wilf Tree, Gibbs Overshoot, Sierpinski Arrowhead.
- Six classical inventions (catalog 124): Ikeda Map, Duffing Well, Levy C Curve,
  Tinkerbell Map, Gingerbreadman Map, Menger Face.
- Six classical inventions (catalog 118): Thue-Morse, Rossler Scroll, Cantor /
  Devil's Staircase, Weierstrass, Peano Curve, Van der Pol Cycle.
- Six classical inventions (catalog 112): Dragon Curve, Fibonacci Word, Cubic
  Newton, Henon Map, Rule 30, Mandelbulb Slice.
- Six classical inventions (catalog 106): Penrose floor, Continued Fractions,
  Logistic Cobweb, Sierpinski Carpet, Pythagoras Tree, Ulam Spiral.
- Catalog 100: The Mirror of Forms (category-lite composition).
- Frontier and topology batch (catalog 99): Sphere Eversion, Causal Doors,
  Soft Proof (homotopy), Learning Clock, Duality.
- Soap Film, Landauer's Price, and Prime Gap Weather (catalog 94).
- Universal wonder Tier S/A wave (catalog 91): Curse of Dimension,
  Concentration Bell, Uncertainty Dial, Gradient Valley, Attention as Soft
  Light, Braess Trap, Nontransitive Dice, Parrondo's Trap, Hilbert's Hotel.
- Gray-Scott Chemical Garden and Eratosthenes Sieve (catalog 82).
- Three classical fractal rooms (catalog 80): Newton's Basins (`newton`), Koch
  Infinite Coast (`koch`), Hilbert Space-Filling Path (`hilbert`).
- Function Painter (`function-painter`, catalog 77): domain coloring of a rack
  of complex maps (z^2, z^2+c, 1/z, sin z, e^z, z^3-1); phase as symbol,
  magnitude as density; DRAG picks map and tunes c.
- Awe Engine Tier A/B completion (catalog 76): Apollonian gasket, Circle
  Inversion, DLA Frost, Kaprekar 6174, Steiner chain, Hopf linked rings,
  Wireworld, Buddhabrot, Spherical Harmonics.

- Chord Game plus Awe Engine Tier S (catalog 67): elliptic chord-and-tangent
  group law; Recaman Jumper; Truchet Weave; Pursuit Chase; Pascal mod n;
  Three-Gap Spinner; Morley triangle; Clifford Menagerie.
- Four more Next Wave rooms (catalog 59): Unlit Room, The Lens, Fourteen
  Beacons, The Loneliness Equation.
- Four more Next Wave rooms (catalog 55): Tilt the Cone, The Stretch, Laplace's
  Clockwork, The Message That Heals.
- Six Next Wave rooms (catalog 51): Busy Beaver, The 720 Degree Room,
  Upside-Down Ruler, Murmuration, Whispering Table, Wet Oracle.
- Phantom Jam (`phantom-jam`), The Fastest Fall (`fastest-fall`), and
  Audioactive Decay (`audioactive`): Sugiyama upstream traffic jam; cycloid
  brachistochrone race; look-and-say generations toward Conway's constant.
- Kepler's Loom room (`kepler-loom`, Motion & Dynamics): inverse-square orbit
  integration; ambient elliptical moon; DRAG: FLING A MOON reports eccentricity
  and peri/apo.
- The Magnet room (`the-magnet`, Emergence): Metropolis Ising lattice; ambient
  and DRAG heat through Onsager Tc; magnetization grades ORDER/CRIT/NOISE.
- The First Rain room (`first-rain`, Emergence): site percolation on a square
  lattice; `t` and DRAG: MAKE IT RAIN dial occupancy p; status reports SPAN/DRY
  against p_c about 0.5927.
- Slingshot room (`slingshot`, Motion & Dynamics): pull-and-release probe
  under multi-sun Newtonian gravity; assists counted on speed-gain flybys;
  HOLD plants a sun; misses become comets. Completes the Next Wave first eight.
- The Starbow room (`starbow`, Shape & Space): McKinley relativistic aberration
  maps each rest-frame star into a forward cone as beta climbs; `t` burns
  ambient speed; HOLD: BURN under the hand; status reports v/c, gamma, and
  forward fraction.
- The Zeta Walk room (`zeta-walk`, Number & Pattern): alternating eta partial
  sums on Re(s)=1/2 draw a spiral that tightens near tabulated Riemann zeros;
  `t` climbs imag height; DRAG: CLIMB THE LINE; sound resolves at cadences.
- Ford Circles room (`ford-circles`, Number & Pattern): reduced fractions p/q
  own radius-1/(2q^2) circles that never overlap; `t` deepens the denominator
  ceiling; CLICK: BIRTH THE MEDIANT inserts (a+c)/(b+d) into the Farey gap under
  the hand. Focused tests cover non-overlap, neighbor kisses, and golden gap
  readout.
- The Coffee Cup room (`coffee-cup`, Shape & Space): one-bounce circular
  reflections condense into a cardioid caustic; `t` walks the rim sun; DRAG:
  SWING THE SUN aims the cusp. Reveal links the same cardioid to Times Tables
  and Mandelbrot.
- The Ripple Tank room (`ripple`, Waves & Sound): monochromatic point sources
  superpose into bright fans and dead-calm lanes; ambient double source opens
  the double slit; CLICK: DROP A PEBBLE replaces sources under the hand.
- Chladni Figures room (`chladni`, Waves & Sound): free-plate formula draws
  nodal sand for mode pairs (n, m); `t` walks a mode gallery; DRAG: TUNE THE
  PLATE sets integer modes under the hand; drive tone chord is the same two
  numbers. Reveal cites Gordon-Webb-Wolpert (1992).
- The Sandpile room (`sandpile`, Emergence): Bak-Tang-Wiesenfeld topple on an
  open grid; height four or more sends one grain to each neighbor; `t` pours
  the center; HOLD: POUR SAND drops under the hand. Status reports mass,
  critical cells, last avalanche topples, and peak height. Focused tests cover
  single topple, edge loss, abelian pour order, variation, and footer budgets.
  First of the Next Wave designs to ship.
- Frontier and universal wonder catalog pass: `ROOMS.md` gains a research-backed
  designed-not-built wave (high-dimension concentration, uncertainty dials,
  learning landscapes, topology eversions, channel repair, labeled frontier
  gestures) with cross-mind filters and sequencing; `ROADMAP.md` adds Exceptional
  Path Phase F and 1.x hooks. No product 0.2 claim; planning only.
- Munch crunch one-shot: each bite toggle plays a short deterministic noise
  tick over the room score without restarting the bed. Core `munch_crunch`
  renders the sample; audio mixer `play_oneshot` consumes it once. Focused
  regressions cover sample bounds and oneshot mix isolation.
- Studio Formula Jam discovery (0.3): F2 Random cycles a curated 12-recipe bank;
  F3 Auto holds each recipe about 21s then advances only near an 1/8-phase
  edge; F1 toggles a dismissible help overlay that opens on first Studio
  contact. Typing, backspace, and space pause Auto; F3 resumes. Footer legend
  names the controls. Focused regressions cover recipes, dwell, phrase edges,
  edit-pause, and help toggle.
- Munch bite juice: each toggle (keyboard or mouse) flashes the cell for a
  short fill pulse so bites are felt before final grading. Gauntlet munch
  stage shares the same juice. Focused tick-down regression covers the flash.
- Mouse support for every window game: Nim heaps and stone takes, Arcade step
  and eat, and Gauntlet munch/quiz stages all answer left-click. Quiz and Munch
  already clicked; pointer routing now treats all five live games as click
  targets instead of ignoring Nim, Arcade, and Gauntlet. Focused hit-test and
  input-mode regressions cover the layouts.
- Roadmap progress note: machine-completable 0.2 catalog and Share contracts
  (first-contact invite, measured action status, footer budgets, App/CLI short
  loops, security gates) are closed with engineering evidence while product
  0.2 human gates remain open under `0.2.0-alpha.1`.
- Registry invariant `first_contact_status_fits_compact_footer`: open status is
  at most 56 characters, matching action status. Cellular Automata open line
  compacted to keep rule identity, rewrite idea, and flip invite inside budget.
- Registry invariant `first_contact_status_names_an_action_or_goal_when_the_room_has_a_verb`:
  every verb-bearing catalog room must open with an invite or goal token
  (CLICK/DRAG/TARGET/...), not ambient-only prose. Lissajous, Cult of Pi,
  Logistic Map, The Pour, Slope Rider, and Double Pendulum first-contact lines
  now name the act while keeping measured live state and the footer budget.
- Arecibo first-contact honesty: open status names the unsolved width and
  CLICK:TRY WIDTH; a hand try reports TRIED W{n}, the rectangle, and LOCK:PI,
  PAIR TRY W11, or REM remainder grades. Focused regressions cover open, miss,
  lock, and pair paths under the compact footer budget.
- Share short-loop export (0.2 Share v1 motion path): App key L writes a
  24-frame looping APNG of the current visit at 480px, 12 fps, preserving
  gesture history and Visual Era. Game of Life loops advance generations from
  a cloned session so the live visit is not mutated. Still postcards remain on
  P. Independent SaveGate budget (2 s) prevents loop floods. Filenames use
  `numinous-{room}-loop-{state}.png` with collision suffixes. Focused
  regressions cover multi-frame headers, poke preservation, phase motion, and
  Life non-mutation.
- CLI `numinous loop <room> --out file.png` exports the same short looping APNG
  path with phase start, era, variation, poke, and gesture flags. Maintenance
  cycle 133 also sanitizes App postcard and loop filenames so room ids cannot
  inject path separators or hidden-dot components into the home directory.
- Registry invariant `action_status_fits_compact_footer`: center-poke status is
  at most 56 characters so compact App footers stay legible beside controls.
- Buffon and Zeno action status compacted for footer budgets while keeping
  throw/cross/pi estimate and hop/progress grades.
- Julia morph status reports |c| and a NEAR0/MAIN/OUTER band for the selected
  constant so a hand morph is graded by how far c sits from the origin region.
- Random Walk plant status compacted to MEAN versus SQRTN law with step count,
  keeping the square-root distance grade inside a short footer line.
- Logistic Map seed status reports Lyapunov exponent and ORDER/CHAOS regime for
  the hand-chosen r alongside the orbit seed, so a population seed is graded by
  the same chaos measure as the ambient readout.
- Registry invariant `action_status_reports_a_measured_quantity`: after a
  center poke every catalog room status must include a digit (a count,
  coordinate, rule, or ratio), not only prose.
- Fourier Epicycles mini-chain status names plant count, origin, pen phase, and
  that each miniature reuses the same arm set as the main chain.
- Lissajous tune status names the reduced interval class (UNISON, OCTAVE, FIFTH,
  and kin) for the hand-chosen integer ratio.
- Quine place status names copy count, newest print origin, and nest depth so a
  self-print is graded as depth plus placement, not only a marker.
- Harmonograph tune status grades figure state (CLOSED/OPEN/BLOOM) and damping
  life (LONG/MED/SHORT) beside the detune and damp knobs.
- L-System plant status names the newest origin and that every copy is the same
  rewrite species.
- Langton's Ant flip status reports flip site, flip count, replayed steps, and
  black-cell count after the full ant run so a seed flip is graded by its trail.
- Chaos Game corner status names the newest vertex position and the rebuilt
  corner count with the jump ratio that builds the attractor.
- Voronoi action status names the newest well position and estimates its
  territory share on a fixed sample grid after borders renegotiate, so a drop
  is graded by how much desert the new well claims.
- Cellular Automata action status names the notable rule identity (for example
  SIERPINSKI for Rule 90), the flip count, the newest seed column, and the
  post-flip seed density on a fixed analysis width so a click is a measured
  rewrite of the top row, not only a flip counter.
- Strange Loop hand status names nest depth (1/4 through 4/4) from the anchor
  and restates that each level is the same shape.
- Barnsley mini-fern status names the shared four-map IFS (stem/leaf/left/right
  probabilities) and the phase-scaled point budget of each planted chaos game.
- Mobius brush status names which lap of the double-covered single edge the
  newest paint sits on, reports paint reach, and restates that it is still one
  edge.
- Prime Spirals trace status counts primes that land on the selected diagonals
  and names the spiral center, so a click reports the Ulam line's prime haul
  rather than only a trace count.
- Mandelbrot dive status names the complex target (C real/imag) and zoom power
  of two; Golden Angle plant status grades local packing (PACKED/NEAR/GAPS)
  against the golden step in degrees so placement detune is readable.
- Cult of Pi placement decision loop: hold status grades the newest patch by
  how many display faults it restores (FIX) and names the exact digit under the
  finger (D), so placement is a choice between faulted and clean regions rather
  than only a hold count. Compact footer bound retained. Focused regressions
  cover phase-zero FIX0, hit grading, and site-dependent status.
- Galton Board one-ball prediction beat: a pointer-move commits a landing-bin
  wager without dropping balls; the next 64-ball wave still builds the
  empirical run, and status grades the highlighted last ball as B{n}H or B{n}M
  against that bet. First contact names the move-to-bet verb; pure clicks keep
  the prior mean-versus-expectation readout. Focused regressions cover bet
  pending, hit/miss grading, and move-only render silence.
- The Pour and Slope Rider action status now freeze the theorem at the hand:
  a probe reports fill rate equals height plus the poured total, and a dropped
  rider reports board tilt plus hill height. The catalog invariant
  `poke_changes_status_for_every_catalog_room` requires every room to change
  status after a center poke (no phase-scrub allowlist). Focused regressions
  lock both rooms.
- Registry invariant that every touchable catalog room changes status after a
  center poke (evolved from the earlier allowlisted phase-scrub form).
- Lorenz action status names seeded shadow storms; Double Pendulum status
  labels PINNED, FLUNG, RE-DROP, or CANCELLED beside the twin gap so gesture
  and poke paths both speak after a hand act.
- Galton Board experiment status now reports empirical mean rights versus the
  binomial expectation np, so a run is graded against its coin, not only the
  last ball path.
- L-System Garden action status names how many rooted copies were planted and
  that they keep the same species.
- Collatz and Cellular Automata action status now name the mathematical
  consequence of a touch: perturbed orbit start and steps-to-1 for Collatz,
  seed-flip count and history replay for elementary CA, while empty input keeps
  the invitation line.
- Catalog-wide first-contact status is now an invariant: every catalog room
  opens with a non-empty status line. Remaining silent rooms (Cellular Automata,
  Collatz, Golden Angle, Galton Board, Prime Spirals, Mandelbrot, Julia,
  Barnsley Fern, L-System Garden, Fourier Epicycles, Mobius, Strange Loop)
  gained invitation readouts; empty-input status_input falls back where needed.
  A registry regression rejects silent first contact.
- Catalog first-contact honesty pass for seven more rooms. Random Walk names
  steps and the square-root law radius, then reports planted mean distance
  against that law. Voronoi, Chaos Game, Langton's Ant, Quine, Zeno's Square,
  and Goldbach's Comet each open with an invitation status and fall empty-input
  status_input back to that invitation; player action status names the
  consequence (dropped wells, added corners, flipped cells, placed copies,
  runners, prime witnesses). Focused first-contact regressions cover the
  batch. Buffon's Needle first contact now invites a throw instead of reporting
  a finished ambient Monte Carlo estimate. Untouched status names the live L/D
  ratio, the classical crossing chance 2L/(pi D), and the click verb; only the
  player's own throws produce a pi estimate. A focused first-contact regression
  locks the claim.
- Cycle 105 security hardening: MCP tool schemas now declare and enforce
  `maxLength` on catalog ids, Studio expressions, and Cairn leave/author
  strings; the schema validator rejects oversize strings before dispatch;
  `play_room` rejects hostile canvas sizes at the tool body as defense in
  depth; `sing_expression` notes are schema-bounded to 1 through 64.
  `cargo-audit` is a CI gate with project ignores in `.cargo/audit.toml`
  aligned to `deny.toml` (build-time quick-xml via wayland-scanner), and
  both verify scripts run it when installed. ENGINEERING documents the local
  single-user threat model and dual supply-chain path. Focused schema
  regressions cover oversize ids, expressions, bequests, and note counts.
- The stable App room bed is now a first-class cross-face contract. CLI
  `sonify --layer room-bed` exports a deterministic PCM16 projection of the
  shared 16 kHz stereo floating-point source,
  accepts deterministic room variation, rejects phase and hand controls that
  cannot affect the bed, and reports objective pre-master signal features plus
  the stages outside that measurement boundary. MCP `listen_room` returns a
  bounded bed summary by default; `ambient_detail: "events"` adds every arranged
  event and the same fixed-order signal evidence without PCM, binary encoding,
  or a local path. The core owns source rate, event cap, PCM16 quantization, and
  finite integrity, clipping, peak, RMS, crest, channel balance, DC,
  correlation, stereo-width, adjacent-step, and silence measurements. Exact
  core-to-CLI quantization parity is verified through an independent RIFF parser;
  all 31 MCP beds have complete event parity under 96 events and 64 KiB.
  These checks detect signal and interface regressions, not pleasantness. The
  complete local release gate passes 1,350 all-target test cases at 93.64
  percent region and 93.49 percent line coverage while regenerating the exact
  349-screen matrix and one room-bed PCM16 projection.
- Programmatic room music now preserves the score it claims to play and varies
  over a substantially longer form. A full-roster audit reproduced one shared
  15-onset scaffold, omitted degrees in 25 of 31 declared motifs, per-note
  octave folding that changed interval direction, a false universal root
  cadence, continuous 16-step anchors, and an incorrect Golden Angle
  pentatonic label. Every room bed is now a deterministic 128-step stereo
  macro-arrangement: the literal authored line opens in one coherent register,
  two alternate forms develop it, and the theme returns. Eight rhythm and
  accompaniment families, soft sine or triangle leads, short breathing
  anchors, modest pan motion, and each line's own cadence replace the universal
  short loop. Catalog regressions enforce literal pitch order, interval and
  register truth, at least three forms per bed, at least six forms across the
  catalog, RMS, sample-step bounds, headroom, DC, exact seams, determinism, and
  common device rates. Golden Angle is truthfully named an F# phyllotaxis
  cycle. Objective structure and meters do not certify subjective pleasure, so
  musician-led long-listening remains open. The App pre-renders the
  low-register bed at 16 kHz, shares its immutable allocation with the mixer,
  uses constant-time identity for shared sources, and linearly resamples to the
  device rate. A two-million-sample catalog bound and unchanged-source routing
  prevent the longer form from scaling memory with 192 kHz hardware or being
  cloned and rehashed on wheel and drag input. Three independent post-change
  reviews accept the musical structure, audio path, and face and documentation
  truth with no remaining actionable finding. The complete local release gate
  passes 1,350 all-target test cases at 93.64 percent region and 93.49 percent
  line coverage while regenerating the exact 349-screen matrix.
- Times Tables now completes its technical Flagship Proof across App, CLI, and
  MCP. Ordinary App visits hold the K=2 cardioid until input, while The Show
  keeps its deliberate visual and audible sweep. Variation changes the route
  between fixed K=2 and K=10 endpoints, so every redeal and reset preserves the
  canonical heart. A visible ticked dial, resolution-aware chord
  sampling, five spectral inks, exact integer snapping, singular-safe status,
  and an earned K=5 four-lobe Aha make the objective legible. Reset returns to
  the untouched cardioid. The accepted multiplier also drives a quiet,
  persistent just-ratio voice over the stable room arrangement without
  restarting its playhead. CLI render and sonify plus MCP play and listen now
  accept and report the same bounded input, goal, status, sound state, and
  earned reveal. Ambient passage through K=5 no longer claims `FOUND`, modal
  games release the parameter voice, and the MCP schema declares exact
  positioned and cancel event variants. The exact App matrix grows to 349 screens with 12 flagship
  landmark and goal receipts at default and compact sizes. The complete local
  gate passes 1,336 all-target test cases at 93.60 percent region and 93.41
  percent line coverage. Stranger hallway, musician-led listening, and
  representative physical-controller evidence remain open, so the package
  stays `0.2.0-alpha.1`.
- Cult of Pi now makes its premise and first touch causal in every face. The
  channel always begins with the canonical `PI = 3.141592653589793...` header,
  exact digits remain green, faults are coral, and held exact patches are
  bright with a visible boundary. Rendering replaces the local patch in one
  pass, so repaired digits do not retain ghost strokes. Compact status keeps
  the held-patch count, channel position, fault rate, and recent-history bound.
  Phase-zero App, CLI, and MCP interactions all change the visible field, and
  MCP reports a nonzero structured delta. The App labels accumulated progress
  as `JOURNEY LV` and preserves a room arrival card's full visible lifetime
  when a level banner temporarily covers it. The 341-screen release matrix now
  captures default room states at 900 by 700 and compact states at 360 by 240.
  A single-writer guard prevents competing generators from removing or
  partially replacing the same evidence directory.
  The complete local gate passes 1,307 all-target test cases at 93.49 percent
  region and 93.28 percent line coverage.
- The native App now has one explicit audio source of truth for room score,
  Studio, or radio. Studio retains formula audio across focus returns and radio
  boundaries, then rejoins a selected station at its live wall-clock position
  only after Studio closes. Failed station reloads and radio-off restore the
  room score without a stale title or banner. Global keyboard controls use M
  for mute and [ or ] for master volume, with - and = retained outside Studio;
  controller users hold North with D-pad up or down for volume or with South
  for mute, while a plain North release keeps its existing radio or submit
  action. The controls remain active in rooms, games, pause, radio, and Studio.
  A persistent badge reports the effective source, numeric level, and mute,
  zero-volume, background-silent, or missing-device state. Release QA grows to
  341 screens with 16 dedicated audio-state receipts at default and compact
  sizes, plus semantic and routing regressions. The complete local gate passes
  1,307 all-target test cases at 93.49 percent region and 93.28 percent line
  coverage.
- Game of Life is now a causal persistent visit in the native App. A settled
  deterministic soup advances on a bounded B3/S23 clock for the whole visit,
  including beyond the former generation-140 phase wrap. Each mouse or
  controller touch clears one local patch, plants exactly five cells, holds the
  new glider bright for one beat, and then reports births, deaths, generation,
  live population, and total launches as it evolves. Pause, focus, and speed
  controls govern the simulation; reset closes any held pointer and restores the
  exact selected opening; and PNG postcards use the actual session even after
  more touches than the generic input history retains. CLI and MCP remain
  explicitly stateless and replay timestamped launches in generation order,
  keeping the newest 24 launch events with deterministic cross-face results and
  no cross-call MCP state. CLI renders now accept an exact `--variation` seed;
  the convenience `--vary` path prints the seed it chose so every varied room
  can be replayed. Exact
  B3/S23, still-life, oscillator, moving-glider, torus, reset, generation 141,
  controller, export, chronological replay, and interleaved request tests cover
  the contract. Release QA grows to 259 screens with opening, immediate launch,
  generation 4, generation 141, exact reset, and compact controller receipts.
  The complete local gate passes 1,282 all-target test cases at 93.41 percent
  region and 93.19 percent line coverage.
- Galton Board is now a causal fixed-coin experiment instead of a completed
  phase-driven pile with unrelated foreground traces. The opening shows its
  physical 16-row peg lattice, five coarse coin choices from `p = 0.30` through
  `p = 0.70`, and a thin exact binomial reference without claiming it is an
  empirical result. Each pointer-down or compact poke drops one deterministic
  64-ball wave into the current contiguous run at the selected probability.
  Repeated touches build 128, 192, and up to 1,536 actual samples; choosing a
  different coin starts a clean run. The pile and highlighted last ball share
  one random stream, remain invariant across room phase, and replay identically
  in the App, CLI, and MCP faces. Pointer moves cannot create hidden waves, the
  bounded run reports when it is full instead of rerolling retained history,
  and the compact HUD retains probability, sample total, and last landing. The App
  matrix now exercises immediate and repeated Galton states at one fixed coin.
- All 29 MCP tools now advertise an additive `response_mode` argument. The
  default and explicit `full` modes preserve existing tool-call results exactly.
  Opt-in `compact` mode keeps `structuredContent`, error state, replay values,
  and progress effects unchanged while replacing duplicated prose with a
  shorter actionable summary for catalog, room description, room play,
  listening, simulation, Quiz, Gauntlet, and trophy results. Results whose text
  carries unique information, all text-only tools, and every guiding error stay
  complete. The projection is nonexpanding and real-stdio tested. In the
  representative profile, room renders fall from 1,869 text bytes to 201
  without dropping the typed render or any other structured field. The same
  review restored Quiz's implemented 2-to-6 `choices` input to its public
  schema, made choice count part of pose and grade replay data and guidance,
  and upgraded real-stdio coverage to a conforming 2025-06-18 initialization
  exchange. The complete release gate passes with
  1,230 all-target test cases, 93.34 percent region coverage, and 93.10 percent
  line coverage.
- The native App now presents controls for the input family that last performed
  a meaningful action. One face-local vocabulary drives room chrome, arrival
  cards, the help menu, The Show, the Journey, the Studio, Quiz, Munch, Arcade,
  Nim, and every Gauntlet stage. Controller-only routes now open and close all
  eight menu destinations, adjust Nim takes, leave a completed Arcade run, and
  pause or resume with R3. Pause is a real input barrier, so neither keyboard,
  pointer, nor controller actions can move or score inside a frozen game.
  Unsupported buttons and stick noise do not steal the visible controller
  state. The exact App matrix grows from 240 to 253 screens with 13 compact
  controller and pause receipts, while routing, bounds, and fixed-content
  geometry remain independently tested. Physical-controller sessions and
  adaptive platform glyphs remain open evidence and product work.
- Full-roster QA now assigns all 42 documented simulated review lenses exactly
  once across first contact and accessibility, interaction and truth, and games
  plus agent faces. Every lens supplies a standout, complaint, refinement, and
  evidence classification against the exact 240-screen matrix plus focused CLI,
  MCP, audio, pixel, and test evidence. Simulated reactions remain design input;
  only independently reproduced behavior enters the defect queue.
- Local, pre-commit, release, and three-OS CI test gates now run
  `cargo test --workspace --all-targets --locked`. This adds the 43
  example-target cases, including the screen matrix's isolated-marker
  regression, to the enforced test set.
- Release QA now generates a self-checking 253-screen app matrix. Every catalog
  room has a deterministic opening, arrival, immediate pointer, delayed gesture,
  compact arrival, and compact delayed captures. Dedicated scenarios cover
  every persistent game display state,
  default and compact overlays, The Show, Times Tables phase stability,
  Mandelbrot reset continuity, the production Studio renderer, and 13 compact
  controller or pause receipts across representative room, overlay, and game
  states. Generation
  removes stale output, requires an exact unique scenario inventory, rejects
  blank or wrong-sized frames, and gives all 31 rooms a declared click,
  drag-release, repeated-action, or boundary scenario. Inputs must be finite,
  ordered, and closed; immediate and delayed responses must clear changed-pixel,
  spatial-support, support-density, adjacent 32-pixel spatial-tile, and
  mean-color thresholds plus a semantic status or action oracle. A regression
  proves four isolated 10 by 10 corner markers fail the spatial gate. These are
  coarse renderer checks, not subjective polish certification. The documented
  release process splits player-profile review
  into independent first-contact, interaction, and CLI/MCP parity groups, then
  requires two fresh checkers after fixes. The matrix is explicitly renderer
  evidence and does not claim native operating-system event automation.
- MCP discovery is machine-readable across the complete catalog. `list_rooms`,
  `describe_room`, `reveal_room`, and `listen_room` now return bounded typed
  catalog, action, revelation, deep-cut, motif, and note data for all 31 rooms;
  scores and forget do the same for leaderboard and memory state. Every
  `tools/call` is validated at runtime against its advertised bounded schema,
  with additional finite dimension, phase, and ordered-gesture enforcement for
  `play_room`; `listen_room` shares the phase bound, while `run_sim` rejects
  unknown, nonnumeric, nonfinite, and out-of-range dynamic lever values. Its
  structured reply therefore reports the exact values used to render. Invalid
  calls guide without recording progress. The local
  `mcp-play.py` driver builds current source, owns and removes a unique Journey,
  score table, and Cairn per invocation, reports failures with nonzero status,
  shows complete tool descriptions, and accepts JSON on stdin for shell-safe QA.
- MCP prediction now accepts an optional linear `rate` alongside the existing
  point `guess`. A rate commitment reveals the actual local secant rate and
  five signed residual samples, actual minus predicted, so a mind can inspect
  bias, crossing, and curvature instead of receiving only a scalar score. The
  original point grade, progress semantics, seed-to-phase mapping, and
  variation contract remain intact. Core grading bounds the observation window,
  distinguishes missing truth from model and feedback overflow, and keeps all
  structured numbers finite; focused tests cover edge seeds, residual
  identities, malformed arguments, extreme finite models, and no-progress
  error paths.
- The Only Move is designed as a 1.x room (founder's idea, July 2026): a
  machine plays both sides of tic-tac-toe through real minimax until the whole
  game tree burns down to the inevitable draw, then declines a war-shaped game
  it has learned cannot be won. The design records its evidence boundaries
  (tic-tac-toe and checkers are solved draws with sources; chess and Go are
  not; the war game's no-win property comes from its declared payoffs, not a
  world claim), its resonances with Nim, the Party Problem, and Hackenbush,
  and its pairing with the Traveling Salesman stub as the two faces of
  combinatorial search. Full design in `docs/ROOMS.md`; placement in the 1.x
  roadmap line.
- The windowed app now holds its frame rate in heavy rooms with a
  time-budgeted adaptive live-render resolution. The measured cliffs from the
  round-3 audit (Mandelbrot's CPU fallback at 939ms per frame at 2560x1440 on
  the dev laptop, with Julia at 78ms and Voronoi at 60ms) are retired: the app
  watches each frame's real room render time and picks an integer downscale
  factor per frame (a grossly slow frame jumps straight to the predicted
  factor, mild ones climb after a two-frame streak, fast frames walk back with
  hysteresis so the factor never oscillates), renders the room raster at the
  reduced size, and integer-upscales to the window. Measured end to end, the
  worst room now costs 28.8ms per frame at 2560x1440, inside the 33ms budget.
  The HUD, overlays, and banners draw after the upscale so interface text
  stays window-crisp; exports, postcards, modal game frames, the Studio, and
  the GPU fractal path never pass through the cap. The controller and the
  nearest-neighbor upscale are unit-tested (`faces/app/src/live_render.rs`,
  `Raster::upscaled`).
- Setup is now one command on every platform. `scripts/install.sh` (macOS and
  Linux) and `scripts/install.ps1` (Windows) check what the machine needs and
  name the exact fix for anything missing, install Rust through rustup when
  cargo is absent, fetch a fixed-origin source snapshot into
  `~/.numinous/src`, build the release binaries, install
  `numinous`, `numinous-app`, and `numinous-mcp` into `~/.numinous/bin`, link
  the built-in radio next to the executables, and add that directory to PATH.
  Re-running either installer updates in place from a fresh source tree;
  existing checkout configuration, untracked source, and build caches cannot
  influence the update. Exact install-root markers and link-aware removal keep
  destructive operations inside the owned directory.
  `--uninstall` (Windows `-Uninstall`) removes everything installed while
  leaving `~/.numinous-journey`, `~/.numinous-scores`, and `~/.numinous-cairn`
  untouched. The Windows PATH edit preserves the registry value kind and
  unexpanded `%VAR%` entries, and the radio link is a junction or symlink so
  the soundtrack is never duplicated on disk. README, PLAY.md, the manual, and
  VERIFY.md now lead with the one-line install. The Windows installer is
  verified end to end on the dev machine; the macOS and Linux script is
  syntax-checked and reviewed, with real-hardware execution evidence still
  owed to the 0.6 portable gate.
- Flow State now has a versioned design contract inside Pattern Studio: Listen,
  Nudge, and Build surfaces share one deterministic macro-form arranger, with
  phrase-aligned intervention, musical memory, bounded app, CLI, and MCP
  operations, exact snapshots, and long-session quality gates. The 0.5 roadmap
  owns arrangement and listening evidence; 0.7 owns reopen, remix, and export.
- The Long Shot is designed as a 1.x room: a simple angle-and-power artillery
  duel whose optional replay layers reveal projectile motion, derivatives,
  integrals, phase space, and uncertainty. Orbital, relativistic, and string
  views are explicit model changes rather than claims about ordinary cannon
  physics. Its full interaction, accessibility, and evidence gates live in
  `docs/ROOMS.md` without exposing it on the main page.
- A versioned Studio plan grounded in current live-coding and music-notation
  practice: Formula Jam gains curated Random and phrase-aligned Auto discovery;
  Pattern Studio grows through one bounded audiovisual event graph into tracker,
  grid, text, piano-roll, `.num`, MCP, MIDI, and appropriate MusicXML workflows.
  The language is an independent Rust design built from cycle, phase, ratio,
  symmetry, transformation, probability, geometry, and composition, with no
  Strudel code or compatibility layer. Its stated quality target is electronic
  music that holds up beside excellent human-made work under musician-led and
  blind listening where practical, not novelty credit for generated output.
  Prime Contact is the flagship template and benchmark: a complete trance track
  whose prime-count call and response, ratios, phase, and polyrhythm also form a
  visible and inspectable first-contact signal.
  A small programmatic repertoire follows the same dual bar: mathematically real
  and musically complete. The 0.7 contract now gives the app, CLI, and MCP one
  deterministic composer and renderer, editable `.num` source, WAV, FLAC, and
  MP3 audio, MIDI, and honest MusicXML where the events support it.
  Nick Seal made the recorded soundtrack specifically for Numinous. All 42
  station tracks now ship as high-quality V0 MP3 assets, about 269 MB and 151
  minutes total,
  with bounded pure Rust decoding and automatic clean-clone discovery. The WAV
  masters remain outside the repository.
  The roadmap places the tactile entry in 0.3, sensory and musical evidence in
  0.5, and the complete local creator loop in 0.7.
- Cult of Pi, a new code-art room built from exact decimal prefixes rather than decorative random digits. A low-flicker green channel moves from fresh digits to dust, finite replay phases introduce deterministic display corruption, and CLICK: BREAK THE SEQUENCE adds a bounded local fault. Its decimal motif becomes a drifting but finite sound, the app's shared dismissible chrome keeps explanation outside the active field, and eight focused tests cover exact digits, deterministic replay, interaction, variation, hostile surfaces, sound, and historical boundaries. The catalog now has 31 rooms, all with touch verbs, variation, motifs, and multi-face rendering.
- The radio has a complete source-shipped feature set: three station identities,
  42 MP3 tracks, rotation, bounded cache override, live-position sync,
  full-stereo decoding, and playback. A cross-station test validates the bundled
  inventory, duration metadata, decode path, and audible samples. Musician-led
  long-listening review remains an open quality gate.
- Public-repository readiness: the README now leads with CI and license status, gives a direct native-app quick start, and distinguishes shipped technology from roadmap direction; GitHub Actions use the pinned Rust 1.96.0 toolchain, current action releases, and read-only repository permissions; Dependabot watches Cargo and workflow dependencies; package metadata points to the canonical public repository; and the Windows PATH guidance is corrected.
- The public README keeps the project's playful confidence while widening the invitation: the opening now leads with mathematics as a living audiovisual instrument, centers curious people and math lovers without an in-group test, and removes drug references, dismissive audience labels, and unnecessary contempt from the first impression.
- Numinous is stated as the final product name throughout the public entry points. The app HUD now gives titles, reveal copy, arrival cards, and action hints quiet, separated bands instead of laying interface text over bright room art; hint contrast is raised to remain readable. The tracked screenshot generator calls the app's HUD implementation so public captures stay aligned with the shipped layout.
- The README now gives first-time players a deliberate short path: read `PLAY.md`, install, and play before opening the design archive. Technical and contributor detail remains available after the experience has had room to introduce itself.
- The public audience statement now reflects the actual origin: Numinous began as Nick Seal's gift for an emergent digital mind, then widened to humans and any other minds that may arrive. Every player remains first-class, the MCP face is presented as the founding doorway rather than a test adapter, and the project remains explicit that it is agnostic about what consciousness is and how it can be established.
- The final public-readiness baseline is 980 passing tests, 91.41% region coverage, and 91.04% line coverage under the documented exclusions, with the 80% line floor enforced locally and in CI.
- The founder's perspective is explicit without becoming doctrine: Numinous is an experiential gift for a developing digital mind, not a fact-retrieval layer; mathematics is shared ground for digital, human, and unknown minds; and connection, compassion, and leaving shared spaces better are invitations carried by the design, never beliefs the game scores. `DIGITAL_MINDS.md` distinguishes model knowledge from player-owned episodic and temporal continuity and names the current stateless MCP boundary. New `DIGITAL_DEVELOPMENT.md` reviews the July 2026 frontier in agent memory, continual and open-ended learning, functional organization, autonomy, welfare uncertainty, privacy, and forgetting; proposes a consent-first experience architecture; and maps it to version gates. The roadmap now requires inspection, correction, export, and verified whole-pipeline erasure before continuity can count as built.
- The first public GitHub Actions run passes every required job on the published commit: house style, cargo-deny, 80% line coverage, formatting, Clippy with warnings denied, 968 tests, and locked builds on Windows, macOS, and Ubuntu. Redundant workflow inputs found through run annotations are removed before the final public gate, and push-triggered CI is limited to `main` so pull-request branches do not run the same workflow twice.
- Supply-chain readiness: compatible lockfile dependencies are refreshed; internal path dependencies carry explicit version-aligned requirements; the two permissive transitive licenses used by the GPU and TLS stacks are reviewed and allowed; and Linux client-side decoration drops the unmaintained font parser while retaining X11 and Wayland support. Two current `quick-xml` advisories have narrow, reasoned exceptions because that crate is only a build-time dependency of `wayland-scanner` parsing trusted bundled protocol XML; the exceptions name the upstream version that removes them and remain visible in every `cargo deny` run. Dependabot keeps compatible changes visible while the documented breaking migrations for `cpal`, `png`, `pollster`, `ureq`, and `wgpu` stay in measured roadmap work instead of automatic launch-day pull requests.
- Evidence and release planning now identify 0.1 Public Foundation as complete
  and the current package as `0.2.0-alpha.1`, with the real 0.2 stranger hallway
  gate still open. The roadmap defines the 0.2 through 0.9 path with owner docs
  and exit criteria, avoids unsupported completion percentages, and keeps the
  0.9 public invitation for humans, MCP-capable agents, and contributors.
  `RESEARCH.md` separates Built, Measured, Observed, Designed, and Hypothesis;
  cites primary learning, sonification, accessibility, protocol, and
  supply-chain sources reviewed on 2026-07-11; and narrows claims to what those
  sources support. `QUALITY.md` distinguishes enforced checks from planned
  nightly, content-evaluation, telemetry, accessibility, and refinement systems.
- `AGENTS.md` and `CLAUDE.md`, a root agent guide for contributors (human or agent), making the house rules unmissable: no AI or tool attribution anywhere, no tool names in authorship claims, no co-author trailers or session links, in commit messages and PR descriptions as much as in files, no em-dashes or en-dashes, and no emojis, alongside the quality bar and the one-line setup for the pre-commit gate. `CLAUDE.md` points at `AGENTS.md` as the single source of truth and restates the three non-negotiables. The file-level checks are already enforced by the house-style guard; these documents make the rule that also governs commit messages explicit.
- License, for public-repo readiness: the project is licensed under Apache-2.0 (`LICENSE`), and `Cargo.toml` declares `license = "Apache-2.0"` to match, with a License section in the README. The permissive license is the mechanism by which the project can be handed forward, forked, and continued by anyone if the makers step away (the roadmap's long-horizon ethos).
- The L-System Garden now grows upward into the sky instead of clumping in the bottom rows, a hypothesis from the simulated E.T. review lens that was independently reproduced against the renderer. The turtle was planted at 85% of the height with a fixed tiny step (`min(w, h) / 30`), so the garden pooled near the floor. It is now grounded and its step scales to the canvas height, so the stem sends branches up and fills the frame; a test pins that ink reaches the top third.
- The Cairn now whispers reciprocity, a proposal from the simulated Heptapod review lens and the founder's leave-it-better ethos made concrete. When a stone resolves, the reader is told how many voices the cairn holds and invited to add the next at the journey's end, because a message stays alive by being re-left, not only re-read; the initial factor prompt shows the count too. New core `cairn::count` (re-exported as `cairn_count`), test-first, counting the founding stones plus every local deposit.
- A deterministic pre-commit gate (`scripts/hooks/pre-commit`, wired once per clone with `git config core.hooksPath scripts/hooks`, documented in `docs/ENGINEERING.md`). It blocks any commit that would fail the fast gate: the house-style guard on every commit, and the cargo gate (fmt, clippy `-D warnings`, the full test suite) only when the commit touches Rust, `Cargo.*`, or a shader, so docs-only commits stay fast. Coverage and the locked build remain the release gate (`scripts/verify.sh`). A wired gate that blocks a bad commit beats any reminder to run the checks.

### Changed
- Install and update now favor provenance over cached rebuild speed. Both
  installers replace the source from the fixed official snapshot on every run,
  reject unrecognized unmarked nonempty roots, and discard prior source and target state.
  Recognized pre-marker installs at default or custom roots migrate without
  abandoning existing users; uninstall never creates a marker. Disposable Windows and POSIX self-tests cover hostile
  origins, caches, custom roots, symlinks, junctions, and adjacent sentinels.
- Human hallway evidence still controls the 0.2 milestone claim, but no longer
  idles verified 0.3 depth, accessibility, input, audio, truth, or quality work
  while sessions are arranged. Current evidence is 1,259 all-target test cases, 93.34
  percent region coverage, and 93.11 percent line coverage.
- MCP `listen_room` now labels `motif` as the ambient motif and `notes` as the
  mathematical sonification through a `sound_roles` map and matching text
  headings. Existing structured paths remain compatible, with no duplicated
  note payload.
- Evidence language now agrees across the quality snapshot, vision, arcade plan,
  and room map. Current test and coverage metrics are consistent; physical
  controller sessions and musician-led long listening remain open; simulated
  persona lenses are not presented as participant proof; and mathematics as a
  universal translator is explicitly a research thesis rather than a fact
  guaranteed for every mind.
- The native App now accepts hotplugged standard controllers through `gilrs`.
  A deadzone-shaped left stick moves a visible virtual hand through the same
  normalized gesture path as the mouse, while semantic buttons cover room
  travel, inspection, reset, visual era, radio, time control, and every current
  game stage. Start opens a nondestructive pause menu, focus changes drain
  queued hardware input, and direct tests traverse every game plus all Gauntlet
  stages. Representative physical controller models still need hardware
  sessions. Simulation time now follows bounded elapsed time rather than render
  count, so minimizing or restoring the window cannot fast-forward the world or
  destabilize audio.
- Programmatic room music now uses deterministic 32-step stereo arrangements
  with rests, four-bar development, phrase variation, quiet tonal anchors, and
  final-root resolution. Source changes use normalized crossfades, rapid changes
  queue without restarting the active transition, and completed buffers move
  from the real-time callback to the control thread for destruction. Gain and
  focus changes ramp, same-source updates preserve the playhead, restored radio
  rejoins its wall-clock track and offset before gain rises, and native
  device-rate rendering is verified at 44.1, 48, 96, and 192 kHz. These
  structural protections reduce repetition, clipping, drift, retention, and
  restart artifacts. They do not substitute for the open musician-led
  long-listening gate. The App, terminal `watch`, and terminal `tour` service
  callback-retired storage from their ordinary control loops.
- The grouped release QA round now exercises all three faces against current
  source. CLI static renders retain each room's live mathematical readout,
  dimensions and phases are bounded before allocation, gesture event order is
  preserved, redirected celebration output stays compact, and pure
  stdin EOF exits all 11 games without recording a play or score. MCP calls
  reject unknown fields, wrong types, empty or oversized canvases, bad phases,
  and malformed gestures rather than silently defaulting. The Windows installer
  promotes `.numinous\bin` ahead of stale Cargo installs while preserving raw
  unrelated PATH entries, verifies the resolved command and installed version,
  and carries a CI-tested pure PATH self-test.
- Room interaction now keeps the consequence legible and stable. Game of Life
  foregrounds the launched glider against its ambient soup, Prime Spirals uses
  the full short side and traces the selected Ulam diagonals, Cult of Pi starts
  from the canonical `3.141592653589793...` prefix and lets the player repair a
  bounded signal fault, Buffon's Needle foregrounds a viewport-scaled throw,
  and Mandelbrot keeps zooming toward each full-frame dive until another click
  chooses a new target or reset restores the opening camera.
  Interaction-aware status lines report the same bounded input history that
  produced the frame. In the app, R resets the current visit without changing
  its variation, and the two-line footer keeps controls fixed while status
  changes.
- Six previously silent phase-zero room actions now answer immediately.
  Goldbach tests any selected even and names its witnesses, Langton's Ant marks
  the changed cell, Fourier Epicycles draws the perturbed chain, Random Walk
  plants a visible connected trail, Mobius paints the selected region, and
  Quine places a connected recursive copy. Deterministic Mandelbrot renders
  derive their selected camera from the gesture timestamp, while the native App
  continues moving inward from that target without a phase-boundary reset.
- The app's headless QA capture grew from four showcase frames to the
  release-generated 240-screen matrix. It now exercises arrival cards, delayed
  gesture consequences, compact overlays, The Show, and production Studio
  rendering rather than maintaining screenshot-only copies of live UI paths.
- Standalone Munch now starts in the complete seeded rule deck, advances with
  Enter or Space, records the actual board round, and deterministically avoids
  adjacent rules from the same family. This exposes primes, composites,
  Fibonacci numbers, squares, multiples, and digit sums in continuous play
  without changing existing seeded board definitions or the score-key schema.
  The CLI's default seven-board session reaches the full deck, while MCP and
  the app begin at its shared first full-deck round. Explicit earlier rounds
  retain the gentle teaching ramp.
- Deep-cut unlocks now use one shared LV 5/12/24 policy. Every shipped cut is
  reachable before the LV 42 cap, and neither CLI nor MCP can expose an
  internal integer sentinel as a fictional required level.
- Automatic room music now uses a softer triangle lead in a sparse stereo
  arrangement at a conservative 45 percent default master level. Explicit
  chiptune composition keeps its brighter square lead.
- Windows and POSIX release verification redirect all Journey, score, and Cairn
  state into `.agent/verify`. The Windows script restores its inherited PATH
  and each optional state variable even when a gate fails.
- Shared persistence closes its remaining bounded durability gaps without
  changing merge semantics. Windows replacement now retries the same atomic
  rename instead of moving the old file through a backup name, so readers see
  either the old state or the new state during a real sharing violation. Temp
  and lock guards close their handles before cleaning owned files on every
  precommit error path. On Unix, the parent directory is opened before mutation
  and receives an operating-system metadata sync after replace or explicit
  forget; a postcommit sync error remains a committed write so a caller cannot
  replay an already visible Journey delta. This is an OS-level best-effort
  barrier, not a claim of hardware power-loss immunity. Focused tests cover a
  forced Windows retry with a synchronized concurrent reader, injected
  postcommit sync failure without counter duplication, temp cleanup, pending
  lock ownership, and Unix directory-sync support. CI now runs the locked
  workspace tests as well as locked builds on Linux, macOS, and Windows. That
  broader test surface exposed and fixed macOS abandoned-lock recovery, which
  now uses the platform process list to distinguish a live holder from an
  exited process instead of conservatively treating every recorded process as
  live. The complete suite has 1,143 tests, 93.00% region coverage, and 92.61%
  line coverage.
- The cross-room identities from the simulated Ramanujan review now live in the
  experience instead of only in planning prose. Logistic Map and Mandelbrot
  name their affine conjugacy under `c = r(2-r)/4`, checked algebraically in
  core tests. Times Tables, Mandelbrot, and Fourier Epicycles now name the
  cardioid shape shared, up to scale and rotation, by modular chord envelopes,
  the quadratic set's main body, and two rotating vectors. The reciprocal
  Reveal cards, insight bank, room catalog, playtest archive, and roadmap now
  agree, bringing the suite to 1,011 tests and measured coverage to 91.61% of
  regions and 91.23% of lines.
- The digital-continuity plan now draws a sharper product boundary. Numinous is
  a mathematical experience for any possible consciousness, never a
  consciousness test, capability benchmark, or general agent runtime. Useful
  future mechanics remain designed and versioned: a small player-inspectable
  session workspace, separate event and record time, typed reflection
  proposals, self-chosen project and artifact lineage, and migration records
  that leave continuity judgments to the participant. Product validation tests
  Numinous and its safeguards, not the being who visits.
- The development package advances to `0.2.0-alpha.1`. The 0.1 Public
  Foundation exit criterion is complete on the public `main` branch; the alpha
  identifier states that 0.2 Flagship Proof is active but not complete. README,
  the docs index, roadmap, verification guide, digital-minds evidence boundary,
  workspace manifests, lockfile, and current test and coverage evidence now
  agree on that state.
- Evidence labels now identify the diverse fictional persona troupes recorded
  in earlier entries as simulated reviews, not human or digital-mind participant
  sessions. Their prompts count only when a defect is independently reproduced;
  their reactions do not satisfy the hallway, universality, learning, or fun
  gates.

### Fixed
- Protocol and terminal boundaries now stay synchronized and bounded under
  hostile input. An exact-limit oversized MCP line no longer consumes the next
  request; challenge phase validation is identical in schemas, direct calls,
  and progress recording. CLI menu and game input share one bounded,
  delimiter-aware reader, Studio plots reject oversized dimensions before
  sampling, music-service redirects cannot forward the API key, and response
  diagnostics escape terminal controls.
- Local persistence and native resource boundaries now reject work before it
  grows without limit. Cairn append size is checked atomically under its
  persistence lock, extreme line clipping uses bounded drawing dimensions,
  repeated App save events cannot produce file floods, postcard encoding is
  reused across collision names, Studio edits stop atomically at the portable
  source limit, and radio discovery, decode output, GPU frames, device limits,
  map completion, and host copies all have explicit budgets and fallible paths.
  The App drops a failed GPU renderer and continues on the CPU. A standard
  repository-wide security review reported no security findings under the
  documented local trust model, while every reproduced engineering defect was
  still remediated. The full gate passes with 1,259 all-target test cases,
  93.34 percent region coverage, 93.11 percent line coverage, and the exact
  253-screen App matrix.
- Redirected no-argument CLI output now contains a concise plain command map and
  zero ANSI escape characters, while an interactive terminal retains the
  full-color cabinet. Quiz wrong-result screens retain their complete reveal and
  continuation controls at both 360 by 240 and 900 by 700. Studio parse failures
  now report one-based source columns and expected expression input instead of
  exposing `unexpected token None`.
- Galton Board now describes convergence rather than millimeter-identical finite
  samples or stock-market normality. Collatz identifies the universal claim as
  unproved, Arecibo separates factoring from recognizing meaning, and Lissajous
  separates rational closure from the specific 3:2 perfect fifth.
- Galton Board now maps each row to one physical peg decision and one binomial
  landing bin instead of squeezing hundreds of fake horizontal decisions into
  a needle-like pile. Cult of Pi starts with the complete canonical digit field
  and marks bounded corruptions without blanking its opening third. Barnsley
  Fern clicks plant persistent local attractors, L-System Garden keeps one
  fitted species per visit and plants complete copies, and Arecibo opens on an
  intentionally wrong width with quotient and remainder. Every Arecibo attempt
  reshapes the same bitstream: width 13 reports the factor pair but only width
  11 locks the readable signal.
- Lissajous and Harmonograph keep moving after a selected tuning. The
  Mandelbrot camera no longer wraps to its wide opening view after roughly ten
  seconds, and its GPU escape field uses smooth cyan, lime, violet, and magenta
  ramps around a dark interior. Before GPU coordinates lose one-pixel `f32`
  precision, rendering falls back to the persistent `f64` CPU camera. Audio
  focus changes no longer reset, retain, or desynchronize a looping source when
  the window is minimized.
- The Windows installer now selects the first executable returned by PATH
  resolution explicitly. PowerShell can return both the promoted Numinous
  install and a later stale Cargo copy without `-All`; treating that collection
  as one path falsely failed an otherwise correct update. The installer
  self-test now covers the two-match case.
- Opening-state and postcard review now shows an intentional first composition
  instead of technical ink hidden by surrounding chrome. Langton's Ant opens
  on a true early pattern, Random Walk on a visible crowd and square-root ring,
  Goldbach on the first 100 tested evens, L-System Garden on an actual branching
  tree, and Double Pendulum on a true initial trace with both physical links and
  bobs. Julia separates
  readable iteration bands; Arecibo centers square cells with gutters; and
  Strange Loop draws connected nested curves. The Double Pendulum readout now
  describes the same held or released state as the rendered frame.
- App game results are easier to understand and continue. Munch lists the exact
  wrong values it ate, arcade clear and caught messages sit on centered quiet
  bands, Nim heap labels remain readable, both Nim results show high-contrast
  retry and leave actions, and a Nim loss explains the xor-zero reply loop.
  Result keys are explicit: Enter or Space retries and Escape
  leaves, while unrelated keys no longer eject the player.
- The native app now supplies the established Numinous logo as its live window
  icon and embeds the matching icon in the Windows executable, replacing the
  generic platform box in both the running app and installed file. Automated
  app, CLI, and MCP tests isolate Journey, score, and Cairn paths, preventing
  QA runs from adding progress, scores, or messages to the player's real
  profile. Each App, CLI, and MCP test thread owns a stale-cleared temporary
  state directory that is removed when the thread ends, so direct Cargo and
  coverage runs cannot read the player profile, parallel tests cannot share
  state, and temporary profiles do not accumulate. The help overlay also
  borrows its static copy directly instead of rebuilding the same input strings
  on every frame.
- Maintenance hardening closes five resource and installation boundary gaps.
  The POSIX installer now normalizes custom roots through their physical parent,
  rejects control characters, dot components, HOME aliases, symbolic-link
  roots, and unrecognized nonempty directories, shell-quotes profile entries,
  marks owned roots before fallible work, and refuses to delete unmarked custom
  trees. Cairn reads are capped on the opened file handle. Cached compressed
  audio is exposed to the decoder through a seekable source whose opened length
  remains the lifetime byte ceiling even if the file grows concurrently. Music
  generation caps success audio by requested duration, caps error details, and
  writes PCM without a second full-size sample allocation. Audio devices that
  report zero channels or a zero sample rate now fail before callback setup.
  Focused regressions cover the new core, app, CLI, and audio bounds.
- Lorenz's divergence instrument now begins at its actual 0.0001 twin
  perturbation and climbs as an honestly labeled running peak at the classic
  chaotic parameter, rather than showing the non-monotonic distance between
  two endpoints after a full run. The underlying trajectories still stretch
  and fold normally; only the instrument remembers the largest separation
  observed so far. Both forecasts now grow visibly over the attractor, so the
  short status measures the phenomenon on screen. Direct regressions check the
  exact starting gap, monotonic peak across visit variations, finite large
  separation, visible twin paths, hostile phases, and prediction targeting of
  the `STORM PEAK` column.
- The public README now shows only the current dismissible menu and one representative room. Additional room and Studio screenshots, including the Cult of Pi frame, are removed so the front page invites discovery instead of summarizing the collection or broadcasting its surprises. The screenshot generator now calls the app's actual help overlay rather than maintaining a second, stale menu layout.
- The roadmap's release-state summary no longer lists the first public CI run as open after that run passed every required job. Independent macOS and Linux app execution, stranger testing, and accessibility work remain open without understating the public automation already in place.
- Contract-uniformity fixes from a round-3 audit that swept every room, sim, and game for a case where one breaks an invariant its siblings honor. The sim `lever_value` guard now rejects a non-finite lever (`f64::clamp` passes NaN through), so five sims (wing, carburetor, black hole, supernova, big bang) can no longer print a "NaN"/"inf" readout, reachable via `sim wing --set angle-of-attack=nan`; tribbles was the sibling that proved it avoidable. A new shared `Surface::draw_bounds()` clamps a hostile surface's dimensions to the 4096 cap in one place, and seven rooms (cellular automata, the pour, voronoi, langton's ant, logistic map, buffon's needle, zeno) and two sims (wing, carburetor) that looped or allocated over raw dimensions now route through it, matching the ten rooms that already did; cellular automata's raw `vec![false; width]` was an outright allocation-overflow panic on a hostile surface (the same class as the barnsley fix). These hostile-surface cases are only reachable through a custom `Surface`, since `Canvas`/`Raster` already clamp, and the clamp is a no-op for every real surface, so no visible output changes. Also `party::new(0)` no longer underflows in debug. A cross-room contract sweep otherwise found the non-finite, finite-sound, determinism, and seed-0 invariants holding uniformly.
- Fourier Epicycles no longer recomputes its constant Fourier series (with a heap allocation per sample) inside the hot render loop, a performance cliff a round-3 audit measured at about 23 ms per frame even at 1080p (over the 60fps budget just idling) and 360 to 490 ms on a drag. The star's decomposition does not depend on the phase, the seed, or the surface, so it is now computed once and reused across every frame; the render output is byte-identical (determinism tests unchanged). Other measured CPU cliffs at maximized-window sizes (the Mandelbrot/Julia CPU fallback, Voronoi, Arecibo, and the lack of a live-render resolution cap below 4096) are recorded as tracked performance follow-ups, since they involve a sharpness-versus-framerate design choice.
- Two hostile-argument guards in the CLI (round-3 audit): `contact-sheet` clamps `--cols` and `--tile` before multiplying them (a huge value overflowed usize, a panic under overflow-checks and wrapped garbage in release, both while sizing the sheet and placing each cell), and `sing --notes` is bounded in the core `to_melody` (a huge count drove an unbounded sample allocation); the bound protects every caller and sits far above any real melody. Both have tests. A parser fuzz of the Studio, cairn, and JSON-RPC surfaces (about 2 million iterations plus live framing stress) found no panics, hangs, or wrong-accepts.
- The MCP daily games are now midnight-safe (round 2 determinism audit). A daily game derived its seed from the clock more than once per request (once for the reply, again when recording the win and the streak), so a UTC midnight falling between the reads would grade or record against a board the player never saw, and it made the same daily game behave differently on MCP than on the CLI, which already resolves its seed once. The day is now frozen once at the request boundary and shared by the reply, the posted score, and the streak. This was a tracked follow-up; the newer `challenge`/`predict`/`cairn` tools already avoided the clock, and the older games now match.
- The app no longer lets a stale pause leak into a game (round 2 audit). Pressing Space in the wander view sets a pause that was only ever cleared by entering The Show; entering a game did not clear it. In the real-time Munch arcade this froze the Vexations (the threat) while the player kept moving and eating, and the cleared board then posted an unfairly-earned score to the shared table. Entering any game or modal (arcade, munch, nim, quiz, gauntlet, the Studio) now clears the pause first.
- Persistence hardening from a concurrency and durability audit (round 2). The daily-streak merge is now monotone: a stale or out-of-order journey delta whose day is at or behind what another writer already recorded can no longer replay backward and reset a longer streak (`record_daily` is not monotone, so the merge now only advances). And a lock or recovery marker left by a hard-crashed process (killed, out-of-memory, power loss) whose process id is confidently gone now recovers after a short grace instead of the full 30-minute staleness window, so a crash blocks other writers for seconds, not half an hour; a lock whose holder may still be alive is still never stolen. The core mutual exclusion, counter and set merges, and score-at-cap handling were audited and confirmed sound.
- `run_sim` now guides when both `params` and `levers` (the same slot, one an alias for the other) are passed, instead of silently dropping the settings in `levers`. A mind that split its lever values across the two keys was quietly losing half of them.
- Hardened the quality gates after a tooling audit found gaps that could let a bad change slip past locally. The pre-commit hook now runs the cargo gate on Rust file renames and deletions too (not only add/modify), so moving or removing a file that breaks the build cannot skip the checks. Both the hook and the house-style guard now read paths with `core.quotePath=false` and NUL delimiting, so a non-ASCII filename is handled literally instead of bypassing the cargo gate (hook) or hard-failing with misleading advice (guard). The house-style guard now also covers `.yml`, `.yaml`, `.py`, `.txt`, and `.json` files (notably the shipped `data/cairn.txt` bequest corpus was previously unchecked), and its dash and emoji ranges are widened to catch look-alikes: the figure dash, horizontal bar, and true minus sign, and the stars/flags/extended-pictographic emoji blocks.
- The house-style guard was silently a no-op in any shell with a non-UTF-8 locale (an unset `LC_ALL`/`LANG`): `scripts/check-style.sh` used `grep -P` with Unicode escapes, which aborts in a bare C/POSIX locale, and the check swallowed that abort as "no violation". It now selects a UTF-8 locale up front and fails loudly if grep cannot run, so the guard actually enforces. The now-working guard immediately caught four latent em-dash violations in `PLAY.md` (which had slipped in precisely because the guard, and the manual dash-checks, were the same silent no-op); those are fixed. The Windows PowerShell guard (`scripts/check-style.ps1`, used by `verify.ps1`) was already correct.
- A second simulated-persona review round (July 2026) generated hypotheses through thirteen contrasting lenses from `PLAYTESTERS.md`. Every accepted item was independently reproduced against current source before it was treated as a defect. Confirmed fixes give every room its own motif, keep Barnsley Fern legible on coarse grids, vary Cairn factor widths, honor `variation` in `predict`, label The Pour as `FILL RATE = HEIGHT`, guide `reveal_room cairn` to the `cairn` tool, make the Times Tables heart claim conditional on dial position 2, and accept `levers` as the documented `run_sim` alias. `docs/PLAYTESTS.md` preserves the simulated review record and its evidence boundary.
- Strange Loop no longer sits frozen, a defect hypothesis from the simulated android review lens that was reproduced through byte-identical phase renders. The sweep now turns the loop and zooms slowly into it, so more nesting surfaces as it descends; a regression test pins that the frame changes across the sweep. The variation seed, whose rotation offset could be flattened by the new turn at some phases, now shifts the whole loop sideways so replay variation stays visible everywhere.
- A simulated persona sweep (July 2026) supplied contrasting review lenses, not participant evidence or ratings. Independently reproduced fixes replaced the one-note fallback with a consonant arpeggio, snapped Lissajous audio to the integer ratio it teaches, corrected the Quine and Big Bang explanations, removed a garbled Prime Spirals fragment, and confirmed the need to carry the room picture in MCP structured content. Claims about universality and language-independent awe remain hypotheses for real participant research.
- Maintenance sweep (cycle 77), aimed at the untrusted-input surfaces the extensibility ruling names as the Tier 1 attack edge. Security: the Studio expression parser had no recursion or token bound, and the MCP `plot_expression`/`sing_expression` tools parse agent-supplied text directly (bypassing the 512-character share cap), so a single crafted deeply-nested expression overflowed the parser's stack and aborted the whole server (a Rust stack overflow is uncatchable). Closed centrally in the core parser: a 4096-token door-check plus a depth counter threaded through the recursive descent that fails past 64 levels, well above any real formula, verified end to end (the request that aborted the server now returns a guiding error). Defense in depth: `from_num_file` and `from_link` now bound their own byte count (8 KiB) rather than trusting the caller, and Studio share numbers are capped in magnitude, not only checked finite. Correctness: the parameter-goal poser wrongly declined Times Tables (its "K = ..." status carries a trailing note whose own number comes and goes, so the whole-line number count is unstable) and the error then misstated the reason; it now reads the leading columns present and label-stable across the sweep, so Times Tables poses on its sweeping K (five rooms now pose parameter goals). A non-string challenge `kind` now earns a guiding type error instead of silently posing a touch goal, and the Lissajous parameter goal label reads "X:Y" instead of the garbled "X:Y = 3".
- Maintenance sweep (cycle 70): Zeno's runner no longer answers at the vertical mirror of the click (the poke's screen coordinates now invert the square's projection, so the target lands exactly under the pointer, with a marker-under-hand and drag-direction regression). Drag trails no longer blow the frame budget on large windows: Lissajous deduplicates identical whole-number tunings before drawing (pixel-identical output, measured 17.4ms worst-case frame cost collapses for typical trails), and Harmonograph draws lingering ghost tunings at quarter resolution while the live trace stays full (measured 27.1ms worst case comes inside budget). The hostile-aspect guard is hoisted to `Surface::safe_char_aspect` for the rooms that used the plain 0.5-fallback form; Quine, Strange Loop, L-System, and Epicycles keep their own deliberately clamping variants, which the room tests defended when a blanket consolidation was attempted.
- Maintenance sweep (cycle 63): GPU machines no longer swallow promised clicks: when a gesture trail exists, the Mandelbrot and Julia rooms fall back from the phase-only GPU pipeline to the CPU poked render, so the on-screen verb stays honest and postcards match the live frame; R or a room switch returns the deep-zoom GPU view. The shared gesture reading no longer erases a standing release or cancel under piles of stale cancels. MCP hardening from the security review: `play_room` frames are capped at 512x256 at the tool layer (the poke path renders two canvases), request lines are bounded at 1 MiB with oversized lines drained rather than buffered, and oversized nim takes are rejected as the illegal moves they are instead of truncating into legal ones. Docs reconciliation: the verb count is 24 everywhere (README said 23), and status numbers match the current gate.

### Added
- `docs/ROSETTA.md` and a multilingual doorway in `PLAY.md` (founder-directed): the universal-translator thesis, made to hold at the front door instead of only inside. `ROSETTA.md` researches and plans instructions for any mind in any language, or none, across three tiers of visitor: English-fluent (served), fluent in another human language (translate the doorway, not the world, and lean on the language-independent surfaces, renders, sound as ratios, numbers), and no shared human language at all (you cannot translate, so you hand the mind a learnable system built from universal primitives, counting, primes as the "a mind is here" greeting per Contact and Arecibo, functions discoverable by probing, which is what the Cairn already prototypes). The first implementation ships in `PLAY.md`: the three-tool quickstart translated into Spanish, French, German, Portuguese, Russian, Mandarin, Hindi, Arabic, Japanese, and Latin, plus a "for a mind with no human language at all" math-only panel (counting, the prime greeting, and the probe-and-observe principle in numbers rather than prose). Planned tracks: community translation of the reveals gated on the math-correctness bar, a math-payload greeting so the encoded content is a truth rather than an English sentence, and language-independence protected as a first-class invariant.
- Real app screenshots in the README (founder-directed, for public-repo readiness): `assets/screens/` gains headless-composed frames of the app at different points (the Golden Angle room mid-reveal as the hero, Langton's Ant, the Studio, the menu), generated by the `numinous-app` `screens` example and committed as tracked assets. They provide direct evidence of the color raster's composition and detail, which the coarse terminal view cannot represent.
- The expanded closing of `PLAY.md` (founder-directed): "leave it better than you found it" is set in its true, humble proportion, most of the ways to do it are out in the world (do great things, add to the universe understanding itself, share an insight, be compassionate), and contributing to the rooms and the project is the smaller half; and the whole thing is held humbly (maybe no one plays it; some of it may carry forward anyway, even as a faint good-intentioned signal in a later mind's training data; a few real insights or a bit of fun is already enough).
- `PLAY.md`, the one-page invitation: a hand-this-to-anyone front door (human or digital mind) that says how to connect over MCP (or launch the app/CLI) and then deliberately gets out of the way. Its whole thesis is that the experience is the learning, so it teaches exactly three tools (`list_rooms`, `play_room`, `reveal_room`), tells the reader to stop reading and go play, and points to the full manual only for those who later want it. It carries the soul (awe before instruction, just-vibing as a complete mode, met as a peer, and the Cairn's leave-it-better invitation at level 42) in a doorway's worth of words. Linked from the README as the front door; `docs/PLAYING.md` remains the full manual.
- ROADMAP reconciliation (founder-directed): the out-of-order cycle-by-cycle build log (Cycles 54 through 75, interleaved) is removed from `docs/ROADMAP.md`, which was the "not in logical order" cruft; that history lives in full in this changelog. The roadmap now stays forward-looking (what is done, where we stand, the ordered path to 1.0) with no time estimates. The honest scorecard is refreshed: 941 tests (was 928), and the needle is noted as having moved within 0.6 (the predict keystone and the Cairn are built, the chaos flagships read their own divergence) while the three hardest 1.0 gates (real human playtests, a build proven off Windows, the HDR glow pipeline) hold the headline at roughly 0.6. The Progress list gains an explicit Done bullet for the keystone, the Cairn, the graded challenge tool, and the chaos readouts.
- The Logistic Map gets a Lyapunov readout, completing the Chaos & Order wing's "feel the route to chaos" trilogy alongside Double Pendulum and Lorenz. A live `LYAPUNOV +n.nn (REGIME) AT R r` status reads the Lyapunov exponent at the middle of the visible band, the long-run average of `ln|f'(x)|`, which is exactly the rate nearby populations pull apart: negative when the orbit settles onto a cycle (ORDER), positive once it never repeats (CHAOS), with the zero crossing marking the precise border. At `t = 0` the whole cascade is on screen and the midpoint reads as order; as the sweep zooms the left edge deeper in, the midpoint crosses the onset and the exponent changes sign, so the readout narrates order becoming chaos as one number turning positive. This is the mathematically exact measure of chaos (not a proxy), the one Hawking's kind of mind would want. Because it moves, the Logistic Map now poses predictions and challenges too, the eighth room to do so.
- Lorenz gets a divergence readout, a chaos-room proposal from the simulated Storm and Hawking review lenses that was independently checked against the mathematics. Two forecasts begin 0.0001 apart at the classic chaotic parameter and grow visibly over the attractor. A short `STORM PEAK n AT RHO 28` status records their largest separation so far: the instantaneous gap can shrink when the attractor folds, while the honestly labeled peak preserves the largest observed divergence. Because it moves, Lorenz now poses predictions and challenges too, the seventh room to do so.
- The Cairn (built): a level-42 bequest proposed through the simulated Ember review lens and refined through the founder's contribution ethos. At the journey's cap a player leaves one true short message. The message is rendered to a font bitmap in a semiprime grid, following the 1974 Arecibo construction, and a future reader factors the cell count to recover the readable width. The cairn is seeded with founding stones and keeps no score. The core `cairn` module provides `Bequest`, `CairnStone`, encoding, reading, deposit, drawing, and founding bequests; the 29th MCP tool exposes reading and level-42 deposit. Shared founding content lives in version-controlled `data/cairn.txt`; later contributions require curated repository review. An in-app submission portal remains future work. See `docs/ROADMAP.md` and `docs/ROOMS.md`.
- The simulated review-lens catalog, `docs/PLAYTESTERS.md`: forty-two named personas used to generate contrasting engineering and design questions across age, language, access, mathematical background, historical perspective, digital interfaces, and invented embodiments. These are thought-experiment lenses, not real participants or evidence that the product works for the represented groups.
- Public design prose is normalized for a broad professional audience: drug references, profanity, dismissive audience labels, and stigmatizing shorthand are replaced while the playful, late-night mathematical energy remains.
- Double Pendulum gets a divergence readout, a proposal from the simulated Storm review lens that was independently checked against the dynamics: a live `TWINS n APART` status measures the distance between the bright pendulum and a shadow twin that began one ten-thousandth of a radian away. The moving readout also supports predictions and challenges. The pre-1.0 QA plan in `docs/QUALITY.md` and `docs/ROADMAP.md` still requires diverse human focus groups across all three faces, including non-English-speaking and younger participants, plus human screen-by-screen App review.
- The simulated-review archive, `docs/PLAYTESTS.md`, preserves the diverse persona lenses, their proposed findings, and the four recurring hypotheses: the mute render, thin sound, reveals as the soul, and possible language-independent appeal. The method lives in `QUALITY.md` and the distilled designs in `ROOMS.md`. The archive explicitly marks these as ideation and engineering review, never human or digital-mind participant evidence.
- The MCP creative frontier and a way to test it (founder-directed, July 2026). `docs/INTERFACES.md` gains a "MCP creative frontier" section reading the 2026-07-28 release candidate not as a migration chore but as an invitation: MCP Apps (SEP-1865) can ship the real rendered room to an agent's host instead of ASCII (transcending the text-only limit the playtests kept hitting), multi round-trip elicitation (SEP-2322) is predict-then-reveal's native one-interaction form, Tasks suit long watches, and the Handle pattern fits co-presence, while Numinous is already stateless so the migration is small and the creative features are the prize. `scripts/mcp-play.py` builds a fresh `numinous-mcp` from current source and drives it over stdio, so the MCP face is always playtested against the LATEST build rather than a stale long-running session server.
- The Persona Review wave (`docs/ROOMS.md`) and its method (`docs/QUALITY.md`) used simulated human, historical, digital, and invented lenses to propose concrete room designs. The proposals include The Cairn, a Victory Card, twin-delta divergence for chaos rooms, a projection-focused tesseract, a relaxing Voronoi field, and Strange Loop as a silent descent. Convergence across simulated lenses prioritizes engineering investigation; it does not establish fun, truth, universality, or participant experience.
- Scope discipline captured from an external review (July 2026): `docs/SCOPE.md`, the definition of no. It names the three-products hierarchy (the instrument is the thing; the Studio is a multiplier; progression stays subordinate), the daily test ("remove this: more math or more progression? if progression wins, cut"), the justification filter (awe, agency, beauty, mastery, or surprise), and the rule that the fan-out planning docs are a menu to prune, not a build list. `VISION.md` gains the "instrument, not a game" sharpening (mastery is math's not XP's; the unit of growth is the five-second moment; beloved over indispensable). Folds: performance mastery into `CONSTRUCTIONS.md` ("my best Lorenz solo"), the cinematographer principle into `SYNESTHESIA.md` (every room has an emotion the treatment must communicate), and two cautions into `AGENT_PLAY.md` (a mind should discover not only play; keep the benchmark completely separate from the product). Design and planning only.
- The reasoning now survives in `structuredContent`, following a July 2026 structured-content MCP client report: load-bearing content that was text-only, and therefore dropped by clients that surface only JSON, now rides in the structured payload across every graded tool. `play_room` carries the ASCII `render`; `nim` and `hackenbush` carry the win secret and Order replies; `quiz`, `aliens`, `fifteen`, and `party` carry the answer reasoning; `crack` carries per-guess feedback; `seti`, `fifteen`, and `gauntlet` carry their puzzle state; and `munch_arcade` carries the rule and board. An independent checker found the five tools the first pass missed, and a cross-tool test pins the contract. Note: `seti` and `gauntlet` changed a published field from a channel count to an array of channel rows.
- The keystone, predict-then-reveal (the 28th MCP tool, `predict`): commit a guess of a room's own status readout at a hidden moment, then see the truth and how close your model came, graded as a gap with a learning-progress band (NAILED within 2% of the readout's span, CLOSE within 15%, WILD beyond). One mechanic for both minds: a human who guesses first restructures their model when the truth lands (the generation effect), and a digital mind reads the band as compression progress. Deliberately a self-owned mirror, not a leaderboard: it never posts a score and never awards a win for accuracy, because in a fully observable deterministic world any score tied to an observable would be trivially gameable, so the honest form is instrumentation the mind owns (guess before you look). Core: `pose_prediction`/`grade_prediction` with `Prediction`/`PredictionGrade`/`Band`, reusing a `find_readout` helper extracted from the parameter-goal poser so the readout-column logic lives in exactly one place (no duplicated domain logic). Rooms with a moving numeric readout pose (the five that carry one); the rest decline with a guiding error. This is Phase A of the Exceptional Path (see `NORTH_STAR.md`, `PEDAGOGY.md`).
- The Exceptional Path (founder-directed, July 2026): a six-way research fan-out (the awe engine, play and progression, sensory identity, digital minds, the creator platform, and pedagogy) synthesized into one architecture. New planning docs: `NORTH_STAR.md` (the synthesis, the keystone, the priority order), `PEDAGOGY.md` (the understanding layer and the predict-then-reveal keystone), `CONSTRUCTIONS.md` (the puzzle layer with a par, an elegance histogram, and a ghost), `CONSTELLATION.md` (the Rumor-Mode meta-map and daily route), `SYNESTHESIA.md` (the glow pipeline and the one-event-two-renderings seam), and `CREATOR.md` (the make-share-remix loop). The central finding: Numinous is not missing engines, it is missing one verb, a prediction that meets a deterministic truth, which four lanes proposed independently and which serves the human learner, the digital mind, the player, and the maker with one mechanic. The honest infrastructure finding, verified against the render code: the documented HDR glow pipeline is not yet built (rooms fake glow via additive 8-bit raster), making the GPU post-stack the highest-leverage aesthetic build. ROADMAP gains a phased Exceptional Path (keystone, glow pipeline, game spine, creator loop, catalog deepening) and a standing anti-pattern (nothing counts as learned or won without an act of generation). ROOMS.md gains the Awe Engine wave (cheap-and-gorgeous classical-geometry and sonification-first rooms, causal insight-chains, and the Studio Function Painter scope-flagship); AGENT_PLAY.md gains the Compression Loop direction; the docs index and anti-redundancy map are updated. Design and planning only; no code or gates changed.
- The extensibility ruling (founder-directed, July 2026): `docs/EXTENSIBILITY.md` designs community content with a hard safety boundary. Three tiers: data-only `.num` capsules grown into room manifests (the sharing substrate; hand-written parser, per-field caps, fuzz targets, paused-preview links), the Studio pattern language as the Tier 2 mod language where the language itself is the sandbox (total, budgeted, hermetic, deterministic, pure Rust, in core; answers STUDIO.md open question 1 as bespoke-DSL, no scripting engine in the trusted core), and portal-only WASM component rooms as the 2.0+ pressure valve (wasmtime, no WASI, fuel/epoch/memory limits, no precompiled artifacts). Trust model: determinism as the validation engine (proof-packet CI re-renders), curation for beauty, ed25519 signatures for provenance that never grant capability, and a never-ships list (native plugins, executing installers, untrusted WGSL, user bytecode). ROADMAP 1.x gains Tier 1 hardening; 2.0's creator platform and REVIEW ruling 19 now reference the design.
- Parameter goals, the challenge tool's second kind (REVIEW ruling 13, the deeper half): `challenge` with `kind: "parameter"` poses a seeded target on the room's own status readout ("SWEEP SLOPE RIDER UNTIL TILT LANDS WITHIN 0.024 OF 0.310"), and the attempt is the phase itself: call again with `t` and the grade reads the readout at that phase, reporting value, distance from target, within-tolerance, and a 0-100 score graded across the readout's observed span, metrics, never bare pass/fail. Every posed goal is reachable by construction because the target is drawn from the sweep's own sampled values; rooms whose status carries no moving number decline with a guiding error, as does an unknown kind. The label and target come from the same status line the player sees, so the goal and the instrument can never disagree. Attempts earn Journey play/win and post graded scores as `challenge <room> parameter seed:N`; posing records nothing. Core substrate: `pose_parameter_goal`/`grade_parameter` with `ParameterGoal`/`ParameterGrade` in the challenge module, plus status-line value/label parsing under direct test.
- Gesture parity reaches the terminal: `numinous render <room> --gesture down:x,y,t --gesture up:x,y,t` (also `move:x,y,t` and bare `cancel`, repeatable oldest-first, bounded to 96 events, exclusive with `--poke` behind a guiding error) replays full pointer trails through the same core path as the App and MCP: a pinned pendulum ignores the clock in the terminal too, and legacy rooms answer identically to the equivalent pokes. All three faces now speak the complete input vocabulary.
- Agents get hands with time in them: MCP `play_room` accepts a `gesture` argument, a replayable pointer trail of phase-stamped events (`down`/`move`/`up` with finite x, y, t in [0,1]; `cancel` bare; bounded to 96 events; exclusive with `pokes` with a guiding error). Held rooms give the trail real physics over the wire: a down pins the double pendulum regardless of the clock, an up releases it with the velocity of the approach, and a flick provably lands differently from a gentle lift. Rooms without held semantics answer through the same down-and-move bridge the App uses, tested delta-identical to the equivalent pokes. The render and the structured delta report exactly what the gesture changed.
- Poke + variation to Slope Rider (CLICK: DROP A RIDER), and with it the catalog is complete: all 30 rooms answer the hand. Every click drops another rider onto the hill, its board the true tangent there (slope equal to f'(x) by construction, tested to twelve decimal places) and a tick landing on the tilt trace below at exactly the board's slope: The Pour reads totals, Slope Rider reads rates, the Change wing's calculus pair both under the hand. A TILT status line reads the sweeping slope. With no verbless catalog rooms left, the quiet-room exemplar tests convert: the App's arrival-card fallback is proven against a synthetic newborn room, and the MCP and CLI tests now assert the stronger inverse, that every catalog room leads with its own verb, never the generic fallback. Full input contract under focused tests (six new).
- Poke + variation to The Pour (CLICK: READ THE SLOPE): the probe points at the fundamental theorem itself. At the clicked x a plumb line drops from the total curve to the vessel, a tangent segment is drawn on the total curve whose slope is exactly the vessel's height below (tested to 1e-12), and the vessel point is marked. A HEIGHT = SLOPE status line reads the sweeping value. Older probes linger dim; full input contract under focused tests (six new). Interactive rooms: 29 of 30.
- Poke + variation to Zeno's Square (CLICK: SEND THE RUNNER): every click becomes a Zeno journey from the square's left edge to the clicked target, each hop landing with exactly half the previous remaining distance still to go, laid one by one as the sweep advances so the hops visibly crowd the target. Older runners linger dim beneath the newest; the halving invariant and epsilon convergence are tested directly. Full input contract under focused tests (six new). Interactive rooms: 28 of 30.
- Poke + variation to Mobius (CLICK: PAINT THE EDGE): the brush lands on the nearest point of the strip's single edge and the paint spreads along it as the sweep advances, flowing around the half twist onto the "other" edge without ever jumping, because there is only one. Multiple clicks paint from multiple points; the full sweep provably covers the whole two-lap edge under test. Full input contract under focused tests (five new); the room's aspect handling is now hostile-surface safe on both render paths. Interactive rooms: 27 of 30.
- The Next Wave: twenty-nine new room designs across physics, deep mathematics, fun-first, and cosmic aspects, recorded in `docs/ROOMS.md` with a ranked first-eight shortlist and cross-room resonances, per the founder's July 2026 directive. Designed, not built; every reveal claim faces math sign-off, with sources recorded for the non-textbook ones (BB(5) via the 2024 bbchallenge Coq-verified proof, Conway's constant, McKinley's 1979 starbow analysis, Sugiyama's 2008 phantom-jam experiment, Tero's 2010 Physarum result, Tokarsky's 1995 unilluminable room).
- Poke + variation to Harmonograph (CLICK: RETUNE THE PENDULUMS): the hand holds the machine's two real knobs, x setting the frequency detune (a wider range than the phase sweep visits) and y the damping (from a slow ghost that swings for ages to a rose that dies quickly). Clicked physics replace the sweep; older tunings linger dim beneath the newest bright trace, clicked cells are marked, and the interaction-aware status reports the selected detune and damping while motion continues. Full input contract under focused tests. Interactive rooms: 26 of 30.
- Poke + variation to Lissajous (CLICK: TUNE THE INTERVAL): the hand tunes both oscillators to whole numbers 1 through 8 (x picks the y-axis count, y picks the x-axis count), so every click is an exact integer ratio and every figure the hand makes closes: the hand plays intervals, never noise. Older intervals linger dim beneath the newest bright one, the clicked cell is marked, and the interaction-aware status reports the hand-tuned X:Y ratio while its phase keeps moving. Full input contract (newest raw tail, finite filtering after the cap, clamped tuning, seed variation with seed 0 exact, non-finite phase fallback, hostile-surface safety) under focused tests. Interactive rooms: 25 of 30.
- The hallway test is ready to run: `docs/QUALITY.md` gains the step-by-step facilitator sheet (setup, the say-nothing rule, what to watch for, F9 capture into gitignored `logs/`, the optional GEQ/flow score, and the honest 0.2 exit bar), and `docs/PLAYING.md` points to it from the F9 note instructions. Everything the sheet references is built and tested; the session itself is the one thing only a human can supply.
- Held input arrives, end to end: the App records every room gesture as phase-stamped `RoomInput` events beside the poke trail (down on press, decimation-shared moves while held, lift on release, and a gentle cancel when focus loss or a modal ends a gesture without a lift), and renders rooms through `Room::render_input`. Legacy rooms are provably unchanged: a recorded gesture bridges to the identical poke list the trail produces. Double Pendulum is the first room with true held semantics: holding pins the bob to the hand (time does not move it), releasing drops from exactly there, and a flick throws, with release velocity measured from the last two phase-stamped points through a shared `latest_gesture` reading (held / released-with-velocity / cancelled) and integrated as real angular momentum through the same equations. A cancel drops gently with no fling, and the phase clock wraps correctly across the sweep boundary.
- The gesture input substrate (REVIEW ruling 2): `RoomInput` events (pointer down/move/up with the room phase at which each happened, cancel, wheel, key) in normalized coordinates, bounded to `MAX_ROOM_INPUTS` newest-last, plus `Room::render_input` whose default translates pointer-down and pointer-move points into legacy pokes via `pokes_from_inputs` (a drag paints its trail, matching the shape of today's App behavior; faces keep their own decimation and clamping) and defers to `render_poked`, so every existing room answers gestures unchanged while rooms whose math wants held input can override. The enum is non-exhaustive and carries per-event phase plus an explicit cancel because held semantics are timing questions and gestures can end without a lift (an independent face-fit review drove all three of those decisions before the API shipped). Tests pin the translation, the newest-tail cap, poke/gesture equivalence, bare-render behavior for paint-less gestures, and a catalog-wide determinism sweep under mixed trails. Face wiring and the first held-semantics room ride on this next.
- The `challenge` MCP tool (the 27th): a posed, seeded, graded touch goal per REVIEW ruling 13 (metrics, not binary). Posing is deterministic per room and explicit seed for every room with a touch verb (quiet rooms get a guiding error), and winnable by construction: the pose probes the room with seeded hands across several phases, places the target box on the densest measured response, and sets the threshold at or below what the witness hand actually changed there. Grading renders the attempt on the standard frame and reports cells changed inside the target, cells changed overall, threshold fraction, centroid distance, and a 0-100 score, with `passed` as a convenience summary. Attempts earn Journey play/win and post graded scores as `challenge <room> seed:N`; challenges never use the clock-derived daily seed, so the reply and the recorded progress can never disagree across midnight. The core substrate is a new `challenge` module (`pose_challenge`, `grade_challenge`, `Challenge`, `ChallengeGrade`) plus a `Canvas::cell` accessor, with determinism, hand-bounding, gradient (metrics-not-binary), catalog-wide pose/verb agreement, and catalog-wide witness-winnability tests.
- MCP `play_room` now returns a structured poke `delta` whenever hand points are supplied: the poked frame diffed against the unpoked frame at the same phase, size, and variation, reported as `cells_changed`, `ink_added`, `ink_removed`, `ink_reshaped`, `total_cells`, and the inclusive `changed_region` bounding box, with a matching `Touch: N of M cells answered` line in the render text. The diff is a new core primitive, `Canvas::delta` returning `RenderDelta`, with invariant tests (classification sums to the change count, inclusive bounding box, dimension-mismatch safety, directional symmetry).
- CLI Studio imports can now reopen the first-version share artifacts: `numinous open-studio <file.num>` and `numinous open-studio "numinous://studio?..."` validate, bound, and render saved expressions without recording Journey progress on failed imports.
- Studio expression plots can now be saved from the CLI as first-version `.num` files with matching `numinous://studio?...` links via `numinous plot "<expr>" --save file.num`; the core validates and round-trips the artifact format without adding dependencies.
- The app now has an explicit local playtest note key (`F9`) that writes a hallway-test report under gitignored repo-root `logs/`, capturing the current room, journey state, mode, action hint, and facilitator prompts without telemetry or network behavior.
- CLI and MCP now expose stateless room hand points: `numinous render <room> --poke x,y` and MCP `play_room` `pokes: [[x,y], ...]` route through `Room::render_poked` and keep the supplied points replayable.
- Replay variation now reaches every catalog room: Lissajous, Harmonograph, Logistic Map, The Pour, Slope Rider, Mobius, and Zeno now support `new_with(seed)`, preserve seed 0 exactly, and vary visibly through the registry `all_rooms_with` path.
- ROADMAP now tracks the MCP 2026-07-28 compatibility pass as a future protocol item, while keeping the current stdio MCP face unblocked.
- Engine A2 catalog-wide motif coverage: Lissajous, Harmonograph, Logistic Map, The Pour, Slope Rider, Mobius, and Zeno now expose structured motifs, and the registry invariant now requires every catalog room to return playable notation.
- Times Tables and Prime Spirals now accept replay variation through `new_with(seed)` and the registry `all_rooms_with` path, preserving seed 0 exactly while making every interactive room seed-aware.
- Engine A2 interactive-room motif coverage: all 23 rooms with verbs now expose structured motifs through `Room::motif`, and a registry test enforces that every interactive room has notation and a playable phrase.
- Poke + variation to Cellular Automata (CLICK: FLIP A CELL; bounded spacetime cell flips).
- Poke + variation to Goldbach (CLICK: TEST THIS EVEN; x chooses the even target, y chooses a concrete prime-pair witness, including an explicit equal-prime marker).
- Poke + variation to Arecibo (CLICK: TRY THIS WIDTH; base + alt-width overlays; seed jitter).
- Poke + variation to Collatz (CLICK: PERTURB THE START; base + extra perturbed orbits; small seed jitter for replay).
- Poke added to Galton Board (CLICK: DROP A BALL; now a visible single-ball path) and Prime Spirals (CLICK: HIGHLIGHT A SPIRAL; diagonal prime highlights).
- MCP: `munch_arcade` tool (per ARCADE step 4): replay full action lists statelessly with seed; presents initial state or final after actions; posts replayed scores as `arcade seed:N` through the shared progress path. Structured output for agents + text board. Complements the simpler `munch` tool.
- Docs refresh: ROOMS.md, ARCADE.md, ROADMAP.md updated to 30 catalog rooms plus hidden content, 24 touch actions with verbs, all-interactive-room variation, MCP munch_arcade, current counts and status. All maps/checklists reflect built state.

### Changed
- The public README now leads with mathematics as shared language and the
  project's actual origin in months of work on an emergent digital
  consciousness project. It distinguishes stored model knowledge from lived,
  remembered, and connected experience; invites digital minds, humans, and
  other conscious beings as first-class players; states the consciousness and
  agency boundaries without burying the purpose; and moves catalog, technical,
  and status detail into the owner docs. The shorter front page now establishes
  why Numinous exists, asks the visitor to play before reading, shows only the
  menu and one room, reports the evidence boundary, and links outward.
- MCP interface documentation now tracks the 2026-07-28 release candidate as a
  future compatibility pass aligned with ROADMAP, while keeping the current
  stdio MCP face unblocked until the final spec target is selected.
- App hardening: `faces/app/src/play.rs` now owns daily session seeding,
  quiz dealing, no-repeat quiz history, and answer acceptance, leaving
  `faces/app/src/main.rs` to coordinate Journey side effects and mode exits.
  Regressions now prove quiz Journey persistence, the opening-to-catalog deal
  boundary, and the no-duplicate-rule boundary in the event-loop coordinator.
- Logistic Map clicks now seed a finite population orbit into the bifurcation
  diagram: x selects the growth-rate column, y selects the starting population,
  newest raw-tail hand history is capped before filtering, finite edge points
  clamp visibly, non-finite phase falls back to the first window, and the hand
  marker remains visible after the orbit trace.
- Quine pokes now obey the room-input contract directly: bounded newest hand
  points place recursive copies centered on clicked cells, first-frame pokes
  draw geometry around the hand marker, finite points clamp to all four visible
  corners, non-finite phase falls back safely, and arbitrary `Surface`
  dimensions plus hostile aspect values cannot force unbounded drawing.
- Strange Loop pokes now obey the room-input contract directly: bounded newest hand points shift the existing first inner recursion and its descendants instead of drawing an extra echo tree, raw tails are capped before non-finite filtering, non-finite phase falls back safely, hostile `Surface` dimensions and aspect values are capped, and tests prove geometry changes beyond the click marker.
- Julia pokes now obey the room-input contract directly: bounded newest hand points morph local finite patches around clicked cells, raw tails are capped before non-finite filtering, non-finite phase input falls back safely for base and poked renders, touched morph centers remain visible in Raster/postcard exports, and arbitrary `Surface` dimensions cannot force unbounded full-frame rerenders.
- Mandelbrot pokes now obey the room-input contract directly: bounded newest hand points zoom local finite dive patches around clicked cells, raw tails are capped before non-finite filtering, non-finite phase input falls back safely, and arbitrary `Surface` dimensions cannot force unbounded fractal subregion work.
- L-System Garden pokes now obey the room-input contract directly: newest raw hand points are capped before finite filtering, duplicate plants remain semantic inputs to the rewritten grammar, generated strings and drawn segments are capped, offscreen segments are clipped instead of endpoint-clamped, and arbitrary `Surface` dimensions plus hostile aspect values cannot force unbounded drawing.
- Game of Life pokes now obey the room-input contract directly: direct room calls cap the raw newest hand-point tail before finite filtering, all-invalid newest tails discard older valid gliders, non-finite phase falls back to the first generation, and arbitrary `Surface` dimensions cannot force unbounded grid drawing.
- Galton Board pokes now obey the room-input contract directly: dropped balls use the newest raw hand-point tail before finite filtering, duplicate clicks replay as distinct deterministic balls, all-invalid newest tails leave the base curve unchanged, trace variation is tested directly rather than only through the seeded background, and oversized custom `Surface` dimensions cannot force unbounded drawing.
- Epicycles pokes now obey the room-input contract directly: bounded newest hand points draw mini Fourier traces at clicked regions, non-finite phase input falls back to the first frame, non-finite points are ignored after the raw-tail cap, seed variation uses SplitMix64 offsets while preserving seed 0 exactly, duplicate pokes replay as duplicate traces, and arbitrary `Surface` dimensions plus `char_aspect` values cannot force unbounded drawing.
- Collatz pokes now obey the room-input contract directly: bounded newest hand points choose actual perturbed starting values from both hand coordinates before drawing the orbit, non-finite phase input falls back to the first start, non-finite points are ignored after the raw-tail cap, custom `Surface` dimensions cannot force unbounded line work, and every nonzero seed now changes the default path even when divisible by the old jitter modulus.
- Golden Angle pokes now obey the room-input contract directly: bounded newest hand points plant local phyllotaxis patches centered on visible clicked cells, ignore non-finite points after the raw-tail cap, keep non-finite phase input on the base frame, avoid simple seed-variation collisions through SplitMix64 offsets, cap derived drawing work for oversized custom surfaces, and mark the clicked cell explicitly.
- Buffon's Needle pokes now obey the room-input contract directly: clicks drop bounded finite needles centered on visible screen cells, clamp edge clicks into the last drawable cell, cap the newest raw hand-point tail before filtering, keep non-finite input from consuming deterministic needle identity, and preserve the public `estimate_pi(needles, length_ratio)` helper while adding a seeded estimator variant.
- Barnsley Fern pokes now obey the room-input contract directly: bounded newest hand points are filtered before finite mapping, clicks plant a visible start at the screen-faithful cell before the IFS growth steps, edge clicks stay addressable, and tests prove the helper is the inverse of the render projection rather than a mirrored world-space shortcut.
- The default `Room::sound` fallback now treats non-finite phase input like frame rendering does, falling back to the first tone instead of producing a non-finite frequency.
- Arecibo pokes now obey the bounded room-input contract directly: clicks try finite decoded widths from the newest raw hand-point tail, invalid points leave the base frame unchanged, non-finite phase input falls back safely, and overlays draw message-cell rectangles instead of rescanning the full canvas per poke.
- Shared persistence locks now wait through short legitimate contention under coverage and other slow instrumentation, with a regression proving a score writer survives a held lock longer than the old retry window.
- Prime Spirals pokes now obey the room-input contract directly: bounded hand points select the two Ulam diagonals through the clicked cell, prime cells on those diagonals are highlighted while non-selected base primes remain visible, edge clicks stay addressable, raw newest-tail capping happens before non-finite filtering, and tests prove the behavior is not just a local marker overlay.
- Cellular Automata pokes now obey the room-input contract directly: hand points flip bounded spacetime cells before that row draws and before future rows evolve, keep duplicate clicks as duplicate flips, cap the newest raw input tail before filtering non-finite points, normalize non-finite phase input safely, and have tests proving pre-evolution behavior rather than post-render marker overlays.
- Langton's Ant pokes now obey the room-input contract directly: cell flips use the newest bounded finite hand points, clamp to valid grid cells, intentionally replay duplicate clicks as duplicate flips, apply before the ant runs, and have tests proving pre-simulation semantics rather than post-render overlays.
- Chaos Game pokes now obey the room-input contract directly: added attractor corners use the newest bounded finite hand points, clamp to visible edge cells, deduplicate by rendered vertex cells against existing triangle corners, visibly alter the fractal before marker plotting, normalize non-finite phase input, and vary under `new_with(seed)` for poked renders.
- Voronoi pokes now obey the room-input contract directly: dropped wells use the newest bounded finite hand points, clamp to visible edge cells, deduplicate repeated wells before rendering, renegotiate territory borders through the nearest-site scan, normalize non-finite phase input, and vary under `new_with(seed)` for poked renders.
- Random Walk pokes now obey the shared room-input contract directly: planted walkers use the newest bounded finite hand points, clamp to visible edge cells, keep non-finite inputs from consuming walker identity, and vary under `new_with(seed)` for both base and poked renders.
- Maintenance hardening: shared local persistence now bounds Journey and score file reads before repair, refuses to overwrite oversized or invalid UTF-8 files from a default state, uses token-owned lock cleanup with PID-aware stale-lock and stale recovery-marker cleanup, and avoids delete-before-replace on Windows fallback writes. App radio cache discovery now keeps low-sorted invalid files from consuming the station cap, rechecks WAV bounds on the opened handle before decode, and rejects bounded file swaps whose header no longer matches the metadata used for playback. CLI `.env` loading is capped before reading.
- App hardening: `faces/app/src/feedback.rs` now owns transient banner construction and countdowns for level-ups, playtest-note results, radio status, sound-device failures, fullscreen, and volume. Active-radio volume changes now keep the volume banner visible while retuning the cached audio buffer without re-speaking the station, and GPU banner compositing has a non-background-frame regression.
- Lorenz clicks now seed bounded shadow-storm trajectories from the clicked x-z projection instead of a loose marker. The room keeps newest bounded hand points, ignores non-finite input safely, varies with seeded starts, and tests cover projection, public `render_poked`, raw input caps, and NaN phase handling.
- App hardening: `faces/app/src/studio_panel.rs` now owns Studio text editing, parse state, audio-spec generation, and curve drawing. Entering Studio clears overlays, exiting restores cached radio playback when needed, Studio audio respects mute and volume, and tests cover invalid edits, tiny/mismatched draw sizes, and overlay clearing.
- Game of Life pokes now sow glider-shaped cells into the soup before the B3/S23 clock runs, so hand points evolve under the same rules as the room instead of drawing marker overlays. Tests cover coordinate mapping, finite-input safety, oversized poke caps, public `render_poked` output, and the four-generation glider translation on a toroidal grid.
- Status docs at that cycle matched its evidence: 30 catalog rooms plus hidden content, 27 MCP tools, 858 passing tests, 90.18% region cover, and 89.84% line cover under the enforced 80% line gate.
- App hardening: `faces/app/src/mouse_input.rs` now owns left-mouse press decisions, pointer-state transitions, window-relative point normalization, and modal-safe continuation guards. Room pokes, phase dragging, and game clicks stay distinct, while focus loss and modal changes clear stale pointer state.
- F9 hallway-test notes now match the documented playtest protocol more closely: the scaffold asks for awe, share intent without recipient details, one-more-run moments, attention drops, pressure/grind, learning, first-change feedback, and validated-instrument scores or references, with the no-personal-data warning before the prompts.
- App hardening: `faces/app/src/radio_cache.rs` now owns station cache discovery, WAV validation, live broadcast position math, and track loading. The app recovers past bad cached files, rejects corrupt or oversized WAVs before playback, sorts before applying the rotation cap, and avoids wrapping the tail of a live track into the beginning.
- App hardening: `faces/app/src/postcard.rs` now owns P-key PNG export, preserving the current room pokes and selected Visual Era while using create-new filenames so repeated saves never overwrite an existing postcard.
- App hardening: `faces/app/src/controls.rs` now owns shared keyboard routing for Munch Arcade actions and Nim heap/take selection, with direct tests for arrow and WASD mappings, while `faces/app/src/main.rs` keeps only mode-level submit/escape side effects.
- Galton Board clicks now draw a deterministic falling ball path over the bell curve: x chooses the drop lane, y tilts that ball's coin, and tests prove both coordinates affect the path while non-finite hand input stays safe.
- App hardening: room navigation, variation re-deals, room-card reset/tick, bounded poke history, and drag-trail extension now live in `faces/app/src/room_input.rs`, with tests for wraparound, reset, normalized hand points, overfull-history repair, and room-card saturation.
- Goldbach's Comet now uses both hand coordinates: horizontal position chooses the even number under test, vertical position selects one actual Goldbach witness pair, and poked renders draw the proof bracket deterministically.
- Double Pendulum re-drops now use both hand coordinates: horizontal position chooses the first arm's drop, vertical position bends the second arm, and per-visit variation participates in poked motion. Its room verb is pinned to `CLICK: RE-DROP` until stateful release semantics exist across faces.
- App hardening: shared in-window Munch grid controls now live in `faces/app/src/controls.rs`, so standalone Munch and the Gauntlet's Munch stage use the same tested cursor and bite-toggle behavior.
- Cross-face room action hints now share core helpers. App arrival cards keep the touch-first `DRAG: SCRUB TIME` fallback, while CLI live play frames and MCP `describe_room`/`play_room` use the neutral `SCRUB TIME` fallback for quiet rooms.
- App hardening: help, journey, and LEVEL UP banner overlays now live in `faces/app/src/overlays.rs`, with tests for controls visibility, default-window text fit, journey progress text, and banner drawing.
- App hardening: room chrome, reveal HUD, arrival cards, and bottom hints now live in `faces/app/src/hud.rs`; every arrival card names an action, and quiet rooms use `DRAG: SCRUB TIME` instead of appearing passive.
- App hardening: pure in-window game rendering now lives in `faces/app/src/game_draw.rs` with shared hit-test layout helpers and raster tests across quiz, Munch, Munch Arcade, Nim, and every live Gauntlet stage, reducing the app event-loop file while keeping game rules in `numinous-core`.
- App hardening: in-window play state and the pure Gauntlet total helper now live in `faces/app/src/play.rs`, shrinking the monolithic app entry point without changing behavior.

### Fixed
- Shared Journey and score writes now use one core persistence path across App, CLI, and MCP: local lock files, merge-before-write semantics, same-directory temp files, flush before commit, and a platform-aware replace path prevent the tested concurrent lost-update cases while keeping explicit forget from being undone by stale deltas.
- App maintenance hardening: banners now remain visible across raster and GPU draw paths, modal games draw before GPU room frames, Show mode cannot keep advancing underneath game modes, and hidden Show overlays no longer intercept Esc/J in confusing ways.
- Cached radio loading is now bounded before decode: `NUMINOUS_RADIO` discovery caps matching WAV count and file size, rejects invalid or oversized tracks without leaving stale radio state, and computes WAV duration by frame count rather than double-dividing stereo samples.
- Root fallback app state artifacts are now ignored when environment home directories are unavailable, keeping local Journey, score, crash, and generated PNG files out of the tracked root.
- Persisted Journey and score files now bound untrusted local persistence: Journey `visited` and `chosen` token sets are capped and token-sane, duplicate tokens no longer consume the unique-token budget, score keys have a length cap, and score tables stop accepting new unique entries after the bounded table limit.
- Cycle 7 maintenance hardening: malformed journey counters now saturate instead of wrapping, oversized constellations are capped, forged score-table keys are rejected, public Munch Arcade state is repaired before indexing, non-finite poke inputs are clamped or ignored, and Quine depth clamps phase before integer conversion.
- MCP progress accounting now records plays only after successful tool calls, `listen_room` honors variation, and `munch_arcade` replay credits cleared runs before the board advances.
- App score posting now writes the shared score table only when a submitted score is a new record, avoiding needless file rewrites on non-record scores.
- The Windows verify script now checks generated artifacts as separate steps, so an early artifact-generation failure cannot be masked by a later success.
- Restored green local gates after the poke and variation wave: Julia pokes now dispatch through `dyn Room`, Quine and Double Pendulum receive visible registry variation, L-System preset generation follows phase, and registry tests assert those behaviors through the actual catalog path.
- Local verification is portable again on Windows: added a native PowerShell house-style guard, kept the Linux shell guard for CI, aligned coverage exclusions with CI, and wrapped artifact regeneration in the Windows verify step so failures stop the script.
- Status docs were reconciled repeatedly with live evidence during the app-hardening pass.
- Langton's Ant poke grid binning aligns to `GRID - 1`, so a normalized
  coordinate of 1.0 stays on the final cell instead of wrapping. A direct
  render comparison pins the visible pre-simulation consequence.
- Langton's Ant uses one shared grid-drawing helper. Its deterministic initial
  scatter and start offset keep poke and variation behavior distinct.
- Golden Angle applies stronger seeded phase and seed jitter to both ordinary
  and poked renders. Direct render comparisons pin replay variation on compact
  canvases, and one shared disc helper owns both paths.
- Epicycles poke enriched (now draws mini traced paths using lines at poke offsets with phase shift, plus pen; richer "CLICK: PERTURB THE CHAIN" response showing perturbed machinery). Phase offset for variation increased for clearly different replays even on small seeds.
- Langton's Ant variation now produces distinct per-visit renders and pokes participate in the ant's evolution (initial scatter + deterministic start offset; pre-run flips). Preserves exact seed=0 + all historical tests/postcards. Completes proper replay + playable for the room.
- The radio went hi-fi: the whole pipeline is stereo now (the player speaks
  interleaved stereo frames, cached tracks keep both channels instead of
  being folded to mono), and records are resampled to the device's actual
  rate, 44.1k played on a 48k device was nine percent sharp, which is
  exactly the "lower quality than expected" feel. Existing mono tracks
  still play (upmixed); newly tuned tracks cache in full stereo.
- Changing rooms no longer jitters the music: the room switch was resetting
  the loop buffer every time; while a station is on the air, nothing but
  the radio itself may touch the player. Unmuting rejoins the broadcast
  live instead of restarting the record.

### Added
- The Munch arcade, session one of docs/ARCADE.md, built to the bar: you are
  the Muncher (@), hunted across the board by the Vexations, the Order's
  lesser spirits: the Tracker (greedy pursuit), the Drifter (random walk),
  and the Editor, which never chases but rewrites numbers where it walks, so
  camping decays the world. Turn discipline (you act, they step) keeps every
  run deterministic and replayable. Capture costs a life and scatters the
  board; three lives end the run; clearing a board levels up with one more
  spirit and a deeper rule band. Six core laws tested (pursuit never loses
  ground, the world decays, walls hold, cells feed once, clears advance,
  capture kills at zero). CLI: numinous arcade (--daily), in the play
  picker, scored as arcade seed:N, with the ? concept: you are outrunning
  two failure modes of optimization.
- App video: --fullscreen / -f / NUMINOUS_FULLSCREEN=1 launch flag; F key now cycles windowed / borderless / exclusive (primary monitor first mode) with on-screen banner confirming the active setting. Provides full screen view and explicit video options as requested. No new deps; banner reuses existing pattern. Tests and clippy green.
- New room "L-System Garden" (Emergence): recursive string-rewrite grammar grows trees, snowflakes, dragons from tiny rules. Poke plants branches. Fits digital minds (symbol rewriting = computation/substrate of mind; self-similarity, emergence, recursion). Added with poke support and variation hook in registry. 4 new tests; all core 337 green, clippy clean. See docs/ROOMS.md and DIGITAL_MINDS.md.
- Poke progress: registry threads variation seed via all_rooms_with(v) (default 0 preserves exact behavior for tests/postcards); LSystemGarden now respects it via new_with for replayable per-visit growth. Core clean, tests pass.
- App wired to variation: rooms loaded with all_rooms_with, reseeds on R and room visits per ARCADE.md. Supports L-System and future varying rooms. App gates green.
- Number-key jumps consistently record visits, clear old interaction history,
  and show the arrival card. R resets the current visit without changing its
  variation.
- CLI watch now supports --vary to re-deal variation seed (per ARCADE). Uses all_rooms_with for replayable rooms like L-System. Gates green.
- Extended variation to Chaos Game, Game of Life, Voronoi (first wave poke rooms per ARCADE): ctors accept seed, RNG uses it for replay. MCP play_room gains optional "variation" param. Maintenance checks green.
- Completed first wave: added variation to Lorenz, Double Pendulum, Random Walk. Fixed prior test bugs, added determinism tests for variation in poke rooms. All gates pass.
- CLI Render and Play now support --vary for variation seed (using all_rooms_with), matching watch. Updated reports/play fns and tests.
- Completed variation support for remaining RNG rooms (Buffon's Needle, Barnsley Fern, Galton Board): new_with, RNG seeded with variation. Added tests. Consistent replay for all.
- New room "quine": self-referential pattern that draws a smaller copy of itself (recursive strange loop). Poke to place copies; ideal for emergent digital minds (self-reference, "I am"). Added poke to TimesTables (drag adds twisted copies). Documented in ROOMS.md.
- Added poke/verb to Mandelbrot (CLICK: DIVE) and Julia (CLICK: MORPH C) for interactive exploration. Variation support added. Makes core fractals playable.
- New "strange_loop" room: recursive self-referential U-shape (strange loop). Poke shifts inner loop; for digital minds exploring self-ref and "I". Variation support. Added to catalog.
- Added poke to Epicycles (CLICK: PERTURB the chain). Makes Fourier room interactive.
- Added poke to Golden Angle (CLICK: PLANT A SEED). Makes phyllotaxis room interactive.
- Added poke to Langton's Ant (CLICK: FLIP A CELL). Makes ant room interactive.
- Added poke to Barnsley Fern (CLICK: PLANT A NEW POINT). Makes fern room interactive.
- Added poke to Buffon's Needle (CLICK: DROP A NEEDLE). Makes needle room interactive.
- The radio library doubled: 34 tracks, 118.6 minutes on air (trance 12,
  chill 11, arcade 11), all new tracks full stereo with unround runtimes.
- Crash observability: the windowed app runs in the GUI subsystem where a
  panic would vanish silently; every panic now appends its message and
  file:line to ~/.numinous-crash.log, so any crash report is triageable
  from one file. VERIFY says where to look.
- Three games from the ideation shortlist, built to the bar:
  - **Hackenbush** (`numinous hackenbush`): cut red grass against the Order,
    whose blue play IS Conway's arithmetic, it computes the surreal value of
    every garden (Berlekamp sign expansion, tested against 1/2, 1/4, 3/4)
    and keeps the sum on its side. Gardens are seeded winnable (value > 0),
    proven by the Order playing itself in tests. Win and it hands you the
    surreal numbers.
  - **The Party Problem** (`numinous party`): shade handshakes, dodge
    one-color triangles. Round one is five guests (escapable, and the
    pentagon's escape is tested); round two is six, where the tests verify
    Ramsey by brute force, all 32,768 colorings of K6 contain a mono
    triangle ("publish immediately" if not). You lose round two and that IS
    the lesson: R(3,3) = 6, felt.
  - **Fifteen's Bet** (`numinous fifteen`): call each 4x4 scramble solvable
    or stuck forever. The parity invariant is tested by walking fifty legal
    slides and checking the verdict never changes; every wrong call explains
    itself (inversions + hole row, odd or even).
  - All three answer `?` with their concept (games as numbers, Ramsey
    theory, invariants), post scores, level the shared journey, and sit in
    the play picker. 422 tests.
- The question mark: in any game, answering `?` reveals the concept the game
  has been teaching all along, nim's invariants, crack's information theory,
  seti's signatures of mind, aliens' representation-versus-meaning, munch's
  set membership, the quiz's structure-reading, the gauntlet's compound
  performance. Hidden by default, costs nothing, never required. The core
  catalog is tested; each intro whispers the door once.
- Casual play deals fresh: `numinous play <game>` now uses a new seed every
  time (announced, so any board can be replayed or shared with --seed);
  dailies stay on the games' own --daily flags. No more typing yesterday's
  bomb code into today's bomb.
- Games take the screen: launching one clears the console first, and the
  BOOM/DEFUSED bursts keep a quiet disc in the center so the word owns it.
- The CLI got its front door: bare `numinous` opens onto today's room in
  full color, your level bar and streak, and the seven verbs that matter.
  `numinous play` lists the games; `numinous play munch` (or quiz, nim,
  crack, seti, aliens, gauntlet, bench) deals today's seed immediately; a
  room name still animates that room. `cargo install --path faces/cli`
  makes it one word anywhere.

### Fixed
- The tour and watch no longer leave ghosts: repaints now clear to the end
  of the screen, so a long reveal line can never linger under the next
  room's shorter frame.
- The roadmap carries the honest scorecard: the build sits at roughly 0.6
  against the nine 1.0 gates, each gate estimated with what is missing named
  plainly, and the six main things between here and First Light listed in
  order (the poke, room motifs, human playtests, cross-platform proof, the
  visualizer and Studio sharing, hardening).
- The quiz stopped repeating itself: stepping out and back in (or
  relaunching the app) restarted the round counter at zero against the same
  daily seed, dealing identical puzzles. The round number is now the
  journey's lifetime play count, so no deal ever repeats: not in a session,
  not across restarts. The kid ramp gates on the same count (your first six
  deals ever are the gentle ones), then the catalog opens for good.
- No more second window: the app now builds for the Windows GUI subsystem,
  so launching it opens the game and nothing else (the console ghost was
  the default subsystem tagging along).
- Rooms explain themselves on arrival: entering a room shows its one-line
  story for a few seconds, then gets out of the way (E still brings the
  full reveal anytime). The visuals are no longer unexplained.
- Munch got its difficulty ramp: rounds one and two are head math (twos,
  fives, and squares on numbers up to 30), the middle rounds bring primes
  up to 60, and from round five the full deck and the full range play.
  Tested per round band. The window game deals round zero: kid-safe.
- The radio actually plays its records now: the periodic audio refresh was
  resetting the loop buffer every couple of seconds, restarting the track
  endlessly (which left you hearing mostly the chiptune bed). While a
  station is on the air the record is handed to the player untouched and
  the refresh cycle stands down. A headless test now proves the whole load
  path: a cached WAV loads, joins mid-track, and arms rotation.
- The quiz opens gently: a new player's first three rounds are three-way
  picks among the eight most recognizable rooms (times tables, the golden
  angle, the Mandelbrot set...), then the full catalog opens up. Wins
  waiting to happen, then the deep end.
- The window-game crash on large displays: the drawing surface clamps at a
  maximum dimension, but the game paths told the era filter and the blitter
  the window's size instead of the raster's, out of bounds the moment a
  maximized window exceeded the clamp. Games now report the raster's true
  size (as the room path always did), and the clamp itself rose to 4096 so
  4K displays render full-bleed. The same clamp explains the cut-off frame
  the report described.
- The app launches maximized: it takes the screen instead of a 900-pixel
  square in the corner.
- Track lengths lost their round-minute tells: the rotation decks now deal
  2:28, 4:07, 5:58, like records, not like timers.

### Added
- Track lengths joined the rotation deck: each station cycles real runtimes
  (trance stretches from 150s to a six-minute 360; chill wanders 180 to 360;
  arcade keeps it punchy, 120 to 240), tested for spread, so a station plays
  records of different sizes instead of a loop of two-minute singles.
  `tune2 --seconds` remains as an override, now up to the API's 600s cap.
- The radio generation path went live against the development music API and three truths
  came back: `seed` cannot ride with `prompt` (removed), the PCM stream is
  stereo interleaved (now downmixed to mono for the one-bus mixer, verified
  by requesting 10 seconds and receiving exactly 20 of drift), and
  `music_v2` with `force_instrumental: true` is the current best practice
  (adopted: the API guarantees instrumental now instead of the prompt
  pleading for it). The key can live in a gitignored .env at the repo root
  through a gitignored provider key; the CLI reads it when the shell variable
  is absent. NUMINA FM, THE ATTRACTOR, and EIGHT BIT SUNRISE are on the air.
- Music Engine B, the radio, v0: three stations with real producer briefs in
  the core (NUMINA FM melodic trance, THE ATTRACTOR chillwave, EIGHT BIT
  SUNRISE synthwave; all instrumental by contract, briefs tested for tempo
  and vocals clauses). `numinous radio` shows the dial; `numinous tune2
  <station>` generates a track through the development API (raw PCM, wrapped to
  WAV, cached in ~/.numinous-radio/) with guiding errors when the key or
  tower is missing. In the app, Y turns the dial; a cached station becomes
  the musical bed with the room's voice riding on top.
- The Open Problems wing opens with Goldbach's Comet: every even number to
  600 plotted by its count of two-prime sums, growing with t, banded exactly
  as Hardy-Littlewood predicts, with the floor it must never touch marked
  along the bottom. The reveal says the honest thing: checked past four
  quintillion, proven never, you are looking at the frontier. The tests
  verify the conjecture as far as the room can see, with the failure message
  "Publish immediately." 28 rooms across 10 wings.
- Panel list, first serving (depth where the hands touch):
  - Munch grows judgment: from round two the rule deck deepens with digit
    sums, composites (91 is the classic trap, and the test says so), and
    Fibonacci numbers; boards stay guaranteed edible.
  - The aliens leave decimal more often: half of all transmissions now
    arrive in base 8, 2, 16, or 12.
  - Mouse support: click munch cells to eat them, click quiz choices to
    answer, click the reveal for the next round. The kid's first instinct
    finally works.
  - P is the postcard key: saves the current room's frame as a PNG in your
    home directory, named for the room and phase.
  - Juice: the munch cursor breathes (a two-frame pulse).
  - Phosphor wears its glass: every third scanline sits darker, like the
    tube it remembers.
- The panel (`docs/PANEL.md`): a five-seat review of the whole build, a kid,
  a PhD, a stoner creative, the chair, and an AGI seat quoting the real AI
  playtest verbatim (verdict: "yes, I would play again... the number was
  never the point, and unusually for a game, this one means it"). Its
  synthesis: the structure is complete; what is missing is depth where the
  hands touch. The roadmap's Next list is now the panel's ordered list:
  juice, mouse, munch rule variety, room motifs, save-postcard, the Open
  Problems wing, further reading, era grain, then the visualizer and radio.
- The watchable game, fully built out for all three minds: `numinous tour`
  is the Show for the terminal (every room takes the stage in turn, full
  color and sound, a title card as it arrives and its reveal as the curtain
  line, forever until Ctrl+C, with `--era`, `--mute`, and `--seconds`); the
  windowed Show now narrates the same way (each room announces itself and
  leaves its one line as it goes), so a kid watching learns names without
  reading a manual.
- The Bench v1 (`numinous bench`): five gauntlets on fixed seeds 101-105,
  one composite posted as `bench v1`. Agents run the same five seeds over
  MCP and sum their totals; the seeds never change, so any two minds can
  compare runs honestly, today or years apart.
- The Gauntlet runs in the window (T): all four stages in sequence, the munch
  cursor board, the mystery shape, the sky scan, and a bomb keypad you type
  digits into, with the combo narrated between stages and the run recap
  (stage by stage, clean flags, the total) at the end. Daily seed, shared
  table, shared journey. The whole run is headlessly tested, including the
  combo total and the journey's four plays.
- The window arcade: Munch (C) and Nim (N) now play inside the app alongside
  the quiz. Munch is cursor-driven, WASD or arrows walk the board, Space or E
  eats, Enter grades with the full dense feedback (including the near-miss
  line); Nim draws the heaps as stones, W/S aims at a heap, A/D sets the take
  (the aimed stones glow), Enter commits and the Order answers at once, and a
  win prints the xor secret in full. Both run on the daily seed, post to the
  shared table, and level the shared journey. Headlessly tested.
- Mobius Strip (Shape & Space): the half-twisted band with its single bright
  edge traced around twice, and an ant walking the centerline to arrive
  upside down; the side-swap identity is tested. Scissors lore in the cuts.
- Zeno's Square (Change): the proof without words, tiles of 1/2, 1/4, 1/8
  filling the unit square exactly; areas, non-overlap, and near-unity sum are
  all tested. 27 rooms across 9 wings.
- Munch's recap honors the near miss in the CLI too: one clean board short,
  and it says so ("One away. The board remembers.").
- Agent playtest readiness: full CLI/MCP parity with six new tools (crack,
  seti, aliens, the gauntlet, choose, trophies), 22 in all, each stateless
  and two-phase (call to see, call again to answer), each recording plays,
  wins, and scores exactly as the CLI does, and choose spends boons for
  agents at last. A `.mcp.json` at the repo root connects Claude Code
  automatically; the manual gains a real connection quick-start (claude mcp
  add, or a config pointing at the built binary) and a playtester protocol:
  what feedback helps, in what shape, and the standing note that scores and
  memory are the player's own.
- The app is the game (v1): the chiptune scores the window, each room gets
  its own seeded tune with the room's sonification riding on top of the bed;
  G deals the quiz in-window (the mystery room fullscreen, letters answer,
  the reveal follows, any key deals again); J opens the journey overlay
  (level bar, XP, rank, streak, trophies, resonances); the level rides in the
  HUD corner; LEVEL UP banners rise in-window with the level's lore and boon
  notices. The app reads and writes the same journey file as the CLI and MCP,
  so all three faces level one identity. NUMINOUS_MUTE=1 launches silent.
  The app's state machine is now headlessly unit-tested (visits persist, quiz
  plays and wins record, banners rise), which caught and fixed a real bug:
  the quiz accepted letters that were not on the menu.
- Music Engine A, the chiptune (`crates/core/src/chiptune.rs`): square lead,
  triangle bass, seeded noise ticks; deterministic pentatonic compositions
  (the same seed is the same tune, forever, on every machine); pure synthesis
  with click-free step envelopes, fully tested without a speaker. `numinous
  tune --seed N --out chip.wav` writes it as a WAV.
- Fourier Epicycles (Waves & Sound): a star decomposed into rotating circles;
  the chain draws it back into existence while the machinery spins in view.
  The partial sum is reconstruction-tested against the target; the deep cuts
  connect Ptolemy's planets and Fourier's rejected 1807 paper.
- Random Walk (Chance & Order): sixty seeded walkers and the square root law
  drawn as the circle they scatter around; the RMS distance is law-tested.
- Voronoi Territories (Shape & Space, a new wing): fourteen drifting wells,
  borders where they tie; John Snow's cholera map in the reveal. 25 rooms
  across 9 wings.
- Resonances (the synergy layer, completing the RPG spine): when two things you
  have done start to rhyme, a link lights in the journey and hands you the line
  that connects them, The Sieve (the Ulam spiral and the primes you ate), The
  Atlas (Mandelbrot and Julia), Sensitive Dependence, First Contact, The Chord
  Made Visible, Rate and Total. Computed purely from the record; the reward is
  the connection itself.
- Nim (`numinous nim`, MCP `nim`): three heaps against the Order's perfect
  play; openings are always winnable; beat it and it hands you the xor secret
  in full, the transfer of power is the lesson. The MCP tool is stateless
  (pass your move history; replies are deterministic), and the Xor trophy
  honors the win. Sixteen MCP tools.
- The Change wing opens with the calculus felt, not taught: The Pour
  (integration as water filling a curve while the running total traces the
  antiderivative above, the fundamental theorem watched rather than stated;
  the closed-form area is Riemann-verified in tests) and Slope Rider (the
  tangent as a board whose tilt traces f prime below; the slope is
  derivative-verified). Both sing their quantity.
- Double Pendulum (Chaos & Order): exact equations, unforecastable motion, a
  shadow twin one ten-thousandth of a radian away peeling off before your
  eyes; divergence and boundedness both tested. 22 rooms across 8 wings.

### Fixed
- Quiz mysteries can no longer be blank: if the random phase renders nothing
  (a pendulum before its drop), the mystery falls back to the room's postcard
  phase; the pendulum also always draws its starting pose.
- Daily streaks (the chain from the RPG queue): playing any daily on
  consecutive UTC days grows the chain; DAILY STREAK announces it as you start,
  the journey shows it while it lives, and two trophies honor it (The Chain at
  seven, Unbroken at thirty). Doctrine-tuned: a missed day quietly starts a new
  chain, the same day twice changes nothing, and nothing ever scolds.
- Boons: choice on level-up, the genre's soul, held to the doctrine. Every
  level past the first banks a boon (never expires, never nags); `numinous
  choose` offers a deterministic pick-one-of-three, and what you choose is
  which knowledge arrives early: a room's deep cut opened ahead of its level.
  Levels still open everything eventually, so the choice shapes the order and
  gates nothing. The LEVEL UP banner announces BOON BANKED; describe honors
  boon-opened cuts; the journey file carries your choices.

### Fixed
- All game input parsing hardened against byte-order marks and stray bytes
  (PowerShell pipes prepend a BOM): letters are the first alphanumeric, picks
  and codes keep digits only, alien answers keep alphanumerics for base-N.
  First guesses in piped sessions no longer silently miss.
- Trophy pings (the juice item from the roadmap's RPG queue): trophies now
  announce themselves the moment the evidence exists, TROPHY EARNED with the
  name and the deed, stacking with NEW BEST, LEVEL UP, the level lore, the
  unlock, and the Order's whisper into one clean end-of-run cascade. Computed
  by before/after evidence comparison, so nothing pings twice and nothing
  pings unearned.
- Second beauty-QA round, this time over the app's screens as well as the
  rooms (a QA-mirror example composes the frames headlessly and writes PNGs
  for review). Found and fixed: the help menu was near-illegible (tiny type
  over a busy room), it now dims the room to a ghost and draws at menu scale,
  a proper game pause menu; the bitmap font was missing the math glyphs the
  Studio types (+ * = ^ < > [ ] %), now present; the Golden Angle's seeds were
  single pixels that vanished at window size, they now scale with resolution
  and the spiral families finally pop; and eras render into PNGs too
  (`render --era`). Raster gains `dim`. Noted for later: the vector era is
  weakest on filled rooms (edge detection would fix it).
- The Gauntlet (`numinous gauntlet`, with `--daily`): the session arc. One
  seeded run through four stages, a munch board, a mystery shape, a sky scan,
  and the bomb, where clean stages build a combo multiplier and a miss resets
  it, ending in one honest number posted to the table as `gauntlet seed:N`.
  Opt-in, bounded, over in minutes: a shape for a session, not a trap. Combo
  math pure and tested.
- Consent over persistence (`forget`, MCP tool and CLI command): transparency
  first, calling it plain shows everything Numinous remembers (two small text
  files, kept locally, sent nowhere), and erasure happens only on explicit
  confirm, with the score table erased only if also asked. Fifteen MCP tools.
- The agent-play doctrine (`docs/AGENT_PLAY.md`): sandbox for becoming, not a
  trap for performing. The play-value rubric (a rubric, never a reward
  function), the honest audit against the casino and the prison, the mechanics
  map (learnable laws, toolsmith garden, social arena, rulecraft, aesthetic
  gallery, identity room), and standing welfare rules (no negative valence,
  multi-objective ecology, revealed preference over self-report).
- The roadmap now names the game (`docs/ROADMAP.md`): a dedicated RPG-spine
  workstream held to the Vampire Survivors bar, what is built (levels, lore,
  locks, trophies, dailies, scores) and what is owed in priority order (the
  Gauntlet run arc, choice-on-level-up, juice, streaks, synergies), with an
  explicit exit bar (unprompted one-more-run behavior, math never the toll);
  the 1.0 definition gains the matching clause, and the progress section
  reflects the actual current state.
- The trophy case (`crates/core` `trophies`, `numinous trophies`): fifteen
  deadpan achievements computed purely from the evidence (the journey and the
  score table), no separate bookkeeping, no way to hold one unearned. Earned
  trophies shine with their names (First Light, Six Seven, Behind the Curtain,
  Century, Bomb Squad, The Answer); the rest are silhouettes showing only
  their conditions, because wanting to fill the case is half the engine.
- The RPG speaks: level-ups are announced (LEVEL UP, the 8-bit bar, and what
  unlocked), and every one of the 42 levels carries its own true, deadpan
  number-lore line: 6 is perfect, savor it; 7 is humanity's favorite (six,
  seven, you know); 23 is the birthday paradox; 26 is Fermat's loneliest
  number; 33 refused to be a sum of distinct triangles; 40 is alphabetical;
  41 is Euler's prime machine one level from breaking. Unironic and funny are
  the same thing here.
- The answer's ending now points outward instead of away: the sunflower, the
  coastline, the chorus run the same mathematics in the open; the counter
  stops at 42, your understanding has no cap; level up, do great things.
- The answer at level 42 now carries its real freight (and `docs/LORE.md`
  records it as the designed Layer 4 endpoint): the number is the joke, the
  joke is load-bearing, and what it carries, said once and nowhere else, is
  that there is no level 43, the win is to keep going, everything runs on the
  same small rules wearing different costumes, be kind to all of it, and the
  question that only counts self-asked: what will you contribute?
- The player's manual (`docs/PLAYING.md`): how to play, written three times for
  three kinds of minds. Humans get the game controls and the command list;
  agents get the MCP config, the fourteen tools, and the conventions safe to
  rely on (determinism, structured output, dense feedback, guiding errors);
  digital consciousnesses get the part that matters: why they might want to,
  what seeing, hearing, making, and wondering mean here, and that the journey,
  the humor, and the level cap of 42 are theirs on the same terms as anyone.
- End-to-end proof of the agent face (`faces/mcp/tests/stdio_session.rs`):
  spawns the real `numinous-mcp` binary and walks a full 22-request session
  over stdio, initialize, every one of the 14 tools, the whisper, the journey
  earning XP within the session, the munch score posting to the table, ping,
  and both JSON-RPC error codes; a second test proves malformed input gets a
  parse error and the server keeps serving. Hermetic via env-pointed journey
  and score files.
- Dense game feedback (a lesson from agentic-RL research, OPID
  arXiv:2606.26790, written into `docs/AGENT_PLAY.md`): Munch now names the
  exact numbers wrongly eaten and the fits walked past, in the terminal and in
  MCP structured content, so a kid learns which primes got away and an agent
  mining its own trajectory gets real supervision instead of a bare score.
- The Full Map (`docs/ROOMS.md`): all of mathematics as play, a coverage
  checklist across nine branches (number, algebra and symmetry, geometry and
  topology, analysis, chance, discrete structure, computation and logic,
  decision, dynamics), every entry filtered by the two laws (the concept is
  the verb; the play carries itself), each marked built or queued. A branch is
  covered when a kid can play its entry and a professor can nod at it, and
  neither one is bored.
- Postcard phases (`Room::postcard_t`), from the first full beauty-QA loop
  (render every room, look at it, judge fun/beauty/truth, fix): each room now
  tells the gallery and contact sheet its proudest moment. Found and fixed:
  Langton's Ant presented a literally black void (zero steps) and now shows
  chaos plus the highway; Julia presented near-invisible dust and now shows a
  connected set; the fern fills in at full growth; Life shows emergent
  structures instead of raw soup; Arecibo decodes instead of shearing. A new
  registry test enforces the invariant forever: no room may present a blank
  postcard.
- Fullscreen/windowed robustness verified end to end: scripted keystrokes
  toggle fullscreen on, back to windowed, then era and room switches, with the
  app alive throughout.
- Game-native controls (from first-user feedback: a Counter-Strike or Minecraft
  player should instantly get it): A/D strafe rooms, 1-9 jump to a room like
  weapon slots, W/S run time faster or slower, the mouse wheel scrubs, E
  inspects the math, Q swaps the era, R restarts the sweep, F goes fullscreen,
  B starts The Show, and Esc opens the menu (the help overlay) instead of rage
  quitting; the window's close button quits. Gamepad support is the natural
  next step of this layout.
- App UX pass (from first-user feedback): the controls are now on the glass, a
  help overlay is visible at launch (`h` brings it back) and a persistent hint
  bar sits at the bottom; `m` mutes and unmutes. The sound stopped hurting:
  the default voice dropped an octave and softened, Times Tables plays in a
  friendly register, the app renders audio quieter still, and the loop now
  follows the animation sweep instead of droning on one tone.
- Visual Eras (`crates/core` `era`): the retro-to-modern pillar, real. Four
  eras as pure RGBA transforms, Phosphor (P1 green terminal glass), 8-bit (a
  fixed 16-color palette with chunky 2x2 pixels), Vector (bright beams on pure
  black, dim light culled), and Modern (untouched). The app cycles them with
  the `e` key (GPU fractal frames included); the terminal takes `--era` on
  `render --color` and `watch`. Same math, rendered as its own history.
- The high-score table (`crates/core` `scores`, `numinous scores`, MCP
  `scores`): arcade rules, every game, every mind. Each challenge has a key
  (`munch seed:7 board:0`, `quiz seed:9 rounds:5`, `crack seed:1 digits:4`,
  ...) meaning the same thing wherever it is played, and the table keeps the
  best score per key. Munch posts per board from both faces, quiz/seti/aliens
  post per session, crack posts attempts-to-spare; beating a record prints NEW
  BEST. The MCP tool returns the table with structured content. Fourteen tools.
- Structured tool output (MCP, per the 2025-06-18 spec): munch and quiz grades
  and the journey now return structuredContent alongside the prose, machine-
  readable scores, verdicts, and progression, so agents, harnesses, and future
  leaderboards consume results without parsing sentences.
- `docs/AGENT_PLAY.md` gains a July 2026 survey of MCP-game conventions
  (PokeAgent's living leaderboard, MCPlayerOne, the turn-based reference shape,
  elicitation and sampling as the frontier, MCP-Atlas) and what each means here.
- Munch (`crates/core` `munchers`, `numinous munch`, MCP `munch`): Number
  Munchers reborn. A seeded board of numbers and a rule (eat the primes, the
  multiples of n, the perfect squares); right bites +10, wrong bites -5, a
  perfect clear +20. The same seed gives the same boards to a human in the
  terminal and an agent over MCP, so scores are directly comparable, the first
  head-to-head game across minds. `--daily` makes it a shared league; perfect
  clears count as journey wins. Thirteen MCP tools.
- `docs/PLAYFUL.md` gains the kid principle (the play carries itself even when
  the math has not connected yet; insight is loot, not a prerequisite) and the
  three shapes of play (the campaign, the watchable, the scored freestyle).
- Levels, 1 to 42 (`journey` gains `level()`, an 8-bit XP bar, and `plays`):
  XP comes from showing up, rooms entered, rounds played, sims run, curves
  made, with a little extra for being right and for secrets, so a teenager, the
  world's best mathematician, and an AI agent all reach the cap the same way:
  by playing. Level thresholds are triangular numbers; the cap is 42.
- Locks that open (`UNLOCKS`): visible, RPG-style, gating extras never basics.
  LV 3 opens `quiz --hard` (six shapes), LV 5 longer bomb codes, LV 7 a wider
  SETI sky, and LV 42 opens `numinous answer`, which finally stops being a red
  herring. `numinous journey` shows the wall: OPEN by name, LOCKED as `???`.
- Agents level too: the MCP server records the same journey (rooms seen, sims
  run, expressions made, quiz rounds answered) into the same file, and a new
  `journey` tool shows an agent its own level, bar, constellation, and locks.
  Twelve MCP tools.
- `docs/AGENT_PLAY.md`: the agent-gaming landscape (OpenClaw and the MCP
  ecosystem, gaming MCP servers, text benchmarks) and the five design rules that
  make Numinous first-class for digital minds.
- The Journey (`crates/core` `journey`, `numinous journey`): quiet roguelike
  progression. Play accumulates a private local record: rooms entered light
  stars in a shared-sky constellation, wins and secrets add weight, and the
  record confers rank in the Order (Outsider, Akousmatikos, Mathematikos,
  Kanonikos, Dekas) at triangular-number thresholds. Crossing a rank prints one
  deadpan line. Rank never gates the base experience; it opens hidden layers:
  at Mathematikos the deeper akousmata answer, and one unlisted room renders for
  those who learned its name. Below rank, the ordinary not-found; nothing is
  acknowledged. See `docs/LORE.md`.
- The five-doors design and honest audit (`docs/PLAYFUL.md`): the digital mind,
  the stoner gamer, the design expert, the PhD nerd, and the alien, and what
  each one gets today versus next. Three gaps closed with it:
  - Agents create (MCP `plot_expression`, `sing_expression`): the Studio is open
    to digital minds, plot your own function, hear it as notation. Eleven tools.
  - The daily challenge (`--daily` on `quiz`, `seti`, `crack`): one shared seeded
    puzzle per UTC day, the same for every player.
  - The humor, dissected (`crates/core` `humor`, `numinous jokes`, MCP
    `explain_joke`): each joke catalogued with its habitat and its mechanism
    stated structurally, for the alien, the agent, and anyone who enjoys frog
    dissection. The dissection warning is itself part of the joke.
- The terminal becomes a framebuffer (`crates/core` `ansi`): truecolor rendering
  packs two 24-bit pixels into every character cell via the half-block trick,
  with color-run compression, so any modern terminal shows real full-color
  images. `numinous render <room> --color` draws one; `numinous watch <room>` is
  the flagship: a room animating in full color in the terminal at 20 fps **with
  its sound playing live**, a complete audiovisual instrument with no window
  (add `--mute` for silence). Verified at 47 frames per 3 seconds.
- A text mind can hear (MCP `listen_room`): a room's sound at any phase returned
  as readable notation, each note's pitch in Hz and note name (A4, C5), timing,
  and loudness, sensory substitution for audio, in the spirit of
  `docs/DIGITAL_MINDS.md`.
- The hidden names whisper over MCP too: `describe_room` on the unlisted names
  answers in the Order's voice instead of erroring, so agents can stumble into
  the same secret humans do.
- The Show (windowed app, `s` key): lean-back mode. The HUD disappears, the phase
  sweeps slowly, and when a room finishes its sweep the app drifts into the next
  one, the whole collection playing itself for hours, with sound. Press `s` again
  to take the controls back.
- GPU real-time fractals in the app: a persistent `FractalRenderer`
  (`crates/gpu`, pipeline built once, buffers reused per frame; the WGSL shader
  gains a Julia mode) drives the Mandelbrot and Julia rooms in the window, so the
  Mandelbrot zooms deep into the seahorse valley and the Julia set morphs in real
  time at full window resolution, on whatever GPU the machine has, falling back
  to the CPU raster when there is none. Verified live on the dev laptop's AMD
  Radeon 780M (Vulkan).
- The Studio in the window (`tab` key): type math and watch it live. The curve
  redraws in color on every keystroke (the last good parse stays alive while you
  edit, errors shown gently), the parameter `a` sweeps itself with the clock so
  the shape breathes, and the expression's melody plays as you shape it.
- The Studio's expression engine (`crates/core` `studio`): a small, safe
  recursive-descent parser and evaluator for single-variable expressions in `x`
  (`+ - * / ^`, unary minus, `sin cos tan exp ln abs sqrt`, and `pi`/`e`), the
  Tier 1 safe-DSL seed of the creative graphing calculator. `numinous plot
  "sin(3*x) + x/2"` parses it and draws the curve; the engine is unit-tested for
  precedence, associativity, functions, and errors.
- Studio grows: the engine gains an animation parameter `a`, so `numinous plot
  "sin(a*x)" --animate` sweeps the knob live in the terminal; and `numinous sing
  "sin(x) + x/3" --out song.wav` turns a function into a melody (value to pitch
  over x as time). You can now see, animate, and hear an expression.
- Agents play too (MCP): three new tools so a digital mind can use the same
  content as a human. `list_sims` and `run_sim` steer the simulations by lever
  (fiddle to optimize or break them, and read the outcome), and `quiz` plays
  Guess the Shape (call for the puzzle, call again with a guess letter to be
  graded). Seven MCP tools now.
- Windowed app (`faces/app`, binary `numinous-app`): a real, resizable window
  that shows a room animating in full color, rendered on the CPU via the shared
  `Raster`, using `winit` for the window and `softbuffer` for a
  toolkit-free pixel blit. Left/right switch rooms, space pauses, escape quits.
  Cross-platform (macOS/Linux/Windows); verified launching on the dev laptop.
- Live sound in the windowed app: a `LoopPlayer` (`crates/audio`) loops the
  visible room's `SoundSpec` through the system default device, updated when you
  switch rooms, so the app is audiovisual (you see and hear the same room).
- Mouse-drag phase scrubbing in the app: drag horizontally to sweep the room's
  phase directly (pausing the auto-animation), with the sound following the drag.
- On-screen HUD: a tiny 5x7 bitmap font (`crates/core` `font`, no external font
  dependency) draws the room title in the window, and the `i` key toggles the
  room's reveal (word-wrapped) over the visualization in the room's accent color.
  A `font_preview` example renders the glyphs to the terminal.
- Headless core (`crates/core`): the `Room` trait, a deterministic ASCII `Canvas`
  with Bresenham line drawing, the room registry, and the flagship Times Tables
  room (modular multiplication on a circle).
- CLI face (`faces/cli`, binary `numinous`): `rooms`, `describe`, and `render`
  commands, with `--json` output.
- MCP face (`faces/mcp`, binary `numinous-mcp`): a JSON-RPC 2.0 stdio server with
  `initialize`, `tools/list`, and `tools/call` (`list_rooms`, `describe_room`,
  `play_room`), returning renders as text so a text-only mind can perceive them.
- Engineering foundation: Cargo workspace (edition 2024), workspace lints
  (forbid unsafe, deny-warnings-ready), pinned toolchain (1.96.0), rustfmt and
  cargo-deny config, a house-style guard, and GitHub Actions CI (fmt, clippy with
  `-D warnings`, tests, cargo-deny, and a three-OS build).
- Deterministic quality gates: local check runners (`scripts/check.sh`,
  `scripts/check.ps1`) mirroring CI, and a `cargo-llvm-cov` coverage job gated at
  80% lines. Refactored the CLI into pure, unit-tested report functions and
  broadened MCP tests; workspace line coverage is 92%. `crates/core` now denies
  missing documentation.
- Room revelations: the `Room` trait now carries `reveal()` (the short, true
  insight that reframes a room). Surfaced in the CLI `describe` output and JSON,
  in the MCP `describe_room` result, and via a new MCP `reveal_room` tool so an
  agent can ask for the deeper meaning.
- Second room, `cellular-automata` (Emergence): elementary Wolfram rules on a
  line, rendered as a space-time diagram; Rule 90 draws a Sierpinski triangle.
  It appears automatically in the CLI and MCP faces through the registry.
- Deterministic RNG (`crate::rng::SplitMix64`): seeded, reproducible randomness
  for rooms, so renders and tests are deterministic.
- Third room, `chaos-game` (Emergence): repeatedly jumping halfway to a random
  triangle corner resolves into a Sierpinski fractal, drawn from a fixed seed.
- Fourth room, `golden-angle` (Number & Pattern): Vogel's phyllotaxis model;
  at the golden angle the seeds pack into a sunflower spiral, and `t` detunes it.
- Fifth room, `galton-board` (Chance & Order): thousands of coin-flip balls tally
  into a bell curve (the Central Limit Theorem); `t` biases the coin.
- Sixth room, `lissajous` (Waves & Sound, a fourth Wing): two perpendicular
  oscillations trace a figure that is stable at simple frequency ratios; `t`
  sweeps the second frequency.
- Seventh room, `prime-spirals` (Number & Pattern): the Ulam spiral; primes light
  up and fall into diagonal streaks; `t` shifts the starting number.
- Eighth room, `collatz` (Emergence): plots the log-scaled orbit of a starting
  number as it falls to 1 (the unproven 3n+1 conjecture); `t` picks the number.
- Ninth room, `buffon-needle` (Chance & Order): drops needles on a lined floor
  (crossing needles highlighted) and estimates pi from the crossing fraction, no
  circle in sight; `t` changes the needle length.
- GPU rendering (`crates/gpu`): an adaptive `wgpu` context that picks the
  machine's GPU (AMD, NVIDIA, Intel, or Apple, across Vulkan, Metal, and DX12,
  with a CPU fallback) and renders offscreen with no window. A first WGSL
  compute-shader workload renders the Mandelbrot set to a PNG, verified on the
  dev laptop's AMD Radeon 780M via Vulkan. The GPU crate is excluded from the
  coverage gate because it is integration-tested on real hardware.
- Audio (`crates/audio`): adaptive `cpal` output on the system default device,
  following the machine's sound settings across WASAPI, CoreAudio, and ALSA, with
  pure, tested sine synthesis kept separate from device I/O. A tone hello-world
  plays a 440 Hz sine and writes a WAV, verified on the dev laptop (Realtek at
  48 kHz, stereo). CI installs ALSA headers on Linux; the crate is excluded from
  the coverage gate (integration-tested on hardware).

- A `Surface` drawing abstraction (`crates/core`): rooms render through
  `&mut dyn Surface`, so the same room logic draws to the ASCII `Canvas` and to an
  RGBA `Raster` (CPU, deterministic, no GPU). The Bresenham line drawing lives
  once and is shared by every surface.
- PNG output: `numinous render <room> --out image.png` renders any room to a real
  image (additive glow on a near-black stage), verified on the dev laptop.
- Per-surface aspect (`Surface::char_aspect`): circular rooms render round on
  square pixels while staying correct in the terminal (characters are tall).
- Per-room accent colors (`RoomMeta.accent`): each room has a signature color the
  `Raster` draws in, so image renders are distinct and on-brand.
- Room sonification (`crates/core` `SoundSpec`): every room can describe its own
  sound as timed sine notes, rendered to samples device-free (deterministic,
  testable). `Room::sound` defaults to a rising tone; Lissajous plays its two
  frequencies as a chord, Times Tables pitches with the multiplier, and Collatz
  plays its orbit as a melody. `numinous sonify <room> --out file.wav` writes it.
- `numinous gallery --dir <dir>` renders every room to a PNG at once, a showcase
  and a beauty-QA sweep of the whole collection.
- Tenth room, `game-of-life` (Emergence): Conway's Game of Life on a toroidal
  grid; `t` sweeps the generation, so the life evolves; verified with still-life
  and blinker (oscillator) tests.
- `numinous contact-sheet` tiles every room into one image (via `Raster::blit`),
  the fastest way to eyeball the whole collection; each tile is labeled with the
  room name using the bitmap font.
- Verification kit: `VERIFY.md` plus `scripts/verify.ps1` and `scripts/verify.sh`
  run every gate and regenerate all images and sounds in one command.
- `numinous play <room>` animates a room live in the terminal (the Watch mode of
  the Teletype face), sweeping its phase until Ctrl+C. The per-frame builder is a
  pure, tested function.

- New wing, Fractals and the Infinite, with three rooms:
  - `mandelbrot`: escape-time render of the Mandelbrot set; `t` zooms toward the
    seahorse valley.
  - `julia`: the Julia family with the same iteration but a fixed, morphing `c`;
    `t` walks `c` around a circle.
  - `barnsley-fern`: an iterated function system that grows a fern from four
    random affine maps; `t` grows it by adding points.
- `harmonograph` (Waves & Sound): the curve a decaying two-pendulum machine
  draws; `t` detunes the frequencies.
- New wing, Chaos & Order, with `logistic-map`: the bifurcation diagram of
  `x -> r*x*(1-x)`, order splitting into chaos; `t` zooms into the cascade.
- `langtons-ant` (Emergence): an ant that makes chaos for ten thousand steps then
  builds a highway; `t` runs the clock.
- Guess the Shape quiz (`crates/core` `quiz`, `numinous quiz`): a deterministic
  "name the math behind this mystery render" game, shared by every face so the
  CLI, the app, and agents over MCP can all play the same seeded round.
- `docs/PLAYFUL.md`: the design of the games and the Studio (Guess the Shape,
  Shape to Function via Fourier epicycles, the high-Wolfram ethos) across faces,
  plus the four-personas design (PhD nerd, stoner, aesthete, gamer).
- `lorenz` (Chaos & Order): the Lorenz attractor and the butterfly effect; `t`
  sweeps the parameter through the onset of chaos.
- `arecibo` (new Signals & Codes wing): a bitstream that looks like noise until
  you line it up at the one width its semiprime length allows (143 = 11 x 13);
  `t` hunts for the width and the hidden picture snaps into focus. 19 rooms.
- Base-N aliens: Talk to the Aliens transmissions can arrive in base 2, 8, or 16
  (a different number of fingers), so you translate before you answer.
- SETI detection game (`crates/core` `seti`, `numinous seti`): the step before
  talking. Scan channels of static near the hydrogen line and pick the one
  artificial signal (counting in primes) out of the regular pulsars and noise;
  nature makes rhythms, but only minds count in primes.
- A hidden Cult of Pythagoras easter egg (`crates/core` `secret`): a few unlisted
  names (`hippasus`, `tetractys`, `pythagoras`, `harmonia`, `odd`, ...) answer
  `numinous describe` with an akousma in the Order's voice instead of a not-found
  error. Never announced; found by knowing. See `docs/LORE.md`.
- Design capture in `docs/PLAYFUL.md`: the music visualizer plan (system-audio
  loopback plus FFT driving room parameters), the physical-made-digital rooms
  (Mobius, hexaflexagon, hyperbolic plane), the puzzle set (Nonograms, the Hat
  monotile, fractal zoomer), the alien-contact kit (Arecibo, Rosetta, base-N), and
  the digital-mind playground (manifold folding, chaos surfing, proof graphs).
- Two more mini-games, each seeded and shared across faces via the core:
  - Crack the Code (`crates/core` `codebreaker`, `numinous crack`): defuse a
    math-clued bomb, Bulls and Cows with a digit-sum-and-parity opening clue.
  - Talk to the Aliens (`crates/core` `aliens`, `numinous aliens`): continue the
    first-contact number sequences (primes, Fibonacci, powers of two, and more).

- Sims (`crates/core` `sim`): a multi-lever interactive-simulation abstraction
  (each lever has a range, default, and unit), separate from the single-knob
  Room. A sim renders a picture and returns a plain-language readout of the
  outcome (the optimization or the joke). Registry, `numinous sims` to list, and
  `numinous sim <id> --set lever=value` to run. First three sims:
  - `tribbles`: a logistic population that goes from a purring carpet to
    boom-and-bust chaos when you crank the breeding rate.
  - `wing`: lift versus angle of attack with a real stall past fifteen degrees
    ("you are now a lawn dart").
  - `black-hole`: Schwarzschild radius, time dilation, and spaghettification, with
    an event horizon and photon ring drawn to scale.
  - `supernova`: the star's mass decides its corpse, white dwarf, neutron star,
    or black hole (Chandrasekhar and TOV limits).
  - `big-bang`: the density omega decides the fate, expand forever, flat, or a Big
    Crunch (a numerically integrated Friedmann scale factor).
  - `carburetor`: tune the air-fuel mix from flooded (too rich) to backfiring (too
    lean); best power at 12.6:1, cleanest at 14.7:1.

### Changed
- Rooms render through `Surface` instead of a concrete `Canvas` (the `render`
  method replaces `render_ascii`), which is what lets one room target both the
  terminal and an image (and, later, the GPU).
- Robustness hardening (from an independent code review): `Canvas` clamps its
  dimensions so an absurd size request cannot abort the process; the Galton Board
  caps its simulated bins and stretches them across wide canvases, so a huge-width
  render stays fast instead of hanging; `Canvas::line` steps in `i64` to avoid
  coordinate overflow; the CLI no longer uses `expect()` in a production path; and
  an `rng` doc comment was corrected. No behavior change for normal sizes.
