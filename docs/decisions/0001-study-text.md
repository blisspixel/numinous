# Decision 1: shared study content and bundled text rendering

Date: 2026-09-05. Status: accepted for the study reader.

## Context

Room play should be available immediately. A visitor should also be able to
request an explanation or its mathematical foundations without earning access.
The existing uppercase bitmap font cannot faithfully show Japanese prose,
case-sensitive notation, or combining characters. Rendering translations as
images would lose reflow, text identity, and parity with CLI and MCP readers.

## Decision

The dependency-free core owns typed study blocks, mathematical notation,
references, depth selection, stable block IDs, and explicit locale resolution.
It does not read player progress or write a reading history. App, CLI, and MCP
consume that source. CLI JSON and MCP structured content share one presentation
module in `faces/shared/study_json.rs`; neither face depends on another face.
The workspace remains the build and distribution unit.

The App renders this document through `cosmic-text` 0.19.0 with only `std` and
`swash` features, plus pinned `unicode-script` and `unicode-segmentation`.
Three unmodified Noto font files are embedded in the executable. Font selection
uses an explicit locale and an explicit database containing only those files.
It never discovers system fonts. The source, release versions, byte counts,
checksums, and original licenses are recorded in
[the font inventory](../../assets/fonts/README.md).

This adds about 6.08 MiB of font data and raises the workspace compiler floor
to Rust 1.89. The pinned development toolchain remains separate from that
minimum. The font database behind `cosmic-text` 0.19 still depends on the
unmaintained `ttf-parser`, so the advisory gates carry one documented
exception until a `cosmic-text` release takes `fontdb` 0.24 or later. Native packages carry the font notices and original licenses inside
their verified file manifest. The Cargo SPDX inventory does not yet enumerate
the embedded fonts as separate components.

## Consequences and limits

The study reader preserves UTF-8 source and mathematical case. Extended
grapheme boundaries anchor the reading position across changes in width.
Math spans select a mathematical font; equations remain linear text, without
stacked fractions or an OpenType MATH layout engine. Prose, equations, and long
URLs wrap. An indivisible cluster that cannot fit is retained and clipped,
with unsupported clusters exposed through the renderer's missing-glyph data.

Requests have bounded source, span, glyph, width, and font-size limits. The
adapter retains at most four layouts and bounds its raster glyph cache to
4,096 images and 8 MiB. An immutable retained layout belongs to its originating
renderer. Input capture and the room tick barrier belong to the App runtime,
where opening, navigating, and closing the reader preserve accepted room state.
Only a deliberate language change writes a preference.

This decision establishes a reading surface. It does not translate the whole
App, add Unicode editing or IME support to Studio, promise complete character
coverage, or establish linguistic review of every supported language. The
Japanese Lissajous treatment is labeled a translation draft. Locale syntax
admission is distinct from translation availability. See [Study](../STUDY.md)
for the public contract and [Rosetta](../ROSETTA.md) for the remaining work.

## Alternatives considered

- Expanding the existing bitmap atlas would retain its visual style but would
  make shaping, grapheme ownership, and large script coverage local maintenance.
- System fonts would reduce the bundled data but make character availability,
  metrics, and screenshot behavior depend on the player's installation.
- A browser-based reader would introduce another runtime and document system
  into the native App. The current linear treatment does not require one.

The chosen adapter keeps its costs and limitations visible. Source tests and
native render plates cover the actual content; player usability and broader
translation review require separate evidence.
