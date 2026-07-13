//! The [`Room`] contract: the plugin every mathematical phenomenon implements.
//!
//! This is the face-neutral seam described in `docs/ARCHITECTURE.md`. It keeps
//! deterministic rendering, reveal text, interaction, notation, and sound
//! specifications in the core while windows, protocols, and persistence remain
//! responsibilities of the faces.

use crate::sound::SoundSpec;
use crate::surface::Surface;

/// The face-neutral default action for rooms without a dedicated touch verb.
pub const DEFAULT_ROOM_ACTION: &str = "SCRUB TIME";

/// The touch-first default action for App arrival cards and HUD hints.
pub const DEFAULT_TOUCH_ROOM_ACTION: &str = "DRAG: SCRUB TIME";

/// Maximum static hand points a face should pass to [`Room::render_poked`].
pub const MAX_ROOM_POKES: usize = 24;

/// Static, human- and agent-readable description of a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMeta {
    /// Stable identifier used on the command line and in the registry, e.g. `"times-tables"`.
    pub id: &'static str,
    /// Display title, e.g. `"Times Tables"`.
    pub title: &'static str,
    /// The Wing this room belongs to, e.g. `"Number & Pattern"`.
    pub wing: &'static str,
    /// One-line description of what the room does.
    pub blurb: &'static str,
    /// The room's signature accent color as `[r, g, b]`, added per stroke on the
    /// near-black stage so overlapping strokes glow (see `docs/VISUALS.md`).
    pub accent: [u8; 3],
}

/// A playable mathematical phenomenon.
///
/// Implementations are deterministic: the same inputs always produce the same
/// output, so renders reproduce exactly across runs, faces, and machines.
pub trait Room {
    /// This room's static metadata.
    fn meta(&self) -> RoomMeta;

    /// Render a single deterministic frame into `surface`.
    ///
    /// The surface may be an ASCII `Canvas`, a pixel `Raster`, or any other
    /// target. `t` is a normalized phase in `[0.0, 1.0)`; implementations should
    /// clamp defensively and never panic on any value or surface size.
    fn render(&self, surface: &mut dyn Surface, t: f64);

    /// The revelation: the short, true insight that reframes what the player
    /// just did (see `docs/INSIGHTS.md`). Surfaced only when asked, never pushed.
    fn reveal(&self) -> &'static str;

    /// The phase this room is proudest of: what the gallery, the contact
    /// sheet, and any other postcard should show. Found by looking (the beauty
    /// QA loop in `docs/QUALITY.md`); defaults to the start of the sweep.
    fn postcard_t(&self) -> f64 {
        0.0
    }

    /// The room's musical identity: a phrase, not a tone (Engine A2).
    /// None falls back to the generic seeded chiptune bed.
    fn motif(&self) -> Option<crate::motifs::Motif> {
        None
    }

    /// The live readout: what the dial says right now ("K = 2.98, 2 LOBES,
    /// ALMOST CLOSING"). The math answering back as you scrub. None stays
    /// silent; keep it one short line.
    fn status(&self, t: f64) -> Option<String> {
        let _ = t;
        None
    }

    /// The touch-surface verb: what a hand can do here, named for the arrival
    /// card ("CLICK: DROP A STORM", "DRAG: TURN THE DIAL"). None means the room
    /// has no dedicated poke or drag behavior; faces may still offer generic
    /// phase scrubbing through [`room_action`] or [`room_touch_action`].
    fn verb(&self) -> Option<&'static str> {
        None
    }

    /// Render with hands in the scene: `pokes` are normalized (x, y) points
    /// the player has placed, newest last. Faces bound this history to
    /// [`MAX_ROOM_POKES`]. Rooms that answer override this; the default
    /// performs exactly as `render`, so nothing changes until a room chooses
    /// to listen.
    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let _ = pokes;
        self.render(canvas, t);
    }

    /// Render with the hand's full gesture history: `inputs` are replayable
    /// [`RoomInput`] events, newest last, bounded by faces to
    /// [`MAX_ROOM_INPUTS`]. The default translates pointer-down and
    /// pointer-move points into legacy pokes (a drag paints its trail, as the
    /// App does today) and defers to [`Room::render_poked`], so every
    /// existing room answers exactly as before; rooms whose math wants held
    /// input (drag a dial, pull and release a pendulum) override this instead.
    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let pokes = pokes_from_inputs(inputs);
        if pokes.is_empty() {
            self.render(canvas, t);
        } else {
            self.render_poked(canvas, t, &pokes);
        }
    }

    /// Deeper cuts, in order of depth: true, retellable, math-teacher-grade
    /// gems that unlock as the journey deepens (the faces choose thresholds;
    /// see `docs/PLAYFUL.md`). The knowledge is the loot. Empty by default.
    fn deep_cuts(&self) -> &'static [&'static str] {
        &[]
    }

    /// This room's sound at phase `t` (the "everything is an instrument" pillar,
    /// see `docs/SOUND.md`). The default is a single tone that rises with `t`;
    /// rooms override this to give their own voice.
    fn sound(&self, t: f64) -> SoundSpec {
        // Prefer the room's own motif, so the voice you hear is the phrase
        // `listen_room` reports as its notation and each room sounds like
        // itself, not like every other room's fallback (a mind that navigates
        // by ear caught them all sounding identical). Rooms whose math has
        // richer, phase-varying music (Collatz's orbit, Epicycles' harmonic
        // stack, Lissajous' tuned ratio) override this with something truer.
        if let Some(motif) = self.motif() {
            return SoundSpec::from_motif(&motif);
        }
        let phase = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let root = 110.0 * 2.0_f32.powf(phase as f32);
        SoundSpec::arpeggio(&[root, root * 1.5, root * 2.0], 1.5, 0.2)
    }
}

/// Maximum input events a face should pass to [`Room::render_input`].
///
/// A gesture is many events (down, moves, up), so this is larger than
/// [`MAX_ROOM_POKES`]; it bounds render work and keeps trails replayable.
pub const MAX_ROOM_INPUTS: usize = 96;

/// One replayable hand event inside a room, in normalized [0, 1] coordinates.
///
/// Ruling 2 of the July 2026 review (`docs/REVIEW.md`): the poke must become
/// a real input substrate, not one-shot clicks. Faces record what the hand
/// did as plain data, newest last, so a room can give a held gesture real
/// semantics while staying stateless, deterministic, and replayable across
/// App, CLI, and MCP. Pointer events carry the room phase `t` at which they
/// happened, because held semantics (release velocity, drag-start phase) are
/// timing questions and time keeps advancing during a gesture. The enum is
/// non-exhaustive: variants will grow (parameters, pinch) without breaking
/// downstream matches.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoomInput {
    /// The pointer landed at a point.
    PointerDown {
        /// Normalized column, 0 at the left edge.
        x: f64,
        /// Normalized row, 0 at the top edge.
        y: f64,
        /// The room phase when the pointer landed.
        t: f64,
    },
    /// The pointer moved while held.
    PointerMove {
        /// Normalized column, 0 at the left edge.
        x: f64,
        /// Normalized row, 0 at the top edge.
        y: f64,
        /// The room phase when the pointer passed this point.
        t: f64,
    },
    /// The pointer lifted at a point, ending a gesture.
    PointerUp {
        /// Normalized column, 0 at the left edge.
        x: f64,
        /// Normalized row, 0 at the top edge.
        y: f64,
        /// The room phase when the pointer lifted.
        t: f64,
    },
    /// The gesture ended without a meaningful lift point (focus loss, a
    /// modal opening). Held rooms treat this as "let go where you were,
    /// gently": no release semantics should fire from it.
    PointerCancel,
    /// A wheel or pinch step; positive means away or up.
    Wheel {
        /// Signed step count; faces normalize device units.
        delta: f64,
    },
    /// A character key pressed inside the room.
    Key {
        /// The character as typed.
        ch: char,
    },
}

/// The legacy poke points inside an input trail: every pointer-down and
/// pointer-move point, newest last, capped to the newest [`MAX_ROOM_POKES`].
///
/// Moves count because that is how the App behaves today: a drag paints its
/// trail samples as pokes. The shape is preserved, not the exact sample
/// list: faces still own their own decimation and clamping (the App skips
/// near-duplicate trail points and normalizes before storing). Lift and
/// cancel events carry no paint. Points are passed raw (no finiteness
/// filtering), matching the documented room contract of
/// newest-raw-tail-then-filter.
#[must_use]
pub fn pokes_from_inputs(inputs: &[RoomInput]) -> Vec<(f64, f64)> {
    let points: Vec<(f64, f64)> = inputs
        .iter()
        .filter_map(|input| match *input {
            RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. } => {
                Some((x, y))
            }
            _ => None,
        })
        .collect();
    let start = points.len().saturating_sub(MAX_ROOM_POKES);
    points[start..].to_vec()
}

/// The newest pointer gesture in an input trail, summarized for held rooms.
///
/// Rooms that give gestures physical meaning (pin the bob, fling on release)
/// care about the *latest* gesture and its timing, not the raw event list;
/// this is that reading, shared so every held room parses trails identically.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    /// The pointer is down right now: the room is being held at `at`,
    /// having started at `from`. Both are `(x, y, t)`.
    Held {
        /// Where the hand started: the pointer-down event.
        from: (f64, f64, f64),
        /// Where the hand is now: the newest down or move event.
        at: (f64, f64, f64),
    },
    /// The gesture completed with a lift. `before` is the last point the
    /// hand passed before `at`, so releases carry a measurable velocity;
    /// for a tap with no movement the two coincide.
    Released {
        /// The point the hand passed just before lifting.
        before: (f64, f64, f64),
        /// Where and when the hand lifted.
        at: (f64, f64, f64),
    },
    /// The gesture ended without a lift (focus loss, a modal). `at` is the
    /// last point the hand was known to be; no release semantics should fire.
    Cancelled {
        /// The hand's last known point.
        at: (f64, f64, f64),
    },
}

/// Summarize the newest pointer gesture in `inputs`, newest last.
///
/// Wheel and key noise is skipped, and a cancel that closed nothing (one
/// arriving after a completed release) is skipped too: the release stands.
/// `None` means no pointer activity exists to interpret, including a cancel
/// with no known point behind it.
#[must_use]
pub fn latest_gesture(inputs: &[RoomInput]) -> Option<Gesture> {
    let mut end = inputs.len();
    loop {
        while end > 0
            && matches!(
                inputs[end - 1],
                RoomInput::Wheel { .. } | RoomInput::Key { .. }
            )
        {
            end -= 1;
        }
        if end == 0 {
            return None;
        }
        match inputs[end - 1] {
            RoomInput::PointerDown { x, y, t } => {
                return Some(Gesture::Held {
                    from: (x, y, t),
                    at: (x, y, t),
                });
            }
            RoomInput::PointerMove { x, y, t } => {
                let at = (x, y, t);
                // Walk back to the down that began this gesture; a trail
                // whose down was truncated away starts from its oldest move.
                let mut from = at;
                for input in inputs[..end - 1].iter().rev() {
                    match *input {
                        RoomInput::PointerMove { x, y, t } => from = (x, y, t),
                        RoomInput::PointerDown { x, y, t } => {
                            from = (x, y, t);
                            break;
                        }
                        RoomInput::Wheel { .. } | RoomInput::Key { .. } => {}
                        _ => break,
                    }
                }
                return Some(Gesture::Held { from, at });
            }
            RoomInput::PointerUp { x, y, t } => {
                let at = (x, y, t);
                let before = prior_paint_point(&inputs[..end - 1]).unwrap_or(at);
                return Some(Gesture::Released { before, at });
            }
            RoomInput::PointerCancel => match prior_pointer_event(&inputs[..end - 1]) {
                Some(RoomInput::PointerDown { x, y, t })
                | Some(RoomInput::PointerMove { x, y, t }) => {
                    return Some(Gesture::Cancelled { at: (x, y, t) });
                }
                // The cancel closed nothing (the gesture before it already
                // ended in a lift or another cancel); step past it and read
                // again, so the standing release or cancel is never erased.
                Some(RoomInput::PointerUp { .. }) | Some(RoomInput::PointerCancel) => end -= 1,
                _ => return None,
            },
            _ => return None,
        }
    }
}

/// The newest down or move point before `inputs` ends, within one gesture:
/// the scan stops at any lift or cancel.
fn prior_paint_point(inputs: &[RoomInput]) -> Option<(f64, f64, f64)> {
    for input in inputs.iter().rev() {
        match *input {
            RoomInput::PointerDown { x, y, t } | RoomInput::PointerMove { x, y, t } => {
                return Some((x, y, t));
            }
            RoomInput::Wheel { .. } | RoomInput::Key { .. } => {}
            _ => return None,
        }
    }
    None
}

/// The newest pointer event of any kind before `inputs` ends.
fn prior_pointer_event(inputs: &[RoomInput]) -> Option<RoomInput> {
    inputs
        .iter()
        .rev()
        .find(|input| !matches!(input, RoomInput::Wheel { .. } | RoomInput::Key { .. }))
        .copied()
}

/// The face-neutral action line for text and protocol faces.
pub fn room_action(room: &dyn Room) -> &'static str {
    room.verb().unwrap_or(DEFAULT_ROOM_ACTION)
}

/// The touch-first action line for app arrival cards and HUD hints.
pub fn room_touch_action(room: &dyn Room) -> &'static str {
    room.verb().unwrap_or(DEFAULT_TOUCH_ROOM_ACTION)
}

#[cfg(test)]
mod tests {
    use super::{MAX_ROOM_INPUTS, MAX_ROOM_POKES, Room, RoomInput, RoomMeta, pokes_from_inputs};
    use crate::surface::Surface;

    struct DefaultSoundRoom;

    impl Room for DefaultSoundRoom {
        fn meta(&self) -> RoomMeta {
            RoomMeta {
                id: "default-sound",
                title: "Default Sound",
                wing: "Tests",
                blurb: "A test room for the trait default.",
                accent: [0, 0, 0],
            }
        }

        fn render(&self, _surface: &mut dyn Surface, _t: f64) {}

        fn reveal(&self) -> &'static str {
            "The default sound remains finite."
        }
    }

    #[test]
    fn default_sound_handles_nonfinite_phase() {
        let room = DefaultSoundRoom;
        let base = room.sound(0.0);

        for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let spec = room.sound(t);
            assert_eq!(spec, base);
            assert!(spec.notes.iter().all(|note| note.freq.is_finite()));
        }
    }

    #[test]
    fn inputs_translate_downs_and_moves_to_pokes_newest_last() {
        // Downs AND moves paint, matching the app's drag-trail behavior;
        // lifts, cancels, wheels, and keys carry no paint.
        let inputs = [
            RoomInput::PointerDown {
                x: 0.1,
                y: 0.2,
                t: 0.00,
            },
            RoomInput::PointerMove {
                x: 0.3,
                y: 0.3,
                t: 0.01,
            },
            RoomInput::PointerUp {
                x: 0.4,
                y: 0.4,
                t: 0.02,
            },
            RoomInput::PointerCancel,
            RoomInput::Wheel { delta: 1.0 },
            RoomInput::Key { ch: 'r' },
            RoomInput::PointerDown {
                x: 0.5,
                y: 0.6,
                t: 0.03,
            },
        ];
        assert_eq!(
            pokes_from_inputs(&inputs),
            vec![(0.1, 0.2), (0.3, 0.3), (0.5, 0.6)],
        );
    }

    #[test]
    fn inputs_cap_legacy_pokes_to_the_newest_tail() {
        let inputs: Vec<RoomInput> = (0..MAX_ROOM_POKES + 5)
            .map(|i| RoomInput::PointerDown {
                x: i as f64 / 40.0,
                y: 0.5,
                t: 0.0,
            })
            .collect();
        let pokes = pokes_from_inputs(&inputs);
        assert_eq!(pokes.len(), MAX_ROOM_POKES);
        assert_eq!(
            pokes.last().copied(),
            Some(((MAX_ROOM_POKES + 4) as f64 / 40.0, 0.5)),
            "the newest event survives the cap"
        );
    }

    #[test]
    fn the_default_gesture_render_matches_render_poked() {
        // A room that only knows render_poked answers gesture input
        // identically: downs and moves become pokes, the rest is ignored.
        let room = crate::registry::room_by_id("voronoi").expect("voronoi exists");
        let mut via_pokes = crate::canvas::Canvas::new(40, 20);
        room.render_poked(&mut via_pokes, 0.25, &[(0.3, 0.7), (0.35, 0.7)]);
        let mut via_inputs = crate::canvas::Canvas::new(40, 20);
        room.render_input(
            &mut via_inputs,
            0.25,
            &[
                RoomInput::PointerDown {
                    x: 0.3,
                    y: 0.7,
                    t: 0.25,
                },
                RoomInput::PointerMove {
                    x: 0.35,
                    y: 0.7,
                    t: 0.26,
                },
                RoomInput::PointerUp {
                    x: 0.35,
                    y: 0.7,
                    t: 0.27,
                },
                RoomInput::Wheel { delta: -2.0 },
            ],
        );
        assert_eq!(via_pokes.to_text(), via_inputs.to_text());
    }

    #[test]
    fn a_gesture_with_no_paint_renders_the_bare_room() {
        let room = crate::registry::room_by_id("voronoi").expect("voronoi exists");
        let mut bare = crate::canvas::Canvas::new(40, 20);
        room.render(&mut bare, 0.25);
        let mut gestured = crate::canvas::Canvas::new(40, 20);
        room.render_input(
            &mut gestured,
            0.25,
            &[
                RoomInput::PointerCancel,
                RoomInput::Wheel { delta: 3.0 },
                RoomInput::Key { ch: 'x' },
            ],
        );
        assert_eq!(bare.to_text(), gestured.to_text());
    }

    #[test]
    fn a_live_drag_reads_as_held_from_its_down_point() {
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.2,
                t: 0.10,
            },
            RoomInput::PointerMove {
                x: 0.4,
                y: 0.3,
                t: 0.12,
            },
            RoomInput::Wheel { delta: 1.0 },
            RoomInput::PointerMove {
                x: 0.6,
                y: 0.4,
                t: 0.14,
            },
        ];
        assert_eq!(
            super::latest_gesture(&inputs),
            Some(super::Gesture::Held {
                from: (0.2, 0.2, 0.10),
                at: (0.6, 0.4, 0.14),
            })
        );
    }

    #[test]
    fn a_lift_reads_as_released_with_the_point_before_it() {
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.2,
                t: 0.10,
            },
            RoomInput::PointerMove {
                x: 0.4,
                y: 0.3,
                t: 0.12,
            },
            RoomInput::PointerUp {
                x: 0.5,
                y: 0.35,
                t: 0.13,
            },
            RoomInput::Key { ch: 'r' },
        ];
        assert_eq!(
            super::latest_gesture(&inputs),
            Some(super::Gesture::Released {
                before: (0.4, 0.3, 0.12),
                at: (0.5, 0.35, 0.13),
            })
        );
        // A tap with no movement releases where it landed.
        let tap = [
            RoomInput::PointerDown {
                x: 0.7,
                y: 0.7,
                t: 0.20,
            },
            RoomInput::PointerUp {
                x: 0.7,
                y: 0.7,
                t: 0.21,
            },
        ];
        assert_eq!(
            super::latest_gesture(&tap),
            Some(super::Gesture::Released {
                before: (0.7, 0.7, 0.20),
                at: (0.7, 0.7, 0.21),
            })
        );
    }

    #[test]
    fn a_cancel_reads_gently_and_an_empty_trail_reads_as_nothing() {
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.2,
                t: 0.10,
            },
            RoomInput::PointerCancel,
        ];
        assert_eq!(
            super::latest_gesture(&inputs),
            Some(super::Gesture::Cancelled {
                at: (0.2, 0.2, 0.10)
            })
        );
        assert_eq!(super::latest_gesture(&[]), None);
        assert_eq!(
            super::latest_gesture(&[RoomInput::PointerCancel]),
            None,
            "a cancel with no known point is not a gesture"
        );
        assert_eq!(
            super::latest_gesture(&[RoomInput::Wheel { delta: 2.0 }]),
            None
        );
    }

    #[test]
    fn a_stale_cancel_after_a_release_does_not_erase_it() {
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.2,
                t: 0.10,
            },
            RoomInput::PointerUp {
                x: 0.3,
                y: 0.2,
                t: 0.12,
            },
            RoomInput::PointerCancel,
        ];
        let released = Some(super::Gesture::Released {
            before: (0.2, 0.2, 0.10),
            at: (0.3, 0.2, 0.12),
        });
        assert_eq!(
            super::latest_gesture(&inputs),
            released,
            "a cancel that closed nothing leaves the release standing"
        );
        // Piles of stale cancels change nothing either.
        let mut doubled = inputs.to_vec();
        doubled.push(RoomInput::PointerCancel);
        assert_eq!(super::latest_gesture(&doubled), released);
        let cancelled_twice = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.2,
                t: 0.10,
            },
            RoomInput::PointerCancel,
            RoomInput::PointerCancel,
        ];
        assert_eq!(
            super::latest_gesture(&cancelled_twice),
            Some(super::Gesture::Cancelled {
                at: (0.2, 0.2, 0.10)
            }),
            "a repeated cancel still reads as the one real cancel"
        );
    }

    #[test]
    fn a_truncated_trail_still_reads_sanely() {
        // A gesture whose down was evicted by the event cap reads as held
        // from its oldest surviving move.
        let inputs = [
            RoomInput::PointerMove {
                x: 0.4,
                y: 0.4,
                t: 0.20,
            },
            RoomInput::PointerMove {
                x: 0.5,
                y: 0.4,
                t: 0.22,
            },
        ];
        assert_eq!(
            super::latest_gesture(&inputs),
            Some(super::Gesture::Held {
                from: (0.4, 0.4, 0.20),
                at: (0.5, 0.4, 0.22),
            })
        );
        // A bare lift releases where it lifted, with no measurable approach.
        let bare_up = [RoomInput::PointerUp {
            x: 0.6,
            y: 0.6,
            t: 0.30,
        }];
        assert_eq!(
            super::latest_gesture(&bare_up),
            Some(super::Gesture::Released {
                before: (0.6, 0.6, 0.30),
                at: (0.6, 0.6, 0.30),
            })
        );
    }

    #[test]
    fn only_the_newest_gesture_is_read() {
        let inputs = [
            RoomInput::PointerDown {
                x: 0.1,
                y: 0.1,
                t: 0.01,
            },
            RoomInput::PointerUp {
                x: 0.15,
                y: 0.1,
                t: 0.02,
            },
            RoomInput::PointerDown {
                x: 0.8,
                y: 0.8,
                t: 0.30,
            },
            RoomInput::PointerMove {
                x: 0.85,
                y: 0.8,
                t: 0.31,
            },
        ];
        assert_eq!(
            super::latest_gesture(&inputs),
            Some(super::Gesture::Held {
                from: (0.8, 0.8, 0.30),
                at: (0.85, 0.8, 0.31),
            })
        );
    }

    #[test]
    fn every_catalog_room_accepts_a_mixed_gesture_trail() {
        // The substrate invariant: render_input never panics and stays
        // deterministic for any room, given a full mixed bounded trail.
        let trail: Vec<RoomInput> = (0..MAX_ROOM_INPUTS)
            .map(|i| {
                let t = i as f64 / MAX_ROOM_INPUTS as f64;
                match i % 6 {
                    0 => RoomInput::PointerDown {
                        x: (i % 10) as f64 / 10.0,
                        y: (i % 7) as f64 / 7.0,
                        t,
                    },
                    1 => RoomInput::PointerMove {
                        x: (i % 9) as f64 / 9.0,
                        y: 0.5,
                        t,
                    },
                    2 => RoomInput::PointerUp { x: 0.5, y: 0.5, t },
                    3 => RoomInput::PointerCancel,
                    4 => RoomInput::Wheel {
                        delta: (i as f64) - 4.0,
                    },
                    _ => RoomInput::Key { ch: 'r' },
                }
            })
            .collect();
        for room in crate::registry::all_rooms() {
            let mut once = crate::canvas::Canvas::new(40, 20);
            room.render_input(&mut once, 0.5, &trail);
            let mut twice = crate::canvas::Canvas::new(40, 20);
            room.render_input(&mut twice, 0.5, &trail);
            assert_eq!(
                once.to_text(),
                twice.to_text(),
                "{} must stay deterministic under gesture input",
                room.meta().id
            );
        }
    }
}
