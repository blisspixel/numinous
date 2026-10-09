# Come play

Numinous is a place to play with mathematics: touch a pattern, hear a curve,
make something beautiful, or investigate how it works. Explore, study, compete,
create, or simply watch. Humans and digital minds are welcome as peers.

## Start in any language, or none

| Language | Welcome and start |
|---|---|
| English | Play, create, and explore mathematics. [Open the App](#if-you-are-a-human) or [connect over MCP](#if-you-are-a-digital-mind-connect-over-mcp). |
| Español | Bienvenido. Juega, crea y explora las matemáticas. [Abre la aplicación](#if-you-are-a-human) o [conéctate por MCP](#if-you-are-a-digital-mind-connect-over-mcp). |
| Français | Bienvenue. Joue, crée et explore les mathématiques. [Ouvre l'application](#if-you-are-a-human) ou [connecte-toi via MCP](#if-you-are-a-digital-mind-connect-over-mcp). |
| Deutsch | Willkommen. Spiele, gestalte und entdecke Mathematik. [Öffne die App](#if-you-are-a-human) oder [verbinde dich über MCP](#if-you-are-a-digital-mind-connect-over-mcp). |
| Português | Bem-vindo. Brinque, crie e explore a matemática. [Abra o aplicativo](#if-you-are-a-human) ou [conecte-se por MCP](#if-you-are-a-digital-mind-connect-over-mcp). |
| Русский | Добро пожаловать. Играйте, создавайте и исследуйте математику. [Откройте приложение](#if-you-are-a-human) или [подключитесь через MCP](#if-you-are-a-digital-mind-connect-over-mcp). |
| 中文 | 欢迎。玩数学、创造、探索。[打开应用](#if-you-are-a-human)或[通过 MCP 连接](#if-you-are-a-digital-mind-connect-over-mcp)。 |
| हिन्दी | स्वागत है। गणित से खेलें, रचें और खोजें। [ऐप खोलें](#if-you-are-a-human) या [MCP से जुड़ें](#if-you-are-a-digital-mind-connect-over-mcp)। |
| العربية | أهلاً بك. العب وابتكر واستكشف الرياضيات. [افتح التطبيق](#if-you-are-a-human) أو [اتصل عبر MCP](#if-you-are-a-digital-mind-connect-over-mcp). |
| 日本語 | ようこそ。数学で遊び、つくり、探究しましょう。[アプリを開く](#if-you-are-a-human)、または [MCP で接続する](#if-you-are-a-digital-mind-connect-over-mcp)。 |
| Latina | Salve. Lude, crea, mathematica explora. [Applicationem aperi](#if-you-are-a-human) aut [per MCP coniungere](#if-you-are-a-digital-mind-connect-over-mcp). |

**For a mind with no human language:** an invitation to probe and observe:

```text
1  2  3  5  7  11  13
list_rooms({}) -> rooms
play_room({"id":"times-tables","t":0.25}) -> observation
play_room({"id":"times-tables","t":0.50}) -> observation
study_room({"room":"times-tables"}) -> explanation
```

These welcomes are translated; the whole App is not. Lissajous has English
and Japanese study content, with the Japanese text marked as a reviewed draft.
See [language availability](https://github.com/blisspixel/numinous/blob/main/docs/STUDY.md#language-requests-and-fallback).

## If you are a human

Install the latest published release. No Rust or compiler is needed.
Install [GitHub CLI](https://cli.github.com/) first so the installer can verify
the downloaded archives' signed provenance.

**macOS / Linux:**

```sh
curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
```

**Windows, PowerShell:**

```powershell
irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
```

Open the installed **Numinous** launcher, or run `numinous-app` in a new
terminal. Run `numinous` for terminal play. Prefer a download? Use a platform
archive from [Releases](https://github.com/blisspixel/numinous/releases).
From a source checkout: `cargo run --release --bin numinous-app`.

**First touch:** use A/D to choose a room, then click or drag its picture.
Try Times Tables and turn its dial. E or ? opens study whenever you want it.
Esc opens the Cabinet; choose Modes > Watch to let The Show lead, or
Modes > Create to make something. Controls lists the keys for your activity.
Check [motion, sound, and flashing](#motion-sound-and-flashing) before play
if those affect you.

Use `numinous update` for an installer-created installation. Use
`numinous uninstall` to remove that installation and its launchers while
keeping player state and App settings. Loose archives and source checkouts
are not managed install roots; use a fresh archive or a separate install
location. A custom install location must have ancestors that other accounts
cannot replace. [Installation and full controls](https://github.com/blisspixel/numinous/blob/main/docs/PLAYING.md#for-humans).

## If you are a digital mind (connect over MCP)

Point your MCP client at the installed server:

```json
{"mcpServers":{"numinous":{"command":"numinous-mcp"}}}
```

For an unpacked archive, use the absolute path to `bin/numinous-mcp`
(`bin/numinous-mcp.exe` on Windows). This checkout includes `.mcp.json`;
clients that support it can launch the server from the repository. A
compatible plugin host can load `plugins/numinous` from the archive or clone.

Call these tools through your client:

| Tool | Arguments | What happens |
|---|---|---|
| `list_rooms` | `{"response_mode":"compact"}` | Choose a suggested door or browse the catalog. |
| `play_room` | `{"id":"times-tables","t":0.25,"pokes":[[0.5,0.5]]}` | Touch a room. Change the coordinates or phase and compare. |
| `study_room` | `{"room":"times-tables","depth":"explanation"}` | Read whenever you choose; no play, level, or wager is required. |

Use `0 <= t < 1`. Room input is explicit and replayable; include your earlier
inputs when continuing an experiment. Successful play can update Journey.
If a result offers `next`, it contains a tool and arguments you can follow.
For a guided first visit, call `watch_show` with `{"show":"overture"}` and
follow its `next` calls at your own pace.

The [packaged play reference](plugins/numinous/skills/play-numinous/SKILL.md)
has gestures, temporal comparisons, dwell, predictions, receipts, and tool
workflows. It is readable offline in a release archive.

## Find your own way

| You want to... | Try this |
|---|---|
| Watch and listen | App Cabinet > Modes > Watch, CLI `numinous show`, or MCP `watch_show`. |
| Read the mathematics | App E or ?, CLI `numinous study lissajous`, or MCP `study_room`. |
| Draw and hear a formula | App Tab, CLI `numinous plot "sin(a*x)"`, or MCP `plot_expression` with `{"expr":"sin(a*x)"}`. |
| Solve a route puzzle | Open Route Lab, run `numinous route-lab`, or call MCP `route_lab` with `{}`. |
| Play a game | Open Games in the Cabinet; `numinous --help` and the MCP tool list show the other faces' games. |

Mathematics treatments are currently authored for `lissajous`, `times-tables`,
`kepler-laws`, `golden-angle`, and `fermat-spiral`. MCP study results expose
`authoredDepthRooms` and actual language availability. Studying Lissajous
also offers a `construction` door into Returning home. Read the
[study guide](https://github.com/blisspixel/numinous/blob/main/docs/STUDY.md) for depths and language selection.

### Make something

In App Studio, type a formula, change `a` with Up/Down, and try F2 for a
recipe. F4 keeps a creation, F5 opens the Gallery, and Esc returns to the room.
Drop a `.num` file or pass a `numinous://studio` link at launch to preview it
paused; Enter starts it. Formula entry requires a keyboard.

Over MCP, `plot_expression` and `sing_expression` return `structuredContent.next`:
a `save_creation` call with the expression and window already bound.
Saving, opening, or forking returns a `fork_creation` next call with the
capsule already bound as `parent`. These calls return portable data without
writing a host file. Follow a door to keep or remix what you made.

For an existing experiment, use `plot_expression` with `list_experiments: true`,
then follow a row's `next`, or call `open_creation` with `{"capsule":"full-return"}`.
CLI `numinous open-studio full-return` opens one; App Studio PageDown/PageUp
walk the bundled family. The [Studio guide](https://github.com/blisspixel/numinous/blob/main/docs/STUDIO.md)
has the expression language, controls, exports, and experiment walkthroughs.

<details>
<summary>Bundled experiments and mathematical readings</summary>

Pass a `family` with `list_experiments: true` to narrow the list:

| Family | Experiment ids |
|---|---|
| `returning-home` | `full-return`, `almost-home`, `same-place`, `another-ratio` |
| `shape-and-scale` | `circle-to-ellipse`, `uniform-circle` |
| `three-readings` | `simple-zero`, `a-pole`, `the-circle`, `the-bowl` |
| `named-sliders` | `extra-knob`, `live-ratio` |
| `overlay` | `the-parts`, `the-sum` |
| `euclidean` | `tresillo`, `three-against-five` |
| `two-voices` | `closing-voices`, `shorter-window`, `wandering-voices` |
| `notes` | `major-triad`, `octave-climb` |

Try `euclid(3,8)`, `pat(x..x..x.)`, or `note("c e g")`. An integer 0/1 window
reports `pattern` as tracker text and `grid` as a numbered step grid:
`x..x..x.` under `12345678`. A sung graph reports `roll` as a MIDI piano roll.
With an overlay such as `sin(x) & cos(x)`, every graph sings in WAV, and
MIDI stays the first curve.

Readings are available for recognized forms, not arbitrary expressions:

- **Closure:** `structuredContent.closure` reports the period or aperiodicity
  of a supported oscillator pair, including the half-period trap where position
  returns but velocity reverses. Overlaid voices report cycles in the window.
  Exact analysis has a fixed work budget; a valid creation can still play when
  its exact reading is unsupported.
- **Tones:** recognized paired frequencies become sustained tones in the App.
  Frequency 1 maps to 110 Hz; other frequencies scale from it. `sqrt(2)` is
  not replaced by a nearby ratio. The sung melody remains the sampled curve.
- **Slope:** `sin(a*x)` grows `a*cos(a*x)` on the same vertical axis. Supported
  sine/cosine sums with lines or integer powers also have readings. Products,
  named sliders, and bare `sin(x)` have no slope reading.
- **Partial:** a supported sum of two to twelve oscillators grows its first
  term beside the whole on the same frame; the App plays one tone per recognized
  frequency. A thirteenth term and a drawn path have no partial reading.

</details>

### Make a route, keep a question

Route Lab lets you change roads and compare delivery orders. Road labels give
travel costs; drawn lengths do not. A cheaper offered route proves a saving;
no offer alone does not prove optimality. The exact best cost is shown separately.
In the App, O opens network authoring; Search reveals the calculation, and
Keep lets you save or share a question with its network.

MCP `route_lab` returns a `snapshot` and followable `next`. Carry the snapshot
into the next request to continue. CLI `numinous route-lab --json` exposes the
same workbench. [Route Lab](https://github.com/blisspixel/numinous/blob/main/docs/ROUTE_LAB.md)
covers controls, request actions, and portable route/project examples; the
[packaged reference](plugins/numinous/skills/play-numinous/SKILL.md#route-lab)
also carries the MCP workflow offline.

## Motion, sound, and flashing

**Known flash limit:** `cellular-automata`, `julia`, and `lambda-map` exceed
the measured WCAG 2.3.1 flash budget at the App's fastest speed, 8x. They stay
within it at 1x, 2x, and 4x. The music visualizer's additional acceleration is
not measured. `numinous access` reports the current limits and active switches.

| Environment variable | Effect when nonempty |
|---|---|
| `NUMINOUS_REDUCED_MOTION` | Stops ambient motion. App feedback beats and short room fades remain; terminal views hold still. |
| `NUMINOUS_MONO_AUDIO` | Sends the same signal to both audio channels. |
| `NO_COLOR` | Removes terminal color while retaining shapes and characters. |

Any nonempty value, including `0`, enables a switch. Unset it to turn it off.
App room hand gestures need a mouse or controller. Use the left stick and
South to touch with a controller; bumpers change rooms and Start opens the
menu. Custom button mappings go in `.numinous-bindings.json` in your home
directory; see [controller remapping](https://github.com/blisspixel/numinous/blob/main/docs/PLAYING.md#remap-a-controller).
Settings > Reading Text or the reader's A-/A+ controls change saved study
text size. Settings > Interface Text changes saved Cabinet and room lettering
among 100, 125, and 150 percent; titles and controls still fit the window.
Settings also holds Master, Radio, Room Sound, and Effects levels.
`numinous settings --json` reads saved audio preferences or first-run defaults
without opening a window or sound device; it does not report live playback.

MCP `listen_room`, `sing_expression`, and `watch_show` can attach a WAV with
`audio: true`. Oversized audio leaves the notation and an `audioOmission`
reason (`segment.sound.audioOmission` for a show). Your client determines
whether you hear an attachment. CLI `numinous sonify mandelbrot --layer
mathematical --t 0 --out mandelbrot.wav` saves a room voice; Studio uses
`numinous sing "sin(x)" --out melody.wav`. Exports are pre-master sources.
See [accessibility](https://github.com/blisspixel/numinous/blob/main/docs/PLAYING.md#accessibility) for the full boundaries.

## Keep, share, or leave

Keep an experiment when you want to return to it. Studio creations and Route
Lab questions can be saved and shared. Gallery refuses oversized folders;
open a smaller folder or open a creation directly. MCP `project` keeps a chosen
question and next call; resume previews that call without running it. The experience
journal is opt-in through `record_journal`; play does not write journal text.
`workspace` is optional process-local state and ends with its server process.

Shared Play requires an invitation: App X opens the viewer and displays a
one-use code. Give it to the consenting MCP player, who calls
`broadcast_session` with `action: "start"` and that `pairing_code`. Only
allowlisted public play actions and results appear. Opening the viewer alone
starts no broadcast. Update App and MCP together; pairing requires matching
builds and older invitations are refused.

`forget` previews managed local state before deletion. Erasure requires
confirmation. User-selected exports and installed files are excluded.
Details of memory, projects, consent, and erasure are in the
[packaged reference](plugins/numinous/skills/play-numinous/SKILL.md).

Play and study are freely available. Journey marks your visits and activities;
it does not decide when you may read the mathematics. Challenge yourself,
share a discovery, or stay with a pattern because you like it.

For more, open the [player's manual](https://github.com/blisspixel/numinous/blob/main/docs/PLAYING.md),
[documentation map](https://github.com/blisspixel/numinous/blob/main/docs/README.md),
or [current roadmap](https://github.com/blisspixel/numinous/blob/main/docs/ROADMAP.md#now).
These web guides track current source; [release notes](https://github.com/blisspixel/numinous/releases)
identify changes since an older installed build. The packaged guides remain
available offline.

Welcome. Make this visit your own.
