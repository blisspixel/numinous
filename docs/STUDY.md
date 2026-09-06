# Study a room

Explore, watch, or create for as long as you like. When you want an explanation,
open study directly. Reading requires no visit, level, wager, or consolidation.
It awards no reward, records no reading completion, and does not change Journey.

## Choose your depth

| Depth | Content |
| --- | --- |
| `explanation` | A short explanation, or the pilot's suggested experiment and intuition. This is the default. |
| `notes` | Existing room explanations, advanced notes, and citations where present. |
| `mathematics` | An authored treatment with assumptions, derivations, examples, limits, and references. |

Two rooms currently have an authored Mathematics treatment. Lissajous covers
full oscillator state, periods and position returns, phase, torus motion,
occupation measure, recurrence, and the limits of the drawing and sound. Times
Tables derives the envelope of its chords in closed form, identifies it as an
epicycloid, and proves it has one cusp fewer than the multiplier, which is the
shape a player watches bloom on the dial. A citation or an advanced note is not
silently presented as that treatment.

You never have to hunt for it. Explanation and notes exist for every room, so
they are always available. Because an authored treatment is scarce, every study
response carries the rooms that have one, and asking a room that lacks one names
them in the refusal. That pointer is coverage, not permission: reading any depth
that exists has no visit, level, or progress requirement, so a named room opens
immediately.

The Times Tables treatment is English only. The Lissajous Japanese draft was
independently reviewed and this text was not, so a Japanese request for Times
Tables resolves to English and reports `translation_unavailable` rather than
offering an unchecked translation.

Lissajous has English content and a Japanese `reviewed_draft`. Its original
English room explanation, deep cuts, and catalog citation remain in Notes and
are labeled English when Japanese was requested. Other rooms reuse their
existing English explanation and notes. Translation coverage is per block;
the Japanese draft is not complete App localization or native-speaker validation.

## In the App

From room play, press **E** or **?**, or choose **EXPLAIN** in the Cabinet.
Select any depth immediately. An unwritten depth displays an availability
message and leaves the other depths accessible.

| Action | Keyboard or mouse |
| --- | --- |
| Scroll | Up/Down, mouse wheel, Page Up/Page Down, Home/End |
| Change depth | Left/Right or click a depth tab |
| Open Mathematics directly | Enter |
| Change English/Japanese preference | L or the language button |
| Return | Esc, E, ?, or Back |

With default controller bindings, Select opens or closes study, the D-pad
scrolls vertically and changes depth horizontally, and the room-navigation
shoulder buttons move by pages. The primary button opens Mathematics; Back
returns. The reader's controller labels follow the loaded bindings.

The reader preserves case and wraps prose, linear equations, and reference
URLs. Its buttons stay fixed while the body scrolls. Each depth retains its
position while the reader is open; resizing anchors the view to the text.
Changing language keeps the selected depth and starts that text at the top.
Closing returns to the room or Cabinet page that opened it, with room tuning,
phase, and accepted input history retained. The language choice is saved as a
preference, separately from Journey.

The seven staged room experiments are a separate choice: press **U** in room
play or select **EXPERIMENT** in the Cabinet where offered. The chosen path
can accept a prediction or sufficient observations and offer an earned
connection. **Enter** advances that connection when offered; **U** or **Esc**
returns to free play. Existing calls and earned progress remain. **E** still
opens study without completing that path. Other rooms retain their existing
optional prediction controls.

## From the CLI or MCP

Name a catalog room explicitly. CLI and MCP requests default to English and
Explanation independently of the App's saved language preference.

```sh
numinous study lissajous
numinous study lissajous --locale ja --depth mathematics
numinous study lissajous --block lissajous.recurrence --json
numinous study times-tables --locale haw --depth notes
```

For MCP, call `study_room` with arguments such as:

```json
{"room":"lissajous","locale":"ja-JP","depth":"mathematics"}
```

To open a single block directly, replace `depth` with
`"block":"lissajous.recurrence"`. Do not supply both. The response lists
available depths and stable block IDs, so no earlier reading is necessary.
Malformed parameters, unknown rooms, and missing blocks or depths return an
error. For example, requesting Mathematics for Times Tables does not return
Notes in its place.

CLI `--json` and MCP `structuredContent` use `numinous.room-study`, schema
version 1. They share `selection`, `locale`, `contentLocales`,
`availableDepths`, `availableBlocks`, `authoredDepthRooms`, and selected
`blocks`. `authoredDepthRooms` is catalog-wide rather than about the requested
room, and lists only depths whose coverage is a real subset, so explanation and
notes are absent by design. It was added to schema version 1 additively: a
reader that ignores it sees exactly the document it saw before. Paragraph runs
distinguish text from mathematical notation; equations and references are
separate parts. References retain their source ID, title, URL, and description.
Plain text preserves the same content and reports availability and fallback.
Transport keys and metadata labels remain English.
Study calls stay outside the Shared Play broadcast; reading and language
selection remain with the participant making the request.

The existing CLI `reveal` and MCP `reveal_room` retain their progression rules.
Use `study` or `study_room` for unrestricted reading. Existing experimental
collectors and scoring protocols are separate from these read-only requests.

## Language requests and fallback

The locale grammar is deliberately bounded, not a complete BCP 47 validator:

- At most 63 ASCII bytes in total.
- A primary language of 2 to 8 ASCII letters.
- Up to seven additional subtags, separated by hyphens, each containing 2 to
  8 ASCII letters or digits.
- Requests are case-insensitive and stored in lowercase. Empty values,
  whitespace, underscores, and singleton extension or private-use syntax refuse.

Examples include `en`, `ja-JP`, `zh-Hant-TW`, `haw`, and `tlh`. Accepting a tag
does not establish that the language exists in a registry or has a translation.
Lookup uses the available primary language, then English. Document and block
metadata each report `requested`, `resolved`, and `fallback`:

| Request and content | Resolved | Fallback |
| --- | --- | --- |
| `ja`, Lissajous Mathematics | `ja` | `null` |
| `ja-JP`, Lissajous Mathematics | `ja` | `parent_language` |
| `ja`, Lissajous original Notes | `en` | `translation_unavailable` |
| `haw` or `tlh`, current room content | `en` | `translation_unavailable` |

Depth names are exactly `explanation`, `notes`, and `mathematics`. Block IDs
are at most 128 ASCII bytes: lowercase letters and digits in nonempty
components separated by dots or hyphens, with at least one dot. Use the IDs
returned by the room; valid syntax alone does not mean a block exists.

## What remains

The six rooms listed in [Rosetta](ROSETTA.md#values-independent-of-labels)
provide typed numerical grading channels independent of English labels. Other
rooms still use the compatibility parser; full grading localization is unfinished.
Reading support does not add Unicode naming or IME editing to Studio, translate
the full App shell, or establish complete Unicode coverage. Native-speaker
review, more room treatments, and a reviewed curriculum remain work to do.

Opening a page, receiving a correct answer, or preserving a session record does
not establish understanding, enjoyment, or lived memory. Human and agent
learning claims need their own transfer and retention evidence. Numinous does
not determine whether an agent is conscious or whether a report of enjoyment
corresponds to subjective experience. The aim is voluntary, accessible play
and serious explanation for the visitors who can use it; see
[Pedagogy](PEDAGOGY.md) for the evidence boundaries.
