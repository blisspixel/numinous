//! Window routing for the canonical route editor.

use super::{App, gamepad, input_legend, menu, project_resume, route_authoring};
use route_authoring::{Action, Effect, Page, Panel};
use winit::keyboard::{Key, NamedKey};

impl App {
    pub(super) fn open_route_authoring(&mut self) {
        if self.route_authoring.is_none() {
            let snapshot = numinous_core::rooms::route_lab::RouteLab::new_with(self.variation)
                .native_snapshot(&self.inputs);
            self.route_authoring = Some(Panel::new(
                numinous_core::route_workbench::RouteWorkbench::from_snapshot(snapshot)
                    .expect("native room exports validated state"),
            ));
        }
        self.activate_route_authoring();
    }

    fn activate_route_authoring(&mut self) {
        self.clear_pointer_state();
        self.close_menu();
        self.the_show = false;
        self.studio = false;
        self.paused = false;
        self.show_journey = false;
        self.route_active = true;
        self.route_primary_held = false;
        self.update_audio();
        if let Some(player) = &self.player {
            player.clear_parameter_voice();
        }
    }

    pub(super) fn open_route_creation(&mut self, creation: numinous_core::RouteCreation) {
        if !(self.route_active && self.project_resume.is_some()) {
            self.route_preview_previous = self.route_authoring.take();
        }
        self.route_authoring = Some(Panel::opened(creation));
        self.activate_route_authoring();
    }

    pub(super) fn close_route_authoring(&mut self) {
        self.route_active = false;
        self.route_primary_held = false;
        if self.project_resume.is_some() {
            self.route_authoring = self.route_preview_previous.take();
        }
        self.project_resume = None;
        self.close_menu();
        self.clear_pointer_state();
        self.update_audio();
    }

    fn route_effect(&mut self, effect: Effect) {
        match effect {
            Effect::None => {}
            Effect::Close => self.close_route_authoring(),
            Effect::Keep => self.keep_route_at(&super::local_state_paths().project),
        }
    }

    pub(super) fn keep_route_at(&mut self, path: &std::path::Path) {
        let Some(panel) = self.route_authoring.as_mut() else {
            return;
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let result = panel.project_draft(timestamp).and_then(|draft| {
            numinous_core::keep_project_file(path, &draft).map_err(|error| error.to_string())
        });
        panel.message = match result {
            Ok(_) => "QUESTION KEPT. CABINET: THE QUESTION REOPENS IT.".into(),
            Err(error) => format!("KEEP REFUSED: {}", numinous_core::display_safe(&error)),
        };
        self.refresh_kept_project_menu();
    }

    pub(super) fn handle_route_authoring_key(&mut self, key: &Key, repeat: bool) -> bool {
        if !self.route_active || self.show_help || self.study.is_some() || self.console.is_open() {
            return false;
        }
        self.input_mode = input_legend::InputMode::KeyboardMouse;
        if repeat {
            return true;
        }
        if self.project_resume.is_some() {
            match key {
                Key::Named(NamedKey::Enter) => self.confirm_kept_project(),
                Key::Named(NamedKey::Escape) => self.dismiss_kept_project(),
                _ => {}
            }
            return true;
        }
        let Some(panel) = self.route_authoring.as_mut() else {
            return false;
        };
        let effect = match key {
            Key::Named(NamedKey::Escape) => Effect::Close,
            Key::Named(NamedKey::Tab) => {
                panel.navigate(1);
                Effect::None
            }
            Key::Named(NamedKey::ArrowRight) => {
                panel.navigate_direction(1, 0);
                Effect::None
            }
            Key::Named(NamedKey::ArrowLeft) => {
                panel.navigate_direction(-1, 0);
                Effect::None
            }
            Key::Named(NamedKey::ArrowDown) => {
                panel.navigate_direction(0, 1);
                Effect::None
            }
            Key::Named(NamedKey::ArrowUp) => {
                panel.navigate_direction(0, -1);
                Effect::None
            }
            Key::Named(NamedKey::Enter) => panel.activate(),
            Key::Named(NamedKey::Backspace) if panel.page == Page::Keep => {
                panel.act(Action::Backspace)
            }
            Key::Named(NamedKey::Space) if panel.page == Page::Keep => {
                panel.push_text(" ");
                Effect::None
            }
            Key::Character(text) if panel.page == Page::Keep => {
                panel.push_text(text);
                Effect::None
            }
            Key::Character(text)
                if matches!(text.as_str(), "m" | "M" | "+" | "-" | "=" | "q" | "Q") =>
            {
                return false;
            }
            _ => Effect::None,
        };
        self.route_effect(effect);
        true
    }

    pub(super) fn handle_route_pointer(&mut self, point: (f64, f64), activate: bool) -> bool {
        if !self.route_active || self.show_help || self.study.is_some() {
            return false;
        }
        if self.project_resume.is_some() {
            // The preview's visible OPEN button is an explicit confirmation.
            let effect = self
                .route_authoring
                .as_mut()
                .map(|panel| panel.pointer_at(point, activate))
                .unwrap_or(Effect::None);
            if effect == Effect::Close {
                self.dismiss_kept_project();
            } else if self
                .route_authoring
                .as_ref()
                .is_some_and(|panel| !panel.paused)
            {
                self.project_resume = None;
                self.route_preview_previous = None;
            }
            return true;
        }
        let effect = self
            .route_authoring
            .as_mut()
            .map(|panel| panel.pointer_at(point, activate))
            .unwrap_or(Effect::None);
        self.route_effect(effect);
        true
    }

    pub(super) fn handle_route_gamepad(&mut self, command: gamepad::Command) -> bool {
        if !self.route_active || self.show_help || self.study.is_some() {
            return false;
        }
        self.input_mode = input_legend::InputMode::Controller;
        if command == gamepad::Command::Menu {
            self.open_activity_menu(menu::ActivityKind::Route);
            return true;
        }
        if command == gamepad::Command::Back {
            self.close_route_authoring();
            return true;
        }
        if matches!(
            command,
            gamepad::Command::PrimaryUp | gamepad::Command::CancelPointer
        ) {
            self.route_primary_held = false;
            return true;
        }
        if command == gamepad::Command::PrimaryDown {
            if self.route_primary_held {
                return true;
            }
            self.route_primary_held = true;
        }
        let Some(panel) = self.route_authoring.as_mut() else {
            return true;
        };
        let effect = match command {
            gamepad::Command::Left => {
                panel.navigate_direction(-1, 0);
                Effect::None
            }
            gamepad::Command::Right => {
                panel.navigate_direction(1, 0);
                Effect::None
            }
            gamepad::Command::Up => {
                panel.navigate_direction(0, -1);
                Effect::None
            }
            gamepad::Command::Down => {
                panel.navigate_direction(0, 1);
                Effect::None
            }
            gamepad::Command::PointerMoved { point, .. } => panel.pointer_at(point, false),
            gamepad::Command::PrimaryDown => panel.controller_activate(self.gamepad.cursor()),
            _ => Effect::None,
        };
        if self
            .project_resume
            .as_ref()
            .is_some_and(|plate| plate.creation_state == project_resume::CreationState::Present)
            && !panel.paused
        {
            self.project_resume = None;
            self.route_preview_previous = None;
        }
        self.route_effect(effect);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numinous_core::Surface;

    #[test]
    fn route_authoring_nested_previews_and_studio_entry_preserve_original_session() {
        let mut app = crate::tests::headless("route-editor-nested-preview");
        app.close_menu();
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "route-lab")
            .unwrap();
        app.open_route_authoring();
        let panel = app.route_authoring.as_mut().unwrap();
        panel.act(Action::Cost(1));
        panel.act(Action::Page(Page::Search));
        for _ in 0..3 {
            panel.act(Action::Next);
        }
        panel.act(Action::Search);
        panel.act(Action::Step);
        panel.question = "Current unsaved question".into();
        let original = panel.workbench().snapshot();
        let source = numinous_core::RouteCreation::new(
            numinous_core::route_workbench::RouteWorkbench::first_town()
                .town()
                .clone(),
        )
        .unwrap();
        let capsule = source.to_capsule();
        let project = app.journey_file.with_extension("project");
        let journal = app.journey_file.with_extension("journal");
        numinous_core::keep_project_file(
            &project,
            &numinous_core::ProjectDraft {
                recorded_at_utc: 1,
                question: "A different kept question".into(),
                next: numinous_core::ProjectNext::OpenRoute {
                    capsule: capsule.clone(),
                },
                rooms: vec!["route-lab".into()],
                evidence: vec![],
                creation: Some(capsule),
            },
        )
        .unwrap();
        let bytes = std::fs::read(&project).unwrap();
        app.open_kept_project_at(&project, &journal);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.open_kept_project_at(&project, &journal);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.dismiss_kept_project();
        assert!(!app.route_active);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            original
        );
        assert_eq!(
            app.route_authoring.as_ref().unwrap().question,
            "Current unsaved question"
        );
        assert!(app.route_preview_previous.is_none());
        app.open_kept_project_at(&project, &journal);
        let studio = numinous_core::first_studio_construction("lissajous")
            .unwrap()
            .creation();
        app.open_studio_creation(&studio);
        assert!(app.studio);
        assert!(!app.route_active);
        assert!(app.project_resume.is_none());
        assert!(app.route_preview_previous.is_none());
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            original
        );
        assert_eq!(
            app.route_authoring.as_ref().unwrap().question,
            "Current unsaved question"
        );
        assert_eq!(std::fs::read(&project).unwrap(), bytes);
        assert!(!journal.exists());
        let _ = std::fs::remove_file(project);
    }
    fn editor(name: &str) -> App {
        let mut app = crate::tests::headless(name);
        app.close_menu();
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "route-lab")
            .unwrap();
        app.open_route_authoring();
        app
    }

    fn button_point(app: &App, action: Action) -> (f64, f64) {
        let button = app
            .route_authoring
            .as_ref()
            .unwrap()
            .buttons()
            .into_iter()
            .find(|button| button.action == action)
            .unwrap();
        let (x, y, w, h) = button.bounds;
        (x + w / 2.0, y + h / 2.0)
    }

    fn keep_question(app: &mut App) -> (Vec<u8>, numinous_core::LocalStatePaths) {
        app.route_authoring
            .as_mut()
            .unwrap()
            .act(Action::Page(Page::Keep));
        assert!(app.handle_route_authoring_key(
            &Key::Character("How does this network change?".into()),
            false
        ));
        let point = button_point(app, Action::Keep);
        app.begin_pointer_at(point);
        app.end_pointer_at(point);
        let paths = crate::local_state_paths();
        (std::fs::read(&paths.project).unwrap(), paths)
    }

    #[test]
    fn route_authoring_pointer_preview_open_cancel_and_blank_space_preserve_files() {
        let mut app = editor("route-editor-pointer-preview");
        let (bytes, paths) = keep_question(&mut app);
        app.route_authoring.as_mut().unwrap().act(Action::Cost(1));
        let unsaved = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        app.open_kept_project_at(&paths.project, &paths.journal);
        let preview = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        app.begin_pointer_at((0.99, 0.75));
        app.move_pointer_to((0.99, 0.75), true);
        app.end_pointer_at((0.99, 0.75));
        assert!(app.project_resume.is_some());
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            preview
        );
        let leave = button_point(&app, Action::Close);
        app.begin_pointer_at(leave);
        app.end_pointer_at(leave);
        assert!(!app.route_active);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            unsaved
        );
        app.open_kept_project_at(&paths.project, &paths.journal);
        let open = button_point(&app, Action::Confirm);
        app.move_pointer_to(open, true);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.begin_pointer_at(open);
        app.end_pointer_at(open);
        assert!(app.route_active);
        assert!(!app.route_authoring.as_ref().unwrap().paused);
        assert!(app.project_resume.is_none());
        assert!(app.route_preview_previous.is_none());
        assert_eq!(std::fs::read(&paths.project).unwrap(), bytes);
        assert!(!paths.journal.exists());
        assert!(app.inputs.is_empty());
    }

    #[test]
    fn route_authoring_keyboard_navigation_and_preview_confirmation_own_the_activity() {
        let mut app = editor("route-editor-keyboard-navigation");
        let before = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        assert!(app.handle_route_authoring_key(&Key::Named(NamedKey::Tab), false));
        app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), false);
        assert_eq!(app.route_authoring.as_ref().unwrap().page, Page::Roads);
        for key in [
            NamedKey::ArrowRight,
            NamedKey::ArrowLeft,
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
        ] {
            assert!(app.handle_route_authoring_key(&Key::Named(key), false));
        }
        app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), false);
        assert_eq!(app.route_authoring.as_ref().unwrap().page, Page::Search);
        assert!(app.handle_route_authoring_key(&Key::Named(NamedKey::F7), false));
        assert!(!app.handle_route_authoring_key(&Key::Character("m".into()), false));
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            before
        );
        let (bytes, paths) = keep_question(&mut app);
        app.open_kept_project_at(&paths.project, &paths.journal);
        app.handle_route_authoring_key(&Key::Character("g".into()), false);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.handle_route_authoring_key(&Key::Named(NamedKey::Escape), false);
        assert!(!app.route_active);
        app.open_kept_project_at(&paths.project, &paths.journal);
        app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), false);
        assert!(app.project_resume.is_none());
        assert!(!app.route_authoring.as_ref().unwrap().paused);
        assert_eq!(std::fs::read(&paths.project).unwrap(), bytes);
    }

    #[test]
    fn route_authoring_controller_virtual_pointer_and_dpad_confirm_without_room_input() {
        let mut app = editor("route-editor-controller-pointer");
        let point = button_point(&app, Action::Page(Page::Roads));
        app.gamepad.set_cursor_for_test(point);
        app.handle_gamepad_command(gamepad::Command::PointerMoved { point, held: false });
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        assert_eq!(app.route_authoring.as_ref().unwrap().page, Page::Roads);
        let point = button_point(&app, Action::Cost(1));
        app.gamepad.set_cursor_for_test(point);
        app.handle_gamepad_command(gamepad::Command::PointerMoved { point, held: true });
        let before = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().revision(),
            before.revision + 1
        );
        app.route_authoring
            .as_mut()
            .unwrap()
            .act(Action::Page(Page::View));
        for command in [
            gamepad::Command::Right,
            gamepad::Command::Left,
            gamepad::Command::Up,
            gamepad::Command::Down,
        ] {
            app.handle_gamepad_command(command);
        }
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        assert_eq!(app.route_authoring.as_ref().unwrap().page, Page::Search);
        let (bytes, paths) = keep_question(&mut app);
        app.open_kept_project_at(&paths.project, &paths.journal);
        let point = button_point(&app, Action::Confirm);
        app.gamepad.set_cursor_for_test(point);
        app.handle_gamepad_command(gamepad::Command::PointerMoved { point, held: false });
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        assert!(app.project_resume.is_none());
        assert!(!app.route_authoring.as_ref().unwrap().paused);
        app.handle_gamepad_command(gamepad::Command::Back);
        assert!(!app.route_active);
        assert!(app.inputs.is_empty());
        assert_eq!(std::fs::read(&paths.project).unwrap(), bytes);
    }

    #[test]
    fn route_authoring_keep_refusals_preserve_editor_and_existing_project_bytes() {
        let mut app = editor("route-editor-keep-refusal");
        let paths = crate::local_state_paths();
        app.route_authoring
            .as_mut()
            .unwrap()
            .act(Action::Page(Page::Keep));
        let before = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        let point = button_point(&app, Action::Keep);
        app.begin_pointer_at(point);
        app.end_pointer_at(point);
        assert!(!paths.project.exists());
        assert!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .message
                .contains("KEEP REFUSED")
        );
        app.handle_route_authoring_key(&Key::Character("A deliberate question".into()), false);
        std::fs::write(&paths.project, b"unreadable project payload").unwrap();
        app.keep_route_at(&paths.project);
        assert_eq!(
            std::fs::read(&paths.project).unwrap(),
            b"unreadable project payload"
        );
        assert!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .message
                .contains("KEEP REFUSED")
        );
        assert_eq!(
            app.route_authoring.as_ref().unwrap().question,
            "A deliberate question"
        );
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            before
        );
        assert!(!paths.journal.exists());
        app.close_route_authoring();
        app.open_route_authoring();
        assert_eq!(
            app.route_authoring.as_ref().unwrap().question,
            "A deliberate question"
        );
        app.open_activity_menu(menu::ActivityKind::Route);
        assert!(!app.handle_route_pointer((0.5, 0.5), true));
        assert!(!app.handle_route_gamepad(gamepad::Command::PrimaryDown));
        app.close_menu();
        app.console.open();
        assert!(!app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), false));
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            before
        );
    }
    #[test]
    fn route_authoring_search_real_pointer_keyboard_controller_and_draw_do_not_autoplay() {
        use numinous_core::route_workbench::RouteSearchState;
        let mut app = crate::tests::headless("route-search-alpha29-native");
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "route-lab")
            .unwrap();
        app.apply_menu_intent(menu::MenuIntent::ConstructRoom);
        assert!(app.route_active);
        let initial_inputs = app.inputs.clone();
        let original_town = app
            .route_authoring
            .as_ref()
            .unwrap()
            .workbench()
            .town()
            .clone();
        let click = |app: &mut App, action: Action| {
            let point = button_point(app, action);
            app.begin_pointer_at(point);
            app.end_pointer_at(point);
        };
        click(&mut app, Action::Page(Page::Search));
        for _ in 0..3 {
            click(&mut app, Action::Next);
        }
        click(&mut app, Action::Search);
        let initial = app
            .route_authoring
            .as_ref()
            .unwrap()
            .workbench()
            .trace()
            .unwrap()
            .view();
        assert_eq!(initial.cursor, 0);
        assert_eq!((initial.from, initial.to), (0, 3));
        let initial_map = app.modal_frame(360, 240).unwrap().to_rgba();
        let step = button_point(&app, Action::Step);
        app.move_pointer_to(step, true);
        assert_eq!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .workbench()
                .trace()
                .unwrap()
                .cursor(),
            0
        );
        let index = app
            .route_authoring
            .as_ref()
            .unwrap()
            .buttons()
            .iter()
            .position(|button| button.action == Action::Step)
            .unwrap();
        let panel = app.route_authoring.as_mut().unwrap();
        panel.act(Action::Page(Page::Search));
        panel.navigate(index as i32);
        app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), false);
        app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), true);
        assert_eq!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .workbench()
                .trace()
                .unwrap()
                .cursor(),
            1
        );
        app.gamepad.set_cursor_for_test(step);
        app.handle_gamepad_command(gamepad::Command::PointerMoved {
            point: step,
            held: true,
        });
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        let partial = app
            .route_authoring
            .as_ref()
            .unwrap()
            .workbench()
            .trace()
            .unwrap()
            .view();
        assert_eq!(partial.cursor, 2);
        assert_eq!(partial.junctions[1].state, RouteSearchState::Tentative);
        assert_ne!(app.modal_frame(360, 240).unwrap().to_rgba(), initial_map);
        app.handle_gamepad_command(gamepad::Command::Menu);
        assert!(app.show_help);
        app.begin_pointer_at(step);
        app.end_pointer_at(step);
        assert_eq!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .workbench()
                .trace()
                .unwrap()
                .view(),
            partial
        );
        app.handle_gamepad_command(gamepad::Command::Back);
        assert!(!app.show_help);
        for (width, height) in [(360, 240), (900, 700), (1600, 700), (1280, 400)] {
            let raster = app.modal_frame(width, height).unwrap();
            assert_eq!((raster.width(), raster.height()), (width, height));
            assert_eq!(
                app.route_authoring
                    .as_ref()
                    .unwrap()
                    .workbench()
                    .trace()
                    .unwrap()
                    .view(),
                partial
            );
        }
        let back = button_point(&app, Action::Back);
        app.gamepad.set_cursor_for_test(back);
        app.handle_gamepad_command(gamepad::Command::PointerMoved {
            point: back,
            held: false,
        });
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        assert_eq!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .workbench()
                .trace()
                .unwrap()
                .view()
                .junctions[1]
                .state,
            RouteSearchState::Unseen
        );
        click(&mut app, Action::Page(Page::Roads));
        click(&mut app, Action::Cost(1));
        click(&mut app, Action::Page(Page::Search));
        assert!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .workbench()
                .trace()
                .is_none()
        );
        app.modal_frame(360, 240).unwrap();
        assert_eq!(app.inputs, initial_inputs);
        assert_ne!(
            app.route_authoring.as_ref().unwrap().workbench().town(),
            &original_town
        );
        app.close_route_authoring();
        app.open_route_authoring();
        assert!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .workbench()
                .trace()
                .is_none()
        );
    }
}
