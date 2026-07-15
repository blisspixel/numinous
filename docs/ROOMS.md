# The Rooms

The content catalog: the phenomena Numinous is built from. Each **room** is one playable mathematical object. Rooms are grouped into **Wings** by feeling, not by curriculum.

**Current status (as of 2026-07):** 31 catalog rooms across 10 wings plus hidden content. Per-visit variation seed is threaded through registry/app/CLI/MCP; all 31 catalog rooms use it for replay novelty, while hidden content stays outside the catalog replay contract. All 31 catalog rooms have `verb()` + `render_poked()` touch actions (usually CLICK or DRAG on arrival cards), and all 31 catalog rooms expose Engine A2 motifs. See `ARCADE.md` for design.

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
  and MCP share the same goal, status, accepted hand state, and earned reveal.
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
- **Aha:** "Make the pile match the outline, then make it lean." The finite pile stays noisy while its shape becomes easier to recognize.
- **Reveal:** *"The coin probability alone does not determine the next landing. With one probability fixed, the number of right turns in a 16-flip landing follows exactly Binomial(16, p), and repeated waves make the empirical pile estimate that discrete distribution. With many rows and a coin away from either extreme, a normal curve can approximate the binomial, the direction formalized by the Central Limit Theorem. This board displays the finite binomial itself."*
- **Sound:** balls tick on pegs (rain-stick / bucket-drum texture); the pile's growth swells a soft pad.

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

**Current interaction inventory (2026-07):** 31 catalog rooms plus hidden content are built. Every catalog room exposes a touch verb, replayable bounded input, and per-visit variation across the app, CLI, and MCP. Representative actions include ADD A CORNER in Chaos Game, PLACE A 5-CELL GLIDER in Life, FLIP A CELL in Cellular Automata and Langton's Ant, SEED A SHADOW STORM in Lorenz, PLANT A WALKER in Random Walk, DROP A WELL in Voronoi, TRACE PRIME DIAGONALS in Prime Spirals, PLANT A SEED in Golden Angle, RESTORE AND HOLD A PATCH in Cult of Pi, THROW A NEEDLE in Buffon, DIVE AT POINT in Mandelbrot, MORPH C in Julia, TURN THE DIAL in Times Tables, and TEST THIS EVEN in Goldbach. Full-frame or held responses use `render_input`; interaction-aware readouts use `status_input` in every face.

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
- [ ] **Benford's law** - a fraud-detective game: two ledgers, one cooked; the leading digits snitch.
- [ ] **RSA in miniature** - extend Crack the Code: multiply two primes and watch why the bomb squad cannot reverse it.

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
- [ ] **Bayes** - a lie-detector game: update your suspicion die-roll by die-roll; feel evidence accumulate instead of computing it.
- [x] **Random walks** - the drunkard: stumble n steps, end up sqrt(n) from the bar, every time, on average.
- [ ] **Birthday paradox** - a party-filling toy: watch the collision arrive absurdly early; bet against it and lose.
- [ ] **Markov chains** - a weather machine with dials: today decides tomorrow; find the steady state by feel.

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
- [x] **Double pendulum** - grab it, drop it, and watch two of them disagree from a pixel of difference.
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

## Founder's room idea (July 2026): The Only Move

**Status:** designed, not built. Roadmap position: 1.x, after the current
flagship gates, alongside The Long Shot. Wing: Decision (games and fairness),
with a Computation resonance.

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

**Built now (190 catalog + hidden):** Times Tables (flagship dial), Mandelbrot and
Julia, Cult of Pi, Life and Cellular Automata and Langton and Rule 30, The
Sandpile, The First Rain, The Magnet, Phantom Jam, Chaos Game, Golden Angle,
Galton and Buffon, Lissajous, Chladni Figures, Ripple Tank, The Coffee Cup,
Ford Circles, The Zeta Walk, The Starbow, Slingshot, Kepler's Loom, The Fastest
Fall, Audioactive Decay, Harmonograph, Epicycles, L-System and Barnsley, Lorenz
and Henon and Double Pendulum and Logistic Map, Collatz, Prime Spirals and Ulam
Spiral, Goldbach, Voronoi, Random Walk, Arecibo, Mobius, Zeno, The Pour, Slope
Rider, Quine, Strange Loop, Penrose, Continued Fractions, Logistic Cobweb,
Sierpinski Carpet, Pythagoras Tree, Dragon Curve, Fibonacci Word, Cubic Newton,
Mandelbulb Slice, Nova, Magnet Fractal, Lambda Map, Feigenbaum Ladder, Menger
Carpet, Vicsek, Chua, Cat Map, Blancmange, Rose, Kuramoto, H-Tree, Percolation,
Ising, Lotka-Volterra, Poincare Disc, Cycloid, Brusselator, plus Awe Engine /
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
| **Nontransitive Dice** | A beats B, B beats C, C beats A. | ROLL: THE TRIO | **built** (`nontransitive`) |
| **Parrondo's Trap** | Two losing games, alternating, win. | TOGGLE: THE RULE | **built** (`parrondo`) |
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
