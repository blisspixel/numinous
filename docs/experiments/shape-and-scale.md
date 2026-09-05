# Shape and scale

Make a circle you can stretch. Keep a version whose proportions you like, give
it a name, and pass along both the picture and its recipe.

Open [Circle to ellipse](circle-to-ellipse.num) in the App:

```text
numinous-app docs/experiments/circle-to-ellipse.num
```

It opens paused. Enter starts its melody. Tap Up or Down to change `a` by 0.25;
Home restores `a = 1`. A parameter edit also starts its melody. Try
`a = 4`, twelve Up taps from 1, or explore other positive quarter-step settings.

F4 opens the naming step; Enter saves your capsule, postcard, melody MIDI, and
share link. An edited share records the opened creation as its parent and
keeps its time window. Someone else can reopen your exact formula and parameter,
then make another branch. No score or explanation is required to keep it.

Compare [Uniform circle](uniform-circle.num) when you feel like another puzzle:

| Recipe | Something to notice |
| --- | --- |
| `x(t)=a*cos(t); y(t)=sin(t)` | What changes between `a = 1` and `a = 4`? How wide is the shape compared with its height? |
| `x(t)=a*cos(t); y(t)=a*sin(t)` | Change `a` from 1 to 4 again. Can the mathematical size change while the fitted picture looks the same? |

Both capsules save one full turn, from `t = 0` to the floating-point value of
`2*pi`, starting at `a = 1`. The CLI can draw either file with
`numinous open-studio`, followed by its path. MCP `open_creation` accepts the
same capsule text in its `capsule` field.

<details>
<summary>What the view preserves, and what it hides</summary>

The first recipe describes a unit circle at `a = 1`. At `a = 4`, it describes
an ellipse with horizontal semiaxis 4 and vertical semiaxis 1. Its width is
four times its height. Equal numerical units on the two displayed axes preserve
that proportion, with finite sampling and pixel rounding.

The second recipe describes a circle of radius `a` for positive `a`. Its radius
quadruples from 1 to 4, but Studio fits each complete sampled path into the view.
The circles therefore look the same apart from sampling and rounding. Equal
units within one view preserve shape; they do not provide an absolute scale
shared by separately fitted views. Use the formula and saved value to compare
their mathematical sizes.

Studio sings sampled `y(t)` values. Changing `a` in the first recipe changes
only x, so its numerical melody input stays unchanged while the picture
stretches.

For a deeper construction, conjecture how the enclosed area changes in each
family. For positive `a`, the transformations of the unit disk are
`(u,v) -> (a*u,v)` and `(u,v) -> (a*u,a*v)`. Can you justify the area factors
from those maps? The sampled picture is a clue, not a proof. Give someone a new
recipe and a question to explore with it if you find a direction worth following.

</details>
