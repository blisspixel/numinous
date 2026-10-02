//! Native route authoring. Core owns validation, undo, search and comparison.

use numinous_core::route::{Road, RouteEvent};
use numinous_core::route_workbench::{
    EditableRoad, RouteEdit, RouteSearchState, RouteSearchView, RouteWorkbench,
    RouteWorkbenchSnapshot,
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
    Share,
    Browse,
    QuestionPage(i32),
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
    Close,
    Keep,
    Share,
    Browse,
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
    received: Option<numinous_core::ProjectDocument>,
    received_kept: bool,
    pub shared_path: Option<String>,
    question_text: std::cell::RefCell<Option<QuestionText>>,
}

#[derive(Debug)]
struct QuestionText {
    renderer: crate::study_text::StudyText,
    layout: Option<std::sync::Arc<crate::study_text::TextLayout>>,
    source: String,
    dimensions: (u32, u32),
    scale: i32,
    scroll: f32,
    maximum: f32,
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
            received: None,
            received_kept: false,
            shared_path: None,
            question_text: std::cell::RefCell::new(None),
        }
    }

    pub fn opened(creation: RouteCreation) -> Self {
        let mut panel = Self::new(creation.open());
        panel.source = Some(creation);
        panel.paused = true;
        panel
    }

    /// Preview a portable question without importing or executing its next call.
    pub fn received(document: numinous_core::ProjectDocument) -> Result<Self, String> {
        let preview = document.preview(
            &numinous_core::Journal::default(),
            numinous_core::ReceiptCheck::NotSupplied,
        );
        if preview.creation.kind != Some(numinous_core::CreationKind::Route) {
            return Err("This project does not contain a route creation".into());
        }
        let capsule = preview
            .creation
            .capsule
            .as_deref()
            .ok_or("This route creation cannot be opened")?;
        let creation = RouteCreation::from_capsule(capsule).map_err(|error| error.to_string())?;
        let mut panel = Self::opened(creation);
        panel.question = preview.question;
        panel.received = Some(document);
        Ok(panel)
    }

    /// Forward the admitted document while its question and creation remain unchanged.
    pub fn received_document(&self) -> Option<&numinous_core::ProjectDocument> {
        let document = self.received.as_ref()?;
        let preview = document.preview(
            &numinous_core::Journal::default(),
            numinous_core::ReceiptCheck::NotSupplied,
        );
        let draft = self.project_draft(0).ok()?;
        (draft.question == preview.question
            && draft.creation.as_deref() == preview.creation.capsule.as_deref())
        .then_some(document)
    }

    pub fn is_received_preview(&self) -> bool {
        self.received.is_some() && self.paused
    }

    pub fn mark_received_kept(&mut self) {
        self.received_kept = true;
        self.message = "QUESTION KEPT. OPEN STARTS THIS NETWORK.".into();
    }

    pub fn workbench(&self) -> &RouteWorkbench {
        &self.workbench
    }

    pub fn push_text(&mut self, text: &str) {
        if self.page == Page::Keep && !self.paused {
            self.shared_path = None;
            let mut count = self.question.chars().count();
            for ch in text.chars().filter(|ch| (' '..='~').contains(ch)) {
                if count < numinous_core::MAX_WORKSPACE_TEXT_CHARS {
                    self.question.push(ch);
                    count += 1;
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
        let mut entries: Vec<(String, Action)> = if self.is_received_preview() {
            vec![
                ("OPEN ROUTE".into(), Action::Confirm),
                (
                    if self.received_kept {
                        "QUESTION KEPT"
                    } else {
                        "KEEP QUESTION"
                    }
                    .into(),
                    Action::Keep,
                ),
                ("CANCEL".into(), Action::Close),
            ]
        } else if self.paused {
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
        if self.is_received_preview()
            && self
                .question_text
                .borrow()
                .as_ref()
                .is_some_and(|text| text.maximum > 0.0)
        {
            entries.extend([
                ("QUESTION <".into(), Action::QuestionPage(-1)),
                ("QUESTION >".into(), Action::QuestionPage(1)),
            ]);
        }
        let tab_count = entries.len();
        let mut body: Vec<(&str, Action)> = if self.paused {
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
                    ("JUNCTION <", Action::Previous),
                    ("JUNCTION >", Action::Next),
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
                    ("SHARE QUESTION", Action::Share),
                    ("GALLERY", Action::Browse),
                    ("REMIX SOURCE", Action::Remix),
                    ("BASELINE", Action::Reset),
                ],
            }
        };
        if !self.paused
            && self.page == Page::Keep
            && self
                .question_text
                .borrow()
                .as_ref()
                .is_some_and(|text| text.maximum > 0.0)
        {
            body.extend([
                (
                    if self.shared_path.is_some() {
                        "PATH <"
                    } else {
                        "QUESTION <"
                    },
                    Action::QuestionPage(-1),
                ),
                (
                    if self.shared_path.is_some() {
                        "PATH >"
                    } else {
                        "QUESTION >"
                    },
                    Action::QuestionPage(1),
                ),
            ]);
        }
        entries.extend(
            body.into_iter()
                .map(|(label, action)| (label.to_string(), action)),
        );
        entries
            .into_iter()
            .enumerate()
            .map(|(index, (label, action))| {
                let bounds = if self.is_received_preview() {
                    if index < 3 {
                        (0.02 + index as f64 * 0.325, 0.76, 0.315, 0.085)
                    } else {
                        (0.02 + (index - 3) as f64 * 0.325, 0.43, 0.315, 0.045)
                    }
                } else if self.paused {
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
                        (if self.page == Page::Keep {
                            0.60
                        } else if self.page == Page::Search {
                            0.72
                        } else {
                            0.40
                        }) + row as f64
                            * if self.page == Page::Keep {
                                0.065
                            } else {
                                0.075
                            },
                        0.315,
                        if self.page == Page::Keep { 0.06 } else { 0.065 },
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
            return Effect::None;
        }
        if self.page == Page::Search && activate && !self.paused {
            let nearest = self
                .network_locations()
                .into_iter()
                .enumerate()
                .filter_map(|(junction, center)| {
                    let dx = (point.0 - center.0) / 0.025;
                    let dy = (point.1 - center.1) / 0.018;
                    let distance = dx * dx + dy * dy;
                    (distance <= 1.0).then_some((junction, distance))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((junction, _)) = nearest {
                self.junction = junction;
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
        if let Action::QuestionPage(direction) = action {
            if (self.is_received_preview() || self.page == Page::Keep)
                && let Some(text) = self.question_text.get_mut().as_mut()
            {
                text.scroll = (text.scroll
                    + text.dimensions.1 as f32 * direction.clamp(-1, 1) as f32)
                    .clamp(0.0, text.maximum);
            }
            return Effect::None;
        }
        if action == Action::Close {
            return Effect::Close;
        }
        if self.paused {
            if action == Action::Keep && self.received.is_some() {
                return Effect::Keep;
            }
            if action == Action::Confirm {
                self.paused = false;
            }
            return Effect::None;
        }
        self.shared_path = None;
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
                if let Some(selected) = self.selected_road().filter(|_| !self.is_received_preview())
                {
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
            Action::Share => return Effect::Share,
            Action::Browse => return Effect::Browse,
            Action::Close | Action::Confirm | Action::QuestionPage(_) => {}
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
        let comparison = if self.page == Page::Search {
            self.workbench.trace().map_or_else(
                || {
                    format!(
                        "SEARCH {}>{}: START, THEN STEP",
                        town.stops[0], self.junction
                    )
                },
                |trace| {
                    let view = trace.view();
                    format!(
                        "SEARCH {}>{}: {}/{} DECISIONS {}",
                        view.from,
                        view.to,
                        view.cursor,
                        view.event_count,
                        if view.completed { "COMPLETE" } else { "MANUAL" }
                    )
                },
            )
        } else {
            match self.workbench.compare() {
                Ok(comparison) => format!(
                    "ROUND TRIP {}  EXACT BEST {}",
                    comparison.current.cost, comparison.exact.tour.cost
                ),
                Err(error) => format!("NO COMPARISON: {error}"),
            }
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

    fn search_narration(&self) -> String {
        let Some(trace) = self.workbench.trace() else {
            return format!(
                "START RECORDS DEPOT {} TO JUNCTION {}",
                self.workbench.town().stops[0],
                self.junction
            );
        };
        if let Some(result) = trace.result() {
            return match result {
                Ok(path) => format!(
                    "FINAL PATH COST {} FROM {} TO {}",
                    path.cost,
                    trace.snapshot().from,
                    trace.snapshot().to
                ),
                Err(_) => "NO PATH. REOPEN ROADS OR CHANGE THE TARGET.".into(),
            };
        }
        match trace.view().active_event {
            Some(RouteEvent::Settled { junction, cost }) => {
                format!("{junction} FINAL: COST {cost} FROM SOURCE")
            }
            Some(RouteEvent::Relaxed { from, to, cost }) => {
                format!("{from}->{to} IMPROVED: TENTATIVE COST {cost}")
            }
            None => "STEP REVEALS ONE RECORDED DECISION. BACK HIDES IT.".into(),
        }
    }

    fn search_inspector(&self, view: Option<&RouteSearchView>) -> String {
        let Some(reading) = view.and_then(|view| view.junctions.get(self.junction)) else {
            return format!(
                "JUNCTION {}: UNSEEN. START USES THIS TARGET.",
                self.junction
            );
        };
        let state = match reading.state {
            RouteSearchState::Unseen => "UNSEEN",
            RouteSearchState::Tentative => "TENTATIVE",
            RouteSearchState::Settled => "FINAL",
            RouteSearchState::Unreachable => "UNREACHABLE",
        };
        format!(
            "JUNCTION {}: {} COST {} VIA {}",
            self.junction,
            state,
            reading
                .cost
                .map_or_else(|| "?".into(), |cost| cost.to_string()),
            reading
                .predecessor
                .map_or_else(|| "-".into(), |from| from.to_string())
        )
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
                format!("{prefix} {junction} FINAL FROM SOURCE {cost}")
            }
            Some(RouteEvent::Relaxed { from, to, cost }) => {
                format!("{prefix} {to} TENTATIVE {cost} VIA {from}")
            }
            None => prefix,
        }
    }

    pub fn draw(&self, width: usize, height: usize, controller_hint: Option<&str>) -> Raster {
        let mut raster = Raster::with_accent(width, height, [240, 180, 110]);
        let (width, height) = (raster.width(), raster.height());
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
            if self.is_received_preview() {
                "ROUTE QUESTION: PREVIEW"
            } else if self.paused {
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
        if self.is_received_preview() {
            self.draw_question(&mut raster, width, height, scale);
            if self.workbench.town().junctions > 8 {
                let town = self.workbench.town();
                text(
                    &mut raster,
                    &format!(
                        "{} / {} / {}",
                        numinous_core::counted(town.junctions, "junction"),
                        numinous_core::counted(town.roads.len(), "road"),
                        numinous_core::counted(town.stops.len(), "stop")
                    )
                    .to_uppercase(),
                    0.635,
                );
            }
            text(
                &mut raster,
                "OPEN EMBEDDED ROUTE. NEXT CALL NOT RUN.",
                0.675,
            );
            text(
                &mut raster,
                "KEEP IMPORTS THE QUESTION. CANCEL RESTORES.",
                0.71,
            );
        } else if self.page == Page::Keep {
            if let Some(path) = &self.shared_path {
                self.draw_question_in(
                    &mut raster,
                    (width, height),
                    crate::study_text::TextViewport {
                        x: 6,
                        y: (height as f64 * 0.29) as i32,
                        width: width.saturating_sub(12).clamp(1, 4096) as u32,
                        height: (height as f64 * 0.24) as u32,
                    },
                    &format!("Shared to: {path}"),
                    scale,
                );
            } else {
                self.draw_question(&mut raster, width, height, scale);
            }
            text(
                &mut raster,
                &format!(
                    "CHAR {}. KEEP STORES NETWORK / ORDER.",
                    LETTERS[self.character] as char
                ),
                0.55,
            );
        } else if self.page == Page::Search {
            text(&mut raster, &self.search_narration(), 0.29);
            text(
                &mut raster,
                "[] FINAL <> TENTATIVE . UNSEEN X CUT OFF",
                0.34,
            );
            text(
                &mut raster,
                "DASH: VIA  DOUBLE: LATEST  TRIPLE: PATH. CLICK NODE",
                0.38,
            );
            let view = self.workbench.trace().map(|trace| trace.view());
            self.draw_network(&mut raster, width, height, scale, view.as_ref());
            text(&mut raster, &self.search_inspector(view.as_ref()), 0.675);
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
            self.draw_network(&mut raster, width, height, scale, None);
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

    fn draw_question(&self, raster: &mut Raster, width: usize, height: usize, scale: i32) {
        use crate::study_text::TextViewport;
        let (top, band) = if self.is_received_preview() {
            (0.15, 0.27)
        } else {
            (0.29, 0.24)
        };
        let viewport = TextViewport {
            x: 6,
            y: (height as f64 * top) as i32,
            width: width.saturating_sub(12).clamp(1, 4096) as u32,
            height: (height as f64 * band) as u32,
        };
        self.draw_question_in(
            raster,
            (width, height),
            viewport,
            &format!("Question: {}", self.question),
            scale,
        );
    }

    /// Draw a case-preserving route question caption with the bundled question renderer.
    pub fn draw_question_caption(
        &self,
        raster: &mut Raster,
        viewport: crate::study_text::TextViewport,
    ) {
        let dimensions = (raster.width(), raster.height());
        self.draw_question_in(
            raster,
            dimensions,
            viewport,
            &format!("Route: {}", self.question),
            1,
        );
    }

    fn draw_question_in(
        &self,
        raster: &mut Raster,
        frame_dimensions: (usize, usize),
        viewport: crate::study_text::TextViewport,
        source: &str,
        scale: i32,
    ) {
        use crate::study_text::{StudyText, TextRole, TextSpan};
        let result = (|| {
            let mut cache = self.question_text.borrow_mut();
            if cache.is_none() {
                *cache = Some(QuestionText {
                    renderer: StudyText::new("en")?,
                    layout: None,
                    source: String::new(),
                    dimensions: (0, 0),
                    scale: 0,
                    scroll: 0.0,
                    maximum: 0.0,
                });
            }
            let text = cache.as_mut().unwrap();
            let source = numinous_core::display_safe(source);
            let dimensions = (viewport.width, viewport.height);
            if text.source != source || text.dimensions != dimensions || text.scale != scale {
                let anchor = text
                    .layout
                    .as_ref()
                    .map_or(0, |layout| layout.source_offset_at_scroll(text.scroll));
                let mut fit = |source: &str| {
                    let spans = [TextSpan {
                        text: source,
                        role: TextRole::Prose,
                    }];
                    let mut font_size = (10 * scale).clamp(10, 28) as f32;
                    loop {
                        let layout = text.renderer.layout(&spans, viewport.width, font_size)?;
                        if layout.height() <= viewport.height as f32 || font_size <= 8.0 {
                            break Ok::<_, crate::study_text::TextError>(layout);
                        }
                        font_size -= 2.0;
                    }
                };
                let mut layout = fit(&source)?;
                if !layout.missing_glyphs().is_empty() {
                    layout = fit(&format!("Some characters unavailable. {source}"))?;
                }
                text.maximum = layout.max_scroll(viewport.height);
                text.scroll = if text.source == source {
                    layout.scroll_for_source_offset(anchor).min(text.maximum)
                } else {
                    0.0
                };
                text.layout = Some(layout);
                text.source = source;
                text.dimensions = dimensions;
                text.scale = scale;
            }
            let layout = text.layout.as_ref().unwrap();
            let mut rgba = raster.to_rgba();
            text.renderer.draw(
                layout,
                &mut rgba,
                frame_dimensions,
                viewport,
                text.scroll,
                [233, 237, 240, 255],
            )?;
            raster.set_rgba(&rgba);
            Ok::<_, crate::study_text::TextError>(())
        })();
        if result.is_err() {
            numinous_core::draw_text(
                raster,
                "QUESTION TEXT COULD NOT BE DRAWN",
                6,
                viewport.y,
                scale,
                '#',
            );
        }
    }

    /// Draw a portable thumbnail through the editor's authored network renderer.
    pub fn thumbnail(&self, width: usize, height: usize) -> Raster {
        let mut raster = Raster::with_accent(width, height, [240, 180, 110]);
        let (width, height) = (raster.width(), raster.height());
        let points = (0..self.workbench.town().junctions)
            .map(|id| {
                let angle = std::f64::consts::TAU * id as f64
                    / self.workbench.town().junctions as f64
                    - std::f64::consts::FRAC_PI_2;
                (
                    (width as f64 * (0.5 + 0.35 * angle.cos())) as i32,
                    (height as f64 * (0.5 + 0.35 * angle.sin())) as i32,
                )
            })
            .collect::<Vec<_>>();
        self.draw_network_at(&mut raster, (width, height), 1, None, &points);
        raster
    }

    fn network_locations(&self) -> Vec<(f64, f64)> {
        let search = self.page == Page::Search || self.is_received_preview();
        (0..self.workbench.town().junctions)
            .map(|id| {
                let angle = std::f64::consts::TAU * id as f64
                    / self.workbench.town().junctions as f64
                    - std::f64::consts::FRAC_PI_2;
                (
                    0.5 + 0.35 * angle.cos(),
                    if search {
                        0.55 + 0.065 * angle.sin()
                    } else {
                        0.73 + 0.11 * angle.sin()
                    },
                )
            })
            .collect()
    }

    fn network_points(&self, width: usize, height: usize) -> Vec<(i32, i32)> {
        self.network_locations()
            .into_iter()
            .map(|(x, y)| ((width as f64 * x) as i32, (height as f64 * y) as i32))
            .collect()
    }

    fn draw_network(
        &self,
        raster: &mut Raster,
        width: usize,
        height: usize,
        scale: i32,
        view: Option<&RouteSearchView>,
    ) {
        let points = self.network_points(width, height);
        self.draw_network_at(raster, (width, height), scale, view, &points);
    }

    fn draw_network_at(
        &self,
        raster: &mut Raster,
        dimensions: (usize, usize),
        scale: i32,
        view: Option<&RouteSearchView>,
        points: &[(i32, i32)],
    ) {
        let (width, height) = dimensions;
        let town = self.workbench.town();
        for road in &town.roads {
            let (a, b) = (points[road.road.from], points[road.road.to]);
            let mut background = BackgroundRoads {
                raster,
                quiet: self.page == Page::Search || self.is_received_preview(),
            };
            if road.open {
                background.line(a.0, a.1, b.0, b.1, '.');
            } else {
                let mid = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
                background.line(a.0, a.1, (a.0 + mid.0) / 2, (a.1 + mid.1) / 2, '.');
                background.line(b.0, b.1, (b.0 + mid.0) / 2, (b.1 + mid.1) / 2, '.');
            }
        }
        for road in town.roads.iter().filter(|road| !road.open) {
            let (a, b) = (points[road.road.from], points[road.road.to]);
            numinous_core::draw_text(raster, "X", (a.0 + b.0) / 2, (a.1 + b.1) / 2, scale, '#');
        }
        if self.page == Page::Search {
            if let Some(view) = view {
                for junction in &view.junctions {
                    if let Some(from) = junction.predecessor {
                        draw_predecessor(raster, points[from], points[junction.junction], scale);
                    }
                }
                if let Some(RouteEvent::Relaxed { from, to, .. }) = view.active_event {
                    draw_stroke(raster, points[from], points[to], 2);
                }
                if let Some(Ok(path)) = self.workbench.trace().and_then(|trace| trace.result()) {
                    for pair in path.junctions.windows(2) {
                        draw_stroke(raster, points[pair[0]], points[pair[1]], 3);
                    }
                }
            }
            let dense = town.junctions > 8;
            let active = view.and_then(|view| match view.active_event {
                Some(RouteEvent::Settled { junction, .. }) => Some(junction),
                Some(RouteEvent::Relaxed { to, .. }) => Some(to),
                None => None,
            });
            let inspected_label = format!(">{}", self.junction);
            let inspected_width = inspected_label.len() as i32 * 6 * scale;
            let inspected_origin = search_label_origin(
                points[self.junction],
                inspected_width,
                1,
                scale,
                width,
                height,
            );
            for (id, &point) in points.iter().enumerate() {
                let reading = view.and_then(|view| view.junctions.get(id));
                let state = reading.map_or(RouteSearchState::Unseen, |reading| reading.state);
                let radius = if dense { 1 } else { 3 * scale };
                if dense && id == self.junction {
                    draw_inspection_brackets(raster, point, 3 * scale);
                }
                draw_search_node(raster, point, radius, state);
                if !dense || id == self.junction || active == Some(id) {
                    let label = if !dense || id != self.junction {
                        format!(
                            "{id}:{}",
                            reading
                                .and_then(|reading| reading.cost)
                                .map_or_else(|| "?".into(), |cost| cost.to_string())
                        )
                    } else {
                        inspected_label.clone()
                    };
                    let text_width = label.len() as i32 * 6 * scale;
                    let (x, mut y) =
                        search_label_origin(point, text_width, radius, scale, width, height);
                    if dense
                        && id != self.junction
                        && x < inspected_origin.0 + inspected_width + 2
                        && x + text_width + 2 > inspected_origin.0
                        && y < inspected_origin.1 + 7 * scale + 2
                        && y + 7 * scale + 2 > inspected_origin.1
                    {
                        y = inspected_origin.1 + 7 * scale + 3;
                    }
                    numinous_core::draw_text(raster, &label, x, y, scale, '#');
                }
            }
        } else {
            if let Ok(comparison) = self.workbench.compare() {
                for pair in comparison.current.walk.windows(2) {
                    draw_stroke(raster, points[pair[0]], points[pair[1]], 2);
                }
            }
            for (id, point) in points.iter().enumerate() {
                if self.is_received_preview() && town.junctions > 8 {
                    let radius = 2 * scale;
                    if id == town.stops[0] {
                        draw_search_node(raster, *point, radius, RouteSearchState::Settled);
                    } else if town.stops.contains(&id) {
                        draw_search_node(raster, *point, radius, RouteSearchState::Tentative);
                    } else {
                        raster.plot(point.0, point.1, '#');
                    }
                    if id == town.stops[0] || id == self.junction {
                        let label = if id == town.stops[0] {
                            format!("{id}D")
                        } else {
                            format!(">{id}")
                        };
                        numinous_core::draw_text(
                            raster,
                            &label,
                            point.0 + radius + 2,
                            point.1,
                            scale,
                            '#',
                        );
                    }
                    continue;
                }
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
}

struct BackgroundRoads<'a> {
    raster: &'a mut Raster,
    quiet: bool,
}

impl Surface for BackgroundRoads<'_> {
    fn width(&self) -> usize {
        self.raster.width()
    }
    fn height(&self) -> usize {
        self.raster.height()
    }
    fn plot(&mut self, x: i32, y: i32, mark: char) {
        if self.quiet {
            // Replace the background ink so dense crossings cannot outshine
            // the recorded search. Surface owns line geometry and clipping.
            self.raster.shade_rect(x, y, 1, 1, 0.0);
        } else {
            self.raster.plot(x, y, mark);
        }
    }
}

fn search_label_origin(
    point: (i32, i32),
    text_width: i32,
    radius: i32,
    scale: i32,
    width: usize,
    height: usize,
) -> (i32, i32) {
    let (x, y) = if point.1 < (height as f64 * 0.515) as i32 {
        (point.0 - text_width / 2, point.1 - radius - 2 - 7 * scale)
    } else if point.1 > (height as f64 * 0.585) as i32 {
        (point.0 - text_width / 2, point.1 + radius + 2)
    } else if point.0 < width as i32 / 2 {
        (point.0 - radius - 2 - text_width, point.1 - 3 * scale)
    } else {
        (point.0 + radius + 2, point.1 - 3 * scale)
    };
    (x.clamp(2, (width as i32 - text_width - 2).max(2)), y)
}

fn draw_inspection_brackets(raster: &mut Raster, point: (i32, i32), radius: i32) {
    let (x, y) = point;
    for side in [-1, 1] {
        let edge = x + side * radius;
        raster.line(edge, y - radius, edge, y + radius, '#');
        raster.line(edge, y - radius, edge - side * radius / 2, y - radius, '#');
        raster.line(edge, y + radius, edge - side * radius / 2, y + radius, '#');
    }
}

fn draw_stroke(raster: &mut Raster, a: (i32, i32), b: (i32, i32), thickness: i32) {
    let horizontal = (b.0 - a.0).abs() >= (b.1 - a.1).abs();
    for offset in 0..thickness {
        let (x, y) = if horizontal { (0, offset) } else { (offset, 0) };
        raster.line(a.0 + x, a.1 + y, b.0 + x, b.1 + y, '#');
    }
}

fn draw_predecessor(raster: &mut Raster, a: (i32, i32), b: (i32, i32), scale: i32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.abs().max(dy.abs()).max(1);
    let dash = 4 * scale;
    for start in (0..length).step_by((dash * 2) as usize) {
        let end = (start + dash).min(length);
        raster.line(
            a.0 + dx * start / length,
            a.1 + dy * start / length,
            a.0 + dx * end / length,
            a.1 + dy * end / length,
            '#',
        );
    }
    let norm = ((dx * dx + dy * dy) as f64).sqrt();
    if norm > 0.0 {
        let (ux, uy) = (dx as f64 / norm, dy as f64 / norm);
        let tip = (a.0 + dx * 3 / 4, a.1 + dy * 3 / 4);
        let arm = (3 * scale) as f64;
        for side in [-1.0, 1.0] {
            raster.line(
                tip.0,
                tip.1,
                tip.0 + (-ux * arm - uy * arm * side) as i32,
                tip.1 + (-uy * arm + ux * arm * side) as i32,
                '#',
            );
        }
    }
}

fn draw_search_node(raster: &mut Raster, point: (i32, i32), radius: i32, state: RouteSearchState) {
    let (x, y) = point;
    match state {
        RouteSearchState::Unseen => raster.plot(x, y, '#'),
        RouteSearchState::Tentative => {
            raster.line(x, y - radius, x + radius, y, '#');
            raster.line(x + radius, y, x, y + radius, '#');
            raster.line(x, y + radius, x - radius, y, '#');
            raster.line(x - radius, y, x, y - radius, '#');
        }
        RouteSearchState::Settled => {
            raster.line(x - radius, y - radius, x + radius, y - radius, '#');
            raster.line(x + radius, y - radius, x + radius, y + radius, '#');
            raster.line(x + radius, y + radius, x - radius, y + radius, '#');
            raster.line(x - radius, y + radius, x - radius, y - radius, '#');
        }
        RouteSearchState::Unreachable => {
            raster.line(x - radius, y - radius, x + radius, y + radius, '#');
            raster.line(x - radius, y + radius, x + radius, y - radius, '#');
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
        panel.draw(360, 240, None);
        panel.act(Action::QuestionPage(i32::MAX));
        let actual = panel.draw(360, 240, None).to_rgba();
        let original = panel.question.clone();
        let layout = panel
            .question_text
            .borrow()
            .as_ref()
            .unwrap()
            .layout
            .clone()
            .unwrap();
        assert_eq!(layout.source(), format!("Question: {original}"));
        assert_eq!(
            panel.question_text.borrow().as_ref().unwrap().scroll,
            layout.max_scroll((240.0 * 0.24) as u32)
        );
        panel.question = format!("{}BBBBB", "a".repeat(275));
        panel.draw(360, 240, None);
        panel.act(Action::QuestionPage(i32::MAX));
        let altered = panel.draw(360, 240, None).to_rgba();
        let row = 360 * 4;
        assert!(
            actual[69 * row..127 * row] != altered[69 * row..127 * row],
            "the final five characters have visible ink"
        );
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
    fn search_panel() -> Panel {
        let mut panel = Panel::new(RouteWorkbench::first_town());
        panel.act(Action::Page(Page::Search));
        for _ in 0..3 {
            panel.act(Action::Next);
        }
        panel.act(Action::Search);
        panel
    }
    fn step_search(panel: &mut Panel) {
        let before = panel.workbench.trace().unwrap().cursor();
        panel.act(Action::Step);
        assert_eq!(panel.workbench.trace().unwrap().cursor(), before + 1);
    }
    fn finish_search(panel: &mut Panel) {
        let limit = panel.workbench.trace().unwrap().events().len();
        for _ in 0..limit {
            if panel.workbench.trace().unwrap().completed() {
                break;
            }
            step_search(panel);
        }
        assert!(panel.workbench.trace().unwrap().completed());
    }
    fn step_until_cost(panel: &mut Panel, junction: usize, cost: u32) {
        let limit = panel.workbench.trace().unwrap().events().len();
        for _ in 0..limit {
            if panel.workbench.trace().unwrap().view().junctions[junction].cost == Some(cost) {
                break;
            }
            step_search(panel);
        }
        assert_eq!(
            panel.workbench.trace().unwrap().view().junctions[junction].cost,
            Some(cost)
        );
    }
    fn pixel(raster: &Raster, x: usize, y: usize) -> [u8; 3] {
        let rgba = raster.to_rgba();
        let offset = (y * raster.width() + x) * 4;
        rgba[offset..offset + 3].try_into().unwrap()
    }
    fn marked_pixel(mark: char) -> [u8; 3] {
        let mut raster = Raster::with_accent(1, 1, [240, 180, 110]);
        raster.plot(0, 0, mark);
        pixel(&raster, 0, 0)
    }
    fn quiet_road_pixel() -> [u8; 3] {
        let mut raster = Raster::with_accent(1, 1, [240, 180, 110]);
        raster.shade_rect(0, 0, 1, 1, 0.0);
        pixel(&raster, 0, 0)
    }
    fn assert_text_pixels(actual: &Raster, text: &str, x: i32, y: i32, scale: i32) {
        let mut expected = Raster::with_accent(actual.width(), actual.height(), [240, 180, 110]);
        numinous_core::draw_text(&mut expected, text, x, y, scale, '#');
        let expected = expected.to_rgba();
        let actual = actual.to_rgba();
        let blank = Raster::with_accent(1, 1, [240, 180, 110]).to_rgba();
        let mut found = 0;
        for (a, e) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
            if e[..3] != blank[..3] {
                assert!(a[..3].iter().zip(&e[..3]).all(|(a, e)| a >= e));
                found += 1;
            }
        }
        assert!(found > 5);
    }
    #[test]
    fn route_authoring_search_raster_does_not_leak_tour_or_final_path_before_completion() {
        let mut panel = search_panel();
        let untouched = panel.workbench.snapshot();
        let zero = panel.draw(360, 240, None);
        assert_eq!(
            pixel(&zero, 225, 132),
            quiet_road_pixel(),
            "unrevealed search keeps the actual road plain, rather than painting the tour"
        );
        assert_eq!(panel.workbench.snapshot(), untouched);
        for _ in 0..5 {
            panel.act(Action::Step);
        }
        let relaxed = panel.workbench.trace().unwrap().view();
        assert_eq!(
            relaxed.active_event,
            Some(RouteEvent::Relaxed {
                from: 1,
                to: 3,
                cost: 4
            })
        );
        assert!(panel.workbench.trace().unwrap().result().is_none());
        let partial = panel.draw(360, 240, None);
        assert_eq!(pixel(&partial, 225, 133), marked_pixel('#'));
        assert_ne!(pixel(&partial, 225, 134), marked_pixel('#'));
        finish_search(&mut panel);
        let complete = panel.draw(360, 240, None);
        assert_eq!(pixel(&complete, 225, 134), marked_pixel('#'));
        assert!(panel.search_narration().contains("FINAL PATH COST 4"));
        panel.act(Action::Back);
        let backed = panel.draw(360, 240, None);
        assert_ne!(pixel(&backed, 225, 134), marked_pixel('#'));
        assert!(!panel.search_narration().contains("FINAL PATH"));
        panel.act(Action::Page(Page::Roads));
        panel.act(Action::Cost(1));
        panel.act(Action::Page(Page::Search));
        assert!(panel.workbench.trace().is_none());
        let invalidated = panel.draw(360, 240, None);
        assert_eq!(pixel(&invalidated, 225, 132), quiet_road_pixel());
        assert!(panel.search_narration().contains("START RECORDS"));
    }
    #[test]
    fn route_authoring_search_node_shapes_costs_and_inspector_follow_the_visible_prefix() {
        let mut panel = search_panel();
        panel.act(Action::Step);
        panel.act(Action::Step);
        panel.act(Action::Previous);
        panel.act(Action::Previous);
        let view = panel.workbench.trace().unwrap().view();
        let selected = panel.search_inspector(Some(&view));
        assert!(selected.contains("JUNCTION 1: TENTATIVE COST 1"));
        assert!(selected.contains("VIA 0"));
        let tentative = panel.draw(360, 240, None);
        assert_ne!(
            pixel(&tentative, 303, 129),
            marked_pixel('#'),
            "tentative diamond has no settled-square corner"
        );
        panel.act(Action::Step);
        panel.act(Action::Step);
        let settled = panel.draw(360, 240, None);
        assert!(
            pixel(&settled, 303, 129)
                .iter()
                .zip(marked_pixel('#'))
                .all(|(a, e)| *a >= e)
        );
        let view = panel.workbench.trace().unwrap().view();
        assert_eq!(view.junctions[1].state, RouteSearchState::Settled);
        assert!(panel.workbench.trace().unwrap().result().is_none());
        assert_eq!(panel.search_narration(), "1 FINAL: COST 1 FROM SOURCE");
        assert!(
            panel
                .search_inspector(Some(&view))
                .contains("JUNCTION 1: FINAL COST 1")
        );
        assert_text_pixels(&settled, "1 FINAL: COST 1 FROM SOURCE", 6, 69, 1);
        panel.act(Action::Back);
        let backward = panel.draw(360, 240, None);
        assert_ne!(pixel(&backward, 303, 129), marked_pixel('#'));
        assert_eq!(
            panel.workbench.trace().unwrap().view().junctions[1].state,
            RouteSearchState::Tentative
        );
        assert!(panel.lines()[1].contains("SEARCH 0>3"));
        assert!(
            panel
                .search_inspector(Some(&panel.workbench.trace().unwrap().view()))
                .contains("JUNCTION 1")
        );
    }
    #[test]
    fn route_authoring_search_dense_disconnected_inspector_and_controls_survive_composition() {
        let mut town = RouteWorkbench::first_town().town().clone();
        town.junctions = 32;
        town.roads.clear();
        let creation = RouteCreation::new(town).unwrap();
        let mut panel = Panel::opened(creation);
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Search));
        for _ in 0..31 {
            panel.act(Action::Next);
        }
        panel.act(Action::Search);
        let initial = panel.workbench.trace().unwrap().view();
        assert_eq!(initial.junctions[31].state, RouteSearchState::Unseen);
        panel.act(Action::Step);
        let view = panel.workbench.trace().unwrap().view();
        assert_eq!(view.junctions[31].state, RouteSearchState::Unreachable);
        assert!(panel.search_narration().contains("NO PATH"));
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let raster = panel.draw(width, height, None);
            let scale = ((width / 400).min(height / 240)).clamp(1, 3) as i32;
            assert_text_pixels(
                &raster,
                &panel.search_inspector(Some(&view)),
                6,
                (height as f64 * 0.675) as i32,
                scale,
            );
            assert_text_pixels(
                &raster,
                "DASH: VIA  DOUBLE: LATEST  TRIPLE: PATH. CLICK NODE",
                6,
                (height as f64 * 0.38) as i32,
                scale,
            );
            for action in [
                Action::Previous,
                Action::Next,
                Action::Search,
                Action::Back,
                Action::Step,
            ] {
                let button = panel
                    .buttons()
                    .into_iter()
                    .find(|button| button.action == action)
                    .unwrap();
                let (x, y, w, h) = button.bounds;
                assert!(y >= 0.72 && y + h < 0.87);
                assert!(x + w < 1.0);
                panel.pointer_at((x + w / 2.0, y + h / 2.0), false);
            }
            assert_eq!(panel.workbench.trace().unwrap().view(), view);
        }
        panel.act(Action::Back);
        assert_eq!(
            panel.workbench.trace().unwrap().view().junctions[31].state,
            RouteSearchState::Unseen
        );
    }
    #[test]
    fn route_authoring_search_sparse_cost_labels_fit_each_supported_viewport() {
        let mut town = RouteWorkbench::first_town().town().clone();
        town.junctions = 8;
        town.roads = (0..7)
            .map(|from| EditableRoad {
                road: Road {
                    from,
                    to: from + 1,
                    cost: 999,
                },
                open: true,
            })
            .collect();
        let mut panel = Panel::opened(RouteCreation::new(town).unwrap());
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Search));
        for _ in 0..7 {
            panel.act(Action::Next);
        }
        panel.act(Action::Search);
        finish_search(&mut panel);
        let view = panel.workbench.trace().unwrap().view();
        assert_eq!(view.junctions[7].cost, Some(6993));
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let raster = panel.draw(width, height, None);
            let scale = ((width / 400).min(height / 240)).clamp(1, 3) as i32;
            assert_text_pixels(
                &raster,
                "FINAL PATH COST 6993 FROM 0 TO 7",
                6,
                (height as f64 * 0.29) as i32,
                scale,
            );
            assert_text_pixels(
                &raster,
                &panel.search_inspector(Some(&view)),
                6,
                (height as f64 * 0.675) as i32,
                scale,
            );
        }
    }
    #[test]
    fn route_authoring_search_imported_non_depot_source_and_zero_length_path_stay_exact() {
        let mut workbench = RouteWorkbench::first_town();
        workbench.start_trace(2, 2).unwrap();
        let mut panel = Panel::new(workbench);
        panel.act(Action::Page(Page::Search));
        let initial = panel.workbench.trace().unwrap().view();
        assert_eq!(initial.junctions[2].cost, Some(0));
        assert_eq!(initial.junctions[0].cost, None);
        assert!(panel.lines()[0].contains("SEARCH 2>2"));
        finish_search(&mut panel);
        let path = panel
            .workbench
            .trace()
            .unwrap()
            .result()
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(path.junctions, [2]);
        assert_eq!(path.cost, 0);
        assert_eq!(panel.search_narration(), "FINAL PATH COST 0 FROM 2 TO 2");
        let complete = panel.draw(360, 240, None);
        assert_ne!(
            pixel(&complete, 225, 134),
            marked_pixel('#'),
            "zero-length result adds no final road"
        );
        panel.act(Action::Back);
        assert!(!panel.search_narration().contains("FINAL PATH"));
        panel.act(Action::Next);
        assert!(panel.lines()[0].contains("SEARCH 2>2"));
        panel.act(Action::Search);
        let restarted = panel.workbench.trace().unwrap().view();
        assert_eq!((restarted.from, restarted.to, restarted.cursor), (0, 1, 0));
    }
    #[test]
    fn route_authoring_search_improved_predecessor_is_replaced_and_restored_by_back() {
        let town = numinous_core::route_workbench::RouteTownSnapshot {
            junctions: 4,
            roads: vec![
                EditableRoad {
                    road: Road {
                        from: 0,
                        to: 1,
                        cost: 10,
                    },
                    open: true,
                },
                EditableRoad {
                    road: Road {
                        from: 0,
                        to: 2,
                        cost: 25,
                    },
                    open: true,
                },
                EditableRoad {
                    road: Road {
                        from: 1,
                        to: 2,
                        cost: 10,
                    },
                    open: true,
                },
                EditableRoad {
                    road: Road {
                        from: 2,
                        to: 3,
                        cost: 5,
                    },
                    open: true,
                },
            ],
            stops: vec![0, 1, 2],
            order: vec![0, 1, 2],
        };
        let mut panel = Panel::opened(RouteCreation::new(town).unwrap());
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Search));
        panel.act(Action::Next);
        panel.act(Action::Next);
        panel.act(Action::Search);
        step_until_cost(&mut panel, 2, 25);
        let first = panel.workbench.trace().unwrap().view();
        assert_eq!(first.junctions[2].predecessor, Some(0));
        assert!(panel.search_inspector(Some(&first)).contains("COST 25"));
        step_until_cost(&mut panel, 2, 20);
        let improved = panel.workbench.trace().unwrap().view();
        assert_eq!(improved.junctions[2].predecessor, Some(1));
        assert_eq!(
            improved.active_event,
            Some(RouteEvent::Relaxed {
                from: 1,
                to: 2,
                cost: 20
            })
        );
        panel.act(Action::Back);
        let restored = panel.workbench.trace().unwrap().view();
        assert_eq!(restored.junctions[2].cost, Some(25));
        assert_eq!(restored.junctions[2].predecessor, Some(0));
        panel.act(Action::Step);
        assert_eq!(panel.workbench.trace().unwrap().view(), improved);
        let image = panel.draw(360, 240, None);
        assert_text_pixels(&image, &panel.search_inspector(Some(&improved)), 6, 162, 1);
    }

    #[test]
    fn route_authoring_search_node_click_inspects_without_editing_or_advancing() {
        let mut panel = search_panel();
        panel.act(Action::Step);
        panel.act(Action::Step);
        let before = panel.workbench.snapshot();
        let locations = panel.network_locations();
        assert_eq!(panel.junction, 3);
        assert_eq!(panel.pointer_at(locations[1], false), Effect::None);
        assert_eq!(
            panel.junction, 3,
            "hover leaves inspection selection unchanged"
        );
        assert_eq!(panel.pointer_at(locations[1], true), Effect::None);
        assert_eq!(panel.junction, 1);
        let view = panel.workbench.trace().unwrap().view();
        assert!(
            panel
                .search_inspector(Some(&view))
                .contains("TENTATIVE COST 1")
        );
        assert_eq!(panel.workbench.snapshot(), before);
        panel.pointer_at(locations[3], true);
        assert_eq!(panel.junction, 3);
        panel.pointer_at((f64::NAN, f64::INFINITY), true);
        panel.pointer_at((0.01, 0.62), true);
        assert_eq!(panel.junction, 3);
        assert_eq!(panel.workbench.snapshot(), before);
        let mut paused = Panel::opened(RouteCreation::new(panel.workbench.town().clone()).unwrap());
        paused.page = Page::Search;
        paused.pointer_at(locations[1], true);
        assert_eq!(paused.junction, 0);
    }

    #[test]
    fn route_authoring_dense_search_keeps_every_background_road_quiet_and_latest_endpoint_visible()
    {
        let town = numinous_core::route_workbench::RouteTownSnapshot {
            junctions: 32,
            roads: (0..32)
                .flat_map(|from| ((from + 1)..32).map(move |to| (from, to)))
                .take(96)
                .map(|(from, to)| EditableRoad {
                    road: Road {
                        from,
                        to,
                        cost: ((from + to) % 9 + 1) as u32,
                    },
                    open: true,
                })
                .collect(),
            stops: vec![0, 4, 9, 15, 22, 31],
            order: vec![0, 31, 9, 22, 4, 15],
        };
        let mut panel = Panel::opened(RouteCreation::new(town).unwrap());
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Search));
        for _ in 0..31 {
            panel.act(Action::Next);
        }
        panel.act(Action::Search);
        for _ in 0..12 {
            panel.act(Action::Step);
        }
        let before = panel.workbench.snapshot();
        assert_eq!(
            panel.workbench.trace().unwrap().view().active_event,
            Some(RouteEvent::Relaxed {
                from: 0,
                to: 11,
                cost: 3
            })
        );
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let scale = ((width / 400).min(height / 240)).clamp(1, 3) as i32;
            let points = panel.network_points(width, height);
            let mut road_mask = Raster::with_accent(width, height, [240, 180, 110]);
            for road in &panel.workbench.town().roads {
                let (a, b) = (points[road.road.from], points[road.road.to]);
                road_mask.line(a.0, a.1, b.0, b.1, '.');
            }
            let blank = Raster::with_accent(width, height, [240, 180, 110]).to_rgba();
            let mask = road_mask.to_rgba();
            let actual = panel.draw(width, height, None);
            let rgba = actual.to_rgba();
            let tone = quiet_road_pixel();
            let mut road_pixels = 0;
            let mut quiet_pixels = 0;
            for ((actual, expected), blank) in rgba
                .chunks_exact(4)
                .zip(mask.chunks_exact(4))
                .zip(blank.chunks_exact(4))
            {
                if expected[..3] != blank[..3] {
                    road_pixels += 1;
                    assert!(
                        actual[..3].iter().zip(tone).all(|(a, tone)| *a >= tone),
                        "every actual road remains visible, including dense crossings"
                    );
                    quiet_pixels += usize::from(actual[..3] == tone);
                }
            }
            assert!(
                quiet_pixels * 2 > road_pixels,
                "background crossings must not dominate the revealed prefix"
            );
            let active_origin =
                search_label_origin(points[11], 4 * 6 * scale, 1, scale, width, height);
            assert_text_pixels(&actual, "11:3", active_origin.0, active_origin.1, scale);
            assert_text_pixels(
                &actual,
                &panel.search_inspector(Some(&panel.workbench.trace().unwrap().view())),
                6,
                (height as f64 * 0.675) as i32,
                scale,
            );
            assert!(
                pixel(
                    &actual,
                    (points[31].0 - 3 * scale) as usize,
                    points[31].1 as usize
                )
                .iter()
                .zip(marked_pixel('#'))
                .all(|(a, ink)| *a >= ink)
            );
            assert_eq!(panel.workbench.snapshot(), before);
        }
        panel.pointer_at(panel.network_locations()[12], true);
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let scale = ((width / 400).min(height / 240)).clamp(1, 3) as i32;
            let points = panel.network_points(width, height);
            let selected = search_label_origin(points[12], 3 * 6 * scale, 1, scale, width, height);
            let active = search_label_origin(points[11], 4 * 6 * scale, 1, scale, width, height);
            let overlapping = active.0 < selected.0 + 3 * 6 * scale + 2
                && active.0 + 4 * 6 * scale + 2 > selected.0
                && active.1 < selected.1 + 7 * scale + 2
                && active.1 + 7 * scale + 2 > selected.1;
            let actual = panel.draw(width, height, None);
            assert_text_pixels(&actual, ">12", selected.0, selected.1, scale);
            assert_text_pixels(
                &actual,
                "11:3",
                active.0,
                if overlapping {
                    selected.1 + 7 * scale + 3
                } else {
                    active.1
                },
                scale,
            );
            assert_eq!(panel.workbench.snapshot(), before);
        }
        panel.act(Action::Page(Page::Roads));
        panel.act(Action::ToggleRoad);
        panel.act(Action::Page(Page::Search));
        let points = panel.network_points(900, 700);
        let closed = panel.draw(900, 700, None);
        assert_text_pixels(
            &closed,
            "X",
            (points[0].0 + points[1].0) / 2,
            (points[0].1 + points[1].1) / 2,
            2,
        );
    }

    fn route_question_document(
        question: &str,
        next: Option<ProjectNext>,
    ) -> numinous_core::ProjectDocument {
        let parent = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut town = parent.town().clone();
        town.roads[3].road.cost = 7;
        let child = parent.remix(town).unwrap();
        let mut panel = Panel::opened(child);
        panel.question = question.into();
        let mut draft = panel.project_draft(42).unwrap();
        if let Some(next) = next {
            draft.next = next;
        }
        draft
            .evidence
            .push(numinous_core::ProjectEvidence::Journal {
                digest: [9; 32],
                entry_id: Some(7),
            });
        numinous_core::ProjectDocument::from_draft(&draft).unwrap()
    }

    #[test]
    fn route_question_received_preview_opens_embedded_network_without_running_next() {
        let other = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        for next in [
            ProjectNext::RemixRoute,
            ProjectNext::OpenRoute {
                capsule: other.to_capsule(),
            },
        ] {
            let document = route_question_document("Why is this road expensive?", Some(next));
            let canonical = document.to_document();
            let preview = document.preview(
                &numinous_core::Journal::default(),
                numinous_core::ReceiptCheck::NotSupplied,
            );
            let creation =
                RouteCreation::from_capsule(preview.creation.capsule.as_deref().unwrap()).unwrap();
            let mut panel = Panel::received(document).unwrap();
            assert!(panel.is_received_preview());
            assert_eq!(panel.question, preview.question);
            assert_eq!(panel.workbench.town(), creation.town());
            assert!(panel.workbench.trace().is_none());
            assert!(panel.workbench.snapshot().undo.is_empty());
            let initial = panel.workbench.snapshot();
            panel.act(Action::Cost(1));
            assert_eq!(panel.workbench.snapshot(), initial);
            assert_eq!(panel.act(Action::Keep), Effect::Keep);
            panel.mark_received_kept();
            assert!(
                panel
                    .buttons()
                    .iter()
                    .any(|button| button.label == "QUESTION KEPT")
            );
            assert!(panel.paused);
            panel.act(Action::Confirm);
            assert!(!panel.paused);
            assert_eq!(panel.received_document().unwrap().to_document(), canonical);
            assert_eq!(panel.workbench.town(), creation.town());
            panel.act(Action::Page(Page::Keep));
            panel.push_text(" More");
            assert!(panel.received_document().is_none());
            panel.act(Action::Reset);
            assert_eq!(panel.workbench.snapshot(), initial);
        }
    }

    #[test]
    fn route_question_received_document_tracks_network_and_question_without_hidden_lineage() {
        let document = route_question_document("Which road matters?", None);
        let mut panel = Panel::received(document.clone()).unwrap();
        panel.act(Action::Confirm);
        panel.act(Action::Cost(1));
        assert!(panel.received_document().is_none());
        panel.act(Action::Undo);
        assert_eq!(panel.received_document(), Some(&document));
        panel.act(Action::Page(Page::Search));
        panel.act(Action::Search);
        panel.act(Action::Step);
        assert_eq!(panel.received_document(), Some(&document));
        let imported = RouteCreation::from_capsule(
            panel
                .project_draft(10)
                .unwrap()
                .creation
                .as_deref()
                .unwrap(),
        )
        .unwrap();
        panel.act(Action::Remix);
        let remixed = RouteCreation::from_capsule(
            panel
                .project_draft(10)
                .unwrap()
                .creation
                .as_deref()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(remixed.parent_identity(), Some(imported.identity()));
        assert_ne!(remixed.parent_identity(), imported.parent_identity());
        assert!(panel.received_document().is_none());
    }

    #[test]
    fn route_question_received_requires_a_route_and_draws_full_question_above_controls() {
        let studio = numinous_core::StudioCreation::new("x", -1.0, 1.0, 0.0).unwrap();
        let capsule = studio.to_num_file();
        let draft = ProjectDraft {
            recorded_at_utc: 0,
            question: "A Studio question".into(),
            next: ProjectNext::OpenCreation {
                capsule: capsule.clone(),
            },
            rooms: vec!["mandelbrot".into()],
            evidence: vec![],
            creation: Some(capsule),
        };
        assert!(
            Panel::received(numinous_core::ProjectDocument::from_draft(&draft).unwrap()).is_err()
        );
        let question = "q".repeat(numinous_core::MAX_WORKSPACE_TEXT_CHARS);
        let panel = Panel::received(route_question_document(&question, None)).unwrap();
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let raster = panel.draw(width, height, None);
            let cache = panel.question_text.borrow();
            let text = cache.as_ref().unwrap();
            assert_eq!(text.source, format!("Question: {question}"));
            assert_eq!(text.layout.as_ref().unwrap().source(), text.source);
            assert!(text.layout.as_ref().unwrap().missing_glyphs().is_empty());
            drop(cache);
            let mut expected = Raster::with_accent(width, height, [240, 180, 110]);
            panel.draw_question(
                &mut expected,
                width,
                height,
                ((width / 400).min(height / 240)).clamp(1, 3) as i32,
            );
            let actual = raster.to_rgba();
            let expected = expected.to_rgba();
            for y in (height as f64 * 0.15) as usize..(height as f64 * 0.42) as usize {
                let row = y * width * 4;
                assert!(
                    actual[row..row + width * 4] == expected[row..row + width * 4],
                    "question band row {y} at {width}x{height}"
                );
            }
            for button in panel.buttons() {
                let (x, y, w, h) = button.bounds;
                let mut opened = Panel::received(route_question_document(&question, None)).unwrap();
                opened.draw(width, height, None);
                let effect = opened.pointer_at((x + w / 2.0, y + h / 2.0), true);
                assert_eq!(
                    effect,
                    match button.action {
                        Action::Keep => Effect::Keep,
                        Action::Close => Effect::Close,
                        _ => Effect::None,
                    }
                );
                assert_eq!(opened.paused, button.action != Action::Confirm);
            }
        }
    }

    #[test]
    fn route_question_share_and_browse_controls_preserve_current_search_and_lineage() {
        let mut panel =
            Panel::received(route_question_document("Keep this question", None)).unwrap();
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Search));
        panel.act(Action::Search);
        panel.act(Action::Step);
        let state = panel.workbench.snapshot();
        panel.act(Action::Page(Page::Keep));
        assert_eq!(panel.act(Action::Share), Effect::Share);
        panel.shared_path = Some("C:/example/question.project".into());
        for (width, height) in [(360, 240), (900, 700)] {
            panel.draw(width, height, None);
        }
        assert_eq!(panel.act(Action::Browse), Effect::Browse);
        assert_eq!(panel.workbench.snapshot(), state);
        assert_eq!(panel.thumbnail(200, 100).width(), 200);
        panel.push_text("?");
        assert!(panel.shared_path.is_none());
    }

    #[test]
    fn route_question_dense_preview_preserves_network_roles_and_counts_without_unused_id_labels() {
        let mut town = RouteWorkbench::first_town().town().clone();
        town.junctions = 32;
        let mut source = Panel::opened(RouteCreation::new(town).unwrap());
        source.question = "Inspect this network".into();
        let panel = Panel::received(
            numinous_core::ProjectDocument::from_draft(&source.project_draft(0).unwrap()).unwrap(),
        )
        .unwrap();
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let raster = panel.draw(width, height, None);
            let scale = ((width / 400).min(height / 240)).clamp(1, 3) as i32;
            assert_text_pixels(
                &raster,
                "32 JUNCTIONS / 5 ROADS / 4 STOPS",
                6,
                (height as f64 * 0.635) as i32,
                scale,
            );
            let points = panel.network_points(width, height);
            let rgba = raster.to_rgba();
            let pixel_at = |x: i32, y: i32| {
                let offset = (y as usize * width + x as usize) * 4;
                &rgba[offset..offset + 3]
            };
            let mark = marked_pixel('#');
            let radius = 2 * scale;
            for (x, y) in [
                (points[0].0 - radius, points[0].1 - radius),
                (points[0].0 + radius, points[0].1 + radius),
                (points[1].0 - radius, points[1].1),
                (points[1].0, points[1].1 + radius),
            ] {
                assert!(
                    pixel_at(x, y)
                        .iter()
                        .zip(mark)
                        .all(|(actual, expected)| *actual >= expected)
                );
            }
            let isolated = points[31];
            assert_eq!(pixel_at(isolated.0, isolated.1), &mark);
            let blank = Raster::with_accent(1, 1, [240, 180, 110]).to_rgba();
            for y in isolated.1..isolated.1 + 7 * scale {
                for x in isolated.0 + 1..isolated.0 + 11 * scale {
                    assert_eq!(pixel_at(x, y), &blank[..3]);
                }
            }
            assert_text_pixels(&raster, "0D", points[0].0 + radius + 2, points[0].1, scale);
        }
    }

    #[test]
    fn route_question_unicode_paging_and_resize_preserve_literal_source_and_canonical_network() {
        let question = "道路の配送順序".repeat(40);
        assert_eq!(question.chars().count(), 280);
        let document = route_question_document(&question, None);
        let canonical = document.to_document();
        let mut panel = Panel::received(document).unwrap();
        let initial = panel.workbench.snapshot();
        let first = panel.draw(360, 240, None).to_rgba();
        let layout = panel
            .question_text
            .borrow()
            .as_ref()
            .unwrap()
            .layout
            .clone()
            .unwrap();
        assert_eq!(layout.source(), format!("Question: {question}"));
        assert!(layout.missing_glyphs().is_empty());
        assert!(panel.question_text.borrow().as_ref().unwrap().maximum > 0.0);
        panel.draw(360, 240, None);
        assert!(std::sync::Arc::ptr_eq(
            &layout,
            panel
                .question_text
                .borrow()
                .as_ref()
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
        ));
        assert!(
            panel
                .buttons()
                .iter()
                .any(|button| button.action == Action::QuestionPage(1))
        );
        panel.act(Action::QuestionPage(i32::MAX));
        let last = panel.draw(360, 240, None).to_rgba();
        assert!(
            first != last,
            "the remaining question has a different rendered page"
        );
        {
            let cache = panel.question_text.borrow();
            let text = cache.as_ref().unwrap();
            assert_eq!(text.scroll, text.maximum);
            assert!(
                text.layout
                    .as_ref()
                    .unwrap()
                    .source_offset_at_scroll(text.scroll)
                    > 0
            );
        }
        for (width, height) in [(900, 700), (1600, 700), (1280, 400), (360, 240)] {
            panel.draw(width, height, None);
            let cache = panel.question_text.borrow();
            let text = cache.as_ref().unwrap();
            assert_eq!(
                text.layout.as_ref().unwrap().source(),
                format!("Question: {question}")
            );
            assert!((0.0..=text.maximum).contains(&text.scroll));
            assert_eq!(
                text.dimensions.0,
                width.saturating_sub(12).clamp(1, 4096) as u32
            );
        }
        panel.act(Action::QuestionPage(i32::MIN));
        assert_eq!(panel.question_text.borrow().as_ref().unwrap().scroll, 0.0);
        assert_eq!(panel.workbench.snapshot(), initial);
        assert_eq!(panel.received_document().unwrap().to_document(), canonical);
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Keep));
        panel.draw(360, 240, None);
        panel.act(Action::QuestionPage(1));
        assert!(panel.question_text.borrow().as_ref().unwrap().scroll > 0.0);
        assert_eq!(panel.question, question);
        assert_eq!(panel.received_document().unwrap().to_document(), canonical);
    }

    #[test]
    fn route_question_unicode_edit_counts_characters_and_preserves_combining_and_supplementary_data()
     {
        let question = format!("{}e\u{301}\u{10400}", "道".repeat(150));
        assert!(question.len() > numinous_core::MAX_WORKSPACE_TEXT_CHARS);
        let mut panel = Panel::received(route_question_document(&question, None)).unwrap();
        panel.act(Action::Confirm);
        panel.act(Action::Page(Page::Keep));
        panel.push_text(" ASCII");
        assert_eq!(panel.question, format!("{question} ASCII"));
        panel.push_text(&"x".repeat(300));
        assert_eq!(
            panel.question.chars().count(),
            numinous_core::MAX_WORKSPACE_TEXT_CHARS
        );
        assert!(panel.question.starts_with(&question));
        assert_eq!(panel.project_draft(0).unwrap().question, panel.question);
        panel.draw(360, 240, None);
        assert!(
            panel
                .question_text
                .borrow()
                .as_ref()
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .source()
                .starts_with("Some characters unavailable.")
        );
        assert!(panel.question.starts_with(&question));
        panel.act(Action::Backspace);
        assert_eq!(panel.question.chars().count(), 279);
    }

    #[test]
    fn route_question_text_adapter_refuses_oversized_public_draft_without_blank_success_or_domain_change()
     {
        let mut panel =
            Panel::received(route_question_document("Bounded admission", None)).unwrap();
        panel.question = "x".repeat(65_536);
        let before = panel.workbench.snapshot();
        let raster = panel.draw(360, 240, None);
        assert_text_pixels(&raster, "QUESTION TEXT COULD NOT BE DRAWN", 6, 36, 1);
        assert_eq!(panel.workbench.snapshot(), before);
        assert!(panel.received_document().is_none());
        panel.act(Action::QuestionPage(1));
        panel.question = "Recovered".into();
        panel.draw(360, 240, None);
        assert_eq!(
            panel
                .question_text
                .borrow()
                .as_ref()
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .source(),
            "Question: Recovered"
        );
    }
    #[test]
    fn route_question_shared_receipt_preserves_case_unicode_and_pages_inside_its_band() {
        let document = route_question_document("Preserve the route question", None);
        let path = format!(
            "/home/山田/MixedCase/{}TailCase.project",
            "ReadableFolder/".repeat(1_100)
        );
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let mut panel = Panel::received(document.clone()).unwrap();
            panel.act(Action::Confirm);
            panel.act(Action::Page(Page::Search));
            panel.act(Action::Next);
            panel.act(Action::Search);
            panel.act(Action::Step);
            panel.act(Action::Page(Page::Keep));
            let state = panel.workbench.snapshot();
            panel.shared_path = Some(path.clone());
            let first = panel.draw(width, height, None).to_rgba();
            let (maximum, page_height) = {
                let cache = panel.question_text.borrow();
                let text = cache.as_ref().unwrap();
                let layout = text.layout.as_ref().unwrap();
                assert_eq!(text.source, format!("Shared to: {path}"));
                assert_eq!(layout.source(), text.source);
                assert!(layout.missing_glyphs().is_empty());
                assert!(text.maximum > 0.0);
                (text.maximum, text.dimensions.1)
            };
            for button in panel
                .buttons()
                .iter()
                .filter(|button| matches!(button.action, Action::QuestionPage(_)))
            {
                assert!(button.label.starts_with("PATH "));
            }
            let pages = (maximum / page_height as f32).ceil() as usize + 1;
            for _ in 0..pages {
                panel.act(Action::QuestionPage(1));
            }
            let last = panel.draw(width, height, None).to_rgba();
            assert_eq!(
                panel.question_text.borrow().as_ref().unwrap().scroll,
                maximum
            );
            let band = (height as f64 * 0.29) as usize * width * 4
                ..((height as f64 * 0.29) as usize + (height as f64 * 0.24) as usize) * width * 4;
            assert!(
                first[band.clone()] != last[band.clone()],
                "path tail is reachable"
            );
            assert!(first[..band.start] == last[..band.start]);
            assert!(
                first[band.end..] == last[band.end..],
                "paging cannot cover controls"
            );
            for _ in 0..pages {
                panel.act(Action::QuestionPage(-1));
            }
            assert!(panel.draw(width, height, None).to_rgba() == first);
            for (altered, witness) in [
                (path.replace("山田", "道路"), "Unicode directory"),
                (path.replace("MixedCase", "MIXEDCASE"), "filename case"),
            ] {
                panel.shared_path = Some(altered);
                let changed = panel.draw(width, height, None).to_rgba();
                assert!(
                    first[band.clone()] != changed[band.clone()],
                    "{witness} has distinct visible ink"
                );
            }
            panel.shared_path = Some(path.clone());
            panel.draw(width, height, None);
            assert_eq!(panel.shared_path.as_deref(), Some(path.as_str()));
            assert_eq!(panel.workbench.snapshot(), state);
            assert_eq!(panel.received_document(), Some(&document));
        }
    }

    #[test]
    fn route_question_resize_cache_matches_fresh_render_at_scale_thresholds() {
        let document = route_question_document("Which road changes the result?", None);
        for (width, first_height, next_height) in [(900, 479, 480), (1200, 719, 720)] {
            let reused = Panel::received(document.clone()).unwrap();
            reused.draw(width, first_height, None);
            let before_dimensions = reused.question_text.borrow().as_ref().unwrap().dimensions;
            let resized = reused.draw(width, next_height, None).to_rgba();
            let after_dimensions = reused.question_text.borrow().as_ref().unwrap().dimensions;
            assert_eq!(
                before_dimensions, after_dimensions,
                "the fixture isolates a scale collision"
            );
            let fresh = Panel::received(document.clone())
                .unwrap()
                .draw(width, next_height, None)
                .to_rgba();
            assert!(
                resized == fresh,
                "resized question font matches a fresh {width}x{next_height} frame"
            );
            assert_eq!(reused.received_document(), Some(&document));
        }
    }
}
