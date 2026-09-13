# Named pitches

`note("c e g")` writes named MIDI pitches as a graph over steps. The sample
at `x` is the MIDI number of step `floor(x) rem n`. A rest is `.`. The
graph is the melody; the piano roll is the same notes that sing.

These two small creations let you see a triad, then hear it climb to the
octave. There is no score or required order.

From a packaged install, open a capsule by id: `major-triad`,
`octave-climb`. `plot_expression` with `list_experiments` true and
`family` `notes` lists them. `open_creation` accepts those ids.

| Creation | Try this |
| --- | --- |
| [A major triad](major-triad.num) | Three named pitches. Which intervals sit between them, and what would a fourth pitch do? |
| [An octave climb](octave-climb.num) | The last pitch is an octave above the first. Does the climb feel like a return? |

The trial does not gate play. Type `note("c e g")` to make your own, or
`note("c . e g")` to rest between pitches. Letters `c` through `b`, optional
`#` or `s` sharp and `b` flat, optional octave `0` through `9`, default
octave 4. At most 64 pitches. This is the same formula that draws and
sings, not a second document.

<details>
<summary>The mathematics</summary>

C4 is MIDI 60, A4 is 69, and frequency is `440 * 2^((midi - 69) / 12)`.
The graph's `y` is that MIDI number, so a rest is undefined rather than
the root. Overlay already layers a named melody with a 0/1 rhythm. Named
pitches ignore the stored scale map: the letters are the pitches.

</details>
