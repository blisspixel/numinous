# Sound: The Sonification & Sound-Design Bible

How Numinous *sounds*, and specifically how math *becomes* sound. This is the design bible for the "everything is an instrument" pillar. It complements `MUSIC.md` (which covers the two music engines and the radio stations); this doc covers the grammar of sonification, the synthesis architecture, and the per-room sound design.

**Implementation status, 2026-07-18:** every catalog room ships a structured
motif and deterministic sonification. The App's default room bed is a 128-step
stereo macro-arrangement with a soft sine or triangle lead, a literal authored
theme, two developed forms, a return, breathing consonant anchors, and a silent
loop seam. Eight rhythm and accompaniment families replace one universal form;
one shared register preserves authored intervals, and each motif keeps its own
cadence. Catalog checks bound RMS, sample steps, headroom, DC, seams, and output
at common device rates. Those checks do not establish listening comfort.
The App pre-renders each low-register bed once at 16 kHz, shares the immutable
allocation with the mixer, and linearly resamples it to the device rate. The
catalog source stays below two million interleaved samples, avoiding
device-rate-scaled buffers and repeated copies on room input. Changed sources use a short,
normalized crossfade. Master volume and window-focus state use smoothed gain,
so neither restarts the source; minimizing or switching away fades the App.
Completed crossfade storage is retired by the callback and destroyed by the
control thread, keeping large radio buffers out of real-time destruction and
preventing indefinite retention. Short one-shot storage is also prepared before
the callback mutex, retained when playback finishes, and reclaimed by the
ordinary App update loop. Restoring the App first rejoins the radio's
wall-clock track and offset, then fades audio in. Studio owns formula audio
until it closes, at which point a selected station rejoins live. A shared
master level and mute work in every App mode through keyboard and controller
routes. A persistent badge names the active source, numeric level, and
effective mute, zero-volume, background-silent, or missing-device state. The
audio source is no longer rebuilt from render-loop cadence. Native device rates from 44.1 through 192 kHz
are covered by pitch and duration tests. Built-in radio remains the sole source
while tuned.

Watch Agent is an explicit fourth App audio owner. A strictly accepted native
room selection calls the same core `sound_input` state used by CLI and MCP; a
strictly attested Formula Jam selection calls the same bounded core melody
mapping used by the live Studio. The viewer validates every catalog room sound
against finite 64-second and 512-note caps, renders once at a fixed 16 kHz
source rate, and lets the mixer resample without a device-rate-sized source
copy. Public sequence identity prevents redraws from restarting the phrase.
Scrubbing replaces the source, while invalid, forged, unsupported, and Nim
selections explicitly publish silence. Mute, volume, focus fade, and output
failure use the existing local controls. Closing the viewer restores the room
score or rejoins a valid selected radio at its wall-clock position. None of
these local audio operations sends a command to the MCP player.

Formula Jam curated recipe changes use a request-scoped 600 ms equal-power
source crossfade paired with the Studio's 600 ms curve morph. A deferred source
retains its own requested duration instead of mutating an active fade. Requested
durations must be finite and between 5 ms and 2 seconds. Manual edits continue
to use the default 30 ms source response. Temporary invalid text and literal
spaces reuse the last-good sound target, and an equivalent target preserves its
active playhead while ramping the existing coefficients without a level swell.
These edits interrupt a long recipe fade from its exact current audible mix
instead of waiting behind it. Ownership
changes follow the same bounded path, and a repeated interruption waits behind
at most that short default fade so callback work cannot grow without bound.
Rejected duplicate buffers and superseded pending buffers are destroyed only
after the mixer mutex is released.
These bounds and synchronization checks do not establish native callback timing
or perceptual smoothness.

Times Tables, Galton Board, and Double Pendulum ship continuous
input-sonification through one bounded seam. Their stable room arrangements
remain the bed while a quiet
two-oscillator voice follows accepted mathematical input. Frequency and ratio
targets smooth over 40 milliseconds inside the callback, oscillator phases
persist, invalid targets fade closed, and source playhead continuity is tested.
At integer K, Times Tables uses the exact ratio `k:(k-1)`, so visual closure and
audible consonance are one state. Galton maps the five fixed coins to ordered C
major-pentatonic roots and maps bias strength to the exact larger-to-smaller
Bernoulli odds ratio: 7:3, 3:2, 1:1, 3:2, or 7:3. CLI `sonify` and MCP
`listen_room` accept the same bounded pokes or gestures and render a
deterministic snapshot of each state. Double Pendulum uses one interaction
state for pixels, status, and sound: first-arm drop selects an ordered
minor-pentatonic root, second-arm bend opens a symmetric interval from 1:1 to
3:2, and bounded release speed raises quiet gain from 0.03 toward 0.05. A
completed fling also voices the future separation of the same two integrated
states as one bounded stereo event, described below.
CLI `sonify --layer room-bed` separately exports a deterministic PCM16
projection of the stable 16 kHz stereo App source, while MCP
`listen_room.ambient_bed` exposes its arrangement summary
or complete bounded events and signal metrics. The shared analyzer measures
finite integrity, clipping, peak, RMS, crest, channel balance, DC, correlation,
stereo side-to-mid ratio, adjacent steps, and silence fraction in fixed order.
Those metrics describe the pre-master source only and do not measure comfort,
fatigue, beauty, or musical quality. MCP never returns PCM or local paths.
The Show supplies the same moving phase to picture and voice on every frame and
ignores retained hand input. Entering any modal game fades the parameter voice
instead of leaking room audio across ownership boundaries.

Game of Life adds the first generation event voice. The exact B3/S23 step loop
marks a fixed birth mask, and that mask drives both the visible `@` cells and a
105 ms stereo texture over the stable bed. Every birth contributes to one of
twelve vertical C major-pentatonic rows, its row's horizontal centroid, and the
generation's density. A fixed row reduction bounds synthesis independently of
population. One newest-only tracker compares the planted glider with its exact
four-phase B3/S23 shape and requires an empty one-cell halo. A valid phase adds
one C major-seventh accent panned from the toroidal horizontal position;
collision removes it, and a new launch replaces it. This adds five expected-cell
reads, at most 45 halo reads, and one voice. CLI and MCP expose the same active
pitch rows with birth-row amplitudes weighted by relative birth counts, plus the
optional phase note. Their mono
snapshot cannot represent the App's pan. If elapsed-time catch-up advances
several generations before one frame, only the newest presented generation
sounds, so audio does not replay a stale burst behind the picture. Modal, room,
Studio, and radio transitions
cancel the pending texture. Mono output devices receive both stereo channels
through a bounded downmix. These are structural and signal checks, not evidence
that the texture is pleasant on speakers or headphones.

Galton Board adds one fixed event for every accepted 64-ball wave. The same
deterministic random stream that builds the empirical pile now fills a fixed 17
by 17 newest-wave mass grid. Each destination row reduces its reachable cells
into at most five C major-pentatonic pitch buckets. Square-root amplitude follows
total bucket mass and mass-weighted equal-power pan follows its horizontal
centroid. Above that quiet field, the exact highlighted 16-edge trace still
drives 16 short peg tones and one longer tone at the displayed landing bin. The
fixed half-second renderer performs 1,088 exact path visits, scans at most 152
reachable cells, and adds at most 80 aggregate tones plus 17 highlighted tones
before audio submission. It rejects device rates outside 8 kHz through 192 kHz
and never schedules per-ball work in the callback. Only a newest finite
pointer-down creates the event. Bet motion and release preserve it while the
room score owns audio; Show, modal, Studio, radio, reset, and room transitions
retire it. CLI and MCP continue to expose the deterministic selected-coin
snapshot, not this App-only stereo event. Native callback timing, a growing-pile
pad, and musician-led listening remain open.

Double Pendulum adds one fixed event when a finite newest lift completes a
fling. Seven paired pulses measure the tip gap between the main and shadow RK4
trajectories at 0, 1,000, 2,000, 3,000, 4,000, 5,000, and 6,000 steps. Both states
advance once through those ordered horizons. Each pair starts from the continuous
gesture voice's minor-pentatonic root and bounded momentum gain. The voices
begin in unison at center; growth from the room's one ten-thousandth radian
offset through four orders of tip separation opens the shadow voice toward one
octave and the pair toward 0.85 equal-power stereo width. This logarithmic
mapping makes exponential separation audible without claiming to estimate a
Lyapunov exponent. The fixed 720 ms renderer performs 14 bounded tone additions
before submission, rejects device rates outside 8 kHz through 192 kHz, and
requires a newest finite pointer-up so stale history cannot replay. The App
offers accepted down, move, and lift events through one room-neutral seam:
Galton admits down, Double Pendulum admits lift, and rooms without a discrete
consequence remain silent. Radio changes close an open gesture before room-score
ownership can return. Native callback timing, physical-device behavior,
participant discovery, and musician-led listening remain open.

DSP is implemented locally without `fundsp`. A first shared gain and source
bus is shipped. Sample-accurate event scheduling, per-Era voices, global
tuning, richer spatialization, a soft limiter, and independent room, radio,
and UI volume controls below remain design targets, not shipped claims.

## Philosophy: synesthesia, not sound effects

The north star is **synesthesia**, sight and sound as one perception, in the lineage of Tetsuya Mizuguchi's *Rez* and *Tetris Effect* and the synth-exploration game *FRACT OSC*. Sound in Numinous is never a "sound effect" bolted onto a visual. It is a *second rendering of the same math*, through the ear instead of the eye. When the two channels agree, the insight lands twice and becomes something you feel in your body.

Two research-backed commitments:
- **Musical, not raw.** Sonification only creates awe if it is *music*, not a Geiger counter. We quantize to scales and tune to real ratios so exploration always sounds good (see mapping rules below).
- **Generative sound deepens flow.** Real-time generative music that responds to input measurably increases the player's sense of flow (the state we are engineering for throughout). So the sound is not a loop under the game; it is generated by the game, from the math, live.

## The grammar: how math maps to sound

The shared vocabulary every room draws from. Consistency here is what lets a player's ear "learn the math" across rooms.

| Math quantity | Sound parameter | Why it feels right |
| --- | --- | --- |
| A value / magnitude | **Pitch** (quantized to a scale) | Higher number, higher note; instantly legible. |
| An integer ratio | **A musical interval** | Consonance *is* small-integer ratios. 2:3 sounds like a perfect fifth because it *is* one. |
| Alignment / closure / resonance | **Consonance and resolution** | When the math "lines up," the sound resolves. When it does not, it gently tenses. This is the ear detecting truth. |
| A sequence or stream of events | **Rhythm** | Primes, Collatz steps, births in Life, needle-drops: each event is an onset. |
| Complexity / dimension / energy | **Timbre / harmonic richness** | Simple state, pure tone; complex state, rich spectrum. |
| Position in space | **Stereo/spatial pan and depth** | Left-right and near-far place the event in the field. |
| Rate / density | **Tempo and voice count** | Faster or denser math, busier texture. |

**The rule that keeps it musical:** map continuous math to **quantized** notes in a chosen scale, and derive intervals from the math's actual **integer ratios** (just intonation where the math is exact). The default scale should make "wrong" inputs sound *interesting*, never painful, so a player can wander the parameter space and it always sounds like music.

## Synthesis architecture

- **A shared house voice + master bus.** One coherent synth identity and one master chain (reverb, gentle compression, limiter, global volume, mute) so the whole app sounds like one instrument, the way it looks like one place. Rooms request notes and drones; the bus keeps them coherent.
- **Per-Era voices.** The synth voice swaps with the Visual Era (see `VISUALS.md` and `MUSIC.md`): 4-bit and 8-bit chiptune (pulse/triangle/noise), 16-bit FM, oscilloscope analog (pure sine/saw, the waveform you see), and the modern tuned house synth. One room, every Era, from one mapping.
- **Sample-accurate scheduling.** Sound is scheduled ahead on the audio thread against the **audio clock, which is the app's master timeline**; visuals read from that clock. Nothing musical is fired from the render loop. This is what makes sight and sound feel locked together instead of loosely correlated.
- **Global key and tempo target.** A future shared bus can hold one key and BPM
  so room sonification harmonizes with a station instead of clashing.
- **Ambient by default, expressive on touch.** Untouched, a room breathes a calm generative drone. Touched, it becomes an instrument you are playing.

## Tuning: the math is the tuning system

- **Just intonation from the math's own ratios.** Where a room's math produces exact integer ratios (Lissajous, harmonic series, additive synth), tune those intervals *justly*, so the consonance you hear is the consonance the math describes. This is not a stylistic choice; it is the most honest possible sonification.
- **Microtonality where the math demands it.** Some phenomena are not on the 12-note grid, and should not be forced onto it. The golden-angle detune, for instance, produces slow *beating* between nearly-but-not-quite-aligned tones, and that beating is the audible signature of irrationality. Let it.
- **Scales as mood.** Different rooms and wings can use distinct scales. The
  current motifs do this locally; global-key coordination remains planned.

## Per-room sound design

Extending the one-line sound notes in `ROOMS.md` with technique. The principle in each case: the sound is generated from the *same* state that drives the visual.

- **Times Tables:** a constant D3 root and the ratio `k:(k-1)` turn the dial
  into just intervals. K=2 is 2:1, K=3 is 3:2, K=4 is 4:3, and the earned K=5
  target is 5:4. The App glides this low-level voice over the stable room bed.
- **Double Pendulum:** the shipped input voice follows the exact initial state
  consumed by the integrator. Horizontal hand position selects one of five
  minor-pentatonic drop roots, vertical bend opens a symmetric 1:1 through 3:2
  interval, and release velocity raises quiet gain without restarting the bed.
  A completed fling adds seven paired pulses from the RK4 model's future twin
  gaps, opening unison toward one octave and center toward wide stereo as the simulated tips
  separate. The continuous layer sonifies the cause of the storm; the bounded
  event lets the same consequence unfold audibly without scheduling the long
  trace in the callback.
- **Chaos Game:** each corner is a note of a chord; the accumulating dot-density becomes a shimmering granular pad, a cloud of tiny grains thickening as the fractal fills.
- **Game of Life:** the shipped first event layer reduces each exact generation
  into twelve fixed C major-pentatonic pitch rows. Every birth contributes to
  its row's weight and horizontal stereo centroid; total activity adds bounded
  harmonic color. The same birth mask marks the visible new cells. A second
  fixed voice follows the newest glider's four exact isolated phases as
  C major-seventh accents and stops on collision. Literal independently timed
  onsets per cell and a sustained colony pad remain later sensory-bus work.
- **Cellular Automata:** each generation's row is read left-to-right as a rhythm; complex rules (30, 110) produce complex, evolving beats, simple rules produce steady pulses.
- **Fourier Epicycles:** each circle is a pure sine at its frequency; your drawing literally *is* the chord of its Fourier components. You hear the transform of your own doodle.
- **Lissajous / Harmonograph:** the two frequencies are the two audible tones. A stable figure is a consonant interval you *see and hear at once*; an off-ratio figure tumbles visually and beats audibly.
- **Pendulum Wave:** each pendulum plinks at the bottom of its swing, producing a self-generating polyrhythm that phases out of and back into unison, Steve-Reich-in-math.
- **Mandelbrot Dive:** iteration-count-to-escape maps to pitch across the field; zooming sweeps a drone through octaves; the fractal boundary shimmers with high harmonics where iteration counts scatter.
- **Prime Spirals:** scanning the spiral, each prime is an onset, an irregular-but-clearly-not-random rhythm you can *hear* has structure. Euclidean-rhythm relatives (see `MUSIC.md`) tie it to danceable time.
- **Collatz:** up-steps (3n+1) raise pitch, down-steps (n/2) lower it, so each starting number plays its own unpredictable little melody that always resolves home to 1.
- **Golden Angle:** each seed plinks; the golden angle yields an evenly-spaced, satisfying rhythm, and detuning the angle makes the rhythm stumble and the tones beat.
- **Galton Board:** the shipped first layer maps the selected five coins to
  ordered C major-pentatonic roots and exact symmetric bias-odds intervals.
  The shipped second layer follows the exact highlighted newest ball through
  all sixteen peg decisions as a panned half-second tick sequence and landing
  tone. The shipped third layer reduces every exact path in the newest 64-ball
  wave into a quiet row-pitch mass texture with mass-weighted stereo position.
  A soft growing-pile pad remains planned.
- **Buffon's Needle:** each needle ticks, every line-crossing rings a bell, and the ensemble's pitch bends toward "in tune" as the running estimate converges on pi.
- **4D Objects:** the fourth-axis rotation drives a Shepard tone, a pitch that seems to rise forever, an *audible* impossible direction to match the visual one.
- **Sizes of Infinity:** countable sets play as a steady, listable pulse; the uncountable diagonal is a tone that slips endlessly between the notes and never lands.

## Spatialization

- **Stereo as default, spatial where it pays.** Position in the field maps to pan; depth (near/far) to level and filtering. Rooms with real depth (hyperbolic space, 4D) can use binaural/HRTF on headphones so the warp has audible space.
- **The field is an ensemble.** Dense rooms need bounded spatial reductions so
  activity becomes texture instead of an unbounded pile of clicks. Life ships
  twelve pitch-row centroids plus one optional tracked-glider voice. Galton
  reduces the exact newest 64-ball wave into at most 80 mass-first row-pitch
  tones, then layers its exact highlighted path as sixteen panned peg events
  plus its landing. Finer onset fields for primes and Life remain planned.

## Interaction & UI sound

- **Touch has a voice, partially built.** Times Tables, Galton Board, and Double
  Pendulum ship persistent input-audio paths through the smoothed parameter
  voice. Life ships one bounded generation-event texture with an exact
  tracked-glider phase accent, and Galton ships one bounded all-ball wave
  texture with an exact newest-ball peg sequence. Tuned event layers and
  equivalent mathematical voices in other rooms remain planned.
- **Transitions are washes.** Room-to-room dissolves carry a reverb wash through black, matching the visual cross-dissolve (see `VISUALS.md`).
- **Reveal has a resolution.** Summoning a Revelation card lands on a small, satisfying harmonic resolution, the sonic version of the floor tilting.

## Accessibility & silence

- **Beautiful in silence.** A prominent, graceful mute. The visuals must fully carry the experience with the sound off (the library, the office, the sleeping-roommate 2am). Muting is never a downgrade.
- **Full control.** Independent volumes for room sonification, the radio (Engine B), and UI; a master; and a hard mute.
- **No painful surprises.** No sudden loud onsets, no harsh strobing-audio; loudness is managed on the master bus. Reduce-motion never silences the room, and mute never freezes the visuals.

## Open questions
1. How hard to quantize room sonification to the global radio key before a room stops sounding like *itself* (shared with `MUSIC.md`).
2. DSP budget: how many simultaneous voices and grains the future custom DSP
   sustains per platform under render load.
3. Default scale/tuning per wing: one house scale for coherence vs. per-wing scales for identity.
4. Binaural on by default for headphones, or opt-in (some players dislike HRTF coloration).
