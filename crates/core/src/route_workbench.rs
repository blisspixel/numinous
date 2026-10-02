//! Editable street networks with bounded undo and caller-paced search playback.
//!
//! A network can remain structurally valid while its deliveries are unreachable
//! or too few for a tour. Edits retain that question and return diagnostics at
//! comparison time. Snapshots carry player state, not imported mathematical
//! proof: any imported trace is regenerated against the supplied current roads.

use std::fmt;

use crate::route::{
    ExactRoute, Road, RouteError, RouteEvent, RouteExchange, RouteProblem, RouteTour, StreetPath,
    StreetTrace, shortest_street_trace, validate_route_roads, validate_route_stops,
};

/// Maximum retained previous towns, oldest first.
pub const MAX_ROUTE_UNDO: usize = 16;

/// Maximum junctions in generated practice maps. Their complete graph fits
/// comfortably inside the ordinary workbench's road bound.
pub const MAX_RANDOM_ROUTE_JUNCTIONS: usize = 12;

/// Bounded options for a reproducible connected practice map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteRandomMapOptions {
    /// Junction count, in 3..=12.
    pub junctions: usize,
    /// Extra connections after the random spanning tree, sampled without replacement.
    pub extra_roads: usize,
    /// Inclusive upper cost bound, in 1..=999; every road costs at least one.
    pub max_cost: u32,
}

impl Default for RouteRandomMapOptions {
    fn default() -> Self {
        Self {
            junctions: 12,
            extra_roads: 8,
            max_cost: 99,
        }
    }
}

/// One existing road with its explicit open or closed state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditableRoad {
    /// Validated endpoints and positive integer travel cost, retained on closure.
    pub road: Road,
    /// Whether this road is available to shortest-path search.
    pub open: bool,
}

/// A bounded editable town and the player's proposed service order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTownSnapshot {
    /// Junction identifiers are `0..junctions`, with 3..32 junctions.
    pub junctions: usize,
    /// At most 96 distinct roads, including closed roads.
    pub roads: Vec<EditableRoad>,
    /// Depot first, then distinct required deliveries, with 1..10 stops.
    pub stops: Vec<usize>,
    /// Depot first, containing every required stop exactly once.
    pub order: Vec<usize>,
}

/// A cursor into a freshly regenerated trace for one snapshot revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteTraceSnapshot {
    /// Current workbench revision; an earlier revision is refused.
    pub revision: u64,
    /// SHA-256 of canonical current town state. Detects inconsistent snapshots,
    /// not authorship or historical authenticity; callers may recompute it.
    pub town_identity: [u8; 32],
    /// Search source junction.
    pub from: usize,
    /// Search destination junction.
    pub to: usize,
    /// Number of actual solver events already revealed.
    pub cursor: usize,
}

/// Caller-carried bounded editing state, including explicit undo and playback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteWorkbenchSnapshot {
    /// Monotonic local revision, incremented by a changed edit or undo.
    pub revision: u64,
    /// Current town, which may lack a feasible tour.
    pub current: RouteTownSnapshot,
    /// Previous towns, oldest first, bounded by [`MAX_ROUTE_UNDO`].
    pub undo: Vec<RouteTownSnapshot>,
    /// Optional playback cursor; events and path claims are never imported.
    pub trace: Option<RouteTraceSnapshot>,
}

/// An edit or a deliberate route construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteEdit {
    /// Replace the authored network, required stops, and delivery order as one
    /// validated transaction. Structural editing keeps the same bounded undo
    /// and revision contract as a cost or order change.
    Network(RouteTownSnapshot),
    /// Change an existing road's retained cost, including a closed road.
    RoadCost {
        /// One endpoint; endpoint order is immaterial.
        from: usize,
        /// Other endpoint.
        to: usize,
        /// New positive cost in 1..999.
        cost: u32,
    },
    /// Close or reopen an existing road without replacing its cost.
    RoadOpen {
        /// One endpoint; endpoint order is immaterial.
        from: usize,
        /// Other endpoint.
        to: usize,
        /// New availability.
        open: bool,
    },
    /// Replace the required stops, depot first. Existing surviving deliveries
    /// retain relative service order; new deliveries append by identifier.
    RequiredStops(Vec<usize>),
    /// Choose an existing required stop as depot, rotating the current order.
    Depot(usize),
    /// Replace the player's service order with a validated permutation.
    Order(Vec<usize>),
    /// Deliberately replace the player order with nearest-neighbor's candidate.
    Greedy,
    /// Accept the best strictly improving two-edge exchange, if one exists.
    Improve,
}

/// An invalid structural edit, unavailable comparison, or refused playback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteWorkbenchError {
    /// Malformed town, invalid order, or an infeasible requested construction.
    Invalid(RouteError),
    /// The endpoints do not identify an existing road.
    MissingRoad {
        /// Requested first endpoint.
        from: usize,
        /// Requested second endpoint.
        to: usize,
    },
    /// A requested depot is not among the current required stops.
    DepotNotRequired(usize),
    /// Imported undo exceeds its bound or cannot fit the supplied revision.
    UndoLimit,
    /// No next revision can be represented.
    Revision,
    /// Playback has not been started, or was invalidated by an edit.
    TraceAbsent,
    /// A cursor names a different revision from its current town.
    TraceRevision,
    /// A cursor's canonical town identity differs from its supplied current town.
    TraceIdentity,
    /// A cursor exceeds the actual regenerated event count.
    TraceCursor,
    /// Random-map options exceed the declared junction, connection, or cost bounds.
    RandomOptions,
}

impl fmt::Display for RouteWorkbenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(f, "{error}"),
            Self::MissingRoad { from, to } => write!(f, "no road between {from} and {to}"),
            Self::DepotNotRequired(junction) => {
                write!(f, "depot {junction} is not a required stop")
            }
            Self::UndoLimit => write!(f, "route undo history exceeds its bound or revision"),
            Self::Revision => write!(f, "route revision cannot advance"),
            Self::TraceAbsent => write!(f, "no current route trace; start a new calculation"),
            Self::TraceRevision => write!(f, "route trace belongs to a different revision"),
            Self::TraceIdentity => {
                write!(f, "search playback belongs to a different street network")
            }
            Self::TraceCursor => write!(f, "route cursor exceeds the recorded calculation"),
            Self::RandomOptions => write!(
                f,
                "random map needs 3..12 junctions, 1..999 maximum cost, and no more extra roads than unused connections"
            ),
        }
    }
}

impl std::error::Error for RouteWorkbenchError {}

impl From<RouteError> for RouteWorkbenchError {
    fn from(error: RouteError) -> Self {
        Self::Invalid(error)
    }
}

/// Current candidate, heuristic, exact certificate, and optional saving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteComparison {
    /// Player order evaluated over current open streets, including its return.
    pub current: RouteTour,
    /// Nearest-neighbor's feasible candidate, not a certificate.
    pub greedy: RouteTour,
    /// Exact minimum for the current finite integer-cost town.
    pub exact: ExactRoute,
    /// Best strictly improving exchange from the player's order.
    pub proposal: Option<RouteExchange>,
}

/// What the revealed shortest-path decisions establish about one junction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteSearchState {
    /// No revealed road relaxation has reached this junction.
    Unseen,
    /// A distance is known, but its minimum has not yet been established.
    Tentative,
    /// A revealed settlement establishes the minimum distance from the source.
    Settled,
    /// The complete search establishes that this junction cannot be reached.
    Unreachable,
}

/// One junction's distance and predecessor established by a revealed prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSearchJunction {
    /// Junction identifier in the current street network.
    pub junction: usize,
    /// Distance from the source, absent when unseen or unreachable.
    pub cost: Option<u32>,
    /// Predecessor from the most recent strict improvement, absent at source.
    pub predecessor: Option<usize>,
    /// Whether the revealed decisions establish a tentative or final distance.
    pub state: RouteSearchState,
}

/// Search state inferred only from decisions the caller has already revealed.
///
/// Rows are in junction identifier order. At cursor zero the source alone has
/// tentative distance zero. Unseen rows become unreachable only when the full
/// search has been revealed. The final path remains separate in
/// [`RouteTracePlayback::result`], which withholds it until completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSearchView {
    /// Recorded search source, independent of a face's inspected junction.
    pub from: usize,
    /// Recorded destination, independent of a face's next target selection.
    pub to: usize,
    /// Number of recorded decisions the caller has revealed.
    pub cursor: usize,
    /// Total recorded decision count, without exposing their unseen contents.
    pub event_count: usize,
    /// Whether every recorded decision has been revealed.
    pub completed: bool,
    /// Current distance, predecessor, and state for every network junction.
    pub junctions: Vec<RouteSearchJunction>,
    /// Latest revealed decision, or none before the first step.
    pub active_event: Option<RouteEvent>,
}

/// A genuine recorded calculation whose presentation advances only on request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTracePlayback {
    snapshot: RouteTraceSnapshot,
    calculation: StreetTrace,
    junctions: usize,
}

impl RouteTracePlayback {
    /// Current revision, endpoints, and caller-controlled cursor.
    #[must_use]
    pub const fn snapshot(&self) -> RouteTraceSnapshot {
        self.snapshot
    }
    /// Every actual recorded decision, independent of presentation speed.
    #[must_use]
    pub fn events(&self) -> &[RouteEvent] {
        &self.calculation.events
    }
    /// Only the prefix already revealed by caller-paced stepping.
    #[must_use]
    pub fn visible_events(&self) -> &[RouteEvent] {
        &self.events()[..self.snapshot.cursor]
    }
    /// Number of revealed decisions.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.snapshot.cursor
    }
    /// Whether every recorded decision has been revealed.
    #[must_use]
    pub fn completed(&self) -> bool {
        self.cursor() == self.events().len()
    }
    /// The final path or unreachable result, withheld until playback completes.
    #[must_use]
    pub fn result(&self) -> Option<&Result<StreetPath, RouteError>> {
        self.completed().then_some(&self.calculation.result)
    }

    /// Project the caller-visible prefix without exposing future decisions.
    ///
    /// Rebuilding from the prefix makes backward seeks erase later knowledge.
    /// This folds recorded decisions; it performs no search or comparison.
    #[must_use]
    pub fn view(&self) -> RouteSearchView {
        let mut junctions = (0..self.junctions)
            .map(|junction| RouteSearchJunction {
                junction,
                cost: None,
                predecessor: None,
                state: RouteSearchState::Unseen,
            })
            .collect::<Vec<_>>();
        junctions[self.snapshot.from].cost = Some(0);
        junctions[self.snapshot.from].state = RouteSearchState::Tentative;
        for event in self.visible_events() {
            match *event {
                RouteEvent::Settled { junction, cost } => {
                    junctions[junction].cost = Some(cost);
                    junctions[junction].state = RouteSearchState::Settled;
                }
                RouteEvent::Relaxed { from, to, cost } => {
                    junctions[to].cost = Some(cost);
                    junctions[to].predecessor = Some(from);
                    junctions[to].state = RouteSearchState::Tentative;
                }
            }
        }
        let completed = self.completed();
        if completed {
            for junction in &mut junctions {
                if junction.state == RouteSearchState::Unseen {
                    junction.state = RouteSearchState::Unreachable;
                }
            }
        }
        RouteSearchView {
            from: self.snapshot.from,
            to: self.snapshot.to,
            cursor: self.cursor(),
            event_count: self.events().len(),
            completed,
            junctions,
            active_event: self.visible_events().last().copied(),
        }
    }
}

/// Canonical editor state shared by native and typed faces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteWorkbench {
    revision: u64,
    current: RouteTownSnapshot,
    undo: Vec<RouteTownSnapshot>,
    trace: Option<RouteTracePlayback>,
}

fn validate_town(mut town: RouteTownSnapshot) -> Result<RouteTownSnapshot, RouteError> {
    // Bound every collection before cloning, sorting, or solver work.
    if town.roads.len() > crate::route::MAX_ROUTE_ROADS
        || town.stops.len() > crate::route::MAX_ROUTE_STOPS
        || town.order.len() > crate::route::MAX_ROUTE_STOPS
    {
        return Err(RouteError::Size);
    }
    let mut roads: Vec<_> = town.roads.iter().map(|editable| editable.road).collect();
    validate_route_roads(town.junctions, &mut roads)?;
    town.roads.sort_unstable_by_key(|editable| {
        (
            editable.road.from.min(editable.road.to),
            editable.road.from.max(editable.road.to),
        )
    });
    for (editable, road) in town.roads.iter_mut().zip(roads) {
        editable.road = road;
    }
    validate_route_stops(town.junctions, &mut town.stops, 1)?;
    if town.order.len() != town.stops.len() || town.order.first() != town.stops.first() {
        return Err(RouteError::Tour);
    }
    for (index, junction) in town.order.iter().enumerate() {
        if !town.stops.contains(junction) || town.order[..index].contains(junction) {
            return Err(RouteError::Tour);
        }
    }
    Ok(town)
}

impl RouteWorkbench {
    /// Generate a connected practice map reproducibly from its seed and options.
    /// A random-parent spanning tree guarantees connectivity. Extra roads are
    /// sampled without replacement in bounded work; positive integer costs are
    /// independent of presentation. Up to six initial deliveries keep comparison
    /// inexpensive; the remaining junctions are transit nodes available to search.
    ///
    /// # Errors
    ///
    /// Returns `RandomOptions` before allocation for out-of-range options, or an
    /// ordinary admission error if generated data violates network bounds.
    pub fn random_map(
        seed: u64,
        options: RouteRandomMapOptions,
    ) -> Result<Self, RouteWorkbenchError> {
        let n = options.junctions;
        if !(3..=MAX_RANDOM_ROUTE_JUNCTIONS).contains(&n)
            || !(1..=crate::route::MAX_ROUTE_ROAD_COST).contains(&options.max_cost)
        {
            return Err(RouteWorkbenchError::RandomOptions);
        }
        let remaining = n * (n - 1) / 2 - (n - 1);
        if options.extra_roads > remaining {
            return Err(RouteWorkbenchError::RandomOptions);
        }
        let mut rng = crate::rng::SplitMix64::new(seed);
        let mut roads = Vec::with_capacity(n - 1 + options.extra_roads);
        for to in 1..n {
            let from = rng.below(to as u64) as usize;
            let cost = 1 + rng.below(u64::from(options.max_cost)) as u32;
            roads.push(EditableRoad {
                road: Road { from, to, cost },
                open: true,
            });
        }
        let mut candidates = Vec::with_capacity(remaining);
        for from in 0..n {
            for to in from + 1..n {
                if !roads
                    .iter()
                    .any(|editable| (editable.road.from, editable.road.to) == (from, to))
                {
                    candidates.push((from, to));
                }
            }
        }
        for index in 0..options.extra_roads {
            let selected = index + rng.below((candidates.len() - index) as u64) as usize;
            candidates.swap(index, selected);
            let (from, to) = candidates[index];
            let cost = 1 + rng.below(u64::from(options.max_cost)) as u32;
            roads.push(EditableRoad {
                road: Road { from, to, cost },
                open: true,
            });
        }
        let stops: Vec<_> = (0..n.min(6)).collect();
        Self::from_snapshot(RouteWorkbenchSnapshot {
            revision: 0,
            current: RouteTownSnapshot {
                junctions: n,
                roads,
                order: stops.clone(),
                stops,
            },
            undo: Vec::new(),
            trace: None,
        })
    }

    /// Open the documented four-stop town with every road available.
    #[must_use]
    pub fn first_town() -> Self {
        let roads = [(0, 1, 1), (0, 2, 2), (1, 2, 2), (1, 3, 3), (2, 3, 2)]
            .into_iter()
            .map(|(from, to, cost)| EditableRoad {
                road: Road { from, to, cost },
                open: true,
            })
            .collect();
        Self {
            revision: 0,
            current: RouteTownSnapshot {
                junctions: 4,
                roads,
                stops: vec![0, 1, 2, 3],
                order: vec![0, 1, 2, 3],
            },
            undo: Vec::new(),
            trace: None,
        }
    }

    /// Begin editing an already admitted town, with no undo or playback.
    #[must_use]
    pub fn from_problem(problem: RouteProblem) -> Self {
        Self {
            revision: 0,
            current: RouteTownSnapshot {
                junctions: problem.junction_count(),
                roads: problem
                    .roads()
                    .iter()
                    .map(|&road| EditableRoad { road, open: true })
                    .collect(),
                stops: problem.stops().to_vec(),
                order: problem.stops().to_vec(),
            },
            undo: Vec::new(),
            trace: None,
        }
    }

    /// Validate caller-carried state. Infeasible tours are retained; structural
    /// corruption is refused. Collection and undo bounds precede computation.
    /// Trace events are regenerated from current streets. The supplied cursor
    /// refers to that fresh calculation, not proof imported from another town.
    pub fn from_snapshot(snapshot: RouteWorkbenchSnapshot) -> Result<Self, RouteWorkbenchError> {
        if snapshot.undo.len() > MAX_ROUTE_UNDO || snapshot.undo.len() as u64 > snapshot.revision {
            return Err(RouteWorkbenchError::UndoLimit);
        }
        let current = validate_town(snapshot.current)?;
        let undo = snapshot
            .undo
            .into_iter()
            .map(validate_town)
            .collect::<Result<Vec<_>, _>>()?;
        let mut workbench = Self {
            revision: snapshot.revision,
            current,
            undo,
            trace: None,
        };
        if let Some(trace) = snapshot.trace {
            if trace.revision != workbench.revision {
                return Err(RouteWorkbenchError::TraceRevision);
            }
            if trace.town_identity != workbench.town_identity() {
                return Err(RouteWorkbenchError::TraceIdentity);
            }
            if trace.cursor > crate::route::MAX_ROUTE_JUNCTIONS + 2 * crate::route::MAX_ROUTE_ROADS
            {
                return Err(RouteWorkbenchError::TraceCursor);
            }
            workbench.start_trace(trace.from, trace.to)?;
            workbench.seek_trace(trace.cursor)?;
        }
        Ok(workbench)
    }

    /// Export bounded editing state. No event or mathematical claim is trusted
    /// when this snapshot is later reopened.
    #[must_use]
    pub fn snapshot(&self) -> RouteWorkbenchSnapshot {
        RouteWorkbenchSnapshot {
            revision: self.revision,
            current: self.current.clone(),
            undo: self.undo.clone(),
            trace: self.trace.as_ref().map(RouteTracePlayback::snapshot),
        }
    }

    /// Current immutable editable town.
    #[must_use]
    pub const fn town(&self) -> &RouteTownSnapshot {
        &self.current
    }
    /// Current local revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// SHA-256 of versioned canonical town content, including closed costs and
    /// the selected order. This detects a stale cursor; it is not an authorship
    /// or history attestation. No identity is accepted in place of validation.
    #[must_use]
    pub fn town_identity(&self) -> [u8; 32] {
        let mut bytes = b"numinous-route-town 1\n".to_vec();
        bytes.extend_from_slice(&(self.current.junctions as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.current.roads.len() as u64).to_le_bytes());
        for editable in &self.current.roads {
            bytes.extend_from_slice(&(editable.road.from as u64).to_le_bytes());
            bytes.extend_from_slice(&(editable.road.to as u64).to_le_bytes());
            bytes.extend_from_slice(&editable.road.cost.to_le_bytes());
            bytes.push(u8::from(editable.open));
        }
        for values in [&self.current.stops, &self.current.order] {
            bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
            for &value in values {
                bytes.extend_from_slice(&(value as u64).to_le_bytes());
            }
        }
        crate::sha256::digest(&bytes)
    }

    fn open_roads(&self) -> Vec<Road> {
        self.current
            .roads
            .iter()
            .filter(|road| road.open)
            .map(|road| road.road)
            .collect()
    }

    /// Compare against the current streets without changing the player order.
    /// Too few stops or unreachable deliveries produce an explicit diagnostic.
    pub fn compare(&self) -> Result<RouteComparison, RouteError> {
        if self.current.stops.len() < 3 {
            return Err(RouteError::Size);
        }
        let roads = self.open_roads();
        if roads.is_empty() {
            return Err(RouteError::Unreachable {
                from: self.current.stops[0],
                to: self.current.stops[1],
            });
        }
        let metric = RouteProblem::new(self.current.junctions, roads, self.current.stops.clone())?
            .metric()?;
        Ok(RouteComparison {
            current: metric.evaluate(&self.current.order)?,
            greedy: metric.nearest_neighbor()?,
            exact: metric.exact()?,
            proposal: metric.best_exchange(&self.current.order)?,
        })
    }

    /// Apply one validated edit transactionally. A genuine change increments
    /// revision, retains a bounded undo town, and invalidates playback. A no-op
    /// does none of those things. Failed edits never replace current state.
    pub fn apply(&mut self, edit: RouteEdit) -> Result<bool, RouteWorkbenchError> {
        let mut next = self.current.clone();
        match edit {
            RouteEdit::Network(network) => next = network,
            RouteEdit::RoadCost { from, to, cost } => {
                let road = find_road_mut(&mut next, from, to)?;
                road.road.cost = cost;
            }
            RouteEdit::RoadOpen { from, to, open } => {
                find_road_mut(&mut next, from, to)?.open = open
            }
            RouteEdit::Order(order) => next.order = order,
            RouteEdit::RequiredStops(mut stops) => {
                validate_route_stops(next.junctions, &mut stops, 1)?;
                let depot = stops[0];
                // A new depot already on the route changes its starting point,
                // not the closed cycle. Filter only after that rotation.
                if let Some(position) = next.order.iter().position(|&junction| junction == depot) {
                    next.order.rotate_left(position);
                }
                let mut order = vec![depot];
                order.extend(
                    next.order
                        .iter()
                        .copied()
                        .filter(|junction| *junction != depot && stops.contains(junction)),
                );
                for &junction in &stops[1..] {
                    if !order.contains(&junction) {
                        order.push(junction);
                    }
                }
                next.stops = stops;
                next.order = order;
            }
            RouteEdit::Depot(depot) => {
                let position = next
                    .order
                    .iter()
                    .position(|&junction| junction == depot)
                    .ok_or(RouteWorkbenchError::DepotNotRequired(depot))?;
                next.order.rotate_left(position);
                next.stops = next.order.clone();
            }
            RouteEdit::Greedy => next.order = self.compare()?.greedy.order,
            RouteEdit::Improve => {
                if let Some(proposal) = self.compare()?.proposal {
                    next.order = proposal.tour.order;
                }
            }
        }
        next = validate_town(next)?;
        if next == self.current {
            return Ok(false);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(RouteWorkbenchError::Revision)?;
        if self.undo.len() == MAX_ROUTE_UNDO {
            self.undo.remove(0);
        }
        self.undo.push(std::mem::replace(&mut self.current, next));
        self.revision = revision;
        self.trace = None;
        Ok(true)
    }

    /// Restore the newest previous town, including its costs and order. Undo
    /// receives a fresh revision and invalidates old playback rather than
    /// reviving a certificate for an earlier revision. Empty undo is a no-op.
    pub fn undo(&mut self) -> Result<bool, RouteWorkbenchError> {
        if self.undo.is_empty() {
            return Ok(false);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(RouteWorkbenchError::Revision)?;
        if let Some(town) = self.undo.pop() {
            self.current = town;
        }
        self.revision = revision;
        self.trace = None;
        Ok(true)
    }

    /// Record an actual shortest-path calculation at cursor zero. Required
    /// stops need not be feasible for this search. Failure validates nothing
    /// and leaves any existing playback unchanged.
    pub fn start_trace(&mut self, from: usize, to: usize) -> Result<(), RouteWorkbenchError> {
        let calculation =
            shortest_street_trace(self.current.junctions, self.open_roads(), from, to)?;
        self.trace = Some(RouteTracePlayback {
            snapshot: RouteTraceSnapshot {
                revision: self.revision,
                town_identity: self.town_identity(),
                from,
                to,
                cursor: 0,
            },
            calculation,
            junctions: self.current.junctions,
        });
        Ok(())
    }

    /// Seek to an exact recorded event boundary. The caller controls pacing;
    /// changing the cursor performs no new solver work and changes no revision.
    pub fn seek_trace(&mut self, cursor: usize) -> Result<(), RouteWorkbenchError> {
        let trace = self
            .trace
            .as_mut()
            .ok_or(RouteWorkbenchError::TraceAbsent)?;
        if trace.snapshot.revision != self.revision {
            return Err(RouteWorkbenchError::TraceRevision);
        }
        if cursor > trace.events().len() {
            return Err(RouteWorkbenchError::TraceCursor);
        }
        trace.snapshot.cursor = cursor;
        Ok(())
    }

    /// Current revision-bound recorded calculation, if one has been started.
    #[must_use]
    pub const fn trace(&self) -> Option<&RouteTracePlayback> {
        self.trace.as_ref()
    }
}

fn find_road_mut(
    town: &mut RouteTownSnapshot,
    from: usize,
    to: usize,
) -> Result<&mut EditableRoad, RouteWorkbenchError> {
    if from >= town.junctions || to >= town.junctions {
        return Err(RouteError::Junction.into());
    }
    let pair = (from.min(to), from.max(to));
    town.roads
        .iter_mut()
        .find(|road| (road.road.from, road.road.to) == pair)
        .ok_or(RouteWorkbenchError::MissingRoad { from, to })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_maps_are_repeatable_connected_and_match_independent_path_costs() {
        for seed in [0, 1, 17, u64::MAX] {
            for junctions in 3..=MAX_RANDOM_ROUTE_JUNCTIONS {
                let remaining = junctions * (junctions - 1) / 2 - (junctions - 1);
                for extra_roads in [0, remaining / 2, remaining] {
                    let options = RouteRandomMapOptions {
                        junctions,
                        extra_roads,
                        max_cost: 999,
                    };
                    let mut workbench = RouteWorkbench::random_map(seed, options).unwrap();
                    assert_eq!(
                        workbench.snapshot(),
                        RouteWorkbench::random_map(seed, options)
                            .unwrap()
                            .snapshot()
                    );
                    let town = workbench.town();
                    assert_eq!(town.roads.len(), junctions - 1 + extra_roads);
                    assert_eq!(town.stops, (0..junctions.min(6)).collect::<Vec<_>>());
                    let pairs: std::collections::BTreeSet<_> = town
                        .roads
                        .iter()
                        .map(|road| (road.road.from, road.road.to))
                        .collect();
                    assert_eq!(pairs.len(), town.roads.len());
                    let mut reachable = vec![false; junctions];
                    let mut frontier = vec![0];
                    reachable[0] = true;
                    while let Some(from) = frontier.pop() {
                        for road in &town.roads {
                            assert!(road.open);
                            assert!((1..=options.max_cost).contains(&road.road.cost));
                            let to = if road.road.from == from {
                                Some(road.road.to)
                            } else if road.road.to == from {
                                Some(road.road.from)
                            } else {
                                None
                            };
                            if let Some(to) = to
                                && !reachable[to]
                            {
                                reachable[to] = true;
                                frontier.push(to);
                            }
                        }
                    }
                    assert!(reachable.into_iter().all(|reached| reached));
                    // Independent all-pairs dynamic programming, without the
                    // production predecessor or Dijkstra recurrence.
                    let mut costs = vec![vec![u64::MAX / 4; junctions]; junctions];
                    for (node, row) in costs.iter_mut().enumerate() {
                        row[node] = 0;
                    }
                    for road in &town.roads {
                        costs[road.road.from][road.road.to] = u64::from(road.road.cost);
                        costs[road.road.to][road.road.from] = u64::from(road.road.cost);
                    }
                    for via in 0..junctions {
                        for from in 0..junctions {
                            for to in 0..junctions {
                                costs[from][to] =
                                    costs[from][to].min(costs[from][via] + costs[via][to]);
                            }
                        }
                    }
                    let source = seed as usize % junctions;
                    for (target, expected) in costs[source].iter().enumerate() {
                        workbench.start_trace(source, target).unwrap();
                        assert_eq!(workbench.trace().unwrap().view().from, source);
                        assert!(workbench.trace().unwrap().result().is_none());
                        let end = workbench.trace().unwrap().events().len();
                        workbench.seek_trace(end).unwrap();
                        let path = workbench
                            .trace()
                            .unwrap()
                            .result()
                            .unwrap()
                            .as_ref()
                            .unwrap();
                        assert_eq!(u64::from(path.cost), *expected);
                        assert_eq!(path.junctions.first(), Some(&source));
                        assert_eq!(path.junctions.last(), Some(&target));
                    }
                }
            }
        }
        assert_ne!(
            RouteWorkbench::random_map(0, RouteRandomMapOptions::default())
                .unwrap()
                .town(),
            RouteWorkbench::random_map(1, RouteRandomMapOptions::default())
                .unwrap()
                .town()
        );
    }

    #[test]
    fn random_map_options_refuse_extremes_before_work_and_unit_costs_are_real() {
        let defaults = RouteRandomMapOptions::default();
        for options in [
            RouteRandomMapOptions {
                junctions: 0,
                ..defaults
            },
            RouteRandomMapOptions {
                junctions: 2,
                ..defaults
            },
            RouteRandomMapOptions {
                junctions: MAX_RANDOM_ROUTE_JUNCTIONS + 1,
                ..defaults
            },
            RouteRandomMapOptions {
                junctions: usize::MAX,
                ..defaults
            },
            RouteRandomMapOptions {
                extra_roads: usize::MAX,
                ..defaults
            },
            RouteRandomMapOptions {
                max_cost: 0,
                ..defaults
            },
            RouteRandomMapOptions {
                max_cost: 1000,
                ..defaults
            },
            RouteRandomMapOptions {
                max_cost: u32::MAX,
                ..defaults
            },
            RouteRandomMapOptions {
                junctions: 3,
                extra_roads: 2,
                max_cost: 1,
            },
        ] {
            assert_eq!(
                RouteWorkbench::random_map(1, options),
                Err(RouteWorkbenchError::RandomOptions)
            );
        }
        let triangle = RouteWorkbench::random_map(
            u64::MAX,
            RouteRandomMapOptions {
                junctions: 3,
                extra_roads: 1,
                max_cost: 1,
            },
        )
        .unwrap();
        let comparison = triangle.compare().unwrap();
        assert_eq!(comparison.current.cost, 3);
        assert_eq!(comparison.exact.tour.cost, 3);
        assert!(comparison.proposal.is_none());
        assert_eq!(
            comparison.current.walk.first(),
            comparison.current.walk.last()
        );
    }

    #[test]
    fn random_map_replacement_undo_and_capsules_use_existing_network_contract() {
        let generated = RouteWorkbench::random_map(42, RouteRandomMapOptions::default()).unwrap();
        assert_eq!(generated.revision(), 0);
        assert!(generated.snapshot().undo.is_empty());
        assert!(generated.trace().is_none());
        let mut edited = RouteWorkbench::first_town();
        edited.start_trace(0, 3).unwrap();
        let original = edited.town().clone();
        assert!(
            edited
                .apply(RouteEdit::Network(generated.town().clone()))
                .unwrap()
        );
        assert!(edited.trace().is_none());
        let capsule = crate::RouteCreation::new(edited.town().clone()).unwrap();
        assert_eq!(
            crate::RouteCreation::from_capsule(&capsule.to_capsule())
                .unwrap()
                .open()
                .town(),
            generated.town()
        );
        assert!(edited.undo().unwrap());
        assert_eq!(edited.town(), &original);
        assert!(edited.trace().is_none());
    }

    #[test]
    fn a_structural_edit_replaces_the_network_atomically_and_undo_restores_it() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.start_trace(0, 3).unwrap();
        workbench.seek_trace(1).unwrap();
        let original = workbench.town().clone();
        let network = RouteTownSnapshot {
            junctions: 5,
            roads: [(0, 1, 2), (1, 2, 3), (2, 3, 4), (3, 4, 5)]
                .into_iter()
                .map(|(from, to, cost)| EditableRoad {
                    road: Road { from, to, cost },
                    open: true,
                })
                .chain([EditableRoad {
                    road: Road {
                        from: 0,
                        to: 4,
                        cost: 40,
                    },
                    open: false,
                }])
                .collect(),
            stops: vec![4, 1, 2],
            order: vec![4, 2, 1],
        };
        assert!(workbench.apply(RouteEdit::Network(network)).unwrap());
        assert_eq!(workbench.revision(), 1);
        assert!(workbench.trace().is_none());
        let comparison = workbench.compare().unwrap();
        // On a line, the required span is traversed twice: 2*(3+4+5).
        assert_eq!(comparison.current.cost, 24);
        assert_eq!(comparison.exact.tour.cost, 24);
        assert_eq!(comparison.current.walk, vec![4, 3, 2, 1, 2, 3, 4]);
        assert_eq!(workbench.town().order, vec![4, 2, 1]);
        assert!(workbench.undo().unwrap());
        assert_eq!(workbench.town(), &original);
        assert_eq!(workbench.revision(), 2);
        assert!(workbench.trace().is_none());
    }

    #[test]
    fn structural_refusal_and_canonical_noop_leave_trace_and_history_untouched() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.start_trace(0, 3).unwrap();
        workbench.seek_trace(2).unwrap();
        let original = workbench.snapshot();
        let mut reordered = original.current.clone();
        reordered.roads.reverse();
        for road in &mut reordered.roads {
            std::mem::swap(&mut road.road.from, &mut road.road.to);
        }
        assert!(!workbench.apply(RouteEdit::Network(reordered)).unwrap());
        assert_eq!(workbench.snapshot(), original);
        for malformed in 0..7 {
            let mut network = original.current.clone();
            match malformed {
                0 => network.junctions = 3,
                1 => network.roads.push(network.roads[0]),
                2 => network.roads[0].road.cost = 0,
                3 => network.stops = vec![0, 1, 1],
                4 => network.order = vec![0, 1, 2],
                5 => network.roads = vec![network.roads[0]; crate::route::MAX_ROUTE_ROADS + 1],
                _ => network.junctions = crate::route::MAX_ROUTE_JUNCTIONS + 1,
            }
            assert!(workbench.apply(RouteEdit::Network(network)).is_err());
            assert_eq!(workbench.snapshot(), original);
        }
    }

    #[test]
    fn structural_drafts_remain_repairable_and_revision_failure_is_transactional() {
        let mut workbench = RouteWorkbench::first_town();
        let mut draft = workbench.town().clone();
        draft.roads.clear();
        draft.stops = vec![2];
        draft.order = vec![2];
        assert!(workbench.apply(RouteEdit::Network(draft)).unwrap());
        assert_eq!(workbench.compare(), Err(RouteError::Size));
        assert!(workbench.undo().unwrap());
        assert_eq!(workbench.compare().unwrap().current.cost, 9);
        let mut snapshot = workbench.snapshot();
        snapshot.revision = u64::MAX;
        let mut exhausted = RouteWorkbench::from_snapshot(snapshot.clone()).unwrap();
        let mut replacement = snapshot.current.clone();
        replacement.junctions += 1;
        assert_eq!(
            exhausted.apply(RouteEdit::Network(replacement)),
            Err(RouteWorkbenchError::Revision)
        );
        assert_eq!(exhausted.snapshot(), snapshot);
    }

    fn cost_edit(cost: u32) -> RouteEdit {
        RouteEdit::RoadCost {
            from: 1,
            to: 3,
            cost,
        }
    }

    #[test]
    fn closing_and_reopening_retains_authored_cost_and_an_infeasible_town() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.apply(cost_edit(7)).unwrap();
        workbench
            .apply(RouteEdit::RoadOpen {
                from: 3,
                to: 1,
                open: false,
            })
            .unwrap();
        workbench
            .apply(RouteEdit::RoadOpen {
                from: 2,
                to: 3,
                open: false,
            })
            .unwrap();
        assert_eq!(
            workbench.compare(),
            Err(RouteError::Unreachable { from: 0, to: 3 })
        );
        assert_eq!(
            workbench.town().roads[3],
            EditableRoad {
                road: Road {
                    from: 1,
                    to: 3,
                    cost: 7
                },
                open: false
            }
        );
        workbench
            .apply(RouteEdit::RoadOpen {
                from: 1,
                to: 3,
                open: true,
            })
            .unwrap();
        let comparison = workbench.compare().unwrap();
        // D is a leaf behind the cost-7 street: the player pays
        // AB + BC + CBD + DBA = 1 + 2 + 9 + 8, while a cheapest
        // tour traverses BD twice plus the cost-5 ABC triangle.
        assert_eq!(comparison.current.cost, 20);
        assert_eq!(comparison.exact.tour.cost, 19);
        assert_eq!(workbench.town().roads[3].road.cost, 7);
        workbench
            .apply(RouteEdit::RoadOpen {
                from: 2,
                to: 3,
                open: true,
            })
            .unwrap();
        assert_eq!(workbench.compare().unwrap().exact.tour.cost, 9);
    }

    #[test]
    fn too_few_deliveries_and_no_open_roads_are_retained_with_precise_diagnostics() {
        let mut workbench = RouteWorkbench::first_town();
        workbench
            .apply(RouteEdit::RequiredStops(vec![0, 1]))
            .unwrap();
        assert_eq!(workbench.town().order, vec![0, 1]);
        assert_eq!(workbench.compare(), Err(RouteError::Size));
        workbench.start_trace(0, 3).unwrap();
        assert!(workbench.trace().unwrap().result().is_none());
        workbench.undo().unwrap();
        assert_eq!(workbench.compare().unwrap().exact.tour.cost, 8);
        let roads = workbench.town().roads.clone();
        for editable in roads {
            workbench
                .apply(RouteEdit::RoadOpen {
                    from: editable.road.from,
                    to: editable.road.to,
                    open: false,
                })
                .unwrap();
        }
        assert_eq!(
            workbench.compare(),
            Err(RouteError::Unreachable { from: 0, to: 1 })
        );
        workbench.start_trace(0, 3).unwrap();
        assert_eq!(
            workbench.trace().unwrap().events(),
            &[RouteEvent::Settled {
                junction: 0,
                cost: 0
            }]
        );
        workbench.seek_trace(1).unwrap();
        assert_eq!(
            workbench.trace().unwrap().result(),
            Some(&Err(RouteError::Unreachable { from: 0, to: 3 }))
        );
        assert_eq!(
            RouteWorkbench::from_snapshot(workbench.snapshot()).unwrap(),
            workbench
        );
    }

    #[test]
    fn depot_rotation_and_stop_replacement_preserve_the_player_candidate() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.apply(RouteEdit::Order(vec![0, 2, 3, 1])).unwrap();
        let cost = workbench.compare().unwrap().current.cost;
        workbench.apply(RouteEdit::Depot(3)).unwrap();
        assert_eq!(workbench.town().stops, vec![3, 0, 1, 2]);
        assert_eq!(workbench.town().order, vec![3, 1, 0, 2]);
        assert_eq!(workbench.compare().unwrap().current.cost, cost);
        workbench
            .apply(RouteEdit::RequiredStops(vec![3, 2, 0]))
            .unwrap();
        assert_eq!(workbench.town().order, vec![3, 0, 2]);
        workbench
            .apply(RouteEdit::RequiredStops(vec![0, 3, 2, 1]))
            .unwrap();
        assert_eq!(workbench.town().order, vec![0, 2, 3, 1]);
        let before = workbench.snapshot();
        assert_eq!(
            workbench.apply(RouteEdit::Depot(99)),
            Err(RouteWorkbenchError::DepotNotRequired(99))
        );
        assert_eq!(workbench.snapshot(), before);
    }

    #[test]
    fn deliberate_route_construction_and_undo_restore_exact_costs_and_orders() {
        let mut workbench = RouteWorkbench::first_town();
        let initial = workbench.town().clone();
        assert!(workbench.apply(RouteEdit::Improve).unwrap());
        assert_eq!(workbench.town().order, vec![0, 1, 3, 2]);
        assert_eq!(workbench.compare().unwrap().current.cost, 8);
        assert!(!workbench.apply(RouteEdit::Improve).unwrap());
        assert_eq!(workbench.revision(), 1);
        workbench.apply(cost_edit(5)).unwrap();
        assert_eq!(workbench.compare().unwrap().current.cost, 9);
        workbench.apply(RouteEdit::Greedy).unwrap();
        assert_eq!(workbench.town().order, vec![0, 1, 2, 3]);
        assert_eq!(workbench.revision(), 3);
        for expected_revision in 4..=6 {
            assert!(workbench.undo().unwrap());
            assert_eq!(workbench.revision(), expected_revision);
        }
        assert_eq!(workbench.town(), &initial);
        assert!(!workbench.undo().unwrap());
        assert_eq!(workbench.revision(), 6);
    }

    #[test]
    fn playback_exposes_only_actual_recorded_decisions_at_the_chosen_cursor() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.start_trace(0, 3).unwrap();
        let events = workbench.trace().unwrap().events().to_vec();
        let revision = workbench.revision();
        let identity = workbench.town_identity();
        let mut tentative = [None; 4];
        tentative[0] = Some(0);
        let mut settled = [false; 4];
        for (index, event) in events.iter().enumerate() {
            match *event {
                RouteEvent::Settled { junction, cost } => {
                    let min = tentative
                        .iter()
                        .enumerate()
                        .filter(|(node, _)| !settled[*node])
                        .filter_map(|(node, value)| value.map(|value| (value, node)))
                        .min()
                        .unwrap();
                    assert_eq!(min, (cost, junction));
                    settled[junction] = true;
                }
                RouteEvent::Relaxed { from, to, cost } => {
                    assert!(settled[from]);
                    let road = workbench
                        .town()
                        .roads
                        .iter()
                        .find(|road| {
                            road.open
                                && ((road.road.from == from && road.road.to == to)
                                    || (road.road.to == from && road.road.from == to))
                        })
                        .unwrap();
                    assert_eq!(tentative[from].unwrap() + road.road.cost, cost);
                    assert!(tentative[to].is_none_or(|old| cost < old));
                    tentative[to] = Some(cost);
                }
            }
            workbench.seek_trace(index + 1).unwrap();
            let playback = workbench.trace().unwrap();
            assert_eq!(playback.visible_events(), &events[..index + 1]);
            assert_eq!(playback.result().is_some(), index + 1 == events.len());
            assert_eq!(workbench.revision(), revision);
            assert_eq!(workbench.town_identity(), identity);
        }
        let result = workbench
            .trace()
            .unwrap()
            .result()
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(result.cost, 4);
        assert_eq!(
            (result.junctions.first(), result.junctions.last()),
            (Some(&0), Some(&3))
        );
        let fully_revealed = workbench.snapshot();
        workbench.seek_trace(0).unwrap();
        assert!(workbench.trace().unwrap().visible_events().is_empty());
        assert!(workbench.trace().unwrap().result().is_none());
        workbench.seek_trace(events.len()).unwrap();
        assert_eq!(workbench.snapshot(), fully_revealed);
        assert_eq!(
            workbench.seek_trace(events.len() + 1),
            Err(RouteWorkbenchError::TraceCursor)
        );
        assert_eq!(workbench.snapshot(), fully_revealed);
    }

    #[test]
    fn edits_and_undo_invalidate_trace_while_no_ops_and_failed_edits_preserve_it() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.start_trace(0, 3).unwrap();
        workbench.seek_trace(2).unwrap();
        let before = workbench.snapshot();
        assert!(!workbench.apply(cost_edit(3)).unwrap());
        assert_eq!(workbench.snapshot(), before);
        assert!(workbench.apply(cost_edit(0)).is_err());
        assert_eq!(workbench.snapshot(), before);
        assert!(workbench.start_trace(0, 99).is_err());
        assert_eq!(workbench.snapshot(), before);
        workbench.apply(cost_edit(5)).unwrap();
        assert!(workbench.trace().is_none());
        assert_eq!(
            workbench.seek_trace(0),
            Err(RouteWorkbenchError::TraceAbsent)
        );
        workbench.start_trace(0, 3).unwrap();
        workbench.undo().unwrap();
        assert!(workbench.trace().is_none());
        assert_eq!(workbench.revision(), 2);
    }

    #[test]
    fn bounded_undo_and_revision_overflow_are_transactional() {
        let mut workbench = RouteWorkbench::first_town();
        let mut previous = Vec::new();
        for index in 0..40 {
            previous.push(workbench.town().clone());
            workbench.apply(cost_edit(1 + (index % 9))).unwrap();
            assert!(workbench.snapshot().undo.len() <= MAX_ROUTE_UNDO);
        }
        assert_eq!(workbench.snapshot().undo.len(), MAX_ROUTE_UNDO);
        for expected in previous.into_iter().rev().take(MAX_ROUTE_UNDO) {
            assert!(workbench.undo().unwrap());
            assert_eq!(workbench.town(), &expected);
        }
        assert!(!workbench.undo().unwrap());
        let mut snapshot = RouteWorkbench::first_town().snapshot();
        snapshot.revision = u64::MAX;
        snapshot.undo.push(snapshot.current.clone());
        let mut full = RouteWorkbench::from_snapshot(snapshot).unwrap();
        let before = full.snapshot();
        assert_eq!(full.apply(cost_edit(4)), Err(RouteWorkbenchError::Revision));
        assert_eq!(full.snapshot(), before);
        assert_eq!(full.undo(), Err(RouteWorkbenchError::Revision));
        assert_eq!(full.snapshot(), before);
        assert!(!full.apply(cost_edit(3)).unwrap());
    }

    #[test]
    fn snapshot_identity_is_canonical_and_rejects_same_revision_changed_towns() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.start_trace(0, 3).unwrap();
        workbench.seek_trace(1).unwrap();
        let original = workbench.snapshot();
        let mut permuted = original.clone();
        permuted.current.roads.reverse();
        for road in &mut permuted.current.roads {
            std::mem::swap(&mut road.road.from, &mut road.road.to);
        }
        permuted.current.stops = vec![0, 3, 2, 1];
        let restored = RouteWorkbench::from_snapshot(permuted).unwrap();
        assert_eq!(restored, workbench);
        for field in 0..4 {
            let mut changed = original.clone();
            match field {
                0 => changed.current.roads[3].road.cost = 5,
                1 => changed.current.roads[3].open = false,
                2 => changed.current.order = vec![0, 1, 3, 2],
                _ => {
                    changed.current.stops = vec![0, 1, 2];
                    changed.current.order = vec![0, 1, 2];
                }
            }
            assert_eq!(
                RouteWorkbench::from_snapshot(changed),
                Err(RouteWorkbenchError::TraceIdentity)
            );
        }
        let mut old = original.clone();
        old.trace.as_mut().unwrap().revision = 1;
        assert_eq!(
            RouteWorkbench::from_snapshot(old),
            Err(RouteWorkbenchError::TraceRevision)
        );
        let mut bad_cursor = original;
        bad_cursor.trace.as_mut().unwrap().cursor = usize::MAX;
        assert_eq!(
            RouteWorkbench::from_snapshot(bad_cursor),
            Err(RouteWorkbenchError::TraceCursor)
        );
    }

    #[test]
    fn malformed_imports_and_edits_are_refused_without_displacing_work() {
        let original = RouteWorkbench::first_town().snapshot();
        for field in 0..9 {
            let mut snapshot = original.clone();
            match field {
                0 => snapshot.current.junctions = usize::MAX,
                1 => {
                    snapshot.current.roads =
                        vec![snapshot.current.roads[0]; crate::route::MAX_ROUTE_ROADS + 1]
                }
                2 => snapshot.current.stops = vec![0; crate::route::MAX_ROUTE_STOPS + 1],
                3 => snapshot.current.order = vec![0; crate::route::MAX_ROUTE_STOPS + 1],
                4 => snapshot.current.roads[0].road.cost = 0,
                5 => snapshot.current.roads[0].road.to = 99,
                6 => snapshot.current.stops = vec![0, 1, 1],
                7 => snapshot.current.order = vec![0, 1, 2, 2],
                _ => snapshot.undo = vec![snapshot.current.clone(); MAX_ROUTE_UNDO + 1],
            }
            assert!(RouteWorkbench::from_snapshot(snapshot).is_err());
        }
        let mut revision = original.clone();
        revision.undo.push(revision.current.clone());
        assert_eq!(
            RouteWorkbench::from_snapshot(revision),
            Err(RouteWorkbenchError::UndoLimit)
        );
        let mut malformed_undo = original.clone();
        malformed_undo.revision = 1;
        malformed_undo.undo.push(malformed_undo.current.clone());
        malformed_undo.undo[0].order = vec![99];
        assert!(RouteWorkbench::from_snapshot(malformed_undo).is_err());
        let mut workbench = RouteWorkbench::first_town();
        for edit in [
            RouteEdit::RoadCost {
                from: 0,
                to: 3,
                cost: 2,
            },
            RouteEdit::RoadOpen {
                from: 0,
                to: 99,
                open: false,
            },
            RouteEdit::Order(vec![0, 1, 2]),
            RouteEdit::RequiredStops(vec![0, 1, 1]),
        ] {
            assert!(workbench.apply(edit).is_err());
            assert_eq!(workbench.snapshot(), original);
        }
    }

    #[test]
    fn an_arbitrary_line_town_keeps_its_authored_stops_and_expands_real_streets() {
        let problem = RouteProblem::new(
            5,
            vec![
                Road {
                    from: 0,
                    to: 1,
                    cost: 4,
                },
                Road {
                    from: 1,
                    to: 2,
                    cost: 3,
                },
                Road {
                    from: 2,
                    to: 3,
                    cost: 5,
                },
                Road {
                    from: 3,
                    to: 4,
                    cost: 2,
                },
            ],
            vec![0, 2, 4],
        )
        .unwrap();
        let mut workbench = RouteWorkbench::from_problem(problem);
        assert_eq!(workbench.compare().unwrap().exact.tour.cost, 28);
        workbench.apply(RouteEdit::Depot(4)).unwrap();
        assert_eq!(workbench.town().order, vec![4, 0, 2]);
        workbench
            .apply(RouteEdit::RequiredStops(vec![4, 1, 3]))
            .unwrap();
        assert_eq!(workbench.town().order, vec![4, 1, 3]);
        // Any closed walk on this line must cross each required separating
        // street twice: 2*(3+5+2), with the unused outer AB road omitted.
        let tour = workbench.compare().unwrap().exact.tour;
        assert_eq!(tour.cost, 20);
        assert!(!tour.walk.contains(&0));
        workbench
            .apply(RouteEdit::RoadOpen {
                from: 1,
                to: 2,
                open: false,
            })
            .unwrap();
        assert_eq!(
            workbench.compare(),
            Err(RouteError::Unreachable { from: 4, to: 1 })
        );
        workbench.start_trace(4, 0).unwrap();
        let length = workbench.trace().unwrap().events().len();
        workbench.seek_trace(length).unwrap();
        assert_eq!(
            workbench.trace().unwrap().result(),
            Some(&Err(RouteError::Unreachable { from: 4, to: 0 }))
        );
        assert_eq!(
            RouteWorkbench::from_snapshot(workbench.snapshot()).unwrap(),
            workbench
        );
        workbench.undo().unwrap();
        assert_eq!(workbench.compare().unwrap().exact.tour.cost, 20);
    }

    fn search_town(junctions: usize, roads: &[(usize, usize, u32)]) -> RouteWorkbench {
        RouteWorkbench::from_snapshot(RouteWorkbenchSnapshot {
            revision: 0,
            current: RouteTownSnapshot {
                junctions,
                roads: roads
                    .iter()
                    .map(|&(from, to, cost)| EditableRoad {
                        road: Road { from, to, cost },
                        open: true,
                    })
                    .collect(),
                stops: vec![0],
                order: vec![0],
            },
            undo: Vec::new(),
            trace: None,
        })
        .unwrap()
    }

    #[test]
    fn search_view_has_independent_prefix_truth_and_erases_later_knowledge_on_seek() {
        use RouteSearchState::{Settled, Tentative, Unreachable, Unseen};
        let mut workbench =
            search_town(5, &[(0, 1, 8), (0, 2, 2), (1, 2, 3), (1, 3, 1), (2, 3, 9)]);
        workbench.start_trace(0, 3).unwrap();
        // The cheaper 0-2-1 path replaces cost 8 with 5; its extension through
        // 1 replaces 0-2-3 cost 11 with 6. Junction 4 is isolated throughout.
        let decisions = [
            RouteEvent::Settled {
                junction: 0,
                cost: 0,
            },
            RouteEvent::Relaxed {
                from: 0,
                to: 1,
                cost: 8,
            },
            RouteEvent::Relaxed {
                from: 0,
                to: 2,
                cost: 2,
            },
            RouteEvent::Settled {
                junction: 2,
                cost: 2,
            },
            RouteEvent::Relaxed {
                from: 2,
                to: 1,
                cost: 5,
            },
            RouteEvent::Relaxed {
                from: 2,
                to: 3,
                cost: 11,
            },
            RouteEvent::Settled {
                junction: 1,
                cost: 5,
            },
            RouteEvent::Relaxed {
                from: 1,
                to: 3,
                cost: 6,
            },
            RouteEvent::Settled {
                junction: 3,
                cost: 6,
            },
        ];
        let unknown = (None, None, Unseen);
        let zero = (Some(0), None, Settled);
        let states = [
            [
                (Some(0), None, Tentative),
                unknown,
                unknown,
                unknown,
                unknown,
            ],
            [zero, unknown, unknown, unknown, unknown],
            [
                zero,
                (Some(8), Some(0), Tentative),
                unknown,
                unknown,
                unknown,
            ],
            [
                zero,
                (Some(8), Some(0), Tentative),
                (Some(2), Some(0), Tentative),
                unknown,
                unknown,
            ],
            [
                zero,
                (Some(8), Some(0), Tentative),
                (Some(2), Some(0), Settled),
                unknown,
                unknown,
            ],
            [
                zero,
                (Some(5), Some(2), Tentative),
                (Some(2), Some(0), Settled),
                unknown,
                unknown,
            ],
            [
                zero,
                (Some(5), Some(2), Tentative),
                (Some(2), Some(0), Settled),
                (Some(11), Some(2), Tentative),
                unknown,
            ],
            [
                zero,
                (Some(5), Some(2), Settled),
                (Some(2), Some(0), Settled),
                (Some(11), Some(2), Tentative),
                unknown,
            ],
            [
                zero,
                (Some(5), Some(2), Settled),
                (Some(2), Some(0), Settled),
                (Some(6), Some(1), Tentative),
                unknown,
            ],
            [
                zero,
                (Some(5), Some(2), Settled),
                (Some(2), Some(0), Settled),
                (Some(6), Some(1), Settled),
                (None, None, Unreachable),
            ],
        ];
        assert_eq!(workbench.trace().unwrap().events(), decisions);
        for cursor in (0..=decisions.len()).chain([8, 5, 0, 3, 9]) {
            workbench.seek_trace(cursor).unwrap();
            let playback = workbench.trace().unwrap();
            let view = playback.view();
            let expected = states[cursor]
                .iter()
                .enumerate()
                .map(
                    |(junction, &(cost, predecessor, state))| RouteSearchJunction {
                        junction,
                        cost,
                        predecessor,
                        state,
                    },
                )
                .collect();
            assert_eq!(
                view,
                RouteSearchView {
                    from: 0,
                    to: 3,
                    cursor,
                    event_count: 9,
                    completed: cursor == 9,
                    junctions: expected,
                    active_event: cursor.checked_sub(1).map(|index| decisions[index]),
                }
            );
            assert_eq!(playback.result().is_some(), cursor == 9);
        }
        let path = workbench
            .trace()
            .unwrap()
            .result()
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(path.junctions, vec![0, 2, 1, 3]);
        assert_eq!(path.cost, 6);
    }

    #[test]
    fn equal_cost_search_alternatives_retain_the_first_revealed_predecessor() {
        let mut workbench = search_town(5, &[(0, 1, 2), (0, 2, 2), (1, 3, 3), (2, 3, 3)]);
        workbench.start_trace(0, 3).unwrap();
        let count = workbench.trace().unwrap().events().len();
        assert_eq!(count, 7);
        workbench.seek_trace(5).unwrap();
        let before = workbench.trace().unwrap().view();
        assert_eq!(
            before.junctions[3],
            RouteSearchJunction {
                junction: 3,
                cost: Some(5),
                predecessor: Some(1),
                state: RouteSearchState::Tentative,
            }
        );
        workbench.seek_trace(6).unwrap();
        let after = workbench.trace().unwrap().view();
        assert_eq!(after.junctions[3], before.junctions[3]);
        assert_eq!(after.junctions[2].state, RouteSearchState::Settled);
        assert_eq!(
            after.active_event,
            Some(RouteEvent::Settled {
                junction: 2,
                cost: 2
            })
        );
        assert!(workbench.trace().unwrap().result().is_none());
        workbench.seek_trace(count).unwrap();
        assert_eq!(
            workbench.trace().unwrap().view().junctions[3].predecessor,
            Some(1)
        );
        assert_eq!(
            workbench
                .trace()
                .unwrap()
                .result()
                .unwrap()
                .as_ref()
                .unwrap()
                .junctions,
            vec![0, 1, 3]
        );
    }

    #[test]
    fn search_view_completion_distinguishes_unknown_from_proven_unreachable() {
        for target in [31, 0] {
            let mut workbench = search_town(crate::route::MAX_ROUTE_JUNCTIONS, &[]);
            workbench.start_trace(31, target).unwrap();
            let initial = workbench.trace().unwrap().view();
            assert_eq!(initial.junctions.len(), crate::route::MAX_ROUTE_JUNCTIONS);
            assert_eq!(
                initial.junctions[31],
                RouteSearchJunction {
                    junction: 31,
                    cost: Some(0),
                    predecessor: None,
                    state: RouteSearchState::Tentative,
                }
            );
            assert!(
                initial.junctions[..31]
                    .iter()
                    .all(|junction| junction.state == RouteSearchState::Unseen
                        && junction.cost.is_none()
                        && junction.predecessor.is_none())
            );
            assert!(!initial.completed);
            assert!(initial.active_event.is_none());
            assert!(workbench.trace().unwrap().result().is_none());
            workbench.seek_trace(1).unwrap();
            let playback = workbench.trace().unwrap();
            let final_view = playback.view();
            assert!(final_view.completed);
            assert_eq!(final_view.junctions[31].state, RouteSearchState::Settled);
            assert!(
                final_view.junctions[..31]
                    .iter()
                    .all(|junction| junction.state == RouteSearchState::Unreachable
                        && junction.cost.is_none()
                        && junction.predecessor.is_none())
            );
            if target == 31 {
                let path = playback.result().unwrap().as_ref().unwrap();
                assert_eq!(path.cost, 0);
                assert_eq!(path.junctions, vec![31]);
            } else {
                assert_eq!(
                    playback.result(),
                    Some(&Err(RouteError::Unreachable { from: 31, to: 0 }))
                );
            }
            workbench.seek_trace(0).unwrap();
            assert_eq!(workbench.trace().unwrap().view(), initial);
        }
        let mut connected = RouteWorkbench::first_town();
        connected.start_trace(0, 0).unwrap();
        connected.seek_trace(1).unwrap();
        let playback = connected.trace().unwrap();
        assert_eq!(
            playback.view().junctions[0].state,
            RouteSearchState::Settled
        );
        assert!(!playback.view().completed);
        assert!(playback.result().is_none());
        let count = playback.events().len();
        connected.seek_trace(count).unwrap();
        let playback = connected.trace().unwrap();
        assert!(
            playback
                .view()
                .junctions
                .iter()
                .all(|junction| junction.state == RouteSearchState::Settled)
        );
        let path = playback.result().unwrap().as_ref().unwrap();
        assert_eq!(path.junctions, vec![0]);
        assert_eq!(path.cost, 0);
    }

    #[test]
    fn search_view_reimport_rebuilds_prefix_and_edits_invalidate_it_atomically() {
        let mut workbench =
            search_town(5, &[(0, 1, 8), (0, 2, 2), (1, 2, 3), (1, 3, 1), (2, 3, 9)]);
        workbench.start_trace(0, 3).unwrap();
        workbench.seek_trace(5).unwrap();
        let snapshot = workbench.snapshot();
        let view = workbench.trace().unwrap().view();
        let restored = RouteWorkbench::from_snapshot(snapshot.clone()).unwrap();
        assert_eq!(restored.snapshot(), snapshot);
        assert_eq!(restored.trace().unwrap().view(), view);
        let mut stale = snapshot.clone();
        stale.current.roads[0].road.cost = 7;
        assert_eq!(
            RouteWorkbench::from_snapshot(stale),
            Err(RouteWorkbenchError::TraceIdentity)
        );
        assert!(
            !workbench
                .apply(RouteEdit::RoadCost {
                    from: 0,
                    to: 1,
                    cost: 8
                })
                .unwrap()
        );
        assert!(
            workbench
                .apply(RouteEdit::RoadCost {
                    from: 0,
                    to: 1,
                    cost: 0
                })
                .is_err()
        );
        assert_eq!(workbench.trace().unwrap().view(), view);
        workbench
            .apply(RouteEdit::RoadOpen {
                from: 0,
                to: 2,
                open: false,
            })
            .unwrap();
        assert!(workbench.trace().is_none());
        workbench.start_trace(0, 3).unwrap();
        assert_eq!(workbench.trace().unwrap().view().cursor, 0);
        workbench.seek_trace(2).unwrap();
        assert_eq!(workbench.trace().unwrap().view().junctions[1].cost, Some(8));
        workbench.undo().unwrap();
        assert!(workbench.trace().is_none());
        workbench.start_trace(0, 3).unwrap();
        workbench.seek_trace(5).unwrap();
        assert_eq!(workbench.trace().unwrap().view(), view);
    }
}
