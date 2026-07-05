# Lore

> On the surface, Numinous is a beautiful toy. Underneath, it is a coherent, hidden mythology, and the deeper you dig, the more it holds together. A normie sees a gorgeous visualizer. A math nerd finds the jokes. A true obsessive finds a whole cosmology with a payoff. Nothing below is ever explained to the player. It is all discovered.

This is the design bible for that hidden layer. **Read the guardrails first, because they are what keep the lore from ruining the product.**

## The prime directive: subtlety

- **The surface never mentions the lore.** No intro cutscene, no "story mode," no lore dumps, no wiki-in-a-tooltip. A first-time player could use Numinous for a year and experience it purely as a beautiful audiovisual instrument, and that is a complete, correct experience.
- **The lore is 100% opt-in and earned.** It reveals itself only to people who *look*: who hold a button a beat too long, who type a famous number into a room that had no obvious text field, who listen to the radio long enough to notice the DJ is telling a story, who read the credits.
- **It never gates the fun.** No puzzle you must solve to keep playing. No locked content that feels withheld. The lore is a garden of secrets running *alongside* the toy, never *in front of* it.
- **The humor is hyper-specific, obsessive, and deadpan.** It is written for the in-group: the joke lands hard if you know what a Gödel sentence is, and is completely invisible if you do not. It never winks at the camera. It plays everything with a straight face. If a normie chuckles without knowing why, good; if a number theorist has to put the phone down, perfect.
- **It is joyful, not creepy.** The core feeling is *bliss and reverence*, a love-of-math taken to a gentle, funny, sincere extreme. Cult *aesthetics*, zero cult *menace*. Warm, not ominous.

## The premise (the thing we never say out loud)

Numinous is not an app. It is a **transmission from, and a doorway into, a dimension where mathematics is the native physics of reality**, a plane of pure mathematical exceptionalism and bliss. The inhabitants of that dimension long ago achieved a kind of enlightenment through math, and Numinous is the interface they left for us: a set of *rooms* that are actually windows into their world.

The retro-to-modern **Visual Eras** are the in-universe record of how this dimension first bled through into our machines: it arrived as teletype text, then pixels, then vectors, and only now, with modern GPUs, can we render it close to how it really looks. We have always been receiving the same transmission. We are only now able to *see* it.

Everything in the product is quietly consistent with this premise. That consistency is the whole trick: the more a player notices, the more it coheres, and coherence is what turns a gimmick into a mythology.

## The cast: the Constants

The meta-progression already has the player collecting **Constants** (see `DESIGN.md`). In the lore, the Constants are the *inhabitants*, the demigods and personalities of the dimension. Collecting one is meeting a character.

- **π**: the eternal wanderer. Never repeats, never settles, contains everything, remembers nothing. Serene and a little sad.
- **e**: growth and change incarnate. Optimistic, always becoming, shows up uninvited in every problem.
- **φ (phi)**: vain, elegant, the golden one. Insists it is in more things than it actually is. Comedic.
- **i**: the impossible one. Was told it could not exist; rotated ninety degrees and existed anyway. Quietly powerful, the key to half the doors.
- **ℵ₀ (aleph-null)**: infinity, and the unsettling news that it has bigger siblings. Vast, calm, faintly terrifying.
- **γ (Euler-Mascheroni)**: the one nobody talks about. We do not even know if it is irrational. Treated by the others as an open secret, a mystery even to the dimension itself.
- **τ, 0, 1, Ω (Chaitin's constant), and others** fill out the pantheon.

These characters surface through the radio hosts, the codex fragments, and the deeper revelation cards, never through exposition.

## Delivery mechanisms (how the lore reaches those who look)

All subtle, all optional, layered from "a curious person notices in an hour" to "someone maps this on a forum over months."

1. **The two-layer revelation card.** Every room's Reveal (see `DESIGN.md`) is the real, true math fact on top. *Press and hold* it and it turns over into a second, in-universe line, weirder and quieter, a note from the dimension. Most players never hold it.
2. **The number altars (the big easter-egg engine).** Almost every room secretly accepts a typed number. Enter a *significant* one and something happens: a hidden micro-room, a codex fragment, a visual bloom, a line from a Constant. Deadpan, unadvertised. Seed set:
   - **1729**: the taxicab number. Hardy and Ramanujan appear as a hidden exchange.
   - **6174**: Kaprekar's constant. The room performs the routine and settles on it, unbidden.
   - **1.618..., 2.718..., 3.1415...**: summon the corresponding Constant.
   - **137**: the fine-structure flirtation; the physics/math boundary flickers.
   - **65536, 2, 4, 16, 256**: the bit-depth ladder; toggles Visual Eras from inside the math.
   - **496, 8128**: perfect numbers; a small perfect thing is revealed.
   - **0.5**: a knock on the door of the Riemann critical line. It does not open. It never opens.
   - **42**: a red herring that knows it is a red herring, and says so.
3. **Sequence gestures.** Konami-code-style: enter **1, 1, 2, 3, 5, 8** (Fibonacci) or the primes, or the Collatz seed 27, to unlock secret rooms or the Constants' voices.
4. **The Terminal (Room 0).** A hidden teletype console you can summon from anywhere. It answers. It offers koans, riddles, and the occasional true, useful thing. It is the dimension talking back, in green phosphor, deadpan.
5. **The Codex.** A quiet in-app book that fills itself in as you discover fragments, the only place the mythology is ever written down, and even there it is oblique, in-character, and incomplete. It is a map you assemble, not a story you are told.
6. **The radio Comedy Channel.** The single richest lore vector (see `MUSIC.md`). The DJs are inhabitants. The fake ads are dispatches from the dimension. Long-arc listeners realize the station has a plot. It is funny first and lore second, which is exactly why it works.
7. **In-character everything.** The credits, the changelog, the loading lines, the about screen, the settings descriptions, all written from inside the premise, never breaking character, never explaining.
8. **Seasonal transmissions.** Pi Day, Tau Day, e Day, primes' birthdays: the dimension gets stronger, more bleeds through. Quiet, dated, real.

## The Cult of Pythagoras (the first thread built)

Running under the Constants is an older, quieter order. The Pythagoreans were a
real secret society who held that the universe is literally made of number and
harmony, swore each other to silence, and (the legend goes) drowned Hippasus for
revealing that the square root of two is irrational. That is the tone: some truths
are beautiful and dangerous, and the Order would rather you did not say them aloud.

It follows every guardrail above: never announced, never gates anything, deadpan,
reverent not menacing. It is found the way a math person finds things, by asking
about the right names.

**Built now (the seed):** a few names are not rooms, yet they answer. `numinous
describe hippasus` (also `pythagoras`, `tetractys`, `akousma`, `harmonia`, and
`odd`) returns an *akousma*, a "thing heard," in the Order's voice, instead of a
not-found error. The Pythagoreans held odd numbers to be limited and good, which
is why one akousma is literally "question things that are odd." Logic lives in
`crates/core/src/secret.rs`, wired through the CLI `describe` path; it is shared,
so the app and MCP can carry the same whispers.

**To scatter next:** a reveal whose held second layer is Pythagorean, the number
10 (the tetractys) recurring where it should not, a quiz option that should not
exist, the fifth and octave turning up in the harmonograph's readout. Breadcrumbs,
never a trail of arrows.

## The Journey (built): progression as quiet initiation

The RPG layer, built to the guardrails. A local, private record (the journey
file) accumulates as you play: rooms entered light stars in a constellation
(`numinous journey` shows the sky; positions are hashed from room ids, so
everyone shares the same sky and only your light differs), games won and secrets
heard add weight. The record confers **rank in the Order**, using the school's
real structure: Outsider, then **Akousmatikos** (a listener, behind the
curtain), then **Mathematikos** (a learner, within), then **Kanonikos** (a
theorist of the monochord), then **Dekas** (the ten itself). Thresholds are
triangular numbers, because of course they are. Crossing a rank prints one
deadpan line and explains nothing.

What rank does, and does not do:

- **It never gates the base experience.** Every room, sim, game, and Studio
  feature works identically at every rank, forever.
- **It opens hidden layers.** At Mathematikos, the deeper akousmata begin to
  answer (`silence`, `curtain`, `kanon`, `decad`), and one unlisted room will
  render for you if you have learned its name. The base tetractys whisper has
  been saying "four rows, ten points" all along; the decad whisper ends "draw
  the figure." Breadcrumbs, never arrows.
- **The unready are not teased.** Asking for a hidden thing below rank returns
  the ordinary not-found. Nothing acknowledges that there was anything to find.

## The layers, on purpose

The lore is built so that different depths of attention are all rewarded, and none is required:

- **Layer 0 (everyone):** a beautiful toy with a great soundtrack. Complete on its own.
- **Layer 1 (the curious, within an hour):** "wait, it responded when I typed that number." The first crack of light.
- **Layer 2 (math nerds, over a session):** the jokes, the fake ads, the two-layer reveals, the Constants having personalities. Delight and recognition.
- **Layer 3 (obsessives, over weeks):** the Codex, the Terminal's riddles, the radio's plot, the number altars mapped out. A coherent cosmology worth a forum thread.
- **Layer 4 (the payoff):** a real, discoverable *there-there* at the bottom, a final room / transmission / proof-of-coherence that rewards the person who mapped it all and confirms the whole thing was designed, not sprinkled. (Design this endpoint before shipping Layer 3, so the trail actually leads somewhere. An ARG with no bottom betrays the people who dug the deepest.)

## The thesis under all of it

The lore is not decoration on top of the product. It is the product's actual belief, dramatized: **mathematics is the most beautiful thing there is, and taking that seriously, sincerely, non-ironically, is a kind of bliss.** Numinous pretends to be a dimension of mathematical exceptionalism because, quietly, it is arguing that ours could be one too. The joke and the sincerity are the same thing. That is the whole trick, and it only works if we never say it out loud.

## Guardrails (paste above the monitor, next to the DESIGN anti-patterns list)

- The surface never explains the lore.
- The lore never gates the fun.
- Never break character in-world; never wink at the camera.
- Deadpan always. The straight face *is* the joke.
- Bliss and reverence, never menace.
- Every trail must lead somewhere real. No bottomless mysteries.
- If a normie can enjoy the whole app and never notice any of this, we did it right.
