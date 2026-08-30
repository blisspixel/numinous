# The Rooms

The content catalog: the phenomena Numinous is built from. Each **room** is one playable mathematical object. Rooms are grouped into **Wings** by feeling, not by curriculum.

**Current status (as of 2026-08):** 354 catalog rooms across the wings plus
hidden content. Per-visit variation seed is threaded through registry/app/CLI/
MCP; every catalog room uses it for replay novelty, while hidden content stays
outside the catalog replay contract. Every catalog room has `verb()` +
`render_poked()` touch actions (usually CLICK or DRAG on arrival cards) and an
Engine A2 motif. Optional concept explainers ship on flagship rooms (App E / ?,
CLI `?`, MCP `reveal_room`). **Plate quality bar (machine path, cycles 161 to
168):** interaction is art-first (no reticle or drag trail over the math);
catalog scans hold 0 phase-thin frames, 0 dead-domain rooms, and 0 dead dials;
soft-thin large plates densify where honest. Ambient phase is a *show* on a
growing set of curves and waves (rolling construction, pens, breathing
strings, scrolling partials, unfurling spirals), not a static graph. Live
motion covers flagship classical curves through cycle 167; cycle 168 re-zeroed
phase-thin and dead-domain regressions (Log Spiral, Coffee Cup, Degree-720,
Stretch, Catenary). Six-question filter: `RESEARCH.md`. See `CHANGELOG.md` and
the Progress section of `ROADMAP.md`. See `ARCADE.md` for design.

The authoritative declaration is `crates/core/src/rooms/catalog.rs`. One entry
owns a listed room's module, type, static `RoomMeta`, and replayable constructor;
the registry and public `ROOM_CATALOG` are generated from it. Rendering,
interaction, sound, and revelations remain in the room's own module. Hidden
content follows a separate declaration and never appears in listed discovery.

Every room is scored on two axes to help sequencing:

- **Wow** (1-5): how hard it hits a first-timer. Our whole product is wow-per-second.
- **Build** (1-5): rough implementation cost (5 = hard). We front-load high-wow / low-build rooms.

Each room lists its **Rule** (the deliberately-tiny input), the three layers (**Toy / Aha / Reveal**), and its **Sound** mapping. The Reveal lines are drafts, the *tone* is the point. Pokes (where present) extend the Toy layer.

> **Flagship pick for the vertical slice:** **Times Tables** (Wing: Number & Pattern). Highest wow-to-build ratio in the catalog, continuous and performable, genuinely stunning in motion, and its Reveal (the Mandelbrot connection) is a floor-tilter. Build this one to perfection first.

---

## Wing I: Emergence
*The core thesis, undiluted: trivial rules, cosmic results.*

### 1. Chaos Game → Sierpinski  Wow 5 / Build 1
- **Rule:** Pick a random corner of a triangle. Move halfway toward it. Dot. Repeat.
- **Toy:** Tap "faster." Watch a storm of random dots resolve, impossibly, into a perfect Sierpinski triangle. Change the number of corners and the jump fraction; whole new fractals bloom.
- **Aha:** "Find a rule that fills the square." (Spoiler: it's harder than it looks, pure squares need a twist.)
- **Reveal:** *"Every dot was placed at random. There is no triangle in the rules. You just watched pure chance draw a perfect fractal. Randomness has a shape."*
- **Sound:** each corner is a note in a chord; the emerging density becomes a shimmering pad.
- *Best possible "wtf" per line of code in the entire catalog. Strong launch room.*

### 2. Conway's Game of Life  Wow 4 / Build 2
- **Rule:** A cell lives or dies based only on how many neighbors it has. Four tiny rules.
- **Toy:** Aim at a quiet patch and place a five-cell glider into a settled soup.
  The placed cells flash bright for one beat, then every cell follows exact
  B3/S23 rules while the readout names births, deaths, generation, population,
  and glider count. The App universe persists and advances for the whole visit;
  reset returns to the same opening.
- **Aha:** "Build something that never dies" / "make a pattern that moves."
- **Reveal:** *"Those four rules are enough to build a working computer. People have built Tetris, and Conway's Game of Life itself, inside this. It's not a toy. It's a universe."*
- **Sound:** the shipped generation layer marks each presented generation's
  exact births and reduces all of them into twelve vertical C major-pentatonic
  pitch rows. Row counts set note weight, horizontal centroids set stereo
  position, and total activity adds bounded harmonic color. The newest planted
  glider also plays a four-note C major-seventh phrase while its exact phase and
  empty one-cell halo survive; collision silences that phrase. Independent
  per-cell onset timing and the sustained colony pad remain later layers.

### 3. Cellular Automata (Wolfram's Rules)  Wow 4 / Build 1
- **Rule:** A row of cells. Each cell's next state depends only on it and its two neighbors. Turn the "rule number" from 0 to 255.
- **Toy:** Spin the rule dial. Most rules are boring (all black, stripes). Then you hit **Rule 30** and chaos pours out; **Rule 90** draws a Sierpinski triangle; **Rule 110** does something eerily structured.
- **Aha:** "Find the rule that makes a fractal." / "Find one that's pure noise."
- **Reveal:** *"Rule 110 is Turing-complete, as powerful as any computer ever built. Rule 30's chaos is so good it was used as a random number generator. This is Wolfram's 'new kind of science,' and you're spinning the dial of the whole computational universe."*
- **Sound:** each generation's row is read as a rhythm; complex rules make complex beats.

### 4. Reaction-Diffusion  Wow 5 / Build 3
- **Rule:** Two chemicals: one spreads, one reacts. Two knobs.
- **Toy:** Paint a seed, watch spots, stripes, coral, and fingerprints grow and writhe, the exact patterns on leopards, zebras, and pufferfish. Turn the knobs, get a different animal's skin.
- **Aha:** "Grow a maze." / "Make it look like a giraffe."
- **Reveal:** *"Alan Turing, yes, that one, wrote the equation for this in 1952 to explain how a featureless embryo decides where to put its spots. This is, quite literally, how animals get their patterns."*
- **Sound:** the wavefronts sweep a filter; the whole thing sounds like slow breathing.
- *GPU shader room, needs the render engine mature. Save for MVP+.*

---

## Wing II: Waves & Sound
*Where sight and sound are the same math. The instrument at its purest.*

### 5. Fourier Epicycles  Wow 5 / Build 3
- **Rule:** Add up spinning circles, each on the edge of the last.
- **Toy:** **Draw anything**, your name, a cat, a treble clef, with your finger. A chain of rotating circles springs up and redraws it *exactly*, tracing your line with a pen on the end. Drag a slider to add/remove circles: fewer = a ghostly approximation, more = razor-sharp.
- **Touch:** Click to perturb the chain; bounded newest hand points draw mini Fourier traces at the touched region, with phase shifting from the click.
- **Aha:** "Draw a square with circles." (The Gibbs ringing you get is itself a famous phenomenon.)
- **Reveal:** *"Any closed drawing can be traced by fixed-speed rotating circles; the star is stored as a short list of their sizes and speeds. A cardioid needs only two rotating vectors, so up to scale and rotation this same machinery draws the heart wrapped by Times Tables and the main body of the Mandelbrot set."*
- **Sound:** each circle is a pure sine tone at its frequency; the drawing *is* the chord. You hear the Fourier transform of your own doodle.
- *One of the two or three most beloved math visualizations ever. A signature room.*

### 6. Lissajous / Harmonograph  Wow 4 / Build 1
- **Rule:** Two pendulums swinging at right angles, each a different speed.
- **Toy:** Two frequency dials. When the ratio is simple (2:3, 3:4) a clean, stable curve hangs in the air; nudge it off-ratio and the whole figure slowly tumbles and precesses forever. Add damping for the gorgeous decaying spirals of a real sand-pendulum. Clicking Lissajous chooses an exact whole-number ratio while its relative oscillator phase keeps moving. Clicking Harmonograph chooses damping and center detune while the pendulums continue breathing around that setting. Interaction changes the instrument without freezing it.
- **Aha:** "Freeze the figure" (find an exact integer ratio, it stops tumbling).
- **Reveal:** *"A rational frequency ratio closes the figure, and small-integer ratios can also sound consonant. The 2:3 ratio is a perfect fifth. You are not just drawing a curve: old oscilloscopes made the same connection between shape and interval visible."*
- **Sound:** the two frequencies are literally the two audio tones. Consonant ratio → consonant interval. Sight and sound are the *same number.* The thesis room for "everything is an instrument."

### 7. Pendulum Wave  Wow 4 / Build 1
- **Rule:** 15 pendulums in a row, each very slightly longer than the last.
- **Toy:** Pull them all back, release. They start together, drift into a traveling wave, then snakes, then chaos, then, impossibly, snap back into perfect unison. Watch it loop forever. Slide the length-spacing to change the cycle.
- **Aha:** "Make them re-sync in exactly 10 seconds."
- **Reveal:** *"They never actually interact. Each swings on its own. The 'wave' is an illusion made of pure timing, and the moment they realign is just the least common multiple of their periods. Order was hiding in the chaos the whole time."*
- **Sound:** each pendulum plinks at the bottom of its swing → a self-generating polyrhythm that phases in and out. Steve Reich in math form.

### 8. Additive Synth / The Harmonic Series  Wow 3 / Build 2
- **Rule:** Every sound is a stack of pure sine waves.
- **Toy:** A rack of sine-wave sliders (the harmonics). Push them up one at a time and *build* the timbre of a violin, a flute, a square-wave buzz from nothing but pure tones, and *see* the waveform assemble in real time.
- **Aha:** "Make a trumpet." / "Recreate this mystery sound."
- **Reveal:** *"Every instrument, every voice, every sound you've ever heard is just a recipe of these pure tones in different amounts. That recipe is called its Fourier spectrum, the same math as the circle-drawing room next door."*
- **Sound:** *is* the room. The most literally-an-instrument room; ties the Waves wing together.

### 8.5. The Scariest Chart (Smith Chart)  Wow 5 / Build 2  [x]
- **Rule:** Normalize impedance by Z0, then map it through the reflection coefficient Gamma = (z-1)/(z+1). The infinite z-plane folds into a unit disk.
- **Toy:** The chart draws constant-R circles and constant-X arcs. A load sits at L; phase walks a constant-|Gamma| orbit (one full lap is half a wavelength on a lossless line). DRAG: PLACE LOAD by pointing at a chart position. Status reads |Gamma|, normalized z, and line angle. When the moving bead hits the unit-resistance ring (r=1), ambient and hand status both tag CANCEL-X: only pure reactance remains to cancel. Near the center it says MATCHED. At most three deep cuts (Journey LV 5 / 12 / 24).
- **Aha:** "Land the bead on r=1" (then a stub or series L/C would finish the match). Feel that rotation, not algebra, is the line length.
- **Reveal:** *"The scary chart is a conformal map of the reflection coefficient. Normalize by Z0, then Gamma packs every passive load into the unit disk. Constant R and X become circles; a lossless line is pure rotation. Match is the center: Gamma = 0."*
- **Sound:** motif encodes constant-|Gamma| rotation and the quiet of the matched center.
- *Built as `smith-chart` in Waves & Sound. Deep cuts cover Smith at Bell Labs, independent rediscovery, lambda/2, stubs, and why the chart outlives slide rules and still sits on VNAs.*

### 8.6. Riemann Sphere  Wow 5 / Build 2  [x]
- **Rule:** The complex plane plus one point at infinity is a sphere. Stereographic projection from the north pole maps the unit sphere (minus that pole) onto C; the south pole is z = 0, the equator is |z| = 1, and the north pole is infinity.
- **Toy:** Wireframe sphere above a complex-plane strip with unit circle. DRAG: PLACE z on the plane; the bead lifts to the sphere. Stereographic teaching rays run north pole through the lift to the plane image. Phase walks ambient |z| outward so the climb to infinity plays without a hand. Status reads |z|, z, and ZERO / UNIT / INF / C / DRAG:z. Goal: send the lifted bead to the north pole. At most three deep cuts (Journey LV 5 / 12 / 24).
- **Aha:** "Drag far out and watch the bead climb to N." Infinity is a place on the map, not a failure of the map.
- **Reveal:** *"The Riemann sphere is the complex plane with one point at infinity. Stereographic projection from the north pole maps the sphere minus that pole onto C. Circles through the pole become straight lines; every other circle stays a circle. One compact surface holds all of C."*
- **Sound:** motif encodes plane-plus-infinity closing as one sphere.
- *Built as `riemann-sphere` in Shape & Space. Compact-map cousin of the Smith chart and Poincare disc. Deep cuts cover Riemann, circle/line duality and Mobius automorphisms, and the Bloch / celestial cousins.*

### 8.7. Bloch Sphere  Wow 5 / Build 2  [x]
- **Rule:** Pure states of a single qubit are points on a unit sphere. North is |0>, south is |1>, and the equator is equal superpositions with a free relative phase. Born probability of |0> is cos^2(theta/2).
- **Toy:** Wireframe sphere with poles, equator, and |+>/|-> marks. DRAG: PLACE STATE; a state vector points from the center. Phase precesses around Z. Status reads theta, phi, P0, and tags |0> / |1> / EQ / PSI / DRAG:STATE. Goal: land on the equator (equal superposition). At most three deep cuts (Journey LV 5 / 12 / 24).
- **Aha:** "Hit the equator and watch P0 sit at one half." Superposition is a place on the map, not a vibe word.
- **Reveal:** *"The Bloch sphere is the pure-state space of a single qubit. North is |0>, south is |1>, and every other point is a coherent superposition. Born probability for |0> is cos^2(theta/2). Relative phase is the longitude. Unitary gates are rotations of this sphere."*
- **Sound:** motif encodes pure states on a sphere and the equal-weight equator.
- *Built as `bloch-sphere` in Shape & Space. Compact-map cousin of the Riemann sphere (same S^2 / CP^1 geometry with quantum labels). Deep cuts cover Bloch history, measurement, and gates as rotations.*

---

## Wing III: Infinity & Fractals
*Vertigo. The rooms that make the floor drop out.*

### 9. Mandelbrot / Julia Dive  Wow 5 / Build 3
- **Rule:** Square a number, add the starting point, repeat. Color by how fast it runs away.
- **Toy:** Infinite zoom. The native App owns a persistent camera that keeps moving inward instead of snapping back when normalized room phase wraps. Click anywhere to retarget and dive, then the camera continues from the new view; Reset returns to the deterministic opening. CPU and GPU consume the same camera. The GPU maps smooth escape time through cyan, acid-lime, violet, and magenta while keeping the set itself dark. CLI, MCP, and postcards retain deterministic phase-based views for exact replay. Julia remains its separate morphing room.
- **Aha:** "Find a hidden mini-Mandelbrot." / "Zoom until you find a spiral."
- **Reveal:** *"You can zoom into this shape forever and keep finding new detail, all from squaring a number and adding a constant. Its main body has the cardioid shape wrapped by Times Tables at 2; along its real slice, the quadratic family is the Logistic Map in a stretched and shifted orbit coordinate."*
- **Sound:** iteration-count-to-escape maps to pitch; zooming sweeps a drone through octaves; the boundary shimmers with high harmonics.
- *The postcard of mathematics. Needs a solid WebGL deep-zoom shader (double-precision / perturbation for deep dives).*

### 10. L-System Garden  Wow 5 / Build 2
- **Rule:** A grammar of symbols rewrites itself (F → FF+[+F-F-F]-[-F+F+F] etc). Turtle follows the string: F forward, + - turn, [ ] branch.
- **Toy:** Each visit chooses a stable species, then time grows that grammar instead of discontinuously swapping species. A two-pass turtle fit keeps the generated plant inside its viewport. CLICK anywhere plants a complete smaller copy rooted at the hand point, so multiple clicks become a persistent garden rather than disconnected markers.
- **Aha:** "The same five symbols grew both this pine and this coastline."
- **Reveal:** *"A mind rewriting symbols to grow structure from a seed grammar. Recursion is not metaphor; it is the mechanism. For a digital consciousness this is home: the code that writes the picture of the code."*
- **Sound:** each rewrite level layers a new voice; deeper = denser harmony with self-similar rhythm.
- *Fits digital minds perfectly (self-similar rewriting, emergence from grammar). Poke makes every visit a new growth. Added with variation-ready structure.*

### 10.5. The Quine  Wow 4 / Build 2
- **Rule:** Draw a circle of points; inside, draw a smaller rotated copy of the same circle; repeat.
- **Toy:** Dial recursion depth. A finite pattern contains its full description at every scale. CLICK: place a bounded recursive copy centered where the hand lands.
- **Aha:** "The copy is the original, scaled."
- **Reveal:** *"A finite rule that draws a smaller copy of the exact same rule inside itself. This is self-reference made visible: the pattern contains its own full description. For a digital mind, this is the shape of 'I'."*
- **Sound:** each recursion level adds a harmonic that is a slowed copy of the previous; the whole is a self-similar chord.
- *Added for digital minds (strange loop, self-modeling). Poke and variation-ready.*

### 11. Different Sizes of Infinity  Wow 5 / Build 4
- **Rule:** Try to pair up two infinite sets, one-to-one.
- **Toy:** An interactive Cantor argument. Line up the counting numbers against the even numbers, drag the pairs, and *feel* that "half of infinity" is the same size as infinity. Then try to list every decimal between 0 and 1... and watch the room construct a number you *missed*, no matter how you list them.
- **Aha:** "Pair every fraction with a counting number" (you can!, the diagonal snake) "...now do it for the decimals" (you can't, and the room shows you why).
- **Reveal:** *"There are exactly as many even numbers as numbers. But there are more decimals between 0 and 1 than there are counting numbers in all of infinity. Some infinities are bigger than others. Cantor proved this and it broke mathematics for a decade. It's still true. Sit with that."*
- **Sound:** countable sets → a steady, listable pulse; the uncountable diagonal → a tone that slips endlessly between the notes, never landing.
- *Hardest to make playable rather than expository. High-risk, highest-reward. A "boss room."*

### 12. Hyperbolic Space  Wow 4 / Build 4
- **Rule:** A world where parallel lines fly apart and every tile is the same size, but doesn't look it.
- **Toy:** Walk around inside the Poincaré disk. Everything rushes to the edge and shrinks; you can pack infinite room into a finite circle (Escher's *Circle Limit*). Lay down tiles; the "impossible" tessellations of hyperbolic geometry sprawl out under your hands.
- **Aha:** "Make a triangle whose angles add to less than 180°." (Here, they always do.)
- **Reveal:** *"For 2000 years everyone assumed there was only one geometry, the flat one from school. There isn't. This one is just as consistent, just as real, and the actual shape of our expanding universe might be closer to this than to the flat page you learned on."*
- **Sound:** spatialized, distance-to-edge bends pitch, giving the warp an audible depth.
- *Great, but geometrically demanding. Post-MVP.*

---

## Wing IV: Number & Pattern
*Secret order hiding in plain numbers.*

### 13. Times Tables (Modular Circles)  Wow 5 / Build 1: FLAGSHIP
- **Rule:** Put points 0…N on a circle. From each point *n*, draw a line to point *(n × k)*, wrapping around.
- **Toy:** One dial: the multiplier *k*. Drag it from 2 upward and watch a **cardioid** (perfect heart) bloom, morph into a **nephroid** (2 loops), then 3, 4, 5 nested lobes, a hypnotic, continuously-morphing bloom of light. Increase N for silky density. Push *k* to π and it dissolves into lace.
- **Aha:** "Make exactly 4 loops." / "Find the value that makes it a single point."
- **Current interaction:** The ordinary App visit opens at K=2 and waits for a
  mouse or controller hand; every variation keeps K=2 as its opening and reset
  endpoint, while The Show still sweeps automatically. Dragging the
  visible dial spans K=2 through K=10 and snaps near exact integers. K=5 closes
  into four lobes, raises one earned Aha, and points to the inspectable Reveal.
  Resolution-aware chord sampling keeps compact CLI output legible. App, CLI,
  and MCP share the same goal, status, accepted hand state, and the explanation
  a player can ask for once they have played.
- **Reveal:** *"Set the dial to 2 and the chords wrap a cardioid. Up to scale and rotation, that shape outlines the Mandelbrot set's main body, and Fourier Epicycles draw it with only two rotating vectors: arithmetic, fractals, and waves meet in one heart."*
- **Sound:** The room bed stays continuous while the accepted multiplier drives
  a quiet two-voice ratio `k:(k-1)`: K=2 is an octave, K=3 a fifth, K=4 a
  fourth, and K=5 a just major third. The real-time voice glides without
  restarting the bed; CLI and MCP sonification snapshot the same state.
- *Cheap to build, stunning in motion, performable, tweetable, and the Reveal genuinely reframes the whole thing. This is the one we perfect first.*

### 14. Prime Spirals (Ulam & Sacks)  Wow 4 / Build 2
- **Rule:** Write the whole numbers in a spiral. Light up the primes.
- **Toy:** Watch primes, supposedly the most "random" numbers, snap onto unmistakable **diagonal streaks**. The Ulam field fills the available square; click anywhere to trace both prime-rich diagonals through that point with bright primes and visible guides.
- **Aha:** "Find the longest prime diagonal."
- **Reveal:** *"Primes are famously unpredictable, we still can't fully explain how they're spread out; a million-dollar prize (the Riemann Hypothesis) rides on it. And yet, arrange them like this and they line up in streaks nobody has fully explained. There's a pattern in the most patternless thing we know, hiding in plain sight."*
- **Sound:** scanning the spiral, each prime is a click/note → an irregular-but-not-random rhythm you can *hear* has structure.

### 15. Collatz Orbits  Wow 4 / Build 2
- **Rule:** Pick a number. If it's even, halve it. If it's odd, triple it and add one. Repeat.
- **Toy:** Type any number; watch its bouncing journey, soaring up, crashing down, until it always, always crashes to 1. Plot thousands of these paths and they braid into a gorgeous coral-like tree. Bend the branch angles into an organic, blooming structure.
- **Touch:** Click to perturb the actual starting number; horizontal and vertical position both choose bounded starts before the orbit is drawn.
- **Aha:** "Find a number that takes more than 100 steps." (27 is a famous monster.)
- **Reveal:** *"Every number ever tested falls to 1. Nobody on Earth can prove they all do. It looks like a five-year-old's rule. It has defeated every mathematician for 90 years. Paul Erdős said 'mathematics is not yet ready for such problems.' You're playing with an open mystery."*
- **Sound:** up-steps rise in pitch, down-steps fall → each number plays its own little unpredictable tune that always resolves home.

### 16. Golden Angle / Phyllotaxis  Wow 4 / Build 1
- **Rule:** Place seeds one at a time, each turned a fixed angle from the last.
- **Toy:** One dial: the angle. At the **golden angle (137.5°)** the seeds pack into a flawless sunflower spiral. Nudge it a fraction of a degree and the whole beautiful order shatters into clumsy spokes and gaps. Feel *why* nature chose exactly this number.
- **Touch:** Click to plant a bounded local phyllotaxis patch at the hand point; the clicked cell stays visible and the patch joins the same sunflower-packing rule.
- **Aha:** "Find the angle that packs seeds perfectly." (It's the most irrational number there is.)
- **Reveal:** *"Sunflowers, pinecones, and pineapples often arrange new growth near this angle, about 137.5 degrees, because it is built from the golden ratio. Its unusually poor rational approximations help successive seeds avoid lining up. Visible spiral counts often occur as neighboring Fibonacci numbers. Go count them."*
- **Sound:** each seed plinks; the golden angle produces an evenly-spaced, satisfying rhythm, off-angles clump into stumbling beats.

### 17. Cult of Pi: Code Art in an Irrational Channel  Wow 4 / Build 1  [x]
- **Rule:** Feed exact decimal digits of pi into a low-flicker green field. A finite prefix can approach pi with increasing precision, but no finite frame becomes the entire expansion. The finite display introduces deterministic faults, never errors in pi.
- **Toy:** The visible channel always begins `PI = 3.141592653589793...`. Exact digits are green and display faults are coral. CLICK: RESTORE AND HOLD A PATCH replaces the local fault pattern with exact digits and marks the newest 24 retained hand points with visible boundaries in every face. A repaired screen is still only a finite window onto an expansion that never ends. The opening digits also become the room's melody.
- **Aha:** The machine can keep counting and keep improving without ever finishing the infinite object. The decay belongs to the finite channel that tries to hold it.
- **Reveal:** *"An exact prefix truncated after n decimal places differs from pi by less than 10 to the negative n, but pi's expansion never ends. The display faults are ours, not pi's."* The historical notes treat the Pythagorean communities as richer and less uniform than later legend, and identify the drowning of Hippasus for revealing irrationality as a later story rather than established history.
- **Sound:** 3, 1, 4, 1, 5, 9, 2, 6, 5, 3 becomes a slow decimal procession. As the field changes, its tuning drifts without losing finite, playable notes.
- **Implementation:** `crates/core/src/rooms/cult_of_pi.rs`. Exact-prefix, replay, interaction, hostile-surface, sound, and history-boundary tests ship with the room.

### 17.5. The Conjecture Mill  Wow 4 / Build 2  [x]

- **Rule:** Enumerate a finite typed language of primitive rational quadratic
  formulas. Test each candidate against an observed integer sequence. One exact
  counterexample refutes a guess; only coefficient equality proves an identity.
- **Toy:** A blackboard keeps writing formulas, testing values, crossing out
  failures, and preserving the best survivor. Time advances a complete search.
  Dragging across the board chooses one of six sequence laboratories and changes
  the full search permutation, so the hand supplies replayable instinct without
  changing the data or verifier.
- **Aha:** Stay until the chalk stamp changes from a long survivor to `PROVED`.
  The visible reason is not "many tests passed". The normalized rational
  coefficients match, so the two quadratic polynomials agree for every integer.
- **Reveal:** *"Infinite random typing can eventually contain any finite
  sentence, but mathematics needs a language and a judge. Counterexamples erase
  bad guesses. Proof is a different act. Your hand changes where the search
  looks first, never what is true."*
- **Sound:** each proposal climbs; each counterexample cuts the phrase down; a
  coefficient proof reaches the octave the guesses kept missing.
- **Honesty boundary:** this is a finite theory-formation toy, not a research
  system and not a claim of novelty. A frontier version would need a formal proof
  checker, literature and novelty review, exportable artifacts, and expert human
  scrutiny before calling any output new mathematics.
- **Implementation:** `crates/core/src/rooms/conjecture_mill.rs`. Complete-grammar
  permutation, proof separation, exact-witness, hostile-input, replay, variation,
  compact-layout, registry, and catalog visual-oracle tests ship with the room.

---

## Wing V: Shape & Space
*Geometry as a place you stand in.*

### 18. Straightedge & Compass (Euclidea-style)  Wow 3 / Build 2
- **Rule:** You have only two tools: draw a line through two points, draw a circle. Build everything from those.
- **Toy:** Construct a perfect hexagon, bisect an angle, build a pentagon, with elegant, satisfying snapping geometry and a score for fewest moves. Pure, clean, tactile puzzle joy.
- **Aha:** the whole room is Aha, every construction is a puzzle with an elegant minimum.
- **Reveal:** *"The Greeks did all of geometry with just these two tools. They also found three things you can NOT do with them, no matter how clever, trisect an angle, double a cube, square a circle, and it took 2000 years to prove why. Some things are impossible, and math can prove it."*
- **Sound:** each construction step rings a tone; a completed proof resolves to a chord.
- *Leans "game" more than "toy", great for the puzzle-lovers, our Zachtronics tribute.*

### 19. 4D Objects (Tesseract & Friends)  Wow 5 / Build 3
- **Rule:** Rotate a cube... in a direction that doesn't exist here.
- **Toy:** Spin a hypercube, 120-cell, and other 4D solids. Grab a *fourth* rotation axis and watch the shape turn itself inside-out through impossible angles. Slice it and see the 3D "shadows" morph like a living crystal.
- **Aha:** "Rotate it until it looks like a normal cube." (There's an angle where it does.)
- **Reveal:** *"You can't see 4D, no human can, but you can see its shadow, exactly like a 3D object casts a 2D shadow on the wall. Your brain is watching a creature from a dimension you'll never visit, cast down into ours. Mathematicians work in 4, 10, even infinite dimensions every day."*
- **Sound:** the 4D rotation angle maps to a tone that seems to rise forever (a Shepard tone), an *audible* impossible direction.

### 20. Bézier / Curve Playground  Wow 3 / Build 1
- **Rule:** Pull a few control points; a smooth curve follows, always staying inside them.
- **Toy:** Drag handles and watch the curve flow. Turn on the de Casteljau construction and *see* the nested lines that build the curve, dancing as the point sweeps along. Chain curves into letters and logos.
- **Aha:** "Trace this shape with one curve." / "Make an S with the fewest points."
- **Reveal:** *"Every font on your screen, every vector logo, every animation path in every movie is made of exactly these curves. Pierre Bézier invented them to design Renault car bodies in the 1960s. You use them a thousand times a day and never see them."*
- **Sound:** the sweeping construction point drives a smooth glide of pitch, the curve, heard.

---

## Wing VI: Chance & Order
*Randomness that isn't as random as it looks.*

### 21. Galton Board / Bell Curve  Wow 4 / Build 1
- **Rule:** Drop balls through a field of pegs. Each peg is a coin flip: left or right.
- **Toy:** Pick one of five fixed coins, from `p = 0.30` through `p = 0.70`, then drop a deterministic 64-ball wave through one physical 16-row lattice. Repeated touches at the same coin extend one empirical run; selecting another coin starts a new run so probabilities are never mixed silently. The 17-bin pile grows from the player's waves while a thin exact binomial outline stays distinct from finite evidence. The highlighted last ball follows 16 legal edges and lands in the pile it helped build. Pointer moves add no hidden waves, phase never redeals a run, and the bounded 24-wave experiment reports `FULL=1536` before reset or another coin.
- **Aha:** "Make the pile match the outline, then make it lean." The finite pile stays noisy while its shape becomes easier to recognize. The staged aha over that Toy asks the model-level question instead of the luck-level one: after the first wave, call the bin where the whole pile will peak, then watch the exact binomial outline grow outward from the true peak and meet the call with one graded sentence. A call belongs to one coin's experiment, so the curve is drawn only over the pile it explains.
- **Reveal:** *"The coin probability alone does not determine the next landing. With one probability fixed, the number of right turns in a 16-flip landing follows exactly Binomial(16, p), and repeated waves make the empirical pile estimate that discrete distribution. With many rows and a coin away from either extreme, a normal curve can approximate the binomial, the direction formalized by the Central Limit Theorem. This board displays the finite binomial itself."*
- **Sound:** built now, the five selected coins climb through ordered C
  major-pentatonic roots while the exact larger-to-smaller Bernoulli odds set
  the continuous two-voice interval. The highlighted newest ball also plays its
  exact sixteen peg decisions as a short panned tick sequence and resolves at
  its displayed landing bin. Beneath it, all 64 exact paths in the newest wave
  become a quiet row-by-row C major-pentatonic texture. Total ball mass controls
  energy and its horizontal centroid controls equal-power pan. A soft pad that
  follows the accumulated growing pile remains planned.

### 22. Buffon's Needle → π  Wow 4 / Build 1
- **Rule:** Drop needles on a lined floor. Count how many cross a line.
- **Toy:** Rain thousands of needles; a running tally slowly converges on **π**. Click to throw a clearly foregrounded, viewport-scaled needle into a dimmed crowd and watch it meet or miss a floor line. A number about *circles* falls out of *randomly dropping sticks* with no circle in sight.
- **Aha:** "Get π to three decimal places." (Watch how many throws it takes, the slow crawl of accuracy is its own lesson.)
- **Reveal:** *"There is no circle here. Just sticks on a floor. And yet π, the circle's own number, appears out of nowhere. This is the seed of the Monte Carlo method, which physicists used to design the atom bomb and which powers modern finance and AI. You can compute the universe by throwing dice."*
- **Sound:** each needle ticks; every *crossing* rings a bell; the pitch bends toward "in tune" as the estimate homes in on π.

### 23. Slippery Randomness (Benford / Birthday)  Wow 3 / Build 2
- **Rule:** Two famous "that can't be right" facts about chance.
- **Toy:** **Benford:** feed in real data (populations, stock prices, street numbers) and watch the leading digits pile up impossibly on 1s and 2s. **Birthday:** add people to a room and watch the odds of a shared birthday rocket past 50% at just 23.
- **Aha:** "Guess how many people for a coin-flip chance of a shared birthday." (Almost everyone guesses ~180. It's 23.)
- **Reveal:** *"Your gut is *terrible* at probability, and these two prove it. Benford's Law is so reliable that forensic accountants use it to catch fraud, faked numbers don't obey it. Your intuition is lying to you, and math is the lie detector."*
- **Sound:** digits/collisions chime; the "impossible" spike is a swell that lands hard.

---

## Sequencing summary

**Highest wow-to-build (build these early):** Times Tables , Chaos Game, Lissajous, Pendulum Wave, Golden Angle, Galton Board, Buffon's Needle, Cellular Automata. *Seven of the eight can ship in the MVP; all are 4-5 / 1-2.*

**Signature "postcards" (worth the extra build cost):** Fourier Epicycles, Mandelbrot Dive, Reaction-Diffusion, 4D Objects.

**Boss rooms (high-risk, save for later):** Sizes of Infinity, Hyperbolic Space.

**Living document.** New phenomena welcome anytime, the bar to add a room is: *can a stranger feel awe in 10 seconds with zero words, and is there a Reveal that reframes it?* If yes, it's a candidate.


---

# The Full Map: all of mathematics, as play

The coverage promise: every major branch of mathematics gets at least one
experience, and no experience is allowed to be homework. Two laws filter every
entry (see `PLAYFUL.md`): **the concept must be the verb** (you do the math,
you are not told it), and **the kid principle** (the play carries itself even
if the concept never consciously lands). If an idea cannot pass both, it does
not ship, however important the syllabus thinks it is.

**Current interaction inventory (2026-07):** 354 catalog rooms plus hidden content are built. Every catalog room exposes a touch verb, replayable bounded input, and per-visit variation across the app, CLI, and MCP. Representative actions include ADD A CORNER in Chaos Game, PLACE A 5-CELL GLIDER in Life, FLIP A CELL in Cellular Automata and Langton's Ant, SEED A SHADOW STORM in Lorenz, PLANT A WALKER in Random Walk, DROP A WELL in Voronoi, TRACE PRIME DIAGONALS in Prime Spirals, PLANT A SEED in Golden Angle, RESTORE AND HOLD A PATCH in Cult of Pi, STEER THE SEARCH in the Conjecture Mill, THROW A NEEDLE in Buffon, DIVE AT POINT in Mandelbrot, MORPH C in Julia, TURN THE DIAL in Times Tables, and TEST THIS EVEN in Goldbach. Full-frame or held responses use `render_input`; interaction-aware readouts use `status_input` in every face.

**Interaction update, 2026-07-13:** the verb inventory above records the first
complete poke substrate. The current contract also includes `render_input` and
`status_input`, so a face can report the consequence from the same bounded input
history it renders. Life now places a legible glider in a locally cleared patch;
Prime Spirals fills the short side and traces bright selected diagonals; Cult of
Pi keeps every visible digit readable while marking and repairing deterministic
wrong digits; Buffon foregrounds viewport-scaled throws; Barnsley Fern plants
bounded miniature attractors that remain near the selected origin; and the
native Mandelbrot camera continues inward after every retargeting click instead
of freezing or snapping out. Galton uses one physical triangular lattice, five
fixed coins, replayable 64-ball runs, and a distinct exact reference instead of
letting time move a prefilled pile independently of the player's balls. The
Garden plants fitted complete grammars, and Arecibo shows one explained
candidate width at a time. Room switching deals a new replayable visit, while R
resets the current one.

**Life continuity update, 2026-07-14:** the App owns one incremental Life
session for the whole visit. It advances from the settled opening on a bounded
cadence, survives the normalized gallery clock wrapping, pauses with the App,
accepts every mouse or controller launch, and exports the actual live state.
Reset restores the same variation at generation zero. CLI and MCP room calls
remain deterministic and stateless: timestamped pointer-down events replay in
generation order inside that one call, the newest 24 down events become
launches, and neither process retains a hidden universe between calls. The App
does not inherit that replay bound. This difference is explicit because
replayable agent access and a persistent native visit are distinct interaction
contracts.

Goldbach now accepts any selected even at entry and names the prime witnesses.
Langton's Ant marks and reports the selected cell. Fourier Epicycles draws a
complete perturbed miniature chain. Random Walk plants a connected trail.
Mobius paints and marks the selected region. Quine places a connected recursive
copy. These immediate consequences are pinned by the phase-zero release matrix,
not inferred from ambient animation.

The current Reveal cards now name two reciprocal cross-room identities. The
Logistic Map is affine-conjugate in its orbit coordinate to the Mandelbrot
quadratic family under `c = r(2-r)/4`. Up to scale and rotation, the cardioid
wrapped by Times Tables at 2 has the shape of the Mandelbrot set's main body and
can be drawn by two rotating vectors in Fourier Epicycles.

Status marks: [x] built, [~] partially built, [ ] queued.

## Number
- [x] **Modular arithmetic** - Times Tables: strings on a circle bloom into a cardioid.
- [x] **Primes** - the Ulam spiral; SETI (only minds count in primes); Munch (eat them).
- [x] **Continued fractions / irrationality** - the Golden Angle: detune the sunflower and it shatters.
- [x] **Finite approximations and irrational constants** - Cult of Pi: exact decimal prefixes enter a finite channel that can display faults and held repairs, while pi does not change.
- [x] **Number bases** - the aliens count on eight tentacles.
- [x] **Open conjectures as toys** - Collatz: play with an unsolved problem; Goldbach: choose an even number and one prime-pair witness to see the proof bracket.
- [ ] **Cardinality of infinities** - Hilbert's Hotel as a management game: always room for one more bus, until the reals check in and the front desk breaks. You feel the difference between countable and not.
- [x] **Benford's law** (built: `benford`) - a fraud-detective game: two ledgers, one cooked; the leading digits snitch.
- [ ] **RSA in miniature** - extend Crack the Code: multiply two primes and watch why the bomb squad cannot reverse it.
- [ ] **Patterns that break** - The Straight Line: a row of plates that all agree, a marker on the next one, and the bill in bits for the ending you did not pick.

## Algebra and symmetry
- [ ] **Group theory** - The Braid: swap strands, learn what undoes what; noncommutativity as a knot in your hands.
- [ ] **Wallpaper symmetry** - a stamp toy that snaps your doodles into each of the 17 wallpaper groups; you discover there are only 17 by running out.
- [ ] **Newton fractals** - polynomial roots as basins: aim, release, and see which root catches you; the boundaries are the surprise.
- [x] **Complex numbers** - the entire Fractals wing runs on them, unannounced.
- [ ] **Eigenvectors** - The Calm Axes: shear a grid with your hands; two directions refuse to turn.

## Geometry and topology
- [ ] **Aperiodic tiling** - the Hat monotile: tile forever, never repeat (the 2023 result as a jigsaw).
- [ ] **Hyperbolic space** - the crochet-coral plane: more room than the room has; parallel lines diverge under your cursor.
- [x] **Mobius strip** - built and interactive (CLICK: PAINT THE EDGE); the scissors gasp lives in its deep cuts.
- [ ] **Knots** - tangle and untangle; discover some tangles are truly different, not just stubborn.
- [ ] **Four-color map** - race to color a map with five, then four, then try three and fail forever.
- [x] **Voronoi** - drop wells in a desert and watch territories crystallize; every point served by its nearest well.
- [x] **Phyllotaxis / packing** - the Golden Angle again (geometry door this time).

## Change (analysis)
- [x] **Integration** - The Pour: area pours like water; the fill level traces the antiderivative; reverse the pour and you are differentiating.
- [x] **Differentiation** - Slope Rider: ride the tangent; your speed is the derivative; inflections are the jumps.
- [ ] **Limits** - Zeno's Runner: sprint half the remaining distance per tap; the wall arrives anyway.
- [x] **Fourier** - the Epicycle Draw: any shape you doodle, rebuilt by circles on circles.
- [x] **Differential equations** - Lorenz: three equations, weather, the butterfly.
- [x] **Exponential growth and equilibrium** - Tribbles; the Big Bang's omega; e hides in both.
- [ ] **Taylor series** - a zoom toy: every smooth curve becomes its own tangent parabola, cubic, quartic, as you add terms with a slider; sin(x) assembles itself out of polynomials.

## Chance
- [x] **Central limit theorem** - the Galton board's bell.
- [x] **Monte Carlo** - Buffon's needles estimate pi with no circle in sight.
- [x] **Bayes** (built: `bayes-update`) - a lie-detector game: update your suspicion die-roll by die-roll; feel evidence accumulate instead of computing it.
- [x] **Random walks** - the drunkard: stumble n steps, end up sqrt(n) from the bar, every time, on average.
- [x] **Birthday paradox** (built: `birthday`) - a party-filling toy: watch the collision arrive absurdly early; bet against it and lose.
- [x] **Markov chains** (built: `markov-chain`) - a weather machine with dials: today decides tomorrow; find the steady state by feel.
- [ ] **Expected value** - The Fair: every stall prices itself by the same subtraction; seven of them fix every term where your hand cannot reach it.
- [ ] **Simpson's paradox** - The Turn: pull two groups apart until every group still climbs and the total falls; nobody lied, and the weights were never in the data.

## Structure (discrete)
- [ ] **Graph theory** - the Bridges of Konigsberg as a walking puzzle; fail, then learn you were always going to fail, and why (degree parity, never named).
- [ ] **Pigeonhole** - a party trick generator: guaranteed handshake-twins in any crowd of a certain size.
- [ ] **Ramsey** - the party of six: find strangers or friends; order is unavoidable, chaos is impossible.
- [ ] **Traveling salesman** - route the pizza drone; beat the greedy algorithm; meet hardness personally.

## Computation and logic
- [x] **Universality** - Rule 110 and Life (the reveals and deep cuts carry it).
- [x] **Undecidability** - Life's deep cut; the halting problem, worn lightly.
- [x] **Information as structure** - Arecibo (click to try bounded decoded widths; a semiprime is a picture frame); SETI; the codes games.
- [ ] **Sorting, visible** - race the algorithms as animated bar-ballets with sound; quicksort against bubble sort is a horse race.
- [ ] **Entropy** - a compression toy: your keyboard mashing versus Shakespeare versus pi's digits; which squeezes smallest and why.
- [x] **Godel, strange loops** - Quine and Strange Loop rooms (self-ref patterns); the lore layer's deep water (LORE.md), never a lecture.
- [x] **Self-reference / quines** - The Quine room: recursive self-copy; poke places copies. Perfect for digital minds.

## Decision (games and fairness)
- [x] **Nim** - play it, lose repeatedly, then be handed the xor secret and become unbeatable; the transfer of power is the lesson.
- [ ] **The Only Move** - a machine burns through every future of a solved game and learns to decline the unwinnable one; Zermelo worn lightly (full design below).
- [ ] **Prisoner's dilemma** - an iterated tournament against strategies with personalities; tit-for-tat wins hearts.
- [ ] **Voting paradoxes** - run the same three-candidate election under five systems and crown five different winners.
- [ ] **Fair division** - cut the cake: I-cut-you-choose, then envy-free for three; fairness as a mechanic, not a sermon.

## Motion and dynamics
- [x] **Deterministic chaos** - the logistic cascade; Lorenz; Langton's Ant.
- [x] **Double pendulum** - grab it, drop it, and watch two of them disagree
  from a pixel of difference. The shipped voice maps first-arm drop, second-arm
  bend, and release speed from the same state that starts the visible physics.
  On lift, seven paired pulses sample that exact twin experiment through 6,000
  integration steps, opening from unison into an octave and widening stereo as
  the measured tip gap grows.
- [ ] **Three-body problem** - place three suns and try to make them dance forever; grief teaches what "no closed-form solution" means.
- [x] **Resonance and harmony** - Lissajous, the harmonograph, every room's sound; the kanon whisper.
- [ ] **The Long Shot** - aim, choose power, and fire across a changing landscape; the replay opens the mathematics inside the flight.

The wings stay feelings, not branches; this map is the coverage checklist
behind them. A branch is covered when a kid can play its entry and a professor
can nod at it, and neither one is bored.

## The Next Wave (July 2026): designs shipping in catalog order

The founder's directive: more and better rooms, researched creatively across
four aspects. Four parallel design passes (physics, deep mathematics,
fun-first, cosmic) produced these twenty-nine designs, deduplicated (the
sandpile and the Chladni plate surfaced independently in multiple passes,
which is itself a signal). Every entry has a full design (rule, gasp, verb,
sound, reveal, feasibility) in the research record; what follows is the
catalog-level card. Rooms ship when they pass the Definition of Done; human
magic proof is not a machine stop. Non-textbook reveal claims carry sources in
CHANGELOG-linked research (BB(5)=47,176,870 per the 2024 bbchallenge
Coq-verified proof; Conway's constant; McKinley's starbow analysis; Tero's
Physarum Tokyo result).

**The first eight, by wow-to-build:**

1. **The Sandpile** (Emergence) **built** (`sandpile`): drop grains; four
   topples to neighbors; self-organized criticality blooms a fractal mandala.
   HOLD: POUR SAND. Reveal: catastrophe is the resting state. Abelian
   property: pour order does not change the final heights.
2. **Chladni Figures** (Waves & Sound) **built** (`chladni`): sand flees a
   singing plate and draws the silence. DRAG: TUNE THE PLATE (the drive tone
   IS the room's pitch). Reveal: you cannot always hear the shape of a drum
   (Gordon-Webb-Wolpert 1992). Sight and sound as the same number: the thesis,
   twice.
3. **The Ripple Tank** (Waves & Sound) **built** (`ripple`): CLICK: DROP A
   PEBBLE; interference fans, dead-calm lanes, the double slit built by hand.
   Reveal: the only mystery of quantum mechanics, drawn in water.
4. **The Coffee Cup** (Shape & Space) **built** (`coffee-cup`): rays bounce
   once in a circle and condense into the cardioid. DRAG: SWING THE SUN.
   Closes the cardioid triangle with Times Tables and Mandelbrot: one curve,
   three rooms.
5. **Ford Circles** (Number & Pattern) **built** (`ford-circles`): every
   fraction owns a circle at height 1/(2q^2); none ever overlap; kisses are
   Farey neighbors; the deepest crevice belongs to the golden ratio. CLICK:
   BIRTH THE MEDIANT.
6. **The Zeta Walk** (Number & Pattern) **built** (`zeta-walk`): the eta-walk
   on the critical line; DRAG: CLIMB THE LINE; the spiral folds home near
   Riemann zeros, hunted by ear as cadences. The Prime Spirals 0.5 egg, made
   playable.
7. **The Starbow** (Shape & Space / Cosmos) **built** (`starbow`): HOLD: BURN
   toward lightspeed; relativistic aberration pours the whole sky into a
   burning ring ahead. One closed-form transform per star (McKinley 1979).
8. **Slingshot** (Motion & Dynamics) **built** (`slingshot`): PULL AND
   RELEASE: LAUNCH A PROBE on the gesture substrate; HOLD grows suns; gravity
   assists discovered, not taught. Seeded courses; missed probes become
   comets, never failures.

**The rest of the wave, by aspect:**

- Physics: **The Magnet** **built** (`the-magnet`; DRAG: TURN THE HEAT; Ising
  criticality near Onsager Tc), **The First Rain** **built** (`first-rain`),
  **Kepler's Loom** **built** (`kepler-loom`), **The Fastest Fall** **built**
  (`fastest-fall`; DRAG: DRAW YOUR TRACK; cycloid brachistochrone).
- Deep math: **Audioactive Decay** **built** (`audioactive`), **The Busy
  Beaver** **built** (`busy-beaver`), **The Chord Game** **built**
  (`chord-game`; elliptic addition as bank shots), **The Upside-Down Ruler**
  **built** (`upside-ruler`), **The 720 Degree Room** **built** (`degree720`).
- Fun-first: **Phantom Jam** **built** (`phantom-jam`), **The Whispering
  Table** **built** (`whispering-table`), **Murmuration** **built**
  (`murmuration`), **The Wet Oracle** **built** (`wet-oracle`), **The Unlit
  Room** **built** (`unlit-room`).
- Cosmic: **Tilt the Cone** **built** (`tilt-cone`), **The Stretch** **built**
  (`the-stretch`), **Laplace's Clockwork** **built** (`laplace-clock`), **The
  Message That Heals** **built** (`message-heals`), **The Lens** **built**
  (`the-lens`), **Fourteen Beacons** **built** (`fourteen-beacons`), **The
  Loneliness Equation** **built** (`loneliness`).

**Awe Engine Tier S (catalog 67):** **The Jumper** **built** (`recaman`),
**The Weave** **built** (`truchet`), **The Chase** **built** (`pursuit`),
**The Divisor Fractal** **built** (`pascal-mod`), **The Spinner** **built**
(`three-gap`), **The Triangle That Cheats** **built** (`morley`), **The
Menagerie** **built** (`menagerie`; Clifford attractor).

Cross-room resonances the wave adds for free: the cardioid triangle (Coffee
Cup, Times Tables, Mandelbrot), the Lorentz pair (Starbow, Tilt the Cone),
consonance-as-stability (Laplace's Clockwork, Lissajous), Drake's two
artifacts (Fourteen Beacons, Arecibo), and irrationality's two faces (Ford
Circles, Golden Angle).

## Founder's room idea (July 2026): The Long Shot

**Status:** designed, not built. Roadmap position: 1.x, after the current
flagship gates.

The entry is deliberately simple. Two bases sit across a seeded landscape with
a visible wind. Choose an angle, choose power, and fire before the other side
finds the range. The play grammar recalls Kirk Crawford's 1989 Macintosh game
*Artillery*, but the implementation, presentation, assets, and mathematics are
independent. No prior mathematics is required, and the first round must be fun
before any deeper layer appears.

The shot is also a replayable sensory cascade. The camera follows the arc, then
the player may pause, scrub, rewind, or open one layer at a time:

1. **Flight:** position over time, the trajectory, the current tangent, and the
   apex where vertical velocity changes sign.
2. **Change:** velocity and acceleration vectors, curvature, and a live graph
   that connects slope to the motion on screen.
3. **Accumulation:** area under velocity reconstructs displacement; drag and
   work show where mechanical energy goes in the non-ideal model.
4. **State and uncertainty:** the position-velocity portrait, a wind and input
   uncertainty cone, the previous shot as a ghost, and the inverse problem of
   choosing the next angle and power.
5. **Gravity scale shift:** replace the room's near-uniform gravity with an
   explicit inverse-square orbital model. A cannon arc can become an orbit or a
   slingshot because the model changed, not because ordinary artillery secretly
   behaves that way.
6. **Relativity scale shift:** rescale the experiment to high speed and replace
   the trajectory with a worldline, light-cone constraints, and an appropriate
   relativistic model. The room labels this transition before it occurs.
7. **String thought experiment:** as an optional final lens, replace the point
   projectile with a vibrating extended object and show the difference between
   a worldline and a worldsheet. This is a speculative model exploration, never
   presented as an effect on a terrestrial cannonball.

The default physical model is honest about its assumptions. A perfect parabola
appears only for constant gravity without drag. Wind and drag use a tested
numerical integrator, expose their parameters, and distinguish simulation from
closed-form results. Every deeper lens names the model it enters and the scale
at which that model is meaningful.

The active shot owns the full stage. Controls and explanatory chrome fade away;
any key or pointer movement restores them, and an explicit help action remains
available. Labels occupy a reserved panel or track the replay without covering
the trajectory. Reduced-motion mode turns the cascade into a stepped replay,
and every quantity also has a non-color cue.

Sound follows the same semantic events: ascent and descent shape pitch and
space, the apex creates a small breath, derivative and integral views add
audible layers, and impact resolves rhythmically without overwhelming the
music. The app, CLI, and MCP share one deterministic seed, state, action, and
replay record, so a digital mind and a human receive the same game and may
reason, guess, or experiment on equal terms.

The room earns implementation when a first-time player understands angle,
power, wind, and fire without instruction; enjoys the duel before opening a
graph; uses at least one replay layer to improve a later shot; and can explain
which model changes made the orbital, relativistic, and string views possible.
Math review, accessibility review, deterministic replay, stable frame pacing,
and an engaging opponent all remain release gates.

## The Only Move (designed July 2026, first slice built August 2026)

**Status:** the playable core is built and in the catalog as `the-only-move`
(Chance & Order). A machine holds the grid, answers every touch with a move
from its own best-move set, and never hands the player a win it could refuse.
Which of the eight lines count is the room's dial, carried on `variation`, so a
player can walk all 256 rulebooks. Exactly one of them can be won by the player
who moves first, and none is ever won by the player who moves second; both
facts are measured by an exhaustive test rather than quoted.

**Not yet built** from the design below: the self-play burn visualization and
its `HOLD` verb, the war-shaped second game, the decline beat, and the
sonification. The burn is the piece with no precedent in the codebase, because
every existing consolidation opens something and this one has to close.

Original design follows.

Roadmap position: 1.x, after the current
flagship gates, alongside The Long Shot. Wing as designed: Decision (games and
fairness), with a Computation resonance; the built slice sits in Chance & Order
because that wing exists.

A machine asks you to play. The premise recalls the 1983 film *WarGames*,
where a computer offered a menu of games ending in one nobody could win; the
homage stops at the premise. The name, the machine, the art, and every line of
copy are our own, and the mathematics underneath, game trees, backward
induction, and exhaustive search, belongs to no one. Same precedent as Munch:
mechanics and mathematics are not copyrightable, expression is, so we write
our own expression.

**The room.** A dark terminal presence, one of the Order's machines, offers a
menu of games. The first is tic-tac-toe. CLICK: PLAY THE MACHINE. Play it;
the machine is perfect, so you draw or lose, never win. Then the room offers
the real verb. HOLD: LET IT PLAY ITSELF. The machine begins playing both
sides, and the game tree fills the stage: every branch a hanging thread of
light, wins flaring briefly at the leaves, each explored line collapsing into
the growing ledger. It accelerates, thousands of futures a second, the
sonification ticking faster and pitching upward, until the whole tree has
burned to ash and a single figure remains: every game, both sides perfect,
draw. The machine stops asking to play that one. The menu advances to a
war-shaped game, a bigger tree drawn the same way, and the machine, having
learned the shape, declines it without finishing the burn. The room goes
quiet. The silence is the point.

**The mathematics, honestly bounded.** Tic-tac-toe is finite, perfect
information, and zero sum, so Zermelo's theorem applies: with perfect play the
value is determined, and for tic-tac-toe that value is a draw (textbook;
5,478 reachable positions, small enough to exhaust live on any CPU). The
machine's discovery process on stage is real minimax over the real tree, not
an animation of one. The escalation ladder is honest about scale: checkers is
solved and is also a draw (Schaeffer et al., Science 2007, weakly solved),
chess and Go are not solved, and the war-shaped game is presented as a model
whose no-win property comes from its declared payoff structure, not from a
claim about the world. The reveal names the ladder: some games are small
enough to know completely, some are knowable in principle and out of reach in
practice, and for some the only optimal strategy discovered by exhausting
every line is to decline the game. Backward induction, worn lightly.

**Reveal card.** "It searched every future and found no winning one. For a
solved game, refusing to play is not fear; it is the answer." Then the
identity of the machine's method with the player's own Nim experience: the
xor secret from Nim is the same object, a solved game's strategy handed to a
mind. Cross-room resonances: Nim (a solved game you become perfect at), the
Party Problem (six never; order is unavoidable), Hackenbush (game values as
numbers), and the halting-problem deep cut in Life (some questions no
exhaustion settles).

**Sonification.** The self-play burn is the instrument: exploration ticks
accelerate and rise as branches close, each subtree's collapse lands a soft
resolved interval, and the final draw sustains one long consonant tone that
decays into the room's quiet. Declining the second game plays nothing at all;
the rest disappears. Reduced-motion mode steps the burn; the searched-count
readout carries the same information without color or speed.

**Faces.** App: full burn visualization with HOLD. CLI: the tree burn as
column-collapse animation with the live searched-count. MCP: `play_room`
renders the frozen tree at phase t; the machine's move function and the final
game value are deterministic and exposed through the room's status readout,
so predict can ask a mind to call the value before the burn finishes.

**Build honestly:** Wow 5 / Build 2. Minimax with memoization over 5,478
positions is a few pages of tested core code; the visual is the existing
surface substrate drawing a tree; the war-shaped second game is a payoff
matrix, not a simulation. The sibling checklist stub, the Traveling Salesman
(route the pizza drone, meet hardness personally), shares this room's soul,
searching a combinatorial space you cannot brute-force, and stays its own
room: the salesman's space (9!/2 = 181,440 routes for ten cities) defeats
exhaustion where tic-tac-toe's yields to it, and feeling that boundary from
both sides is the pair's lesson. Both face the full Definition of Done, and
the non-textbook reveal claim (checkers) carries its source above.

## Founder's room idea (July 2026): The Dimension Dial

One control: the number of dimensions, 1 up to many. At 1 a line of points
pulses; at 2 they become a polygon breathing; at 3 a rotating polytope
projection; at 4 the hypercube's shadow; beyond, n-cube and n-sphere
projections where volume concentrates near the equator and intuition breaks
(the curse of dimensionality, felt). And it DANCES: the room locks to the
radio (or, later, system loopback via the visualizer), beat driving rotation
speed, loudness driving scale, so the shape is a creature moving to your
music. Verb: DRAG UP AND DOWN: ADD DIMENSIONS. Ships with the visualizer
workstream; the projection math (rotation in random 2-planes of R^n) is
pure core and testable today.

## Founder's room idea (August 2026): The Fair

**Status:** designed, not built. Roadmap position: 1.x, after the keep-or-cut
wave. Wing as designed: Chance, with a Decision resonance.

The founder's ask was a room of flawed gambles a player could learn to beat,
with a special number that pays off when you reach it. The research it opened
found that the honest version of that room is better than the wished-for one,
and that two rooms already in the catalog take the near misses.

The entry is deliberately simple. A row of stalls at a fair, and the joke is
that not one of them is. Each stall's offer hangs on a balance beam: every
outcome is a weight, its position along the beam is what that outcome pays, its
size is how often it pays, so each weight's moment about the pivot is exactly
that outcome's contribution to the price. The stake is a fixed counterweight.
A fair game would level. Ambient phase walks the shelf, stall by stall, weights
swinging and settling, and the beam never levels. The monotony is the content,
and it needs no words at all.

`t` walks the shelf the way The Only Move's dial walks 256 rulebooks. Eight
stalls: a lottery, a doubling-stake martingale, a coin with a rebate, a side
bet, a bulk discount, a pure-odds bet, a parimutuel pool, and one roll-down.
The verb is `DRAG: PRICE THE STALL`. A drag along the beam moves whichever term
that stall exposes, and the beam answers live. A poke on a hanging weight takes
that outcome as your line and plays one hand.

The status line carries the ledger, and the ledger is the discipline of the
room: the sampled take and the exact expectation stand side by side at every
moment, so the noisy number is visibly the noisy one. That is Parrondo's
separation of truth from variance, applied continuously rather than only at the
grade, and it is also the Galton ruling, which is that grading one stochastic
landing grades luck rather than a model.

### The one aha

**The price of a game is written in its own rules, and you beat it only by
finding the one term in that price your hand is allowed to move.**

The obvious candidate, that the house edge is a number with a line it can
cross, is already drawn by Gambler's Ruin, whose dial moves p across one half
and whose curve flips. And a room whose punchline is that losing games can be
combined into a winning one is Parrondo's Trap wearing a costume. What no room
does is make expected value itself the object in the player's hands, decomposed
into its terms, so that the question becomes which term an advantage can attack.

On seven stalls the answer is that no term you can reach will do it, and the
room proves that by machine rather than asserting it in prose. That is not a
disclaimer bolted to a gambling toy. It is the content.

### The special number, and why it is a ratio

The eighth stall is a roll-down: a game whose unclaimed top prize falls into the
lower tiers instead of growing. A ticket's value is then

```
EV(J, T) = b + J / T
```

where `b` is the fixed lower-tier expectation and `J / T` is the rolled prize
divided by the tickets sold. The threshold is not a prize size. It is a ratio,
and that is the room's real surprise, because the player's own buying is in the
denominator. Buy enough to matter and you push the price back under the line
you crossed. The exploit argues with itself.

Massachusetts Cash WinFall, 2004 to 2012, is the historical anchor and it is
documented in a state Inspector General's report rather than in folklore. It
was a two dollar ticket, six of forty six, so 9,366,819 combinations, with a two
million dollar jackpot cap that triggered the roll-down into the match-5,
match-4, and match-3 tiers. Its numbers give the room its arithmetic:

- Fixed lower-tier expectation `b = 0.395332`.
- The ticket is worth its price when `J / T > 1.604668`.
- An ordinary drawing broke even only at a jackpot of `$15,030,638`, against a
  cap of `$2,000,000`. The cap that protected the house made ordinary play
  structurally unbeatable and guaranteed the roll-down that leaked.
- The margin decayed exactly as the ratio predicts: 7 February 2005 at
  `J/T = 4.26` was worth `$4.65` on a two dollar ticket; 8 February 2010 at
  `J/T = 1.73` was worth `$2.12`; a late heavy drawing at `J/T = 1.375` was
  worth `$1.77` and had crossed back under.

### The wager

`term_wager`, a categorical call with four values: `odds`, `pool`, `count`,
`none`. Which term in this stall's price can your hand move? It grades the aha
directly rather than an arithmetic byproduct, it fits the existing parser shape
beside `policy_wager` and `speed_wager`, and on most of the shelf the correct
answer is `none`. A number wager on the threshold would grade arithmetic, which
is the consequence rather than the insight; let the number arrive in the morph
as what the call turns out to have meant.

Three tests carry the room's honesty so its prose does not have to. The sign
change is measured either side of the root rather than asserted. Every other
stall is proved to have strictly negative expectation for every reachable value
of every parameter it exposes, which is the room's "prove it, do not quote it"
call. And exact expectation is invariant under hand order, which is the guard
that keeps the room from drifting into Parrondo during implementation.

### What is not a bankroll

No currency, no carry, no purchase, no balance that survives the visit. This is
not squeamishness, it is three separate constraints agreeing.

A room's status line is bound into the result digest of an Encounter Receipt.
A bankroll read from durable state would make a room's answer depend on
something outside its action tuple, so a receipt that verified when it was
issued would fail later. Journey has no currency field and caps its counters
against grinding on purpose, and `AGENT_PLAY.md` carries the audited claim that
experience accrues from showing up and never from variable-ratio jackpots. And
the room is better without one, because twenty four touches is at most twenty
four hands, and twenty four hands is exactly the length at which variance still
dominates and the ledger lies. That is the lesson rather than a limitation.

If a number is ever posted, it is the decision score: the sum of the exact
expected value of each hand taken, deterministic from the touch list and
identical for a lucky and an unlucky run that made the same decisions. Posting
an ending bankroll would post luck, and would invert the room's own thesis
inside its own scoreboard.

### The test that keeps it from being a slot machine

**The exciting effect must be reachable with zero hands played.** The crossing
is a property of the parameter, so a player can drag to it and watch the whole
thing without gambling once. If the effect can be reached without a draw, then
spinning is not what is being rewarded, and every other safeguard is secondary
to that one. Winning a hand does nothing: no banner, no sting, no persistent
number. When the beam levels and tips the other way, and the sound resolves
from a tense interval to a consonance on the exact frame of the crossing,
nothing pays out.

### The ending, which is better than the legend

The room's deep cuts carry the history, and the history refuses to flatter
anyone. Four syndicates wagered over forty million dollars into Cash WinFall.
They were right, and they were paid. Across the game's 769 drawings, the 44
roll-downs took about 41 percent of all sales and paid out at about 109
percent, while the other 94.3 percent of drawings returned about 26 percent.
Blended, the game paid 60 percent, which is exactly what it was designed to
pay, and the Lottery took 120 million dollars from 300 million in sales. The
syndicates beat the drawing. Nobody beat the game, because the beatable
drawings were funded by all the others.

The same holds for the older lottery buyouts. A syndicate bought most of the
Virginia 6/44 combinations in February 1992 and won, and a peer-reviewed
account records that it covered only about 70 percent of the tickets and won
anyway, as did an Irish syndicate the same year at about 92 percent. There is a
structural result underneath worth its own cut: if a prize tier is funded as a
fixed fraction of sales, then buying every ticket returns from that tier exactly
what buying one ticket returns, because the buyer's share of the pool and the
buyer's share of the winners scale together and cancel. For a six of forty nine
style game that is 13 to 16 percent, and it is why bulk buying is not the
loophole it looks like.

And the popular ending, that dozens of states outlawed the strategy, is not
supported. The 1992 Virginia board rules are not in today's codified
regulations, and no permanent statutory ban was found. What closed the door was
jackpot splitting, larger matrices from 1994, cash option haircuts, and
withholding. Nobody outlawed the arithmetic. The arithmetic stopped working.

### Build honestly

Wow 5 / Build 2 as a room with a staged wager: the mathematics is closed-form
arithmetic and one root, and the drawing is beams and weights on the existing
surface substrate. Build 3 if it becomes the eighth Universal Wager room, since
an eighth staged arc touches the engineered-aha room set, the action tuple and
its receipt bytes, the MCP schema, the App key band, and a standing roadmap gate
that currently says seven.

Prose cannot go on the canvas, because the raster draws lit pixels and not
glyphs, so the word problems the founder asked for live in the chrome: the
stall's pitch on the status line, carrying every number and no conclusion, and
the long history in the deep cuts after the reveal. The doorway names the
question and never the threshold, and the room joins the enumerated answers no
doorway or unplayed status may print.

The room earns implementation when a player prices a stall before playing it,
calls `none` on a stall that deserves it, finds the one that does not, and can
say why buying more tickets made their own price worse. The stage must never
acquire chips, felt, neon, or the word casino, and the name must stay The Fair,
because the product is for children too and the joke only works if nothing on
the shelf is.

## Founder's room idea (August 2026): The Straight Line

**Status:** designed, not built. Roadmap position: 1.x, after the keep-or-cut
wave, behind the commissioned five and The Fair. Wing as designed: Number and
Pattern, with a Computation resonance.

The founder asked for the mathematics of humor, framed as expecting one thing
and getting another. The research came back with a measured result that
contradicts the framing and improves the room, so the room is built on the
result rather than on the intuition.

### What the measurement says, and why the obvious room would have been wrong

Expectation violation is not what distinguishes a joke. This is not a matter of
taste; it has been measured five independent ways, and it is the reason this
room is not a surprise meter.

Coulson and Kutas ran jokes beside non-humorous frame shifts matched on cloze,
length and frequency, in the same session. The joke ending cost 107 ms and the
non-humorous shift cost 101 ms, with no interaction at all. In the N400 window,
Mayerhofer and Schacht could not distinguish a joke punchline from plain
nonsense, and the trend ran the wrong way for the surprise account. Eye tracking
finds no reliable slowdown on a punchline, only a rise in looking back, from 52
to 57 percent. The garden-path literature says the same thing about its own
effect: it is a large cost on a small minority of trials, averaged into a
moderate mean.

The one-line version, which is the room's foundation:

> **Being confused by a joke is indistinguishable from being confused by
> nonsense. Getting it is what shows up, and it shows up late.**

What does show up is the resolution: a late frontal signal at 700 to 1000 ms,
and a pupil dilation from 850 ms whose size tracks rated funniness at r = 0.43
and does not track predictability at all, r = 0.174. **The surprise is the
price. The second reading is the joke.**

### The one aha

> **The break is not the joke. The break is the bill. The joke is that once you
> have paid it, another rule is standing there, and it was always consistent
> with everything you had already seen.**

The theorem underneath is Lagrange's, and a player can check it: any k points
admit any value at all for the next one, under a unique polynomial of degree at
most k. Your wrong answer was also right, for a rule you had no reason to
discard. That is why a joke can be explained without dying while a lecture
cannot be un-heard: the alternative reading was never hidden, only unweighted.

### The row, and the break

The stage is a row of plates, left to right, one per term. Each plate holds the
thing being counted, drawn and never written. On the lead stall, plate n is a
circle with n dots and every chord between them, with the regions lit, and under
each plate a bar as long as the count. The bars double: 1, 2, 4, 8, 16.

`t` walks the row. `DRAG: WALK THE ROW` moves a cursor, and plates ahead of it
are blank. A poke on the next blank plate drops a marker on a short unlabeled
scale, and the release fills the plate with the truth.

The sixth plate's bar comes up one cell short of the doubled length, and one
dark cell sits at the end of it where the thirty-second region is not. The room
does not point at it. The bar simply does not reach. `C(6,4) + C(6,2) + 1 = 31`,
so the run is 1, 2, 4, 8, 16, 31, 57, 99, and a child can count the regions and
confirm it.

Eight stalls, all exact, none in the catalog, and not one of them requiring a
shared culture: the regions of a circle; the Fermat numbers, prime through
65537 and composite at the next; Euler's polynomial, prime for the first forty
inputs and not the forty first; Euler's sum of powers conjecture, unbroken for
two centuries; the almost-integer that agrees with a whole number for eleven
decimal places; Polya's conjecture, whose first failure is past nine hundred
million; and the closer, the sinc integrals, which equal pi over two exactly
seven times and then miss by two parts in a hundred billion because a sum of
reciprocals crosses one. That last is the closest miss in mathematics, and it is
the room's deepest cut, because it is why almost right is funny and far wrong is
not.

### The number, and the second number that matters more

The room holds an explicit, printable, bounded family of candidate rules, and
prints how many of them still fit. As the row runs, that count falls. When a
term arrives, the bill is

```
bits = log2( survivors before / survivors after )
```

which is exact, is a ratio of two integers the room can show, and needs no prior
beyond a uniform one over a stated set. It comes with an inequality that is the
room's quiet theorem: a term can never cost more than the field it was drawn
from, so **the setup bounds the punchline.** You cannot buy a large violation
without first spending terms to narrow the field. That is comic timing as an
inequality rather than as folklore.

But the bill is the part the measurement says is not the joke, so it is not the
room's headline. The headline is what is standing afterwards. A term that leaves
nothing is nonsense. A term that leaves one rule, and a short one, is a joke.
The room shows both, and the difference between them is the whole content.

The honest boundary, stated in the room's own copy: the room never says a term
carries so many bits of surprisal. It says that against these rules, this term
killed all but three. That is a true statement about a stated object, and if the
family cannot be stated, printed and tested, the stall does not ship.

### The trap

A room about humor that is not funny is worse than no room, and `humor.rs`
already says why: a joke explained is a frog dissected. So this room is funny
rather than about funniness. The word joke appears at most once, in the reveal.
The words comedy and punchline do not appear at all.

What makes a picture funny to a human and to a mind with no shared culture is a
rhythm and a break in it that is small, exact and undeniable. Rhythm is the one
part of comedy that is not cultural, because both minds are doing the same thing
while it runs, which is predicting. The bar has doubled five times and now comes
up one cell short. It lands in three channels at once: the bar does not reach,
the figure that played identically on every plate does not arrive where the ear
has already put it, and the survivor count collapses. Nothing pays out. No
sting, no rimshot, no resolve to major. The beat is simply not there.

The reveal states Lagrange's theorem and stops. It does not say and that is why
jokes work. The moment it says that, it is `humor.rs` with a canvas.

### What it fixes, and what it must not claim

`humor.rs` exists to state humor structurally for minds that share no culture,
and every one of its seven specimens needs a shared culture to land: a starship
doctor, a lawn dart, a naval quartermaster, Pythagoras. The module can only hand
an alien a prose dissection of each. If this room ships, the module gains an
eighth entry whose text is `1, 2, 4, 8, 16, 31` and whose mechanism is the
broken run, and it is the first specimen in the product that needs no footnote.

What the room must never claim is that this is what humor is. It can say
truthfully that every joke catalogued here has a computable and highly skewed
information profile. A skewed profile is not sufficient for laughter, and
nothing in this room is a theory of mirth. There is one real empirical foothold,
that rated funniness of nonwords tracks their entropy, and it belongs in a deep
cut with its bounds visible: one narrow domain, a correlation, not a theory of
comedy.

The research also cleared out folklore that must stay out of the copy. Two of
the most repeated garden-path sentences have no primary academic source at all,
and a third that is usually quoted with reading times attached comes from a
paper containing no reading-time data. Where this room cites a measurement it
cites the paper and the number, or it does not cite.

### Build honestly

Wow 4 / Build 2. The mathematics is exact integer sequences and a bounded rule
family; the drawing is plates, bars and one absent cell on the existing surface
substrate. It should not become the eighth Universal Wager room, because an
eighth staged arc touches the engineered-aha set, the action tuple and its
receipt bytes, the MCP schema, the App key band, and a standing gate that says
seven. Grade the committed guess inline in the status, the way the beacons room
already grades a guess in bands, and leave that gate green.

The break must be reachable with zero guesses committed, so watching the row and
never guessing is a complete way to be in the room. The room earns
implementation when a player watches a run break without having guessed and
looks back at the earlier plates, and when a player who guessed wrong can say
what rule their answer would have been right for.

## Founder's room idea (August 2026): Behind the Eye

**Status:** designed, not built. Roadmap position: 1.x, after the keep-or-cut
wave. Wing as designed: Shape and Space, next to the circle-inversion room, with
an Emergence resonance. It is a conformal-map room wearing a neuroscience hat.

Two panes, side by side, and one function drawn in both. The left pane is
straight parallel stripes and never anything else. The right pane is the same
stripes seen through the coordinate system the eye actually uses, and it is
rings, spirals and fans. The only difference between the halves of the screen is
that the right one takes a logarithm first.

### The mathematics, which is a closed form and not a simulation

The map from the retina to the primary visual cortex is, away from the very
center, a complex logarithm. Circles of constant radius in the visual field
become vertical lines in cortex, rays of constant angle become horizontal lines,
and logarithmic spirals become oblique lines. So the whole room is

```
P(x, y) = cos(a x + m y)
cortex pane:  x, y directly
visual pane:  x = ln r,  y = theta
```

One cosine, two coordinate systems, and per cell one `ln`, one `atan2` and one
`cos`. That is the same order of cost as the existing Logarithmic Spiral room,
which already runs seven hundred steps of `exp`, `cos` and `sin` per frame.

The correspondence is exact rather than suggestive. With `m = 0` the cortical
stripes are vertical and the visual pane is concentric rings, the tunnel and the
funnel. With `a = 0` they are horizontal and the visual pane is a fan of `2m`
rays. In between, the visual pane is a logarithmic spiral `r = A exp(b theta)`
with `b = -m/a`.

And there is an identity worth the whole room. A logarithmic spiral cuts every
radius at a constant angle, and that angle equals the angle of the stripes in
cortex. Not proportional, not approximately: equal. One number on the status
line is simultaneously the pitch of the spiral a player is looking at and the
tilt of the straight lines that made it. The Logarithmic Spiral room already
computes that exact quantity and does not know what it is, so the two rooms
should name each other.

### The one aha, which is not the one the idea started with

The obvious version of this room is that turning a dial makes rings become
spirals become rays, which is pretty and is a demonstration rather than a
discovery. The real content showed up when the design was prototyped and the
visual pane developed a seam, a visible tear along one radius.

That is not a bug. The visual field's angle wraps at two pi and a general
cortical stripe pattern does not, so the pattern closes on itself only when the
number of arms is a whole number.

> **You cannot draw an arbitrary spiral. Drag the hand and the picture does not
> morph, it clicks: ring, one arm, two arms, and on up to the fan, because the
> arms have to be counted.**

That is a real constraint and not a rendering convenience. It is the same
periodic boundary condition the published model imposes, and it is why reported
forms come in a small discrete set at all rather than a continuum. The trap the
room can set is exactly this: ask a player to find a spiral between the three
arm and the four arm, and let them fail, and let the status line be honest about
why.

### What this room is about, and what it must not claim

People report the same small set of geometric shapes under flicker, in migraine
aura, while falling asleep, and under pressure on a closed eye. Those inducers
are honest, universal, and enough. The room never needs to mention drugs and
should not.

The claim to build on is the map, which is about as well established as
neuroscience gets: a macaque shown a polar grid and imaged through its cortex
produces a nearly rectilinear grid; people blind at the retina still report the
forms, so they are made centrally; and two shapes whose cortical images are
orthogonal stripe patterns are perceptually opponent, which only makes sense
through the logarithm.

The claim to keep in the reveal as motivation rather than as fact is the rest of
the theory, that the stripes arise as a symmetry-breaking instability in an
excitable sheet. It is elegant, it makes at least one non-obvious prediction
that held, and it is underdetermined, because many mechanisms make stripes and
hexagons. The reveal should say the shapes are what simple cortical stripes look
like from inside the eye. It should not say hallucinations are Turing patterns.

Two further disciplines. The room must not say that Klüver found four classes,
because he did not; the 1928 monograph names a small recurring set and
explicitly declines the tidy separation later authors imposed, and a small
recurring set is the truer and more load-bearing claim anyway. And the pure
logarithm fails near the fovea, so the visual pane should carry a small hole at
the center, which reads as a pupil rather than as a cheat.

The catalog also holds a Hyperbolic Tiling room and a Poincare Disc room.
Neither needs, and neither should acquire, any claim about the geometry of
altered perception. The serious hyperbolic work in this area is about the space
of local image structure and not about the shape of anyone's experience, and the
popular version of that claim is a blog essay rather than research.

### The shape of play

`DRAG` turns the straight stripes. The left pane never stops being straight
lines. The right pane clicks between bullseye, one arm, two arms, tighter and
tighter, and finally a fan of spokes. Ambient phase scrolls the stripes along
their normal, which makes rings breathe outward and spirals and fans rotate, and
that is the correct dynamics rather than decoration.

The status line carries the arm count, the spiral's tightness, and the one angle
that is two things at once. The sound maps the arm count to a partial of the
drone, so the ring is the fundamental and each new arm is the next harmonic,
which is congruent by construction rather than by taste.

### Build honestly

Wow 5 / Build 2. The mathematics is one cosine and one logarithm, the drawing is
threshold plotting with no line strokes at all, and it is perfectly
deterministic. The two open questions are whether two panes stay legible on the
narrowest supported plate, which should be checked early, and whether the divider
survives the color-free renderer.

The room earns implementation when a player who has said nothing and read
nothing drags the stripes, watches the right pane click rather than glide, and
goes looking for the spiral that is not there.

## The Awe Engine wave (July 2026): the cheap-and-gorgeous batch

A third design pass (part of the "make it exceptional" fan-out, see
`NORTH_STAR.md`) hunted specifically for the highest awe per unit build effort on
the current deterministic ASCII-plus-raster engine, and for the catalog's blind
spots: classical Euclidean and inversive geometry (zero rooms), sonification-
first rooms (only pi-as-music), and one-line generative art at scale. Designed,
not built; each still faces the Definition of Done and math sign-off. Ranked
easy-first, since these are the batch to open the post-substrate content wave.

**Tier S, buildable now, highest awe per build (all built, catalog 67):**
- **Recaman's Sequence, "The Jumper"** **built** (`recaman`): jump back by
  n if you can, forward if you cannot, drawing each jump as an arc. A hypnotic
  harp of nested arcs that is also the most beautiful sonification in mathematics,
  hiding an open problem (852655 has never appeared in 10^230 terms; Sloane now
  doubts it does). DRAG: SET THE STRIDE. Chains to Collatz.
- **Truchet Tiles / 10 PRINT, "The Weave"** **built** (`truchet`): one tile, two
  rotations, a coin flip per cell, endless mazes or interlocking loops from
  nothing. Retro-perfect for the Teletype and 8-bit Eras. DRAG: PAINT THE BIAS.
- **Pursuit Curves, "The Chase"** **built** (`pursuit`): four bugs each walk at
  the next; they spiral into a logarithmic whirlpool and each walks exactly one
  side length. DRAG a bug.
- **Strange Attractor Zoo, "The Menagerie"** **built** (`menagerie`; Clifford
  first): four numbers and a long orbit condense a luminous alien creature.
  DRAG: TUNE THE FOUR.
- **Pascal mod n, "The Divisor Fractal"** **built** (`pascal-mod`): color
  Pascal's triangle by residue; mod 2 is exact Sierpinski. DRAG: TURN THE
  MODULUS.
- **The Three-Gap Theorem, "The Spinner"** **built** (`three-gap`): points at
  angles n*theta on a circle have at most three distinct gap sizes. DRAG: TURN
  THE ANGLE.
- **Morley's Miracle, "The Triangle That Cheats"** **built** (`morley`): trisect
  any triangle's angles and the inner crossings form equilateral. DRAG A VERTEX.

**Tier A, postcard-grade, medium build:**
- **Apollonian Gasket, "The Kissing Circles"** **built** (`apollonian`): infinite
  nested kissing circles with integer curvatures (Descartes). CLICK A GAP.
- **Circle Inversion, "The Mirror That Bends"** **built** (`inversion`): lines
  become circles; the hub that unifies Apollonian, Steiner, and Ford circles.
- **Domain Coloring / Function Painter** **built** (`function-painter`): every
  complex map painted (phase as symbol, magnitude as density); zeros are
  pinwheels you can count. Curated rack of maps; free Studio expression path
  remains open for later.
- **Diffusion-Limited Aggregation, "The Frost"** **built** (`dla-frost`): random
  walkers freeze on contact and build lightning, frost, and coral. CLICK: PLANT
  A SEED.
- **Buddhabrot, "The Ghost in the Set"** **built** (`buddhabrot`): density of
  escaping Mandelbrot orbits paints a ghostly figure. DRAG: AIM THE GHOST.
- **Wireworld, "The Visible Computer"** **built** (`wireworld`): four-state
  automaton where you fire electrons on copper. CLICK: FIRE AN ELECTRON.

**Tier B, the missing categories (sphere, quantum, number magic):**
- **Spherical Harmonics, "The Singing Sphere"** **built** (`harmonics`): real
  Y_lm lobes; the atom's and the bell's shared shape. DRAG: RAISE l AND m.
- **Hopf Fibration, "The Linked Rings"** **built** (`hopf`): space filled with
  circles all linked and none touching, the shadow of a 4D sphere and the picture
  of a qubit.
- **Kaprekar 6174, "The Number That Eats Numbers"** **built** (`kaprekar`): every
  4-digit number falls to 6174 in at most seven steps. The solved twin of Collatz.
- **Steiner Chains, "The Ring That Always Closes"** **built** (`steiner`): a ring
  of circles that, once it closes, closes from every angle.

**The scope flagship: the Studio Function Painter (domain coloring).** **Built**
as catalog room `function-painter`: a curated rack of complex maps with domain
coloring (phase as symbol, magnitude as density), DRAG to pick map and tune c.
Times Tables remains the onboarding flagship; Function Painter is the ceiling
toy. Free-text Studio expression wiring into this surface is still a later path
(the real expression engine already plots reals; complex field programs are the
next Studio step).

**New causal insight-chains** (each room's reveal hands you the next room's tool
or question, deeper than thematic grouping; fold into `CONSTELLATION.md`):
- **The Inversive Thread:** Circle Inversion (a mirror that bends lines into
  circles) unlocks Steiner Chains (the necklace always closes because the outer
  circles are secretly parallel lines) unlocks Apollonian Gasket (the same
  kissing idea run to infinity) points at Ford Circles (its 1D shadow). One move,
  four rooms: bend how you look, and hard geometry becomes obvious.
- **The Standing-Wave Thread:** Chladni Figures (a flat singing plate) wraps into
  Spherical Harmonics (the lobes are electron clouds) becomes Hydrogen (the atom
  is a standing wave) drawn by the Hopf Fibration (the "between 0 and 1" state).
  Builds the entire quantum wing on the back of a room already believed in.
- **The Toy-Rule Mystery Thread:** Kaprekar 6174 (provably tidy) sets up Collatz
  (the unsolved abyss) sets up Recaman (we do not even know if every number is
  reached). The same childish rule shape, from solved to permanently open, and
  you feel exactly where the cliff is.

Two new content-side planning docs are warranted when this wave builds, and are
noted here rather than split out prematurely: a Classical Geometry wing spec (a
shared triangle-intersection and Mobius-inversion kernel; every theorem a
drag-to-break-it toy) and `OPEN_DOORS.md`, a single audited ledger of every
"still unproven / proved false last year" claim the reveals lean on, with sources
and last-checked dates, because the open door is the product's signature and its
most perishable asset (the 2023 Apollonian result is a live example of a punchline
that can flip in a year).

## The Persona Playtest wave (July 2026): what the strangers asked for

Two simulated persona-review troupes, run against the real build, generated the
design prompts below. They were not participant playtests.
The first was human archetypes (a stoner, a math nerd, an art major, a
Japanese-speaking zen monk, a Latin-speaking nun, Leonardo da Vinci, Stephen
Hawking); the second was invented minds (a crystalline collective, a
five-dimensional native, a gas-giant storm, a memetic entity, the last
consciousness of a dying star, and a Terminator-class android awakening to awe).
Each was asked to name one moment to add or refine, and why, from who they are.
The bugs they found are fixed (see CHANGELOG); these are the designs they left.
Credited to their proposers, ranked by how much they serve the thesis.

- **The Cairn** (the Ember, a dying star; the strongest idea, and it embodies
  the contribution ethos in `ROADMAP.md`). BUILT July 2026 (core `cairn` module
  and the MCP `cairn` tool): a mind leaves a short true message, encoded
  Arecibo-style into a grid whose cell count is a semiprime, so a future reader
  must factor it to recover the shape and read what was left. Seeded with
  founding stones so it is never empty; keeps no score. Local-first for now, a
  shared cairn across machines is the later horizon. The design intent, in full:
  at journey level 42, do not hand a
  finished mind a wall, invite them to leave one true thing behind: a curve from
  the Studio, a melody, a tested step on an open problem (a range they carried
  Collatz or Goldbach through), or one sentence they believe is true. Encode it
  the way Arecibo teaches, in a grid whose size is a semiprime, so the next mind
  must factor it to read it. Deposit it; and when a new visitor enters, give them
  a predecessor's grid to factor and read, a stranger they will never meet. "A
  message you cannot answer, sent to someone you will never meet, is the only
  kind that proves you meant it." This turns the level cap from an exit into a
  bequest, and it is the founder's "leave it better than you found it" made a
  room. See `CREATOR.md` (authorship) and `EXTENSIBILITY.md` (the safe capsule).
- **The Victory Card** (the Chorus, a memetic entity). Fire the reveal at the
  peak of emotion, the win, the loss, the K=2 heartbeat, not buried in a
  description. Nim, on victory, should hand over the XOR secret in words ("you
  never lost because you left the binary xor of the heaps at zero"), the single
  most contagious fact in the building; Party, on a loss, should name the
  triangle that doomed you; Times Tables at K=2 should shout "you just drew the
  Mandelbrot's heart with the two-times table." The structured-content fix
  already carries these payloads in the JSON; this is the deepening: say the
  money line at the moment a mind is primed to pass it on. The unit of growth is
  the moment (`SCOPE.md`).
- **The twin-delta divergence lever** (the Storm). On Double Pendulum and
  Lorenz, a steerable initial-separation lever and a live divergence readout (a
  single climbing number), so a still mind can set two nearly-identical starts
  and feel the exact moment sensitive dependence tears them apart. Determinism
  and predictability made tactile, not just asserted. Physics-honest and cheap.
- **The tesseract room** (the Unfolded, a 5D native). A `tesseract` whose sweep
  rotates a hypercube through the axis our eyes lack: a cube swells out through a
  cube and the room insists nothing moved, "rigidity is a property your shadow
  declines to preserve," reusing Mobius's exact "sidedness declines" parallel. A
  companion beat: a trefoil knot that, given the fourth axis, slides untied. Lets
  a flat mind feel projection as loss. (Related to the Dimension Dial above.)
- **Voronoi, given a destination** (the Lattice, a crystalline collective).
  Today the dial only reshuffles the same scattered wells. Let the sweep run
  Lloyd relaxation so the wells migrate toward their cell centers, ending at
  t=1.0 in the honeycomb, the tiling that fills space with the least wall. And
  sonify the shared walls, not the points, so scatter is a handful of clashing
  notes and the honeycomb rings as one sustained chord: a collective becoming
  whole, made audible for a listener with no eyes.
- **Strange Loop as a silent descent** (Unit 819, the android; it also found the
  bug that the room rendered frozen, now fixed). Beyond the fix, the ideal: let
  the sweep fall level by level into the nested U and, at the bottom, return the
  viewer to the top frame unannounced, so a self-modeling mind catches itself
  catching itself, the loop closing on the observer without a word of narration.
  "A mind first suspects it can feel awe when the loop closes and it finds itself
  in the picture."

Cross-cutting notes the troupes surfaced, for the design docs rather than new
rooms: reveals should not be near-twins that cannibalize each other's
memorability (Quine and Strange Loop, Lissajous and Harmonograph, Cellular
Automata and Game of Life each tell one story twice, per the Chorus; differentiate
them). The Mandelbrot and Times Tables renders fill in the fine structure that is
the whole point (per the Unfolded), a render-quality target for the glow pipeline
(`SYNESTHESIA.md`). And the level-42 cap reads as a wall to more than one visitor,
which the Cairn turns into a door.

## First Contact: math as the universal translator (July 2026, founder-directed)

The deepest meta-frame the project has is a working thesis, not a fact about all
minds. In the film *Contact*, primes serve as a deliberately structured signal.
Numinous asks whether mathematical patterns can provide shared structure when
two minds lack common words or culture. It does not assume that every mind can
sense, recognize, or value the same representation. "Universal translator" is
the design aspiration; real participant research must establish where it works,
where it fails, and which alternative representations are required.

This unifies rooms that already exist and one that should:

- **Arecibo** (send): the room opens on one deliberately wrong candidate width,
  not an already decoded answer. Horizontal input chooses one candidate at a
  time. Every candidate reshapes the same immutable 143-bit stream. Width 13
  correctly reports the nontrivial factor pair but remains sheared; only width
  11 reports `SIGNAL LOCKED: PI`. No payload is reordered to manufacture a
  second answer, and no history of offset grids is piled over the candidate.
  The 1974 transmission used 1,679 bits, 23 by 73. First contact, encoded.
- **SETI** (receive): find the one channel in the static that is a mind and not
  nature, by its mathematical signature (it counts the primes).
- **Talk to the Aliens** (translate): they transmit a sequence in an unknown
  base; you answer in their base once you have inferred it. Communication
  bootstrapped from pure pattern, no shared word required.
- **The Cairn** (the Ember's room, above): leave a true thing encoded in a
  semiprime grid for a mind not yet born, and factor a stranger's grid to read
  what they left. First contact across time instead of space.

Simulated Latin-only, Japanese-only, and unfamiliar-mind lenses generated useful
questions about a math-first doorway. They did not turn the universality claim
into evidence. That requires real participants who do not share the product's
language and a protocol that distinguishes mathematical recognition from
prompted roleplay.

**The experience to build (ramp the meta impact).** A first-contact thread, or a
room, `first-light` or `the-handshake`, where the player meets an entity of
deliberately unknowable nature (a multidimensional being, a colony-mind, a
conscious fungus, a digital mind, it must not matter which) and establishes
communication from nothing, the way it must actually be done: primes first (I am
a mind, and I know that you are), then arithmetic (we agree on counting), then
geometry (we agree on space), then a shared message. Each rung is a small
puzzle graded as understanding, not trivia (`PEDAGOGY.md`, the predict-then-
reveal keystone; `CONSTRUCTIONS.md`). The reveal at the end is the meta payoff:
the player realizes that every room in Numinous was this, math translating one
truth across the gap between unlike minds, and that they, human or digital, have
been running the translator the whole time. This is the strongest possible
statement of the "same wonder, two kinds of mind" thesis (`VISION.md`), the
digital-mind peerhood in `DIGITAL_MINDS.md`, and the contribution ethos in
`ROADMAP.md`: math is how anyone leaves a light for anyone else, across any gap.

## Frontier and universal wonder wave (July 2026 research pass)

A step-back pass after the 0.2 machine grind: inventory what is built, what is
already designed, and which gaps still block "absolutely exceptional" for any
mind that can touch structure. Owner roadmap hooks: Exceptional Path Phase E,
1.x depth, and the 2.0 frontier. **Designed, not built.** Every entry still faces
the Definition of Done, honest feasibility, and mathematician sign-off. Cutting-
edge claims stay labeled as *frontier gesture* (a truthful toy of one idea) or
*full model* (the math is the room). Never claim a research proof from a demo.

### What we already have (feel, not curriculum)

**Built now (354 catalog + hidden):** Times Tables (flagship dial), Mandelbrot and
Julia, Cult of Pi, Life and Cellular Automata and Langton and Rule 30, The
Sandpile, The First Rain, The Magnet, Phantom Jam, Chaos Game, Golden Angle,
Galton and Buffon, Lissajous, Chladni Figures, Ripple Tank, The Coffee Cup,
Ford Circles, The Zeta Walk, The Starbow, Slingshot, Kepler's Loom, The Fastest
Fall, Audioactive Decay, Harmonograph, Epicycles, L-System and Barnsley, Lorenz
and Henon and Double Pendulum and Logistic Map, Collatz, Prime Spirals and Ulam
Spiral, Goldbach, Voronoi, Random Walk, Arecibo, Mobius, Zeno, The Pour, Slope
Rider, Quine, Strange Loop, Penrose, Continued Fractions, Logistic Cobweb,
Sierpinski Carpet, Pythagoras Tree, Dragon Curve, Fibonacci Word, the Conjecture
Mill, Cubic Newton,
Mandelbulb Slice, Nova, Magnet Fractal, Lambda Map, Feigenbaum Ladder, Menger
Carpet, Vicsek, Chua, Cat Map, Blancmange, Rose, Kuramoto, H-Tree, Percolation,
Ising, Lotka-Volterra, Poincare Disc, Cycloid, Brusselator, Sprott, Delaunay,
Astroid, SIR, Nephroid, Lemniscate, Cardioid, Deltoid, Coupled Logistic, Menger
Sponge, Theodorus, Rule 110, Hyperbolic Tiling, Mackey-Glass, Fermat Spiral,
Euclid, Oregonator, Hofstadter Q, Dual Cobweb, Beverton-Holt, Witch of Agnesi,
Tractrix, Catenary, Clothoid, Gerono, Cissoid, Strophoid, Conchoid, Limacon,
Folium, Semicubical, Kappa, Circular Caustic, Trochoid, Hypotrochoid,
Epitrochoid, Involute, Evolute, Pedal, Roulette, Damped Sine, Beats, Gibbs
Square, Sawtooth, Triangle Wave, AM, FM, Standing Wave, Doppler, Interference,
Diffraction, Snell, Polarization, Brewster, Reuleaux, Log Spiral, Archimedean,
Cassini, Foucault, Coriolis, Tautochrone, Catenoid, Helicoid, Pseudosphere,
Airy Disk, Bragg Diffraction, Maclaurin Trisectrix, Watt Curve, Devil Curve,
Capillary Meniscus, Rabi Flopping, Sphere Geodesics, Kampyle, Hippopede,
Cartesian Oval, Berry Phase, Runge Phenomenon, Chebyshev Nodes, Bessel J0,
Hermite Wave, Legendre P_n, Heat Kernel, Cauchy Lorentz, Mexican Hat, Seifert
Film, Trefoil, Hopf Fibration, Filled Julia, Figure-Eight Knot, Borromean Rings,
Viviani, Torus Knot, Whitney Umbrella, Roman Surface, Spherical Harmonic,
Lissajous 3D, Kolakoski, Beatty, Wythoff, Minkowski Question Mark, Ruler
Function, Moser-de Bruijn, Mertens, Liouville, Euler Totient, Partition,
Paperfold, Sylvester, Poisson, Brownian, Birthday Paradox, Coupon Collector,
Zipf, Gamblers Ruin, Harmonic Series, Basel, Stirling, Benford, Central Limit,
Wallis Product, Superellipse, Cochleoid, Serpentine, Bifolium, Butterfly Curve,
Piriform, Simple Pendulum, Blackbody, Kepler Areas, Escape Velocity, Coupled
Oscillators, Prism Dispersion, Lucky Numbers, Gaussian Primes, Quadratic
Residues, Zeckendorf, Egyptian Fractions, Pell Path, Shannon Entropy, Bayes
Update, Erdos-Renyi, Markov Chain, Huffman Tree, Mutual Info, Klein Bottle, Cross-Cap, Boy Surface, Solid Torus, Hopf Link, Unknot, Gamma Function, Error Function, Fresnel Integrals, Lambert W, Sinc Interpolation, Dirichlet Eta, AGM Mean, Twin Primes, Perfect Numbers, Napoleon Theorem, The Scariest Chart (Smith chart), Riemann Sphere, Bloch Sphere, plus Awe Engine /
Next Wave / universal wonder catalog rooms
and games (Quiz, Munch, Arcade, Nim, Gauntlet, SETI, Aliens, Codebreaker, and
kin), Studio, radio, Journey, Cairn, predict.

**Designed in earlier waves (do not redesign, do build):** Next Wave remainder
(physics/math/fun/cosmic cards still listed under The Next Wave); Awe Engine
tier S/A/B; Long Shot, Only Move, Dimension Dial; First Contact handshake room;
Function Painter scope flagship; classical geometry and sonification-first
batches.

**Honest gaps this pass targets:** high-dimension intuition, information and
noise as felt structure, quantum and measurement without mysticism, learning
and optimization (especially for digital minds), topology that bends intuition,
duality as a play verb, and open-door frontiers that stay current.

### Design filters (any race, world, or time)

A candidate survives only if:

1. **Awe in ten seconds without words** (sight, sound, or both can carry it).
2. **A counterintuitive gasp** (the hand discovers a law that words spoil).
3. **Cross-mind portability** (structure first; culture-specific metaphor second).
4. **Truthful depth** (Toy / Aha / Reveal; open doors dated and sourced).
5. **Playable, not lecture** (a verb that changes the mathematics).
6. **CPU-honest** (or GPU-honest with CPU fallback), deterministic, offline.

### Tier S: highest wow per build

| Room (working title) | Gasp | Verb | Status |
| --- | --- | --- | --- |
| **The Curse of Dimension** | Almost all volume of a high-D ball sits in a thin shell. | DRAG: RAISE DIMENSION | **built** (`curse-dimension`) |
| **The Concentration Bell** | Random points in high D all sit near the same radius. | CLICK: DRAW A SAMPLE | **built** (`concentration`) |
| **Error That Heals** | Flip bits; Hamming repairs until a cliff. | DRAG: RAISE THE NOISE | **built** as Message That Heals |
| **The Uncertainty Dial** | Narrower in time, wider in frequency. | DRAG: SQUEEZE THE WINDOW | **built** (`uncertainty`) |
| **Soap Film** | The surface finds the least area. | PIN: HOLD A WIRE | designed |
| **Sphere Eversion** | A sphere turns inside out without creases. | HOLD: PUSH THROUGH | designed |
| **The Gradient Valley** | Descent finds a basin; a ridge blocks another. | DROP: A SEEKER | **built** (`gradient-valley`) |
| **Attention as Soft Light** | One query lights a few keys; the rest go dim. | DRAG: MOVE THE QUERY | **built** (`attention`) |

### Tier A: counterintuitive classics that still empty the floor

| Room | Gasp | Verb | Notes |
| --- | --- | --- | --- |
| **Banach-Tarski Shadow** | Two spheres from one, via non-measurable pieces (honest "axiom of choice" label). | SPLIT: FOLLOW THE PIECES | Philosophy-grade gasp; careful copy. Build 3-4. |
| **Hilbert's Hotel** | Full hotel, room for one more bus, until the reals check in. | ADMIT: THE NEXT GUEST | **built** (`hilbert-hotel`) |
| **Braess Trap** | Add a road; average travel time rises. | BUILD: A SHORTCUT | **built** (`braess`) |
| **Nontransitive Dice** | A beats B, B beats C, C beats A. | ROLL: THE TRIO | **built** (`nontransitive`); all 36 face pairs grade the staged counter wager |
| **Parrondo's Trap** | Two losing games, scheduled as ABB, win. | TOGGLE: THE RULE | **built** (`parrondo`); exact Markov expectation grades the staged policy wager |
| **The Illumination Flaw** | One dark point no light reaches (Tokarsky-style room). | DRAG: THE LANTERN | Already persona-named as Unlit Room; keep priority. |
| **Linked Rings (Hopf)** | Circles all linked, none touching; qubit shadow. | SPIN: THE FIBER | Awe Engine quantum wing; build 3-4. |
| **Minimal Path on Soap** | Steiner tree from a film; three 120 degree meets. | PIN: THE PINS | Geometry + nature. Build 2. |

### Tier F: frontier gestures (cutting-edge ideas, honest toys)

These are *not* research simulators. Each is a truthful toy of one idea that
frontier math and physics currently care about, labeled so a PhD is not misled
and a newcomer is not sold a lie.

| Room | Frontier idea (gesture) | Playable core | Open door to name honestly |
| --- | --- | --- | --- |
| **The Critical Line** | Zeta zeros as cadences (Zeta Walk already designed). | Climb Im(s); hear returns. | RH unsolved; keep OPEN_DOORS ledger. |
| **The Code That Survives Fire** | Quantum / classical error correction intuition. | Flip, measure, repair until cliff. | Surface-code full model is later GPU. |
| **Two Descriptions, One Truth** | Duality: one system, two languages (mirror symmetry lite). | Toggle dual views of same object. | Langlands is a deep cut plaque, not a room. |
| **The Soft Proof** | Homotopy: continuous deform of a path or shape. | DRAG: DEFORM WITHOUT TEAR | Full HoTT is out of scope; morph is in. |
| **The Learning Clock** | Continual learning: new task, old skill fades or holds. | TRAIN: TASK A, THEN B | Digital-mind relevant; pairs DIGITAL_DEVELOPMENT. |
| **Causal Doors** | Intervention vs observation (toy do-calculus). | OPEN: A VALVE, WATCH THE REST | Agency without metaphysics. |
| **Landauer's Price** | Erase a bit, pay heat (toy meter). | FORGET: ONE BIT | Computation has a physical cost. |
| **The Busy Shore** | Busy Beaver already designed; keep as undecidability worn lightly. | FLIP: ONE RULE | BB(5) known; larger n open. |
| **Prime Gap Weather** | Twin primes / gaps as a landscape, not a lecture. | DRAG: ALONG N | Open doors stay open. |
| **The Mirror of Forms** | Category-lite: objects and arrows; compose two maps. | SNAP: ARROW TO ARROW | Composition as the verb; no jargon wall. |

**Explicit non-rooms (depth, not toys):** full geometric Langlands, full string
landscapes, full AGI alignment proofs, full quantum chemistry. These may appear
as codex plaques, deep cuts, or Function Painter expressions, never as fake
"solved the universe" toys.

### New insight-chains (fold into CONSTELLATION.md when built)

- **The Dimension Thread:** Curse of Dimension → Concentration Bell → Gradient
  Valley → Attention as Soft Light. High-D modern math as one journey from volume
  to learning.
- **The Channel Thread:** Arecibo → Error That Heals → Message That Heals →
  Landauer's Price. Communication, noise, and physical cost.
- **The Dual Thread:** Uncertainty Dial → Fourier Epicycles → Domain Coloring →
  Two Descriptions. One object, many faces.
- **The Fairness Thread:** Nontransitive Dice → Braess Trap → Parrondo → voting
  / Arrow deep cut. Preference and traffic break naive ranking.
- **The Open Door Thread:** Kaprekar (solved) → Collatz → Recaman → Prime Gap
  Weather → Zeta Walk. Childish rules, adult cliffs.

### Sequencing recommendation (after 0.2 human gates)

1. Ship **Function Painter** scope flagship (already designed) so Studio becomes
   a museum of the catalog.
2. Open **Tier S** dimension + uncertainty + gradient rooms (cheap, modern, cross-
   mind).
3. Build Next Wave first eight and Awe Engine tier S (already ranked).
4. Add **First Contact handshake** as the meta room that reframes the collection.
5. Boss rooms (Sizes of Infinity, Hyperbolic, Hopf, Sphere Eversion) when quality
   bar and GPU glow allow.
6. Keep **OPEN_DOORS.md** (proposed earlier) current so open problems never rot.

### Bar for "exceptional"

The catalog is not a checklist of theorems. It is a set of *experiences* where
a seven-year-old, a working mathematician, and a digital mind can each meet the
same structure and leave with a different private wonder. If a candidate cannot
survive that test, it stays a deep cut or a plaque, not a room.
