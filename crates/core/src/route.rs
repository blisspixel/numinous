//! Bounded street routes and exact comparisons for Route Lab.
//!
//! Roads are undirected with positive integer travel costs. A tour visits each
//! required stop and returns to its depot through shortest street paths, which
//! may pass other stops or revisit junctions. Diagram length is never a cost.

use std::fmt;

/// Maximum junction count in an admitted street network.
pub const MAX_ROUTE_JUNCTIONS: usize = 32;
/// Maximum undirected road count in an admitted street network.
pub const MAX_ROUTE_ROADS: usize = 96;
/// Maximum required stop count, including the depot.
pub const MAX_ROUTE_STOPS: usize = 10;
/// Largest admitted road cost, in travel units.
pub const MAX_ROUTE_ROAD_COST: u32 = 999;

/// One undirected street between two junction indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Road {
    /// First junction, in `0..junction_count`.
    pub from: usize,
    /// Second junction, different from the first.
    pub to: usize,
    /// Positive integer travel cost, independent of drawn length.
    pub cost: u32,
}

/// A refused problem, invalid tour, or unreachable destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    /// A problem exceeds the declared size bounds or has too few stops.
    Size,
    /// A junction index is outside the admitted map.
    Junction,
    /// A road loops, duplicates another road, or has an invalid cost.
    Road,
    /// Required stops are duplicated.
    DuplicateStop,
    /// Required stops have no open street path between them.
    Unreachable {
        /// Departure junction.
        from: usize,
        /// Destination junction.
        to: usize,
    },
    /// An order does not start at the depot and contain every stop exactly once.
    Tour,
    /// A checked cost sum or reconstructed path violated its bounds.
    Arithmetic,
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size => write!(
                f,
                "route requires 3..10 stops, at most 32 junctions and 96 roads"
            ),
            Self::Junction => write!(f, "route junction is outside the map"),
            Self::Road => write!(f, "roads must be distinct, non-looping, and cost 1..999"),
            Self::DuplicateStop => write!(f, "route stops must be distinct"),
            Self::Unreachable { from, to } => write!(f, "no open street path from {from} to {to}"),
            Self::Tour => write!(
                f,
                "delivery order must start at the depot and include each required stop once"
            ),
            Self::Arithmetic => write!(f, "route cost or reconstruction exceeds its bounds"),
        }
    }
}

impl std::error::Error for RouteError {}

/// A validated immutable street network and its required stops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteProblem {
    junctions: usize,
    roads: Vec<Road>,
    stops: Vec<usize>,
}

/// A real shortest-path solver decision, in computation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteEvent {
    /// A junction's minimum distance has been established.
    Settled {
        /// Junction index.
        junction: usize,
        /// Exact distance from the source.
        cost: u32,
    },
    /// An open road produced a strictly cheaper tentative distance.
    Relaxed {
        /// Predecessor junction.
        from: usize,
        /// Improved junction.
        to: usize,
        /// New tentative distance from the source.
        cost: u32,
    },
}

/// One shortest street path with a bounded deterministic calculation trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreetPath {
    /// Junctions in order, including both endpoints.
    pub junctions: Vec<usize>,
    /// Sum of the actual open roads along the path.
    pub cost: u32,
    /// Recorded settling and strict relaxation decisions.
    pub events: Vec<RouteEvent>,
}

/// A complete bounded street calculation, including an unreachable outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreetTrace {
    /// Actual settled and relaxed decisions, in computation order.
    pub events: Vec<RouteEvent>,
    /// Final path or an explicit unreachable diagnostic.
    pub result: Result<StreetPath, RouteError>,
}

/// Trace shortest-path search over structurally valid streets independently of
/// required tour stops. Empty and disconnected road sets remain meaningful:
/// unreachable destinations return a trace with an explicit failed result.
/// Invalid endpoints or malformed roads fail before search. Work and event
/// bounds are the same as [`RouteProblem::shortest_path`].
pub fn shortest_street_trace(
    junctions: usize,
    mut roads: Vec<Road>,
    from: usize,
    to: usize,
) -> Result<StreetTrace, RouteError> {
    validate_route_roads(junctions, &mut roads)?;
    if from >= junctions || to >= junctions {
        return Err(RouteError::Junction);
    }
    // This private search carrier is never exposed as an admitted tour
    // problem. Public RouteProblem constructors retain their stop guarantees.
    let search = RouteProblem {
        junctions,
        roads,
        stops: Vec::new(),
    };
    let (distances, previous, events) = search.dijkstra(from)?;
    let result = match distances[to] {
        Some(cost) => Ok(StreetPath {
            junctions: reconstruct_path(from, to, &previous)?,
            cost,
            events: events.clone(),
        }),
        None => Err(RouteError::Unreachable { from, to }),
    };
    Ok(StreetTrace { events, result })
}

pub(crate) fn validate_route_roads(junctions: usize, roads: &mut [Road]) -> Result<(), RouteError> {
    if !(3..=MAX_ROUTE_JUNCTIONS).contains(&junctions) || roads.len() > MAX_ROUTE_ROADS {
        return Err(RouteError::Size);
    }
    for road in roads.iter_mut() {
        if road.from >= junctions || road.to >= junctions {
            return Err(RouteError::Junction);
        }
        if road.from == road.to || !(1..=MAX_ROUTE_ROAD_COST).contains(&road.cost) {
            return Err(RouteError::Road);
        }
        if road.from > road.to {
            std::mem::swap(&mut road.from, &mut road.to);
        }
    }
    roads.sort_unstable_by_key(|road| (road.from, road.to));
    if roads
        .windows(2)
        .any(|pair| (pair[0].from, pair[0].to) == (pair[1].from, pair[1].to))
    {
        return Err(RouteError::Road);
    }
    Ok(())
}

pub(crate) fn validate_route_stops(
    junctions: usize,
    stops: &mut [usize],
    minimum: usize,
) -> Result<(), RouteError> {
    if stops.len() < minimum || stops.is_empty() || stops.len() > MAX_ROUTE_STOPS {
        return Err(RouteError::Size);
    }
    if stops.iter().any(|&stop| stop >= junctions) {
        return Err(RouteError::Junction);
    }
    let depot = stops[0];
    stops[1..].sort_unstable();
    if stops[1..].contains(&depot) || stops[1..].windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(RouteError::DuplicateStop);
    }
    Ok(())
}

impl RouteProblem {
    /// Validate and canonicalize a town. The first stop is the depot; the
    /// remaining stops and roads are sorted for stable identifier tie breaks.
    /// All required stops must be reachable from the depot. Unused isolated
    /// junctions are allowed. Input vectors are bounded before solver work.
    pub fn new(
        junctions: usize,
        mut roads: Vec<Road>,
        mut stops: Vec<usize>,
    ) -> Result<Self, RouteError> {
        if !(3..=MAX_ROUTE_JUNCTIONS).contains(&junctions)
            || !(3..=MAX_ROUTE_STOPS).contains(&stops.len())
            || roads.is_empty()
            || roads.len() > MAX_ROUTE_ROADS
        {
            return Err(RouteError::Size);
        }
        validate_route_roads(junctions, &mut roads)?;
        validate_route_stops(junctions, &mut stops, 3)?;
        let depot = stops[0];
        let problem = Self {
            junctions,
            roads,
            stops,
        };
        let distances = problem.dijkstra(depot)?.0;
        for &stop in &problem.stops {
            if distances[stop].is_none() {
                return Err(RouteError::Unreachable {
                    from: depot,
                    to: stop,
                });
            }
        }
        Ok(problem)
    }

    /// Admitted junction count.
    #[must_use]
    pub const fn junction_count(&self) -> usize {
        self.junctions
    }

    /// Canonical open streets.
    #[must_use]
    pub fn roads(&self) -> &[Road] {
        &self.roads
    }

    /// Depot first, then required deliveries in identifier order.
    #[must_use]
    pub fn stops(&self) -> &[usize] {
        &self.stops
    }

    fn dijkstra(&self, source: usize) -> Result<DijkstraResult, RouteError> {
        if source >= self.junctions {
            return Err(RouteError::Junction);
        }
        let mut distances: Vec<Option<u32>> = vec![None; self.junctions];
        let mut previous = vec![None; self.junctions];
        let mut settled = vec![false; self.junctions];
        let mut events = Vec::new();
        distances[source] = Some(0);
        for _ in 0..self.junctions {
            let next = (0..self.junctions)
                .filter(|&node| !settled[node])
                .filter_map(|node| distances[node].map(|cost| (cost, node)))
                .min();
            let Some((cost, node)) = next else {
                break;
            };
            settled[node] = true;
            events.push(RouteEvent::Settled {
                junction: node,
                cost,
            });
            for road in &self.roads {
                let neighbor = if road.from == node {
                    road.to
                } else if road.to == node {
                    road.from
                } else {
                    continue;
                };
                if settled[neighbor] {
                    continue;
                }
                let candidate = cost.checked_add(road.cost).ok_or(RouteError::Arithmetic)?;
                if distances[neighbor].is_none_or(|old| candidate < old) {
                    distances[neighbor] = Some(candidate);
                    previous[neighbor] = Some(node);
                    events.push(RouteEvent::Relaxed {
                        from: node,
                        to: neighbor,
                        cost: candidate,
                    });
                }
            }
        }
        Ok((distances, previous, events))
    }

    /// Compute one shortest street path. Equal-distance junctions settle by
    /// ascending identifier; equal alternatives retain the first predecessor.
    /// The trace has at most `junctions + 2*roads` events and invents none.
    pub fn shortest_path(&self, from: usize, to: usize) -> Result<StreetPath, RouteError> {
        if to >= self.junctions {
            return Err(RouteError::Junction);
        }
        let (distances, previous, events) = self.dijkstra(from)?;
        let cost = distances[to].ok_or(RouteError::Unreachable { from, to })?;
        let junctions = reconstruct_path(from, to, &previous)?;
        Ok(StreetPath {
            junctions,
            cost,
            events,
        })
    }

    /// Compute the metric closure for this exact immutable town. Tours and
    /// comparisons are reconstructed against these roads, never drawn length.
    pub fn metric(&self) -> Result<RouteMetric, RouteError> {
        let n = self.stops.len();
        let mut distances = vec![vec![0; n]; n];
        let mut paths = vec![vec![Vec::new(); n]; n];
        for (i, &source) in self.stops.iter().enumerate() {
            let (row, previous, _) = self.dijkstra(source)?;
            for (j, &destination) in self.stops.iter().enumerate() {
                distances[i][j] = row[destination].ok_or(RouteError::Unreachable {
                    from: source,
                    to: destination,
                })?;
                paths[i][j] = reconstruct_path(source, destination, &previous)?;
            }
        }
        Ok(RouteMetric {
            problem: self.clone(),
            distances,
            paths,
        })
    }
}

type DijkstraResult = (Vec<Option<u32>>, Vec<Option<usize>>, Vec<RouteEvent>);

fn reconstruct_path(
    from: usize,
    to: usize,
    previous: &[Option<usize>],
) -> Result<Vec<usize>, RouteError> {
    let mut path = vec![to];
    let mut cursor = to;
    while cursor != from {
        cursor = previous
            .get(cursor)
            .copied()
            .flatten()
            .ok_or(RouteError::Arithmetic)?;
        path.push(cursor);
        if path.len() > previous.len() {
            return Err(RouteError::Arithmetic);
        }
    }
    path.reverse();
    Ok(path)
}

/// A checked stop order and its expanded closed street walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTour {
    /// Depot first, every delivery once, without a duplicate final depot.
    pub order: Vec<usize>,
    /// Exact closed cost, including the return to the depot.
    pub cost: u32,
    /// Actual street junctions, starting and ending at the depot.
    pub walk: Vec<usize>,
}

/// A possible exchange of two nonadjacent tour edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteExchange {
    /// Index of the first replaced edge's departure in the stop order.
    pub first: usize,
    /// Index of the second replaced edge's departure in the stop order.
    pub second: usize,
    /// New cost minus old cost; a negative value is an improvement.
    pub delta: i64,
    /// The independently re-evaluated tour after reversing the segment.
    pub tour: RouteTour,
}

/// An exhaustive subset dynamic-programming comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactRoute {
    /// A globally cheapest tour for this town's declared integer costs.
    pub tour: RouteTour,
    /// Number of completed reachable subset/end-stop states, not tours searched.
    pub states_completed: usize,
}

/// Shortest-path distances and street reconstructions bound to one town.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMetric {
    problem: RouteProblem,
    distances: Vec<Vec<u32>>,
    paths: Vec<Vec<Vec<usize>>>,
}

impl RouteMetric {
    /// The exact town whose roads and costs this comparison uses.
    #[must_use]
    pub fn problem(&self) -> &RouteProblem {
        &self.problem
    }

    /// Symmetric metric distances, in [`RouteProblem::stops`] order.
    #[must_use]
    pub fn distances(&self) -> &[Vec<u32>] {
        &self.distances
    }

    fn order_indices(&self, order: &[usize]) -> Result<Vec<usize>, RouteError> {
        let stops = self.problem.stops();
        if order.len() != stops.len() || order.first() != stops.first() {
            return Err(RouteError::Tour);
        }
        let mut indices = Vec::with_capacity(order.len());
        for node in order {
            let index = stops
                .iter()
                .position(|stop| stop == node)
                .ok_or(RouteError::Tour)?;
            if indices.contains(&index) {
                return Err(RouteError::Tour);
            }
            indices.push(index);
        }
        Ok(indices)
    }

    /// Validate a proposed order, include its return leg, and expand every leg
    /// through real open streets. A repeated delivery is never silently skipped.
    pub fn evaluate(&self, order: &[usize]) -> Result<RouteTour, RouteError> {
        let indices = self.order_indices(order)?;
        let mut cost = 0u32;
        let mut walk = vec![order[0]];
        for i in 0..indices.len() {
            let from = indices[i];
            let to = indices[(i + 1) % indices.len()];
            cost = cost
                .checked_add(self.distances[from][to])
                .ok_or(RouteError::Arithmetic)?;
            walk.extend_from_slice(&self.paths[from][to][1..]);
        }
        Ok(RouteTour {
            order: order.to_vec(),
            cost,
            walk,
        })
    }

    /// Form the nearest-neighbor heuristic, resolving equal leg costs by
    /// junction identifier. Its result is feasible, not certified optimal.
    pub fn nearest_neighbor(&self) -> Result<RouteTour, RouteError> {
        let mut indices = vec![0];
        let mut current = 0;
        while indices.len() < self.problem.stops.len() {
            let next = (1..self.problem.stops.len())
                .filter(|node| !indices.contains(node))
                .min_by_key(|&node| (self.distances[current][node], self.problem.stops[node]))
                .ok_or(RouteError::Tour)?;
            indices.push(next);
            current = next;
        }
        let order: Vec<_> = indices
            .into_iter()
            .map(|index| self.problem.stops[index])
            .collect();
        self.evaluate(&order)
    }

    /// Examine every nonadjacent two-edge exchange in deterministic order.
    /// Reversals are valid because roads are undirected. Each exact delta is
    /// checked against full closed-tour recomputation. At most 35 proposals
    /// exist under the ten-stop bound.
    pub fn exchanges(&self, order: &[usize]) -> Result<Vec<RouteExchange>, RouteError> {
        let indices = self.order_indices(order)?;
        let original = self.evaluate(order)?;
        let n = order.len();
        let mut exchanges = Vec::new();
        for first in 0..n {
            for second in first + 2..n {
                if first == 0 && second == n - 1 {
                    continue;
                }
                let (a, b, c, d) = (
                    indices[first],
                    indices[(first + 1) % n],
                    indices[second],
                    indices[(second + 1) % n],
                );
                let delta = i64::from(self.distances[a][c]) + i64::from(self.distances[b][d])
                    - i64::from(self.distances[a][b])
                    - i64::from(self.distances[c][d]);
                let mut replacement = order.to_vec();
                replacement[first + 1..=second].reverse();
                let tour = self.evaluate(&replacement)?;
                if i64::from(tour.cost) - i64::from(original.cost) != delta {
                    return Err(RouteError::Arithmetic);
                }
                exchanges.push(RouteExchange {
                    first,
                    second,
                    delta,
                    tour,
                });
            }
        }
        Ok(exchanges)
    }

    /// Return the best strictly improving exchange, or none after a complete
    /// pass. None establishes two-edge local optimality, not global optimality.
    pub fn best_exchange(&self, order: &[usize]) -> Result<Option<RouteExchange>, RouteError> {
        Ok(self
            .exchanges(order)?
            .into_iter()
            .filter(|exchange| exchange.delta < 0)
            .min_by_key(|exchange| (exchange.delta, exchange.first, exchange.second)))
    }

    /// Certify the minimum closed tour by subset dynamic programming, with
    /// the depot fixed first. The maximum table is 512 by 9 states. Every
    /// non-depot stop belongs to exactly one bit; costs use checked arithmetic.
    pub fn exact(&self) -> Result<ExactRoute, RouteError> {
        let m = self.problem.stops.len() - 1;
        let size = 1usize << m;
        let mut costs: Vec<Vec<Option<u32>>> = vec![vec![None; m]; size];
        let mut previous = vec![vec![None; m]; size];
        let mut states_completed = 0;
        for subset in 1..size {
            for last in 0..m {
                let bit = 1 << last;
                if subset & bit == 0 {
                    continue;
                }
                let before = subset ^ bit;
                let best = if before == 0 {
                    Some((self.distances[0][last + 1], None))
                } else {
                    let mut best: Option<(u32, Option<usize>)> = None;
                    for (predecessor, prefix) in costs[before].iter().copied().enumerate() {
                        if before & (1 << predecessor) == 0 {
                            continue;
                        }
                        let Some(prefix) = prefix else {
                            continue;
                        };
                        let candidate = prefix
                            .checked_add(self.distances[predecessor + 1][last + 1])
                            .ok_or(RouteError::Arithmetic)?;
                        if best.is_none_or(|old| candidate < old.0) {
                            best = Some((candidate, Some(predecessor)));
                        }
                    }
                    best
                };
                if let Some((cost, predecessor)) = best {
                    costs[subset][last] = Some(cost);
                    previous[subset][last] = predecessor;
                    states_completed += 1;
                }
            }
        }
        let all = size - 1;
        let mut finish: Option<(u32, usize)> = None;
        for (last, prefix) in costs[all].iter().copied().enumerate() {
            let prefix = prefix.ok_or(RouteError::Arithmetic)?;
            let candidate = prefix
                .checked_add(self.distances[last + 1][0])
                .ok_or(RouteError::Arithmetic)?;
            if finish.is_none_or(|old| candidate < old.0) {
                finish = Some((candidate, last));
            }
        }
        let (cost, mut last) = finish.ok_or(RouteError::Arithmetic)?;
        let mut subset = all;
        let mut order = Vec::with_capacity(m + 1);
        while subset != 0 {
            order.push(self.problem.stops[last + 1]);
            let predecessor = previous[subset][last];
            subset ^= 1 << last;
            if subset != 0 {
                last = predecessor.ok_or(RouteError::Arithmetic)?;
            }
        }
        order.push(self.problem.stops[0]);
        order.reverse();
        let tour = self.evaluate(&order)?;
        if tour.cost != cost {
            return Err(RouteError::Arithmetic);
        }
        Ok(ExactRoute {
            tour,
            states_completed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn fixture() -> RouteProblem {
        RouteProblem::new(
            4,
            vec![
                Road {
                    from: 0,
                    to: 1,
                    cost: 1,
                },
                Road {
                    from: 0,
                    to: 2,
                    cost: 2,
                },
                Road {
                    from: 1,
                    to: 2,
                    cost: 2,
                },
                Road {
                    from: 1,
                    to: 3,
                    cost: 3,
                },
                Road {
                    from: 2,
                    to: 3,
                    cost: 2,
                },
            ],
            vec![0, 1, 2, 3],
        )
        .unwrap()
    }

    fn all_pairs(problem: &RouteProblem) -> Vec<Vec<u32>> {
        let n = problem.junction_count();
        let mut distances = vec![vec![u32::MAX / 4; n]; n];
        for (i, row) in distances.iter_mut().enumerate() {
            row[i] = 0;
        }
        for road in problem.roads() {
            distances[road.from][road.to] = road.cost;
            distances[road.to][road.from] = road.cost;
        }
        for middle in 0..n {
            for from in 0..n {
                for to in 0..n {
                    distances[from][to] =
                        distances[from][to].min(distances[from][middle] + distances[middle][to]);
                }
            }
        }
        distances
    }

    fn enumerate(
        deliveries: &mut [usize],
        prefix: usize,
        depot: usize,
        distances: &[Vec<u32>],
        best: &mut u32,
    ) {
        if prefix == deliveries.len() {
            let mut cost = distances[depot][deliveries[0]];
            for pair in deliveries.windows(2) {
                cost += distances[pair[0]][pair[1]];
            }
            cost += distances[deliveries[deliveries.len() - 1]][depot];
            *best = (*best).min(cost);
        } else {
            for index in prefix..deliveries.len() {
                deliveries.swap(prefix, index);
                enumerate(deliveries, prefix + 1, depot, distances, best);
                deliveries.swap(prefix, index);
            }
        }
    }

    fn exhaustive(problem: &RouteProblem, distances: &[Vec<u32>]) -> u32 {
        let mut best = u32::MAX;
        enumerate(
            &mut problem.stops()[1..].to_vec(),
            0,
            problem.stops()[0],
            distances,
            &mut best,
        );
        best
    }

    fn assert_street_cost(problem: &RouteProblem, walk: &[usize], expected: u32) {
        let actual: u32 = walk
            .windows(2)
            .map(|pair| {
                problem
                    .roads()
                    .iter()
                    .find(|road| {
                        (road.from == pair[0] && road.to == pair[1])
                            || (road.to == pair[0] && road.from == pair[1])
                    })
                    .expect("every expanded step is an actual open road")
                    .cost
            })
            .sum();
        assert_eq!(actual, expected);
    }

    #[test]
    fn documented_town_has_the_complete_three_tour_comparison() {
        let problem = fixture();
        let metric = problem.metric().unwrap();
        assert_eq!(
            metric.distances(),
            &[
                vec![0, 1, 2, 4],
                vec![1, 0, 2, 3],
                vec![2, 2, 0, 2],
                vec![4, 3, 2, 0]
            ]
        );
        for (order, expected) in [([0, 1, 2, 3], 9), ([0, 1, 3, 2], 8), ([0, 2, 1, 3], 11)] {
            let tour = metric.evaluate(&order).unwrap();
            assert_eq!(tour.cost, expected);
            assert_eq!(tour.walk.first(), Some(&0));
            assert_eq!(tour.walk.last(), Some(&0));
            assert_street_cost(&problem, &tour.walk, expected);
        }
        assert_eq!(metric.nearest_neighbor().unwrap().order, vec![0, 1, 2, 3]);
        let exchange = metric.best_exchange(&[0, 1, 2, 3]).unwrap().unwrap();
        assert_eq!(
            (exchange.first, exchange.second, exchange.delta),
            (1, 3, -1)
        );
        assert_eq!(exchange.tour.order, vec![0, 1, 3, 2]);
        assert!(
            metric
                .best_exchange(&exchange.tour.order)
                .unwrap()
                .is_none()
        );
        let exact = metric.exact().unwrap();
        assert_eq!(exact.tour.cost, 8);
        assert_eq!(exact.states_completed, 12);
        assert_eq!(metric, problem.metric().unwrap());
        assert_eq!(exact, metric.exact().unwrap());
    }

    #[test]
    fn independent_all_pairs_and_permutations_check_seeded_towns() {
        let mut rng = SplitMix64::new(91);
        for n in 3..=7 {
            for _ in 0..12 {
                // A chain ensures connectivity; optional other roads introduce
                // alternative paths, ties, and metric shortcuts.
                let mut roads = Vec::new();
                for from in 0..n {
                    for to in from + 1..n {
                        if to == from + 1 || rng.below(3) != 0 {
                            roads.push(Road {
                                from,
                                to,
                                cost: 1 + rng.below(30) as u32,
                            });
                        }
                    }
                }
                let depot = rng.below(n as u64) as usize;
                let mut stops = vec![depot];
                stops.extend((0..n).filter(|&node| node != depot));
                let problem = RouteProblem::new(n, roads, stops).unwrap();
                let oracle = all_pairs(&problem);
                for (from, row) in oracle.iter().enumerate() {
                    for (to, &cost) in row.iter().enumerate() {
                        let path = problem.shortest_path(from, to).unwrap();
                        assert_eq!(path.cost, cost);
                        assert_street_cost(&problem, &path.junctions, cost);
                        assert!(
                            path.events.len()
                                <= problem.junction_count() + 2 * problem.roads().len()
                        );
                        assert_eq!(path, problem.shortest_path(from, to).unwrap());
                    }
                }
                let metric = problem.metric().unwrap();
                let exact = metric.exact().unwrap();
                assert_eq!(exact.tour.cost, exhaustive(&problem, &oracle));
                assert_street_cost(&problem, &exact.tour.walk, exact.tour.cost);
                assert_eq!(exact.states_completed, (n - 1) * (1usize << (n - 2)));
                let mut current = metric.nearest_neighbor().unwrap();
                for exchange in metric.exchanges(&current.order).unwrap() {
                    assert_eq!(
                        i64::from(exchange.tour.cost) - i64::from(current.cost),
                        exchange.delta
                    );
                }
                while let Some(exchange) = metric.best_exchange(&current.order).unwrap() {
                    assert!(exchange.tour.cost < current.cost);
                    current = exchange.tour;
                }
                assert!(current.cost >= exact.tour.cost);
            }
        }
    }

    #[test]
    fn two_edge_local_optimum_is_not_a_global_certificate() {
        let roads = [
            (1, 0, 20),
            (2, 0, 7),
            (2, 1, 15),
            (3, 0, 17),
            (3, 1, 16),
            (3, 2, 17),
            (4, 0, 12),
            (4, 1, 15),
            (4, 2, 1),
            (4, 3, 2),
            (5, 0, 6),
            (5, 1, 12),
            (5, 2, 5),
            (5, 3, 9),
            (5, 4, 2),
        ]
        .into_iter()
        .map(|(from, to, cost)| Road { from, to, cost })
        .collect();
        let problem = RouteProblem::new(6, roads, vec![0, 1, 2, 3, 4, 5]).unwrap();
        let metric = problem.metric().unwrap();
        let order = [0, 5, 4, 2, 3, 1];
        assert_eq!(metric.evaluate(&order).unwrap().cost, 46);
        assert!(metric.best_exchange(&order).unwrap().is_none());
        assert_eq!(metric.exact().unwrap().tour.cost, 44);
        assert_eq!(exhaustive(&problem, &all_pairs(&problem)), 44);
    }

    #[test]
    fn positive_scaling_relabeling_and_changed_roads_preserve_the_right_claims() {
        let original = fixture();
        let scale = RouteProblem::new(
            4,
            original
                .roads()
                .iter()
                .map(|road| Road {
                    cost: road.cost * 7,
                    ..*road
                })
                .collect(),
            original.stops().to_vec(),
        )
        .unwrap();
        assert_eq!(scale.metric().unwrap().exact().unwrap().tour.cost, 56);
        let permutation = [3, 1, 0, 2];
        let relabeled = RouteProblem::new(
            4,
            original
                .roads()
                .iter()
                .map(|road| Road {
                    from: permutation[road.from],
                    to: permutation[road.to],
                    cost: road.cost,
                })
                .collect(),
            original.stops().iter().map(|&id| permutation[id]).collect(),
        )
        .unwrap();
        assert_eq!(relabeled.metric().unwrap().exact().unwrap().tour.cost, 8);
        let mut edited_roads = original.roads().to_vec();
        edited_roads
            .iter_mut()
            .find(|road| (road.from, road.to) == (1, 3))
            .unwrap()
            .cost = 9;
        let edited = RouteProblem::new(4, edited_roads, original.stops().to_vec()).unwrap();
        let metric = edited.metric().unwrap();
        assert_eq!(
            metric.nearest_neighbor().unwrap().cost,
            metric.exact().unwrap().tour.cost
        );
        assert_eq!(metric.evaluate(&[0, 1, 3, 2]).unwrap().cost, 9);
        assert_eq!(
            original
                .metric()
                .unwrap()
                .evaluate(&[0, 1, 3, 2])
                .unwrap()
                .cost,
            8
        );
        let closed = RouteProblem::new(
            4,
            original
                .roads()
                .iter()
                .copied()
                .filter(|road| (road.from, road.to) != (1, 3))
                .collect(),
            original.stops().to_vec(),
        )
        .unwrap();
        assert_eq!(closed.shortest_path(1, 3).unwrap().cost, 4);
    }

    #[test]
    fn ties_and_input_order_have_deterministic_traces() {
        let roads = vec![
            Road {
                from: 0,
                to: 1,
                cost: 1,
            },
            Road {
                from: 0,
                to: 2,
                cost: 1,
            },
            Road {
                from: 1,
                to: 3,
                cost: 1,
            },
            Road {
                from: 2,
                to: 3,
                cost: 1,
            },
        ];
        let first = RouteProblem::new(4, roads.clone(), vec![0, 3, 2, 1]).unwrap();
        let mut reversed = roads;
        reversed.reverse();
        let second = RouteProblem::new(4, reversed, vec![0, 1, 3, 2]).unwrap();
        assert_eq!(first, second);
        let path = first.shortest_path(0, 3).unwrap();
        assert_eq!(path.junctions, vec![0, 1, 3]);
        assert_eq!(path, second.shortest_path(0, 3).unwrap());
        assert_eq!(
            first.metric().unwrap().nearest_neighbor().unwrap().order,
            vec![0, 1, 3, 2]
        );
    }

    #[test]
    fn malformed_and_disconnected_inputs_are_refused_before_comparison() {
        let fixture = fixture();
        for junctions in [0, 2, MAX_ROUTE_JUNCTIONS + 1, usize::MAX] {
            assert_eq!(
                RouteProblem::new(junctions, fixture.roads().to_vec(), vec![0, 1, 2]),
                Err(RouteError::Size)
            );
        }
        for stops in [
            vec![0, 1],
            vec![0, 1, 1],
            vec![0, 1, 9],
            vec![0; MAX_ROUTE_STOPS + 1],
        ] {
            assert!(RouteProblem::new(4, fixture.roads().to_vec(), stops).is_err());
        }
        for road in [
            Road {
                from: 0,
                to: 0,
                cost: 1,
            },
            Road {
                from: 0,
                to: 1,
                cost: 0,
            },
            Road {
                from: 0,
                to: 1,
                cost: 1000,
            },
            Road {
                from: 0,
                to: 9,
                cost: 1,
            },
        ] {
            let mut roads = fixture.roads().to_vec();
            roads.push(road);
            assert!(RouteProblem::new(4, roads, vec![0, 1, 2, 3]).is_err());
        }
        let mut roads = fixture.roads().to_vec();
        roads.push(Road {
            from: 1,
            to: 0,
            cost: 1,
        });
        assert_eq!(
            RouteProblem::new(4, roads, vec![0, 1, 2, 3]),
            Err(RouteError::Road)
        );
        assert_eq!(
            RouteProblem::new(
                4,
                vec![Road {
                    from: 0,
                    to: 1,
                    cost: 1
                }],
                vec![0, 1, 2]
            ),
            Err(RouteError::Unreachable { from: 0, to: 2 })
        );
        assert_eq!(
            RouteProblem::new(
                4,
                vec![
                    Road {
                        from: 0,
                        to: 1,
                        cost: 1
                    };
                    MAX_ROUTE_ROADS + 1
                ],
                vec![0, 1, 2]
            ),
            Err(RouteError::Size)
        );
        let metric = fixture.metric().unwrap();
        for order in [&[0, 1, 2][..], &[1, 0, 2, 3], &[0, 1, 2, 2], &[0, 1, 2, 99]] {
            assert_eq!(metric.evaluate(order), Err(RouteError::Tour));
            assert_eq!(metric.exchanges(order), Err(RouteError::Tour));
        }
        assert_eq!(fixture.shortest_path(99, 0), Err(RouteError::Junction));
        assert_eq!(fixture.shortest_path(0, 99), Err(RouteError::Junction));
        let isolated = RouteProblem::new(5, fixture.roads().to_vec(), vec![0, 1, 2, 3]).unwrap();
        assert_eq!(
            isolated.shortest_path(0, 4),
            Err(RouteError::Unreachable { from: 0, to: 4 })
        );
    }

    #[test]
    fn maximum_sized_problem_has_bounded_work_and_real_reconstruction() {
        let mut roads = Vec::new();
        for node in 1..MAX_ROUTE_JUNCTIONS {
            roads.push(Road {
                from: node - 1,
                to: node,
                cost: 999,
            });
        }
        'outer: for from in 0..MAX_ROUTE_JUNCTIONS {
            for to in from + 2..MAX_ROUTE_JUNCTIONS {
                roads.push(Road {
                    from,
                    to,
                    cost: 999,
                });
                if roads.len() == MAX_ROUTE_ROADS {
                    break 'outer;
                }
            }
        }
        let problem =
            RouteProblem::new(32, roads, vec![0, 3, 6, 9, 12, 15, 18, 21, 24, 31]).unwrap();
        let metric = problem.metric().unwrap();
        let exact = metric.exact().unwrap();
        assert_eq!(exact.states_completed, 2304);
        assert!(exact.tour.walk.len() <= MAX_ROUTE_STOPS * (MAX_ROUTE_JUNCTIONS - 1) + 1);
        assert_street_cost(&problem, &exact.tour.walk, exact.tour.cost);
        assert_eq!(metric.exchanges(&exact.tour.order).unwrap().len(), 35);
        assert!(problem.shortest_path(0, 31).unwrap().events.len() <= 32 + 2 * 96);
    }
}
