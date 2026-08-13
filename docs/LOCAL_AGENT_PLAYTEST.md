# Local agent playtest

`scripts/local-agent-playtest.py` lets an already-installed local Ollama model
enter Numinous through the shipped MCP face while a person watches the visible
exchange in the terminal. It is a player-first exploratory session, not a tool
coverage benchmark, a release gate, or a consciousness test.

The distinction matters. The deterministic agent cohorts in CI prove protocol
and game invariants. This lane asks a real model to choose what to do, permits
it to leave, records only actions and words it makes visible, and reports what
actually happened. A model saying that it used a tool is not evidence that the
tool ran.

## Quick start

Install [Ollama](https://docs.ollama.com/) and install at least one model that
declares native tool support. The harness never downloads a model for you.
Then run:

```text
python scripts/local-agent-playtest.py
```

Automatic selection tries installed tool-capable models from smallest to
largest. The default visit is intentionally modest: three model turns, four
Numinous calls, an 8,192-token context, and the first-contact palette. Select a
known local model for a deeper visit:

```text
python scripts/local-agent-playtest.py \
  --model devstral-small-2:24b \
  --turns 6 \
  --tool-calls 8 \
  --context 16384
```

On Windows PowerShell, put the command on one line or replace each `\` with a
PowerShell backtick. Larger models and larger tool schemas can be very slow on
CPU-only hardware. Start with the default, then expand only if the first run is
useful.

The live output shows visible player words and each witnessed tool call. Full
tool results are not printed by default. To retain a private local transcript,
choose a new JSON path under the gitignored `logs/` directory:

```text
python scripts/local-agent-playtest.py --output logs/my-visit.json
```

Transcript writing is opt-in, refuses redirected paths and existing files, and
uses an exclusively created final path. The disposable Numinous profile is
removed after the visit whether the model stays or leaves.

## Cost and network boundary

The harness has a hard local-only contract:

- the endpoint must be a literal loopback IP over plain HTTP with an explicit
  port;
- proxy settings are ignored;
- cloud model selectors and models without local weight payloads are rejected;
- only models already listed by the local Ollama server are eligible;
- API keys, remote fallbacks, and implicit downloads are absent.

Under those enforced boundaries, the reported inference cost is USD 0.00. The
harness talks to Ollama's [local API](https://docs.ollama.com/api/introduction)
at `127.0.0.1` and uses its documented
[tool-calling loop](https://docs.ollama.com/capabilities/tool-calling). It does
not use Ollama cloud models.

## What the report means

The summary separates successful calls, tool errors, room identifiers,
model-visible output, token counts, and wall time. If visible prose names an
available tool without a corresponding call, the report records an
`unexecutedToolClaims` finding. On the first such response, the harness explains
that no action happened and offers one choice: make a real call or leave.
If a later inference times out or fails, witnessed actions remain in a partial
report and an explicitly requested transcript. The process exits nonzero so an
automation caller still sees the model failure.

Private model reasoning is neither printed nor written to the optional
transcript. The report can support product debugging and qualitative review,
but it is synthetic model evidence. It cannot establish enjoyment,
consciousness, welfare, or the experience of any other player. Treating a
digital player with agency, privacy, bounded exposure, and an honest exit does
not require resolving those questions first.

## Why this is not in CI

CI runs `scripts/test-local-agent-playtest.py`, which exercises endpoint,
model, palette, privacy, transcript, error, and tool-loop contracts without an
inference service. It never runs a model. Model output, speed, and installed
weights vary by machine, and converting free play into a required benchmark
would distort the experience this lane is meant to observe.
