# Euclidean rhythms

A Euclidean rhythm places hits as evenly as possible among a fixed number of
steps. The graph is 1 on a hit and 0 on a rest.

These two small creations let you see tresillo, then hear it against a denser
pattern on the same grid. There is no score or required order.

From a packaged install, open a capsule by id: `tresillo`,
`three-against-five`. `plot_expression` with `list_experiments` true and
`family` `euclidean` lists them. `open_creation` accepts those ids.

| Creation | Try this |
| --- | --- |
| [Tresillo](tresillo.num) | Three hits among eight steps. Are they equally spaced, or only as even as eight allows? |
| [Three against five](three-against-five.num) | Two patterns share eight steps. Where do three hits and five hits land together? |

The trial does not gate play. Type `euclid(3,8)` to make your own, or
`euclid(3,8) & euclid(5,8)` to overlay two. An integer 0/1 window also
reports pattern text: tresillo is `x..x..x.`. The step count is at most 64.
Capsules stay on the versions they already used: a titled graph is still
version 2, and an overlay is still version 7.

<details>
<summary>The mathematics</summary>

E(k, n) places k onsets as evenly as possible among n steps. Step i is
`floor(x) rem n`. A hit is `(i * k) rem n < k`, with k clamped into
`[0, n]`. Tresillo is E(3, 8). A request with n below 1 or above 64 is
undefined. Overlay already layers two such claims on one window.

</details>
