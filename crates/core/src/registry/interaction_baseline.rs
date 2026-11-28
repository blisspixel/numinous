/// Whole-frame or regional peak rates above three flashes per second during
/// the scripted input report at seed zero. General and red columns each give
/// the maximum over the measured eras and speeds for the drag, dense beat
/// requests and musical beat requests, in that order. Zero means within the
/// measured budget. Only the paced beat response contributes to this lookup.
pub const KNOWN_OVER_INTERACTION_FLASHES: &[(&str, [u8; 3], [u8; 3])] = &[
    ("aizawa", [8, 0, 0], [0, 0, 0]),
    ("apollonian", [4, 0, 0], [0, 0, 0]),
    ("arecibo", [6, 0, 0], [0, 0, 0]),
    ("barnsley-fern", [4, 0, 0], [0, 0, 0]),
    ("bedhead", [8, 0, 0], [0, 0, 0]),
    ("benford", [4, 0, 0], [0, 0, 0]),
    ("blackbody", [0, 0, 0], [4, 0, 0]),
    ("borromean", [0, 0, 0], [4, 0, 0]),
    ("cellular-automata", [5, 5, 0], [0, 0, 0]),
    ("chaos-game", [4, 4, 0], [0, 0, 0]),
    ("chua", [0, 0, 0], [4, 0, 0]),
    ("clifford", [4, 0, 0], [0, 0, 0]),
    ("collatz", [11, 8, 5], [0, 0, 0]),
    ("dragon-curve", [6, 0, 0], [8, 0, 0]),
    ("duffing", [12, 0, 0], [0, 0, 0]),
    ("erdos-renyi", [4, 0, 0], [0, 0, 0]),
    ("function-painter", [8, 0, 0], [0, 0, 0]),
    ("gibbs-square", [0, 0, 0], [4, 0, 0]),
    ("gingerbread", [10, 0, 0], [0, 0, 0]),
    ("gray-scott", [4, 0, 0], [0, 0, 0]),
    ("gumowski-mira", [14, 0, 0], [0, 0, 0]),
    ("halvorsen", [4, 0, 0], [12, 0, 0]),
    ("harmonics", [5, 4, 0], [0, 0, 0]),
    ("henon", [4, 0, 0], [0, 0, 0]),
    ("henon-heiles", [10, 0, 0], [0, 0, 0]),
    ("hilbert", [6, 0, 0], [0, 0, 0]),
    ("hippopede", [0, 0, 0], [4, 0, 0]),
    ("hofstadter-q", [10, 0, 0], [0, 0, 0]),
    ("hopalong", [12, 0, 0], [0, 0, 0]),
    ("ikeda", [8, 0, 0], [0, 0, 0]),
    ("interference", [21, 0, 0], [0, 0, 0]),
    ("julia", [0, 4, 0], [0, 0, 0]),
    ("julia-filled", [4, 0, 0], [0, 0, 0]),
    ("lambda-map", [4, 0, 0], [0, 0, 0]),
    ("langtons-ant", [8, 0, 0], [0, 0, 0]),
    ("logistic-cobweb", [10, 0, 0], [0, 0, 0]),
    ("logistic-map", [5, 4, 0], [4, 0, 0]),
    ("lorenz", [7, 15, 14], [0, 0, 0]),
    ("lucky-numbers", [6, 0, 0], [0, 0, 0]),
    ("mackey-glass", [4, 0, 0], [0, 0, 0]),
    ("magnet-fractal", [4, 0, 0], [0, 0, 0]),
    ("mandelbrot", [0, 0, 4], [0, 0, 0]),
    ("markov-chain", [4, 0, 0], [0, 0, 0]),
    ("menagerie", [10, 5, 4], [0, 0, 0]),
    ("menger-sponge", [5, 0, 0], [0, 0, 0]),
    ("minkowski-sausage", [4, 0, 0], [0, 0, 0]),
    ("multibrot", [6, 0, 0], [0, 0, 0]),
    ("mutual-info", [0, 0, 0], [4, 0, 0]),
    ("newton", [6, 0, 0], [0, 0, 0]),
    ("newton-cubic", [6, 0, 0], [0, 0, 0]),
    ("nova", [6, 0, 0], [0, 0, 0]),
    ("peano-curve", [4, 0, 0], [0, 0, 0]),
    ("phantom-jam", [0, 0, 0], [8, 0, 0]),
    ("phoenix", [4, 0, 0], [0, 0, 0]),
    ("pickover", [4, 0, 0], [0, 0, 0]),
    ("polarization", [4, 0, 0], [0, 0, 0]),
    ("pursuit", [0, 0, 0], [5, 0, 0]),
    ("quadratic-map", [6, 0, 0], [0, 0, 0]),
    ("rabinovich-fabrikant", [6, 0, 0], [0, 0, 0]),
    ("random-walk", [7, 0, 0], [0, 0, 0]),
    ("ripple", [5, 0, 0], [0, 0, 0]),
    ("rose", [0, 0, 0], [4, 0, 0]),
    ("rossler", [6, 0, 0], [0, 0, 0]),
    ("rule-110", [4, 0, 0], [0, 0, 0]),
    ("rule-30", [30, 0, 0], [0, 0, 0]),
    ("runge", [0, 0, 0], [6, 0, 0]),
    ("sandpile", [5, 4, 6], [0, 0, 0]),
    ("sierpinski-arrowhead", [4, 0, 0], [0, 0, 0]),
    ("sierpinski-carpet", [4, 0, 0], [0, 0, 0]),
    ("sierpinski-tri", [4, 0, 0], [0, 0, 0]),
    ("sieve", [12, 0, 0], [0, 0, 0]),
    ("sinai-billiard", [7, 0, 0], [0, 0, 0]),
    ("sir", [0, 0, 0], [4, 0, 0]),
    ("sprott", [8, 0, 0], [0, 0, 0]),
    ("tent-map", [4, 0, 0], [0, 0, 0]),
    ("the-magnet", [8, 0, 0], [0, 0, 0]),
    ("thomas", [14, 0, 0], [0, 0, 0]),
    ("times-tables", [6, 0, 0], [0, 0, 0]),
    ("tricorn", [4, 0, 0], [0, 0, 0]),
    ("vicsek", [4, 0, 0], [0, 0, 0]),
    ("zeckendorf", [6, 0, 0], [0, 0, 0]),
];

#[cfg(test)]
mod tests {
    use super::KNOWN_OVER_INTERACTION_FLASHES;

    #[test]
    fn room_lookup_lists_each_over_budget_room_once() {
        for pair in KNOWN_OVER_INTERACTION_FLASHES.windows(2) {
            assert!(pair[0].0 < pair[1].0, "room IDs must be sorted and unique");
        }
        for &(id, general, red) in KNOWN_OVER_INTERACTION_FLASHES {
            assert!(crate::room_meta_by_id(id).is_some(), "unknown room {id}");
            let mut peaks = general
                .into_iter()
                .chain(red)
                .filter(|&n| n != 0)
                .peekable();
            assert!(peaks.peek().is_some(), "no over-budget reading for {id}");
            assert!(
                peaks.all(|n| f64::from(n) > crate::photosensitivity::MAX_FLASHES_PER_SECOND),
                "lookup includes a reading within budget for {id}"
            );
        }
    }
}
