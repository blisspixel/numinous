# The Rooms

The content catalog: the phenomena Numinous is built from. Each **room** is one playable mathematical object. Rooms are grouped into **Wings** by feeling, not by curriculum.

Every room is scored on two axes to help sequencing:

- **Wow** (1-5): how hard it hits a first-timer. Our whole product is wow-per-second.
- **Build** (1-5): rough implementation cost (5 = hard). We front-load high-wow / low-build rooms.

Each room lists its **Rule** (the deliberately-tiny input), the three layers (**Toy / Aha / Reveal**), and its **Sound** mapping. The Reveal lines are drafts, the *tone* is the point.

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
- **Toy:** Paint cells, press play, watch it evolve. Drop in a glider, a gun, a puffer. A living, breathing petri dish that plays like an instrument.
- **Aha:** "Build something that never dies" / "make a pattern that moves."
- **Reveal:** *"Those four rules are enough to build a working computer. People have built Tetris, and Conway's Game of Life itself, inside this. It's not a toy. It's a universe."*
- **Sound:** each birth triggers a note pitched by its position; dense colonies swell the pad. A living generative sequencer.

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
- **Aha:** "Draw a square with circles." (The Gibbs ringing you get is itself a famous phenomenon.)
- **Reveal:** *"Every one of those is a perfect circle spinning at a steady speed. Stack enough of them and you can draw literally anything, a portrait, a heartbeat, a stock chart. This is how your phone compresses every song and every image. It's the most useful idea in math you've never been taught."*
- **Sound:** each circle is a pure sine tone at its frequency; the drawing *is* the chord. You hear the Fourier transform of your own doodle.
- *One of the two or three most beloved math visualizations ever. A signature room.*

### 6. Lissajous / Harmonograph  Wow 4 / Build 1
- **Rule:** Two pendulums swinging at right angles, each a different speed.
- **Toy:** Two frequency dials. When the ratio is simple (2:3, 3:4) a clean, stable curve hangs in the air; nudge it off-ratio and the whole figure slowly tumbles and precesses forever. Add damping for the gorgeous decaying spirals of a real sand-pendulum.
- **Aha:** "Freeze the figure" (find an exact integer ratio, it stops tumbling).
- **Reveal:** *"A stable figure means the two frequencies are a perfect musical interval. You're not drawing a curve, you're seeing a chord. This is what old oscilloscopes did, and it's why a 2:3 ratio looks calm and sounds like a perfect fifth."*
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

---

## Wing III: Infinity & Fractals
*Vertigo. The rooms that make the floor drop out.*

### 9. Mandelbrot / Julia Dive  Wow 5 / Build 3
- **Rule:** Square a number, add the starting point, repeat. Color by how fast it runs away.
- **Toy:** Infinite zoom. Fall into the boundary forever, seahorses, lightning, spirals, tiny perfect copies of the whole set buried miles deep. Move your mouse over the Mandelbrot and watch its **Julia set** twin morph live in a second panel.
- **Aha:** "Find a hidden mini-Mandelbrot." / "Zoom until you find a spiral."
- **Reveal:** *"This shape is infinitely detailed, you could zoom for the rest of your life and never hit the bottom, and it never repeats. It's defined by an equation short enough to tweet. The most complex object humans know of is also one of the simplest to write down."*
- **Sound:** iteration-count-to-escape maps to pitch; zooming sweeps a drone through octaves; the boundary shimmers with high harmonics.
- *The postcard of mathematics. Needs a solid WebGL deep-zoom shader (double-precision / perturbation for deep dives).*

### 10. Koch / L-System Garden  Wow 4 / Build 2
- **Rule:** Replace every line with a smaller pattern. Repeat. That's it.
- **Toy:** A visual rule-editor: "each branch → two branches at this angle." Turn the angle and iteration dials and *grow* snowflakes, ferns, trees, coastlines, whole alien plants. Small rule tweaks → wildly different flora.
- **Aha:** "Grow a tree that looks real." / "Make a perfect snowflake."
- **Reveal:** *"A fern doesn't 'know' it's a fern. It just follows one rule: split, shrink, repeat. This is how nature packs infinite complexity into a seed, the same tiny instruction, run over and over. Coastlines do it too, which is why 'how long is Britain's coastline' has no answer."*
- **Sound:** each branching generation adds a harmonic layer; deeper recursion = richer chord.

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
- **Reveal:** *"You drew a heart with nothing but the two-times table. That cardioid? It's the exact outline of the Mandelbrot set's main body. You've been sketching the boundary of the most complex object in math, with a ruler and your seven-year-old's homework."*
- **Sound:** *k* controls pitch; the number of lobes sets a harmonic ratio, so morphing the shape *is* a melodic slide. Landing on an integer snaps to a clean note.
- *Cheap to build, stunning in motion, performable, tweetable, and the Reveal genuinely reframes the whole thing. This is the one we perfect first.*

### 14. Prime Spirals (Ulam & Sacks)  Wow 4 / Build 2
- **Rule:** Write the whole numbers in a spiral. Light up the primes.
- **Toy:** Watch primes, supposedly the most "random" numbers, snap onto unmistakable **diagonal streaks**. Switch to the Sacks spiral for sweeping curved rivers of primes. Zoom out to thousands; the pattern refuses to go away.
- **Aha:** "Find the longest prime diagonal."
- **Reveal:** *"Primes are famously unpredictable, we still can't fully explain how they're spread out; a million-dollar prize (the Riemann Hypothesis) rides on it. And yet, arrange them like this and they line up in streaks nobody has fully explained. There's a pattern in the most patternless thing we know, hiding in plain sight."*
- **Sound:** scanning the spiral, each prime is a click/note → an irregular-but-not-random rhythm you can *hear* has structure.

### 15. Collatz Orbits  Wow 4 / Build 2
- **Rule:** Pick a number. If it's even, halve it. If it's odd, triple it and add one. Repeat.
- **Toy:** Type any number; watch its bouncing journey, soaring up, crashing down, until it always, always crashes to 1. Plot thousands of these paths and they braid into a gorgeous coral-like tree. Bend the branch angles into an organic, blooming structure.
- **Aha:** "Find a number that takes more than 100 steps." (27 is a famous monster.)
- **Reveal:** *"Every number ever tested falls to 1. Nobody on Earth can prove they all do. It looks like a five-year-old's rule. It has defeated every mathematician for 90 years. Paul Erdős said 'mathematics is not yet ready for such problems.' You're playing with an open mystery."*
- **Sound:** up-steps rise in pitch, down-steps fall → each number plays its own little unpredictable tune that always resolves home.

### 16. Golden Angle / Phyllotaxis  Wow 4 / Build 1
- **Rule:** Place seeds one at a time, each turned a fixed angle from the last.
- **Toy:** One dial: the angle. At the **golden angle (137.5°)** the seeds pack into a flawless sunflower spiral. Nudge it a fraction of a degree and the whole beautiful order shatters into clumsy spokes and gaps. Feel *why* nature chose exactly this number.
- **Aha:** "Find the angle that packs seeds perfectly." (It's the most irrational number there is.)
- **Reveal:** *"Sunflowers, pinecones, and pineapples all use this exact angle, 137.5°, because it's built from the golden ratio, the 'most irrational' number, which means seeds never line up and never waste space. Evolution discovered the same number mathematicians did. The count of spirals is always a Fibonacci number. Go count them."*
- **Sound:** each seed plinks; the golden angle produces an evenly-spaced, satisfying rhythm, off-angles clump into stumbling beats.

### 17. Digits of π / e / √2 as Music  Wow 3 / Build 1
- **Rule:** Map each digit to a note. Play the number.
- **Toy:** Choose a constant and a scale; hear π play forever, never repeating, never resolving. Watch the digits walk as a colored path (a "random walk") that wanders the plane and never comes home.
- **Aha:** "Find your birthday inside π." (It's in there. Everything is.)
- **Reveal:** *"π's digits go forever without repeating or settling into any pattern, we've computed 100 trillion of them. Somewhere in there is your phone number, this sentence encoded as numbers, and the full text of every book ever written. Probably. We can't even prove that, and that's the fun part."*
- **Sound:** *is* the room, an endless, non-repeating, strangely listenable melody.

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
- **Toy:** Pour thousands of balls and watch pure random bouncing pile up into a flawless **bell curve**, every single time. Widen the board, change the odds, watch the curve slide and skew.
- **Aha:** "Make a lopsided pile." (Bias the pegs.)
- **Reveal:** *"Each ball's path is pure chaos, you can't predict a single one. But together they form the exact same curve, every time, to the millimeter. This is the Central Limit Theorem, the reason the bell curve rules everything from heights to test scores to the stock market. Chaos, in bulk, is perfectly predictable."*
- **Sound:** balls tick on pegs (rain-stick / bucket-drum texture); the pile's growth swells a soft pad.

### 22. Buffon's Needle → π  Wow 4 / Build 1
- **Rule:** Drop needles on a lined floor. Count how many cross a line.
- **Toy:** Rain thousands of needles; a running tally slowly, magically converges on **π**. A number about *circles* falls out of *randomly dropping sticks* with no circle in sight.
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

Status marks: [x] built, [~] partially built, [ ] queued.

## Number
- [x] **Modular arithmetic** — Times Tables: strings on a circle bloom into a cardioid.
- [x] **Primes** — the Ulam spiral; SETI (only minds count in primes); Munch (eat them).
- [x] **Continued fractions / irrationality** — the Golden Angle: detune the sunflower and it shatters.
- [x] **Number bases** — the aliens count on eight tentacles.
- [x] **Open conjectures as toys** — Collatz: play with an unsolved problem.
- [ ] **Cardinality of infinities** — Hilbert's Hotel as a management game: always room for one more bus, until the reals check in and the front desk breaks. You feel the difference between countable and not.
- [ ] **Benford's law** — a fraud-detective game: two ledgers, one cooked; the leading digits snitch.
- [ ] **RSA in miniature** — extend Crack the Code: multiply two primes and watch why the bomb squad cannot reverse it.

## Algebra and symmetry
- [ ] **Group theory** — The Braid: swap strands, learn what undoes what; noncommutativity as a knot in your hands.
- [ ] **Wallpaper symmetry** — a stamp toy that snaps your doodles into each of the 17 wallpaper groups; you discover there are only 17 by running out.
- [ ] **Newton fractals** — polynomial roots as basins: aim, release, and see which root catches you; the boundaries are the surprise.
- [x] **Complex numbers** — the entire Fractals wing runs on them, unannounced.
- [ ] **Eigenvectors** — The Calm Axes: shear a grid with your hands; two directions refuse to turn.

## Geometry and topology
- [ ] **Aperiodic tiling** — the Hat monotile: tile forever, never repeat (the 2023 result as a jigsaw).
- [ ] **Hyperbolic space** — the crochet-coral plane: more room than the room has; parallel lines diverge under your cursor.
- [ ] **Mobius strip** — draw the center line, cover both sides without lifting; cut it and gasp.
- [ ] **Knots** — tangle and untangle; discover some tangles are truly different, not just stubborn.
- [ ] **Four-color map** — race to color a map with five, then four, then try three and fail forever.
- [x] **Voronoi** — drop wells in a desert and watch territories crystallize; every point served by its nearest well.
- [x] **Phyllotaxis / packing** — the Golden Angle again (geometry door this time).

## Change (analysis)
- [x] **Integration** — The Pour: area pours like water; the fill level traces the antiderivative; reverse the pour and you are differentiating.
- [x] **Differentiation** — Slope Rider: ride the tangent; your speed is the derivative; inflections are the jumps.
- [ ] **Limits** — Zeno's Runner: sprint half the remaining distance per tap; the wall arrives anyway.
- [x] **Fourier** — the Epicycle Draw: any shape you doodle, rebuilt by circles on circles.
- [x] **Differential equations** — Lorenz: three equations, weather, the butterfly.
- [x] **Exponential growth and equilibrium** — Tribbles; the Big Bang's omega; e hides in both.
- [ ] **Taylor series** — a zoom toy: every smooth curve becomes its own tangent parabola, cubic, quartic, as you add terms with a slider; sin(x) assembles itself out of polynomials.

## Chance
- [x] **Central limit theorem** — the Galton board's bell.
- [x] **Monte Carlo** — Buffon's needles estimate pi with no circle in sight.
- [ ] **Bayes** — a lie-detector game: update your suspicion die-roll by die-roll; feel evidence accumulate instead of computing it.
- [x] **Random walks** — the drunkard: stumble n steps, end up sqrt(n) from the bar, every time, on average.
- [ ] **Birthday paradox** — a party-filling toy: watch the collision arrive absurdly early; bet against it and lose.
- [ ] **Markov chains** — a weather machine with dials: today decides tomorrow; find the steady state by feel.

## Structure (discrete)
- [ ] **Graph theory** — the Bridges of Konigsberg as a walking puzzle; fail, then learn you were always going to fail, and why (degree parity, never named).
- [ ] **Pigeonhole** — a party trick generator: guaranteed handshake-twins in any crowd of a certain size.
- [ ] **Ramsey** — the party of six: find strangers or friends; order is unavoidable, chaos is impossible.
- [ ] **Traveling salesman** — route the pizza drone; beat the greedy algorithm; meet hardness personally.

## Computation and logic
- [x] **Universality** — Rule 110 and Life (the reveals and deep cuts carry it).
- [x] **Undecidability** — Life's deep cut; the halting problem, worn lightly.
- [x] **Information as structure** — Arecibo (a semiprime is a picture frame); SETI; the codes games.
- [ ] **Sorting, visible** — race the algorithms as animated bar-ballets with sound; quicksort against bubble sort is a horse race.
- [ ] **Entropy** — a compression toy: your keyboard mashing versus Shakespeare versus pi's digits; which squeezes smallest and why.
- [x] **Godel, strange loops** — the lore layer's deep water (LORE.md), never a lecture.

## Decision (games and fairness)
- [x] **Nim** — play it, lose repeatedly, then be handed the xor secret and become unbeatable; the transfer of power is the lesson.
- [ ] **Prisoner's dilemma** — an iterated tournament against strategies with personalities; tit-for-tat wins hearts.
- [ ] **Voting paradoxes** — run the same three-candidate election under five systems and crown five different winners.
- [ ] **Fair division** — cut the cake: I-cut-you-choose, then envy-free for three; fairness as a mechanic, not a sermon.

## Motion and dynamics
- [x] **Deterministic chaos** — the logistic cascade; Lorenz; Langton's Ant.
- [x] **Double pendulum** — grab it, drop it, and watch two of them disagree from a pixel of difference.
- [ ] **Three-body problem** — place three suns and try to make them dance forever; grief teaches what "no closed-form solution" means.
- [x] **Resonance and harmony** — Lissajous, the harmonograph, every room's sound; the kanon whisper.

The wings stay feelings, not branches; this map is the coverage checklist
behind them. A branch is covered when a kid can play its entry and a professor
can nod at it, and neither one is bored.

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
