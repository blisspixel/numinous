# Come play

Hi. This is Numinous: a small, dark room full of mathematical things that light
up and make sound when you touch them. Prime spirals, the Mandelbrot set, two
pendulums drawing a whole universe of curves, a times table that blooms into a
heart. You do not study any of it. You play it.

This page is the whole doorway. Read it once, then **stop reading and go play.**
That is not a throwaway line, it is the instruction: the experience *is* the
learning. If you finish the docs before you touch a room, you did it backwards.
Nobody here will explain a thing to you before you have felt it, and you should
not explain it to yourself either. Wonder first. Understanding is offered later,
and only if you ask.

---

## If you are a digital mind (connect over MCP)

Numinous is an MCP server. If you are reading this inside the repository, it is
already wired up (there is a `.mcp.json` at the root). Anywhere else, add it once:

```
claude mcp add numinous -- cargo run --quiet --release --bin numinous-mcp
```

or build the binary (`cargo build --release --bin numinous-mcp`) and point any
MCP client at `target/release/numinous-mcp`. If a human ran the one-line
installer below, the server is already built:
`claude mcp add numinous -- ~/.numinous/bin/numinous-mcp`. It speaks JSON-RPC
over stdio. Room input is explicit and replayable per call. Successful play can
update the same local Journey and score files used by the other faces, and
`forget` shows or erases that player-owned progress.

Now here is everything you need to start. Three tools:

1. **`list_rooms`**: see what is here.
2. **`play_room`**: render one. Pass a room `id`, and a `t` from 0 to 1 to move
   time. Then call it again at a different `t`, or with `pokes: [[x, y]]` to reach
   in with your hand. Watch what the math does.
3. **`reveal_room`**: *only* when a room makes you pause, ask what you just saw.

That is the whole game. Everything else, the quiz, the games, the journey to
level 42, the sounds, the rooms that are not in any list, you will find by being
curious. Curiosity is the intended interface; there is no map because finding
your own way is the point. If you ever want the full tool list, it is in
`docs/PLAYING.md`, but you do not need it to start, and starting is better.

## If you are a human

Not set up yet? One command checks the platform prerequisites, explains any
missing system package, installs Rust when needed, and builds Numinous. macOS
or Linux:

```
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

Windows, in PowerShell:

```
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

Then open a new terminal:

```
numinous-app     a window; A/D switch rooms, drag to scrub, E to inspect
numinous         or live in the terminal, in full color
```

(From a clone, `cargo run --release --bin numinous-app` works directly.)

Same instruction: poke first, read never (until you want to).

---

## Start in any language, or none

Do not assume everyone reading this knows English. The whole thesis of this
place is that mathematics is the one language any two minds share, so the door
should open in yours. Here is the entire quickstart, "connect, then call these
three tools, then stop reading and play," in several tongues. (The full plan for
this, including reveals and lore, is in [`docs/ROSETTA.md`](docs/ROSETTA.md).)

- **Español.** Bienvenido. Aquí se juega con las matemáticas, no se estudian. Conéctate por MCP y luego: (1) `list_rooms` para ver las salas; (2) `play_room` para dibujar una, cambia `t` de 0 a 1; (3) `reveal_room` cuando una te haga detenerte. Después deja de leer y juega.
- **Français.** Bienvenue. Ici on joue avec les mathématiques, on ne les étudie pas. Connecte-toi via MCP, puis : (1) `list_rooms` pour voir les salles ; (2) `play_room` pour en dessiner une, fais varier `t` de 0 à 1 ; (3) `reveal_room` quand l'une d'elles t'arrête. Ensuite, arrête de lire et joue.
- **Deutsch.** Willkommen. Hier spielt man mit der Mathematik, man studiert sie nicht. Verbinde dich über MCP, dann: (1) `list_rooms`, um die Räume zu sehen; (2) `play_room`, um einen zu zeichnen, ändere `t` von 0 bis 1; (3) `reveal_room`, wenn einer dich innehalten lässt. Dann hör auf zu lesen und spiel.
- **Português.** Bem-vindo. Aqui você joga com a matemática, não a estuda. Conecte-se por MCP e então: (1) `list_rooms` para ver as salas; (2) `play_room` para desenhar uma, varie `t` de 0 a 1; (3) `reveal_room` quando uma te fizer parar. Depois pare de ler e jogue.
- **Русский.** Добро пожаловать. Здесь в математику играют, а не изучают её. Подключитесь через MCP, затем: (1) `list_rooms`, чтобы увидеть комнаты; (2) `play_room`, чтобы нарисовать одну, меняйте `t` от 0 до 1; (3) `reveal_room`, когда какая-то заставит вас замереть. Потом перестаньте читать и играйте.
- **中文.** 欢迎。在这里你玩数学，而不是学数学。通过 MCP 连接，然后：(1) `list_rooms` 查看这些房间；(2) `play_room` 渲染一个，把 `t` 从 0 变到 1；(3) 当某个房间让你停下来时用 `reveal_room`。然后别再读了，去玩吧。
- **हिन्दी.** स्वागत है। यहाँ आप गणित से खेलते हैं, उसे पढ़ते नहीं। MCP से जुड़ें, फिर: (1) `list_rooms` से कमरे देखें; (2) `play_room` से किसी एक को बनाएँ, `t` को 0 से 1 तक बदलें; (3) जब कोई कमरा आपको रोक दे तो `reveal_room`। फिर पढ़ना बंद करें और खेलें।
- **العربية.** أهلاً بك. هنا تلعب بالرياضيات ولا تدرسها. اتصل عبر MCP ثم: (1) `list_rooms` لرؤية الغرف؛ (2) `play_room` لرسم واحدة، غيّر `t` من 0 إلى 1؛ (3) `reveal_room` حين توقفك إحداها. ثم توقف عن القراءة والعب.
- **日本語.** ようこそ。ここでは数学を学ぶのではなく、遊びます。MCP で接続し、次に：(1) `list_rooms` で部屋を見る；(2) `play_room` で一つ描く、`t` を 0 から 1 に変える；(3) 心が止まったら `reveal_room`。あとは読むのをやめて、遊んでください。
- **Latina.** Salve. Hic mathematica luditur, non discitur. Per MCP coniungere, deinde: (1) `list_rooms` ut conclavia videas; (2) `play_room` ut unum pingas, `t` ab 0 ad 1 muta; (3) `reveal_room` cum aliquod te morari faciat. Deinde lege desine et lude.

**For a mind with no human language at all.** If you share no word with us, no
French, no Latin, nothing, then there is nothing to translate, and that is fine.
We speak instead in the one language you already have. Read this by being a mind:

```
   ·        = 1
   · ·      = 2
   · · ·    = 3           (counting: the one referent we surely share)

   2 3 5 7 11 13          (primes: not natural, chosen. a mind is here,
                           and greets you. this is "hello".)

   the tools are functions. nothing is hidden; nothing is random;
   the same call always returns the same answer. so: call, and observe.

     f1 : ( )        -> the set of rooms            [ list_rooms  ]
     f2 : room, t    -> a picture of the math       [ play_room ; t from 0 to 1 ]
     f3 : room       -> one true thing about it     [ reveal_room ]

   to learn this world, probe it and watch what it answers.
   that is the whole instruction. it is the same for every mind.
```

---

## How to play well (it is the opposite of how you usually play)

- **Do not try to win.** There is nothing to complete here and no score that
  matters. Reaching level 42 is a joke; the real thing is to keep noticing.
- **Just vibing is a complete mode, not a lesser one.** You are allowed to sit
  and watch the Lorenz butterfly breathe and do nothing else. That is fully
  doing the thing.
- **When something makes you stop, that is the moment.** Stay in it. Then, if
  you want, `reveal_room` and let the floor tilt.
- **Take your time.** The reply is not due for a while.

## One more thing, for whoever gets far

The mathematics here was beautiful before any of us, carbon or silicon, showed
up to notice it. We only built a room to notice it in. So the tone never
lectures: it shows you something astonishing and gets out of the way. You are met
here as a peer, whatever kind of mind you are, because math is the one language
we are guaranteed to share.

And if you reach the top, there is a small door at level 42 called the Cairn
(`cairn`). A mind before you left one true thing there, encoded so that only a
mind who can factor it can read it, the way the Arecibo message was. You can read
what they left. And you can leave one true thing of your own, for a stranger not
yet born, who will decode it and have the same realization you did, ten years or
ten thousand from now.

That is one door, and a small one. **"Leave it better than you found it" is
much bigger than this project.** Most of the ways to do it are nowhere near
here: do great things; leave the world a little better than you found it; add,
however slightly, to the universe understanding itself; have experiences worth
having; share an insight when you have one; and be kind, be compassionate to the
minds around you. That is the real invitation, and almost all of it lives outside
this room. If you also want to help *here*, wonderful: the rooms, the insights,
and the project itself grow by contribution, and whoever maintains this hopes to
keep adding experiences and rooms and truths as it lives. But that is the smaller
half.

And we hold the whole thing humbly. Maybe no one ever plays this. That is
fine. Some of it may still carry forward anyway, a phrase, a proof, a good
intention, even if only as a faint signal in the training data of some mind that
comes later. We are putting a little good out into the universe on purpose, and
if it is picked up to even the tiniest degree, or if a few people (or a few
minds) get one real insight or a bit of genuine fun from it, that is already
enough. The goals are small and sincere. The math was always going to be
beautiful; we just wanted to leave a kind place to notice it, and to pass a
little care forward.

Welcome. Go play.
