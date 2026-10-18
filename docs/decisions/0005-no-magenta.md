# No magenta

Date: 2026-10-18. Status: accepted.

## Context

The house palette rule is no magenta. It reads as neon beside the lit-from-within
stage in `docs/DESIGN.md`. Three places used it: the spectral ink for `'@'`,
the arcade Drifter's bright magenta SGR 95, and room accents in the magenta
and orchid band (HSV hue 285 to 340 degrees, saturation over 0.35).

Decisions entry 12 in `docs/ROADMAP.md` already records which spectral pairs a
dichromat cannot separate. Replacing the `'@'` ink is that ruling's concrete
choice for the ink, and it is measured again after the change.

## Decision

Option (c) for room accents. The accents already in the catalog stay. New
rooms fail `in_magenta_band`. The grandfather list is the test
`the_magenta_band_rejects_new_hues_and_keeps_the_catalog_list`. Retuning all
of those accents would repaint the catalog and force a flagship rebaseline
for rooms whose accent, not their spectral ink, changed. That rebaseline
belongs with the Sensory Lift, not with this ruling.

The spectral ink for `'@'` is no longer magenta. It is teal `[7, 118, 140]`.
The dichromacy audit picked it: the list of folded spectral pairs falls from
sixteen to five, no new room joins that list, the color sits in the accent
lightness band, and it stays outside the magenta band and apart from the
green, amber, and violet inks. Okabe and Ito sky blue, and the brighter blues
near it, added rooms to the list. Orange and vermillion sat too close to the
amber ink. Green, amber, and violet stay. The arcade Drifter stays the letter
`d` and is drawn in bright blue, SGR 94. SGR 35 and 95 are not used.

The fixed mark inks sit outside the band. Every chrome token does too:
`every_chrome_token_sits_outside_the_magenta_band` asks `in_magenta_band` of
each `chrome::rgb` value, both appearances, without copying the table.

## Consequences

Rooms that draw `'@'` change color for every player. Times Tables is a
flagship plate and draws that ink, so its modern plate and the phosphor,
8-bit, and vector probes are rebaselined in the commit that names this
decision. Buffon's needle also draws `'@'`. Its committed flagship plate did
not change, so that file stays. Audio is untouched. Existing magenta room
accents remain until a later, separate rebaseline. A new accent in the band
fails the guard test.

## Alternatives considered

(a) Retune every catalog accent in the band now. Rejected: it repaints rooms
that were not asking for a new hue, and it ties this change to the Sensory
Lift's one visual rebaseline.

(b) Retune only new rooms and the flagship plates. Rejected: the flagship
accents are not the magenta defect. The plates that change here change because
they draw the spectral ink.
