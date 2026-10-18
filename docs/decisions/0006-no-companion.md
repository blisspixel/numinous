# No companion

Date: 2026-10-18. Status: accepted.

## Context

Issue #412 proposed a silent companion in the Cabinet and on the journey
screen: γ, after the player has visited the Harmonic Series. The other
options were the most recently met Constant, which needs a Constants
collection that is not built, or no companion at all.

`docs/LORE.md` already has the Constants as inhabitants. They surface through
the radio, the codex, and the revelation cards. The prime directive is that
the surface never mentions the lore, and that the lore never stands in front
of the toy.

## Decision

No companion. The Cabinet, the journey screen, rooms, Watch, the Show, and
MCP stay free of a character glyph. γ remains a Constant in the lore. It is
not drawn beside the math.

Reduced motion, `NO_COLOR`, and pipes are unchanged, because nothing new is
drawn.

## Consequences

There is no Settings switch and no `NUMINOUS_NO_COMPANION` variable. A later
companion would be a new decision, not a revival of this proposal.
