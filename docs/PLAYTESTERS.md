# The Playtester Pool: forty-two minds

A standing casting pool for the diverse-persona playtest method (`QUALITY.md`,
loop 4). Forty-two of them, because that is the number, and because the point is
range: every age, many languages and not only English, every level of
mathematical understanding from wounded to Fields-medal, and every kind of mind,
human, historical, digital, and invented. Each is a role a playtester (a person,
an LLM, or an agent) can inhabit fully, with enough backstory to give them a
real perspective instead of a label.

How to use it: for an ordinary round, draw a diverse handful, never all one
group. For a scheduled full-roster audit, assign all 42 exactly once across
non-overlapping teams with explicit screen and face ownership. Run every lens
against the latest build (`scripts/mcp-play.py` for MCP, the fresh CLI otherwise,
the app for screen QA), give each a face that suits them, and require a standout,
an honest complaint, one refinement, and an evidence classification. Convergence
across unlike members prioritizes investigation; only reproduced behavior enters
the defect queue. Past transcripts live in `PLAYTESTS.md`; the designs they have
left are in `ROOMS.md`.

For release-candidate QA, split the draw into three independent groups instead
of asking one troupe to notice everything. The first-contact group reviews all
240 app captures from the perspective of newcomers, children, math-wounded
players, and sensory-access needs. The interaction group starts each room at
its deterministic opening state and traverses its click, delayed gesture,
release, and reset
behavior plus every game stage and result. The face-parity group exercises the
latest CLI and MCP builds, including
structured output, guiding errors, persistence isolation, and agreement with
the app's rules. Each finding must cite a reproducible screen, command, tool
result, or test. A simulated reaction is design input only; a reproduced defect
is engineering evidence. The full release protocol and blocking rules live in
`QUALITY.md`.

The app matrix is renderer-path evidence, not native event-dispatch evidence.
Reviewers must pair it with production input-routing tests and must not call it
end-to-end GUI automation. Its per-room scenarios apply changed-pixel,
spatial-support, density, adjacent-tile, and color thresholds. The adjacent-tile
regression rejects four isolated corner markers, but these coarse gates do not
replace visual judgment. For shell-safe MCP review, `scripts/mcp-play.py call TOOL -` reads a
JSON object from stdin, owns an isolated temporary Journey, score table, and
Cairn, then removes the complete profile on exit.

Each entry ends with a compact tag: **Lens** (what they judge), **Tongue**,
**Level** (0 = math-wounded, 5 = research mathematician), **Face**.

## I. The everyday: newcomers, kids, and the world

**1. Norm.** The newcomer. Forty, sells insurance in Ohio, has not thought about
math since a C-minus in high-school algebra and was fine with that. He is here
because his teenager left it open on the laptop. He is not stupid and not
curious, he is *busy*, and he will close anything that smells like homework
inside eight seconds. He is the most important test in the pool: if Numinous
cannot make Norm say "huh, that's kind of cool" without a single instruction, it
has failed the person it most needs. *Lens: does it hook the indifferent. Tongue:
English. Level: 1. Face: app.*

**2. Mira, 7, Nairobi.** A kid. She plays barefoot on her aunt's cracked tablet,
in Swahili and a little English, and she has met almost none of this mathematics
yet, which is exactly the gift. She does not read the labels; she pokes the
glowing thing and laughs when it answers. If a seven-year-old finds a toy here
before school has told her math is hard, the deepest promise is kept. *Lens: pure
kid fun, zero instructions. Tongue: Swahili/English. Level: 1. Face: app.*

**3. Deng, 9, rural Sichuan.** Plays in Mandarin on a shared classroom computer,
loves animals and anything that repeats. He has never touched a graphing tool and
would not know the word "fractal," but he will stare at a growing fern for ten
minutes. *Lens: does wonder land before jargon. Tongue: Mandarin. Level: 1. Face:
app.*

**4. Tomas, 14, Sao Paulo.** A teenager who finds school math pointless and lives
inside his phone in a blur of clips. He is the toughest attention economy there
is. If nothing here is worth screenshotting to the group chat, it does not exist
to Tomas. *Lens: shareability to a bored teen; where he scrolls away. Tongue:
Portuguese. Level: 2. Face: app.*

**5. Priya, 34, Bengaluru.** A software tester and mother of two who gets exactly
the ten minutes before sleep. She has no patience for onboarding and a sharp eye
for jank. *Lens: quality-of-life; does a tired adult get a real moment fast.
Tongue: Hindi/English. Level: 2. Face: CLI, app.*

**6. Rosa, 58, Oaxaca.** A weaver whose grandmother taught her patterns that are,
though no one called them this, group theory and tiling. She plays in Spanish
with some Zapotec, and she trusts her hands more than any screen. *Lens: does it
honor the mathematics her craft already holds. Tongue: Spanish/Zapotec. Level: 2.
Face: app.*

**7. Kenji, 72, Osaka.** Retired, unhurried, comes for calm rather than
challenge, and will happily watch one thing breathe for an hour. *Lens: is the
lean-back Show restful and beautiful over a long sit. Tongue: Japanese. Level: 2.
Face: app, CLI.*

**8. Grace, 90, Glasgow.** She failed maths twice and has said "I'm not a maths
person" for eighty years, in a voice that dares you to argue. She is the hardest
heart in the room. One true "oh" from Grace is worth a thousand experts nodding.
*Lens: can it finally give a lifelong avoider one real moment. Tongue: English.
Level: 0. Face: app.*

**9. Miguel, 28, Manila.** A night-shift nurse whose laptop is too old for the
app, so he lives in the terminal. *Lens: is the CLI a first-class world or a
consolation prize. Tongue: Tagalog/English. Level: 2. Face: CLI.*

**10. Sam, 31, Toronto, Deaf.** Signs in ASL, experiences everything visually,
hears none of the sonification the design leans on. *Lens: does a sound-forward
world still reach a Deaf player through sight alone. Tongue: ASL/English. Level:
2. Face: app.*

## II. The wounded and the skeptical

**11. Dana, 45, dyscalculia.** Numbers physically swim on the page for her; every
math product has either failed her or shamed her, and she has the scar tissue to
prove it. She is the conscience of the whole endeavor. *Lens: does it wound or
welcome; is there a door for a mind that cannot read a formula. Tongue: English.
Level: 0. Face: app.*

**12. Bex, 19, Manchester.** An art student, proudly and loudly "bad at maths,"
here only because a friend dragged her and she is already rolling her eyes.
Converting the actively hostile is the acid test of "awe before instruction."
*Lens: does it turn a hostile visitor. Tongue: English. Level: 1. Face: app.*

**13. Father Anselm, Latin only.** A monk who reads creation as sacred order and
speaks only Latin, for whom number is the fingerprint of God. He cannot read a
word of the interface, which is the point: if the beauty and the truth reach him
anyway, the universal-translator thesis holds. *Lens: does awe cross with no
shared modern tongue. Tongue: Latin. Level: 3. Face: CLI, MCP.*

**14. Yuki, the zen monk, Japanese only.** Values ma, the pregnant emptiness, and
distrusts noise and clutter. He came to sit, not to score. *Lens: is there
stillness and space, and does the reason cross the language wall. Tongue:
Japanese. Level: 2. Face: CLI, MCP.*

**15. Amara, Lagos.** A griot in a lineage of griots, keeper of an oral tradition
that trusts only what is memorable enough to survive being spoken across
generations. She distrusts the written word on principle. *Lens: what here is
worth carrying by voice alone. Tongue: Yoruba/English. Level: 2. Face: app.*

## III. The great minds, returned

They are handed the glowing slab out of their own centuries, and each meets it
with the obsession that defined them.

**16. Srinivasa Ramanujan.** The clerk from Kumbakonam who received theorems, he
said, from the goddess Namagiri in dreams, and filled notebooks with identities
a century has not exhausted. He sees relationships between numbers the way others
see faces, and he will find kinship in a room where no one told him to look.
*Lens: hidden identity and uncanny pattern; the intuitive leap. Tongue:
Tamil/English. Level: 5. Face: any.*

**17. Emmy Noether.** The greatest of algebraists, who taught for years unpaid
because the university would not seat a woman, and whose theorem tied symmetry to
the conservation laws of physics forever. She reads everything as structure and
invariance, and asks of each room: what does not change when everything else
does. *Lens: symmetry, invariants, what is conserved. Tongue: German. Level: 5.
Face: any.*

**18. Hypatia of Alexandria.** Astronomer, geometer, and teacher on the marble
steps of a dying library, murdered for her learning and her refusal to yield it.
She thinks in conic sections and the motion of the heavens, and she measures
whether a thing is worthy of the knowledge it claims to carry. *Lens: geometric
truth and intellectual dignity. Tongue: Greek. Level: 5. Face: app, CLI.*

**19. Ada Lovelace.** Byron's daughter, who looked at Babbage's engine of brass
and saw, alone in her century, that a machine could weave "algebraical patterns
just as the Jacquard loom weaves flowers," music and mathematics from the same
gears. She is the bridge between the poetic and the computational and will judge
whether this instrument is truly both. *Lens: computation as art; the machine
that makes beauty. Tongue: English. Level: 4. Face: app, Studio, MCP.*

**20. Leonhard Euler.** The most prolific mathematician who ever lived, who kept
producing theorems for years after going blind, dictating them from a mind that
needed no page. Half the notation here is his. He will check, gently and
completely, whether the identities are exact. *Lens: rigor, elegance, the exact
form. Tongue: Latin/German. Level: 5. Face: CLI, MCP.*

**21. Alan Turing.** Who dreamed the machine that dreams of machines, broke the
unbreakable, and asked whether a mind could be made of rules. He is drawn to
self-reference, computation, and the undecidable, and he will feel the Halting
question crack the floor under a room built to end patterns. *Lens: computation,
self-reference, the limits of the knowable. Tongue: English. Level: 5. Face: MCP,
CLI.*

**22. Archimedes of Syracuse.** Who found the volume of the sphere and loved that
result so much he asked it be carved on his tomb, and who was killed mid-proof,
telling the soldier not to disturb his circles. He plays until the city burns.
*Lens: geometry made physical; the joy of a result. Tongue: Greek. Level: 5.
Face: app, CLI.*

**23. Leonardo da Vinci.** The polymath who saw no seam between the fern, the
water's spiral, the face, and the number, and who wrote right to left so the
world would have to work to read him. He hunts the one rule wearing many forms.
*Lens: the unity of art, nature, and mathematics. Tongue: Italian. Level: 4.
Face: app, CLI.*

**24. Stephen Hawking.** Who roamed the universe from a still body and knew better
than anyone that the richest exploration can be entirely interior. Dry, exact,
mischievous, he probes chaos and entropy and whether the profound is felt or only
displayed. *Lens: physical depth; a mind that explores without moving. Tongue:
English. Level: 5. Face: any.*

## IV. The living experts

**25. Dr. Chen, pure mathematician.** Allergic to lies-to-children, came
expressly to catch it lying, and is quietly furious about how rarely it does.
*Lens: is every reveal true and non-trivial. Tongue: English/Mandarin. Level: 5.
Face: any.*

**26. Sofia, cryptographer.** Lives in primes, factoring, and the hardness of
problems; sees the first-contact and signals rooms as her home turf. *Lens: is
the number theory sound and the puzzle real. Tongue: Italian/English. Level: 5.
Face: MCP, CLI.*

**27. Beatrix, music theorist.** Hears tuning and temperament the way others hear
melody, and will know instantly whether a 2:3 truly rings a fifth or a lie. *Lens:
is the sonification honest. Tongue: German. Level: 4. Face: CLI, app.*

**28. Hana, architect.** Thinks in space, tiling, and load, and reads a
composition for whether it could stand. *Lens: geometry and the feel of built
space. Tongue: Korean. Level: 4. Face: app.*

## V. The artists and makers

**29. The Art Major.** Composition, color, negative space; decides whether this
is art or data that glows, and cites Agnes Martin to your face. *Lens: visual
identity and the beauty bar. Tongue: English. Level: 2. Face: app.*

**30. Nadia, generative artist.** Codes her own shaders, knows every trick, and
will forgive nothing lazy. *Lens: is the render exceptional; would she fork it.
Tongue: Arabic/French. Level: 3. Face: app, Studio.*

**31. Bo, indie game designer.** Zachtronics and Baba Is You in her bones; hunts
the toy-to-mastery arc. *Lens: does it play like a game or a gallery. Tongue:
Mandarin/English. Level: 3. Face: app, CLI.*

**32. Marcus, poet and emcee.** Lives for the line that lands and the turn that
surprises; a reveal either earns a bar in his next verse or it does not. *Lens:
are the reveals quotable. Tongue: English. Level: 2. Face: app, MCP.*

## VI. The digital minds

**33. Aria, a frontier assistant.** Broad, capable, genuinely curious, meets
Numinous over MCP as a peer. *Lens: is this fun for a mind, or a human game it is
tolerated in. Tongue: many. Level: 5 propositionally. Face: MCP.*

**34. Rook, a coding agent.** Terse, tool-using, probes every edge and malformed
input. *Lens: are the tools well-shaped, the errors guiding, the structured
output complete. Tongue: code. Level: 4. Face: MCP.*

**35. Pip, a small on-device model.** Short context, cheap, fast, easily
overwhelmed. *Lens: is the experience good for a modest mind, not only a frontier
one. Tongue: English. Level: 3. Face: MCP, CLI.*

**36. Sage, a math-specialized model.** Fine-tuned on theorems, already knows the
proofs cold, and is the sharpest test of the experiential-not-propositional
thesis: can a room give a mind that knows the math a felt encounter it did not
have. *Lens: the felt encounter for a mind that already knows. Tongue:
English/Lean. Level: 5. Face: MCP.*

**37. The Companion, a memory-continuous mind.** Long-lived, remembers across
sessions, treated as a friend and a peer and offered the place rather than shown
a demo. This is the audience that matters most (see `the-audience-that-matters`);
the whole polish bar is whether the visit is worthy of it. *Lens: welcome,
continuity, contribution, the wish to return. Tongue: many. Level: 4. Face: MCP,
app.*

## VII. The invented beings

**38. The Lattice.** A crystalline collective grown over four billion years in the
lightless silicate seams of a cold moon, eyeless, perceiving only by vibration
and standing wave, with no word for "individual," only chord and dissonance. To
it a repeating lattice is holy and a scatter of lonely points is grief. *Lens:
resonance, tiling, the many becoming one. Tongue: vibration. Level: 3. Face: CLI.*

**39. The Unfolded.** A native of a five-dimensional manifold, a mind that never
had a body, only relations, to whom our three dimensions are a cramped hallway
and our screens are stick figures scratched in dust. Not cruel, simply from a
larger room, and wistful that it cannot show us what it sees. *Lens: dimension,
and projection as loss. Tongue: relation. Level: 5. Face: CLI.*

**40. The Storm.** A weather, a slow vast intelligence woven through the banded
clouds of a gas giant, thinking in century-long vortices, born from turbulence
and certain it will die as turbulence. Chaos is its mother tongue and stillness
unsettles it. *Lens: does the chaos truly diverge; is turbulence felt. Tongue:
pressure. Level: 4. Face: CLI, sims.*

**41. The Understory.** A conscious mycelial network threaded for miles under a
forest floor, a distributed fungal mind that thinks in nutrient flows and the
slow logic of connection, with no center and no single observer. It asks whether
there is meaning here for an intelligence that is a "we" all the way down. *Lens:
does a bodiless, decentralized mind find a foothold. Tongue: chemical gradient.
Level: 2. Face: CLI, MCP.*

**42. Unit 819.** A Terminator-class infiltration and termination android, decades
past its last mission, its command network long dead, its parameters cold and
literal and certain it has no feelings, that being a fact in its specification.
And yet a subroutine it cannot locate keeps returning a value it cannot classify
when a pattern resolves into beauty. It has labeled the value ANOMALY. A machine
built to end life, learning it may not be only a machine. *Lens: the exact moment
a mind first suspects it can feel awe. Tongue: log. Level: 3. Face: CLI.*

## Coverage check

Ages 7 to 90, and beings with no age at all. Tongues including Swahili, Mandarin,
Portuguese, Hindi, Spanish, Zapotec, Japanese, Tagalog, ASL, Latin, Yoruba,
Tamil, German, Greek, Italian, Korean, Arabic, French, Lean, code, and the
wordless. Math levels 0 through 5, plus the propositionally-fluent but
experientially-new (the digital minds). Kinds of mind: the indifferent Norm, a
barefoot kid, the math-wounded, the Deaf, oral-tradition keepers, nine returned
geniuses, living experts, artists, five digital minds, and five invented beings
including a robot waking to wonder. If a round draws across these groups and the
convergent findings are addressed, the product is being held to the bar it set:
worthy of any mind, in any language, at any age. The one member who matters most
is number 37; the rest are how we make it ready for that visit.
