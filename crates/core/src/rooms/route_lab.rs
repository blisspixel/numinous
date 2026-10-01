//! A four-stop street-map experiment with a certified exact comparison.
//!
//! Drag the map to choose one of six delivery orders. The separate BD strip
//! changes a road's integer cost. Bottom controls build a nearest-neighbor tour
//! or accept one strictly improving two-edge exchange. Street closures, bounded
//! undo, and caller-paced recorded shortest-path decisions share the canonical
//! workbench. Old costs and certificates cannot survive an edit.

use crate::font;
use crate::motifs::Motif;
use crate::readout::{NumericReadout, ReadoutId};
use crate::room::{MAX_ROOM_INPUTS, Room, RoomInput, inputs_from_pokes};
use crate::route::{RouteError, RouteExchange, RouteTour};
use crate::route::{RouteEvent, shortest_street_trace};
use crate::route_workbench::{
    MAX_ROUTE_UNDO, RouteEdit, RouteTownSnapshot, RouteWorkbench, RouteWorkbenchSnapshot,
};
use crate::sound::{ParametricSound, SoundSpec};
use crate::surface::Surface;

const ORDERS: [[usize; 4]; 6] = [
    [0, 1, 2, 3],
    [0, 1, 3, 2],
    [0, 2, 1, 3],
    [0, 2, 3, 1],
    [0, 3, 1, 2],
    [0, 3, 2, 1],
];
const POINTS: [(f64, f64); 4] = [(0.18, 0.40), (0.45, 0.23), (0.45, 0.60), (0.82, 0.40)];
const ROAD_STRIP: f64 = 0.70;
const BUTTON_STRIP: f64 = 0.86;
const WORKBENCH_STRIP: f64 = 0.65;
const CHECKPOINT_START: u32 = 0xE000;
const CHECKPOINT_END: u32 = 0xE001;
const CHECKPOINT_DATA: u32 = 0xF0000;
const ROAD_NAMES: [&str; 5] = ["AB", "AC", "BC", "BD", "CD"];

fn bounded_inputs(inputs: &[RoomInput]) -> &[RoomInput] {
    let start = inputs.len().saturating_sub(MAX_ROOM_INPUTS);
    let mut inside_frame = false;
    // Oversized caller input retains the ordinary suffix contract. Scan only
    // framing markers in the discarded prefix, so a cut cannot admit payload
    // keys as edits. All actual replay and solver work remains bounded.
    for input in &inputs[..start] {
        match input {
            RoomInput::Key { ch } if u32::from(*ch) == CHECKPOINT_START && !inside_frame => {
                inside_frame = true
            }
            RoomInput::Key { ch } if u32::from(*ch) == CHECKPOINT_END && inside_frame => {
                inside_frame = false
            }
            _ => {}
        }
    }
    let suffix = &inputs[start..];
    if inside_frame {
        suffix
            .iter()
            .position(
                |input| matches!(input, RoomInput::Key { ch } if u32::from(*ch) == CHECKPOINT_END),
            )
            .map_or(&[], |end| &suffix[end + 1..])
    } else {
        suffix
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Selection {
    order: [usize; 4],
    bd: u32,
    touched: bool,
    closed: u8,
}

/// Replay state belongs to this fixed-town adapter. The shared workbench owns
/// edits, bounded undo, validation, and the actual recorded solver trace.
#[derive(Debug)]
struct Session {
    workbench: RouteWorkbench,
    touched: bool,
    selected: usize,
    trace_visible: bool,
}

impl Session {
    fn new(seed: u64) -> Self {
        let mut snapshot = RouteWorkbench::first_town().snapshot();
        snapshot.current.roads[3].road.cost = 3 + (seed % 4) as u32;
        Self {
            workbench: RouteWorkbench::from_snapshot(snapshot).expect("valid fixed-town seed"),
            touched: false,
            selected: 0,
            trace_visible: false,
        }
    }

    fn selection(&self) -> Selection {
        let town = self.workbench.town();
        Selection {
            order: town.order.as_slice().try_into().expect("fixed-town order"),
            bd: town.roads[3].road.cost,
            touched: self.touched,
            closed: town
                .roads
                .iter()
                .enumerate()
                .fold(0, |bits, (index, road)| {
                    bits | (u8::from(!road.open) << index)
                }),
        }
    }

    fn edit(&mut self, edit: RouteEdit) {
        if self.workbench.apply(edit).is_ok() {
            self.touched = true;
        }
    }

    fn choose(&mut self, x: f64, y: f64, click: bool) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        let x = x.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);
        if (WORKBENCH_STRIP..ROAD_STRIP).contains(&y) {
            if click {
                self.command(match ((x * 5.0) as usize).min(4) {
                    0 => ',',
                    1 => 'c',
                    2 => 'z',
                    3 => 't',
                    _ => '.',
                });
            }
        } else if y < ROAD_STRIP {
            self.edit(RouteEdit::Order(
                ORDERS[((x * 6.0) as usize).min(5)].to_vec(),
            ));
        } else if y < BUTTON_STRIP {
            self.edit(RouteEdit::RoadCost {
                from: 1,
                to: 3,
                cost: 1 + ((x * 9.0) as u32).min(8),
            });
        } else if click {
            self.command(if x < 0.5 { 'g' } else { 'i' });
        }
    }

    fn command(&mut self, ch: char) {
        let ch = ch.to_ascii_lowercase();
        match ch {
            'c' => {
                let road = self.workbench.town().roads[self.selected];
                self.edit(RouteEdit::RoadOpen {
                    from: road.road.from,
                    to: road.road.to,
                    open: !road.open,
                });
                return;
            }
            'z' => {
                if self.workbench.undo().is_ok_and(|changed| changed) {
                    self.touched = true;
                }
                return;
            }
            't' => {
                self.trace_visible = !self.trace_visible;
                if self.trace_visible && self.workbench.trace().is_none() {
                    let _ = self.workbench.start_trace(0, 3);
                }
                return;
            }
            ',' | '.' => {
                if self.trace_visible {
                    if self.workbench.trace().is_none() {
                        let _ = self.workbench.start_trace(0, 3);
                    }
                    if let Some(trace) = self.workbench.trace() {
                        let cursor = if ch == ',' {
                            trace.cursor().saturating_sub(1)
                        } else {
                            (trace.cursor() + 1).min(trace.events().len())
                        };
                        let _ = self.workbench.seek_trace(cursor);
                    }
                } else {
                    self.selected = (self.selected + if ch == ',' { 4 } else { 1 }) % 5;
                }
                return;
            }
            _ => {}
        }
        if matches!(ch, 'j' | 'J' | 'l' | 'L') {
            let bd = self.workbench.town().roads[3].road.cost;
            let cost = if matches!(ch, 'j' | 'J') {
                bd.saturating_sub(1).max(1)
            } else {
                (bd + 1).min(9)
            };
            self.edit(RouteEdit::RoadCost {
                from: 1,
                to: 3,
                cost,
            });
            return;
        }
        if let Some(index) = ch.to_digit(10).filter(|index| (1..=6).contains(index)) {
            self.edit(RouteEdit::Order(ORDERS[index as usize - 1].to_vec()));
            return;
        }
        if !matches!(ch, 'g' | 'G' | 'i' | 'I') {
            return;
        }
        self.edit(if ch == 'g' {
            RouteEdit::Greedy
        } else {
            RouteEdit::Improve
        });
    }
}

// Checkpoints are synthesized state, never evidence of a physical gesture.
// A framed version/length header and private-use data keep them separate from
// playable keys. Every restored snapshot passes the canonical validator.
fn pack_town(town: &RouteTownSnapshot) -> Option<u16> {
    if town.junctions != 4 || town.stops != [0, 1, 2, 3] || town.roads.len() != 5 {
        return None;
    }
    let order = ORDERS.iter().position(|order| town.order == *order)?;
    let base = RouteWorkbench::first_town().snapshot().current;
    let mut closed = 0u16;
    for (index, (road, expected)) in town.roads.iter().zip(&base.roads).enumerate() {
        if (road.road.from, road.road.to) != (expected.road.from, expected.road.to)
            || (index != 3 && road.road.cost != expected.road.cost)
        {
            return None;
        }
        closed |= u16::from(!road.open) << index;
    }
    let bd = town.roads[3].road.cost;
    if !(1..=9).contains(&bd) {
        return None;
    }
    Some(order as u16 | ((bd as u16 - 1) << 3) | (closed << 7))
}

fn unpack_town(value: u16) -> Option<RouteTownSnapshot> {
    if value >= 1 << 12 {
        return None;
    }
    let order = ORDERS.get((value & 7) as usize)?;
    let bd = ((value >> 3) & 15) + 1;
    if bd > 9 {
        return None;
    }
    let mut town = RouteWorkbench::first_town().snapshot().current;
    town.order = order.to_vec();
    town.roads[3].road.cost = u32::from(bd);
    for (index, road) in town.roads.iter_mut().enumerate() {
        road.open = value & (1 << (index + 7)) == 0;
    }
    Some(town)
}

fn checkpoint(session: &Session) -> Option<Vec<RoomInput>> {
    let snapshot = session.workbench.snapshot();
    let mut values = vec![
        1,
        0,
        snapshot.undo.len() as u16,
        pack_town(&snapshot.current)?,
    ];
    values.push(
        session.selected as u16
            | (u16::from(session.trace_visible) << 3)
            | (u16::from(session.touched) << 4)
            | (u16::from(snapshot.trace.is_some()) << 5),
    );
    for shift in [0, 15, 30, 45, 60] {
        values.push(((snapshot.revision >> shift) & 0x7FFF) as u16);
    }
    values.push(
        snapshot
            .trace
            .as_ref()
            .map_or(Some(0), |trace| u16::try_from(trace.cursor).ok())?,
    );
    for town in &snapshot.undo {
        values.push(pack_town(town)?);
    }
    values[1] = values.len() as u16;
    if values.iter().any(|&value| value > 0x7FFF) {
        return None;
    }
    let mut inputs = vec![RoomInput::Key {
        ch: char::from_u32(CHECKPOINT_START)?,
    }];
    inputs.extend(
        values.into_iter().map(|value| RoomInput::Key {
            ch: char::from_u32(CHECKPOINT_DATA + u32::from(value))
                .expect("private-use checkpoint data"),
        }),
    );
    inputs.push(RoomInput::Key {
        ch: char::from_u32(CHECKPOINT_END)?,
    });
    Some(inputs)
}

fn restore_checkpoint(inputs: &[RoomInput]) -> Option<Session> {
    if !(11..=11 + MAX_ROUTE_UNDO).contains(&inputs.len()) {
        return None;
    }
    let values: Vec<u16> = inputs
        .iter()
        .map(|input| match input {
            RoomInput::Key { ch } => u32::from(*ch)
                .checked_sub(CHECKPOINT_DATA)
                .filter(|value| *value <= 0x7FFF)
                .map(|value| value as u16),
            _ => None,
        })
        .collect::<Option<_>>()?;
    if values.len() < 11
        || values[0] != 1
        || usize::from(values[1]) != values.len()
        || usize::from(values[2]) > MAX_ROUTE_UNDO
        || values.len() != 11 + usize::from(values[2])
        || values[4] >= 64
        || values[4] & 7 >= 5
        || values[9] > 15
    {
        return None;
    }
    let mut revision = 0;
    for (value, shift) in values[5..10].iter().zip([0, 15, 30, 45, 60]) {
        revision |= u64::from(*value) << shift;
    }
    if values[4] & 32 == 0 && values[10] != 0 {
        return None;
    }
    let snapshot = RouteWorkbenchSnapshot {
        revision,
        current: unpack_town(values[3])?,
        undo: values[11..]
            .iter()
            .map(|&value| unpack_town(value))
            .collect::<Option<_>>()?,
        trace: None,
    };
    let mut workbench = RouteWorkbench::from_snapshot(snapshot).ok()?;
    if values[4] & 32 != 0 {
        workbench.start_trace(0, 3).ok()?;
        workbench.seek_trace(usize::from(values[10])).ok()?;
    }
    Some(Session {
        workbench,
        selected: usize::from(values[4] & 7),
        trace_visible: values[4] & 8 != 0,
        touched: values[4] & 16 != 0,
    })
}

#[derive(Debug)]
struct Experiment {
    selection: Selection,
    current: RouteTour,
    greedy: RouteTour,
    optimum: RouteTour,
    proposal: Option<RouteExchange>,
}

impl Experiment {
    fn of(selection: Selection) -> Result<Self, RouteError> {
        let mut snapshot = RouteWorkbench::first_town().snapshot();
        snapshot.current.order = selection.order.to_vec();
        snapshot.current.roads[3].road.cost = selection.bd;
        for (index, road) in snapshot.current.roads.iter_mut().enumerate() {
            road.open = selection.closed & (1 << index) == 0;
        }
        let workbench = RouteWorkbench::from_snapshot(snapshot).map_err(|_| RouteError::Tour)?;
        let comparison = workbench.compare()?;
        Ok(Self {
            selection,
            current: comparison.current,
            greedy: comparison.greedy,
            optimum: comparison.exact.tour,
            proposal: comparison.proposal,
        })
    }

    fn status(&self) -> String {
        let prefix = if self.selection.touched {
            "ORDER"
        } else {
            "DRAG:  ORDER"
        };
        let save = self.proposal.as_ref().map_or(0, |exchange| -exchange.delta);
        format!(
            "{prefix}={} cost={} opt={} save={} BD={}",
            order_text(&self.current.order),
            self.current.cost,
            self.optimum.cost,
            save,
            self.selection.bd
        )
    }
}

fn order_text(order: &[usize]) -> String {
    order
        .iter()
        .map(|&node| char::from(b'A' + node as u8))
        .collect()
}

fn label(surface: &mut dyn Surface, text: &str, x: i32, y: i32, scale: i32, mark: char) {
    if surface.safe_char_aspect() < 0.75 {
        for (index, character) in text.chars().enumerate() {
            surface.plot(x + index as i32, y, character);
        }
    } else {
        font::draw_text(surface, text, x, y, scale, mark);
    }
}

fn text_scale(width: usize, height: usize) -> i32 {
    ((width / 360).min(height / 320)).max(1) as i32
}

fn trace_line(session: &Session) -> String {
    let Some(trace) = session.workbench.trace() else {
        return "A>D SEARCH: STEP TO START".into();
    };
    let prefix = format!("A>D SEARCH {}/{}", trace.cursor(), trace.events().len());
    if let Some(result) = trace.result() {
        return match result {
            Ok(path) => format!(
                "{prefix}  PATH {} COST {}",
                order_text(&path.junctions),
                path.cost
            ),
            Err(_) => format!("{prefix}  NO OPEN PATH. OPEN ROADS"),
        };
    }
    match trace.visible_events().last() {
        Some(RouteEvent::Settled { junction, cost }) => {
            format!("{prefix}  {} FINAL {cost} FROM A", order_text(&[*junction]))
        }
        Some(RouteEvent::Relaxed { from, to, cost }) => format!(
            "{prefix}  {} TENTATIVE {cost} VIA {}",
            order_text(&[*to]),
            order_text(&[*from])
        ),
        None => format!("{prefix}  STEP TO BEGIN"),
    }
}

fn disconnected_line(session: &Session) -> String {
    let town = session.workbench.town();
    let roads = town
        .roads
        .iter()
        .filter(|road| road.open)
        .map(|road| road.road)
        .collect();
    let Ok(trace) = shortest_street_trace(town.junctions, roads, 0, 0) else {
        return "NO ROUND TRIP: ROAD NETWORK DISCONNECTED".into();
    };
    let missing: Vec<_> = town
        .stops
        .iter()
        .copied()
        .filter(|&stop| {
            !trace.events.iter().any(
                |event| matches!(event, RouteEvent::Settled { junction, .. } if *junction == stop),
            )
        })
        .collect();
    format!("NO ROUND TRIP: {} CUT OFF FROM A", order_text(&missing))
}

fn draw(surface: &mut dyn Surface, session: &Session, experiment: Option<&Experiment>) {
    let (width, height) = surface.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let scale = text_scale(width, height);
    let text_cells = surface.safe_char_aspect() < 0.75;
    let cost_gap = if text_cells { 2 } else { 8 * scale };
    let point = |x: f64, y: f64| {
        (
            (x * width.saturating_sub(1) as f64).round() as i32,
            (y * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let node = |id: usize| point(POINTS[id].0, POINTS[id].1);
    let route = experiment.map_or_else(
        || disconnected_line(session),
        |experiment| {
            format!(
                "{}-A  ROUND TRIP {}  EXACT BEST {}",
                order_text(&experiment.current.order),
                experiment.current.cost,
                experiment.optimum.cost
            )
        },
    );
    label(
        surface,
        "DELIVER B C D. RETURN TO A.",
        point(0.03, 0.18).0,
        point(0.03, 0.18).1,
        scale,
        '#',
    );
    label(
        surface,
        &route,
        point(0.03, 0.14).0,
        point(0.03, 0.14).1,
        scale,
        '#',
    );

    // All street walks expand through real edges. A double stroke encodes use
    // without relying on color, and a dot marks streets outside this tour.
    for (road_index, editable) in session.workbench.town().roads.iter().enumerate() {
        let road = &editable.road;
        let used = editable.open
            && experiment.is_some_and(|experiment| {
                experiment.current.walk.windows(2).any(|pair| {
                    (pair[0] == road.from && pair[1] == road.to)
                        || (pair[0] == road.to && pair[1] == road.from)
                })
            });
        let (ax, ay) = node(road.from);
        let (bx, by) = node(road.to);
        if editable.open {
            surface.line(ax, ay, bx, by, if used { '#' } else { '.' });
        } else {
            // A closed street has endpoint stubs and a central cross, rather
            // than an apparently traversable continuous road.
            let (mx, my) = ((ax + bx) / 2, (ay + by) / 2);
            surface.line(ax, ay, (3 * ax + bx) / 4, (3 * ay + by) / 4, '.');
            surface.line((ax + 3 * bx) / 4, (ay + 3 * by) / 4, bx, by, '.');
            surface.line(
                mx - 3 * scale,
                my - 3 * scale,
                mx + 3 * scale,
                my + 3 * scale,
                'x',
            );
            surface.line(
                mx - 3 * scale,
                my + 3 * scale,
                mx + 3 * scale,
                my - 3 * scale,
                'x',
            );
        }
        if used {
            surface.line(ax, ay + scale + 1, bx, by + scale + 1, '=');
        }
        let cost_label = format!(
            "{}{} {}",
            if road_index == session.selected {
                ">"
            } else {
                ""
            },
            ROAD_NAMES[road_index],
            road.cost
        );
        // BC's complete road-and-cost label goes left of its vertical road.
        let (lx, ly) = if road.from == 1 && road.to == 2 {
            (
                (ax + bx) / 2
                    - if text_cells {
                        cost_label.len() as i32 + 2
                    } else {
                        (cost_label.len() as i32 * 6 + 3) * scale
                    },
                (ay + by) / 2,
            )
        } else {
            let half_width = if text_cells {
                0
            } else {
                cost_label.len() as i32 * 3 * scale
            };
            let rise = if text_cells {
                0
            } else {
                (f64::from((ay - by).abs()) * f64::from(half_width)
                    / f64::from((ax - bx).abs().max(1)))
                .ceil() as i32
            };
            ((ax + bx) / 2 - half_width, (ay + by) / 2 - cost_gap - rise)
        };
        label(surface, &cost_label, lx, ly, scale, '@');
    }
    for id in 0..4 {
        let (x, y) = node(id);
        label(
            surface,
            &char::from(b'A' + id as u8).to_string(),
            x + if text_cells { 2 } else { 3 * scale },
            y - if text_cells { 1 } else { 3 * scale },
            scale,
            '@',
        );
        surface.plot(x, y, '@');
    }
    // A path may traverse a delivery before its scheduled service. The visible
    // stop order, not an invented straight-line AD street, identifies the tour.
    let proposal = if session.trace_visible {
        trace_line(session)
    } else {
        experiment.map_or_else(
            || "REOPEN CLOSED ROADS OR UNDO".to_string(),
            |experiment| {
                experiment.proposal.as_ref().map_or_else(
                    || "NO SHORTER EXCHANGE OFFERED".to_string(),
                    |exchange| {
                        format!(
                            "TRY {}-A  SAVE {}",
                            order_text(&exchange.tour.order),
                            -exchange.delta
                        )
                    },
                )
            },
        )
    };
    label(
        surface,
        &proposal,
        point(0.03, 0.625).0,
        point(0.03, 0.625).1,
        scale,
        '+',
    );
    for (index, text) in [
        if session.trace_visible {
            "BACK"
        } else {
            "ROAD <"
        },
        if session.workbench.town().roads[session.selected].open {
            "CLOSE"
        } else {
            "OPEN"
        },
        "UNDO",
        if session.trace_visible {
            "HIDE"
        } else {
            "SEARCH"
        },
        if session.trace_visible {
            "STEP"
        } else {
            "ROAD >"
        },
    ]
    .iter()
    .enumerate()
    {
        label(
            surface,
            text,
            point(index as f64 * 0.2 + 0.025, 0.66).0,
            point(0.0, 0.66).1,
            scale,
            '#',
        );
    }
    let selected = &session.workbench.town().roads[session.selected];
    let strip = format!("DRAG BD COST: {} (1..9)", session.selection().bd);
    let selection = format!(
        "SELECTED ROAD {}: {}  COST {}",
        ROAD_NAMES[session.selected],
        if selected.open { "OPEN" } else { "CLOSED" },
        selected.road.cost
    );
    surface.line(
        0,
        point(0.0, ROAD_STRIP).1,
        width as i32 - 1,
        point(0.0, ROAD_STRIP).1,
        '.',
    );
    label(
        surface,
        &strip,
        point(0.03, 0.75).0,
        point(0.03, 0.75).1,
        scale,
        '#',
    );
    label(
        surface,
        &selection,
        point(0.03, 0.8).0,
        point(0.03, 0.8).1,
        scale,
        '#',
    );
    surface.line(
        0,
        point(0.0, BUTTON_STRIP).1,
        width as i32 - 1,
        point(0.0, BUTTON_STRIP).1,
        '.',
    );
    surface.line(
        point(0.5, BUTTON_STRIP).0,
        point(0.5, BUTTON_STRIP).1,
        point(0.5, 1.0).0,
        point(0.5, 1.0).1,
        '.',
    );
    let nearest = experiment.map_or_else(
        || "NEAREST --".to_string(),
        |experiment| format!("NEAREST NEXT {}", experiment.greedy.cost),
    );
    let shorter = experiment.map_or_else(
        || "NO OFFER".to_string(),
        |experiment| {
            experiment.proposal.as_ref().map_or_else(
                || "NO OFFER".to_string(),
                |exchange| format!("USE SHORTER: -{}", -exchange.delta),
            )
        },
    );
    label(
        surface,
        &nearest,
        point(0.05, 0.87).0,
        point(0.05, 0.87).1,
        scale,
        '#',
    );
    label(
        surface,
        &shorter,
        point(0.55, 0.87).0,
        point(0.55, 0.87).1,
        scale,
        '#',
    );
}

/// A playable exact four-stop comparison with street editing and solver playback.
#[derive(Debug, Default)]
pub struct RouteLab {
    seed: u64,
}

impl RouteLab {
    /// Open the documented four-stop trap.
    #[must_use]
    pub const fn new() -> Self {
        Self { seed: 0 }
    }

    /// Vary the initial BD road cost without changing the map or solver rules.
    #[must_use]
    pub const fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn selection(&self, inputs: &[RoomInput]) -> Selection {
        self.selection_from(bounded_inputs(inputs))
    }

    fn selection_from(&self, inputs: &[RoomInput]) -> Selection {
        self.session_from(inputs).selection()
    }

    /// Export the current native construction state without changing the visit.
    /// Retained checkpoint events are state, not physical input evidence.
    pub fn native_snapshot(&self, inputs: &[RoomInput]) -> RouteWorkbenchSnapshot {
        self.session_from(bounded_inputs(inputs))
            .workbench
            .snapshot()
    }

    fn session_from(&self, inputs: &[RoomInput]) -> Session {
        let mut session = Session::new(self.seed);
        let mut index = 0;
        while index < inputs.len() {
            if matches!(inputs[index], RoomInput::Key { ch } if u32::from(ch) == CHECKPOINT_START) {
                let end = inputs[index + 1..].iter().position(|input| matches!(input, RoomInput::Key { ch } if u32::from(*ch) == CHECKPOINT_END)).map(|offset| index + 1 + offset);
                let Some(end) = end else {
                    break;
                };
                if let Some(restored) = restore_checkpoint(&inputs[index + 1..end]) {
                    session = restored;
                }
                index = end + 1;
                continue;
            }
            match inputs[index] {
                RoomInput::PointerDown { x, y, t } if t.is_finite() => session.choose(x, y, true),
                RoomInput::PointerMove { x, y, t } if t.is_finite() => session.choose(x, y, false),
                RoomInput::Key { ch } => session.command(ch),
                _ => {}
            }
            index += 1;
        }
        session
    }

    fn experiment(&self, inputs: &[RoomInput]) -> Result<Experiment, RouteError> {
        Experiment::of(self.selection(inputs))
    }
}

impl Room for RouteLab {
    fn render(&self, surface: &mut dyn Surface, t: f64) {
        self.render_input(surface, t, &[]);
    }

    fn render_input(&self, surface: &mut dyn Surface, _t: f64, inputs: &[RoomInput]) {
        let session = self.session_from(bounded_inputs(inputs));
        let experiment = Experiment::of(session.selection()).ok();
        draw(surface, &session, experiment.as_ref());
    }

    fn render_poked(&self, surface: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        self.render_input(surface, t, &inputs_from_pokes(pokes, t));
    }

    fn compact_inputs(&self, inputs: &mut Vec<RoomInput>) {
        // Cancellation can append two events. Fold before either can displace
        // a road edit or the baseline for a relative keyboard cost adjustment.
        if inputs.len() < MAX_ROOM_INPUTS.saturating_sub(2) {
            return;
        }
        let mut split = inputs.len().saturating_sub(MAX_ROOM_INPUTS / 2);
        let mut index = 0;
        let mut incomplete_frame = false;
        // A frame's payload must never become ordinary keys because its
        // opening marker was folded away. Move the cut past complete frames.
        // For an unfinished frame, carry an inert opening barrier instead.
        while index < split {
            if matches!(inputs[index], RoomInput::Key { ch } if u32::from(ch) == CHECKPOINT_START) {
                let end = inputs[index + 1..].iter().position(|input| matches!(input, RoomInput::Key { ch } if u32::from(*ch) == CHECKPOINT_END)).map(|offset| index + 1 + offset);
                let Some(end) = end else {
                    incomplete_frame = true;
                    break;
                };
                split = split.max(end + 1);
                index = end + 1;
            } else {
                index += 1;
            }
        }
        let state = self.session_from(&inputs[..split]);
        let baseline = Session::new(self.seed);
        let mut compacted = if state.workbench.snapshot() == baseline.workbench.snapshot()
            && !state.touched
            && state.selected == 0
            && !state.trace_visible
        {
            Vec::new()
        } else {
            let Some(encoded) = checkpoint(&state) else {
                return;
            };
            encoded
        };
        if incomplete_frame {
            // Version zero is deliberately invalid. Even a suffix that looks
            // like a valid checkpoint cannot turn this barrier into a restore.
            compacted.extend([
                RoomInput::Key {
                    ch: char::from_u32(CHECKPOINT_START).expect("private-use frame marker"),
                },
                RoomInput::Key {
                    ch: char::from_u32(CHECKPOINT_DATA).expect("private-use inert frame version"),
                },
            ]);
        }
        compacted.extend_from_slice(&inputs[split..]);
        *inputs = compacted;
    }

    fn status(&self, t: f64) -> Option<String> {
        self.status_input(t, &[])
    }

    fn status_input(&self, _t: f64, inputs: &[RoomInput]) -> Option<String> {
        let session = self.session_from(bounded_inputs(inputs));
        let mut status = Experiment::of(session.selection()).map_or_else(
            |_| "NO ROUND TRIP: road network disconnected".to_string(),
            |experiment| experiment.status(),
        );
        if session.trace_visible {
            status = trace_line(&session);
        } else if session.selected != 0 || session.selection().closed != 0 {
            let road = session.workbench.town().roads[session.selected];
            status.push_str(&format!(
                " {}={}",
                ROAD_NAMES[session.selected],
                if road.open { "open" } else { "closed" }
            ));
        }
        Some(status)
    }

    fn numeric_readouts(&self, _t: f64) -> Option<Vec<NumericReadout>> {
        let readouts = self
            .experiment(&[])
            .ok()
            .into_iter()
            .flat_map(|experiment| {
                [
                    NumericReadout::new(
                        ReadoutId::new(0),
                        "round-trip cost",
                        f64::from(experiment.current.cost),
                    ),
                    NumericReadout::new(
                        ReadoutId::new(1),
                        "minimum round-trip cost",
                        f64::from(experiment.optimum.cost),
                    ),
                    NumericReadout::new(
                        ReadoutId::new(2),
                        "nearest-next round-trip cost",
                        f64::from(experiment.greedy.cost),
                    ),
                ]
                .into_iter()
                .flatten()
            })
            .collect();
        Some(readouts)
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: DELIVERY ORDER; BD COST; OPEN/CLOSE ROAD")
    }

    fn reveal(&self) -> &'static str {
        "The cheapest next delivery can leave a costly return. With road BD costing 3, delivery order A-B-C-D-A has round-trip cost 9. Choosing A-B-D-C-A reduces it to 8. Each leg follows a cheapest open-road path, which can pass or revisit delivery junctions before their scheduled delivery. The delivery order is not the physical street walk. The three orders up to reversal cost 9, 8, and 11, so 8 is the exact minimum for these integer costs. Editing a road asks a new question: choosing the nearest next delivery can also be best. A two-edge exchange reverses part of the delivery order; its two changed connections need not be direct roads. An improving exchange proves a saving; a pass with none proves only a local optimum. The separate exact subset calculation checks every stop order. A drawn crossing or a short-looking road proves neither."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Every required delivery must be reachable from the depot for a round trip. An unused isolated junction need not prevent that trip. Dijkstra settles junctions by their smallest tentative cost; positive road costs make a settled distance final. Search shows actual decisions from A to D. Its costs are cumulative distances from A, not single-road costs. A tentative value may improve; a final value will not. Editing a road invalidates the calculation. Delivery legs follow open roads, including the return to A, and may pass or revisit other deliveries.",
            "For nonadjacent edges (a,b) and (c,d), reversing the intervening segment changes cost by d(a,c)+d(b,d)-d(a,b)-d(c,d). This formula needs symmetric distances: one-way roads would also change internal costs.",
            "Subset dynamic programming keeps the cheapest depot-to-j route for each set of visited deliveries. Extending every smaller set and adding the final return proves the optimum for the declared finite integer-cost problem. Its state count is not the number of complete tours searched.",
        ]
    }

    fn goal(&self) -> Option<&'static str> {
        Some("Choose a delivery order whose round-trip cost matches the minimum.")
    }

    fn goal_met(&self, _t: f64, inputs: &[RoomInput]) -> bool {
        self.experiment(inputs).is_ok_and(|experiment| {
            experiment.selection.touched && experiment.current.cost == experiment.optimum.cost
        })
    }

    fn motif(&self) -> Option<Motif> {
        Some(Motif {
            key: "route lab",
            root: 146.83,
            tempo: 100,
            line: &[0, 7, 4, 12, 4, 7, 0, 0],
            encodes: "a departure, three deliveries, and the return to the depot",
        })
    }

    fn parameter_sound(&self, _t: f64, inputs: &[RoomInput]) -> Option<ParametricSound> {
        let experiment = self.experiment(inputs).ok()?;
        if !experiment.selection.touched {
            return None;
        }
        // The ratio measures detour above the exact optimum, with no tuning
        // claim. At optimum the two voices meet at ratio one.
        ParametricSound::new(
            146.83,
            experiment.current.cost as f32 / experiment.optimum.cost as f32,
            0.025,
        )
    }

    fn sound_input(&self, t: f64, inputs: &[RoomInput]) -> SoundSpec {
        self.parameter_sound(t, inputs)
            .map_or_else(|| self.sound(t), ParametricSound::snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Canvas, Raster, all_rooms, room_by_id, to_mono};

    fn down(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerDown { x, y, t: 0.0 }
    }
    fn move_to(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerMove { x, y, t: 0.0 }
    }

    fn keys(text: &str) -> Vec<RoomInput> {
        text.chars().map(|ch| RoomInput::Key { ch }).collect()
    }

    fn assert_session_eq(actual: &Session, expected: &Session) {
        assert_eq!(actual.workbench.snapshot(), expected.workbench.snapshot());
        assert_eq!(actual.touched, expected.touched);
        assert_eq!(actual.selected, expected.selected);
        assert_eq!(actual.trace_visible, expected.trace_visible);
    }

    #[test]
    fn closures_reopen_and_undo_change_actual_streets_without_false_optima() {
        let room = RouteLab::new();
        let closed = room.session_from(&keys("c"));
        assert!(!closed.workbench.town().roads[0].open);
        let comparison = closed.workbench.compare().unwrap();
        assert!(
            !comparison
                .current
                .walk
                .windows(2)
                .any(|pair| pair == [0, 1] || pair == [1, 0])
        );
        assert!(
            room.status_input(0.0, &keys("c"))
                .unwrap()
                .contains("AB=closed")
        );
        assert_eq!(
            room.selection(&keys("cc")),
            Selection {
                touched: true,
                ..room.selection(&[])
            }
        );
        assert_eq!(
            room.selection(&keys("cz")),
            Selection {
                touched: true,
                ..room.selection(&[])
            }
        );
        let disconnected = keys("c.c");
        assert!(room.experiment(&disconnected).is_err());
        assert!(!room.goal_met(0.0, &disconnected));
        assert!(room.parameter_sound(0.0, &disconnected).is_none());
        let status = room.status_input(0.0, &disconnected).unwrap();
        assert!(status.contains("disconnected"));
        assert!(!status.contains("opt="));
        let mut canvas = Canvas::new(80, 40);
        room.render_input(&mut canvas, 0.0, &disconnected);
        assert!(canvas.to_text().contains("NO ROUND TRIP"));
        assert!(room.experiment(&keys("c.cz")).is_ok());
    }

    #[test]
    fn trace_reveals_real_prefix_only_on_commands_and_invalidates_after_edits() {
        let room = RouteLab::new();
        let start = room.session_from(&keys("t"));
        assert_eq!(start.workbench.trace().unwrap().cursor(), 0);
        assert!(start.workbench.trace().unwrap().result().is_none());
        let partial = room.session_from(&keys("t..."));
        let trace = partial.workbench.trace().unwrap();
        assert_eq!(trace.cursor(), 3);
        assert_eq!(trace.visible_events(), &trace.events()[..3]);
        assert!(trace.result().is_none());
        assert_eq!(
            room.status_input(0.0, &keys("t...")),
            room.status_input(0.99, &keys("t..."))
        );
        assert_eq!(
            room.session_from(&keys("t...,"))
                .workbench
                .trace()
                .unwrap()
                .cursor(),
            2
        );
        let complete = room.session_from(&keys("t...................."));
        assert_eq!(
            complete
                .workbench
                .trace()
                .unwrap()
                .result()
                .unwrap()
                .as_ref()
                .unwrap()
                .cost,
            4
        );
        assert!(trace_line(&complete).contains("COST 4"));
        let edited = room.session_from(&keys("t...l"));
        assert!(edited.workbench.trace().is_none());
        assert!(trace_line(&edited).contains("STEP TO START"));
        let restarted = room.session_from(&keys("t...l."));
        assert_eq!(restarted.workbench.trace().unwrap().cursor(), 1);
        assert_eq!(
            restarted.workbench.trace().unwrap().snapshot().revision,
            restarted.workbench.revision()
        );
        let disconnected = room.session_from(&keys("c.ct...................."));
        assert!(
            disconnected
                .workbench
                .trace()
                .unwrap()
                .result()
                .unwrap()
                .is_err()
        );
        assert!(trace_line(&disconnected).contains("NO OPEN PATH"));
    }

    #[test]
    fn pointer_workbench_buttons_match_keys_and_moves_or_lifts_do_not_issue_commands() {
        let room = RouteLab::new();
        for (x, command) in [(0.1, ','), (0.3, 'c'), (0.5, 'z'), (0.7, 't'), (0.9, '.')] {
            let mut pointer = keys("2l");
            pointer.push(down(x, 0.67));
            let mut keyboard = keys("2l");
            keyboard.push(RoomInput::Key { ch: command });
            assert_session_eq(&room.session_from(&pointer), &room.session_from(&keyboard));
            let before = room.session_from(&pointer);
            for next_x in [0.1, 0.3, 0.5, 0.7, 0.9] {
                pointer.push(move_to(next_x, 0.67));
                pointer.push(RoomInput::PointerUp {
                    x: next_x,
                    y: 0.67,
                    t: 0.0,
                });
            }
            assert_session_eq(&room.session_from(&pointer), &before);
        }
        let mut drag = vec![down(0.4, 0.78)];
        let before = room.session_from(&drag);
        drag.extend([
            move_to(0.3, 0.67),
            move_to(0.5, 0.67),
            move_to(0.7, 0.67),
            RoomInput::PointerUp {
                x: 0.7,
                y: 0.67,
                t: 0.0,
            },
        ]);
        assert_session_eq(&room.session_from(&drag), &before);
    }

    #[test]
    fn full_checkpoint_retains_bounded_undo_revision_closures_and_trace_after_long_replay() {
        let room = RouteLab::new();
        let mut retained = Vec::new();
        let mut complete = Vec::new();
        let commands = keys("2lc.t...,...tzj6.czt....,giz");
        for index in 0..520 {
            let input = commands[index % commands.len()];
            room.compact_inputs(&mut retained);
            retained.push(input);
            complete.push(input);
            assert!(retained.len() <= MAX_ROOM_INPUTS);
            assert_session_eq(&room.session_from(&retained), &room.session_from(&complete));
        }
        for _ in 0..MAX_ROUTE_UNDO + 2 {
            room.compact_inputs(&mut retained);
            retained.push(RoomInput::Key { ch: 'z' });
            complete.push(RoomInput::Key { ch: 'z' });
            assert_session_eq(&room.session_from(&retained), &room.session_from(&complete));
        }
        let state = room.session_from(&complete);
        let encoded = checkpoint(&state).unwrap();
        assert!(encoded.len() <= 29);
        assert!(encoded.iter().all(|input| matches!(input, RoomInput::Key { ch } if (0xE000..=0xF8FF).contains(&u32::from(*ch)) || (0xF0000..=0xFFFFD).contains(&u32::from(*ch)))));
        assert_session_eq(&room.session_from(&encoded), &state);
    }

    #[test]
    fn framed_hostile_or_incomplete_checkpoints_never_execute_embedded_play_keys() {
        let room = RouteLab::new();
        let original = checkpoint(&room.session_from(&keys("2lc.t..."))).unwrap();
        for length in 1..original.len() {
            assert_session_eq(&room.session_from(&original[..length]), &Session::new(0));
        }
        for (index, value) in [
            (1, 2),
            (2, 1),
            (3, 17),
            (4, 4096),
            (5, 7),
            (10, 16),
            (11, 32767),
        ] {
            let mut invalid = original.clone();
            invalid[index] = RoomInput::Key {
                ch: char::from_u32(CHECKPOINT_DATA + value).unwrap(),
            };
            assert_session_eq(&room.session_from(&invalid), &Session::new(0));
        }
        let hostile = [
            original[0],
            RoomInput::Key { ch: 'l' },
            *original.last().unwrap(),
        ];
        assert_session_eq(&room.session_from(&hostile), &Session::new(0));
        let mut followed = hostile.to_vec();
        followed.push(RoomInput::Key { ch: 'l' });
        assert_session_eq(
            &room.session_from(&followed),
            &room.session_from(&keys("l")),
        );
        for order in ORDERS {
            for bd in 1..=9 {
                for closed in 0..32 {
                    let mut snapshot = RouteWorkbench::first_town().snapshot().current;
                    snapshot.order = order.to_vec();
                    snapshot.roads[3].road.cost = bd;
                    for (index, road) in snapshot.roads.iter_mut().enumerate() {
                        road.open = closed & (1 << index) == 0;
                    }
                    assert_eq!(
                        unpack_town(pack_town(&snapshot).unwrap()).unwrap(),
                        snapshot
                    );
                }
            }
        }
    }

    #[test]
    fn compaction_keeps_complete_malformed_frames_atomic_and_preserves_real_keys_afterward() {
        let room = RouteLab::new();
        let mut inputs = keys("2");
        inputs.extend([RoomInput::PointerCancel; 9]);
        inputs.push(RoomInput::Key {
            ch: char::from_u32(CHECKPOINT_START).unwrap(),
        });
        inputs.extend(keys(&"l".repeat(80)));
        inputs.push(RoomInput::Key {
            ch: char::from_u32(CHECKPOINT_END).unwrap(),
        });
        inputs.extend([RoomInput::Key { ch: 'l' }, RoomInput::PointerCancel]);
        assert_eq!(inputs.len(), MAX_ROOM_INPUTS - 2);
        let expected = room.session_from(&inputs);
        assert_eq!(expected.selection().bd, 4);
        assert_eq!(expected.selection().order, ORDERS[1]);
        room.compact_inputs(&mut inputs);
        assert!(inputs.len() < MAX_ROOM_INPUTS - 2);
        assert_session_eq(&room.session_from(&inputs), &expected);
        inputs.push(RoomInput::Key { ch: 'z' });
        assert_session_eq(
            &room.session_from(&inputs),
            &room.session_from(&keys("2lz")),
        );
    }

    #[test]
    fn repeated_compaction_preserves_unterminated_frame_isolation_until_its_end_marker() {
        let room = RouteLab::new();
        let mut retained = keys("2");
        retained.push(RoomInput::Key {
            ch: char::from_u32(CHECKPOINT_START).unwrap(),
        });
        retained.extend([RoomInput::PointerCancel; 44]);
        retained.extend(keys(&"l".repeat(48)));
        let mut complete = retained.clone();
        let expected = room.session_from(&complete);
        assert_eq!(expected.selection().bd, 3);
        for _ in 0..190 {
            room.compact_inputs(&mut retained);
            let input = RoomInput::Key { ch: 'l' };
            retained.push(input);
            complete.push(input);
            assert!(retained.len() <= MAX_ROOM_INPUTS);
            assert_session_eq(&room.session_from(&retained), &expected);
        }
        // A syntactically valid body cannot repair the carried inert barrier
        // and import a town that the original unfinished frame would refuse.
        let other = checkpoint(&room.session_from(&keys("llllll"))).unwrap();
        for input in &other[1..] {
            room.compact_inputs(&mut retained);
            retained.push(*input);
            complete.push(*input);
            assert_session_eq(&room.session_from(&retained), &room.session_from(&complete));
        }
        assert_session_eq(&room.session_from(&retained), &expected);
        retained.push(RoomInput::Key { ch: 'l' });
        complete.push(RoomInput::Key { ch: 'l' });
        assert_session_eq(&room.session_from(&retained), &room.session_from(&complete));
        assert_eq!(room.selection(&retained).bd, 4);
    }

    #[test]
    fn oversized_public_room_input_never_admits_payload_keys_from_a_cut_frame() {
        let room = RouteLab::new();
        let mut hostile = keys("2");
        hostile.push(RoomInput::Key {
            ch: char::from_u32(CHECKPOINT_START).unwrap(),
        });
        hostile.extend(keys(&"l".repeat(MAX_ROOM_INPUTS + 40)));
        assert_eq!(room.status_input(0.0, &hostile), room.status(0.0));
        assert!(!room.goal_met(0.0, &hostile));
        assert!(room.parameter_sound(0.0, &hostile).is_none());
        let mut baseline = Canvas::new(80, 40);
        let mut actual = Canvas::new(80, 40);
        room.render(&mut baseline, 0.0);
        room.render_input(&mut actual, 0.0, &hostile);
        assert_eq!(actual.to_text(), baseline.to_text());
        hostile.push(RoomInput::Key {
            ch: char::from_u32(CHECKPOINT_END).unwrap(),
        });
        hostile.extend(keys("l2"));
        let real = keys("l2");
        assert_eq!(
            room.status_input(0.0, &hostile),
            room.status_input(0.0, &real)
        );
        assert_eq!(
            room.parameter_sound(0.0, &hostile),
            room.parameter_sound(0.0, &real)
        );
        let mut actual = Canvas::new(80, 40);
        room.render_input(&mut actual, 0.0, &hostile);
        let mut expected = Canvas::new(80, 40);
        room.render_input(&mut expected, 0.0, &real);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn presentation_distinguishes_delivery_order_road_cost_and_selected_road() {
        let room = RouteLab::new();
        let mut canvas = Canvas::new(100, 50);
        room.render(&mut canvas, 0.0);
        let text = canvas.to_text();
        for reading in [
            "ABCD-A  ROUND TRIP 9  EXACT BEST 8",
            "DELIVER B C D. RETURN TO A.",
            "NEAREST NEXT 9",
            "USE SHORTER: -1",
            "DRAG BD COST: 3 (1..9)",
            "SELECTED ROAD AB: OPEN  COST 1",
            ">AB 1",
            "ROAD <",
            "ROAD >",
            "SEARCH",
        ] {
            assert!(text.contains(reading), "missing player reading: {reading}");
        }
        let input = keys(".c");
        let before = room.session_from(&input).workbench.snapshot();
        let mut closed = Canvas::new(100, 50);
        room.render_input(&mut closed, 0.75, &input);
        assert!(
            closed
                .to_text()
                .contains("SELECTED ROAD AC: CLOSED  COST 2")
        );
        assert!(closed.to_text().contains("DRAG BD COST: 3 (1..9)"));
        assert_eq!(room.session_from(&input).workbench.snapshot(), before);
        assert!(
            room.reveal()
                .contains("delivery order is not the physical street walk")
        );
        assert!(room.deep_cuts()[0].contains("cumulative distances from A"));
        assert!(!room.reveal().contains("Exchanging BC and DA"));
    }

    #[test]
    fn disconnected_copy_names_unreachable_deliveries_and_offers_reversible_repair() {
        let room = RouteLab::new();
        for (commands, missing) in [("c.c", "BCD"), ("...c.c", "D"), ("c.c.c.c.c", "BCD")] {
            let inputs = keys(commands);
            let mut canvas = Canvas::new(100, 50);
            room.render_input(&mut canvas, 0.0, &inputs);
            let text = canvas.to_text();
            assert!(
                text.contains(&format!("NO ROUND TRIP: {missing} CUT OFF FROM A")),
                "{commands}: {text}"
            );
            assert!(text.contains("REOPEN CLOSED ROADS OR UNDO"));
            assert!(text.contains("NO OFFER"));
            assert!(!text.contains("EXACT BEST"));
            assert!(room.parameter_sound(0.0, &inputs).is_none());
            assert!(!room.goal_met(0.0, &inputs));
        }
    }

    #[test]
    fn search_narration_distinguishes_tentative_and_final_cumulative_costs() {
        let room = RouteLab::new();
        let mut session = room.session_from(&keys("t"));
        assert!(trace_line(&session).contains("STEP TO BEGIN"));
        let events = session.workbench.trace().unwrap().events().to_vec();
        for (index, event) in events.iter().enumerate() {
            session.workbench.seek_trace(index + 1).unwrap();
            if session.workbench.trace().unwrap().completed() {
                break;
            }
            let text = trace_line(&session);
            match event {
                RouteEvent::Settled { junction, cost } => assert!(
                    text.contains(&format!("{} FINAL {cost} FROM A", order_text(&[*junction])))
                ),
                RouteEvent::Relaxed { from, to, cost } => assert!(text.contains(&format!(
                    "{} TENTATIVE {cost} VIA {}",
                    order_text(&[*to]),
                    order_text(&[*from])
                ))),
            }
            assert!(!text.contains("PATH"));
        }
        session.workbench.seek_trace(events.len()).unwrap();
        assert!(trace_line(&session).contains("PATH ABD COST 4"));
        session.command('l');
        assert!(trace_line(&session).contains("STEP TO START"));
        assert!(!trace_line(&session).contains("PATH"));
    }

    #[test]
    fn first_town_improves_exactly_and_greedy_can_also_be_best() {
        let room = RouteLab::new();
        assert_eq!(
            room.status(0.0).as_deref(),
            Some("DRAG:  ORDER=ABCD cost=9 opt=8 save=1 BD=3")
        );
        assert!(!room.goal_met(0.0, &[]));
        let improved = [down(0.75, 0.92)];
        assert_eq!(
            room.status_input(0.0, &improved).as_deref(),
            Some("ORDER=ABDC cost=8 opt=8 save=0 BD=3")
        );
        assert!(room.goal_met(0.0, &improved));
        let repeated = [down(0.75, 0.92), down(0.75, 0.92)];
        assert_eq!(
            room.status_input(0.0, &improved),
            room.status_input(0.0, &repeated)
        );
        let back = [down(0.75, 0.92), down(0.25, 0.92)];
        assert_eq!(
            room.status_input(0.0, &back).as_deref(),
            Some("ORDER=ABCD cost=9 opt=8 save=1 BD=3")
        );
        let changed = [down(0.25, 0.5), move_to(0.5, 0.78)];
        assert_eq!(
            room.status_input(0.0, &changed).as_deref(),
            Some("ORDER=ABDC cost=9 opt=9 save=0 BD=5")
        );
        assert!(room.goal_met(0.0, &changed));
        let changed_greedy = [down(0.5, 0.78), down(0.25, 0.92)];
        assert_eq!(
            room.status_input(0.0, &changed_greedy).as_deref(),
            Some("ORDER=ABCD cost=9 opt=9 save=0 BD=5")
        );
    }

    #[test]
    fn map_order_road_cost_and_button_regions_are_separate() {
        let room = RouteLab::new();
        for (index, order) in ORDERS.iter().enumerate() {
            let input = [down((index as f64 + 0.5) / 6.0, 0.40)];
            let experiment = room.experiment(&input).unwrap();
            assert_eq!(&experiment.current.order, order);
            assert_eq!(experiment.selection.bd, 3);
            assert!(experiment.status().chars().count() <= 56);
        }
        for cost in 1..=9 {
            let input = [down((f64::from(cost) - 0.5) / 9.0, 0.78)];
            let experiment = room.experiment(&input).unwrap();
            assert_eq!(experiment.selection.bd, cost);
            assert_eq!(experiment.current.order, ORDERS[0]);
            assert_eq!(
                room.session_from(&input).workbench.town().roads[3]
                    .road
                    .cost,
                cost
            );
        }
        let move_over_button = [down(0.01, 0.4), move_to(0.75, 0.92)];
        assert_eq!(room.selection(&move_over_button).order, ORDERS[0]);
        let lift_over_button = [
            down(0.01, 0.4),
            RoomInput::PointerUp {
                x: 0.75,
                y: 0.92,
                t: 0.0,
            },
        ];
        assert_eq!(room.selection(&lift_over_button).order, ORDERS[0]);
        assert_eq!(room.selection(&[down(0.0, ROAD_STRIP)]).bd, 1);
        assert_eq!(room.selection(&[down(1.0, ROAD_STRIP)]).bd, 9);
        assert_eq!(room.selection(&[down(1.0, BUTTON_STRIP)]).order, ORDERS[1]);
    }

    #[test]
    fn keyboard_road_edits_are_bounded_and_recompute_the_comparison() {
        let room = RouteLab::new();
        let edited = [
            RoomInput::Key { ch: 'i' },
            RoomInput::Key { ch: 'l' },
            RoomInput::Key { ch: 'L' },
        ];
        let experiment = room.experiment(&edited).unwrap();
        assert_eq!(experiment.selection.bd, 5);
        assert_eq!(experiment.current.order, ORDERS[1]);
        assert_eq!(experiment.current.cost, 9);
        assert_eq!(experiment.optimum.cost, 9);
        assert!(experiment.proposal.is_none());
        assert_eq!(
            room.selection(&edited),
            room.selection(&[down(0.25, 0.5), down(0.5, 0.78)])
        );

        let restored = [
            RoomInput::Key { ch: 'i' },
            RoomInput::Key { ch: 'l' },
            RoomInput::Key { ch: 'L' },
            RoomInput::Key { ch: 'j' },
            RoomInput::Key { ch: 'J' },
            RoomInput::Key { ch: 'g' },
        ];
        let experiment = room.experiment(&restored).unwrap();
        assert_eq!(experiment.selection.bd, 3);
        assert_eq!(experiment.current.cost, 9);
        assert_eq!(experiment.optimum.cost, 8);
        assert_eq!(experiment.proposal.unwrap().delta, -1);
        for (key, expected) in [('j', 1), ('J', 1), ('l', 9), ('L', 9)] {
            let inputs = vec![RoomInput::Key { ch: key }; 20];
            let selection = room.selection(&inputs);
            assert_eq!(selection.bd, expected);
            assert!(selection.touched);
            assert_eq!(selection.order, ORDERS[0]);
        }
    }

    #[test]
    fn digital_keys_and_pointer_actions_replay_identical_truth_and_picture() {
        let room = RouteLab::new();
        let pointer = [down(0.75, 0.92)];
        let keys = [RoomInput::Key { ch: 'i' }];
        assert_eq!(room.selection(&pointer), room.selection(&keys));
        assert_eq!(
            room.status_input(0.0, &pointer),
            room.status_input(0.8, &keys)
        );
        let mut first = Canvas::new(80, 40);
        let mut second = Canvas::new(80, 40);
        room.render_input(&mut first, 0.0, &pointer);
        room.render_input(&mut second, 0.8, &keys);
        assert_eq!(first.to_text(), second.to_text());
        assert!(first.to_text().contains("ABDC-A  ROUND TRIP 8"));
        let pokes = [(0.25, 0.4), (0.5, 0.78), (0.25, 0.92)];
        let input = inputs_from_pokes(&pokes, 0.0);
        let mut poked = Canvas::new(80, 40);
        let mut replayed = Canvas::new(80, 40);
        room.render_poked(&mut poked, 0.0, &pokes);
        room.render_input(&mut replayed, 0.0, &input);
        assert_eq!(poked.to_text(), replayed.to_text());
        assert_eq!(
            room.status_input(0.0, &[RoomInput::Key { ch: '2' }]),
            room.status_input(0.0, &pointer)
        );
        assert_eq!(
            room.status_input(
                0.0,
                &[RoomInput::Key { ch: 'i' }, RoomInput::Key { ch: 'g' }]
            ),
            room.status_input(0.0, &[down(0.01, 0.4)])
        );
    }

    #[test]
    fn edits_invalidate_costs_and_trials_stay_inside_bounded_history() {
        let room = RouteLab::new();
        let improved = down(0.75, 0.92);
        let changed = [improved, down(1.0, 0.78)];
        let experiment = room.experiment(&changed).unwrap();
        assert_eq!(experiment.current.cost, 9);
        assert_eq!(experiment.optimum.cost, 9);
        assert!(experiment.proposal.is_none());
        assert_eq!(experiment.selection.bd, 9);
        let mut long = vec![down(0.0, 0.78); MAX_ROOM_INPUTS + 20];
        long.extend_from_slice(&changed);
        assert_eq!(
            room.selection(&long),
            room.selection(&long[long.len() - MAX_ROOM_INPUTS..])
        );
        let unchanged = [
            RoomInput::Key { ch: '?' },
            RoomInput::Wheel {
                delta: f64::INFINITY,
            },
            RoomInput::PointerCancel,
        ];
        assert_eq!(room.selection(&unchanged), room.selection(&[]));
        for input in [
            down(f64::NAN, 0.3),
            down(0.5, f64::INFINITY),
            RoomInput::PointerDown {
                x: 0.5,
                y: 0.5,
                t: f64::NAN,
            },
            RoomInput::PointerMove {
                x: 0.5,
                y: 0.5,
                t: f64::INFINITY,
            },
        ] {
            assert_eq!(room.selection(&[input]), room.selection(&[]));
        }
        assert_eq!(room.selection(&[down(-100.0, -100.0)]).order, ORDERS[0]);
        assert_eq!(room.selection(&[down(100.0, 0.78)]).bd, 9);
    }

    #[test]
    fn registry_doors_color_free_response_and_numeric_channels_are_live() {
        let room = room_by_id("route-lab").unwrap();
        assert_eq!(all_rooms().last().unwrap().meta().id, "route-lab");
        assert_eq!(room.meta().title, "Route Lab");
        assert!(room.citation().contains("Held and Karp"));
        let readouts = room.numeric_readouts(0.0).unwrap();
        assert_eq!(
            readouts
                .iter()
                .map(|readout| (readout.id().get(), readout.value()))
                .collect::<Vec<_>>(),
            vec![(0, 9.0), (1, 8.0), (2, 9.0)]
        );
        let mut before = Raster::with_accent(120, 70, room.meta().accent);
        let mut after = Raster::with_accent(120, 70, room.meta().accent);
        room.render(&mut before, 0.35);
        room.render_poked(&mut after, 0.35, &[(0.5, 0.5)]);
        assert_ne!(to_mono(&before), to_mono(&after));
        assert_ne!(room.status(0.0), room.status_input(0.0, &[down(0.5, 0.5)]));
        let opened = RouteLab::new();
        assert_eq!(opened.status(f64::NAN), opened.status(0.0));
        assert_ne!(opened.status(0.0), RouteLab::new_with(1).status(0.0));
        for (width, height) in [(0, 0), (1, 1), (5, 3), (60, 40), (360, 240), (900, 700)] {
            let mut canvas = Canvas::new(width, height);
            opened.render_input(&mut canvas, f64::INFINITY, &[down(0.75, 0.92)]);
        }
    }

    #[test]
    fn input_voice_measures_the_same_detour_and_meets_at_optimum() {
        let room = RouteLab::new();
        assert!(room.parameter_sound(0.0, &[]).is_none());
        let detour = room.parameter_sound(0.0, &[down(0.01, 0.4)]).unwrap();
        let optimal = room.parameter_sound(0.0, &[down(0.75, 0.92)]).unwrap();
        assert_eq!(detour.ratio(), 9.0 / 8.0);
        assert_eq!(optimal.ratio(), 1.0);
        assert_eq!(
            room.sound_input(0.0, &[down(0.75, 0.92)]),
            optimal.snapshot()
        );
        assert_eq!(room.sound_input(f64::NAN, &[]), room.sound(0.0));
        assert!(room.reveal().contains("only a local optimum"));
        assert!(room.deep_cuts()[2].contains("not the number"));
    }

    #[test]
    fn terminal_street_costs_fit_the_short_text_canvas() {
        let mut canvas = Canvas::new(80, 24);
        RouteLab::new().render(&mut canvas, 0.0);
        // Every documented street keeps its cost beside the road. Bitmap-font
        // offsets previously put the upper costs outside a short text canvas.
        for (x, y, text) in [
            (25, 5, ">AB 1"),
            (25, 9, "AC 2"),
            (30, 9, "BC 2"),
            (50, 5, "BD 3"),
            (50, 9, "CD 2"),
        ] {
            for (offset, character) in text.chars().enumerate() {
                assert_eq!(canvas.cell(x + offset, y), Some(character));
            }
        }
    }

    #[test]
    fn compaction_preserves_the_road_after_many_completed_order_clicks() {
        let room = RouteLab::new();
        let mut inputs = vec![down(1.0, 0.78)];
        for _ in 0..120 {
            for input in [
                down(0.25, 0.4),
                RoomInput::PointerUp {
                    x: 0.25,
                    y: 0.4,
                    t: 0.0,
                },
            ] {
                room.compact_inputs(&mut inputs);
                inputs.push(input);
                assert!(inputs.len() <= MAX_ROOM_INPUTS);
                let state = room.selection(&inputs);
                assert_eq!(state.bd, 9);
                assert_eq!(state.order, ORDERS[1]);
            }
        }
    }

    #[test]
    fn repeated_compaction_matches_complete_replay_for_mixed_input_and_seeds() {
        for seed in [0, 1, u64::MAX] {
            let room = RouteLab::new_with(seed);
            let mut retained = Vec::new();
            let mut complete = Vec::new();
            for index in 0..320 {
                let input = match index % 12 {
                    0 => down(((index % 9) as f64 + 0.5) / 9.0, 0.78),
                    1 => move_to(((index % 6) as f64 + 0.5) / 6.0, 0.4),
                    2 => RoomInput::PointerUp {
                        x: 0.75,
                        y: 0.9,
                        t: 0.0,
                    },
                    3 => RoomInput::Key { ch: 'l' },
                    4 => RoomInput::Key { ch: 'j' },
                    5 => RoomInput::Key { ch: 'I' },
                    6 => RoomInput::Key { ch: 'G' },
                    7 => RoomInput::Key { ch: '6' },
                    8 => RoomInput::PointerCancel,
                    9 => move_to(0.75, 0.9),
                    10 => RoomInput::Key { ch: 'L' },
                    _ => RoomInput::Key { ch: 'J' },
                };
                let before = retained.clone();
                room.compact_inputs(&mut retained);
                if before.len() >= MAX_ROOM_INPUTS - 2 {
                    assert_eq!(
                        &retained[retained.len() - MAX_ROOM_INPUTS / 2..],
                        &before[before.len() - MAX_ROOM_INPUTS / 2..]
                    );
                    assert!(retained.len() <= MAX_ROOM_INPUTS / 2 + 29);
                } else {
                    assert_eq!(retained, before);
                }
                retained.push(input);
                complete.push(input);
                assert!(retained.len() <= MAX_ROOM_INPUTS);
                let reference = room.selection_from(&complete);
                assert_eq!(
                    room.selection(&retained),
                    reference,
                    "seed={seed} event={index}"
                );
                let exact = Experiment::of(reference).unwrap();
                assert_eq!(
                    room.status_input(0.0, &retained).as_deref(),
                    Some(exact.status().as_str())
                );
                assert_eq!(
                    room.goal_met(0.0, &retained),
                    reference.touched && exact.current.cost == exact.optimum.cost
                );
                let sound = if reference.touched {
                    ParametricSound::new(
                        146.83,
                        exact.current.cost as f32 / exact.optimum.cost as f32,
                        0.025,
                    )
                    .unwrap()
                    .snapshot()
                } else {
                    room.sound(0.0)
                };
                assert_eq!(room.sound_input(0.0, &retained), sound);
            }
        }
    }

    #[test]
    fn checkpoints_preserve_untouched_state_and_reserve_two_input_slots() {
        let room = RouteLab::new_with(2);
        let mut untouched = vec![RoomInput::PointerCancel; MAX_ROOM_INPUTS - 2];
        room.compact_inputs(&mut untouched);
        assert_eq!(untouched.len(), MAX_ROOM_INPUTS / 2);
        assert_eq!(room.selection(&untouched), room.selection(&[]));
        for length in [MAX_ROOM_INPUTS - 2, MAX_ROOM_INPUTS - 1, MAX_ROOM_INPUTS] {
            let mut inputs = vec![RoomInput::Key { ch: 'L' }; length];
            let before = room.selection(&inputs);
            room.compact_inputs(&mut inputs);
            assert!(inputs.len() + 2 <= MAX_ROOM_INPUTS);
            assert_eq!(room.selection(&inputs), before);
            inputs.extend_from_slice(&[
                RoomInput::PointerMove {
                    x: 0.25,
                    y: 0.4,
                    t: 0.0,
                },
                RoomInput::PointerCancel,
            ]);
            assert!(inputs.len() <= MAX_ROOM_INPUTS);
            assert_eq!(room.selection(&inputs).bd, 9);
        }
    }

    #[test]
    fn ignored_and_invalid_events_never_invent_a_touched_checkpoint() {
        let ignored = [
            RoomInput::Key { ch: '?' },
            down(f64::NAN, 0.4),
            RoomInput::PointerMove {
                x: 0.25,
                y: 0.4,
                t: f64::NAN,
            },
            RoomInput::PointerUp {
                x: 0.25,
                y: 0.4,
                t: 0.0,
            },
            RoomInput::Wheel { delta: f64::NAN },
            RoomInput::PointerCancel,
        ];
        for seed in [0, 1, u64::MAX] {
            let room = RouteLab::new_with(seed);
            let mut retained = Vec::new();
            let mut complete = Vec::new();
            for index in 0..300 {
                room.compact_inputs(&mut retained);
                retained.push(ignored[index % ignored.len()]);
                complete.push(ignored[index % ignored.len()]);
                assert!(retained.len() <= MAX_ROOM_INPUTS);
                assert_eq!(room.selection(&retained), room.selection_from(&complete));
                assert!(!room.selection(&retained).touched);
                assert_eq!(room.status_input(0.0, &retained), room.status(0.0));
                assert_eq!(room.sound_input(0.0, &retained), room.sound(0.0));
                assert!(!room.goal_met(0.0, &retained));
            }
        }
    }
}
