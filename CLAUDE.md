# Claude Code guide

The full agent guide for this repository is [`AGENTS.md`](AGENTS.md). Read it.
This file exists so Claude Code picks up the same rules; it does not repeat them,
it points at the one source of truth, with the three non-negotiables restated
because they are the easiest to get wrong.

## The three hard rules (from `AGENTS.md`, repeated for emphasis)

They apply to everything you produce, including commit messages and PR
descriptions, not only files:

1. **No AI or tool attribution, anywhere.** Nothing you produce is signed by a
   tool: no "by Codex", no "by Claude", no co-author trailers, no session links,
   no "generated with" note. The work stands on its own.
2. **No em-dashes or en-dashes** (U+2014, U+2013). Use a comma, a colon, or a
   rewrite.
3. **No emojis.**

Everything else, the quality bar, the pre-commit hook, where files live, is in
[`AGENTS.md`](AGENTS.md) and `docs/ENGINEERING.md`.
