//! Static metadata and constructors for every built-in room.
//!
//! A catalog entry is the one edit that registers a room module, its metadata,
//! and its replayable constructor. Rendering and interaction remain in the
//! room's own module.

use crate::room::{Room, RoomMeta, RoomMetadata};

macro_rules! catalog_rooms {
    ($callback:ident) => {
        $callback! {
        (
            times_tables,
            TimesTables,
            RoomMeta {
                id: "times-tables",
                title: "Times Tables",
                wing: "Number & Pattern",
                blurb: "From each point n on a circle, draw a chord to point (n times k); \
                        a cardioid blooms out of the two-times table.",
                accent: [40, 150, 190],
            }
        ),
        (
            cellular_automata,
            CellularAutomata,
            RoomMeta {
                id: "cellular-automata",
                title: "Cellular Automata",
                wing: "Emergence",
                blurb: "One line of cells and one tiny rule per cell; sweep t across notable rules, \
                        where Rule 90 draws a Sierpinski triangle and Rule 30 pours out chaos.",
                accent: [70, 200, 100],
            }
        ),
        (
            chaos_game,
            ChaosGame,
            RoomMeta {
                id: "chaos-game",
                title: "Chaos Game",
                wing: "Emergence",
                blurb: "Jump halfway to a random corner of a triangle, over and over, and pure chance \
                        resolves into a perfect Sierpinski fractal. t tunes the jump fraction.",
                accent: [40, 200, 170],
            }
        ),
        (
            golden_angle,
            GoldenAngle,
            RoomMeta {
                id: "golden-angle",
                title: "Golden Angle",
                wing: "Number & Pattern",
                blurb: "Place seeds one at a time, each turned a fixed angle from the last; at the \
                        golden angle they pack into a flawless sunflower, and a nudge shatters it. \
                        t detunes the angle.",
                accent: [210, 160, 40],
            }
        ),
        (
            galton_board,
            GaltonBoard,
            RoomMeta {
                id: "galton-board",
                title: "Galton Board",
                wing: "Chance & Order",
                blurb: "Choose a left, fair, or right-leaning coin. Each click drops 64 balls through \
                        16 peg rows; repeat it and watch chance settle into a binomial pile.",
                accent: [80, 120, 220],
            }
        ),
        (
            lissajous,
            Lissajous,
            RoomMeta {
                id: "lissajous",
                title: "Lissajous",
                wing: "Waves & Sound",
                blurb: "Two perpendicular oscillations, one per axis; a simple frequency ratio traces a \
                        stable figure and off-ratio it tumbles. t sweeps the second frequency.",
                accent: [230, 90, 130],
            }
        ),
        (
            chladni,
            Chladni,
            RoomMeta {
                id: "chladni",
                title: "Chladni Figures",
                wing: "Waves & Sound",
                blurb: "Sand flees a singing plate and draws the silence: nodal curves of a free square \
                        plate under two mode numbers. t walks the mode gallery; DRAG tunes n and m, and \
                        the drive tone is the figure.",
                accent: [200, 190, 120],
            }
        ),
        (
            ripple,
            Ripple,
            RoomMeta {
                id: "ripple",
                title: "The Ripple Tank",
                wing: "Waves & Sound",
                blurb: "Drop pebbles; circular waves interfere into bright fans and dead-calm lanes. \
                        Two sources build the double slit by hand. t ages the phase.",
                accent: [70, 160, 220],
            }
        ),
        (
            coffee_cup,
            CoffeeCup,
            RoomMeta {
                id: "coffee-cup",
                title: "The Coffee Cup",
                wing: "Shape & Space",
                // The shared-cardioid connection is the Times Tables reveal and
                // the thing its wager is graded on. A doorway that hands it over
                // sells another room's aha to a player who has not bought it.
                blurb: "Rays bounce once in a circle and condense into a bright cusped curve. \
                        t walks the sun on the rim.",
                accent: [230, 150, 90],
            }
        ),
        (
            ford_circles,
            FordCircles,
            RoomMeta {
                id: "ford-circles",
                title: "Ford Circles",
                wing: "Number & Pattern",
                blurb: "Every reduced fraction p/q owns a circle of radius 1/(2q^2). Circles never \
                        overlap; they kiss exactly for Farey neighbors. t deepens denominators; \
                        a click births the mediant that fills the gap under the hand.",
                accent: [180, 140, 220],
            }
        ),
        (
            zeta_walk,
            ZetaWalk,
            RoomMeta {
                id: "zeta-walk",
                title: "The Zeta Walk",
                wing: "Number & Pattern",
                blurb: "Partial sums of the alternating eta series on the critical line draw a spiral \
                        that folds home at Riemann zeros. t climbs the imag height. The Prime Spirals egg, made playable.",
                accent: [140, 100, 220],
            }
        ),
        (
            starbow,
            Starbow,
            RoomMeta {
                id: "starbow",
                title: "The Starbow",
                wing: "Shape & Space",
                blurb: "Burn toward lightspeed; relativistic aberration pours the whole sky into a \
                        burning ring ahead. One closed-form transform per star (McKinley 1979). t \
                        burns ambient beta.",
                accent: [255, 200, 80],
            }
        ),
        (
            slingshot,
            Slingshot,
            RoomMeta {
                id: "slingshot",
                title: "Slingshot",
                wing: "Motion & Dynamics",
                blurb: "Pull and release to launch a probe past suns. Gravity assists are discovered, \
                        not taught; missed shots become comets, never failures. t advances the mission \
                        clock; HOLD grows a sun under the hand.",
                accent: [240, 180, 60],
            }
        ),
        (
            first_rain,
            FirstRain,
            RoomMeta {
                id: "first-rain",
                title: "The First Rain",
                wing: "Emergence",
                blurb: "Sites open with probability p; clusters merge until one spans top to bottom. \
                        That cliff sits near p=0.5927. t rains harder.",
                accent: [80, 140, 220],
            }
        ),
        (
            the_magnet,
            TheMagnet,
            RoomMeta {
                id: "the-magnet",
                title: "The Magnet",
                wing: "Emergence",
                blurb: "Spins lock with their neighbors until heat wins. Cross the critical temperature \
                        and order dissolves. t sets the heat. Universality: one \
                        cliff for many microscopics.",
                accent: [200, 60, 80],
            }
        ),
        (
            kepler_loom,
            KeplerLoom,
            RoomMeta {
                id: "kepler-loom",
                title: "Kepler's Loom",
                wing: "Motion & Dynamics",
                blurb: "Fling a moon around a sun: every bound path is an ellipse with the sun at a \
                        focus. Equal areas in equal times is the metronome. t advances the orbit; \
                        a drag flings a moon.",
                accent: [220, 200, 100],
            }
        ),
        (
            phantom_jam,
            PhantomJam,
            RoomMeta {
                id: "phantom-jam",
                title: "Phantom Jam",
                wing: "Emergence",
                blurb: "One brake on a ring of cars births a dense jam that rolls backward against \
                        traffic. No accident, no bottleneck: just follow-the-leader (Sugiyama 2008). \
                        t runs the ring.",
                accent: [230, 120, 40],
            }
        ),
        (
            fastest_fall,
            FastestFall,
            RoomMeta {
                id: "fastest-fall",
                title: "The Fastest Fall",
                wing: "Change",
                blurb: "The fastest path down under gravity is a cycloid, not a straight line. Draw \
                        any other track and lose the race. t runs the beads.",
                accent: [100, 180, 220],
            }
        ),
        (
            audioactive,
            Audioactive,
            RoomMeta {
                id: "audioactive",
                title: "Audioactive Decay",
                wing: "Number & Pattern",
                blurb: "Speak a digit string by runs and it mutates: look-and-say. Length grows by \
                        Conway's constant; the sequence shatters into 92 atoms. t advances generations; \
                        a click speaks the next line.",
                accent: [180, 100, 200],
            }
        ),
        (
            busy_beaver,
            BusyBeaver,
            RoomMeta {
                id: "busy-beaver",
                title: "The Busy Beaver",
                wing: "Number & Pattern",
                blurb: "A tiny Turing machine races to write ones then halt. BB(5)=47,176,870 is proven; \
                        here a toy champion stops on purpose. t extends the step budget.",
                accent: [120, 80, 200],
            }
        ),
        (
            degree720,
            Degree720,
            RoomMeta {
                id: "degree-720",
                title: "The 720 Degree Room",
                wing: "Shape & Space",
                blurb: "A tethered stone needs two full turns to untwist the belt: 360 is not enough, \
                        720 is. Dirac's belt trick; the quaternion double cover of rotations. t spins; \
                        DRAG rotates the stone.",
                accent: [160, 120, 220],
            }
        ),
        (
            upside_ruler,
            UpsideRuler,
            RoomMeta {
                id: "upside-ruler",
                title: "The Upside-Down Ruler",
                wing: "Number & Pattern",
                blurb: "In the 10-adics, ...999999 + 1 = 0, so ...999999 = -1. A tower of nines waits \
                        for the carry that only resolves at infinity. t grows the tower.",
                accent: [200, 160, 80],
            }
        ),
        (
            murmuration,
            Murmuration,
            RoomMeta {
                id: "murmuration",
                title: "Murmuration",
                wing: "Emergence",
                blurb: "Boids with seven neighbors: separate, align, cohere. The flock shape lives in \
                        no single bird. t flies the cloud.",
                accent: [80, 100, 160],
            }
        ),
        (
            whispering_table,
            WhisperingTable,
            RoomMeta {
                id: "whispering-table",
                title: "The Whispering Table",
                wing: "Shape & Space",
                blurb: "Elliptic billiards: every shot is integrable, caustics are confocal curves, \
                        chaos never starts. t turns the ambient aim; PULL AND RELEASE: SHOOT.",
                accent: [180, 140, 100],
            }
        ),
        (
            wet_oracle,
            WetOracle,
            RoomMeta {
                id: "wet-oracle",
                title: "The Wet Oracle",
                wing: "Emergence",
                blurb: "A slime of agents deposits scent and climbs gradients between foods. Race it \
                        to the shortest path and lose (Tero 2010 Physarum). t grows the network.",
                accent: [120, 180, 90],
            }
        ),
        (
            conjecture_mill,
            ConjectureMill,
            RoomMeta {
                id: "conjecture-mill",
                title: "The Conjecture Mill",
                wing: "Number & Pattern",
                blurb: "Typed formulas crawl across a blackboard. Exact counterexamples erase the \
                        bad; coefficient proof stamps the survivor. Time runs a complete finite \
                        search.",
                accent: [120, 220, 170],
            }
        ),
        (
            tilt_cone,
            TiltCone,
            RoomMeta {
                id: "tilt-cone",
                title: "Tilt the Cone",
                wing: "Shape & Space",
                blurb: "Boost the frame: planes of simultaneity tip, light stays at 45 degrees, \
                        causality holds. Lorentz pair with Starbow.",
                accent: [100, 180, 220],
            }
        ),
        (
            the_stretch,
            TheStretch,
            RoomMeta {
                id: "the-stretch",
                title: "The Stretch",
                wing: "Shape & Space",
                blurb: "Click any galaxy: everyone is the center. Hubble flow makes the rest recede; \
                        redshift grows with distance. t sets H0; CLICK a galaxy to stand there.",
                accent: [180, 100, 160],
            }
        ),
        (
            laplace_clock,
            LaplaceClock,
            RoomMeta {
                id: "laplace-clock",
                title: "Laplace's Clockwork",
                wing: "Motion & Dynamics",
                blurb: "Io, Europa, Ganymede lock 1:2:4; the Laplace angle avoids the triple \
                        conjunction. t turns the clock.",
                accent: [220, 180, 100],
            }
        ),
        (
            message_heals,
            MessageHeals,
            RoomMeta {
                id: "message-heals",
                title: "The Message That Heals",
                wing: "Number & Pattern",
                blurb: "Hamming(7,4) parity bits heal single flips mid-flight until noise wins.",
                accent: [100, 200, 140],
            }
        ),
        (
            unlit_room,
            UnlitRoom,
            RoomMeta {
                id: "unlit-room",
                title: "The Unlit Room",
                wing: "Shape & Space",
                blurb: "Most rooms light everywhere from any lamp; Tokarsky built one that does not. \
                        A marked dark point stays unlit. t turns the beam.",
                accent: [80, 80, 120],
            }
        ),
        (
            the_lens,
            TheLens,
            RoomMeta {
                id: "the-lens",
                title: "The Lens",
                wing: "Shape & Space",
                blurb: "A mass you never see bends background light into Einstein rings and arcs. t \
                        grows the mass.",
                accent: [160, 140, 200],
            }
        ),
        (
            fourteen_beacons,
            FourteenBeacons,
            RoomMeta {
                id: "fourteen-beacons",
                title: "Fourteen Beacons",
                wing: "Shape & Space",
                blurb: "Fourteen pulsar ticks around the Sun, one longer home mark: a toy of the \
                        Pioneer plaque. t pulses the periods.",
                accent: [200, 200, 120],
            }
        ),
        (
            loneliness,
            Loneliness,
            RoomMeta {
                id: "loneliness",
                title: "The Loneliness Equation",
                wing: "Number & Pattern",
                blurb: "Seven Drake dials multiply to N. L, the lifetime of talkers, is drawn longer: \
                        silence can be scheduling, not scarcity. t grows L; DRAG a dial to retune N.",
                accent: [140, 140, 180],
            }
        ),
        (
            chord_game,
            ChordGame,
            RoomMeta {
                id: "chord-game",
                title: "The Chord Game",
                wing: "Number & Pattern",
                blurb: "Elliptic addition: chord two points on y^2 = x^3 + a x + b, flip the third \
                        intersection. The group law behind public-key crypto.",
                accent: [180, 160, 100],
            }
        ),
        (
            recaman,
            Recaman,
            RoomMeta {
                id: "recaman",
                title: "The Jumper",
                wing: "Number & Pattern",
                blurb: "Recaman's sequence: jump back by n if free, else forward. Nested arcs hide an \
                        open seat (852655). t grows terms.",
                accent: [200, 170, 90],
            }
        ),
        (
            truchet,
            Truchet,
            RoomMeta {
                id: "truchet",
                title: "The Weave",
                wing: "Emergence",
                blurb: "One tile, two rotations, a coin flip per cell: Truchet and 10 PRINT mazes from \
                        nothing. t drifts bias.",
                accent: [100, 180, 160],
            }
        ),
        (
            pursuit,
            Pursuit,
            RoomMeta {
                id: "pursuit",
                title: "The Chase",
                wing: "Motion & Dynamics",
                blurb: "Four bugs each walk toward the next: a logarithmic whirlpool where every path \
                        has the same length. t sets speed.",
                accent: [220, 120, 90],
            }
        ),
        (
            pascal_mod,
            PascalMod,
            RoomMeta {
                id: "pascal-mod",
                title: "The Divisor Fractal",
                wing: "Number & Pattern",
                blurb: "Pascal's triangle mod m: residue paints a fractal. mod 2 is Sierpinski; \
                        Kummer ties carries to the pattern. t grows rows.",
                accent: [160, 100, 180],
            }
        ),
        (
            three_gap,
            ThreeGap,
            RoomMeta {
                id: "three-gap",
                title: "The Spinner",
                wing: "Number & Pattern",
                blurb: "Points at n*theta on a circle show at most three gap sizes; the largest is the \
                        sum of the other two. t grows n.",
                accent: [120, 180, 200],
            }
        ),
        (
            morley,
            Morley,
            RoomMeta {
                id: "morley",
                title: "The Triangle That Cheats",
                wing: "Shape & Space",
                blurb: "Trisect any triangle's angles: the inner crossings form a perfect equilateral \
                        (Morley 1899). t wobbles vertices; DRAG A VERTEX.",
                accent: [200, 140, 80],
            }
        ),
        (
            menagerie,
            Menagerie,
            RoomMeta {
                id: "menagerie",
                title: "The Menagerie",
                wing: "Fractals",
                blurb: "Clifford attractor: four numbers and a long orbit condense a luminous alien. \
                        t drifts constants.",
                accent: [180, 90, 140],
            }
        ),
        (
            apollonian,
            Apollonian,
            RoomMeta {
                id: "apollonian",
                title: "The Kissing Circles",
                wing: "Number & Pattern",
                blurb: "Apollonian gasket: Descartes' theorem fills every gap with a kissing circle. \
                        Integer curvatures cascade. t deepens recursion; CLICK A GAP.",
                accent: [100, 160, 200],
            }
        ),
        (
            inversion,
            Inversion,
            RoomMeta {
                id: "inversion",
                title: "The Mirror That Bends",
                wing: "Shape & Space",
                blurb: "Circle inversion: lines become circles, infinity becomes a point. The hub of \
                        Apollonian and Steiner geometry. t drifts props.",
                accent: [140, 180, 220],
            }
        ),
        (
            dla_frost,
            DlaFrost,
            RoomMeta {
                id: "dla-frost",
                title: "The Frost",
                wing: "Emergence",
                blurb: "Diffusion-limited aggregation: random walkers freeze on contact and grow \
                        lightning and coral. t grows the swarm.",
                accent: [180, 220, 255],
            }
        ),
        (
            kaprekar,
            Kaprekar,
            RoomMeta {
                id: "kaprekar",
                title: "The Number That Eats Numbers",
                wing: "Number & Pattern",
                blurb: "Kaprekar's routine: rearrange digits large minus small. Every mixed 4-digit \
                        number falls to 6174 in at most seven steps. t picks a start.",
                accent: [220, 160, 60],
            }
        ),
        (
            steiner,
            Steiner,
            RoomMeta {
                id: "steiner",
                title: "The Ring That Always Closes",
                wing: "Shape & Space",
                blurb: "A Steiner chain of circles fits between two boundaries and closes from every \
                        angle. t sets count.",
                accent: [160, 200, 180],
            }
        ),
        (
            hopf,
            Hopf,
            RoomMeta {
                id: "hopf",
                title: "The Linked Rings",
                wing: "Shape & Space",
                blurb: "Hopf fibration: space filled with circles all linked, none touching. The \
                        shadow of S^3 and a picture of a qubit. t grows fibers.",
                accent: [180, 120, 200],
            }
        ),
        (
            wireworld,
            Wireworld,
            RoomMeta {
                id: "wireworld",
                title: "The Visible Computer",
                wing: "Emergence",
                blurb: "Wireworld: four states, electrons on copper, gates you can watch. t steps the \
                        clock.",
                accent: [255, 200, 40],
            }
        ),
        (
            buddhabrot,
            Buddhabrot,
            RoomMeta {
                id: "buddhabrot",
                title: "The Ghost in the Set",
                wing: "Fractals",
                blurb: "Buddhabrot: density of escaping Mandelbrot orbits paints a ghostly figure. t \
                        deepens iterations.",
                accent: [200, 180, 255],
            }
        ),
        (
            harmonics,
            Harmonics,
            RoomMeta {
                id: "harmonics",
                title: "The Singing Sphere",
                wing: "Waves & Sound",
                blurb: "Real spherical harmonics Y_lm: the lobes of atomic orbitals and of a ringing \
                        sphere. t lifts l.",
                accent: [100, 160, 255],
            }
        ),
        (
            function_painter,
            FunctionPainter,
            RoomMeta {
                id: "function-painter",
                title: "Function Painter",
                wing: "Fractals",
                blurb: "Domain coloring of complex maps: phase is symbol, magnitude is density. z^2, \
                        z^2+c, 1/z, sin z, e^z, z^3-1. t and DRAG pick the map and tune c.",
                accent: [255, 120, 180],
            }
        ),
        (
            newton,
            Newton,
            RoomMeta {
                id: "newton",
                title: "Newton's Basins",
                wing: "Fractals",
                blurb: "Newton's method on z^n-1 paints which root each seed finds. Basin boundaries \
                        are fractal.",
                accent: [255, 100, 80],
            }
        ),
        (
            koch,
            Koch,
            RoomMeta {
                id: "koch",
                title: "The Infinite Coast",
                wing: "Fractals",
                blurb: "Koch snowflake: every generation multiplies the coast by 4/3. Perimeter runs \
                        away; area stays finite.",
                accent: [140, 200, 255],
            }
        ),
        (
            hilbert,
            Hilbert,
            RoomMeta {
                id: "hilbert",
                title: "The Space-Filling Path",
                wing: "Shape & Space",
                blurb: "Hilbert curve: a continuous path that fills the square in the limit. Finite \
                        folds approximate without crossing.",
                accent: [180, 140, 255],
            }
        ),
        (
            gray_scott,
            GrayScott,
            RoomMeta {
                id: "gray-scott",
                title: "The Chemical Garden",
                wing: "Emergence",
                blurb: "Gray-Scott reaction-diffusion: two chemicals paint spots, stripes, and coral. \
                        t drifts feed/kill.",
                accent: [80, 200, 160],
            }
        ),
        (
            sieve,
            Sieve,
            RoomMeta {
                id: "sieve",
                title: "The Sieve",
                wing: "Number & Pattern",
                blurb: "Eratosthenes: cross out multiples, primes remain. \
                        Variation shifts the strike animation seed.",
                accent: [220, 180, 60],
            }
        ),
        (
            curse_dimension,
            CurseDimension,
            RoomMeta {
                id: "curse-dimension",
                title: "The Curse of Dimension",
                wing: "Shape & Space",
                blurb: "Almost all volume of a high-D ball sits in a thin shell; the middle empties.",
                accent: [200, 100, 220],
            }
        ),
        (
            concentration,
            Concentration,
            RoomMeta {
                id: "concentration",
                title: "The Concentration Bell",
                wing: "Number & Pattern",
                blurb: "Random points in high dimension all sit near the same radius; extremes die. t \
                        raises d.",
                accent: [100, 180, 255],
            }
        ),
        (
            uncertainty,
            Uncertainty,
            RoomMeta {
                id: "uncertainty",
                title: "The Uncertainty Dial",
                wing: "Waves & Sound",
                blurb: "Narrower in time, wider in frequency: you cannot own both.",
                accent: [255, 200, 80],
            }
        ),
        (
            gradient_valley,
            GradientValley,
            RoomMeta {
                id: "gradient-valley",
                title: "The Gradient Valley",
                wing: "Number & Pattern",
                blurb: "Descent finds a basin; a ridge blocks another. The landscape lies to the \
                        seeker. t drifts start.",
                accent: [80, 160, 120],
            }
        ),
        (
            attention,
            Attention,
            RoomMeta {
                id: "attention",
                title: "Attention as Soft Light",
                wing: "Number & Pattern",
                blurb: "One query lights a few keys; the rest go dim. Softmax weights are the story. t \
                        warms temperature.",
                accent: [255, 220, 100],
            }
        ),
        (
            braess,
            Braess,
            RoomMeta {
                id: "braess",
                title: "Braess Trap",
                wing: "Emergence",
                blurb: "Add a free shortcut and selfish drivers can all take longer. t toggles the \
                        bridge.",
                accent: [220, 80, 80],
            }
        ),
        (
            nontransitive,
            Nontransitive,
            RoomMeta {
                id: "nontransitive",
                title: "Nontransitive Dice",
                wing: "Number & Pattern",
                blurb: "A beats B, B beats C, C beats A: ranking collapses. t runs trials.",
                accent: [200, 140, 60],
            }
        ),
        (
            parrondo,
            Parrondo,
            RoomMeta {
                id: "parrondo",
                title: "Parrondo's Trap",
                wing: "Number & Pattern",
                // ABB is exactly what policy_wager asks the player to call, so
                // the doorway names the question and not the answer.
                blurb: "Two games that each lose on their own. Schedule them and the pile can climb.",
                accent: [180, 100, 200],
            }
        ),
        (
            hilbert_hotel,
            HilbertHotel,
            RoomMeta {
                id: "hilbert-hotel",
                title: "Hilbert's Hotel",
                wing: "Number & Pattern",
                blurb: "Full hotel, room for one more bus, until the reals check in.",
                accent: [160, 120, 200],
            }
        ),
        (
            soap_film,
            SoapFilm,
            RoomMeta {
                id: "soap-film",
                title: "Soap Film",
                wing: "Shape & Space",
                blurb: "A film finds least length; Steiner junctions meet at 120 degrees. t wobbles \
                        pins.",
                accent: [180, 220, 255],
            }
        ),
        (
            landauer,
            Landauer,
            RoomMeta {
                id: "landauer",
                title: "Landauer's Price",
                wing: "Number & Pattern",
                blurb: "Erase a bit, pay heat: kT ln 2 per irreversible forget. t grows the register; \
                        a click forgets one bit.",
                accent: [255, 120, 60],
            }
        ),
        (
            prime_gaps,
            PrimeGaps,
            RoomMeta {
                id: "prime-gaps",
                title: "Prime Gap Weather",
                wing: "Number & Pattern",
                blurb: "Gaps between primes as a landscape; twins are calm. Open \
                        doors stay open.",
                accent: [100, 200, 140],
            }
        ),
        (
            sphere_eversion,
            SphereEversion,
            RoomMeta {
                id: "sphere-eversion",
                title: "Sphere Eversion",
                wing: "Shape & Space",
                blurb: "A sphere can turn inside out without creases if you allow it to pass through \
                        itself smoothly.",
                accent: [120, 180, 255],
            }
        ),
        (
            causal_doors,
            CausalDoors,
            RoomMeta {
                id: "causal-doors",
                title: "Causal Doors",
                wing: "Number & Pattern",
                blurb: "Watching is not intervening. Force rain or the sprinkler and wetness answers \
                        differently.",
                accent: [100, 160, 200],
            }
        ),
        (
            soft_proof,
            SoftProof,
            RoomMeta {
                id: "soft-proof",
                title: "The Soft Proof",
                wing: "Shape & Space",
                blurb: "Homotopy: continuously deform a path without tearing endpoints free. t sets \
                        the stage.",
                accent: [200, 160, 220],
            }
        ),
        (
            learning_clock,
            LearningClock,
            RoomMeta {
                id: "learning-clock",
                title: "The Learning Clock",
                wing: "Number & Pattern",
                blurb: "Train task A, then B: does A survive? Continual learning as a felt trade.",
                accent: [80, 200, 160],
            }
        ),
        (
            duality,
            Duality,
            RoomMeta {
                id: "duality",
                title: "Two Descriptions, One Truth",
                wing: "Shape & Space",
                blurb: "One polygon, two languages: faces become vertices in the dual.",
                accent: [180, 140, 255],
            }
        ),
        (
            mirror_forms,
            MirrorForms,
            RoomMeta {
                id: "mirror-forms",
                title: "The Mirror of Forms",
                wing: "Shape & Space",
                blurb: "Objects and arrows; compose two maps into one path. Category-lite without a \
                        jargon wall.",
                accent: [200, 180, 100],
            }
        ),
        (
            penrose,
            Penrose,
            RoomMeta {
                id: "penrose",
                title: "The Aperiodic Floor",
                wing: "Shape & Space",
                blurb: "Penrose kites from Robinson triangles: inflation never yields a lattice.",
                accent: [220, 180, 60],
            }
        ),
        (
            continued_frac,
            ContinuedFrac,
            RoomMeta {
                id: "continued-frac",
                title: "The Ladder of Approximations",
                wing: "Number & Pattern",
                blurb: "Continued fractions peel best rationals from a real. Golden is the hardest.",
                accent: [160, 200, 100],
            }
        ),
        (
            logistic_cobweb,
            LogisticCobweb,
            RoomMeta {
                id: "logistic-cobweb",
                title: "The Cobweb",
                wing: "Motion & Dynamics",
                blurb: "Logistic map as cobweb: climb the parabola, slide to y=x.",
                accent: [255, 140, 80],
            }
        ),
        (
            sierpinski_carpet,
            SierpinskiCarpet,
            RoomMeta {
                id: "sierpinski-carpet",
                title: "The Carpet",
                wing: "Fractals",
                blurb: "Sierpinski carpet: punch the middle ninth, forever. Area vanishes; dimension \
                        stays between 1 and 2.",
                accent: [200, 100, 140],
            }
        ),
        (
            pythagoras_tree,
            PythagorasTree,
            RoomMeta {
                id: "pythagoras-tree",
                title: "The Pythagoras Tree",
                wing: "Fractals",
                blurb: "Squares on the sides of right triangles branch forever.",
                accent: [80, 180, 100],
            }
        ),
        (
            ulam_spiral,
            UlamSpiral,
            RoomMeta {
                id: "ulam-spiral",
                title: "The Ulam Spiral",
                wing: "Number & Pattern",
                blurb: "Naturals on a square spiral; primes light diagonals.",
                accent: [100, 140, 255],
            }
        ),
        (
            prime_spirals,
            PrimeSpirals,
            RoomMeta {
                id: "prime-spirals",
                title: "Prime Spirals",
                wing: "Number & Pattern",
                blurb: "Whole numbers spiral from the center and only primes light up. Click a point \
                        to trace the two prime-rich diagonals crossing there.",
                accent: [190, 70, 170],
            }
        ),
        (
            cult_of_pi,
            CultOfPi,
            RoomMeta {
                id: "cult-of-pi",
                title: "Cult of Pi",
                wing: "Number & Pattern",
                blurb: "The exact digits of pi enter a finite channel, age, and develop faults. Click to restore and hold one local patch exact, but no finite screen can ever contain all of pi.",
                accent: [40, 210, 90],
            }
        ),
        (
            collatz,
            Collatz,
            RoomMeta {
                id: "collatz",
                title: "Collatz",
                wing: "Emergence",
                blurb: "Halve it if even, triple it and add one if odd, and repeat. Every tested \
                        start reaches 1, but nobody has proved that all do. t picks the number.",
                accent: [220, 130, 50],
            }
        ),
        (
            buffon_needle,
            BuffonNeedle,
            RoomMeta {
                id: "buffon-needle",
                title: "Buffon's Needle",
                wing: "Chance & Order",
                // Pi is the number number_wager asks the player to call, so the
                // doorway keeps the strangeness and drops the answer.
                blurb: "Throw sticks onto parallel floorboards. Bright sticks cross a line; the ratio \
                        of crossings to throws settles on one exact number, with no circle \
                        anywhere in sight.",
                accent: [140, 100, 230],
            }
        ),
        (
            game_of_life,
            GameOfLife,
            RoomMeta {
                id: "game-of-life",
                title: "Game of Life",
                wing: "Emergence",
                blurb: "Aim at a quiet patch and place five living cells. Birth with 3 neighbors and \
                        survival with 2 or 3 make that glider move by itself.",
                accent: [90, 210, 120],
            }
        ),
        (
            sandpile,
            Sandpile,
            RoomMeta {
                id: "sandpile",
                title: "The Sandpile",
                wing: "Emergence",
                blurb: "Drop grains; four topples to neighbors; self-organized criticality blooms a \
                        fractal mandala. Catastrophe is the resting state. t pours the center; HOLD \
                        pours under the hand.",
                accent: [220, 170, 70],
            }
        ),
        (
            mandelbrot,
            Mandelbrot,
            RoomMeta {
                id: "mandelbrot",
                title: "Mandelbrot Set",
                wing: "Fractals & the Infinite",
                blurb: "Iterate z into z squared plus c and ask if it stays bounded. The points that \
                        do form the most complex object in mathematics. t zooms toward the seahorses.",
                accent: [70, 130, 255],
            }
        ),
        (
            julia,
            Julia,
            RoomMeta {
                id: "julia",
                title: "Julia Set",
                wing: "Fractals & the Infinite",
                blurb: "The same rule as Mandelbrot, but c is fixed and the whole plane is the seed. \
                        Every c grows a different fractal; t walks c around a circle to morph it.",
                accent: [255, 120, 60],
            }
        ),
        (
            barnsley_fern,
            BarnsleyFern,
            RoomMeta {
                id: "barnsley-fern",
                title: "Barnsley Fern",
                wing: "Fractals & the Infinite",
                blurb: "Pick one of four simple transformations at random, over and over, and a fern \
                        grows out of the noise. Click to plant a smaller self-similar fern.",
                accent: [60, 200, 90],
            }
        ),
        (
            lsystem,
            LSystemGarden,
            RoomMeta {
                id: "lsystem-garden",
                title: "L-System Garden",
                wing: "Emergence",
                blurb: "A one-line grammar rewrites itself; branches, curves and plants grow from nothing. Poke to plant or bend. Simple symbols, infinite form.",
                accent: [80, 180, 120],
            }
        ),
        (
            harmonograph,
            Harmonograph,
            RoomMeta {
                id: "harmonograph",
                title: "Harmonograph",
                wing: "Waves & Sound",
                blurb: "Two dying oscillations on each axis draw a curve that spirals inward as the \
                        pendulums lose energy. t detunes the frequencies to open and close the weave.",
                accent: [200, 80, 180],
            }
        ),
        (
            logistic_map,
            LogisticMap,
            RoomMeta {
                id: "logistic-map",
                title: "Logistic Map",
                wing: "Chaos & Order",
                blurb: "Sweep the growth rate of x into r x (1 - x) across the screen and plot where \
                        the population lands: one value, then two, then four, then chaos. t zooms in.",
                accent: [230, 200, 60],
            }
        ),
        (
            langtons_ant,
            LangtonsAnt,
            RoomMeta {
                id: "langtons-ant",
                title: "Langton's Ant",
                wing: "Emergence",
                blurb: "One ant, two rules: turn on the color under you, flip it, step. It makes chaos \
                        for ten thousand steps and then builds a highway forever. t runs the clock.",
                accent: [120, 200, 220],
            }
        ),
        (
            lorenz,
            Lorenz,
            RoomMeta {
                id: "lorenz",
                title: "Lorenz Attractor",
                wing: "Chaos & Order",
                blurb: "Three equations for toy weather. The path never repeats and never escapes its \
                        butterfly-shaped set. t raises the parameter through the onset of chaos.",
                accent: [80, 180, 230],
            }
        ),
        (
            arecibo,
            Arecibo,
            RoomMeta {
                id: "arecibo",
                title: "Arecibo Message",
                wing: "Signals & Codes",
                blurb: "A stream of bits that looks like noise until you line it up at the right width. \
                        The length is a semiprime, so it has one nontrivial rectangle up to rotation.",
                accent: [120, 230, 180],
            }
        ),
        (
            the_pour,
            ThePour,
            RoomMeta {
                id: "the-pour",
                title: "The Pour",
                wing: "Change",
                blurb: "A curve holds water. t pours area in from the left; the rising total traces a \
                        second curve above. You are watching the fundamental theorem of calculus.",
                accent: [80, 160, 255],
            }
        ),
        (
            slope_rider,
            SlopeRider,
            RoomMeta {
                id: "slope-rider",
                title: "Slope Rider",
                wing: "Change",
                blurb: "Ride the tangent line along a curve. The board's tilt is the slope, and the \
                        tilt traces its own curve below as you go: the derivative, drawing itself.",
                accent: [255, 190, 70],
            }
        ),
        (
            double_pendulum,
            DoublePendulum,
            RoomMeta {
                id: "double-pendulum",
                title: "Double Pendulum",
                wing: "Chaos & Order",
                blurb: "One pendulum hanging from another. A deterministic integration shows how a \
                        shadow twin a breath away can peel off before your eyes.",
                accent: [255, 110, 110],
            }
        ),
        (
            epicycles,
            Epicycles,
            RoomMeta {
                id: "epicycles",
                title: "Fourier Epicycles",
                wing: "Waves & Sound",
                blurb: "Circles on circles, each spinning at its own speed, and the tip of the chain \
                        draws a star. Fourier proved the circles can draw anything. t runs the pen.",
                accent: [180, 130, 255],
            }
        ),
        (
            random_walk,
            RandomWalk,
            RoomMeta {
                id: "random-walk",
                title: "Random Walk",
                wing: "Chance & Order",
                blurb: "Sixty walkers stumble one random step at a time. None knows where it is going; \
                        together they obey the square root law. The circle is the law; t is the clock.",
                accent: [140, 220, 160],
            }
        ),
        (
            voronoi,
            Voronoi,
            RoomMeta {
                id: "voronoi",
                title: "Voronoi Territories",
                wing: "Shape & Space",
                blurb: "Fourteen wells in a desert; every point belongs to its nearest one. The \
                        borders are the ties. Giraffes, dragonflies, and mud cracks all know this map.",
                accent: [235, 180, 90],
            }
        ),
        (
            mobius,
            Mobius,
            RoomMeta {
                id: "mobius",
                title: "Mobius Strip",
                wing: "Shape & Space",
                blurb: "A band with a half twist: one side, one edge. The ant walks a full lap and \
                        arrives on the other side without crossing anything. Two laps to get home.",
                accent: [120, 200, 255],
            }
        ),
        (
            zeno,
            Zeno,
            RoomMeta {
                id: "zeno",
                title: "Zeno's Square",
                wing: "Change",
                blurb: "Half the square, then half of what's left, then half of that, forever. \
                        Infinitely many tiles, and they fit exactly. The sum of the halves is one.",
                accent: [200, 160, 255],
            }
        ),
        (
            goldbach,
            Goldbach,
            RoomMeta {
                id: "goldbach",
                title: "Goldbach's Comet",
                wing: "Open Problems",
                blurb: "Every even number, tested: how many ways is it two primes? The counts plot \
                        into a comet. That it never touches zero is unproven. Nobody knows. Go on.",
                accent: [255, 220, 140],
            }
        ),
        (
            quine,
            Quine,
            RoomMeta {
                id: "quine",
                title: "The Quine",
                wing: "Mind & Computation",
                blurb: "A circle that draws a smaller copy of itself inside; the copy draws a smaller copy, forever. A finite rule that contains its own description at every scale.",
                accent: [200, 150, 255],
            }
        ),
        (
            strange_loop,
            StrangeLoop,
            RoomMeta {
                id: "strange-loop",
                title: "Strange Loop",
                wing: "Mind & Computation",
                blurb: "A U that contains a smaller U that contains a smaller U... A finite rule that loops back to itself across levels. This is how 'I' might emerge from symbols referring to symbols.",
                accent: [180, 100, 255],
            }
        ),
        (
            dragon_curve,
            DragonCurve,
            RoomMeta {
                id: "dragon-curve",
                title: "The Paper Dragon",
                wing: "Fractals",
                blurb: "Heighway dragon: fold paper right, then reverse-complement.",
                accent: [220, 60, 80],
            }
        ),
        (
            fibonacci_word,
            FibonacciWord,
            RoomMeta {
                id: "fibonacci-word",
                title: "The Rabbit Sequence",
                wing: "Number & Pattern",
                blurb: "Fibonacci word: 0, 01, 010, 01001, ... the mechanical word of the golden slope.",
                accent: [200, 160, 80],
            }
        ),
        (
            newton_basins_cubic,
            NewtonCubic,
            RoomMeta {
                id: "newton-cubic",
                title: "Cubic Newton",
                wing: "Fractals",
                blurb: "Newton basins for z^3+c: three attractors paint a cubic portrait.",
                accent: [255, 90, 120],
            }
        ),
        (
            henon,
            Henon,
            RoomMeta {
                id: "henon",
                title: "The Henon Map",
                wing: "Fractals",
                blurb: "Henon attractor: one quadratic map, a folded horseshoe of chaos.",
                accent: [180, 100, 200],
            }
        ),
        (
            rules30,
            Rules30,
            RoomMeta {
                id: "rule-30",
                title: "Rule 30",
                wing: "Emergence",
                blurb: "Elementary cellular automaton Rule 30: one black cell becomes structured \
                        chaos.",
                accent: [40, 40, 40],
            }
        ),
        (
            mandelbulb_slice,
            MandelbulbSlice,
            RoomMeta {
                id: "mandelbulb-slice",
                title: "Mandelbulb Slice",
                wing: "Fractals",
                blurb: "A plane cut through the power-8 Mandelbulb.",
                accent: [160, 80, 220],
            }
        ),
        (
            thue_morse,
            ThueMorse,
            RoomMeta {
                id: "thue-morse",
                title: "Thue-Morse Weather",
                wing: "Number & Pattern",
                blurb: "Parity of binary digit sum: cube-free automatic sequence.",
                accent: [80, 180, 120],
            }
        ),
        (
            rossler,
            Rossler,
            RoomMeta {
                id: "rossler",
                title: "The Rossler Scroll",
                wing: "Motion & Dynamics",
                blurb: "One-scroll chaotic attractor in three dimensions, projected.",
                accent: [200, 100, 40],
            }
        ),
        (
            cantor_set,
            CantorSet,
            RoomMeta {
                id: "cantor-set",
                title: "The Devil's Staircase",
                wing: "Fractals",
                blurb: "Middle-third Cantor dust above; Cantor function (devil's staircase) below.",
                accent: [160, 40, 200],
            }
        ),
        (
            weierstrass,
            Weierstrass,
            RoomMeta {
                id: "weierstrass",
                title: "Nowhere Smooth",
                wing: "Fractals",
                blurb: "Weierstrass sum: continuous everywhere, differentiable nowhere.",
                accent: [40, 120, 200],
            }
        ),
        (
            peano_curve,
            PeanoCurve,
            RoomMeta {
                id: "peano-curve",
                title: "Peano's Path",
                wing: "Fractals",
                blurb: "A continuous curve that fills the square (order recursion).",
                accent: [100, 200, 80],
            }
        ),
        (
            van_der_pol,
            VanDerPol,
            RoomMeta {
                id: "van-der-pol",
                title: "Van der Pol Cycle",
                wing: "Motion & Dynamics",
                blurb: "Nonlinear damping births a stable limit cycle.",
                accent: [220, 160, 40],
            }
        ),
        (
            ikeda,
            Ikeda,
            RoomMeta {
                id: "ikeda",
                title: "The Ikeda Map",
                wing: "Motion & Dynamics",
                blurb: "Dissipative complex map from laser cavities: a curly strange attractor.",
                accent: [60, 180, 200],
            }
        ),
        (
            duffing,
            Duffing,
            RoomMeta {
                id: "duffing",
                title: "The Duffing Well",
                wing: "Motion & Dynamics",
                blurb: "Driven cubic oscillator: double-well chaos under strong drive.",
                accent: [180, 80, 40],
            }
        ),
        (
            levy_c,
            LevyC,
            RoomMeta {
                id: "levy-c",
                title: "The Levy C Curve",
                wing: "Fractals",
                blurb: "Self-similar C from the rewrite F -> +F--F+.",
                accent: [40, 160, 220],
            }
        ),
        (
            tinkerbell,
            Tinkerbell,
            RoomMeta {
                id: "tinkerbell",
                title: "Tinkerbell Map",
                wing: "Motion & Dynamics",
                blurb: "Quadratic planar map with a butterfly-shaped attractor.",
                accent: [220, 120, 180],
            }
        ),
        (
            gingerbread,
            Gingerbread,
            RoomMeta {
                id: "gingerbread",
                title: "Gingerbreadman Map",
                wing: "Motion & Dynamics",
                blurb: "Piecewise-linear map whose orbit sketches a cookie silhouette.",
                accent: [180, 100, 40],
            }
        ),
        (
            menger_slice,
            MengerSlice,
            RoomMeta {
                id: "menger-slice",
                title: "Menger Face",
                wing: "Fractals",
                blurb: "Face of the Menger sponge: remove center squares forever.",
                accent: [100, 100, 160],
            }
        ),
        (
            bifurcation,
            Bifurcation,
            RoomMeta {
                id: "bifurcation",
                title: "Bifurcation Weather",
                wing: "Motion & Dynamics",
                blurb: "Logistic map long-term x as r sweeps: period doubling into chaos.",
                accent: [200, 40, 80],
            }
        ),
        (
            stern_brocot,
            SternBrocot,
            RoomMeta {
                id: "stern-brocot",
                title: "Stern-Brocot Tree",
                wing: "Number & Pattern",
                blurb: "Every positive rational once, via mediants of 0/1 and 1/0.",
                accent: [80, 140, 200],
            }
        ),
        (
            josephus,
            Josephus,
            RoomMeta {
                id: "josephus",
                title: "Josephus Circle",
                wing: "Number & Pattern",
                blurb: "Every k-th seat is removed until one remains.",
                accent: [160, 40, 40],
            }
        ),
        (
            calkin_wilf,
            CalkinWilf,
            RoomMeta {
                id: "calkin-wilf",
                title: "Calkin-Wilf Tree",
                wing: "Number & Pattern",
                blurb: "Every positive rational once via left a/(a+b) and right (a+b)/b.",
                accent: [40, 160, 140],
            }
        ),
        (
            fourier_square,
            FourierSquare,
            RoomMeta {
                id: "fourier-square",
                title: "Gibbs Overshoot",
                wing: "Waves & Sound",
                blurb: "Odd-harmonic Fourier sums toward a square wave; ringing refuses to die.",
                accent: [40, 100, 220],
            }
        ),
        (
            sierpinski_arrowhead,
            SierpinskiArrowhead,
            RoomMeta {
                id: "sierpinski-arrowhead",
                title: "Sierpinski Arrowhead",
                wing: "Fractals",
                blurb: "A continuous path whose limit is the Sierpinski gasket.",
                accent: [200, 80, 40],
            }
        ),
        (
            clifford,
            Clifford,
            RoomMeta {
                id: "clifford",
                title: "Clifford Attractor",
                wing: "Motion & Dynamics",
                blurb: "Sin/cos iterated map with dense organic attractors.",
                accent: [80, 200, 160],
            }
        ),
        (
            dejong,
            DeJong,
            RoomMeta {
                id: "dejong",
                title: "Peter de Jong",
                wing: "Motion & Dynamics",
                blurb: "Sin/cos map pair that paints dense filament clouds.",
                accent: [200, 160, 40],
            }
        ),
        (
            svensson,
            Svensson,
            RoomMeta {
                id: "svensson",
                title: "Svensson Map",
                wing: "Motion & Dynamics",
                blurb: "Trigonometric map with dense attractor clouds.",
                accent: [160, 80, 200],
            }
        ),
        (
            bedhead,
            Bedhead,
            RoomMeta {
                id: "bedhead",
                title: "Bedhead Attractor",
                wing: "Motion & Dynamics",
                blurb: "Soft pillow-shaped strange attractor from a trig map.",
                accent: [180, 120, 80],
            }
        ),
        (
            hopalong,
            Hopalong,
            RoomMeta {
                id: "hopalong",
                title: "Hopalong Attractor",
                wing: "Motion & Dynamics",
                blurb: "Martin hopalong map: absolute-value folds into a hoppy cloud.",
                accent: [40, 180, 100],
            }
        ),
        (
            gumowski_mira,
            GumowskiMira,
            RoomMeta {
                id: "gumowski-mira",
                title: "Gumowski-Mira",
                wing: "Motion & Dynamics",
                blurb: "Accelerator beam map that paints butterfly-like attractors.",
                accent: [100, 60, 180],
            }
        ),
        (
            pickover,
            Pickover,
            RoomMeta {
                id: "pickover",
                title: "Pickover Attractor",
                wing: "Motion & Dynamics",
                blurb: "Clifford Pickover's nested trig map, projected.",
                accent: [220, 100, 60],
            }
        ),
        (
            aizawa,
            Aizawa,
            RoomMeta {
                id: "aizawa",
                title: "Aizawa Ring",
                wing: "Motion & Dynamics",
                blurb: "Continuous 3D chaos with a ring-like attractor, projected.",
                accent: [60, 140, 200],
            }
        ),
        (
            thomas,
            Thomas,
            RoomMeta {
                id: "thomas",
                title: "Thomas Attractor",
                wing: "Motion & Dynamics",
                blurb: "Cyclically symmetric continuous chaos.",
                accent: [80, 200, 120],
            }
        ),
        (
            halvorsen,
            Halvorsen,
            RoomMeta {
                id: "halvorsen",
                title: "Halvorsen Attractor",
                wing: "Motion & Dynamics",
                blurb: "Continuous cyclic chaos with quadratic folds.",
                accent: [200, 80, 100],
            }
        ),
        (
            rabinovich_fabrikant,
            RabinovichFabrikant,
            RoomMeta {
                id: "rabinovich-fabrikant",
                title: "Rabinovich-Fabrikant",
                wing: "Motion & Dynamics",
                blurb: "Cubic continuous chaos from plasma physics.",
                accent: [180, 60, 140],
            }
        ),
        (
            three_scroll,
            ThreeScroll,
            RoomMeta {
                id: "three-scroll",
                title: "Three-Scroll Chaos",
                wing: "Motion & Dynamics",
                blurb: "Continuous multi-scroll chaotic flow, projected.",
                accent: [100, 80, 220],
            }
        ),
        (
            lozi,
            Lozi,
            RoomMeta {
                id: "lozi",
                title: "The Lozi Map",
                wing: "Motion & Dynamics",
                blurb: "Piecewise-linear Henon: absolute value folds the plane.",
                accent: [200, 80, 60],
            }
        ),
        (
            baker,
            Baker,
            RoomMeta {
                id: "baker",
                title: "Baker's Map",
                wing: "Motion & Dynamics",
                blurb: "Stretch the square, cut, and stack: classic chaos on [0,1]^2.",
                accent: [180, 120, 40],
            }
        ),
        (
            tent_map,
            TentMap,
            RoomMeta {
                id: "tent-map",
                title: "The Tent Map",
                wing: "Motion & Dynamics",
                blurb: "Piecewise-linear map on [0,1]: cobweb and density.",
                accent: [40, 160, 80],
            }
        ),
        (
            circle_map,
            CircleMap,
            RoomMeta {
                id: "circle-map",
                title: "Arnold Circle Map",
                wing: "Motion & Dynamics",
                blurb: "Mode locking and winding-number staircase on the circle.",
                accent: [80, 100, 220],
            }
        ),
        (
            standard_map,
            StandardMap,
            RoomMeta {
                id: "standard-map",
                title: "Chirikov Map",
                wing: "Motion & Dynamics",
                blurb: "Kicked rotor on a torus: KAM curves break into chaos.",
                accent: [160, 40, 160],
            }
        ),
        (
            elliptical_billiard,
            EllipticalBilliard,
            RoomMeta {
                id: "elliptical-billiard",
                title: "Elliptical Billiard",
                wing: "Shape & Space",
                blurb: "Bounces in an ellipse; caustics and foci.",
                accent: [40, 140, 180],
            }
        ),
        (
            horseshoe,
            Horseshoe,
            RoomMeta {
                id: "horseshoe",
                title: "Smale Horseshoe",
                wing: "Motion & Dynamics",
                blurb: "Stretch and fold a square into a horseshoe: chaos geometry.",
                accent: [200, 100, 40],
            }
        ),
        (
            logistic_orbit,
            LogisticOrbit,
            RoomMeta {
                id: "logistic-orbit",
                title: "Logistic Orbit",
                wing: "Motion & Dynamics",
                blurb: "Return-map cobweb of the logistic map with period guess.",
                accent: [220, 60, 100],
            }
        ),
        (
            sinai_billiard,
            SinaiBilliard,
            RoomMeta {
                id: "sinai-billiard",
                title: "Sinai Billiard",
                wing: "Shape & Space",
                blurb: "Square table with a circular scatterer: hard chaos.",
                accent: [100, 60, 40],
            }
        ),
        (
            henon_heiles,
            HenonHeiles,
            RoomMeta {
                id: "henon-heiles",
                title: "Henon-Heiles",
                wing: "Motion & Dynamics",
                blurb: "Galactic potential toy: energy steers order into chaos.",
                accent: [80, 40, 180],
            }
        ),
        (
            quadratic_map,
            QuadraticMap,
            RoomMeta {
                id: "quadratic-map",
                title: "Quadratic Map",
                wing: "Motion & Dynamics",
                blurb: "Real map x -> x^2 + c: Mandelbrot's one-dimensional cousin.",
                accent: [60, 100, 200],
            }
        ),
        (
            doubling_map,
            DoublingMap,
            RoomMeta {
                id: "doubling-map",
                title: "Angle Doubling",
                wing: "Motion & Dynamics",
                blurb: "Bernoulli shift theta -> 2 theta mod 1: expanding chaos.",
                accent: [40, 180, 160],
            }
        ),
        (
            gauss_map,
            GaussMap,
            RoomMeta {
                id: "gauss-map",
                title: "Gauss Map",
                wing: "Number & Pattern",
                blurb: "Continued-fraction engine: x -> frac(1/x).",
                accent: [120, 80, 200],
            }
        ),
        (
            manneville,
            Manneville,
            RoomMeta {
                id: "manneville",
                title: "Manneville Map",
                wing: "Motion & Dynamics",
                blurb: "Intermittency: long laminar waits, then chaotic bursts.",
                accent: [200, 140, 40],
            }
        ),
        (
            coupled_tent,
            CoupledTent,
            RoomMeta {
                id: "coupled-tent",
                title: "Coupled Tents",
                wing: "Motion & Dynamics",
                blurb: "Two tent maps with coupling: sync or independent chaos.",
                accent: [40, 160, 120],
            }
        ),
        (
            koch_snowflake,
            KochSnowflake,
            RoomMeta {
                id: "koch-snowflake",
                title: "Koch Snowflake",
                wing: "Fractals",
                blurb: "Closed Koch curve: infinite coast, finite area.",
                accent: [100, 180, 220],
            }
        ),
        (
            cesaro,
            Cesaro,
            RoomMeta {
                id: "cesaro",
                title: "Cesaro Fractal",
                wing: "Fractals",
                blurb: "Torn square: Koch rewrite with right angles.",
                accent: [180, 100, 60],
            }
        ),
        (
            minkowski,
            Minkowski,
            RoomMeta {
                id: "minkowski-sausage",
                title: "Minkowski Sausage",
                wing: "Fractals",
                blurb: "Quadratic Koch sausage: a thick fractal polyline.",
                accent: [160, 120, 40],
            }
        ),
        (
            bogdanov,
            Bogdanov,
            RoomMeta {
                id: "bogdanov",
                title: "Bogdanov Map",
                wing: "Motion & Dynamics",
                blurb: "Planar discrete map with a classic chaotic gallery.",
                accent: [180, 60, 100],
            }
        ),
        (
            kaplan_yorke,
            KaplanYorke,
            RoomMeta {
                id: "kaplan-yorke",
                title: "Kaplan-Yorke Map",
                wing: "Motion & Dynamics",
                blurb: "Doubling in x, damped drive in y: fractal attractor.",
                accent: [80, 160, 200],
            }
        ),
        (
            ricker,
            Ricker,
            RoomMeta {
                id: "ricker",
                title: "Ricker Map",
                wing: "Motion & Dynamics",
                blurb: "Population boom-bust: x exp(r(1-x)).",
                accent: [40, 160, 80],
            }
        ),
        (
            farey,
            Farey,
            RoomMeta {
                id: "farey",
                title: "Farey Sequence",
                wing: "Number & Pattern",
                blurb: "All reduced fractions up to denominator Q as a comb.",
                accent: [100, 120, 220],
            }
        ),
        (
            gosper,
            Gosper,
            RoomMeta {
                id: "gosper",
                title: "Gosper Curve",
                wing: "Fractals",
                blurb: "Flowsnake: space-filling path on a hexagonal lattice.",
                accent: [60, 180, 100],
            }
        ),
        (
            sierpinski_tri,
            SierpinskiTri,
            RoomMeta {
                id: "sierpinski-tri",
                title: "Sierpinski Triangle",
                wing: "Fractals",
                blurb: "Recursive midpoint gasket (not the chaos game).",
                accent: [200, 80, 60],
            }
        ),
        (
            burning_ship,
            BurningShip,
            RoomMeta {
                id: "burning-ship",
                title: "Burning Ship",
                wing: "Fractals",
                blurb: "Absolute-value Mandelbrot cousin with a ship silhouette.",
                accent: [200, 40, 40],
            }
        ),
        (
            tricorn,
            Tricorn,
            RoomMeta {
                id: "tricorn",
                title: "Tricorn",
                wing: "Fractals",
                blurb: "Mandelbar set: conjugate squaring, three-lobed body.",
                accent: [120, 60, 200],
            }
        ),
        (
            multibrot,
            Multibrot,
            RoomMeta {
                id: "multibrot",
                title: "Multibrot",
                wing: "Fractals",
                blurb: "z^d + c: Mandelbrot power raised.",
                accent: [160, 40, 180],
            }
        ),
        (
            phoenix,
            Phoenix,
            RoomMeta {
                id: "phoenix",
                title: "Phoenix Fractal",
                wing: "Fractals",
                blurb: "Escape set with a one-step memory of z.",
                accent: [220, 120, 40],
            }
        ),
        (
            lyapunov,
            Lyapunov,
            RoomMeta {
                id: "lyapunov",
                title: "Lyapunov Weather",
                wing: "Motion & Dynamics",
                blurb: "Logistic Lyapunov exponent lambda(r): chaos when positive.",
                accent: [200, 40, 120],
            }
        ),
        (
            collatz_tree,
            CollatzTree,
            RoomMeta {
                id: "collatz-tree",
                title: "Collatz Tree",
                wing: "Number & Pattern",
                blurb: "Inverse hailstone branches from a root.",
                accent: [180, 80, 40],
            }
        ),
        (
            nova,
            Nova,
            RoomMeta {
                id: "nova",
                title: "Nova Fractal",
                wing: "Fractals",
                blurb: "Newton-style rational map as an escape portrait.",
                accent: [200, 80, 160],
            }
        ),
        (
            magnet,
            MagnetFractal,
            RoomMeta {
                id: "magnet-fractal",
                title: "Magnet Fractal",
                wing: "Fractals",
                blurb: "Type-I magnet set: rational map escape portrait.",
                accent: [80, 40, 160],
            }
        ),
        (
            lambda_map,
            LambdaMap,
            RoomMeta {
                id: "lambda-map",
                title: "Lambda Map",
                wing: "Fractals",
                blurb: "Complex logistic z -> lambda z(1-z) as Julia portrait.",
                accent: [40, 140, 200],
            }
        ),
        (
            feigenbaum,
            Feigenbaum,
            RoomMeta {
                id: "feigenbaum",
                title: "Feigenbaum Ladder",
                wing: "Motion & Dynamics",
                blurb: "Period-doubling cascade of the logistic map, marked.",
                accent: [220, 100, 40],
            }
        ),
        (
            menger,
            Menger,
            RoomMeta {
                id: "menger-carpet",
                title: "Menger Carpet",
                wing: "Fractals",
                blurb: "Sierpinski carpet of removed center squares.",
                accent: [100, 100, 140],
            }
        ),
        (
            vicsek,
            Vicsek,
            RoomMeta {
                id: "vicsek",
                title: "Vicsek Fractal",
                wing: "Fractals",
                blurb: "Plus-shaped IFS: crosses at every scale.",
                accent: [160, 160, 40],
            }
        ),
        (
            chua,
            Chua,
            RoomMeta {
                id: "chua",
                title: "Chua Circuit",
                wing: "Motion & Dynamics",
                blurb: "Double-scroll chaos from a nonlinear diode circuit.",
                accent: [200, 60, 80],
            }
        ),
        (
            cat_map,
            CatMap,
            RoomMeta {
                id: "cat-map",
                title: "Arnold Cat Map",
                wing: "Motion & Dynamics",
                blurb: "Toral shear that shreds then rebuilds a face.",
                accent: [180, 100, 40],
            }
        ),
        (
            blancmange,
            Blancmange,
            RoomMeta {
                id: "blancmange",
                title: "Blancmange Curve",
                wing: "Fractals",
                blurb: "Takagi's continuous graph with no tangent anywhere.",
                accent: [220, 180, 200],
            }
        ),
        (
            rose,
            Rose,
            RoomMeta {
                id: "rose",
                title: "Rose Curve",
                wing: "Shape & Space",
                blurb: "Rhodonea petals draw themselves. Watch the pen.",
                accent: [220, 40, 100],
            }
        ),
        (
            kuramoto,
            Kuramoto,
            RoomMeta {
                id: "kuramoto",
                title: "Kuramoto Sync",
                wing: "Motion & Dynamics",
                blurb: "Coupled phase clocks find a shared beat.",
                accent: [40, 160, 200],
            }
        ),
        (
            h_tree,
            HTree,
            RoomMeta {
                id: "h-tree",
                title: "H-Tree",
                wing: "Fractals",
                blurb: "Self-similar H strokes that tile the plane.",
                accent: [80, 140, 80],
            }
        ),
        (
            percolation,
            Percolation,
            RoomMeta {
                id: "percolation",
                title: "Percolation",
                wing: "Chance & Order",
                blurb: "Open sites on a grid until a path crosses.",
                accent: [40, 120, 180],
            }
        ),
        (
            ising,
            Ising,
            RoomMeta {
                id: "ising",
                title: "Ising Lattice",
                wing: "Chance & Order",
                blurb: "Spins freeze or melt across a critical heat.",
                accent: [180, 40, 40],
            }
        ),
        (
            lotka_volterra,
            LotkaVolterra,
            RoomMeta {
                id: "lotka-volterra",
                title: "Lotka-Volterra",
                wing: "Motion & Dynamics",
                blurb: "Predator and prey chase each other in closed orbits.",
                accent: [80, 160, 40],
            }
        ),
        (
            poincare_disc,
            PoincareDisc,
            RoomMeta {
                id: "poincare-disc",
                title: "Poincare Disc",
                wing: "Shape & Space",
                blurb: "Hyperbolic plane inside a circle.",
                accent: [100, 60, 180],
            }
        ),
        (
            cycloid,
            Cycloid,
            RoomMeta {
                id: "cycloid",
                title: "Cycloid",
                wing: "Shape & Space",
                blurb: "A rim point lays cups as a wheel rolls. Watch it roll.",
                accent: [200, 140, 40],
            }
        ),
        (
            brusselator,
            Brusselator,
            RoomMeta {
                id: "brusselator",
                title: "Brusselator",
                wing: "Motion & Dynamics",
                blurb: "Chemical oscillator waves in space-time.",
                accent: [160, 80, 200],
            }
        ),
        (
            sprott,
            Sprott,
            RoomMeta {
                id: "sprott",
                title: "Sprott Attractor",
                wing: "Motion & Dynamics",
                blurb: "Minimal quadratic chaos in three dimensions.",
                accent: [120, 80, 160],
            }
        ),
        (
            delaunay,
            Delaunay,
            RoomMeta {
                id: "delaunay",
                title: "Delaunay Mesh",
                wing: "Shape & Space",
                blurb: "Empty-circle triangulation of scatter points.",
                accent: [40, 140, 100],
            }
        ),
        (
            astroid,
            Astroid,
            RoomMeta {
                id: "astroid",
                title: "Astroid",
                wing: "Shape & Space",
                blurb: "Four-cusped star from a rolling circle.",
                accent: [200, 160, 40],
            }
        ),
        (
            sir,
            Sir,
            RoomMeta {
                id: "sir",
                title: "SIR Epidemic",
                wing: "Chance & Order",
                blurb: "Susceptible, infected, recovered curves.",
                accent: [180, 40, 60],
            }
        ),
        (
            nephroid,
            Nephroid,
            RoomMeta {
                id: "nephroid",
                title: "Nephroid",
                wing: "Shape & Space",
                blurb: "Two-cusped kidney curve from a rolling circle.",
                accent: [180, 100, 60],
            }
        ),
        (
            lemniscate,
            Lemniscate,
            RoomMeta {
                id: "lemniscate",
                title: "Lemniscate",
                wing: "Shape & Space",
                blurb: "Bernoulli infinity draws both lobes. Watch the pen.",
                accent: [160, 40, 120],
            }
        ),
        (
            cardioid,
            Cardioid,
            RoomMeta {
                id: "cardioid",
                title: "Cardioid",
                wing: "Shape & Space",
                blurb: "One-cusped heart from a rolling circle. Watch it draw.",
                accent: [220, 60, 80],
            }
        ),
        (
            deltoid,
            Deltoid,
            RoomMeta {
                id: "deltoid",
                title: "Deltoid",
                wing: "Shape & Space",
                blurb: "Three-cusped hypocycloid draws itself. Watch the pen.",
                accent: [80, 160, 200],
            }
        ),
        (
            coupled_logistic,
            CoupledLogistic,
            RoomMeta {
                id: "coupled-logistic",
                title: "Coupled Logistic",
                wing: "Motion & Dynamics",
                blurb: "Two logistic maps cross-talk into sync or chaos.",
                accent: [200, 120, 40],
            }
        ),
        (
            menger_sponge,
            MengerSponge,
            RoomMeta {
                id: "menger-sponge",
                title: "Menger Sponge",
                wing: "Fractals",
                blurb: "3D cross-removal fractal in twin slices.",
                accent: [100, 100, 120],
            }
        ),
        (
            pythagoras_spiral,
            PythagorasSpiral,
            RoomMeta {
                id: "theodorus",
                title: "Spiral of Theodorus",
                wing: "Number & Pattern",
                blurb: "Stacked right triangles build a root spiral.",
                accent: [160, 120, 40],
            }
        ),
        (
            wolfram_110,
            Wolfram110,
            RoomMeta {
                id: "rule-110",
                title: "Rule 110",
                wing: "Emergence",
                blurb: "Wolfram's Turing-complete elementary CA.",
                accent: [40, 200, 80],
            }
        ),
        (
            hyperbolic_tiling,
            HyperbolicTiling,
            RoomMeta {
                id: "hyperbolic-tiling",
                title: "Hyperbolic Tiling",
                wing: "Shape & Space",
                blurb: "{7,3}-style lattice in the Poincare disc.",
                accent: [120, 40, 160],
            }
        ),
        (
            mackey_glass,
            MackeyGlass,
            RoomMeta {
                id: "mackey-glass",
                title: "Mackey-Glass",
                wing: "Motion & Dynamics",
                blurb: "Delayed feedback births a strange attractor.",
                accent: [40, 140, 120],
            }
        ),
        (
            fermat_spiral,
            FermatSpiral,
            RoomMeta {
                id: "fermat-spiral",
                title: "Fermat Spiral",
                wing: "Shape & Space",
                blurb: "Equal-area arms unfurl together. Watch the tips.",
                accent: [200, 160, 40],
            }
        ),
        (
            euclid_algorithm,
            EuclidAlgorithm,
            RoomMeta {
                id: "euclid",
                title: "Euclid Algorithm",
                wing: "Number & Pattern",
                blurb: "Square-cutting dance that finds gcd.",
                accent: [80, 80, 180],
            }
        ),
        (
            oregonator,
            Oregonator,
            RoomMeta {
                id: "oregonator",
                title: "Oregonator",
                wing: "Motion & Dynamics",
                blurb: "BZ chemical clock reduced to three variables.",
                accent: [200, 40, 140],
            }
        ),
        (
            hofstadter_q,
            HofstadterQ,
            RoomMeta {
                id: "hofstadter-q",
                title: "Hofstadter Q",
                wing: "Number & Pattern",
                blurb: "Chaotic integer recursion as a skyline.",
                accent: [100, 40, 160],
            }
        ),
        (
            dual_cobweb,
            DualCobweb,
            RoomMeta {
                id: "dual-cobweb",
                title: "Dual Cobweb",
                wing: "Motion & Dynamics",
                blurb: "Two logistic cobwebs at neighboring r.",
                accent: [180, 100, 40],
            }
        ),
        (
            beverton_holt,
            BevertonHolt,
            RoomMeta {
                id: "beverton-holt",
                title: "Beverton-Holt",
                wing: "Motion & Dynamics",
                blurb: "Saturating recruitment map for a fishery.",
                accent: [40, 140, 100],
            }
        ),
        (
            witch_of_agnesi,
            WitchOfAgnesi,
            RoomMeta {
                id: "witch-of-agnesi",
                title: "Witch of Agnesi",
                wing: "Shape & Space",
                blurb: "Maria Agnesi's classical cubic bell curve.",
                accent: [160, 80, 160],
            }
        ),
        (
            tractrix,
            Tractrix,
            RoomMeta {
                id: "tractrix",
                title: "Tractrix",
                wing: "Shape & Space",
                blurb: "The path of a pulled dog: constant tangent length.",
                accent: [100, 120, 40],
            }
        ),
        (
            catenary,
            Catenary,
            RoomMeta {
                id: "catenary",
                title: "Catenary",
                wing: "Shape & Space",
                blurb: "The hanging chain: a cosh curve under gravity.",
                accent: [140, 100, 40],
            }
        ),
        (
            clothoid,
            Clothoid,
            RoomMeta {
                id: "clothoid",
                title: "Clothoid",
                wing: "Shape & Space",
                blurb: "Euler spiral: curvature grows with arc length.",
                accent: [60, 100, 180],
            }
        ),
        (
            lemniscate_gerono,
            LemniscateGerono,
            RoomMeta {
                id: "gerono",
                title: "Gerono Eight",
                wing: "Shape & Space",
                blurb: "Figure-eight from x = a cos t, y = a sin t cos t.",
                accent: [180, 60, 140],
            }
        ),
        (
            cissoid,
            Cissoid,
            RoomMeta {
                id: "cissoid",
                title: "Cissoid",
                wing: "Shape & Space",
                blurb: "Diocles' ivy curve for doubling the cube.",
                accent: [80, 140, 60],
            }
        ),
        (
            strophoid,
            Strophoid,
            RoomMeta {
                id: "strophoid",
                title: "Strophoid",
                wing: "Shape & Space",
                blurb: "Twisted belt draws its loop and asymptote. Watch the pen.",
                accent: [160, 100, 40],
            }
        ),
        (
            conchoid,
            Conchoid,
            RoomMeta {
                id: "conchoid",
                title: "Conchoid",
                wing: "Shape & Space",
                blurb: "Nicomedes shell draws both branches. Watch the pen.",
                accent: [40, 120, 160],
            }
        ),
        (
            limacon,
            Limacon,
            RoomMeta {
                id: "limacon",
                title: "Limacon",
                wing: "Shape & Space",
                blurb: "Pascal's snail draws itself: dimple, heart, or loop.",
                accent: [200, 80, 60],
            }
        ),
        (
            folium,
            Folium,
            RoomMeta {
                id: "folium",
                title: "Folium",
                wing: "Shape & Space",
                blurb: "Descartes leaf draws its loop and asymptote. Watch the pen.",
                accent: [60, 140, 80],
            }
        ),
        (
            semicubical,
            Semicubical,
            RoomMeta {
                id: "semicubical",
                title: "Semicubical",
                wing: "Shape & Space",
                blurb: "Cuspidal cubic y squared equals x cubed.",
                accent: [180, 120, 40],
            }
        ),
        (
            kappa,
            Kappa,
            RoomMeta {
                id: "kappa",
                title: "Kappa Curve",
                wing: "Shape & Space",
                blurb: "Classical kappa: r = a cot theta.",
                accent: [120, 80, 160],
            }
        ),
        (
            witch_caustic,
            CircularCaustic,
            RoomMeta {
                id: "circular-caustic",
                title: "Circular Caustic",
                wing: "Shape & Space",
                blurb: "Reflected parallel light envelopes a nephroid.",
                accent: [220, 180, 40],
            }
        ),
        (
            trochoid,
            Trochoid,
            RoomMeta {
                id: "trochoid",
                title: "Trochoid",
                wing: "Shape & Space",
                blurb: "Rolling-circle cups draw themselves. Watch the pen.",
                accent: [160, 100, 40],
            }
        ),
        (
            hypotrochoid,
            Hypotrochoid,
            RoomMeta {
                id: "hypotrochoid",
                title: "Hypotrochoid",
                wing: "Shape & Space",
                blurb: "Spirograph draws itself. Watch the pen.",
                accent: [200, 60, 100],
            }
        ),
        (
            epitrochoid,
            Epitrochoid,
            RoomMeta {
                id: "epitrochoid",
                title: "Epitrochoid",
                wing: "Shape & Space",
                blurb: "Outer rolling roulette draws itself. Watch the pen.",
                accent: [80, 60, 180],
            }
        ),
        (
            involute,
            Involute,
            RoomMeta {
                id: "involute",
                title: "Involute",
                wing: "Shape & Space",
                blurb: "Unwrapping a taut string from a circle.",
                accent: [100, 140, 80],
            }
        ),
        (
            evolute,
            Evolute,
            RoomMeta {
                id: "evolute",
                title: "Ellipse Evolute",
                wing: "Shape & Space",
                blurb: "Envelope of normals to an ellipse.",
                accent: [80, 100, 180],
            }
        ),
        (
            pedal,
            Pedal,
            RoomMeta {
                id: "pedal",
                title: "Pedal Curve",
                wing: "Shape & Space",
                blurb: "Feet of perpendiculars from a focus to circle tangents.",
                accent: [160, 80, 120],
            }
        ),
        (
            roulette,
            Roulette,
            RoomMeta {
                id: "roulette",
                title: "Roulette Gallery",
                wing: "Shape & Space",
                blurb: "Epi and hypo rolling paths overlaid.",
                accent: [180, 40, 100],
            }
        ),
        (
            damped_sine,
            DampedSine,
            RoomMeta {
                id: "damped-sine",
                title: "Damped Sine",
                wing: "Waves & Sound",
                blurb: "Exponential envelope on a pure oscillation.",
                accent: [40, 160, 180],
            }
        ),
        (
            beats,
            Beats,
            RoomMeta {
                id: "beats",
                title: "Beats",
                wing: "Waves & Sound",
                blurb: "Two close tones pulse as one slow envelope.",
                accent: [200, 100, 40],
            }
        ),
        (
            gibbs_square,
            GibbsSquare,
            RoomMeta {
                id: "gibbs-square",
                title: "Gibbs Square",
                wing: "Waves & Sound",
                blurb: "Fourier square partials overshoot at jumps.",
                accent: [200, 80, 40],
            }
        ),
        (
            sawtooth,
            Sawtooth,
            RoomMeta {
                id: "sawtooth",
                title: "Sawtooth",
                wing: "Waves & Sound",
                blurb: "Fourier partials of a ramp: all harmonics.",
                accent: [180, 60, 80],
            }
        ),
        (
            triangle_wave,
            TriangleWave,
            RoomMeta {
                id: "triangle-wave",
                title: "Triangle Wave",
                wing: "Waves & Sound",
                blurb: "Odd harmonics with 1/k squared: soft corners.",
                accent: [60, 140, 160],
            }
        ),
        (
            am_modulation,
            AmModulation,
            RoomMeta {
                id: "am-modulation",
                title: "AM Modulation",
                wing: "Waves & Sound",
                blurb: "Carrier times slow envelope: radio AM.",
                accent: [80, 160, 40],
            }
        ),
        (
            fm_modulation,
            FmModulation,
            RoomMeta {
                id: "fm-modulation",
                title: "FM Modulation",
                wing: "Waves & Sound",
                blurb: "Instantaneous frequency wiggles: radio FM.",
                accent: [140, 60, 180],
            }
        ),
        (
            standing_wave,
            StandingWave,
            RoomMeta {
                id: "standing-wave",
                title: "Standing Wave",
                wing: "Waves & Sound",
                blurb: "Fixed-end string modes that breathe. Watch the antinodes.",
                accent: [40, 100, 200],
            }
        ),
        (
            doppler,
            Doppler,
            RoomMeta {
                id: "doppler",
                title: "Doppler",
                wing: "Waves & Sound",
                blurb: "Moving source packs wavefronts ahead.",
                accent: [200, 100, 60],
            }
        ),
        (
            interference,
            Interference,
            RoomMeta {
                id: "interference",
                title: "Interference",
                wing: "Waves & Sound",
                blurb: "Two sources paint bright and dark fringes.",
                accent: [60, 80, 200],
            }
        ),
        (
            diffraction,
            Diffraction,
            RoomMeta {
                id: "diffraction",
                title: "Diffraction",
                wing: "Waves & Sound",
                blurb: "Single-slit sinc squared intensity pattern.",
                accent: [100, 60, 180],
            }
        ),
        (
            snell,
            Snell,
            RoomMeta {
                id: "snell",
                title: "Snell's Law",
                wing: "Waves & Sound",
                blurb: "Rays bend at an interface; total reflection past critical.",
                accent: [40, 140, 200],
            }
        ),
        (
            polarization,
            Polarization,
            RoomMeta {
                id: "polarization",
                title: "Polarization",
                wing: "Waves & Sound",
                blurb: "Malus: intensity falls as cos squared of angle.",
                accent: [180, 40, 140],
            }
        ),
        (
            brewster,
            Brewster,
            RoomMeta {
                id: "brewster",
                title: "Brewster Angle",
                wing: "Waves & Sound",
                blurb: "Fresnel reflectance dips at tan i = n2/n1.",
                accent: [160, 120, 40],
            }
        ),
        (
            reuleaux,
            Reuleaux,
            RoomMeta {
                id: "reuleaux",
                title: "Reuleaux Triangle",
                wing: "Shape & Space",
                blurb: "Constant-width curve of three arcs.",
                accent: [180, 90, 40],
            }
        ),
        (
            log_spiral,
            LogSpiral,
            RoomMeta {
                id: "log-spiral",
                title: "Logarithmic Spiral",
                wing: "Shape & Space",
                blurb: "Equiangular growth r = a e^{b theta}.",
                accent: [40, 140, 160],
            }
        ),
        (
            archimedean,
            Archimedean,
            RoomMeta {
                id: "archimedean",
                title: "Archimedean Spiral",
                wing: "Shape & Space",
                blurb: "Arithmetic arm unfurls at constant gap. Watch the tip.",
                accent: [90, 120, 50],
            }
        ),
        (
            cassini,
            Cassini,
            RoomMeta {
                id: "cassini",
                title: "Cassini Ovals",
                wing: "Shape & Space",
                blurb: "Two-foci product curves draw themselves. Watch the pen.",
                accent: [140, 60, 120],
            }
        ),
        (
            foucault,
            Foucault,
            RoomMeta {
                id: "foucault",
                title: "Foucault Pendulum",
                wing: "Motion & Dynamics",
                blurb: "Swing plane precesses with sin(latitude).",
                accent: [50, 90, 160],
            }
        ),
        (
            coriolis,
            Coriolis,
            RoomMeta {
                id: "coriolis",
                title: "Coriolis Path",
                wing: "Motion & Dynamics",
                blurb: "Inertial straight line curves under frame spin.",
                accent: [30, 130, 100],
            }
        ),
        (
            tautochrone,
            Tautochrone,
            RoomMeta {
                id: "tautochrone",
                title: "Tautochrone",
                wing: "Motion & Dynamics",
                blurb: "Beads on a cycloid finish together.",
                accent: [70, 110, 180],
            }
        ),
        (
            catenoid,
            Catenoid,
            RoomMeta {
                id: "catenoid",
                title: "Catenoid",
                wing: "Shape & Space",
                blurb: "Minimal surface: revolve a catenary.",
                accent: [160, 100, 60],
            }
        ),
        (
            helicoid,
            Helicoid,
            RoomMeta {
                id: "helicoid",
                title: "Helicoid",
                wing: "Shape & Space",
                blurb: "Ruled minimal screw surface.",
                accent: [80, 140, 90],
            }
        ),
        (
            pseudosphere,
            Pseudosphere,
            RoomMeta {
                id: "pseudosphere",
                title: "Pseudosphere",
                wing: "Shape & Space",
                blurb: "Constant K=-1 from a spun tractrix.",
                accent: [120, 50, 140],
            }
        ),
        (
            airy,
            Airy,
            RoomMeta {
                id: "airy",
                title: "Airy Disk",
                wing: "Waves & Sound",
                blurb: "Circular aperture diffraction rings.",
                accent: [200, 180, 40],
            }
        ),
        (
            bragg,
            Bragg,
            RoomMeta {
                id: "bragg",
                title: "Bragg Diffraction",
                wing: "Waves & Sound",
                blurb: "n lambda = 2 d sin theta on crystal planes.",
                accent: [40, 100, 160],
            }
        ),
        (
            trisectrix,
            Trisectrix,
            RoomMeta {
                id: "trisectrix",
                title: "Maclaurin Trisectrix",
                wing: "Shape & Space",
                blurb: "Classical curve that trisects angles.",
                accent: [150, 70, 100],
            }
        ),
        (
            watt_curve,
            WattCurve,
            RoomMeta {
                id: "watt-curve",
                title: "Watt Curve",
                wing: "Shape & Space",
                blurb: "Midpoint of a two-bar linkage.",
                accent: [90, 90, 40],
            }
        ),
        (
            devil_curve,
            DevilCurve,
            RoomMeta {
                id: "devil-curve",
                title: "Devil Curve",
                wing: "Shape & Space",
                blurb: "Quartic figure-eight of Gabriele.",
                accent: [120, 30, 30],
            }
        ),
        (
            capillary,
            Capillary,
            RoomMeta {
                id: "capillary",
                title: "Capillary Meniscus",
                wing: "Motion & Dynamics",
                blurb: "Young-Laplace rise vs contact angle.",
                accent: [40, 120, 180],
            }
        ),
        (
            rabi,
            Rabi,
            RoomMeta {
                id: "rabi",
                title: "Rabi Flopping",
                wing: "Waves & Sound",
                blurb: "Two-level drive: detune slows full flips.",
                accent: [80, 40, 160],
            }
        ),
        (
            geodesic,
            Geodesic,
            RoomMeta {
                id: "geodesic",
                title: "Sphere Geodesics",
                wing: "Shape & Space",
                blurb: "Great-circle arcs follow sphere geodesics.",
                accent: [30, 100, 140],
            }
        ),
        (
            kampyle,
            Kampyle,
            RoomMeta {
                id: "kampyle",
                title: "Kampyle of Eudoxus",
                wing: "Shape & Space",
                blurb: "Horn curve x^4 = a^2 (x^2+y^2).",
                accent: [140, 100, 40],
            }
        ),
        (
            hippopede,
            Hippopede,
            RoomMeta {
                id: "hippopede",
                title: "Hippopede",
                wing: "Shape & Space",
                blurb: "Proclus horse-fetter draws itself. Watch the pen.",
                accent: [100, 70, 50],
            }
        ),
        (
            cartesian_oval,
            CartesianOval,
            RoomMeta {
                id: "cartesian-oval",
                title: "Cartesian Oval",
                wing: "Shape & Space",
                blurb: "Weighted sum of distances to two foci.",
                accent: [50, 110, 90],
            }
        ),
        (
            berry,
            Berry,
            RoomMeta {
                id: "berry",
                title: "Berry Phase",
                wing: "Waves & Sound",
                blurb: "Holonomy after a closed parameter loop.",
                accent: [160, 80, 180],
            }
        ),
        (
            runge,
            Runge,
            RoomMeta {
                id: "runge",
                title: "Runge Phenomenon",
                wing: "Number & Pattern",
                blurb: "Equispaced high-degree fit oscillates.",
                accent: [180, 50, 50],
            }
        ),
        (
            chebyshev,
            Chebyshev,
            RoomMeta {
                id: "chebyshev",
                title: "Chebyshev Nodes",
                wing: "Number & Pattern",
                blurb: "Min-max nodes tame Runge edges.",
                accent: [40, 140, 80],
            }
        ),
        (
            bessel,
            Bessel,
            RoomMeta {
                id: "bessel",
                title: "Bessel J0",
                wing: "Waves & Sound",
                blurb: "Cylindrical wave zeros as rings.",
                accent: [50, 90, 150],
            }
        ),
        (
            hermite,
            Hermite,
            RoomMeta {
                id: "hermite",
                title: "Hermite Wave",
                wing: "Waves & Sound",
                blurb: "Harmonic oscillator Hermite modes.",
                accent: [90, 40, 140],
            }
        ),
        (
            legendre,
            Legendre,
            RoomMeta {
                id: "legendre",
                title: "Legendre P_n",
                wing: "Number & Pattern",
                blurb: "Orthogonal polynomials on [-1,1].",
                accent: [40, 120, 60],
            }
        ),
        (
            heat_kernel,
            HeatKernel,
            RoomMeta {
                id: "heat-kernel",
                title: "Heat Kernel",
                wing: "Change",
                blurb: "Gaussian spreads as sqrt(t).",
                accent: [200, 80, 30],
            }
        ),
        (
            cauchy_lorentz,
            CauchyLorentz,
            RoomMeta {
                id: "cauchy-lorentz",
                title: "Cauchy Lorentz",
                wing: "Chance & Order",
                blurb: "Heavy-tailed density with no mean.",
                accent: [120, 40, 100],
            }
        ),
        (
            mexican_hat,
            MexicanHat,
            RoomMeta {
                id: "mexican-hat",
                title: "Mexican Hat",
                wing: "Waves & Sound",
                blurb: "Ricker wavelet: second Gaussian derivative.",
                accent: [160, 100, 40],
            }
        ),
        (
            seifert,
            Seifert,
            RoomMeta {
                id: "seifert",
                title: "Seifert Film",
                wing: "Shape & Space",
                blurb: "A surface spanning a link.",
                accent: [80, 60, 140],
            }
        ),
        (
            trefoil,
            Trefoil,
            RoomMeta {
                id: "trefoil",
                title: "Trefoil Knot",
                wing: "Shape & Space",
                blurb: "Simplest nontrivial knot.",
                accent: [140, 50, 80],
            }
        ),
        (
            hopf_fibration,
            HopfFibration,
            RoomMeta {
                id: "hopf-fibration",
                title: "Hopf Fibration",
                wing: "Shape & Space",
                blurb: "S3 fibers as linked circles.",
                accent: [40, 80, 160],
            }
        ),
        (
            julia_set,
            JuliaFilled,
            RoomMeta {
                id: "julia-filled",
                title: "Filled Julia",
                wing: "Fractals",
                blurb: "Filled set for z^2+c.",
                accent: [20, 100, 140],
            }
        ),
        (
            figure_eight_knot,
            FigureEightKnot,
            RoomMeta {
                id: "figure-eight-knot",
                title: "Figure-Eight Knot",
                wing: "Shape & Space",
                blurb: "Second simplest prime knot.",
                accent: [100, 70, 40],
            }
        ),
        (
            borromean,
            Borromean,
            RoomMeta {
                id: "borromean",
                title: "Borromean Rings",
                wing: "Shape & Space",
                blurb: "Three linked as one; no pair linked.",
                accent: [160, 120, 30],
            }
        ),
        (
            viviani,
            Viviani,
            RoomMeta {
                id: "viviani",
                title: "Viviani Curve",
                wing: "Shape & Space",
                blurb: "Sphere meets a tangent cylinder.",
                accent: [50, 120, 140],
            }
        ),
        (
            torus_knot,
            TorusKnot,
            RoomMeta {
                id: "torus-knot",
                title: "Torus Knot",
                wing: "Shape & Space",
                blurb: "T(p,q) winds the torus both ways.",
                accent: [120, 50, 100],
            }
        ),
        (
            whitney_umbrella,
            WhitneyUmbrella,
            RoomMeta {
                id: "whitney-umbrella",
                title: "Whitney Umbrella",
                wing: "Shape & Space",
                blurb: "Cross-cap singularity x=uv, y=u, z=v^2.",
                accent: [90, 70, 40],
            }
        ),
        (
            roman_surface,
            RomanSurface,
            RoomMeta {
                id: "roman-surface",
                title: "Roman Surface",
                wing: "Shape & Space",
                blurb: "Steiner immersion of the projective plane.",
                accent: [140, 40, 80],
            }
        ),
        (
            spherical_harmonic,
            SphericalHarmonic,
            RoomMeta {
                id: "spherical-harmonic",
                title: "Spherical Harmonic",
                wing: "Waves & Sound",
                blurb: "Y_lm nodal lines on the sphere.",
                accent: [40, 100, 160],
            }
        ),
        (
            lissajous_3d,
            Lissajous3d,
            RoomMeta {
                id: "lissajous-3d",
                title: "Lissajous 3D",
                wing: "Waves & Sound",
                blurb: "Three orthogonal sines draw a space knot.",
                accent: [30, 140, 120],
            }
        ),
        (
            kolakoski,
            Kolakoski,
            RoomMeta {
                id: "kolakoski",
                title: "Kolakoski Sequence",
                wing: "Number & Pattern",
                blurb: "Self-describing runs of 1 and 2.",
                accent: [80, 100, 40],
            }
        ),
        (
            beatty,
            Beatty,
            RoomMeta {
                id: "beatty",
                title: "Beatty Sequence",
                wing: "Number & Pattern",
                blurb: "floor(n r) and floor(n s) partition N.",
                accent: [100, 60, 120],
            }
        ),
        (
            wythoff,
            Wythoff,
            RoomMeta {
                id: "wythoff",
                title: "Wythoff Array",
                wing: "Number & Pattern",
                blurb: "Golden Beatty pairs A_k, B_k.",
                accent: [160, 120, 40],
            }
        ),
        (
            minkowski_qm,
            MinkowskiQm,
            RoomMeta {
                id: "minkowski-qm",
                title: "Minkowski Question Mark",
                wing: "Number & Pattern",
                blurb: "?(x) maps CF to dyadics, flattens jumps.",
                accent: [60, 40, 140],
            }
        ),
        (
            ruler_function,
            RulerFunction,
            RoomMeta {
                id: "ruler-function",
                title: "Ruler Function",
                wing: "Number & Pattern",
                blurb: "2-adic height of n: paper ruler marks.",
                accent: [40, 90, 70],
            }
        ),
        (
            moser_debruijn,
            MoserDebruijn,
            RoomMeta {
                id: "moser-debruijn",
                title: "Moser-de Bruijn",
                wing: "Number & Pattern",
                blurb: "Sums of distinct powers of 4.",
                accent: [90, 50, 90],
            }
        ),
        (
            mertens,
            Mertens,
            RoomMeta {
                id: "mertens",
                title: "Mertens Function",
                wing: "Number & Pattern",
                blurb: "M(n) = sum mu(k): Mobius partial sums.",
                accent: [70, 50, 130],
            }
        ),
        (
            liouville,
            Liouville,
            RoomMeta {
                id: "liouville",
                title: "Liouville Function",
                wing: "Number & Pattern",
                blurb: "lambda(n) by total prime factors; L(n) sum.",
                accent: [100, 40, 100],
            }
        ),
        (
            euler_totient,
            EulerTotient,
            RoomMeta {
                id: "euler-totient",
                title: "Euler Totient",
                wing: "Number & Pattern",
                blurb: "phi(n): count of units mod n.",
                accent: [40, 110, 80],
            }
        ),
        (
            partition,
            Partition,
            RoomMeta {
                id: "partition",
                title: "Partition Function",
                wing: "Number & Pattern",
                blurb: "p(n): ways to write n as unordered sums.",
                accent: [140, 90, 40],
            }
        ),
        (
            paperfold,
            Paperfold,
            RoomMeta {
                id: "paperfold",
                title: "Paperfold Sequence",
                wing: "Number & Pattern",
                blurb: "Regular fold bits draw a dragon path.",
                accent: [50, 90, 140],
            }
        ),
        (
            sylvester,
            Sylvester,
            RoomMeta {
                id: "sylvester",
                title: "Sylvester Sequence",
                wing: "Number & Pattern",
                blurb: "Double-exponential Egyptian fraction of 1.",
                accent: [120, 60, 40],
            }
        ),
        (
            poisson,
            Poisson,
            RoomMeta {
                id: "poisson",
                title: "Poisson Process",
                wing: "Chance & Order",
                blurb: "Exponential waits build a living staircase. Watch it run.",
                accent: [40, 120, 100],
            }
        ),
        (
            brownian,
            Brownian,
            RoomMeta {
                id: "brownian",
                title: "Brownian Motion",
                wing: "Chance & Order",
                blurb: "Wiener path from Gaussian steps.",
                accent: [80, 80, 40],
            }
        ),
        (
            birthday,
            Birthday,
            RoomMeta {
                id: "birthday",
                title: "Birthday Paradox",
                wing: "Chance & Order",
                blurb: "Shared birthday odds grow faster than intuition.",
                accent: [160, 60, 80],
            }
        ),
        (
            coupon,
            Coupon,
            RoomMeta {
                id: "coupon",
                title: "Coupon Collector",
                wing: "Chance & Order",
                blurb: "Expected waits n H_n to finish a set.",
                accent: [100, 80, 30],
            }
        ),
        (
            zipf,
            Zipf,
            RoomMeta {
                id: "zipf",
                title: "Zipf Law",
                wing: "Chance & Order",
                blurb: "Rank-frequency power law 1/k^s.",
                accent: [90, 50, 110],
            }
        ),
        (
            gamblers_ruin,
            GamblersRuin,
            RoomMeta {
                id: "gamblers-ruin",
                title: "Gamblers Ruin",
                wing: "Chance & Order",
                blurb: "Random walk absorbed at 0 or N.",
                accent: [130, 40, 40],
            }
        ),
        (
            harmonic_series,
            HarmonicSeries,
            RoomMeta {
                id: "harmonic-series",
                title: "Harmonic Series",
                wing: "Number & Pattern",
                blurb: "H_n grows like ln n + gamma.",
                accent: [50, 100, 140],
            }
        ),
        (
            basel,
            Basel,
            RoomMeta {
                id: "basel",
                title: "Basel Problem",
                wing: "Number & Pattern",
                blurb: "sum 1/n^2 climbs to pi^2/6.",
                accent: [120, 40, 100],
            }
        ),
        (
            stirling,
            Stirling,
            RoomMeta {
                id: "stirling",
                title: "Stirling Approx",
                wing: "Number & Pattern",
                blurb: "n! vs sqrt(2 pi n)(n/e)^n on a log scale.",
                accent: [80, 60, 140],
            }
        ),
        (
            benford,
            Benford,
            RoomMeta {
                id: "benford",
                title: "Benford Law",
                wing: "Chance & Order",
                blurb: "Leading digits: log law P(d)=log(1+1/d).",
                accent: [40, 90, 120],
            }
        ),
        (
            central_limit,
            CentralLimit,
            RoomMeta {
                id: "central-limit",
                title: "Central Limit",
                wing: "Chance & Order",
                blurb: "Means of uniforms become a bell as n grows.",
                accent: [60, 100, 60],
            }
        ),
        (
            wallis,
            Wallis,
            RoomMeta {
                id: "wallis",
                title: "Wallis Product",
                wing: "Number & Pattern",
                blurb: "Product (4k^2)/(4k^2-1) -> pi/2.",
                accent: [100, 50, 80],
            }
        ),
        (
            superellipse,
            Superellipse,
            RoomMeta {
                id: "superellipse",
                title: "Superellipse",
                wing: "Shape & Space",
                blurb: "Lame curve |x|^n+|y|^n=1 from diamond to square.",
                accent: [70, 110, 90],
            }
        ),
        (
            cochleoid,
            Cochleoid,
            RoomMeta {
                id: "cochleoid",
                title: "Cochleoid",
                wing: "Shape & Space",
                blurb: "Snail curve unfurls: r = a sin(th)/th.",
                accent: [140, 80, 50],
            }
        ),
        (
            serpentine,
            Serpentine,
            RoomMeta {
                id: "serpentine",
                title: "Serpentine Curve",
                wing: "Shape & Space",
                blurb: "Newton's snake y = a b x/(x^2+a^2).",
                accent: [50, 120, 70],
            }
        ),
        (
            bifolium,
            Bifolium,
            RoomMeta {
                id: "bifolium",
                title: "Bifolium",
                wing: "Shape & Space",
                blurb: "Two-leaf curve r = a sin th cos^2 th.",
                accent: [90, 140, 50],
            }
        ),
        (
            butterfly_curve,
            ButterflyCurve,
            RoomMeta {
                id: "butterfly-curve",
                title: "Butterfly Curve",
                wing: "Shape & Space",
                blurb: "Temple-Fay wings draw themselves. Watch the pen.",
                accent: [160, 50, 100],
            }
        ),
        (
            piriform,
            Piriform,
            RoomMeta {
                id: "piriform",
                title: "Piriform Curve",
                wing: "Shape & Space",
                blurb: "Pear curve draws from stem to body. Watch the pen.",
                accent: [120, 100, 40],
            }
        ),
        (
            simple_pendulum,
            SimplePendulum,
            RoomMeta {
                id: "simple-pendulum",
                title: "Simple Pendulum",
                wing: "Motion & Dynamics",
                blurb: "Phase portrait: librations and rotations.",
                accent: [40, 80, 140],
            }
        ),
        (
            blackbody,
            Blackbody,
            RoomMeta {
                id: "blackbody",
                title: "Blackbody Spectrum",
                wing: "Waves & Sound",
                blurb: "Planck curve and Wien peak shift with T.",
                accent: [180, 80, 30],
            }
        ),
        (
            kepler_laws,
            KeplerLaws,
            RoomMeta {
                id: "kepler-laws",
                title: "Kepler Areas",
                wing: "Motion & Dynamics",
                blurb: "Equal areas in equal times on an ellipse.",
                accent: [100, 70, 30],
            }
        ),
        (
            escape_velocity,
            EscapeVelocity,
            RoomMeta {
                id: "escape-velocity",
                title: "Escape Velocity",
                wing: "Motion & Dynamics",
                blurb: "v_esc = sqrt(2GM/r); circular is slower by sqrt(2).",
                accent: [50, 50, 120],
            }
        ),
        (
            coupled_osc,
            CoupledOsc,
            RoomMeta {
                id: "coupled-osc",
                title: "Coupled Oscillators",
                wing: "Motion & Dynamics",
                blurb: "Two masses, three springs: normal modes.",
                accent: [80, 100, 50],
            }
        ),
        (
            snell_prism,
            SnellPrism,
            RoomMeta {
                id: "snell-prism",
                title: "Prism Dispersion",
                wing: "Waves & Sound",
                blurb: "n(lambda) splits white light in a prism.",
                accent: [140, 40, 120],
            }
        ),
        (
            lucky_numbers,
            LuckyNumbers,
            RoomMeta {
                id: "lucky-numbers",
                title: "Lucky Numbers",
                wing: "Number & Pattern",
                blurb: "Sieve by counting seats, not multiples.",
                accent: [90, 140, 70],
            }
        ),
        (
            gaussian_primes,
            GaussianPrimes,
            RoomMeta {
                id: "gaussian-primes",
                title: "Gaussian Primes",
                wing: "Number & Pattern",
                blurb: "Primes on the Z[i] lattice.",
                accent: [70, 90, 150],
            }
        ),
        (
            quadratic_residues,
            QuadraticResidues,
            RoomMeta {
                id: "quadratic-residues",
                title: "Quadratic Residues",
                wing: "Number & Pattern",
                blurb: "Legendre symbol checkerboard mod p.",
                accent: [120, 80, 60],
            }
        ),
        (
            zeckendorf,
            Zeckendorf,
            RoomMeta {
                id: "zeckendorf",
                title: "Zeckendorf",
                wing: "Number & Pattern",
                blurb: "Unique Fibonacci base, no adjacent 1s.",
                accent: [100, 70, 130],
            }
        ),
        (
            egyptian_frac,
            EgyptianFrac,
            RoomMeta {
                id: "egyptian-frac",
                title: "Egyptian Fractions",
                wing: "Number & Pattern",
                blurb: "Greedy unit fractions for p/q.",
                accent: [150, 110, 50],
            }
        ),
        (
            pell_path,
            PellPath,
            RoomMeta {
                id: "pell-path",
                title: "Pell Path",
                wing: "Number & Pattern",
                blurb: "Convergents of sqrt(d) chase the Pell hyperbola.",
                accent: [60, 120, 100],
            }
        ),
        (
            shannon_entropy,
            ShannonEntropy,
            RoomMeta {
                id: "shannon-entropy",
                title: "Shannon Entropy",
                wing: "Chance & Noise",
                blurb: "H(p) for a biased coin.",
                accent: [50, 100, 140],
            }
        ),
        (
            bayes_update,
            BayesUpdate,
            RoomMeta {
                id: "bayes-update",
                title: "Bayes Update",
                wing: "Chance & Noise",
                blurb: "Prior times likelihood becomes posterior.",
                accent: [100, 80, 120],
            }
        ),
        (
            erdos_renyi,
            ErdosRenyi,
            RoomMeta {
                id: "erdos-renyi",
                title: "Erdos-Renyi Graph",
                wing: "Chance & Noise",
                blurb: "Random edges with probability p.",
                accent: [80, 110, 70],
            }
        ),
        (
            markov_chain,
            MarkovChain,
            RoomMeta {
                id: "markov-chain",
                title: "Markov Chain",
                wing: "Chance & Noise",
                blurb: "Memoryless walk on states.",
                accent: [90, 60, 100],
            }
        ),
        (
            huffman_tree,
            HuffmanTree,
            RoomMeta {
                id: "huffman-tree",
                title: "Huffman Tree",
                wing: "Chance & Noise",
                blurb: "Optimal prefix codes from frequencies.",
                accent: [70, 120, 90],
            }
        ),
        (
            mutual_info,
            MutualInfo,
            RoomMeta {
                id: "mutual-info",
                title: "Mutual Information",
                wing: "Chance & Noise",
                blurb: "How much X tells you about Y.",
                accent: [110, 90, 50],
            }
        ),
        (
            klein_bottle,
            KleinBottle,
            RoomMeta {
                id: "klein-bottle",
                title: "Klein Bottle",
                wing: "Shape & Space",
                blurb: "A bottle with no inside: non-orientable surface.",
                accent: [80, 50, 120],
            }
        ),
        (
            cross_cap,
            CrossCap,
            RoomMeta {
                id: "cross-cap",
                title: "Cross-Cap",
                wing: "Shape & Space",
                blurb: "RP2 immersion: a cap that crosses itself.",
                accent: [100, 60, 80],
            }
        ),
        (
            boy_surface,
            BoySurface,
            RoomMeta {
                id: "boy-surface",
                title: "Boy Surface",
                wing: "Shape & Space",
                blurb: "RP2 immersed without a free boundary.",
                accent: [60, 90, 110],
            }
        ),
        (
            solid_torus,
            SolidTorus,
            RoomMeta {
                id: "solid-torus",
                title: "Solid Torus",
                wing: "Shape & Space",
                blurb: "Meridian disk spinning inside a doughnut.",
                accent: [70, 100, 90],
            }
        ),
        (
            hopf_link,
            HopfLink,
            RoomMeta {
                id: "hopf-link",
                title: "Hopf Link",
                wing: "Shape & Space",
                blurb: "Two circles, each through the other once.",
                accent: [90, 70, 100],
            }
        ),
        (
            unknot,
            Unknot,
            RoomMeta {
                id: "unknot",
                title: "Unknot",
                wing: "Shape & Space",
                blurb: "A tangled circle that is still the unknot.",
                accent: [50, 80, 100],
            }
        ),
        (
            gamma_func,
            GammaFunc,
            RoomMeta {
                id: "gamma-func",
                title: "Gamma Function",
                wing: "Analysis",
                blurb: "log|Gamma| with poles at nonpositive integers.",
                accent: [100, 70, 50],
            }
        ),
        (
            error_function,
            ErrorFunction,
            RoomMeta {
                id: "error-function",
                title: "Error Function",
                wing: "Analysis",
                blurb: "erf(x): signed Gaussian mass.",
                accent: [60, 100, 80],
            }
        ),
        (
            fresnel_int,
            FresnelInt,
            RoomMeta {
                id: "fresnel-int",
                title: "Fresnel Integrals",
                wing: "Analysis",
                blurb: "C(t), S(t) clothoid spiral to (1/2,1/2).",
                accent: [80, 90, 120],
            }
        ),
        (
            lambert_w,
            LambertW,
            RoomMeta {
                id: "lambert-w",
                title: "Lambert W",
                wing: "Analysis",
                blurb: "Inverse of w e^w, principal branch.",
                accent: [90, 80, 60],
            }
        ),
        (
            sinc_interp,
            SincInterp,
            RoomMeta {
                id: "sinc-interp",
                title: "Sinc Interpolation",
                wing: "Analysis",
                blurb: "Whittaker-Shannon reconstruction from samples.",
                accent: [50, 90, 110],
            }
        ),
        (
            dirichlet_eta,
            DirichletEta,
            RoomMeta {
                id: "dirichlet-eta",
                title: "Dirichlet Eta",
                wing: "Analysis",
                blurb: "Alternating zeta: eta(s)=sum (-1)^{n-1}/n^s.",
                accent: [70, 70, 110],
            }
        ),
        (
            agm_mean,
            AgmMean,
            RoomMeta {
                id: "agm-mean",
                title: "Arithmetic-Geometric Mean",
                wing: "Analysis",
                blurb: "a,g converge by average and geometric mean.",
                accent: [80, 100, 70],
            }
        ),
        (
            twin_primes,
            TwinPrimes,
            RoomMeta {
                id: "twin-primes",
                title: "Twin Primes",
                wing: "Number & Pattern",
                blurb: "Primes that come in pairs (p, p+2).",
                accent: [90, 70, 50],
            }
        ),
        (
            perfect_num,
            PerfectNum,
            RoomMeta {
                id: "perfect-num",
                title: "Perfect Numbers",
                wing: "Number & Pattern",
                blurb: "Even perfects from Mersenne primes.",
                accent: [110, 90, 40],
            }
        ),
        (
            napoleon,
            Napoleon,
            RoomMeta {
                id: "napoleon",
                title: "Napoleon Theorem",
                wing: "Shape & Space",
                blurb: "Equilateral flaps make a new equilateral.",
                accent: [70, 80, 100],
            }
        ),
        (
            smith_chart,
            SmithChart,
            RoomMeta {
                id: "smith-chart",
                title: "The Scariest Chart",
                wing: "Waves & Sound",
                blurb: "Smith chart: the infinite impedance plane folded into a unit \
                        circle of reflection. Phase walks the line.",
                accent: [60, 200, 180],
            }
        ),
        (
            riemann_sphere,
            RiemannSphere,
            RoomMeta {
                id: "riemann-sphere",
                title: "Riemann Sphere",
                wing: "Shape & Space",
                blurb: "One sphere holds every complex number and infinity. \
                        Stereographic projection.",
                accent: [180, 140, 255],
            }
        ),
        (
            bloch_sphere,
            BlochSphere,
            RoomMeta {
                id: "bloch-sphere",
                title: "Bloch Sphere",
                wing: "Shape & Space",
                blurb: "Every pure qubit state is a point on a sphere. |0> and |1> \
                        are poles; the equator is equal superpositions.",
                accent: [100, 200, 255],
            }
        ),
        }
    };
}

macro_rules! hidden_rooms {
    ($callback:ident) => {
        $callback! {
        (
            tetractys,
            Tetractys,
            RoomMeta {
                id: "tetractys",
                title: "Tetractys",
                wing: "The Order",
                blurb: concat!(
                    "One, two, three, four. You were not told about this room, which means you ",
                    "found it, which means it is yours."
                ),
                accent: [240, 220, 120],
            }
        ),
        }
    };
}

macro_rules! metadata_array {
    ($(($module:ident, $room:ident, $metadata:expr)),* $(,)?) => {
        [$($metadata),*]
    };
}

/// Static metadata for every listed room, in catalog order.
pub const ROOM_CATALOG: &[RoomMeta] = &catalog_rooms!(metadata_array);

#[cfg(test)]
macro_rules! source_id_array {
    ($(($module:ident, $room:ident, $metadata:expr)),* $(,)?) => {
        [$((stringify!($module), ($metadata).id)),*]
    };
}

#[cfg(test)]
pub(crate) const ROOM_SOURCE_IDS: &[(&str, &str)] = &catalog_rooms!(source_id_array);

const HIDDEN_ROOM_METADATA: &[RoomMeta] = &hidden_rooms!(metadata_array);

macro_rules! implement_room_metadata {
    ($(($module:ident, $room:ident, $metadata:expr)),* $(,)?) => {
        $(
            impl RoomMetadata for crate::rooms::$module::$room {
                fn meta(&self) -> RoomMeta {
                    $metadata
                }
            }
        )*
    };
}

catalog_rooms!(implement_room_metadata);
hidden_rooms!(implement_room_metadata);

type RoomConstructor = fn(u64) -> Box<dyn Room>;
type HiddenRoomConstructor = fn() -> Box<dyn Room>;

macro_rules! catalog_constructor_array {
    ($(($module:ident, $room:ident, $metadata:expr)),* $(,)?) => {
        [$(
            |variation| -> Box<dyn Room> {
                Box::new(crate::rooms::$module::$room::new_with(variation))
            }
        ),*]
    };
}

macro_rules! hidden_constructor_array {
    ($(($module:ident, $room:ident, $metadata:expr)),* $(,)?) => {
        [$(
            || -> Box<dyn Room> {
                Box::new(crate::rooms::$module::$room::new())
            }
        ),*]
    };
}

const CATALOG_CONSTRUCTORS: &[RoomConstructor] = &catalog_rooms!(catalog_constructor_array);
const HIDDEN_ROOM_CONSTRUCTORS: &[HiddenRoomConstructor] = &hidden_rooms!(hidden_constructor_array);

/// Compatibility spellings that resolve to one canonical listed-room id.
///
/// Aliases never appear in discovery, Journey state, or serialized replay.
/// Construction returns the canonical room, whose metadata remains the only
/// identity every face publishes.
const ROOM_ID_ALIASES: &[(&str, &str)] = &[("kepler-areas", "kepler-laws")];

/// Return the catalog identity for a listed room or compatibility alias.
#[must_use]
pub fn canonical_room_id(id: &str) -> &str {
    ROOM_ID_ALIASES
        .iter()
        .find_map(|(alias, canonical)| (*alias == id).then_some(*canonical))
        .unwrap_or(id)
}

/// Find listed room metadata without constructing or rendering a room.
#[must_use]
pub fn room_meta_by_id(id: &str) -> Option<RoomMeta> {
    let id = canonical_room_id(id);
    ROOM_CATALOG
        .iter()
        .find(|metadata| metadata.id == id)
        .copied()
}

pub(crate) fn construct_all(variation: u64) -> Vec<Box<dyn Room>> {
    CATALOG_CONSTRUCTORS
        .iter()
        .map(|constructor| constructor(variation))
        .collect()
}

pub(crate) fn construct_by_id(id: &str, variation: u64) -> Option<Box<dyn Room>> {
    let id = canonical_room_id(id);
    ROOM_CATALOG
        .iter()
        .position(|metadata| metadata.id == id)
        .map(|index| CATALOG_CONSTRUCTORS[index](variation))
}

pub(crate) fn construct_hidden_by_id(id: &str) -> Option<Box<dyn Room>> {
    HIDDEN_ROOM_METADATA
        .iter()
        .position(|metadata| metadata.id == id)
        .map(|index| HIDDEN_ROOM_CONSTRUCTORS[index]())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHA5_ORDERED_METADATA_CHECKSUM: u64 = 0x82d9_d564_a8cf_5a71;

    fn extend_checksum(mut checksum: u64, bytes: &[u8]) -> u64 {
        for byte in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
            checksum ^= u64::from(*byte);
            checksum = checksum.wrapping_mul(0x0000_0100_0000_01b3);
        }
        checksum
    }

    fn ordered_metadata_checksum() -> u64 {
        let mut checksum = 0xcbf2_9ce4_8422_2325;
        checksum = extend_checksum(checksum, &(ROOM_CATALOG.len() as u64).to_le_bytes());
        for metadata in ROOM_CATALOG {
            for field in [metadata.id, metadata.title, metadata.wing, metadata.blurb] {
                checksum = extend_checksum(checksum, field.as_bytes());
            }
            checksum = extend_checksum(checksum, &metadata.accent);
        }
        checksum
    }

    #[test]
    fn metadata_and_constructors_stay_in_exact_lockstep() {
        assert_eq!(ROOM_CATALOG.len(), CATALOG_CONSTRUCTORS.len());
        assert_eq!(ROOM_CATALOG.len(), ROOM_SOURCE_IDS.len());
        assert_eq!(HIDDEN_ROOM_METADATA.len(), HIDDEN_ROOM_CONSTRUCTORS.len());
        for (index, metadata) in ROOM_CATALOG.iter().enumerate() {
            assert_eq!(CATALOG_CONSTRUCTORS[index](0).meta(), *metadata);
            assert_eq!(ROOM_SOURCE_IDS[index].1, metadata.id);
        }
        for (index, metadata) in HIDDEN_ROOM_METADATA.iter().enumerate() {
            assert_eq!(HIDDEN_ROOM_CONSTRUCTORS[index]().meta(), *metadata);
        }
    }

    #[test]
    fn metadata_lookup_hides_hidden_rooms_and_resolves_aliases() {
        assert_eq!(
            room_meta_by_id("times-tables").map(|metadata| metadata.title),
            Some("Times Tables")
        );
        assert_eq!(
            room_meta_by_id("kepler-areas").map(|metadata| metadata.id),
            Some("kepler-laws")
        );
        assert!(room_meta_by_id("tetractys").is_none());
        assert!(room_meta_by_id("missing").is_none());
    }

    #[test]
    fn compatibility_alias_constructs_the_canonical_varied_room() {
        let alias = construct_by_id("kepler-areas", 17).expect("known alias");
        let canonical = construct_by_id("kepler-laws", 17).expect("canonical room");
        assert_eq!(alias.meta(), canonical.meta());
        assert_eq!(alias.meta().id, "kepler-laws");
    }

    #[test]
    fn listed_and_hidden_ids_are_unique_and_disjoint() {
        let metadata: Vec<_> = ROOM_CATALOG.iter().chain(HIDDEN_ROOM_METADATA).collect();
        for (index, current) in metadata.iter().enumerate() {
            assert!(
                metadata[..index]
                    .iter()
                    .all(|earlier| earlier.id != current.id),
                "duplicate listed or hidden room id: {}",
                current.id
            );
        }
        for (alias, canonical) in ROOM_ID_ALIASES {
            assert!(
                metadata.iter().all(|room| room.id != *alias),
                "room alias collides with listed or hidden id: {alias}"
            );
            assert!(
                ROOM_CATALOG.iter().any(|room| room.id == *canonical),
                "room alias points outside the listed catalog: {alias}"
            );
        }
    }

    #[test]
    fn ordered_metadata_matches_the_alpha5_migration_receipt() {
        assert_eq!(
            ordered_metadata_checksum(),
            ALPHA5_ORDERED_METADATA_CHECKSUM
        );
    }

    #[test]
    fn no_doorway_sells_a_staged_rooms_answer() {
        // `describe_room` is reachable before any play, so a blurb is the one
        // piece of a staged room a player can read without committing. Three
        // doorways used to carry a graded answer: Buffon named pi, Parrondo
        // named ABB, and the Coffee Cup handed over the Times Tables reveal.
        // A doorway may name the question and the strangeness, never the call.
        fn names(blurb: &str, word: &str) -> bool {
            blurb
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|found| found == word)
        }

        // A staged room must not print the call it grades.
        const OWN_ANSWER: [(&str, &str); 4] = [
            ("buffon-needle", "pi"),
            ("parrondo", "abb"),
            ("times-tables", "mandelbrot"),
            ("nontransitive", "nontransitive"),
        ];
        for metadata in ROOM_CATALOG {
            let blurb = metadata.blurb.to_ascii_lowercase();
            for (room, answer) in OWN_ANSWER {
                assert!(
                    metadata.id != room || !names(&blurb, answer),
                    "the {room} doorway prints the call it grades ({answer:?}): {}",
                    metadata.blurb
                );
            }
            // The cardioid-is-the-main-bulb identity is the Times Tables
            // reveal. Naming the Mandelbrot set is fine; asserting that
            // identity in any doorway sells an aha the player has not bought.
            let claims_identity = names(&blurb, "cardioid") && names(&blurb, "mandelbrot")
                || blurb.contains("main bulb");
            assert!(
                !claims_identity,
                "the {} doorway asserts the Times Tables identity: {}",
                metadata.id, metadata.blurb
            );
        }
    }
}
