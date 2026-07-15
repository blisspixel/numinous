//! The room registry: the catalog every face enumerates.
//!
//! The registry is the only thing a face needs to know about; it never depends
//! on a room's internals (see the dependency rule in `docs/ARCHITECTURE.md`).

use crate::room::Room;
use crate::rooms;

/// All built-in rooms, in catalog order. Default variation 0 pins tests and postcards.
#[must_use]
pub fn all_rooms() -> Vec<Box<dyn Room>> {
    all_rooms_with(0)
}

/// All rooms with a per-visit variation seed (default 0 keeps exact behavior for
/// tests, postcards, and determinism). Rooms that support it read the seed for
/// replayable novelty. See ARCADE.md and DIGITAL_MINDS.md.
#[must_use]
pub fn all_rooms_with(variation: u64) -> Vec<Box<dyn Room>> {
    vec![
        Box::new(rooms::times_tables::TimesTables::new_with(variation)),
        Box::new(rooms::cellular_automata::CellularAutomata::new_with(
            variation,
        )),
        Box::new(rooms::chaos_game::ChaosGame::new_with(variation)),
        Box::new(rooms::golden_angle::GoldenAngle::new_with(variation)),
        Box::new(rooms::galton_board::GaltonBoard::new_with(variation)),
        Box::new(rooms::lissajous::Lissajous::new_with(variation)),
        Box::new(rooms::chladni::Chladni::new_with(variation)),
        Box::new(rooms::ripple::Ripple::new_with(variation)),
        Box::new(rooms::coffee_cup::CoffeeCup::new_with(variation)),
        Box::new(rooms::ford_circles::FordCircles::new_with(variation)),
        Box::new(rooms::zeta_walk::ZetaWalk::new_with(variation)),
        Box::new(rooms::starbow::Starbow::new_with(variation)),
        Box::new(rooms::slingshot::Slingshot::new_with(variation)),
        Box::new(rooms::first_rain::FirstRain::new_with(variation)),
        Box::new(rooms::the_magnet::TheMagnet::new_with(variation)),
        Box::new(rooms::kepler_loom::KeplerLoom::new_with(variation)),
        Box::new(rooms::phantom_jam::PhantomJam::new_with(variation)),
        Box::new(rooms::fastest_fall::FastestFall::new_with(variation)),
        Box::new(rooms::audioactive::Audioactive::new_with(variation)),
        Box::new(rooms::busy_beaver::BusyBeaver::new_with(variation)),
        Box::new(rooms::degree720::Degree720::new_with(variation)),
        Box::new(rooms::upside_ruler::UpsideRuler::new_with(variation)),
        Box::new(rooms::murmuration::Murmuration::new_with(variation)),
        Box::new(rooms::whispering_table::WhisperingTable::new_with(
            variation,
        )),
        Box::new(rooms::wet_oracle::WetOracle::new_with(variation)),
        Box::new(rooms::tilt_cone::TiltCone::new_with(variation)),
        Box::new(rooms::the_stretch::TheStretch::new_with(variation)),
        Box::new(rooms::laplace_clock::LaplaceClock::new_with(variation)),
        Box::new(rooms::message_heals::MessageHeals::new_with(variation)),
        Box::new(rooms::unlit_room::UnlitRoom::new_with(variation)),
        Box::new(rooms::the_lens::TheLens::new_with(variation)),
        Box::new(rooms::fourteen_beacons::FourteenBeacons::new_with(
            variation,
        )),
        Box::new(rooms::loneliness::Loneliness::new_with(variation)),
        Box::new(rooms::chord_game::ChordGame::new_with(variation)),
        Box::new(rooms::recaman::Recaman::new_with(variation)),
        Box::new(rooms::truchet::Truchet::new_with(variation)),
        Box::new(rooms::pursuit::Pursuit::new_with(variation)),
        Box::new(rooms::pascal_mod::PascalMod::new_with(variation)),
        Box::new(rooms::three_gap::ThreeGap::new_with(variation)),
        Box::new(rooms::morley::Morley::new_with(variation)),
        Box::new(rooms::menagerie::Menagerie::new_with(variation)),
        Box::new(rooms::apollonian::Apollonian::new_with(variation)),
        Box::new(rooms::inversion::Inversion::new_with(variation)),
        Box::new(rooms::dla_frost::DlaFrost::new_with(variation)),
        Box::new(rooms::kaprekar::Kaprekar::new_with(variation)),
        Box::new(rooms::steiner::Steiner::new_with(variation)),
        Box::new(rooms::hopf::Hopf::new_with(variation)),
        Box::new(rooms::wireworld::Wireworld::new_with(variation)),
        Box::new(rooms::buddhabrot::Buddhabrot::new_with(variation)),
        Box::new(rooms::harmonics::Harmonics::new_with(variation)),
        Box::new(rooms::function_painter::FunctionPainter::new_with(
            variation,
        )),
        Box::new(rooms::newton::Newton::new_with(variation)),
        Box::new(rooms::koch::Koch::new_with(variation)),
        Box::new(rooms::hilbert::Hilbert::new_with(variation)),
        Box::new(rooms::gray_scott::GrayScott::new_with(variation)),
        Box::new(rooms::sieve::Sieve::new_with(variation)),
        Box::new(rooms::curse_dimension::CurseDimension::new_with(variation)),
        Box::new(rooms::concentration::Concentration::new_with(variation)),
        Box::new(rooms::uncertainty::Uncertainty::new_with(variation)),
        Box::new(rooms::gradient_valley::GradientValley::new_with(variation)),
        Box::new(rooms::attention::Attention::new_with(variation)),
        Box::new(rooms::braess::Braess::new_with(variation)),
        Box::new(rooms::nontransitive::Nontransitive::new_with(variation)),
        Box::new(rooms::parrondo::Parrondo::new_with(variation)),
        Box::new(rooms::hilbert_hotel::HilbertHotel::new_with(variation)),
        Box::new(rooms::soap_film::SoapFilm::new_with(variation)),
        Box::new(rooms::landauer::Landauer::new_with(variation)),
        Box::new(rooms::prime_gaps::PrimeGaps::new_with(variation)),
        Box::new(rooms::sphere_eversion::SphereEversion::new_with(variation)),
        Box::new(rooms::causal_doors::CausalDoors::new_with(variation)),
        Box::new(rooms::soft_proof::SoftProof::new_with(variation)),
        Box::new(rooms::learning_clock::LearningClock::new_with(variation)),
        Box::new(rooms::duality::Duality::new_with(variation)),
        Box::new(rooms::mirror_forms::MirrorForms::new_with(variation)),
        Box::new(rooms::penrose::Penrose::new_with(variation)),
        Box::new(rooms::continued_frac::ContinuedFrac::new_with(variation)),
        Box::new(rooms::logistic_cobweb::LogisticCobweb::new_with(variation)),
        Box::new(rooms::sierpinski_carpet::SierpinskiCarpet::new_with(
            variation,
        )),
        Box::new(rooms::pythagoras_tree::PythagorasTree::new_with(variation)),
        Box::new(rooms::ulam_spiral::UlamSpiral::new_with(variation)),
        Box::new(rooms::prime_spirals::PrimeSpirals::new_with(variation)),
        Box::new(rooms::cult_of_pi::CultOfPi::new_with(variation)),
        Box::new(rooms::collatz::Collatz::new_with(variation)),
        Box::new(rooms::buffon_needle::BuffonNeedle::new_with(variation)),
        Box::new(rooms::game_of_life::GameOfLife::new_with(variation)),
        Box::new(rooms::sandpile::Sandpile::new_with(variation)),
        Box::new(rooms::mandelbrot::Mandelbrot::new_with(variation)),
        Box::new(rooms::julia::Julia::new_with(variation)),
        Box::new(rooms::barnsley_fern::BarnsleyFern::new_with(variation)),
        Box::new(rooms::lsystem::LSystemGarden::new_with(variation)),
        Box::new(rooms::harmonograph::Harmonograph::new_with(variation)),
        Box::new(rooms::logistic_map::LogisticMap::new_with(variation)),
        Box::new(rooms::langtons_ant::LangtonsAnt::new_with(variation)),
        Box::new(rooms::lorenz::Lorenz::new_with(variation)),
        Box::new(rooms::arecibo::Arecibo::new_with(variation)),
        Box::new(rooms::the_pour::ThePour::new_with(variation)),
        Box::new(rooms::slope_rider::SlopeRider::new_with(variation)),
        Box::new(rooms::double_pendulum::DoublePendulum::new_with(variation)),
        Box::new(rooms::epicycles::Epicycles::new_with(variation)),
        Box::new(rooms::random_walk::RandomWalk::new_with(variation)),
        Box::new(rooms::voronoi::Voronoi::new_with(variation)),
        Box::new(rooms::mobius::Mobius::new_with(variation)),
        Box::new(rooms::zeno::Zeno::new_with(variation)),
        Box::new(rooms::goldbach::Goldbach::new_with(variation)),
        Box::new(rooms::quine::Quine::new_with(variation)),
        Box::new(rooms::strange_loop::StrangeLoop::new_with(variation)),
        Box::new(rooms::dragon_curve::DragonCurve::new_with(variation)),
        Box::new(rooms::fibonacci_word::FibonacciWord::new_with(variation)),
        Box::new(rooms::newton_basins_cubic::NewtonCubic::new_with(variation)),
        Box::new(rooms::henon::Henon::new_with(variation)),
        Box::new(rooms::rules30::Rules30::new_with(variation)),
        Box::new(rooms::mandelbulb_slice::MandelbulbSlice::new_with(
            variation,
        )),
        Box::new(rooms::thue_morse::ThueMorse::new_with(variation)),
        Box::new(rooms::rossler::Rossler::new_with(variation)),
        Box::new(rooms::cantor_set::CantorSet::new_with(variation)),
        Box::new(rooms::weierstrass::Weierstrass::new_with(variation)),
        Box::new(rooms::peano_curve::PeanoCurve::new_with(variation)),
        Box::new(rooms::van_der_pol::VanDerPol::new_with(variation)),
        Box::new(rooms::ikeda::Ikeda::new_with(variation)),
        Box::new(rooms::duffing::Duffing::new_with(variation)),
        Box::new(rooms::levy_c::LevyC::new_with(variation)),
        Box::new(rooms::tinkerbell::Tinkerbell::new_with(variation)),
        Box::new(rooms::gingerbread::Gingerbread::new_with(variation)),
        Box::new(rooms::menger_slice::MengerSlice::new_with(variation)),
        Box::new(rooms::bifurcation::Bifurcation::new_with(variation)),
        Box::new(rooms::stern_brocot::SternBrocot::new_with(variation)),
        Box::new(rooms::josephus::Josephus::new_with(variation)),
        Box::new(rooms::calkin_wilf::CalkinWilf::new_with(variation)),
        Box::new(rooms::fourier_square::FourierSquare::new_with(variation)),
        Box::new(rooms::sierpinski_arrowhead::SierpinskiArrowhead::new_with(
            variation,
        )),
        Box::new(rooms::clifford::Clifford::new_with(variation)),
        Box::new(rooms::dejong::DeJong::new_with(variation)),
        Box::new(rooms::svensson::Svensson::new_with(variation)),
        Box::new(rooms::bedhead::Bedhead::new_with(variation)),
        Box::new(rooms::hopalong::Hopalong::new_with(variation)),
        Box::new(rooms::gumowski_mira::GumowskiMira::new_with(variation)),
        Box::new(rooms::pickover::Pickover::new_with(variation)),
        Box::new(rooms::aizawa::Aizawa::new_with(variation)),
        Box::new(rooms::thomas::Thomas::new_with(variation)),
        Box::new(rooms::halvorsen::Halvorsen::new_with(variation)),
        Box::new(rooms::rabinovich_fabrikant::RabinovichFabrikant::new_with(
            variation,
        )),
        Box::new(rooms::three_scroll::ThreeScroll::new_with(variation)),
    ]
}

/// Find a built-in room by its stable id, if it exists.
#[must_use]
pub fn room_by_id(id: &str) -> Option<Box<dyn Room>> {
    all_rooms().into_iter().find(|room| room.meta().id == id)
}

/// The rooms that are not in the catalog. Never listed, never announced; the
/// faces decide who may enter (by rank, see `crate::journey`). Calling this a
/// registry function is already saying too much.
#[must_use]
pub fn hidden_room_by_id(id: &str) -> Option<Box<dyn Room>> {
    match id {
        "tetractys" => Some(Box::new(rooms::tetractys::Tetractys::new())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{all_rooms, room_by_id};
    use crate::canvas::Canvas;
    use crate::room::Room;

    fn render_text(room: &dyn Room, t: f64) -> String {
        let mut canvas = Canvas::new(48, 28);
        room.render(&mut canvas, t);
        canvas.to_text()
    }

    fn render_poked_text(room: &dyn Room, t: f64, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 28);
        room.render_poked(&mut canvas, t, pokes);
        canvas.to_text()
    }

    fn room_text(rooms: &[Box<dyn Room>], id: &str, t: f64) -> String {
        let room = rooms
            .iter()
            .find(|room| room.meta().id == id)
            .unwrap_or_else(|| panic!("{id} must be registered"));
        render_text(room.as_ref(), t)
    }

    #[test]
    fn registry_is_non_empty() {
        assert!(!all_rooms().is_empty());
    }

    #[test]
    fn every_room_has_a_unique_id() {
        let rooms = all_rooms();
        let mut ids: Vec<&str> = rooms.iter().map(|r| r.meta().id).collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(unique, ids.len(), "room ids must be unique");
    }

    #[test]
    fn every_room_postcard_has_ink() {
        // The beauty-QA invariant: no room may present an empty postcard.
        for room in all_rooms() {
            let mut canvas = Canvas::new(60, 40);
            room.render(&mut canvas, room.postcard_t());
            assert!(
                canvas.ink_count() > 10,
                "{} is blank at its postcard phase",
                room.meta().id
            );
        }
    }

    #[test]
    fn every_catalog_room_has_first_contact_status() {
        // The kid-principle invariant: first contact always names something
        // readable before the player acts. Empty status is not an invitation.
        for room in all_rooms() {
            let status = room.status(0.0);
            assert!(
                status.as_ref().is_some_and(|s| !s.trim().is_empty()),
                "{} opens silent; first contact needs a status line",
                room.meta().id
            );
        }
    }

    #[test]
    fn first_contact_status_names_an_action_or_goal_when_the_room_has_a_verb() {
        // Rooms that publish a touch verb should invite play on first contact:
        // either a direct action token (CLICK/DRAG/...) or a clear measured
        // goal (TARGET/FOUND/GOAL) so the status is not ambient-only prose.
        const INVITE_TOKENS: &[&str] = &[
            "CLICK", "DRAG", "HOLD", "DROP", "PLANT", "FLIP", "TRY", "SEED", "THROW", "TEST",
            "DIVE", "TOUCH", "PIN", "TURN", "MOVE", "PAINT", "TRACE", "BRUSH", "TUNE", "POUR",
            "RIDE", "SOW", "SCRUB", "PICK", "PUSH", "PULL", "PERTURB", "MORPH", "DIAL", "HAND",
            "COIN", "WAVE", "BET", "FIX", "PLACE", "PRINT", "NEST", "WELL", "STORM", "GLIDER",
            "WIDTH", "ORBIT", "TAP", "SWEEP", "STEER", "AIM", "REPLAY", "LAUNCH", "STRIKE", "CUT",
            "DRAW", "SPIN", "ZOOM", "FOCUS", "POINT", "TARGET", "GOAL", "OPEN", "INVITE", "CHOOSE",
        ];
        let mut shallow = Vec::new();
        for room in all_rooms() {
            let Some(verb) = room.verb() else {
                continue;
            };
            let id = room.meta().id;
            let open = room.status(0.0).unwrap_or_default();
            let upper = open.to_ascii_uppercase();
            let hit = INVITE_TOKENS.iter().any(|token| upper.contains(token));
            if !hit {
                shallow.push(format!("{id}: verb={verb:?} status={open:?}"));
            }
        }
        assert!(
            shallow.is_empty(),
            "first-contact invite missing for:\n{}",
            shallow.join("\n")
        );
    }

    #[test]
    fn poke_changes_status_for_every_catalog_room() {
        // Every catalog room must speak after a center poke: first contact and
        // action consequence stay distinct on the status line.
        use crate::room::RoomInput;
        let poke = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        for room in all_rooms() {
            let id = room.meta().id;
            let open = room.status(0.0).unwrap_or_default();
            let after = room.status_input(0.0, &poke).unwrap_or_default();
            assert_ne!(
                after, open,
                "{id} is touchable but status does not change after a poke"
            );
        }
    }

    #[test]
    fn action_status_reports_a_measured_quantity() {
        // After a center poke, status must carry at least one digit: a measured
        // consequence (count, coordinate, rule number, ratio), not only words.
        use crate::room::RoomInput;
        let poke = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        for room in all_rooms() {
            let id = room.meta().id;
            let after = room.status_input(0.0, &poke).unwrap_or_default();
            assert!(
                after.chars().any(|c| c.is_ascii_digit()),
                "{id} action status has no measured quantity: {after:?}"
            );
        }
    }

    #[test]
    fn action_status_fits_compact_footer() {
        // Compact App footers have a tight character budget beside fixed
        // controls. Center-poke status should stay within a short line.
        use crate::room::RoomInput;
        const MAX_CHARS: usize = 56;
        let poke = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        for room in all_rooms() {
            let id = room.meta().id;
            let after = room.status_input(0.0, &poke).unwrap_or_default();
            assert!(
                after.chars().count() <= MAX_CHARS,
                "{id} action status is too long for compact footer ({}): {after:?}",
                after.chars().count()
            );
        }
    }

    #[test]
    fn first_contact_status_fits_compact_footer() {
        // Open status shares the same footer budget as action status.
        const MAX_CHARS: usize = 56;
        let mut long = Vec::new();
        for room in all_rooms() {
            let id = room.meta().id;
            let open = room.status(0.0).unwrap_or_default();
            let len = open.chars().count();
            if len > MAX_CHARS {
                long.push(format!("{id} ({len}): {open:?}"));
            }
        }
        assert!(
            long.is_empty(),
            "first-contact status too long for compact footer:\n{}",
            long.join("\n")
        );
    }

    #[test]
    fn lookup_by_id_works_and_misses_are_none() {
        assert!(room_by_id("times-tables").is_some());
        assert!(room_by_id("no-such-room").is_none());
    }

    #[test]
    fn all_rooms_with_variation_produces_different_lsystem() {
        use super::all_rooms_with;
        let r0 = all_rooms_with(0);
        let r1 = all_rooms_with(1);
        assert_eq!(r0.len(), r1.len());
        assert_ne!(
            room_text(&r0, "lsystem-garden", 0.5),
            room_text(&r1, "lsystem-garden", 0.5),
            "registry variation must reach the L-System room"
        );
        assert_ne!(
            room_text(&r0, "quine", 0.6),
            room_text(&r1, "quine", 0.6),
            "registry variation must reach the Quine room"
        );
        assert_ne!(
            room_text(&r0, "double-pendulum", 0.75),
            room_text(&r1, "double-pendulum", 0.75),
            "registry variation must reach animated double-pendulum motion"
        );
        assert_ne!(
            room_text(&r0, "times-tables", 0.2),
            room_text(&r1, "times-tables", 0.2),
            "registry variation must reach Times Tables"
        );
        assert_ne!(
            room_text(&r0, "prime-spirals", 0.3),
            room_text(&r1, "prime-spirals", 0.3),
            "registry variation must reach Prime Spirals"
        );
    }

    #[test]
    fn all_rooms_with_variation_reaches_the_late_variation_rooms() {
        use super::all_rooms_with;
        let r0 = all_rooms_with(0);
        let r42 = all_rooms_with(42);
        for (id, phase) in [
            ("lissajous", 0.35),
            ("harmonograph", 0.4),
            ("logistic-map", 0.3),
            ("the-pour", 0.45),
            ("slope-rider", 0.55),
            ("mobius", 0.35),
            ("zeno", 0.75),
        ] {
            assert_ne!(
                room_text(&r0, id, phase),
                room_text(&r42, id, phase),
                "registry variation must reach {id}"
            );
        }
    }

    #[test]
    fn late_variation_room_seed_zero_matches_default() {
        use crate::rooms::{
            harmonograph::Harmonograph, lissajous::Lissajous, logistic_map::LogisticMap,
            mobius::Mobius, slope_rider::SlopeRider, the_pour::ThePour, zeno::Zeno,
        };
        for (id, phase, default, seeded) in [
            (
                "lissajous",
                0.35,
                Box::new(Lissajous::new()) as Box<dyn Room>,
                Box::new(Lissajous::new_with(0)) as Box<dyn Room>,
            ),
            (
                "harmonograph",
                0.4,
                Box::new(Harmonograph::new()) as Box<dyn Room>,
                Box::new(Harmonograph::new_with(0)) as Box<dyn Room>,
            ),
            (
                "logistic-map",
                0.3,
                Box::new(LogisticMap::new()) as Box<dyn Room>,
                Box::new(LogisticMap::new_with(0)) as Box<dyn Room>,
            ),
            (
                "the-pour",
                0.45,
                Box::new(ThePour::new()) as Box<dyn Room>,
                Box::new(ThePour::new_with(0)) as Box<dyn Room>,
            ),
            (
                "slope-rider",
                0.55,
                Box::new(SlopeRider::new()) as Box<dyn Room>,
                Box::new(SlopeRider::new_with(0)) as Box<dyn Room>,
            ),
            (
                "mobius",
                0.35,
                Box::new(Mobius::new()) as Box<dyn Room>,
                Box::new(Mobius::new_with(0)) as Box<dyn Room>,
            ),
            (
                "zeno",
                0.75,
                Box::new(Zeno::new()) as Box<dyn Room>,
                Box::new(Zeno::new_with(0)) as Box<dyn Room>,
            ),
        ] {
            assert_eq!(
                render_text(default.as_ref(), phase),
                render_text(seeded.as_ref(), phase),
                "{id} seed 0 must preserve the default postcard path"
            );
        }
    }

    #[test]
    fn dynamic_rooms_expose_poke_through_trait_objects() {
        let rooms = all_rooms();
        let julia = rooms
            .iter()
            .find(|room| room.meta().id == "julia")
            .expect("julia must be registered");
        assert_eq!(julia.verb(), Some("CLICK: MORPH C"));
        assert_ne!(
            render_text(julia.as_ref(), 0.35),
            render_poked_text(julia.as_ref(), 0.35, &[(0.9, 0.1)]),
            "Julia poke must dispatch through dyn Room"
        );
    }

    #[test]
    fn every_catalog_room_has_a_structured_motif() {
        for room in all_rooms() {
            let meta = room.meta();
            let motif = room
                .motif()
                .unwrap_or_else(|| panic!("{} must have an Engine A2 motif", meta.id));
            assert!(
                !motif.key.trim().is_empty(),
                "{} motif must name a key",
                meta.id
            );
            assert!(
                motif.root.is_finite() && motif.root > 0.0,
                "{} motif root must be a positive finite frequency",
                meta.id
            );
            assert!(
                (40..=220).contains(&motif.tempo),
                "{} motif tempo must stay playable",
                meta.id
            );
            assert!(
                motif.line.len() >= 6,
                "{} motif must be a phrase, not a sting",
                meta.id
            );
            assert!(
                motif.line.iter().any(|&step| step != 0),
                "{} motif must carry melodic movement",
                meta.id
            );
            assert!(
                !motif.encodes.trim().is_empty(),
                "{} motif must explain the mathematical mapping",
                meta.id
            );
            assert_eq!(
                motif.notation().len(),
                motif.line.len(),
                "{} motif notation must cover the whole phrase",
                meta.id
            );
            assert!(
                motif.pattern().seconds() > 0.0,
                "{} motif must render to a nonempty pattern",
                meta.id
            );
        }
    }

    #[test]
    fn all_rooms_with_variation_affects_poke_rooms() {
        use crate::rooms::{
            chaos_game::ChaosGame, game_of_life::GameOfLife, golden_angle::GoldenAngle,
            langtons_ant::LangtonsAnt, sandpile::Sandpile, strange_loop::StrangeLoop,
            voronoi::Voronoi,
        };
        let c0 = ChaosGame::new_with(0);
        let c1 = ChaosGame::new_with(1);
        let mut ca0 = crate::canvas::Canvas::new(32, 16);
        let mut ca1 = crate::canvas::Canvas::new(32, 16);
        c0.render(&mut ca0, 0.5);
        c1.render(&mut ca1, 0.5);
        assert_ne!(ca0.to_text(), ca1.to_text());
        let g0 = GameOfLife::new_with(0);
        let g1 = GameOfLife::new_with(1);
        let mut ga0 = crate::canvas::Canvas::new(32, 16);
        let mut ga1 = crate::canvas::Canvas::new(32, 16);
        g0.render(&mut ga0, 0.3);
        g1.render(&mut ga1, 0.3);
        assert_ne!(ga0.to_text(), ga1.to_text());
        let v0 = Voronoi::new_with(0);
        let v1 = Voronoi::new_with(1);
        let mut va0 = crate::canvas::Canvas::new(32, 16);
        let mut va1 = crate::canvas::Canvas::new(32, 16);
        v0.render(&mut va0, 0.3);
        v1.render(&mut va1, 0.3);
        assert_ne!(va0.to_text(), va1.to_text());
        // Verify StrangeLoop (self-ref) variation affects render (seed-driven rotation)
        let s0 = StrangeLoop::new_with(0);
        let s1 = StrangeLoop::new_with(1);
        let mut sa0 = crate::canvas::Canvas::new(32, 16);
        let mut sa1 = crate::canvas::Canvas::new(32, 16);
        s0.render(&mut sa0, 0.5);
        s1.render(&mut sa1, 0.5);
        assert_ne!(sa0.to_text(), sa1.to_text());
        // GoldenAngle: variation rotates + jitters seed count for visible per-visit novelty (poke plants respect seed too)
        let ga0 = GoldenAngle::new_with(0);
        let ga42 = GoldenAngle::new_with(42);
        let mut gaa0 = crate::canvas::Canvas::new(32, 16);
        let mut gaa42 = crate::canvas::Canvas::new(32, 16);
        ga0.render(&mut gaa0, 0.0);
        ga42.render(&mut gaa42, 0.0);
        assert_ne!(gaa0.to_text(), gaa42.to_text());
        // LangtonsAnt now has functional variation (initial scatter) + poke pre-integration
        let la0 = LangtonsAnt::new_with(0);
        let la1 = LangtonsAnt::new_with(1);
        let mut laa0 = crate::canvas::Canvas::new(32, 16);
        let mut laa1 = crate::canvas::Canvas::new(32, 16);
        la0.render(&mut laa0, 0.5);
        la1.render(&mut laa1, 0.5);
        assert_ne!(laa0.to_text(), laa1.to_text());
        // Sandpile: variation drifts the ambient pour site so the mandala offsets.
        let sp0 = Sandpile::new_with(0);
        let sp1 = Sandpile::new_with(1);
        let mut spa0 = crate::canvas::Canvas::new(32, 16);
        let mut spa1 = crate::canvas::Canvas::new(32, 16);
        sp0.render(&mut spa0, 0.55);
        sp1.render(&mut spa1, 0.55);
        assert_ne!(spa0.to_text(), spa1.to_text());
    }
}
