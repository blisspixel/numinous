# Bundled study fonts

The App study text adapter loads only these three unmodified font files, in
the order below. It does not discover installed fonts or use the operating
system locale. Prose prefers Noto Sans, Japanese script falls back to Noto Sans
JP, and mathematical spans prefer Noto Sans Math. The other bundled faces
remain available when a preferred face lacks a character.

| File | Upstream release | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `noto-sans/NotoSans-Regular.ttf` | Noto Sans 2.015 | 825628 | `f5f552c8c5edb61fe6efb824baf4d4de47b1a8689ab4925ff43f7bd6a4ebece5` |
| `noto-sans-jp/NotoSansJP-Regular.otf` | Noto Sans JP 2.004 | 4533028 | `dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073` |
| `noto-sans-math/NotoSansMath-Regular.ttf` | Noto Sans Math 3.000 | 1015684 | `7283c396e9b22699bb542d9631030dc804a7e5b954f193d8f8f5b5f1162fbc61` |

The fonts total 6,374,340 bytes, about 6.08 MiB. Their original SIL Open Font
License 1.1 texts accompany each file. Font copyright and reserved-name notices
remain intact in those texts and in the original font metadata.

| Original license file | SHA-256 |
| --- | --- |
| `noto-sans/OFL.txt` | `cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a` |
| `noto-sans-jp/LICENSE` | `88f117575237307bdd86a17ef15e21790fc9a662fe4dfb103ca1ca077f0d9982` |
| `noto-sans-math/OFL.txt` | `a1857fdbc5c15797a65c89dbde06ec8158e7bfbe04fad95ea6885bd69388ad82` |

## Exact sources

- Noto Sans: [release 2.015](https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSans-v2.015),
  published 2024-11-20. The file is
  `NotoSans/full/ttf/NotoSans-Regular.ttf` in the official
  [release archive](https://github.com/notofonts/latin-greek-cyrillic/releases/download/NotoSans-v2.015/NotoSans-v2.015.zip).
  The accompanying `noto-sans/OFL.txt` is the archive's original license,
  also available at the [release tag](https://raw.githubusercontent.com/notofonts/latin-greek-cyrillic/NotoSans-v2.015/OFL.txt).
- Noto Sans JP: [Sans 2.004](https://github.com/notofonts/noto-cjk/releases/tag/Sans2.004),
  released 2021-04-28. This is the upstream Japanese subset, not a locally
  modified font. The [exact font source](https://raw.githubusercontent.com/notofonts/noto-cjk/523d033d6cb47f4a80c58a35753646f5c3608a78/Sans/SubsetOTF/JP/NotoSansJP-Regular.otf)
  pins the release's file after its directory reorganization. The original
  [license](https://raw.githubusercontent.com/notofonts/noto-cjk/Sans2.004/LICENSE)
  is `noto-sans-jp/LICENSE`.
- Noto Sans Math: [release 3.000](https://github.com/notofonts/math/releases/tag/NotoSansMath-v3.000),
  published 2024-06-05. The file is
  `NotoSansMath/full/ttf/NotoSansMath-Regular.ttf` in the official
  [release archive](https://github.com/notofonts/math/releases/download/NotoSansMath-v3.000/NotoSansMath-v3.000.zip).
  Its original [license](https://raw.githubusercontent.com/notofonts/math/NotoSansMath-v3.000/OFL.txt)
  is `noto-sans-math/OFL.txt`.

## Coverage and limits

The bundle contains Latin text with Hawaiian kahako and okina, combining
macrons, case-sensitive romanized Klingon, Japanese kana and many kanji, and
Latin mathematical notation. The source tests check representative prose and
equation corpora, including fallback between faces. They do not establish
complete Unicode coverage or coverage of every Japanese name. Unsupported
clusters are reported by the adapter as missing glyphs.

Words wrap first, with an emergency wrap for long equations and URLs. The
adapter checks shaped cluster ownership so that emergency wrapping does not
separate a base character from its combining marks. If a cluster cannot fit
or that wrap would divide it, word wrapping keeps it together. Its measured
advance may then exceed an exceptionally narrow viewport and be clipped.
The reader-width regression uses 324 pixels and includes long English and
Japanese study text, complete linear equations, and an unbroken URL component.

Romanized Klingon uses ordinary Latin letters; this bundle does not promise
pIqaD glyphs. Noto Sans Math supplies mathematical characters, but the adapter
lays out linear text. It does not implement OpenType MATH equation composition
such as stacked fractions or extensible radicals. Character coverage is
separate from translated content and its linguistic and mathematical review.

The shaping dependencies are pinned independently of these font releases.
`cosmic-text` 0.19.0 uses only its `std` and `swash` features here. The App
supplies an empty database populated from these bytes and an explicit locale
and fallback list. `unicode-script` 0.5.8 supplies the fallback's script type;
`unicode-segmentation` 1.13.3 supplies extended grapheme boundaries. Its
[published metadata](https://crates.io/api/v1/crates/unicode-segmentation/1.13.3)
declares Rust 1.85.0; `cosmic-text` establishes the adapter's Rust 1.89 floor.
