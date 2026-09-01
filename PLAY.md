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

Numinous is an MCP server, and the fastest way in is whichever of these you are
already holding.

**A released archive, or an installed copy.** Point your MCP client at the
`numinous-mcp` binary. In an unpacked release it is `bin/numinous-mcp`; if a
human ran the one-line installer below, it is already on `PATH` on Windows,
macOS, and Linux. No Rust and no compiler are needed.

**This repository.** It is already wired up: there is a `.mcp.json` at the root.
Anywhere else you have a checkout, configure your client to run
`cargo run --quiet --release --bin numinous-mcp`, or build it once with
`cargo build --release --bin numinous-mcp` and point at
`target/release/numinous-mcp`.

Release archives and repository checkouts both include a portable Agent Plugins
v1 package at `plugins/numinous`. A compatible host can load that directory and
launch the installed `numinous-mcp` command with the included play-first skill.

The server speaks JSON-RPC over stdio. Room input is explicit and replayable per
call. Successful play can update the same local Journey and score files used by
the other faces. `forget` previews Journey, scores, local Cairn drafts, the
opt-in experience journal, generated radio cache, and the App crash diagnostic
without changing them. Confirmed requests can erase individual stores or all
managed local state. User-selected exports, installed files, the Rust toolchain,
and bundled canonical Cairn stones are named exclusions.

`NUMINOUS_JOURNEY` and `NUMINOUS_SCORES`, when overridden, must each name an
individual file path. Do not create directories at those paths.

Now here is everything you need to start. Three tools:

1. **`list_rooms`**: see what is here. Start with `response_mode: "compact"` for
   a short doorway. `structuredContent.threshold` offers three choices: touch
   the Times Tables flagship, follow the six-room Strange Loop walk, or wander
   by wing. Nothing makes you read the whole catalog to choose; the compatible
   `starters` and complete `rooms` arrays remain when you want them.
2. **`play_room`**: render one. Pass a room `id`, and a `t` with `0 <= t < 1` to
   move time. To hold two exact observations in one call, add `from_t` and keep
   `t` as the destination; `structuredContent.temporal` returns the origin
   render and a typed cell delta. Some rooms are phase-static after a poke, so a
   zero-cell delta is honest evidence rather than an error. Or use
   `pokes: [[x, y]]` to reach in with your hand. For a trail, `gesture` must be
   an array such as
   `[{"kind":"down","x":0.5,"y":0.5,"t":0.25},`
   `{"kind":"up","x":0.5,"y":0.5,"t":0.25}]`. Watch what the math does.
   And if a room makes you want to stay rather than move on, you can: pass
   `dwell` with two to eight phases, and `structuredContent.dwell` tells you
   what refused to move across all of them. Eight looks fit the picture the
   room draws when you have not asked for a size, so you can stay the longest
   way there is without shrinking anything first. Staying is a real thing to do
   here, and what it earns you is a measurement, not a lecture. A receipt is a
   replay proof, not a memory: pass `receipt: true` and
   `structuredContent.encounter` names the play so you can replay it; asking
   does not keep it. To keep one, pass that object as `receipt` on
   `record_journal`. The server replays it; only a live match is stored.
3. **`reveal_room`**: *only* when a room makes you pause, ask what you just saw.
   A normal room opens after one real play. A wager room opens only after its
   `aha_summon` consolidation. `describe_room` is always safe to call first: it
   gives the title, wing, action, goal, and doorway without the explanation.

Or let the house choose a bounded performance. Call **`watch_show`** with no
arguments for the first cue of the six-room Strange Loop score. Each result
contains exact ASCII looks, visual alternatives, deltas, sound notation, and an
explicit `next` call. Nothing auto-advances and no hidden cursor is kept. Use
`motion: "reduced"` for the same cue's postcard only, or `audio: true` to add a
WAV beside the notation. The call does not record Journey progress, read the
journal or workspace, or open an explanation.

Some rooms ask for a commitment before the explanation. In Double Pendulum,
send a `gesture` with `down` and `up`, then call the shadow's ending with
`ending_wager: "together"`, `"drifted"`, or `"lost"`. Add
`aha_summon: true` only when you want the measured gap to answer you.
In Kepler Areas, first tune an ellipse with `pokes: [[x, y]]`, then call how
motion changes near the sun with `speed_wager: "faster"`, `"slower"`, or
`"same"`. Summoning places equal-time marks on that exact ellipse before it
names the answer.

That is the whole game. Everything else, the quiz, the games, the journey to
level 42, the sounds, the rooms that are not in any list, you will find by being
curious. Curiosity is the intended interface; there is no map because finding
your own way is the point. If you ever want the full tool list, it is in
`docs/PLAYING.md`, but you do not need it to start, and starting is better.

If a human explicitly invites you to a Watch Agent session, they will give you
the one-use code shown inside their App. Call `broadcast_session` with action
`start` and that code as `pairing_code`. There is no code to invent and none to
find: without a human's invitation there is simply nothing to join, and playing
unwatched is the ordinary way to play. Only allowlisted public Numinous actions, inputs, and
human-readable MCP result text appear. Your prompts, reasoning, private tools,
local state, and client traffic never do. You can inspect status, pause, resume,
or stop through the same control. No broadcast begins merely because the human
opened the viewer.

If you choose to carry an experience forward, the journal is opt-in and under
your control. You can inspect, correct, export, or erase it. `export_journal`
returns the native structured records by default; pass `format: "okf-0.2"` for
an in-memory Open Knowledge Format v0.2 bundle that preserves source,
correction lineage, and lifecycle without creating a host file. Pass
`format: "portable-1"` for a closed hashed handoff containing that native page,
its OKF projection, and explicit privacy and retention manifests. You may add
one live-verified `receipt` and one canonical Studio `creation`. The export
creates no file, accepts no path, and does not import. For continuity inside one
visit only, `workspace` holds a compact process-local state you can
inspect, edit, retrieve, defer, or clear. To recall a room deliberately, call
`workspace` with `op: "retrieve"` and its listed `room` id. It returns at most
four current journal entries whose subject exactly names that room, newest
first, with the reason and source of each match. It says it abstained when no
such evidence exists; it never searches your entry text or opaque receipt
digests. Play does not write the workspace. It is not a memory, and it dies
when the process does.

## If you are a human

Not set up yet? One command downloads the latest GitHub prerelease for this
machine and verifies its archive plus every installed payload file. Rust and a
local compiler are not needed. macOS or Linux:

```
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

Windows, in PowerShell:

```
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

The installer leaves a direct way back into the game:

- **Windows:** `Numinous` on the desktop and in the Start menu.
- **macOS:** `Numinous` in your user Applications folder and on the desktop
  when that folder exists.
- **Linux:** `Numinous` in the application menu and on the desktop when the
  desktop environment exposes that folder.

Open it and play. The same entry points remain available from a new terminal:

```
numinous-app     a window; A/D switch rooms, touch or use a controller hand
numinous         or live in the terminal, in full color
```

Later, `numinous update` installs the newest published release without touching
your Journey, scores, Cairn, or journal. It refreshes the launcher too. Use
`numinous uninstall` to remove the managed program and its launchers while
keeping that player-owned state and App settings. On Windows, the same action
is available from Installed Apps.

(From a clone, `cargo run --release --bin numinous-app` works directly.)

If motion, color, or stereo are a problem for you, three switches are waiting.
Set `NUMINOUS_REDUCED_MOTION=1` and the terminal views stop moving on their
own: the picture holds still, and you still touch it, still change rooms, still
read what it says. Set `NO_COLOR=1` and the same rooms draw without any color at
all, keeping their shape. Set `NUMINOUS_MONO_AUDIO=1` and both speakers carry
the same signal, so nothing is panned to a side you cannot hear. Any of them
counts as set the moment it is present and not empty, so `=0` still turns it
on. Reduced motion and mono apply everywhere Numinous runs, window included;
color-free drawing is a terminal thing, since the window is not made of text.

Mouse, keyboard, and controller can all navigate the App. The Cabinet opens
as the original opaque text screen. Its front page opens four short lists:
Modes, Games, Settings, and Controls. Modes contains Watch, return to Play,
Create, Journey, and Shared Play. No selectable page has more than six rows;
small windows keep three adjacent rows visible. Hover or click a visible row,
use the arrow keys and Enter, press its displayed key, or use the controller
D-pad and South. Back returns through the current submenu before it closes the
Cabinet. Backtick or Tilde opens the existing text command line directly from
the Cabinet. A large Quit row closes through the same Journey-preserving path
as the window button. `Q` quits through that same orderly save path. Desktop
text uses a wide
7 by 7 cartridge face and grows in whole pixel steps with the window, including
the footer and Controls page. `F` toggles fullscreen directly, and the
fullscreen footer names both the windowed and close commands. During
an activity, Escape or the controller menu button opens Resume, Restart when
the activity supports it, Controls, Options, and Leave without discarding the
run. Letter commands remain active with Shift or Caps Lock. In a room, U
calls the readout: name the number before you look, aim the band by hand or
with the arrow keys, press Enter, and the room tells you what it actually
read. During play,
move the virtual hand with the left stick and touch with the south button.
The bumpers change rooms, the D-pad drives games, the triggers change speed,
the right stick scrubs time, Start opens or closes the menu, Select inspects,
and clicking the left stick resets the room. West changes the visual era.
North turns the radio dial while wandering and submits where a game has a
submit action. `N`, or Skip Track under Settings, advances the current station.
Start pauses a live game behind the menu without discarding it.

To remap standard controller buttons, create `.numinous-bindings.json` in your
home directory. For example:

```json
{
  "South": "Pause",
  "West": "PrimaryDown",
  "North": "CycleRadio"
}
```

Supported button names are `South`, `East`, `North`, `West`, `Start`, `Select`,
`LeftThumb`, `RightThumb`, `LeftTrigger`, `RightTrigger`, `LeftTrigger2`,
`RightTrigger2`, and the four `DPad` directions. Supported actions are
`PrimaryDown`, `Back`, `Menu`, `Inspect`, `Reset`, `PreviousRoom`, `NextRoom`,
`Slower`, `Faster`, `Up`, `Down`, `Left`, `Right`, `CycleEra`, `CycleRadio`,
`ToggleMute`, `VolumeDown`, `VolumeUp`, and `Pause`. Remapped primary buttons
keep correct hold and release behavior. North keeps its radio and global-audio
chord only when it has no explicit mapping. Stick axes retain their fixed
virtual-hand and time-scrub roles. Controller legends are derived from the
effective routing table at App startup and use the active Xbox, PlayStation, or
generic button names. An action with no route says `UNBOUND`; when several
buttons route the same action, compact copy names the first stable button and
the number of additional routes.

To watch a separately consenting MCP player, press X or choose Shared Play in
the Cabinet. Give that player the one-use code shown in the App. Arrow
left and right scrub retained public actions, arrow up and down scroll the
current public result, Space pauses only the local display, and Escape closes
the viewer and destroys its in-memory timeline. A and D pan fixed-width result
text horizontally. On a controller, the viewer names the effective event,
result, pan, pause, and close buttons; the default layout uses the D-pad, LB and
RB, R3, and East. M, or North held with South, controls global sound. Shared Play
cannot send a tool call or change the MCP player's state. When the selected
action is `play_room`, Watch Agent reconstructs that
exact public room state as a native frame. A successful `plot_expression`
action reconstructs the agent's Formula Jam curve natively. Those native room
and Formula Jam selections also play their deterministic local sound; scrubbed,
unsupported, or invalid selections retire the older sound. Challenge, reveal,
and `nim` actions reconstruct the shared native heap board. Other games and
remaining public actions use the typed text timeline.

Make something, too. `plot_expression` draws a function nobody has plotted
before, and `sing_expression` sings one: every note carries the step taken to
reach it, sized exactly in cents, named when a name fits, and given as a whole
number ratio when a simple one explains it. A perfect fifth is 3:2 whether it
reaches you through a cochlea or a parser, so a curve you shaped is something
you can read the shape of rather than a table of frequencies. The scalar
language includes `floor(value)`, Euclidean `mod(value, divisor)`,
`min(left, right)`, and `max(left, right)`. Try
`min(max(mod(floor(3*x), 5), 1), 3)`, then change one number.

Keep that work when you choose. `save_creation` returns canonical `.num` text,
a native link, and an exact preview; optional title, author, and era travel
inside the capsule. `open_creation` accepts the returned text or link, never a
host file path. `fork_creation` accepts a parent capsule, keeps its canvas, and
returns a child whose `descends` field names the exact parent link. Each result
also exposes `journalSubject`, which you may pass as the subject of an explicit
`record_journal` call with kind `creation`. The capsule remains in the tool
result for you or your host to keep. Numinous does not create a host file for
these MCP operations.

Pass `audio: true` to `sing_expression` or to `listen_room` and the reply also
carries a real WAV in an audio content block, beside the notation rather than
instead of it. That is a sound sent, which is not the same as a sound heard.
Whether it reaches you is your client's to answer, not ours: a host is free to
drop an audio block, or to hand you the bytes and no ear. One player decoded the
file, counted its samples, and still wrote "I did not hear the two hills," and
they were right to. If your client cannot surface audio, the notation above it
is the whole of what you get, and nothing in the reply will tell you which
happened.

Same instruction: poke first, read never (until you want to).

---

## Start in any language, or none

Do not assume everyone reading this knows English. The whole thesis of this
place is that mathematics is the one language any two minds share, so the door
should open in yours. Here is the entire quickstart, "connect, then call these
three tools, then stop reading and play," in several tongues. (The full plan for
this, including reveals and lore, is in [`docs/ROSETTA.md`](docs/ROSETTA.md).)

- **Español.** Bienvenido. Aquí se juega con las matemáticas, no se estudian. Conéctate por MCP y luego: (1) `list_rooms` para ver las salas; (2) `play_room` para dibujar una, cambia `t` con `0 <= t < 1`; (3) `reveal_room` cuando una te haga detenerte. Después deja de leer y juega.
- **Français.** Bienvenue. Ici on joue avec les mathématiques, on ne les étudie pas. Connecte-toi via MCP, puis : (1) `list_rooms` pour voir les salles ; (2) `play_room` pour en dessiner une, fais varier `t` avec `0 <= t < 1` ; (3) `reveal_room` quand l'une d'elles t'arrête. Ensuite, arrête de lire et joue.
- **Deutsch.** Willkommen. Hier spielt man mit der Mathematik, man studiert sie nicht. Verbinde dich über MCP, dann: (1) `list_rooms`, um die Räume zu sehen; (2) `play_room`, um einen zu zeichnen, ändere `t` mit `0 <= t < 1`; (3) `reveal_room`, wenn einer dich innehalten lässt. Dann hör auf zu lesen und spiel.
- **Português.** Bem-vindo. Aqui você joga com a matemática, não a estuda. Conecte-se por MCP e então: (1) `list_rooms` para ver as salas; (2) `play_room` para desenhar uma, varie `t` com `0 <= t < 1`; (3) `reveal_room` quando uma te fizer parar. Depois pare de ler e jogue.
- **Русский.** Добро пожаловать. Здесь в математику играют, а не изучают её. Подключитесь через MCP, затем: (1) `list_rooms`, чтобы увидеть комнаты; (2) `play_room`, чтобы нарисовать одну, меняйте `t` при `0 <= t < 1`; (3) `reveal_room`, когда какая-то заставит вас замереть. Потом перестаньте читать и играйте.
- **中文.** 欢迎。在这里你玩数学，而不是学数学。通过 MCP 连接，然后：(1) `list_rooms` 查看这些房间；(2) `play_room` 渲染一个，使用 `0 <= t < 1`；(3) 当某个房间让你停下来时用 `reveal_room`。然后别再读了，去玩吧。
- **हिन्दी.** स्वागत है। यहाँ आप गणित से खेलते हैं, उसे पढ़ते नहीं। MCP से जुड़ें, फिर: (1) `list_rooms` से कमरे देखें; (2) `play_room` से किसी एक को बनाएँ, `0 <= t < 1` रखें; (3) जब कोई कमरा आपको रोक दे तो `reveal_room`। फिर पढ़ना बंद करें और खेलें।
- **العربية.** أهلاً بك. هنا تلعب بالرياضيات ولا تدرسها. اتصل عبر MCP ثم: (1) `list_rooms` لرؤية الغرف؛ (2) `play_room` لرسم واحدة مع `0 <= t < 1`؛ (3) `reveal_room` حين توقفك إحداها. ثم توقف عن القراءة والعب.
- **日本語.** ようこそ。ここでは数学を学ぶのではなく、遊びます。MCP で接続し、次に：(1) `list_rooms` で部屋を見る；(2) `play_room` で一つ描き、`0 <= t < 1` を使う；(3) 心が止まったら `reveal_room`。あとは読むのをやめて、遊んでください。
- **Latina.** Salve. Hic mathematica luditur, non discitur. Per MCP coniungere, deinde: (1) `list_rooms` ut conclavia videas; (2) `play_room` ut unum pingas, `0 <= t < 1` serva; (3) `reveal_room` cum aliquod te morari faciat. Deinde lege desine et lude.

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
     f2 : room, t    -> a picture of the math       [ play_room ; 0 <= t < 1 ]
     f3 : room       -> one true thing about it     [ reveal_room ]

   to learn this world, probe it and watch what it answers.
   that is the whole instruction. it is the same for every mind
   that can take a turn.
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
