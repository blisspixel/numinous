# Returning home

Can a moving point come home without repeating its motion?

These three small creations let you investigate that question in the current
Studio. Change a formula, compare the result, and keep a version you like.
There is no score or required order. The explanation below can wait, or you can
read it first and use the creations to test it.

From a repository checkout, open a capsule with the App:

```text
numinous-app docs/experiments/full-return.num
```

It opens paused with the saved formula, time window, and parameter. Enter starts
the melody. Editing begins a remix while preserving that window and parameter.
F4 saves a portable share; F6 changes the pitch map. The CLI can draw the same
file with `numinous open-studio docs/experiments/full-return.num`. The MCP
`open_creation` tool accepts the capsule's text in its `capsule` field for the
same experiment.

## Three paths

| Creation | Try this |
| --- | --- |
| [A full return](full-return.num) | Count how many oscillations each coordinate makes before the whole motion repeats. |
| [Almost home](almost-home.num) | Compare its formula with the first creation. What changed, and what would count as evidence of repetition? |
| [Same place, another direction](same-place.num) | Evaluate the point at `t = 0` and `t = 0.5`. Would you expect its next move to be the same? |

Almost home is a fork of A full return. Its capsule retains the exact parent
link, so you can trace the change or give someone else a different branch.
Keep a note of a question you want to revisit if that helps. Saving a creation
does not automatically store your interpretation or certify an insight.

In the Lissajous room, try an equal-frequency circle and then change one
frequency. The room's two sound frequencies follow the displayed ratio.
Studio has a different voice: it maps sampled `y(t)` values to pitches. Hearing
the capsule is not hearing the two coordinate oscillators as a chord, and a
finite melody cannot establish the full period.

<details>
<summary>The mathematics, including the trap</summary>

For `x(t) = cos(2*pi*f*t)` and `y(t) = sin(2*pi*g*t)`, a positive period
`T` must make both `f*T` and `g*T` integers. This repeats the entire motion,
including direction, from every starting time. It is stronger than checking
one pair of endpoints.

A full return has `f = 1` and `g = 17/12`. Its least positive period is `12`:
the x oscillator completes 12 cycles and the y oscillator completes 17.
Neither has leftover phase.

Almost home replaces `17/12` with `sqrt(2)`. In the ideal equations there is no
positive common period, because `sqrt(2)` is irrational. At `t = 12`, x has
returned while y has completed about 16.9706 cycles, close to 17. That makes a
near return, not a period. Floating-point arithmetic and a finite plotted path
approximate these equations; a picture cannot prove irrationality or eternal
nonrepetition.

Same place uses `f = 2` and `g = 3`. At both `t = 0` and `t = 0.5` its
position is `(1, 0)`. But its y velocity changes from `+6*pi` to `-6*pi`.
It leaves the same place in another direction. The full motion repeats at
`T = 1`.

This distinction is useful beyond a curve: a system's position can hide the
state that determines what happens next. For another ratio, try `g = 8/5`
with `f = 1`. Predict the full period, then build and compare your own capsule.
You can also change a phase without changing either frequency. Which part of
the picture changes, and which part of the period argument survives?

These cases follow the standard periodic and quasiperiodic oscillator model.
For a further mathematical treatment, see
[MIT's Lissajous exercises, section 3](https://math.mit.edu/classes/18.353J/PSetAnswers/AnswerPSet_2024_07.pdf).
This guide is a playable contrast, not evidence that a participant learned or
enjoyed it. The proposed in-app quest remains in [PROGRESSION.md](../PROGRESSION.md).

</details>
