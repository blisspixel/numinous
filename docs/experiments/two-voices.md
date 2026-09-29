# Two voices

A path is two oscillators drawn as one. An overlay draws those oscillators
as two graphs over the same time window. Every graph sings in WAV; MIDI
stays the first curve.

These two small creations let you see the parts of Returning home. There is
no score or required order. The path capsules remain `full-return` and
`almost-home`.

From a packaged install, open a capsule by id: `closing-voices`,
`shorter-window`, `wandering-voices`. `plot_expression` with
`list_experiments` true and `family` `two-voices` lists them.
`open_creation` accepts those ids. In the App, PageDown after Another
ratio opens Closing voices, then A shorter window, then Wandering voices.
PageUp returns. Opening one reports `closure` with kind `voices`: the
ideal cycles in the window, and a common period when the model has one.
That count is cycles, not peaks. A picture is not the proof. The same
frequencies sound as sustained tones: frequency 1 is 110 Hz, and the
window counts are not the pitches. The sung melody stays the sampled curve.

| Creation | Try this |
| --- | --- |
| [Closing voices](closing-voices.num) | These are the two oscillators of A full return. How many cycles does each complete in this window? |
| [A shorter window](shorter-window.num) | Same oscillators as Closing voices, on a shorter window. Do these cycle counts name the period? |
| [Wandering voices](wandering-voices.num) | Compare with Closing voices. What would a common period require of both cycle counts? |

The trial does not gate play. Type `cos(2*pi*x) & sin(2*pi*(17/12)*x)` to
make your own. A picture of two graphs is not a proof of period.

<details>
<summary>The mathematics</summary>

For `x(t) = cos(2*pi*f*t)` and `y(t) = sin(2*pi*g*t)`, a positive period
`T` must make both `f*T` and `g*T` integers. Closing voices uses `f = 1`
and `g = 17/12` over `[0, 12]`, so the first graph completes 12 cycles and
the second completes 17. That window is one least period. A shorter window
uses the same oscillators over `[0, 1]`: the counts are `1` and `17/12`,
and the least period is still `12`. Wandering voices replaces `17/12` with
`sqrt(2)`. In the ideal equations there is no positive common period. The
second count on `[0, 12]` is `12*sqrt(2)`, which is not 17. Overlay draws
the two claims separately; Returning home draws the path they make together.

</details>
