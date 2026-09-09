# Overlay

Two claims can share one window. An overlay program draws several graphs
together. Every graph sings in WAV; MIDI stays the first curve.

These two small creations let you compare the parts with their sum. There
is no score or required order.

From a packaged install, open a capsule by id: `the-parts`, `the-sum`.
`plot_expression` with `list_experiments` true and `family` `overlay`
lists them. `open_creation` accepts those ids.

| Creation | Try this |
| --- | --- |
| [The parts](the-parts.num) | Two curves share one window. Which one oscillates, and which one is shifted? |
| [The sum](the-sum.num) | The third curve is their sum. Where does it sit when they cancel? |

The trial does not gate play. Capsules write `NUMINOUS_STUDIO 7` only when
more than one graph is present. Type `sin(x) & cos(x)` to make your own.

<details>
<summary>The mathematics</summary>

A graph is one function of x. An overlay is several such functions, sampled
over the same interval, fitted to one shared vertical range. Distinct marks
name the curves in source order: `#`, `*`, `+`, `o`. WAV mixes every
expression; MIDI keeps the first. A sum is a third claim, not a skin of the
overlay.

</details>
