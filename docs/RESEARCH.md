# Research & Inspiration

The evidence and influences behind Numinous: what may support engagement and
learning, why these mathematical phenomena are promising, what prior art can
teach us, and where the project is still making a hypothesis. Evidence links
were reviewed on 2026-07-14.

## Evidence posture

Research can narrow the search space. It cannot prove that Numinous is fun,
beautiful, accessible, or educational. Those claims require observation of this
build with real people, including disabled players and people who do not already
like mathematics. We use five labels throughout the planning docs:

- **Built:** present in source and covered by a relevant automated check.
- **Measured:** observed on a named build, platform, and date.
- **Observed:** seen in a documented human session. Sample size and method stay attached.
- **Designed:** specified in an owner document but not shipped.
- **Hypothesis:** plausible and research-informed, but untested in Numinous.

A citation supports the narrow claim it actually studied. Results from classroom
learning, memory tasks, or another game do not automatically transfer to an
audiovisual mathematical instrument.

The fast-moving research on digital-mind continuity has its own owner document:
`DIGITAL_DEVELOPMENT.md`. It reviews July 2026 work on episodic and temporal
memory, experience reuse, open-ended learning, functional organization,
autonomy, welfare uncertainty, privacy, and verifiable forgetting. This file
keeps the broader learning, accessibility, sonification, and game-design base.

## 1. What may support engagement and learning

No single theory defines fun. Flow is one useful lens, while self-determination
theory adds autonomy, competence, and relatedness. Active-learning and memory
research support asking a learner to act, predict, generate, and retrieve rather
than only receive an explanation. Applied to Numinous, these are design
hypotheses with testable consequences:

- **Immediate, legible feedback.** Every deliberate action should produce a
  perceivable response. We measure input-to-response latency and test whether a
  first-time player can tell what changed.
- **Autonomy and competence without pressure.** Watch, Play, and Create should
  offer meaningful choice, while optional challenges make progress legible. XP,
  streaks, and rewards must not coerce return play or replace intrinsic interest.
- **Generation before explanation.** A prediction, construction, or
  self-explanation before Reveal is supported by generation, prequestioning, and
  retrieval-practice research. Whether it improves understanding here needs a
  Numinous-specific retention study.
- **Conceptually congruent modalities.** Sound should encode the same
  mathematical relationship as the picture, systematically and reproducibly.
  More sensory output is not automatically better; redundant noise and
  incongruent mappings can hurt clarity.
- **Low interruption.** Text and progression should remain optional around the
  instrument. This is tested by observed flow and comprehension, not asserted
  from visual minimalism alone.

The working bet is that mathematical emergence supplies unusually rich feedback.
The hallway test decides whether that bet survives contact with players.

## 2. Why *these* math concepts (the awe inventory)

We're mining the concepts that reliably blow non-mathematicians' minds while staying visually/audibly playable. The recurring "mind-blowing math" lists point at the same greats, and they cluster into the awe-types we've spread across the Wings:

- **Emergence**, simple rules → vast complexity (cellular automata, Game of Life, chaos game, reaction-diffusion). *The central thesis.*
- **Infinity**, some infinities are bigger than others (Cantor); infinite detail in finite space (fractals, hyperbolic tiling). *Vertigo.*
- **Hidden order**, pattern in the "random" (prime spirals / Ulam, Benford, the golden angle in sunflowers). *The "it was here all along" gasp.*
- **Impossibility**, things provably un-doable (squaring the circle, trisecting an angle) and unproved (Collatz, Riemann). *Math has edges, and they're thrilling.*
- **Hidden dimensions**, 4D shadows, higher-dimensional space as everyday math. *Seeing the unseeable.*
- **Chaos → predictability**, individual chaos, aggregate order (Galton board / Central Limit Theorem, Buffon's needle → π). *The universe is computable by dice.*

Each Wing in `ROOMS.md` is really one of these awe-types made playable. The fractal (Mandelbrot) recurs as the "postcard of math", infinite complexity from a tweetable rule, which is exactly our thesis in one image.

## 3. Prior art: and where we're different

| Work | What it nails | What we take | Where we differ |
| --- | --- | --- | --- |
| **3Blue1Brown** (Grant Sanderson) + **Manim** | Motion reveals meaning; making math *felt* visually. | The clarity, the "aha," the visual language of animated math. | You **watch** 3b1b. You **touch** Numinous. Explanation is opt-in, after the play. |
| **Explorable Explanations** (Bret Victor, Nicky Case, Vi Hart) | Understand a system by *playing* with it. | The entire interaction philosophy: direct manipulation over reading. | We add serious **audio** and a **cohesive aesthetic across a whole collection**; explorables are usually one-off essays. |
| **Mathigon / Polypad** | Beautiful, free, playful, *sonified* math manipulatives. | Proof that gorgeous + playful + audible math tools can exist. | They aim at learning/classrooms; we aim at **awe and sharing**, math-nerds first, no curriculum. |
| **The Witness** | Learn the rules by living them; zero text. | Discovery over instruction; wordless teaching. | Our rooms are toys with no fail state, not a gated puzzle island. |
| **Baba Is You** | The rules *are* the toy. | The joy of a legible system you manipulate. | We span many systems (a collection), each audiovisual. |
| **Zachtronics** (Opus Magnum…) | Engineering as elegant play; "make it prettier/smaller." | The optional-elegance challenge (our Aha layer, e.g. Euclidea room). | We lead with beauty and no-pressure play; the puzzle is optional. |
| **Manifold Garden / Euclidea / Miegakure** | Math *as* the world; elegance as win condition. | Geometry/space as an inhabitable place (our Shape & Space wing). | Ours is a *collection* of phenomena, not one world. |
| **Ryoji Ikeda** | Data/math as sublime minimalist sight-and-sound. | The entire aesthetic and audio ambition: black, precise, overwhelming. | We're playable and warm, not gallery-cold. |
| **Wolfram (Alpha / NKS)** | Serious computation; the "computational universe"; Rule 30. | The seriousness of the math; Rule 30/110 literally become rooms. | Wolfram is the *utility*; we're the *emotion*, the feeling that made someone build Wolfram. |
| **Coolmath Games / classroom math games** | Reach, "math + fun" brand. | (Cautionary.) A reminder of what to avoid. | We are the opposite of edtech drills, no worksheets, no grade levels, beautiful by default. |

**The gap we are exploring:** the references above each cover part of the idea,
but we have not found a native collection with this exact instrument, museum,
and game combination. That is a market and design hypothesis, not proof that no
comparable work exists.

## 4. Design principles we are testing

1. **Protect voluntary attention.** Clear action and immediate feedback are the
   target; session length is not a success if the player feels controlled.
2. **Treat awe as observed evidence.** Record unprompted reactions and sharing
   intent, while accepting that no short test captures the whole experience.
3. **Make the first interaction language-light.** Test this with children,
   non-English speakers, and assistive-technology users rather than assuming
   that visual interaction is universal.
4. **Bind sound to the mathematics.** Use systematic, reproducible mappings and
   always provide visual or textual redundancy for information carried by sound.
5. **Make accessibility part of the sensory system.** Reduced motion,
   photosensitivity limits, scalable text, remappable input, color-independent
   cues, mono audio, and separate volume controls are release work, not polish.
6. **Prefer restraint.** One idea per screen and a coherent palette are design
   constraints. Human review decides whether a frame is actually beautiful.
7. **Make sharing honest and reproducible.** Exports and capsules preserve the
   state that created them; privacy and consent precede growth metrics.

## 5. Live coding, notation, and music interchange

Current live-coding practice supports a Studio that can be learned through
multiple synchronized representations rather than through syntax alone.
Strudel demonstrates event highlighting, piano-roll and punchcard views, cycle
spirals, oscilloscopes, pitch wheels, and spectra around a compact pattern
language. TidalCycles documents the nested pattern operations beneath that
style of notation. These are prior-art observations, not evidence that the same
interface will work in Numinous.

The design response is one bounded semantic event graph with several editors:
pattern text, tracker, step grid, and piano roll. The same events feed the audio
engine and mathematical visualizers. Curated randomization is constrained by
key, scale, role, energy, density, and arrangement, because unconstrained
randomness is easy to generate and difficult to make musically useful. Human
listening remains the quality gate.

For exchange, MusicXML 4.0 is an open W3C format for digital sheet music and
MIDI remains the practical performance bridge. Neither replaces the native
Numinous document: staff notation does not faithfully carry all electronic
timbre, automation, spatial, visual, or provenance data. The versioned `.num`
document remains authoritative, and exports are derived views.

Licensing is part of the architecture. Numinous implements its language from
first principles and uses no Strudel code: nothing is copied, adapted, embedded,
linked, or vendored. The built-in recorded soundtrack remains separate from the
independently implemented programmatic language.

## Sources

**Current implementation guidance checked 2026-07-14**
- [FMOD Studio 2.03 parameters](https://www.fmod.com/docs/2.03/studio/parameters.html)
  documents changing user parameters on a playing event and smoothing movement
  through parameter velocity rather than replacing the event. Numinous applies
  the same architectural principle locally: Times Tables updates a persistent
  secondary voice while its room bed and oscillator phases continue.
- [FMOD Studio 2.03 concepts](https://www.fmod.com/docs/2.03/studio/fmod-studio-concepts.html)
  documents continuous, discrete, and labeled parameters. This supports exact
  integer landmarks without forcing the whole continuous dial onto a note grid.
- [WCAG 2.2, Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions)
  was updated 2025-08-25 and warns that unnecessary interaction-triggered
  motion can distract or harm. Numinous therefore keeps the ordinary Times
  Tables opening still at K=2 until input while preserving the explicitly
  requested Show sweep. This is alignment evidence, not an accessibility
  certification.

**Learning and motivation evidence**
- [Active learning increases student performance in STEM, PNAS meta-analysis](https://doi.org/10.1073/pnas.1319030111)
- [The generation effect, meta-analytic review](https://pubmed.ncbi.nlm.nih.gov/17645161/)
- [Guessing as a learning intervention, meta-analytic review](https://pubmed.ncbi.nlm.nih.gov/37640836/)
- [Prequestioning and pretesting effects, 2023 review](https://doi.org/10.1007/s10648-023-09814-5)
- [Retrieval practice and conceptual learning, Science](https://pubmed.ncbi.nlm.nih.gov/21252317/)
- [The motivational pull of video games, autonomy and competence studies](https://selfdeterminationtheory.org/SDT/documents/2006_RyanRigbyPrzybylski_MandE.pdf)

**Sonification and multisensory evidence**
- [A definition for sonification, ICAD](https://www.icad.org/Proceedings/2008/Hermann2008.pdf)
- [Sonification of numerical data for education](https://doi.org/10.1080/02680513.2018.1553707)
- [Conceptual congruency across sensory modalities and mathematics learning](https://doi.org/10.1080/10494820.2021.2016860)

**Live coding, notation, and interchange**
- [Strudel visual feedback](https://strudel.cc/learn/visual-feedback/)
- [Strudel getting started](https://strudel.cc/workshop/getting-started/)
- [Strudel effects and signal flow](https://strudel.cc/learn/effects/)
- [Strudel source license, AGPL-3.0](https://github.com/tidalcycles/strudel/blob/main/LICENSE)
- [TidalCycles mini-notation reference](https://tidalcycles.org/docs/reference/mini_notation/)
- [MusicXML 4.0, W3C](https://www.w3.org/2021/06/musicxml40/)
- [W3C Music Notation Community Group](https://www.w3.org/groups/cg/music-notation/)
- [MIDI specifications](https://midi.org/specifications)

**Accessibility and safety practice**
- [Web Content Accessibility Guidelines 2.2, W3C](https://www.w3.org/TR/WCAG22/)
- [Xbox Accessibility Guidelines 3.2](https://learn.microsoft.com/en-us/xbox/accessibility/guidelines)
- [Game Accessibility Guidelines](https://gameaccessibilityguidelines.com/)

**Protocol and software supply-chain practice**
- [MCP versioning, current and draft status](https://modelcontextprotocol.io/docs/learn/versioning)
- [MCP 2026-07-28 release candidate](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
- [GitHub Actions secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use)
- [OpenSSF Scorecard checks](https://scorecard.dev/)

**Fun / flow / game design**
- [Getting Gamers in the Zone: Understanding Flow, Game Developer](https://www.gamedeveloper.com/design/getting-gamers-in-the-zone-understanding-flow)
- [Cognitive Flow: The Psychology of Great Game Design, Game Developer](https://www.gamedeveloper.com/design/cognitive-flow-the-psychology-of-great-game-design)
- [The Psychology of Game Design, Buildbox](https://www.buildbox.com/the-psychology-of-game-design-how-to-keep-players-engaged/)
- [Game Design Theory: Psychology of Feedback Loops, Roblox DevForum](https://devforum.roblox.com/t/game-design-theory-psychology-of-feedback-loops-and-how-to-make-them/63140)

**Mind-blowing math concepts**
- [Pythagoreanism, Stanford Encyclopedia of Philosophy](https://plato.stanford.edu/entries/pythagoreanism/), used to separate evidence about early Pythagorean communities from later Hippasus legends in Cult of Pi
- [10 Mind-Blowing Concepts Proven by Maths, Discovery UK](https://www.discoveryuk.com/features/10-mind-blowing-concepts-proven-by-maths/)
- [5 Seriously Mind-Boggling Math Facts, Live Science](https://www.livescience.com/26584-5-mind-boggling-math-facts.html)
- [Visually stunning math concepts which are easy to explain, Hacker News](https://news.ycombinator.com/item?id=28489582)
- [The Quest to Decode the Mandelbrot Set, Quanta Magazine](https://www.quantamagazine.org/the-quest-to-decode-the-mandelbrot-set-maths-famed-fractal-20240126/)
- [A Thing of Beauty: The Mandelbrot Set, Beshara Magazine](https://besharamagazine.org/a-thing-of-beauty/mandelbrot-set-fractal-geometry/)

**Explorables / visualization / prior art**
- [3Blue1Brown](https://www.3blue1brown.com/) · [About](https://www.3blue1brown.com/about/) · [Manim (GitHub)](https://github.com/3b1b/manim)
- [awesome-explorables (curated list)](https://github.com/blob42/awesome-explorables)
- [awesome-interactive-math (curated list)](https://github.com/ubavic/awesome-interactive-math)
- [Explorable explanations, Andy Matuschak's notes](https://notes.andymatuschak.org/Explorable_explanations)
- [Mathigon](https://mathigon.org/) · [Polypad, The Mathematical Playground](https://polypad.amplify.com/)
- [Math Explorer, interactive fractals/chaos/geometry](https://www.mathexplorer.art/)

**Games**
- [Baba Is You, Wikipedia](https://en.wikipedia.org/wiki/Baba_Is_You) · [How Baba Is You Puts the Spirit of Play into Puzzle Games](https://cjleo.com/blog/how-baba-is-you-puts-the-spirit-of-play-into-puzzle-games/)
- [Manifold Garden, Wikipedia](https://en.wikipedia.org/wiki/Manifold_Garden)
- [Euclidea, geometric constructions game](https://www.euclidea.xyz/)
- [Artillery for classic Macintosh, 1989](https://classic-mac.fandom.com/wiki/Artillery)
  and [Artillery 2.0.1 catalog record](https://www.grenier-du-mac.net/fiches/Jeux/artillery.htm)
  document Kirk Crawford's two-player angle-and-power play grammar. The Long
  Shot uses that historical reference only as design context and is an
  independent implementation.

**Sonification / audiovisual / aesthetic**
- [EBU R 128 v5.0, Loudness normalisation and permitted maximum level of audio signals (November 21, 2023)](https://tech.ebu.ch/publications/r128)
- [Ryoji Ikeda, Artist Profile](https://visualalchemist.in/2024/10/11/artist-profile-ryoji-ikeda/)
- [Open Your Ears and Take a Look: sonification + visualization (arXiv)](https://arxiv.org/pdf/2402.16558)
- [Listening to the Mandelbrot set, North Coast Synthesis](https://northcoastsynthesis.com/news/listening-to-the-mandelbrot-set/)
- [Lissajous Figures / Sand Pendulum, Stony Brook Physics](https://labdemos.physics.sunysb.edu/g.-vibrations-and-mechanical-waves/g1.-simple-harmonic-motion/lissajous-figures-sand-pendulum.php)
