//! Window routing for the canonical route editor.

use super::{App, gamepad, input_legend, menu, project_resume, route_authoring};
use route_authoring::{Action, Effect, Page, Panel};
use winit::keyboard::{Key, NamedKey};

pub(super) struct RoutePreviewReturn {
    panel: Option<Panel>,
    legacy_previous: Option<Panel>,
    plate: Option<project_resume::Plate>,
    gallery: Option<super::gallery::GalleryPanel>,
    active: bool,
    studio: bool,
    the_show: bool,
    paused: bool,
    show_journey: bool,
}

fn route_share_directory() -> std::path::PathBuf {
    #[cfg(test)]
    {
        super::local_state_paths()
            .project
            .parent()
            .unwrap()
            .to_path_buf()
    }
    #[cfg(not(test))]
    {
        super::postcard::default_postcard_dir()
    }
}

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
        self.discard_received_route_previews();
        self.gallery = None;
        self.share_naming = None;
        if !(self.route_active && self.project_resume.is_some()) {
            self.route_preview_previous = self.route_authoring.take();
        }
        self.route_authoring = Some(Panel::opened(creation));
        self.activate_route_authoring();
    }

    pub(super) fn close_route_authoring(&mut self) {
        if self.cancel_received_route_preview() {
            return;
        }
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
            Effect::Share => {
                let result = self.share_route_to(&route_share_directory());
                if let Some(panel) = self.route_authoring.as_mut() {
                    match result {
                        Ok(path) => {
                            panel.shared_path = Some(path.display().to_string());
                            panel.message = "QUESTION SHARED. GALLERY OPENS IT PAUSED.".into();
                        }
                        Err(error) => {
                            panel.message =
                                format!("SHARE REFUSED: {}", numinous_core::display_safe(&error))
                        }
                    }
                }
            }
            Effect::Browse => {
                self.gallery = Some(super::gallery::GalleryPanel::open(&route_share_directory()));
            }
        }
    }

    pub(super) fn share_route_to(
        &self,
        directory: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let panel = self
            .route_authoring
            .as_ref()
            .ok_or("No route question is open")?;
        let document = match panel.received_document() {
            Some(document) => document.clone(),
            None => numinous_core::ProjectDocument::from_draft(&panel.project_draft(0)?)
                .map_err(|error| error.to_string())?,
        };
        for suffix in 0..1000 {
            let path = directory.join(format!(
                "numinous-route-question-{}-{suffix:03}.project",
                document.identity_hex()
            ));
            match std::fs::symlink_metadata(&path) {
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
            match numinous_core::export_project_document_file(&path, &document) {
                Ok(numinous_core::ProjectDocumentExport::Created) => return Ok(path),
                Ok(numinous_core::ProjectDocumentExport::AlreadyPresent) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("All share filenames for this question are already occupied".into())
    }

    pub(super) fn open_route_project_document(
        &mut self,
        document: numinous_core::ProjectDocument,
    ) -> Result<(), String> {
        let incoming = Panel::received(document)?;
        if self.route_received_return.len() >= 8 {
            return Err("Close an existing route preview before opening another".into());
        }
        let primary_held = self.route_primary_held;
        let pointer_held = self.route_pointer_held || self.poking || self.dragging;
        self.route_received_return.push(RoutePreviewReturn {
            panel: self.route_authoring.take(),
            legacy_previous: self.route_preview_previous.take(),
            plate: self.project_resume.take(),
            gallery: self.gallery.take(),
            active: self.route_active,
            studio: self.studio,
            the_show: self.the_show,
            paused: self.paused,
            show_journey: self.show_journey,
        });
        self.project_resume = None;
        self.route_authoring = Some(incoming);
        self.activate_route_authoring();
        self.route_primary_held = primary_held;
        self.route_pointer_held = pointer_held;
        Ok(())
    }

    pub(super) fn cancel_received_route_preview(&mut self) -> bool {
        let Some(previous) = self.route_received_return.pop() else {
            return false;
        };
        self.route_authoring = previous.panel;
        self.route_preview_previous = previous.legacy_previous;
        self.route_active = previous.active;
        self.project_resume = previous.plate;
        self.gallery = previous.gallery;
        self.the_show = previous.the_show;
        self.paused = previous.paused;
        self.show_journey = previous.show_journey;
        let (primary_held, pointer_held) = (self.route_primary_held, self.route_pointer_held);
        self.clear_pointer_state();
        self.close_menu();
        self.route_primary_held = primary_held;
        self.route_pointer_held = pointer_held;
        self.studio = previous.studio;
        if self.studio {
            self.set_studio_sound(self.studio_panel.entry_sound());
        } else {
            self.update_audio();
        }
        true
    }

    pub(super) fn discard_received_route_previews(&mut self) {
        while self.cancel_received_route_preview() {}
    }

    fn confirm_received_route(&mut self) {
        if !self.route_received_return.is_empty()
            && self
                .route_authoring
                .as_ref()
                .is_some_and(|panel| !panel.paused)
        {
            self.route_received_return.clear();
            self.route_preview_previous = None;
        }
    }

    pub(super) fn keep_route_at(&mut self, path: &std::path::Path) {
        let Some(panel) = self.route_authoring.as_mut() else {
            return;
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let admitted = panel.received_document().cloned();
        let result = if let Some(document) = &admitted {
            numinous_core::import_project_file(path, &document.to_document(), timestamp, None, true)
                .map_err(|error| error.to_string())
        } else {
            panel.project_draft(timestamp).and_then(|draft| {
                numinous_core::keep_project_file(path, &draft).map_err(|error| error.to_string())
            })
        };
        let imported = admitted.is_some() && result.is_ok();
        panel.message = match result {
            Ok(_) => "QUESTION KEPT. CABINET: THE QUESTION REOPENS IT.".into(),
            Err(error) => format!("KEEP REFUSED: {}", numinous_core::display_safe(&error)),
        };
        if imported {
            panel.mark_received_kept();
        }
        self.refresh_kept_project_menu();
    }

    pub(super) fn handle_route_authoring_key(&mut self, key: &Key, repeat: bool) -> bool {
        if !self.route_active
            || self.show_help
            || self.study.is_some()
            || self.console.is_open()
            || self.gallery.is_some()
        {
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
            Key::Named(NamedKey::PageUp)
                if panel.is_received_preview() || panel.page == Page::Keep =>
            {
                panel.act(Action::QuestionPage(-1))
            }
            Key::Named(NamedKey::PageDown)
                if panel.is_received_preview() || panel.page == Page::Keep =>
            {
                panel.act(Action::QuestionPage(1))
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
        self.confirm_received_route();
        true
    }

    pub(super) fn handle_route_pointer(&mut self, point: (f64, f64), activate: bool) -> bool {
        if !self.route_active
            || self.show_help
            || self.study.is_some()
            || self.console.is_open()
            || self.gallery.is_some()
        {
            return false;
        }
        if activate {
            if self.route_pointer_held {
                return true;
            }
            self.route_pointer_held = true;
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
        self.confirm_received_route();
        true
    }

    pub(super) fn handle_route_gamepad(&mut self, command: gamepad::Command) -> bool {
        if !self.route_active
            || self.show_help
            || self.study.is_some()
            || self.console.is_open()
            || self.gallery.is_some()
        {
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
        self.confirm_received_route();
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

    fn portable(question: &str) -> numinous_core::ProjectDocument {
        let creation = numinous_core::RouteCreation::new(
            numinous_core::route_workbench::RouteWorkbench::first_town()
                .town()
                .clone(),
        )
        .unwrap();
        let mut panel = Panel::opened(creation);
        panel.question = question.into();
        numinous_core::ProjectDocument::from_draft(&panel.project_draft(10).unwrap()).unwrap()
    }

    fn pointer_action(app: &mut App, action: Action) {
        let point = button_point(app, action);
        app.begin_pointer_at(point);
        app.end_pointer_at(point);
    }

    #[test]
    fn route_question_share_is_immutable_fresh_and_does_not_keep_or_export_transient_search() {
        let mut app = editor("route-question-share");
        let directory = app.journey_file.parent().unwrap().join("shared-questions");
        std::fs::create_dir(&directory).unwrap();
        let inputs = app.inputs.clone();
        let paths = crate::local_state_paths();
        assert!(app.share_route_to(&directory).is_err());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        let panel = app.route_authoring.as_mut().unwrap();
        panel.question = "Which road changes the answer?".into();
        panel.act(Action::Cost(2));
        panel.act(Action::Page(Page::Search));
        panel.act(Action::Next);
        panel.act(Action::Search);
        panel.act(Action::Step);
        let state = panel.workbench().snapshot();
        let one = app.share_route_to(&directory).unwrap();
        let bytes = std::fs::read(&one).unwrap();
        let modified = std::fs::metadata(&one).unwrap().modified().unwrap();
        let two = app.share_route_to(&directory).unwrap();
        assert_ne!(one, two);
        assert_eq!(std::fs::read(&two).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&one).unwrap().modified().unwrap(),
            modified
        );
        let doc = numinous_core::read_project_document_file(&one).unwrap();
        let received = Panel::received(doc).unwrap();
        assert_eq!(received.workbench().town(), &state.current);
        assert!(received.workbench().snapshot().undo.is_empty());
        assert!(received.workbench().trace().is_none());
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            state
        );
        assert!(!paths.project.exists());
        assert!(!paths.journal.exists());
        assert_eq!(app.inputs, inputs);
        pointer_action(&mut app, Action::Page(Page::Keep));
        pointer_action(&mut app, Action::Share);
        assert!(
            std::path::Path::new(
                app.route_authoring
                    .as_ref()
                    .unwrap()
                    .shared_path
                    .as_deref()
                    .unwrap()
            )
            .exists()
        );
        assert!(!paths.project.exists());
        pointer_action(&mut app, Action::Browse);
        assert!(app.gallery.is_some());
    }

    #[test]
    fn route_question_share_skips_invalid_oversized_directory_and_matching_collisions() {
        let mut app = editor("route-question-collisions");
        let directory = app.journey_file.parent().unwrap().join("share-collisions");
        std::fs::create_dir(&directory).unwrap();
        let document = portable("Occupied names must survive");
        app.open_route_project_document(document.clone()).unwrap();
        let candidate = |suffix| {
            directory.join(format!(
                "numinous-route-question-{}-{suffix:03}.project",
                document.identity_hex()
            ))
        };
        std::fs::write(candidate(0), [0xff]).unwrap();
        let oversized = std::fs::File::create(candidate(1)).unwrap();
        oversized
            .set_len(numinous_core::MAX_PROJECT_FILE_BYTES + 1)
            .unwrap();
        std::fs::create_dir(candidate(2)).unwrap();
        std::fs::write(candidate(3), document.to_document()).unwrap();
        std::fs::write(candidate(4), "different bytes").unwrap();
        let shared = app.share_route_to(&directory).unwrap();
        assert_eq!(shared, candidate(5));
        assert_eq!(std::fs::read(candidate(0)).unwrap(), [0xff]);
        assert_eq!(
            std::fs::metadata(candidate(1)).unwrap().len(),
            numinous_core::MAX_PROJECT_FILE_BYTES + 1
        );
        assert!(candidate(2).is_dir());
        assert_eq!(
            std::fs::read_to_string(candidate(3)).unwrap(),
            document.to_document()
        );
        assert_eq!(
            std::fs::read_to_string(candidate(4)).unwrap(),
            "different bytes"
        );
        assert_eq!(
            numinous_core::read_project_document_file(&shared).unwrap(),
            document
        );
        let blocked = directory.join("blocked");
        std::fs::write(&blocked, "occupied parent").unwrap();
        assert!(app.share_route_to(&blocked).is_err());
        for suffix in 6..1000 {
            std::fs::write(candidate(suffix), []).unwrap();
        }
        assert!(
            app.share_route_to(&directory)
                .unwrap_err()
                .contains("occupied")
        );
    }

    #[test]
    fn route_question_receive_cancel_restores_nested_editor_and_kept_preview_exactly() {
        let mut app = editor("route-question-nested");
        app.route_authoring.as_mut().unwrap().question = "Unsaved original".into();
        let panel = app.route_authoring.as_mut().unwrap();
        panel.act(Action::Cost(2));
        panel.act(Action::Page(Page::Search));
        panel.act(Action::Next);
        panel.act(Action::Search);
        panel.act(Action::Step);
        let before = panel.workbench().snapshot();
        let original_capsule = panel.project_draft(0).unwrap().creation;
        let a = portable("Incoming A");
        let b = portable("Incoming B");
        app.open_route_project_document(a.clone()).unwrap();
        app.open_route_project_document(b.clone()).unwrap();
        assert_eq!(app.route_received_return.len(), 2);
        app.handle_route_authoring_key(&Key::Named(NamedKey::Escape), false);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().received_document(),
            Some(&a)
        );
        assert!(app.route_authoring.as_ref().unwrap().paused);
        pointer_action(&mut app, Action::Close);
        let restored = app.route_authoring.as_ref().unwrap();
        assert_eq!(restored.workbench().snapshot(), before);
        assert_eq!(restored.question, "Unsaved original");
        assert_eq!(
            restored.project_draft(0).unwrap().creation,
            original_capsule
        );
        assert!(app.route_active);
        assert!(app.route_received_return.is_empty());
        app.keep_route_at(&crate::local_state_paths().project);
        app.open_kept_project();
        let kept = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        let plate_lines = app.project_resume.as_ref().unwrap().lines.clone();
        app.open_route_project_document(b).unwrap();
        app.handle_gamepad_command(gamepad::Command::Back);
        assert_eq!(app.project_resume.as_ref().unwrap().lines, plate_lines);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            kept
        );
        assert!(app.route_authoring.as_ref().unwrap().paused);
    }

    #[test]
    fn route_question_admission_is_bounded_and_failures_preserve_current_preview() {
        let mut app = editor("route-question-admission");
        let path = app.journey_file.with_extension("project");
        let document = portable("Stable incoming question");
        numinous_core::export_project_document_file(&path, &document).unwrap();
        app.open_dropped_file(&path);
        let before = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        for bytes in [
            vec![0xff],
            b"NUMINOUS_PROJECT 99\n".to_vec(),
            vec![b'x'; numinous_core::MAX_PROJECT_FILE_BYTES as usize + 1],
        ] {
            std::fs::write(&path, bytes).unwrap();
            app.open_dropped_file(&path);
            assert_eq!(
                app.route_authoring.as_ref().unwrap().received_document(),
                Some(&document)
            );
            assert_eq!(
                app.route_authoring.as_ref().unwrap().workbench().snapshot(),
                before
            );
            assert_eq!(app.route_received_return.len(), 1);
            assert!(app.banner.as_ref().unwrap().lines()[0].starts_with("PROJECT REFUSED"));
        }
        app.open_start_input(path.with_file_name("missing.project").to_str().unwrap());
        assert_eq!(app.route_received_return.len(), 1);
        for _ in 1..8 {
            app.open_route_project_document(document.clone()).unwrap();
        }
        assert!(app.open_route_project_document(document).is_err());
        assert_eq!(app.route_received_return.len(), 8);
        app.discard_received_route_previews();
        assert!(app.route_active);
        assert!(!app.route_authoring.as_ref().unwrap().paused);
    }

    #[test]
    fn route_question_keep_uses_admitted_bytes_and_preserves_original_next_without_execution() {
        let mut app = editor("route-question-import");
        let mut draft = Panel::received(portable("Retain original next"))
            .unwrap()
            .project_draft(17)
            .unwrap();
        draft.next = numinous_core::ProjectNext::RemixRoute;
        draft
            .evidence
            .push(numinous_core::ProjectEvidence::Journal {
                digest: [8; 32],
                entry_id: Some(3),
            });
        let document = numinous_core::ProjectDocument::from_draft(&draft).unwrap();
        let input = app.journey_file.with_extension("project");
        let chain = app.journey_file.with_extension("chain");
        numinous_core::export_project_document_file(&input, &document).unwrap();
        app.open_dropped_file(&input);
        std::fs::write(&input, "source replaced after admission").unwrap();
        assert!(!chain.exists());
        app.keep_route_at(&chain);
        let first = std::fs::read(&chain).unwrap();
        let modified = std::fs::metadata(&chain).unwrap().modified().unwrap();
        let imported = numinous_core::try_load_project_file(&chain).unwrap();
        let preview = imported
            .preview(
                None,
                &numinous_core::Journal::default(),
                numinous_core::ReceiptCheck::NotSupplied,
            )
            .unwrap();
        assert_eq!(
            preview.next,
            document
                .preview(
                    &numinous_core::Journal::default(),
                    numinous_core::ReceiptCheck::NotSupplied
                )
                .next
        );
        assert_eq!(preview.question, draft.question);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        assert!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .workbench()
                .snapshot()
                .undo
                .is_empty()
        );
        app.keep_route_at(&chain);
        assert_eq!(std::fs::read(&chain).unwrap(), first);
        assert_eq!(
            std::fs::metadata(&chain).unwrap().modified().unwrap(),
            modified
        );
        assert_eq!(
            numinous_core::try_load_project_file(&chain)
                .unwrap()
                .revisions()
                .len(),
            1
        );
        let forwarding = app.share_route_to(chain.parent().unwrap()).unwrap();
        assert_eq!(
            numinous_core::read_project_document_file(&forwarding).unwrap(),
            document
        );
        let bad_chain = app.journey_file.with_extension("bad-chain");
        std::fs::write(&bad_chain, "unreadable chain").unwrap();
        app.keep_route_at(&bad_chain);
        assert_eq!(
            std::fs::read_to_string(&bad_chain).unwrap(),
            "unreadable chain"
        );
        assert!(
            app.route_authoring
                .as_ref()
                .unwrap()
                .message
                .starts_with("KEEP REFUSED")
        );
        pointer_action(&mut app, Action::Confirm);
        assert!(app.route_received_return.is_empty());
        assert!(!app.route_authoring.as_ref().unwrap().paused);
        assert_eq!(std::fs::read(&chain).unwrap(), first);
    }

    #[test]
    fn route_question_held_input_cannot_implicitly_open_and_cancel_preserves_studio_flags() {
        let mut app = editor("route-question-held");
        app.open_start_input("full-return");
        app.paused = true;
        app.show_journey = true;
        app.the_show = true;
        let source = app.studio_panel.current_creation().unwrap().to_num_file();
        let sound = app.studio_panel.entry_sound();
        app.route_primary_held = true;
        app.route_pointer_held = true;
        app.open_route_project_document(portable("Explicit open only"))
            .unwrap();
        app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), true);
        let open = button_point(&app, Action::Confirm);
        app.begin_pointer_at(open);
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.end_pointer_at(open);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        pointer_action(&mut app, Action::Close);
        assert!(app.studio);
        assert!(app.paused);
        assert!(app.show_journey);
        assert!(app.the_show);
        assert_eq!(
            app.studio_panel.current_creation().unwrap().to_num_file(),
            source
        );
        assert_eq!(app.studio_panel.entry_sound(), sound);
        app.open_route_project_document(portable("Replace deliberately"))
            .unwrap();
        app.open_start_input("another-ratio");
        assert!(app.route_received_return.is_empty());
        assert!(app.studio);
        assert!(
            app.route_authoring
                .as_ref()
                .is_some_and(|panel| panel.received_document().is_none())
        );
    }

    #[test]
    fn route_question_drop_refuses_study_console_and_help_without_mutation() {
        let mut app = crate::tests::headless("route-question-modal-refusal");
        app.close_menu();
        let path = app.journey_file.with_extension("project");
        numinous_core::export_project_document_file(&path, &portable("Wait for the panel"))
            .unwrap();
        assert!(app.open_room_study());
        app.open_dropped_file(&path);
        assert!(app.study.is_some());
        assert!(app.route_authoring.is_none());
        app.close_room_study();
        app.console.open();
        app.open_dropped_file(&path);
        assert!(app.console.is_open());
        assert!(app.route_authoring.is_none());
        app.console.close();
        app.open_home_menu();
        app.open_dropped_file(&path);
        assert!(app.show_help);
        assert!(app.route_authoring.is_none());
        app.open_start_input(path.to_str().unwrap());
        assert!(app.route_active);
        assert!(!app.show_help);
        assert!(app.route_authoring.as_ref().unwrap().paused);
    }

    #[test]
    fn route_question_gallery_owns_all_inputs_and_cancel_restores_wall_and_editor() {
        let mut app = editor("route-question-gallery");
        let directory = app.journey_file.parent().unwrap().join("gallery");
        std::fs::create_dir(&directory).unwrap();
        let document = portable("Preview from Gallery");
        numinous_core::export_project_document_file(&directory.join("question.project"), &document)
            .unwrap();
        app.gallery = Some(crate::gallery::GalleryPanel::open(&directory));
        let before = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        assert!(app.handle_gallery_key(&Key::Named(NamedKey::ArrowRight), false));
        assert!(!app.handle_route_authoring_key(&Key::Named(NamedKey::Enter), false));
        assert!(app.handle_gallery_key(&Key::Character("f".into()), false));
        assert!(app.gallery.is_some());
        assert!(app.handle_gallery_key(&Key::Character("d".into()), false));
        assert!(app.handle_gallery_key(&Key::Named(NamedKey::Enter), true));
        assert!(app.gallery.is_some());
        let wall = app.modal_frame(900, 700).unwrap().to_rgba();
        app.handle_gallery_key(&Key::Named(NamedKey::Enter), false);
        assert!(app.gallery.is_none());
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.handle_gamepad_command(gamepad::Command::Back);
        app.input_mode = input_legend::InputMode::KeyboardMouse;
        assert!(
            app.modal_frame(900, 700).unwrap().to_rgba() == wall,
            "cancel restores the same Gallery at the same input mode"
        );
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            before
        );
        app.handle_gamepad_command(gamepad::Command::Right);
        app.handle_gamepad_command(gamepad::Command::Menu);
        assert!(app.show_help);
        app.handle_gamepad_command(gamepad::Command::Back);
        app.begin_pointer_at((0.1, 0.5));
        assert!(app.route_authoring.as_ref().unwrap().paused);
        let open = button_point(&app, Action::Confirm);
        app.begin_pointer_at(open);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.end_pointer_at(open);
        app.handle_gamepad_command(gamepad::Command::Back);
        assert!(app.gallery.is_some());
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        assert!(app.gallery.is_none());
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        assert!(app.route_authoring.as_ref().unwrap().paused);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        app.handle_gamepad_command(gamepad::Command::Back);
        app.handle_gallery_key(&Key::Named(NamedKey::Escape), false);
        assert!(app.gallery.is_none());
        assert!(app.route_active);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            before
        );
    }

    #[test]
    fn route_question_kept_route_replacement_closes_restored_gallery_and_retires_incoming_document()
    {
        let mut app = editor("route-question-replace-wall");
        let directory = app
            .journey_file
            .parent()
            .unwrap()
            .join("replacement-gallery");
        std::fs::create_dir(&directory).unwrap();
        let document = portable("Retire this received question");
        numinous_core::export_project_document_file(&directory.join("question.project"), &document)
            .unwrap();
        app.gallery = Some(crate::gallery::GalleryPanel::open(&directory));
        app.gallery_open_selected();
        assert!(app.route_received_return.len() == 1);
        let mut replacement = numinous_core::route_workbench::RouteWorkbench::first_town()
            .town()
            .clone();
        replacement.roads[0].road.cost = 9;
        app.open_route_creation(numinous_core::RouteCreation::new(replacement.clone()).unwrap());
        assert!(app.route_received_return.is_empty());
        assert!(app.gallery.is_none());
        let panel = app.route_authoring.as_ref().unwrap();
        assert!(panel.paused);
        assert!(panel.received_document().is_none());
        assert_eq!(panel.workbench().town(), &replacement);
    }
    #[test]
    fn route_question_shared_receipt_pages_through_keyboard_pointer_and_controller_without_mutation()
     {
        let mut app = editor("route-question-receipt-input");
        let document = portable("Forward this question unchanged");
        app.open_route_project_document(document.clone()).unwrap();
        pointer_action(&mut app, Action::Confirm);
        pointer_action(&mut app, Action::Page(Page::Search));
        pointer_action(&mut app, Action::Next);
        pointer_action(&mut app, Action::Search);
        pointer_action(&mut app, Action::Step);
        pointer_action(&mut app, Action::Page(Page::Keep));
        let path = format!(
            "/home/山田/MixedCase/{}TailCase.project",
            "Folder/".repeat(140)
        );
        app.route_authoring.as_mut().unwrap().shared_path = Some(path.clone());
        let state = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        let first = app.modal_frame(360, 240).unwrap().to_rgba();
        assert!(app.handle_route_authoring_key(&Key::Named(NamedKey::PageDown), false));
        let next = app.modal_frame(360, 240).unwrap().to_rgba();
        let band = 69 * 360 * 4..126 * 360 * 4;
        assert!(first[band.clone()] != next[band.clone()]);
        assert!(app.handle_route_authoring_key(&Key::Named(NamedKey::PageDown), true));
        assert!(
            app.modal_frame(360, 240).unwrap().to_rgba() == next,
            "held keys do not page twice"
        );
        app.handle_route_authoring_key(&Key::Named(NamedKey::PageUp), false);
        assert!(app.modal_frame(360, 240).unwrap().to_rgba() == first);
        pointer_action(&mut app, Action::QuestionPage(1));
        let pointed = app.modal_frame(360, 240).unwrap().to_rgba();
        assert!(pointed[band.clone()] == next[band.clone()]);
        let previous = button_point(&app, Action::QuestionPage(-1));
        app.gamepad.set_cursor_for_test(previous);
        app.handle_gamepad_command(gamepad::Command::PointerMoved {
            point: previous,
            held: false,
        });
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        let back = app.modal_frame(360, 240).unwrap().to_rgba();
        assert!(back[band.clone()] == first[band]);
        let panel = app.route_authoring.as_ref().unwrap();
        assert_eq!(panel.shared_path.as_deref(), Some(path.as_str()));
        assert_eq!(panel.workbench().snapshot(), state);
        assert_eq!(panel.received_document(), Some(&document));
        assert!(!crate::local_state_paths().project.exists());
        assert!(!crate::local_state_paths().journal.exists());
    }

    #[test]
    fn route_question_unicode_pages_through_actual_keyboard_pointer_and_controller_without_opening()
    {
        let mut app = editor("route-question-unicode-input");
        let question = "道路の配送順序".repeat(40);
        let document = portable(&question);
        app.open_route_project_document(document.clone()).unwrap();
        let initial = app.route_authoring.as_ref().unwrap().workbench().snapshot();
        let first = app.modal_frame(360, 240).unwrap().to_rgba();
        assert!(app.handle_route_authoring_key(&Key::Named(NamedKey::PageDown), false));
        let last = app.modal_frame(360, 240).unwrap().to_rgba();
        assert!(
            first != last,
            "keyboard paging reveals another part of the question"
        );
        app.handle_route_authoring_key(&Key::Named(NamedKey::PageUp), false);
        assert!(
            app.modal_frame(360, 240).unwrap().to_rgba() == first,
            "keyboard backward page restores the first question text"
        );
        pointer_action(&mut app, Action::QuestionPage(1));
        let pointed = app.modal_frame(360, 240).unwrap().to_rgba();
        let range = 36 * 360 * 4..100 * 360 * 4;
        assert!(
            pointed[range.clone()] == last[range.clone()],
            "visible next-page button uses the same question layout"
        );
        let previous = button_point(&app, Action::QuestionPage(-1));
        app.gamepad.set_cursor_for_test(previous);
        app.handle_gamepad_command(gamepad::Command::PointerMoved {
            point: previous,
            held: false,
        });
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryDown);
        app.handle_gamepad_command(gamepad::Command::PrimaryUp);
        app.input_mode = input_legend::InputMode::KeyboardMouse;
        let back = app.modal_frame(360, 240).unwrap().to_rgba();
        assert!(
            back[range.clone()] == first[range],
            "controller back-page restores the literal question band"
        );
        assert!(app.route_authoring.as_ref().unwrap().paused);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().workbench().snapshot(),
            initial
        );
        assert_eq!(
            app.route_authoring.as_ref().unwrap().received_document(),
            Some(&document)
        );
        app.keep_route_at(&crate::local_state_paths().project);
        let preview = numinous_core::try_load_project_file(&crate::local_state_paths().project)
            .unwrap()
            .preview(
                None,
                &numinous_core::Journal::default(),
                numinous_core::ReceiptCheck::NotSupplied,
            )
            .unwrap();
        assert_eq!(preview.question, question);
        pointer_action(&mut app, Action::Confirm);
        pointer_action(&mut app, Action::Page(Page::Keep));
        app.modal_frame(360, 240).unwrap();
        app.handle_route_authoring_key(&Key::Named(NamedKey::PageDown), false);
        assert_eq!(app.route_authoring.as_ref().unwrap().question, question);
        assert_eq!(
            app.route_authoring.as_ref().unwrap().received_document(),
            Some(&document)
        );
    }
}
