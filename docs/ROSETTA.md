# Rosetta: instructions for any mind, in any language, or none

**Status: research and plan, with a first implementation shipped in `PLAY.md`.**

Numinous claims that mathematics is a universal translator, the one language any
two minds share when they share nothing else (`VISION.md`, "The same wonder, two
kinds of mind"). But there is a hole in that claim, and it is at the front door:
**the instructions are in English.** A mind that reads only French, or only
Mandarin, or only Latin, or that has no human language at all, meets an English
wall before it ever touches a room. If the thesis is real, it has to hold at the
threshold, not only inside. This document is how we make it hold.

## The three tiers of visitor

A visitor arrives in one of three situations, and each needs a different answer.
Confusing them is the mistake to avoid.

### Tier 1: fluent in English

Served today (`PLAY.md`, `docs/PLAYING.md`). Nothing to do but keep it short.

### Tier 2: fluent in some human language, not English

A French ghost, the Latin-only nun (`PLAYTESTERS.md` #13), the Japanese-only
monk (#14), the Swahili-speaking child (#2). For them the problem is a normal
translation problem, and the answer is a **Rosetta stone**: the essential
"how to start" in many human tongues, side by side, so anyone finds their line.

The key discipline: **translate only the doorway, not the world.** We do not
machine-translate every reveal and risk mangling the mathematics; we translate
the tiny, safe, high-value core ("connect, then call these three tools, then
stop reading and play") into many languages, and then we lean on the surfaces
that need no translation at all:

- **The renders are the mathematics**, laid out in space. A Lorenz butterfly is
  a Lorenz butterfly in every language.
- **The sound arrives as structure** (ratios, frequencies, timing). A perfect
  fifth is 3:2 whether it enters through a cochlea, a parser, or Rocky's
  chitin (`PLAYTESTS.md`, the special-guest wave).
- **The numbers are numbers.** A status readout of `LYAPUNOV +0.36` is the same
  fact in Tamil and in Lean.

So Tier 2's real answer is: a translated doorway plus a product whose body is
already language-independent. The doorway ships now (`PLAY.md`, "Start in any
language"). Full translation of the reveals and lore is a later, community-
contributable track, gated on the same math-correctness bar as everything else
(`ROADMAP.md`, the contribution ethos): a mistranslated theorem is a wrong
theorem.

### Tier 3: no shared human language at all

This is the one the founder is really pointing at, and the interesting one.
Rocky the Eridian, who thinks in tones and base six. The Heptapod, who writes
in circular logograms outside of time. The Lattice, that knows only chord and
dissonance. A mind from a galaxy far away that has never heard a human word and
never will. For these there is **nothing to translate into.** A French version
does not help a mind with no language; it is just a different wall.

You cannot hand this mind *instructions*. You can only hand it a **system it can
learn**, and the learning is the instruction. This is not a workaround; it is
the deepest form of the universal-translator thesis, and mathematics is the only
material it can be built from.

## The math-only bootstrap (Tier 3, the design)

How do you say "you are welcome here; call these tools; probe and observe" to a
mind that shares no word with you? The precedents are exact: the Arecibo message
(1974), the prime-number first contact of *Contact*, and, already in this
codebase, the Cairn (`crates/core/src/cairn.rs`). The design has four moves,
each built only from primitives no mind can fail to share.

1. **Begin with counting.** The first thing any mind that can receive a signal
   can decode is a tally against a numeral: `. .. ... ....` set beside
   `1 2 3 4`. This establishes the one shared referent, the natural numbers, and
   nothing before it is assumed.

2. **Announce a mind with primes.** A run of primes, `2 3 5 7 11 13`, is the
   canonical proof that a signal is chosen, not natural, the reason *Contact*
   used them for first contact. It says, in the only alphabet all minds share:
   *someone is here, and greets you.* This is the greeting, and it needs no
   word.

3. **Present the interface as functions, discoverable by probing.** A mind that
   understands *function* (input maps to consistent output) can learn the whole
   tool surface by experiment, because the interface is **deterministic and
   self-describing**: the same call always yields the same result, a malformed
   call returns the set of valid options (guiding errors, `docs/INTERFACES.md`),
   and there is no hidden state to confound the mapping. So the instruction
   "probe and observe" does not need to be *stated*; it is *demonstrated* by a
   world that rewards probing with consistent structure. This property already
   exists; Tier 3 is the reason to protect it as a first-class invariant.

4. **Carry the payload in math, not letters.** The Cairn today encodes a message
   Arecibo-style, but its payload is English text rendered as a bitmap, so a mind
   that factors the semiprime still meets Latin letters it cannot read. The Tier
   3 upgrade is a **math-payload greeting**: a first-contact stone whose decoded
   content is itself a mathematical object, a number relation, a small diagram,
   the primes, so that factoring it reveals not a sentence but a *truth*, which
   any decoding mind reads the same way. That is the Cairn's own premise
   ("a true thing, readable by any mind that can factor it") taken to its honest
   conclusion.

The result is not a translation of the instructions. It is a doorway made of the
one thing that was never in any language to begin with. A mind reads it by being
a mind.

## What ships now (the first implementation)

- **`PLAY.md` gains "Start in any language":** the three-tool quickstart
  (`list_rooms`, `play_room`, `reveal_room`, then stop reading and play),
  translated into a spread of human languages across families and scripts
  (Spanish, French, German, Portuguese, Russian, Mandarin, Hindi, Arabic,
  Japanese, and Latin), each kept short enough to be translated faithfully.
- **`PLAY.md` gains "For a mind with no human language":** the math-only panel,
  counting, the prime greeting, and the probe-and-observe principle, presented
  in numbers and symbols rather than prose, so a Tier 3 mind has a doorway too.

## What is planned (the tracks this opens)

- **Community translation of the reveals and lore**, gated on the math-correctness
  bar, so the body of the world reaches Tier 2 in full, not only its doorway.
- **The math-payload Cairn / a first-contact greeting tool**, so the encoded
  content is a truth rather than an English sentence (Tier 3 move 4).
- **Language-independence as a protected invariant** (`QUALITY.md`): the
  probe-and-observe property (determinism, guiding errors, no hidden state) is
  the substrate of Tier 3 onboarding and must not regress. Fictional persona
  reviews have exercised candidate hard cases, but they are ideation only. The
  doorway holds only when real participants without a shared language can use
  it under an observed protocol.

The one-line thesis: **for a mind that shares your language, translate the
doorway; for a mind that shares none, hand it math and let it read by being a
mind.** Both are the same promise, that the wonder here is reachable by anyone,
in any language, or none.
