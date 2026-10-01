//! Native route authoring. Core owns validation, undo, search and comparison.

use numinous_core::route::{Road, RouteEvent};
use numinous_core::route_workbench::{
    EditableRoad, RouteEdit, RouteWorkbench, RouteWorkbenchSnapshot,
};
use numinous_core::{ProjectDraft, ProjectNext, Raster, RouteCreation, Surface};

fn wrapped(text: &str, columns: usize) -> Vec<String> {
    numinous_core::wrap_text(text, columns)
        .into_iter()
        .flat_map(|line| {
            let chars = line.chars().collect::<Vec<_>>();
            chars
                .chunks(columns.max(1))
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    View,
    Roads,
    Stops,
    Order,
    Search,
    Keep,
}
const PAGES: [Page; 6] = [
    Page::View,
    Page::Roads,
    Page::Stops,
    Page::Order,
    Page::Search,
    Page::Keep,
];
const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789?,.-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Page(Page),
    Close,
    Previous,
    Next,
    Junctions(i32),
    NewRoad,
    From(i32),
    To(i32),
    Cost(i32),
    CommitRoad,
    ToggleRoad,
    DeleteRoad,
    Delivery,
    Depot,
    Earlier,
    Later,
    Nearest,
    Shorter,
    Undo,
    Search,
    Back,
    Step,
    Reset,
    Remix,
    Character(i32),
    AddCharacter,
    Space,
    Backspace,
    Keep,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
    Close,
    Keep,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub bounds: (f64, f64, f64, f64),
    pub label: String,
    pub action: Action,
}

/// Process-local editing state. Portable creations deliberately omit undo/search.
#[derive(Debug)]
pub struct Panel {
    workbench: RouteWorkbench,
    baseline: RouteWorkbenchSnapshot,
    source: Option<RouteCreation>,
    remix: bool,
    pub page: Page,
    pub question: String,
    pub message: String,
    pub paused: bool,
    road: usize,
    junction: usize,
    order: usize,
    draft: Option<Road>,
    character: usize,
    focus: usize,
    pointer: bool,
}

impl Panel {
    pub fn new(workbench: RouteWorkbench) -> Self {
        let baseline = workbench.snapshot();
        Self {
            workbench,
            baseline,
            source: None,
            remix: false,
            page: Page::View,
            question: String::new(),
            message: String::new(),
            paused: false,
            road: 0,
            junction: 0,
            order: 1,
            draft: None,
            character: 0,
            focus: 0,
            pointer: false,
        }
    }

    pub fn opened(creation: RouteCreation) -> Self {
        let mut panel = Self::new(creation.open());
        panel.source = Some(creation);
        panel.paused = true;
        panel
    }

    pub fn workbench(&self) -> &RouteWorkbench {
        &self.workbench
    }

    pub fn push_text(&mut self, text: &str) {
        if self.page == Page::Keep && !self.paused {
            for ch in text.chars().filter(|ch| (' '..='~').contains(ch)) {
                if self.question.len() < numinous_core::MAX_WORKSPACE_TEXT_CHARS {
                    self.question.push(ch);
                }
            }
        }
    }

    pub fn project_draft(&self, recorded_at_utc: u64) -> Result<ProjectDraft, String> {
        let creation = if self.remix {
            self.source
                .as_ref()
                .ok_or("Open a kept route before remixing")?
                .with_network(self.workbench.town().clone())
        } else if let Some(source) = &self.source {
            source.with_network(self.workbench.town().clone())
        } else {
            RouteCreation::new(self.workbench.town().clone())
        }
        .map_err(|error| error.to_string())?;
        let capsule = creation.to_capsule();
        Ok(ProjectDraft {
            recorded_at_utc,
            question: self.question.clone(),
            next: ProjectNext::OpenRoute {
                capsule: capsule.clone(),
            },
            rooms: vec!["route-lab".into()],
            evidence: vec![],
            creation: Some(capsule),
        })
    }

    fn selected_road(&self) -> Option<EditableRoad> {
        self.workbench.town().roads.get(self.road).copied()
    }

    pub fn buttons(&self) -> Vec<Button> {
        let mut entries: Vec<(String, Action)> = if self.paused {
            vec![
                ("OPEN ROUTE".into(), Action::Confirm),
                ("LEAVE".into(), Action::Close),
            ]
        } else {
            let mut entries = PAGES
                .iter()
                .map(|page| (format!("{page:?}").to_uppercase(), Action::Page(*page)))
                .collect::<Vec<_>>();
            entries.push(("LEAVE".into(), Action::Close));
            entries
        };
        let tab_count = entries.len();
        let body: Vec<(&str, Action)> = if self.paused {
            vec![]
        } else {
            match self.page {
                Page::View => vec![
                    ("UNDO", Action::Undo),
                    ("BASELINE", Action::Reset),
                    ("NEAREST", Action::Nearest),
                    ("USE SHORTER", Action::Shorter),
                ],
                Page::Roads => vec![
                    ("ROAD <", Action::Previous),
                    ("ROAD >", Action::Next),
                    ("JUNCTION -", Action::Junctions(-1)),
                    ("JUNCTION +", Action::Junctions(1)),
                    ("NEW ROAD", Action::NewRoad),
                    ("FROM -", Action::From(-1)),
                    ("FROM +", Action::From(1)),
                    ("TO -", Action::To(-1)),
                    ("TO +", Action::To(1)),
                    ("COST -10", Action::Cost(-10)),
                    ("COST -1", Action::Cost(-1)),
                    ("COST +1", Action::Cost(1)),
                    ("COST +10", Action::Cost(10)),
                    ("ADD ROAD", Action::CommitRoad),
                    ("OPEN/CLOSE", Action::ToggleRoad),
                    ("DELETE ROAD", Action::DeleteRoad),
                    ("UNDO", Action::Undo),
                ],
                Page::Stops => vec![
                    ("JUNCTION <", Action::Previous),
                    ("JUNCTION >", Action::Next),
                    ("DELIVERY +/-", Action::Delivery),
                    ("SET DEPOT", Action::Depot),
                    ("UNDO", Action::Undo),
                ],
                Page::Order => vec![
                    ("STOP <", Action::Previous),
                    ("STOP >", Action::Next),
                    ("EARLIER", Action::Earlier),
                    ("LATER", Action::Later),
                    ("NEAREST", Action::Nearest),
                    ("USE SHORTER", Action::Shorter),
                    ("UNDO", Action::Undo),
                ],
                Page::Search => vec![
                    ("TARGET <", Action::Previous),
                    ("TARGET >", Action::Next),
                    ("START SEARCH", Action::Search),
                    ("BACK", Action::Back),
                    ("STEP", Action::Step),
                ],
                Page::Keep => vec![
                    ("CHAR <", Action::Character(-1)),
                    ("CHAR >", Action::Character(1)),
                    ("ADD CHAR", Action::AddCharacter),
                    ("SPACE", Action::Space),
                    ("BACKSPACE", Action::Backspace),
                    ("KEEP QUESTION", Action::Keep),
                    ("REMIX SOURCE", Action::Remix),
                    ("BASELINE", Action::Reset),
                ],
            }
        };
        entries.extend(
            body.into_iter()
                .map(|(label, action)| (label.to_string(), action)),
        );
        entries
            .into_iter()
            .enumerate()
            .map(|(index, (label, action))| {
                let bounds = if self.paused {
                    (0.02 + index as f64 * 0.49, 0.82, 0.47, 0.09)
                } else if index < tab_count {
                    let columns = 4;
                    let column = index % columns;
                    let row = index / columns;
                    (
                        0.02 + column as f64 * 0.245,
                        0.14 + row as f64 * 0.075,
                        0.235,
                        0.065,
                    )
                } else {
                    let index = index - tab_count;
                    let column = index % 3;
                    let row = index / 3;
                    (
                        0.02 + column as f64 * 0.325,
                        (if self.page == Page::Keep { 0.60 } else { 0.40 }) + row as f64 * 0.075,
                        0.315,
                        0.065,
                    )
                };
                Button {
                    bounds,
                    label: if action == Action::NewRoad && self.draft.is_some() {
                        "CANCEL NEW".into()
                    } else {
                        label
                    },
                    action,
                }
            })
            .collect()
    }

    pub fn pointer_at(&mut self, point: (f64, f64), activate: bool) -> Effect {
        self.pointer = true;
        let buttons = self.buttons();
        if let Some((index, button)) = buttons.iter().enumerate().find(|(_, button)| {
            let (x, y, w, h) = button.bounds;
            point.0 >= x && point.0 < x + w && point.1 >= y && point.1 < y + h
        }) {
            self.focus = index;
            if activate {
                return self.act(button.action);
            }
        }
        Effect::None
    }

    pub fn navigate(&mut self, delta: i32) {
        self.pointer = false;
        self.focus =
            (self.focus as i64 + delta as i64).rem_euclid(self.buttons().len() as i64) as usize;
    }

    pub fn navigate_direction(&mut self, dx: i32, dy: i32) {
        self.pointer = false;
        let buttons = self.buttons();
        let (x, y, w, h) = buttons[self.focus.min(buttons.len() - 1)].bounds;
        let center = (x + w / 2.0, y + h / 2.0);
        let candidate = buttons
            .iter()
            .enumerate()
            .filter_map(|(index, button)| {
                let (x, y, w, h) = button.bounds;
                let diff = (x + w / 2.0 - center.0, y + h / 2.0 - center.1);
                let (primary, secondary) = if dx != 0 {
                    (diff.0 * dx.signum() as f64, diff.1)
                } else {
                    (diff.1 * dy.signum() as f64, diff.0)
                };
                (primary > 0.001)
                    .then_some((index, primary * primary + secondary * secondary * 4.0))
            })
            .min_by(|first, second| first.1.total_cmp(&second.1).then(first.0.cmp(&second.0)));
        if let Some((index, _)) = candidate {
            self.focus = index;
        }
    }

    pub fn activate(&mut self) -> Effect {
        self.act(self.buttons()[self.focus.min(self.buttons().len() - 1)].action)
    }

    pub fn controller_activate(&mut self, point: Option<(f64, f64)>) -> Effect {
        if self.pointer
            && let Some(point) = point
        {
            self.pointer_at(point, true)
        } else {
            self.activate()
        }
    }

    pub fn act(&mut self, action: Action) -> Effect {
        if action == Action::Close {
            return Effect::Close;
        }
        if self.paused {
            if action == Action::Confirm {
                self.paused = false;
            }
            return Effect::None;
        }
        self.message.clear();
        let town = self.workbench.town().clone();
        if self.draft.is_some() && matches!(action, Action::ToggleRoad | Action::DeleteRoad) {
            self.message = "Cancel NEW ROAD or use ROAD < / > to edit an existing road.".into();
            return Effect::None;
        }
        let mut edit = None;
        match action {
            Action::Page(page) => {
                self.page = page;
                self.focus = 0;
            }
            Action::Previous | Action::Next => {
                let delta = if action == Action::Next { 1 } else { -1 };
                match self.page {
                    Page::Roads => {
                        self.draft = None;
                        if !town.roads.is_empty() {
                            self.road = (self.road as i32 + delta)
                                .rem_euclid(town.roads.len() as i32)
                                as usize;
                        }
                    }
                    Page::Order => {
                        if town.order.len() > 1 {
                            self.order = 1
                                + (self.order as i32 - 1 + delta)
                                    .rem_euclid(town.order.len() as i32 - 1)
                                    as usize;
                        }
                    }
                    _ => {
                        self.junction = (self.junction as i64 + delta as i64)
                            .rem_euclid(town.junctions as i64)
                            as usize
                    }
                }
            }
            Action::Junctions(delta) => {
                let mut desired = town.clone();
                desired.junctions = (town.junctions as i64 + delta as i64).max(0) as usize;
                edit = Some(RouteEdit::Network(desired));
            }
            Action::NewRoad if self.draft.is_some() => {
                self.draft = None;
            }
            Action::NewRoad => {
                self.draft = Some(Road {
                    from: 0,
                    to: 1,
                    cost: 1,
                })
            }
            Action::From(delta) | Action::To(delta) => {
                if let Some(draft) = self.draft.as_mut() {
                    let field = if matches!(action, Action::From(_)) {
                        &mut draft.from
                    } else {
                        &mut draft.to
                    };
                    *field =
                        (*field as i64 + delta as i64).rem_euclid(town.junctions as i64) as usize;
                } else {
                    self.message = "Choose NEW ROAD to set endpoints".into();
                }
            }
            Action::Cost(delta) => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.cost = (draft.cost as i64 + delta as i64).clamp(1, 999) as u32;
                } else if let Some(selected) = self.selected_road() {
                    edit = Some(RouteEdit::RoadCost {
                        from: selected.road.from,
                        to: selected.road.to,
                        cost: (selected.road.cost as i64 + delta as i64).clamp(1, 999) as u32,
                    });
                }
            }
            Action::CommitRoad => {
                if let Some(road) = self.draft {
                    let mut desired = town.clone();
                    desired.roads.push(EditableRoad { road, open: true });
                    edit = Some(RouteEdit::Network(desired));
                } else {
                    self.message = "Choose NEW ROAD first".into();
                }
            }
            Action::ToggleRoad => {
                if let Some(selected) = self.selected_road() {
                    edit = Some(RouteEdit::RoadOpen {
                        from: selected.road.from,
                        to: selected.road.to,
                        open: !selected.open,
                    });
                }
            }
            Action::DeleteRoad => {
                if self.selected_road().is_some() {
                    let mut desired = town.clone();
                    desired.roads.remove(self.road);
                    edit = Some(RouteEdit::Network(desired));
                }
            }
            Action::Delivery => {
                if self.junction == town.stops[0] {
                    self.message = "Choose another depot before removing this stop".into();
                } else {
                    let mut stops = town.stops.clone();
                    if stops.contains(&self.junction) {
                        stops.retain(|stop| *stop != self.junction);
                    } else {
                        stops.push(self.junction);
                    }
                    edit = Some(RouteEdit::RequiredStops(stops));
                }
            }
            Action::Depot => edit = Some(RouteEdit::Depot(self.junction)),
            Action::Earlier | Action::Later => {
                if town.order.len() > 1 {
                    let destination = if action == Action::Earlier {
                        self.order.saturating_sub(1).max(1)
                    } else {
                        (self.order + 1).min(town.order.len() - 1)
                    };
                    let mut order = town.order.clone();
                    let selected = self.order.min(order.len() - 1);
                    order.swap(selected, destination);
                    edit = Some(RouteEdit::Order(order));
                    self.order = destination;
                }
            }
            Action::Nearest => edit = Some(RouteEdit::Greedy),
            Action::Shorter => edit = Some(RouteEdit::Improve),
            Action::Undo => {
                self.message = match self.workbench.undo() {
                    Ok(true) => "Previous network restored.".into(),
                    Ok(false) => "Nothing to undo.".into(),
                    Err(error) => error.to_string(),
                };
            }
            Action::Search => {
                if let Err(error) = self.workbench.start_trace(town.stops[0], self.junction) {
                    self.message = error.to_string();
                }
            }
            Action::Back | Action::Step => {
                if let Some(trace) = self.workbench.trace() {
                    let cursor = if action == Action::Back {
                        trace.cursor().saturating_sub(1)
                    } else {
                        (trace.cursor() + 1).min(trace.events().len())
                    };
                    if let Err(error) = self.workbench.seek_trace(cursor) {
                        self.message = error.to_string();
                    }
                } else {
                    self.message = "START SEARCH before stepping".into();
                }
            }
            Action::Reset => {
                self.workbench = RouteWorkbench::from_snapshot(self.baseline.clone())
                    .expect("a locally retained validated baseline");
                self.draft = None;
            }
            Action::Remix => {
                if let Some(source) = &self.source {
                    match source.remix(town.clone()) {
                        Ok(child) => {
                            self.source = Some(child);
                            self.remix = true;
                            self.message = "Remix selected. KEEP records its parent.".into();
                        }
                        Err(error) => self.message = error.to_string(),
                    }
                } else {
                    self.message = "Open a kept route before choosing REMIX".into();
                }
            }
            Action::Character(delta) => {
                self.character =
                    (self.character as i64 + delta as i64).rem_euclid(LETTERS.len() as i64) as usize
            }
            Action::AddCharacter => self.push_text(&(LETTERS[self.character] as char).to_string()),
            Action::Space => self.push_text(" "),
            Action::Backspace => {
                self.question.pop();
            }
            Action::Keep => return Effect::Keep,
            Action::Close | Action::Confirm => {}
        }
        if let Some(edit) = edit {
            match self.workbench.apply(edit) {
                Ok(changed) => {
                    if changed {
                        self.message = "Changed. UNDO restores the previous network.".into();
                        if action == Action::CommitRoad {
                            if let Some(added) = self.draft {
                                self.road = self
                                    .workbench
                                    .town()
                                    .roads
                                    .iter()
                                    .position(|road| {
                                        (road.road.from == added.from && road.road.to == added.to)
                                            || (road.road.from == added.to
                                                && road.road.to == added.from)
                                    })
                                    .expect("canonical network retains the newly admitted road");
                            }
                            self.draft = None;
                        }
                    } else {
                        self.message = if action == Action::Shorter {
                            "No shorter offer. Exact best remains the comparison.".into()
                        } else {
                            "Already at this setting.".into()
                        };
                    }
                }
                Err(error) => self.message = error.to_string(),
            }
        }
        self.road = self
            .road
            .min(self.workbench.town().roads.len().saturating_sub(1));
        self.junction = self.junction.min(self.workbench.town().junctions - 1);
        self.order = self
            .order
            .min(self.workbench.town().order.len().saturating_sub(1))
            .max(1);
        self.focus = self.focus.min(self.buttons().len() - 1);
        Effect::None
    }

    pub fn lines(&self) -> Vec<String> {
        let town = self.workbench.town();
        let comparison = match self.workbench.compare() {
            Ok(comparison) => format!(
                "ROUND TRIP {}  EXACT BEST {}",
                comparison.current.cost, comparison.exact.tour.cost
            ),
            Err(error) => format!("NO COMPARISON: {error}"),
        };
        let order = town
            .order
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(">");
        let detail = match self.page {
            Page::View => format!(
                "{order}>{}  {} / {}",
                town.stops[0],
                numinous_core::counted(town.junctions, "junction"),
                numinous_core::counted(town.roads.len(), "road")
            ),
            Page::Roads => {
                if let Some(road) = self.draft {
                    format!(
                        "NEW {}-{}  COST {}. ADD VALIDATES.",
                        road.from, road.to, road.cost
                    )
                } else if let Some(selected) = self.selected_road() {
                    format!(
                        "ROAD {}/{}: {}-{} COST {} {}",
                        self.road + 1,
                        town.roads.len(),
                        selected.road.from,
                        selected.road.to,
                        selected.road.cost,
                        if selected.open { "OPEN" } else { "CLOSED" }
                    )
                } else {
                    "NO ROADS. NEW ROAD TO CONNECT STOPS.".into()
                }
            }
            Page::Stops => format!(
                "JUNCTION {}: {}  DEPOT {}",
                self.junction,
                if self.junction == town.stops[0] {
                    "DEPOT"
                } else if town.stops.contains(&self.junction) {
                    "DELIVERY"
                } else {
                    "PASS THROUGH"
                },
                town.stops[0]
            ),
            Page::Order => format!(
                "{order}>{}  SELECTED DELIVERY {}",
                town.stops[0],
                town.order
                    .get(self.order)
                    .map_or_else(|| "NONE".into(), |id| id.to_string())
            ),
            Page::Search => format!("TARGET {}. {}", self.junction, self.search_line()),
            Page::Keep => format!(
                "QUESTION: {}",
                if self.question.is_empty() {
                    "TYPE OR ADD CHARACTERS"
                } else {
                    &self.question
                }
            ),
        };
        vec![
            comparison,
            detail,
            match self.page {
                Page::Keep => format!(
                    "CHAR {}. KEEP SAVES NETWORK / ORDER, NOT UNDO / SEARCH.",
                    LETTERS[self.character] as char
                ),
                Page::View => "THICK: STREET WALK. X: CLOSED. D: DEPOT. *: DELIVERY.".into(),
                _ => self.message.clone(),
            },
        ]
    }

    fn search_line(&self) -> String {
        let Some(trace) = self.workbench.trace() else {
            return "START, THEN STEP RECORDED EVENTS".into();
        };
        if let Some(result) = trace.result() {
            let snapshot = trace.snapshot();
            return match result {
                Ok(path) => format!(
                    "SEARCH {}>{}: PATH {:?} COST {}",
                    snapshot.from, snapshot.to, path.junctions, path.cost
                ),
                Err(error) => format!("SEARCH {}>{}: NO PATH: {error}", snapshot.from, snapshot.to),
            };
        }
        let snapshot = trace.snapshot();
        let prefix = format!(
            "SEARCH {}>{} {}/{}",
            snapshot.from,
            snapshot.to,
            trace.cursor(),
            trace.events().len()
        );
        match trace.visible_events().last() {
            Some(RouteEvent::Settled { junction, cost }) => {
                format!("{prefix} {junction} SETTLED FROM SOURCE {cost}")
            }
            Some(RouteEvent::Relaxed { from, to, cost }) => {
                format!("{prefix} {to} TENTATIVE {cost} VIA {from}")
            }
            None => prefix,
        }
    }

    pub fn draw(&self, width: usize, height: usize, controller_hint: Option<&str>) -> Raster {
        let mut raster = Raster::with_accent(width, height, [240, 180, 110]);
        let scale = ((width / 400).min(height / 240)).clamp(1, 3) as i32;
        let columns = width.saturating_sub(12) / (6 * scale as usize);
        let text = |raster: &mut Raster, line: &str, y: f64| {
            let fitted = numinous_core::display_safe(line)
                .chars()
                .take(columns)
                .collect::<String>();
            numinous_core::draw_text(raster, &fitted, 6, (height as f64 * y) as i32, scale, '#');
        };
        text(
            &mut raster,
            if self.paused {
                "ROUTE LAB: PREVIEW. OPEN TO EDIT."
            } else {
                "EDIT ROUTE"
            },
            0.025,
        );
        let lines = self.lines();
        for (index, line) in wrapped(&lines[0], columns).iter().take(2).enumerate() {
            text(
                &mut raster,
                line,
                0.075 + index as f64 * 8.0 * scale as f64 / height as f64,
            );
        }
        if self.page == Page::Keep {
            for (index, line) in lines[1]
                .chars()
                .collect::<Vec<_>>()
                .chunks(columns.max(1))
                .map(|chunk| chunk.iter().collect::<String>())
                .enumerate()
            {
                text(
                    &mut raster,
                    &line,
                    0.29 + index as f64 * 8.0 * scale as f64 / height as f64,
                );
            }
            text(
                &mut raster,
                &format!(
                    "CHAR {}. KEEP STORES NETWORK / ORDER.",
                    LETTERS[self.character] as char
                ),
                0.55,
            );
        } else {
            for (index, line) in wrapped(&lines[1], columns).iter().take(3).enumerate() {
                text(
                    &mut raster,
                    line,
                    0.29 + index as f64 * 8.0 * scale as f64 / height as f64,
                );
            }
            if self.page != Page::Search {
                text(&mut raster, &lines[2], 0.37);
            }
        }
        if self.page == Page::View {
            if let Some(selected) = self.selected_road() {
                text(
                    &mut raster,
                    &format!(
                        "SELECTED {}-{}: COST {} {}",
                        selected.road.from,
                        selected.road.to,
                        selected.road.cost,
                        if selected.open { "OPEN" } else { "CLOSED" }
                    ),
                    0.56,
                );
            }
            self.draw_network(&mut raster, width, height, scale);
        }
        for (index, button) in self.buttons().iter().enumerate() {
            let (x, y, w, h) = button.bounds;
            let (left, top, right, bottom) = (
                (x * width as f64) as i32,
                (y * height as f64) as i32,
                ((x + w) * width as f64) as i32 - 1,
                ((y + h) * height as f64) as i32 - 1,
            );
            raster.line(left, top, right, top, '#');
            raster.line(left, bottom, right, bottom, '#');
            raster.line(left, top, left, bottom, '#');
            raster.line(right, top, right, bottom, '#');
            let available = (right - left - 6).max(0) / (6 * scale);
            let label = format!(
                "{}{}",
                if index == self.focus { ">" } else { "" },
                button.label
            )
            .chars()
            .take(available as usize)
            .collect::<String>();
            numinous_core::draw_text(
                &mut raster,
                &label,
                left + 3,
                top + ((bottom - top - 7 * scale) / 2).max(1),
                scale,
                '#',
            );
        }
        if !self.message.is_empty() {
            for (index, line) in wrapped(&self.message, columns).iter().take(2).enumerate() {
                text(
                    &mut raster,
                    line,
                    0.87 + index as f64 * 8.0 * scale as f64 / height as f64,
                );
            }
        }
        text(
            &mut raster,
            controller_hint.unwrap_or("TAB / ARROWS SELECT. ENTER ACTS. ESC LEAVES."),
            0.95,
        );
        raster
    }

    fn draw_network(&self, raster: &mut Raster, width: usize, height: usize, scale: i32) {
        let town = self.workbench.town();
        let points = (0..town.junctions)
            .map(|id| {
                let angle = std::f64::consts::TAU * id as f64 / town.junctions as f64
                    - std::f64::consts::FRAC_PI_2;
                (
                    (width as f64 * (0.5 + 0.35 * angle.cos())) as i32,
                    (height as f64 * (0.73 + 0.11 * angle.sin())) as i32,
                )
            })
            .collect::<Vec<_>>();
        for road in &town.roads {
            let (a, b) = (points[road.road.from], points[road.road.to]);
            if road.open {
                raster.line(a.0, a.1, b.0, b.1, '.');
            } else {
                let mid = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
                raster.line(a.0, a.1, (a.0 + mid.0) / 2, (a.1 + mid.1) / 2, '.');
                raster.line(b.0, b.1, (b.0 + mid.0) / 2, (b.1 + mid.1) / 2, '.');
                numinous_core::draw_text(raster, "X", mid.0, mid.1, scale, '#');
            }
        }
        if let Ok(comparison) = self.workbench.compare() {
            for pair in comparison.current.walk.windows(2) {
                let (a, b) = (points[pair[0]], points[pair[1]]);
                raster.line(a.0, a.1, b.0, b.1, '#');
                raster.line(a.0 + 1, a.1, b.0 + 1, b.1, '#');
            }
        }
        for (id, point) in points.iter().enumerate() {
            numinous_core::draw_text(
                raster,
                &format!(
                    "{id}{}",
                    if id == town.stops[0] {
                        "D"
                    } else if town.stops.contains(&id) {
                        "*"
                    } else {
                        ""
                    }
                ),
                point.0,
                point.1,
                scale,
                '#',
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn route_authoring_edits_are_transactional_and_undoable() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        let before = panel.workbench.snapshot();
        panel.act(Action::Junctions(-1));
        assert_eq!(panel.workbench.snapshot(), before);
        assert!(!panel.message.is_empty());
        panel.act(Action::Junctions(1));
        assert_eq!(panel.workbench.town().junctions, 5);
        panel.act(Action::Page(Page::Stops));
        for _ in 0..4 {
            panel.act(Action::Next);
        }
        panel.act(Action::Delivery);
        assert!(panel.workbench.town().stops.contains(&4));
        assert!(panel.workbench.compare().is_err());
        panel.act(Action::Undo);
        panel.act(Action::Undo);
        assert_eq!(panel.workbench.town(), &before.current);
    }
    #[test]
    fn route_authoring_pointer_and_focus_use_identical_actions() {
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let mut pointer = Panel::new(RouteWorkbench::first_town());
            let mut keyboard = Panel::new(RouteWorkbench::first_town());
            pointer.act(Action::Page(Page::Roads));
            keyboard.act(Action::Page(Page::Roads));
            let buttons = pointer.buttons();
            let index = buttons
                .iter()
                .position(|button| button.action == Action::ToggleRoad)
                .unwrap();
            let (x, y, w, h) = buttons[index].bounds;
            pointer.pointer_at((x + w / 2.0, y + h / 2.0), true);
            keyboard.navigate(index as i32);
            keyboard.activate();
            assert_eq!(pointer.workbench.snapshot(), keyboard.workbench.snapshot());
            let raster = pointer.draw(width, height, None);
            assert_eq!(raster.width(), width);
            assert_eq!(raster.height(), height);
        }
    }
    #[test]
    fn route_authoring_preview_is_read_only_and_baseline_is_opened_network() {
        let mut workbench = RouteWorkbench::first_town();
        let mut town = workbench.town().clone();
        town.junctions = 5;
        workbench.apply(RouteEdit::Network(town)).unwrap();
        let creation = RouteCreation::new(workbench.town().clone()).unwrap();
        let mut panel = Panel::opened(creation);
        let before = panel.workbench.snapshot();
        panel.act(Action::Cost(1));
        assert_eq!(panel.workbench.snapshot(), before);
        panel.act(Action::Confirm);
        panel.act(Action::Cost(1));
        panel.act(Action::Reset);
        assert_eq!(panel.workbench.snapshot(), before);
        panel.act(Action::Page(Page::Keep));
        panel.push_text("What changes with a new delivery?");
        let draft = panel.project_draft(1).unwrap();
        let restored = RouteCreation::from_capsule(draft.creation.as_deref().unwrap()).unwrap();
        assert_eq!(restored.town(), panel.workbench.town());
    }
    #[test]
    fn route_authoring_new_road_inspector_tracks_canonical_sorting_and_refusal() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.act(Action::Page(Page::Roads));
        panel.act(Action::NewRoad);
        let before = panel.workbench.snapshot();
        panel.act(Action::CommitRoad);
        assert_eq!(panel.workbench.snapshot(), before);
        assert!(panel.draft.is_some());
        panel.act(Action::ToggleRoad);
        panel.act(Action::DeleteRoad);
        assert_eq!(panel.workbench.snapshot(), before);
        panel.act(Action::To(2));
        panel.act(Action::CommitRoad);
        assert!(panel.draft.is_none());
        let selected = panel.selected_road().unwrap();
        assert_eq!((selected.road.from, selected.road.to), (0, 3));
        panel.act(Action::Cost(10));
        assert_eq!(panel.selected_road().unwrap().road.cost, 11);
        panel.act(Action::DeleteRoad);
        assert!(
            !panel
                .workbench
                .town()
                .roads
                .iter()
                .any(|road| road.road.from == 0 && road.road.to == 3)
        );
        panel.act(Action::Undo);
        assert!(
            panel
                .workbench
                .town()
                .roads
                .iter()
                .any(|road| road.road.from == 0 && road.road.to == 3 && road.road.cost == 11)
        );
    }
    #[test]
    fn route_authoring_delivery_depot_and_order_delegate_to_core() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.act(Action::Page(Page::Stops));
        panel.act(Action::Delivery);
        assert_eq!(panel.workbench.town().stops, [0, 1, 2, 3]);
        panel.act(Action::Next);
        panel.act(Action::Depot);
        assert_eq!(panel.workbench.town().order, [1, 2, 3, 0]);
        panel.act(Action::Page(Page::Order));
        panel.act(Action::Later);
        assert_eq!(panel.workbench.town().order, [1, 3, 2, 0]);
        panel.act(Action::Earlier);
        assert_eq!(panel.workbench.town().order, [1, 2, 3, 0]);
        panel.act(Action::Page(Page::Stops));
        panel.act(Action::Next);
        panel.act(Action::Delivery);
        assert_eq!(panel.workbench.town().order, [1, 3, 0]);
        panel.act(Action::Delivery);
        assert_eq!(panel.workbench.town().order, [1, 3, 0, 2]);
    }
    #[test]
    fn route_authoring_search_names_recorded_endpoints_and_only_complete_paths() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.act(Action::Page(Page::Search));
        panel.act(Action::Next);
        panel.act(Action::Next);
        panel.act(Action::Next);
        panel.act(Action::Search);
        assert!(panel.lines()[1].contains("SEARCH 0>3"));
        assert!(!panel.lines()[1].contains("PATH"));
        panel.act(Action::Previous);
        assert!(panel.lines()[1].contains("TARGET 2. SEARCH 0>3"));
        panel.act(Action::Step);
        let partial = panel.workbench.snapshot();
        panel.draw(360, 240, None);
        assert_eq!(panel.workbench.snapshot(), partial);
        for _ in 0..100 {
            panel.act(Action::Step);
        }
        assert!(panel.lines()[1].contains("PATH"));
        panel.act(Action::Back);
        assert!(!panel.lines()[1].contains("PATH"));
        panel.act(Action::Page(Page::Roads));
        panel.act(Action::ToggleRoad);
        assert!(panel.workbench.trace().is_none());
    }
    #[test]
    fn route_authoring_save_preserves_lineage_and_explicit_remix_binds_source_once() {
        let source = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let original = source.to_capsule();
        let mut panel = Panel::opened(source.clone());
        panel.act(Action::Confirm);
        panel.act(Action::Cost(1));
        panel.act(Action::Remix);
        panel.act(Action::Cost(1));
        panel.act(Action::Page(Page::Keep));
        panel.push_text("Which cost matters?");
        let draft = panel.project_draft(1).unwrap();
        let child = RouteCreation::from_capsule(draft.creation.as_deref().unwrap()).unwrap();
        assert_eq!(child.parent_identity(), Some(source.identity()));
        assert_eq!(source.to_capsule(), original);
        let mut reopened = Panel::opened(child.clone());
        reopened.act(Action::Confirm);
        reopened.act(Action::Cost(1));
        reopened.act(Action::Page(Page::Keep));
        reopened.push_text("Continue");
        let next = reopened.project_draft(2).unwrap();
        let next = RouteCreation::from_capsule(next.creation.as_deref().unwrap()).unwrap();
        assert_eq!(next.parent_identity(), child.parent_identity());
    }
    #[test]
    fn route_authoring_every_visible_control_fits_and_hits_at_supported_viewports() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        for page in PAGES {
            panel.act(Action::Page(page));
            for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
                let scale = ((width / 400).min(height / 240)).clamp(1, 3);
                let buttons = panel.buttons();
                for button in &buttons {
                    let (x, y, w, h) = button.bounds;
                    assert!(x >= 0.0 && y >= 0.0 && x + w <= 1.0 && y + h < 0.87);
                    assert!(
                        button.label.len() < ((w * width as f64) as usize - 8) / (6 * scale),
                        "{} at {}x{}",
                        button.label,
                        width,
                        height
                    );
                }
                for (index, button) in buttons.iter().enumerate() {
                    let (x, y, w, h) = button.bounds;
                    panel.pointer_at((x + w / 2.0, y + h / 2.0), false);
                    assert_eq!(panel.focus, index);
                }
                panel.draw(width, height, None);
            }
        }
        panel.act(Action::Page(Page::Keep));
        panel.push_text(&"a".repeat(numinous_core::MAX_WORKSPACE_TEXT_CHARS + 1));
        assert_eq!(
            panel.question.len(),
            numinous_core::MAX_WORKSPACE_TEXT_CHARS
        );
        panel.act(Action::Character(-1));
        panel.act(Action::Backspace);
        panel.act(Action::AddCharacter);
        panel.act(Action::Space);
        assert_eq!(
            panel.question.len(),
            numinous_core::MAX_WORKSPACE_TEXT_CHARS
        );
    }
    #[test]
    fn route_authoring_directional_focus_follows_geometry_and_extreme_deltas_are_bounded() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.focus = 4;
        panel.navigate_direction(0, -1);
        assert_eq!(
            panel.buttons()[panel.focus].action,
            Action::Page(Page::View)
        );
        panel.navigate(i32::MAX);
        panel.navigate(i32::MIN);
        assert!(panel.focus < panel.buttons().len());
        panel.act(Action::NewRoad);
        panel.act(Action::From(i32::MAX));
        panel.act(Action::To(i32::MIN));
        assert!(panel.draft.unwrap().from < 4 && panel.draft.unwrap().to < 4);
        let before = panel.workbench.snapshot();
        panel.act(Action::Junctions(i32::MAX));
        assert_eq!(panel.workbench.snapshot(), before);
        panel.act(Action::Junctions(i32::MIN));
        assert_eq!(panel.workbench.snapshot(), before);
        panel.act(Action::Character(i32::MAX));
        panel.act(Action::Character(i32::MIN));
        assert!(panel.character < LETTERS.len());
    }
    #[test]
    fn route_authoring_maximum_question_tail_is_rendered_even_without_word_breaks() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.act(Action::Page(Page::Keep));
        panel.question = format!("{}QTAIL", "a".repeat(275));
        let actual = panel.draw(360, 240, None).to_rgba();
        let mut expected = Raster::with_accent(360, 240, [240, 180, 110]);
        numinous_core::draw_text(&mut expected, "QTAIL", 324, 101, 1, '#');
        let expected = expected.to_rgba();
        let blank = Raster::with_accent(360, 240, [240, 180, 110]).to_rgba();
        let mut checked = 0;
        for y in 101..108 {
            for x in 324..354 {
                let index = (y * 360 + x) * 4;
                if expected[index..index + 3] != blank[index..index + 3] {
                    assert_eq!(
                        &actual[index..index + 3],
                        &expected[index..index + 3],
                        "tail glyph at {x},{y}"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 30);
    }
    #[test]
    fn route_authoring_maximum_network_stays_inspectable_and_refuses_overflow_transactionally() {
        let mut town = RouteWorkbench::first_town().town().clone();
        town.junctions = 32;
        town.roads.clear();
        for from in 0..32 {
            for to in from + 1..32 {
                if town.roads.len() < 96 {
                    town.roads.push(EditableRoad {
                        road: Road {
                            from,
                            to,
                            cost: 999,
                        },
                        open: true,
                    });
                }
            }
        }
        let creation = RouteCreation::new(town).unwrap();
        let mut panel = Panel::opened(creation);
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Roads));
        for _ in 0..95 {
            panel.act(Action::Next);
        }
        assert!(panel.lines()[1].contains("96/96"));
        let before = panel.workbench.snapshot();
        panel.act(Action::NewRoad);
        panel.act(Action::From(10));
        panel.act(Action::To(10));
        panel.act(Action::CommitRoad);
        assert_eq!(panel.workbench.snapshot(), before);
        assert!(panel.draft.is_some());
        for page in PAGES {
            panel.act(Action::Page(page));
            panel.draw(360, 240, None);
            panel.draw(900, 700, None);
        }
    }
    #[test]
    fn route_authoring_route_offers_and_order_selection_remain_deliberate() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        let opening = panel.workbench.snapshot();
        panel.act(Action::Undo);
        assert_eq!(panel.workbench.snapshot(), opening);
        assert_eq!(panel.message, "Nothing to undo.");
        panel.act(Action::Nearest);
        assert_eq!(panel.workbench.compare().unwrap().current.cost, 9);
        assert_eq!(panel.message, "Already at this setting.");
        panel.act(Action::Shorter);
        assert_eq!(panel.workbench.compare().unwrap().current.cost, 8);
        let best = panel.workbench.snapshot();
        panel.act(Action::Shorter);
        assert_eq!(panel.workbench.snapshot(), best);
        assert!(panel.message.contains("No shorter offer"));
        panel.act(Action::Page(Page::Order));
        panel.act(Action::Next);
        assert_eq!(panel.order, 2);
        panel.act(Action::Previous);
        assert_eq!(panel.order, 1);
        panel.act(Action::Previous);
        assert_eq!(panel.order, 3);
        panel.act(Action::Next);
        assert_eq!(panel.order, 1);
        panel.act(Action::Nearest);
        assert_eq!(panel.workbench.compare().unwrap().current.cost, 9);
        panel.act(Action::Reset);
        assert_eq!(panel.workbench.snapshot(), opening);
        panel.act(Action::Remix);
        assert_eq!(panel.workbench.snapshot(), opening);
        assert!(panel.message.contains("Open a kept route"));
        assert_eq!(panel.act(Action::Keep), Effect::Keep);
        assert_eq!(panel.act(Action::Close), Effect::Close);
        assert_eq!(panel.workbench.snapshot(), opening);
    }

    #[test]
    fn route_authoring_new_road_cancel_bounds_and_reversed_endpoints_preserve_targets() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.act(Action::Page(Page::Roads));
        let opening = panel.workbench.snapshot();
        for action in [Action::From(1), Action::To(-1), Action::CommitRoad] {
            panel.act(action);
            assert_eq!(panel.workbench.snapshot(), opening);
            assert!(panel.message.contains("NEW ROAD"));
        }
        panel.act(Action::NewRoad);
        panel.act(Action::Cost(2000));
        assert_eq!(panel.draft.unwrap().cost, 999);
        panel.act(Action::Cost(-2000));
        assert_eq!(panel.draft.unwrap().cost, 1);
        panel.act(Action::NewRoad);
        assert!(panel.draft.is_none());
        assert_eq!(panel.workbench.snapshot(), opening);
        panel.act(Action::NewRoad);
        panel.act(Action::From(3));
        panel.act(Action::To(-1));
        panel.act(Action::CommitRoad);
        let selected = panel.selected_road().unwrap();
        assert_eq!(
            (
                selected.road.from.min(selected.road.to),
                selected.road.from.max(selected.road.to)
            ),
            (0, 3)
        );
        panel.act(Action::Cost(1));
        assert_eq!(panel.selected_road().unwrap().road.cost, 2);
        panel.act(Action::NewRoad);
        panel.act(Action::Next);
        assert!(panel.draft.is_none());
        panel.act(Action::Reset);
        assert_eq!(panel.workbench.snapshot(), opening);
    }

    #[test]
    fn route_authoring_disconnected_search_and_closed_road_glyph_remain_honest() {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.act(Action::ToggleRoad);
        let actual = panel.draw(900, 700, None).to_rgba();
        let mut expected = Raster::with_accent(900, 700, [240, 180, 110]);
        numinous_core::draw_text(&mut expected, "X", 607, 472, 2, '#');
        let expected = expected.to_rgba();
        let blank = Raster::with_accent(900, 700, [240, 180, 110]).to_rgba();
        let mut glyph_pixels = 0;
        for y in 472..486 {
            for x in 607..617 {
                let index = (y * 900 + x) * 4;
                if expected[index..index + 3] != blank[index..index + 3] {
                    assert_eq!(&actual[index..index + 3], &expected[index..index + 3]);
                    glyph_pixels += 1;
                }
            }
        }
        assert!(glyph_pixels > 40);
        panel.act(Action::Page(Page::Roads));
        for _ in 0..5 {
            panel.act(Action::DeleteRoad);
        }
        assert!(panel.workbench.town().roads.is_empty());
        assert!(panel.lines()[0].contains("NO COMPARISON"));
        assert!(panel.lines()[1].contains("NO ROADS"));
        panel.act(Action::Page(Page::View));
        let empty = panel.draw(900, 700, None).to_rgba();
        let center = (511 * 900 + 450) * 4;
        let background = Raster::with_accent(900, 700, [240, 180, 110]).to_rgba();
        assert_eq!(
            &empty[center..center + 3],
            &background[center..center + 3],
            "disconnected graph must not paint an invented route through its center"
        );
        panel.act(Action::Page(Page::Stops));
        panel.act(Action::Next);
        assert!(panel.lines()[1].contains("DELIVERY"));
        panel.act(Action::Page(Page::Search));
        panel.act(Action::Step);
        assert!(panel.message.contains("START SEARCH"));
        panel.act(Action::Search);
        assert!(!panel.lines()[1].contains("NO PATH"));
        panel.act(Action::Step);
        assert!(panel.lines()[1].contains("NO PATH"));
        assert!(!panel.lines()[1].contains("PATH ["));
        panel.act(Action::Back);
        assert!(!panel.lines()[1].contains("NO PATH"));
        panel.draw(360, 240, None);
        assert_eq!(panel.workbench.trace().unwrap().cursor(), 0);
        panel.act(Action::Reset);
        panel.act(Action::Junctions(1));
        panel.act(Action::Page(Page::Stops));
        for _ in 0..3 {
            panel.act(Action::Next);
        }
        assert!(panel.lines()[1].contains("PASS THROUGH"));
    }

    #[test]
    fn route_authoring_paused_controller_pointer_and_live_prefix_do_not_autoplay() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut panel = Panel::opened(creation);
        let initial = panel.workbench.snapshot();
        let point = {
            let button = &panel.buttons()[0];
            let (x, y, w, h) = button.bounds;
            (x + w / 2.0, y + h / 2.0)
        };
        panel.pointer_at(point, false);
        assert!(panel.paused);
        assert_eq!(panel.workbench.snapshot(), initial);
        panel.draw(360, 240, Some("DPAD SELECT. A ACTS. B LEAVES."));
        assert!(panel.paused);
        panel.controller_activate(Some(point));
        assert!(!panel.paused);
        assert_eq!(panel.workbench.snapshot(), initial);
        panel.act(Action::Page(Page::Search));
        for _ in 0..3 {
            panel.act(Action::Next);
        }
        panel.act(Action::Search);
        panel.act(Action::Step);
        panel.act(Action::Step);
        assert!(panel.lines()[1].contains("TENTATIVE"));
        let partial = panel.workbench.snapshot();
        panel.draw(900, 700, None);
        assert_eq!(panel.workbench.snapshot(), partial);
    }
}
