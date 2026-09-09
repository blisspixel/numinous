# Two voices

A path is two oscillators drawn as one. An overlay draws those oscillators
as two graphs over the same time window. Every graph sings in WAV; MIDI
stays the first curve.

These two small creations let you see the parts of Returning home. There is
no score or required order. The path capsules remain `full-return` and
`almost-home`.

From a packaged install, open a capsule by id: `closing-voices`,
`wandering-voices`. `plot_expression` with `list_experiments` true and
`family` `two-voices` lists them. `open_creation` accepts those ids.
In the App, PageDown after Another ratio opens Closing voices; PageUp
returns.

| Creation | Try this |
| --- | --- |
| [Closing voices](closing-voices.num) | These are the two oscillators of A full return. Count the peaks. How many does each make in this window? |
| [Wandering voices](wandering-voices.num) | Compare with Closing voices. What would a common period require of both counts? |

The trial does not gate play. Type `cos(2*pi*x) & sin(2*pi*(17/12)*x)` to
make your own. A picture of two graphs is not a proof of period.

<details>
<summary>The mathematics</summary>

For `x(t) = cos(2*pi*f*t)` and `y(t) = sin(2*pi*g*t)`, a positive period
`T` must make both `f*T` and `g*T` integers. Closing voices uses `f = 1`
and `g = 17/12` over `[0, 12]`, so the first graph completes 12 cycles and
the second completes 17. Wandering voices replaces `17/12` with `sqrt(2)`.
In the ideal equations there is no positive common period. Overlay draws
the two claims separately; Returning home draws the path they make together.

</details>
