use super::{App, AudioProgram, append_crash_log_at, feedback, postcard, studio_panel};
use numinous_core::Raster;

/// Which naming field the keyboard currently feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NamingField {
    Title,
    Author,
    Credit,
}

/// The naming step's fields, exactly as the player left them.
///
/// Two levels of optionality, because two different questions are being
/// asked. Whether a share carries this at all answers "was the player
/// asked"; each field answers "what did they leave". Collapsing those into
/// one `Option` per field is what made a player who deleted a reopened
/// creation's name watch the old name ship anyway: the form said unnamed
/// while the capsule, the README, the postcard headline, and the folder
/// slug all said otherwise. An emptied field is a clearing, not an
/// absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ShareIdentity {
    pub(super) title: Option<String>,
    pub(super) author: Option<String>,
    pub(super) credit: Option<String>,
}

/// The F4 naming step's editable state: one text line for the creation's
/// name, one for its signature, one for prose credit.
#[derive(Debug, Clone)]
pub(super) struct ShareNaming {
    pub(super) title: String,
    pub(super) author: String,
    pub(super) credit: String,
    pub(super) field: NamingField,
}

impl ShareNaming {
    /// The identity decision these fields carry, clearings included.
    pub(super) fn identity(&self) -> ShareIdentity {
        let field = |value: &str| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        };
        ShareIdentity {
            title: field(&self.title),
            author: field(&self.author),
            credit: field(&self.credit),
        }
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.field {
            NamingField::Title => &mut self.title,
            NamingField::Author => &mut self.author,
            NamingField::Credit => &mut self.credit,
        }
    }
}

impl App {
    /// Report an export result through the title or a durable failure banner.
    pub(super) fn report_export_outcome(
        &mut self,
        success_label: &str,
        failure_line: &'static str,
        outcome: std::io::Result<std::path::PathBuf>,
    ) {
        match outcome {
            Ok(path) => {
                if let Some(window) = &self.window {
                    window.set_title(&format!("Numinous  |  {success_label}: {}", path.display()));
                }
            }
            Err(error) => {
                let _ = append_crash_log_at(
                    &self.crash_log,
                    &format!("{success_label} failed: {error}\n"),
                );
                self.banner = Some(feedback::Banner::status(
                    failure_line,
                    feedback::REFUSAL_FRAMES,
                ));
            }
        }
    }

    pub(super) fn save_postcard_to(
        &self,
        dir: &std::path::Path,
    ) -> std::io::Result<std::path::PathBuf> {
        if self.current_room_is_life() {
            let room = self.rooms[self.current].as_ref();
            let size = postcard::POSTCARD_SIZE as usize;
            let mut raster = Raster::with_accent(size, size, room.meta().accent);
            self.life_session.render(&mut raster);
            let mut rgba = raster.to_rgba();
            self.era.apply(&mut rgba, size, size);
            return postcard::write_rendered_postcard(
                room.meta().id,
                self.life_session.generation(),
                &rgba,
                dir,
            );
        }
        postcard::write_room_postcard(
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.era,
            dir,
        )
    }

    /// Write a short looping APNG of the current visit: one phase cycle, or
    /// advancing Life generations for the persistent Game of Life session.
    pub(super) fn save_short_loop(&mut self) {
        let outcome = self.save_short_loop_to(&postcard::default_postcard_dir());
        self.report_export_outcome(
            "loop saved",
            "LOOP SAVE FAILED  SEE .NUMINOUS-CRASH.LOG",
            outcome,
        );
    }

    fn save_short_loop_to(&self, dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        if self.current_room_is_life() {
            let room = self.rooms[self.current].as_ref();
            return postcard::write_life_loop(
                room.meta().id,
                room.meta().accent,
                &self.life_session,
                self.era,
                dir,
            );
        }
        postcard::write_room_loop(
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.era,
            dir,
        )
    }

    /// Package postcard + loop + README into one share folder (CLI parity).
    pub(super) fn save_share_bundle(&mut self) {
        let outcome = self.save_share_bundle_to(&postcard::default_postcard_dir());
        self.report_export_outcome(
            "share pack",
            "SHARE PACK FAILED  SEE .NUMINOUS-CRASH.LOG",
            outcome,
        );
    }

    /// Write the current room's postcard PNG: the P key.
    pub(super) fn save_postcard(&mut self) {
        let outcome = self.save_postcard_to(&postcard::default_postcard_dir());
        self.report_export_outcome(
            "postcard saved",
            "POSTCARD FAILED  SEE .NUMINOUS-CRASH.LOG",
            outcome,
        );
    }

    fn save_share_bundle_to(&self, dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        if self.current_room_is_life() {
            let room = self.rooms[self.current].as_ref();
            return postcard::write_life_share_bundle(
                room.meta().id,
                room.meta().accent,
                &self.life_session,
                self.era,
                self.variation,
                dir,
            );
        }
        postcard::write_room_share_bundle(
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.era,
            self.variation,
            dir,
        )
    }

    /// Open the F4 naming step. The title prefills from the creation being
    /// shared (so an untouched re-share keeps its identity by default) and
    /// the author from the last signature, because naming happens in the
    /// instrument, not only in CLI flags.
    pub(super) fn begin_share_naming(&mut self) {
        if self.share_naming.is_some() {
            return;
        }
        let identity = self.studio_panel.current_creation(self.t).ok();
        let title = identity
            .as_ref()
            .and_then(|creation| creation.title())
            .unwrap_or_default()
            .to_string();
        let author = identity
            .as_ref()
            .and_then(|creation| creation.author())
            .unwrap_or(&self.remembered_author)
            .to_string();
        let credit = identity
            .as_ref()
            .and_then(|creation| creation.credit())
            .unwrap_or_default()
            .to_string();
        self.share_naming = Some(ShareNaming {
            title,
            author,
            credit,
            field: NamingField::Title,
        });
    }

    /// Append text to the active naming field, under the same printable
    /// ASCII bound the capsule format enforces, so a name the editor
    /// accepts is a name the share cannot refuse.
    pub(super) fn naming_push_text(&mut self, text: &str) {
        let Some(naming) = self.share_naming.as_mut() else {
            return;
        };
        let cap = match naming.field {
            NamingField::Title | NamingField::Author => numinous_core::MAX_META_TEXT_CHARS,
            NamingField::Credit => numinous_core::MAX_CREDIT_CHARS,
        };
        let field = naming.active_field_mut();
        let mut remaining = cap.saturating_sub(field.chars().count());
        for c in text.chars() {
            if remaining > 0 && (' '..='~').contains(&c) {
                field.push(c);
                remaining -= 1;
            }
        }
    }

    pub(super) fn naming_backspace(&mut self) {
        if let Some(naming) = self.share_naming.as_mut() {
            naming.active_field_mut().pop();
        }
    }

    pub(super) fn naming_toggle_field(&mut self) {
        if let Some(naming) = self.share_naming.as_mut() {
            naming.field = match naming.field {
                NamingField::Title => NamingField::Author,
                NamingField::Author => NamingField::Credit,
                NamingField::Credit => NamingField::Title,
            };
        }
    }

    /// Cancel the naming step without sharing anything, and say so: a
    /// closed prompt with no banner would leave whether anything was
    /// written a mystery.
    pub(super) fn cancel_share_naming(&mut self) {
        if self.share_naming.take().is_some() {
            self.banner = Some(feedback::Banner::status("SHARE CANCELLED", 90));
        }
    }

    /// Confirm the naming step: remember the signature and share the bundle.
    pub(super) fn confirm_share_naming(&mut self) {
        let Some(naming) = self.share_naming.take() else {
            return;
        };
        self.remembered_author = naming.author.trim().to_string();
        // An emptied field is the player clearing the name, which the share
        // must honor; it is not the same as never having been asked.
        self.share_studio_creation(Some(naming.identity()));
    }

    /// The Studio share bundle on one key: `creation.num`, the link in the
    /// README, the postcard, and the sung melody as MIDI, into one fresh
    /// share folder.
    ///
    /// Success and failure both speak through the shared export reporter;
    /// the writer discards its own partial folder on failure, so the failure
    /// line stays short rather than promising a cleanup state it cannot
    /// fully guarantee.
    pub(super) fn share_studio_creation(&mut self, identity: Option<ShareIdentity>) {
        match self.share_studio_creation_to(&postcard::default_postcard_dir(), identity) {
            Ok(Ok(dir)) => {
                self.report_export_outcome(
                    "studio share",
                    "SHARE FAILED  SEE .NUMINOUS-CRASH.LOG",
                    Ok(dir),
                );
                self.banner = Some(feedback::Banner::status(
                    "SHARED  .NUM + LINK + PNG + MIDI",
                    90,
                ));
            }
            Ok(Err(studio_panel::ShareRefusal::UnparsedFormula)) => {
                // An unparsed edit has no curve to promise; the refusal names
                // the way forward instead of silently sharing the last-good.
                self.banner = Some(feedback::Banner::status(
                    "FIX THE FORMULA TO SHARE",
                    feedback::REFUSAL_FRAMES,
                ));
            }
            Ok(Err(studio_panel::ShareRefusal::LineageTooLarge)) => {
                // A different refusal deserves a different sentence: telling
                // the player to fix a formula that parses fine points them
                // at the wrong cause.
                self.banner = Some(feedback::Banner::status(
                    "FORK LINEAGE TOO LARGE TO SHARE",
                    feedback::REFUSAL_FRAMES,
                ));
            }
            Err(error) => {
                self.report_export_outcome(
                    "studio share",
                    "SHARE FAILED  SEE .NUMINOUS-CRASH.LOG",
                    Err(error),
                );
            }
        }
    }

    /// Testable body: the outer result is the write, the inner one the
    /// panel's refusal to produce a creation at all.
    pub(super) fn share_studio_creation_to(
        &self,
        parent: &std::path::Path,
        identity: Option<ShareIdentity>,
    ) -> std::io::Result<Result<std::path::PathBuf, studio_panel::ShareRefusal>> {
        let mut creation = match self.studio_panel.current_creation(self.t) {
            Ok(creation) => creation,
            Err(refusal) => return Ok(Err(refusal)),
        };
        // The naming step's fields ride the capsule, clearings included: an
        // untouched reopen carries the opened capsule's own title and author,
        // so a share that ignored an emptied field would ship a name the
        // player had just deleted. The editor enforces the same printable
        // ASCII bound the format validates, so a name that reaches here
        // cannot be refused; if the two rules ever drift, the share fails
        // loudly through the io path rather than silently shipping wrong
        // identity.
        if let Some(ShareIdentity {
            title,
            author,
            credit,
        }) = identity
        {
            creation = match title {
                Some(title) => creation.with_title(&title).map_err(std::io::Error::other)?,
                None => creation.without_title(),
            };
            creation = match author {
                Some(author) => creation
                    .with_author(&author)
                    .map_err(std::io::Error::other)?,
                None => creation.without_author(),
            };
            creation = match credit {
                Some(credit) => creation
                    .with_credit(&credit)
                    .map_err(std::io::Error::other)?,
                None => creation.without_credit(),
            };
        }
        // Record the era only when it says something: Modern is the default
        // look, and omitting it keeps a plain share a version 1 capsule that
        // older builds still open.
        let creation = if self.era == numinous_core::Era::Modern {
            creation
        } else {
            creation.with_era(self.era)
        };
        let rgba = self.studio_panel.postcard_rgba(
            self.t,
            postcard::POSTCARD_SIZE as usize,
            self.era,
            creation.title(),
            creation.author(),
        );
        postcard::write_studio_share_bundle(&creation, &rgba, parent).map(Ok)
    }

    /// Move the Gallery cursor by whole tiles.
    pub(super) fn gallery_move(&mut self, dx: i32, dy: i32) {
        if let Some(gallery) = &mut self.gallery {
            gallery.move_selection(dx, dy);
        }
    }

    /// Walk one step up the remix tree, or say exactly why the cursor
    /// stayed: no lineage and an absent parent are different answers, and
    /// a key that silently does nothing teaches players it is broken.
    pub(super) fn gallery_select_parent(&mut self) {
        const NO_LINEAGE: &str = "THIS ONE DESCENDS FROM NOTHING";
        const PARENT_ABSENT: &str = "ITS PARENT IS NOT ON THIS WALL";
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        match gallery.parent_status() {
            crate::gallery::ParentStatus::Local(_) => {
                let _ = gallery.select_parent();
                // A successful walk retires an earlier refusal, so a stale
                // DESCENDS FROM NOTHING cannot linger over a cursor that
                // just moved; unrelated banners are left alone.
                if self.banner.as_ref().is_some_and(|banner| {
                    banner
                        .lines()
                        .first()
                        .is_some_and(|line| line == NO_LINEAGE || line == PARENT_ABSENT)
                }) {
                    self.banner = None;
                }
            }
            crate::gallery::ParentStatus::NoLineage => {
                self.banner = Some(feedback::Banner::status(
                    NO_LINEAGE,
                    feedback::REFUSAL_FRAMES,
                ));
            }
            crate::gallery::ParentStatus::Absent => {
                self.banner = Some(feedback::Banner::status(
                    PARENT_ABSENT,
                    feedback::REFUSAL_FRAMES,
                ));
            }
        }
    }

    /// Fork the creation under the Gallery cursor: the wall closes and the
    /// Studio holds an editable, singing copy that remembers its parent, so
    /// the next share records the descent.
    ///
    /// No paused preview: the player browsed the wall and chose the fork
    /// gesture themselves, and fork must be as cheap as play.
    pub(super) fn gallery_fork_selected(&mut self) {
        let Some(creation) = self
            .gallery
            .as_ref()
            .and_then(|gallery| gallery.selected_creation())
            .cloned()
        else {
            return;
        };
        self.gallery = None;
        self.quiz = None;
        if let Some(era) = creation.era() {
            self.era = era;
        }
        let spec = self.studio_panel.fork_creation(&creation);
        self.enter_studio_shell();
        self.set_studio_sound(spec);
        self.banner = Some(feedback::Banner::status("FORKED  IT IS YOURS NOW", 90));
    }

    /// Open the creation under the Gallery cursor: the wall closes and the
    /// Studio holds the exact reopened state, paused like any other open.
    pub(super) fn gallery_open_selected(&mut self) {
        let Some(creation) = self
            .gallery
            .as_ref()
            .and_then(|gallery| gallery.selected_creation())
            .cloned()
        else {
            return;
        };
        self.gallery = None;
        self.open_studio_creation(&creation);
        self.banner = Some(feedback::Banner::status("REOPENED  ENTER: PLAY", 90));
    }
    pub(super) fn enter_studio(&mut self) {
        self.enter_studio_shell();
        self.set_studio_sound(self.studio_panel.entry_sound());
    }

    /// Enter Studio mode without touching the panel's formula or voice, so a
    /// reopened creation is not resung by the entry itself.
    fn enter_studio_shell(&mut self) {
        self.the_show = false;
        self.paused = false;
        self.close_menu();
        self.show_journey = false;
        self.studio = true;
        self.audio_program = AudioProgram::Studio;
        if let Some(player) = &self.player {
            player.clear_oneshot();
        }
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    /// Reopen a saved creation in the Studio, exactly and paused.
    ///
    /// The panel pins the saved window and knob; the entry submits silence so
    /// whatever program was playing does not keep sounding under a preview
    /// that has deliberately not started singing yet.
    pub(super) fn open_studio_creation(&mut self, creation: &numinous_core::StudioCreation) {
        // A quiz is stateless and would otherwise keep owning the keyboard
        // over the newly opened Studio; scored runs are guarded at the door
        // in open_dropped_file instead of being silently abandoned here.
        self.quiz = None;
        // The wall and the naming step were both about the creation that
        // was here a moment ago. A new one arriving ends them, or Enter
        // would share a stranger's capsule under the name still on screen,
        // and the REOPENED banner would promise a key the wall had taken.
        self.gallery = None;
        self.share_naming = None;
        // A capsule that recorded its Visual Era reopens in that era: the
        // look is part of what was saved.
        if let Some(era) = creation.era() {
            self.era = era;
        }
        self.studio_panel.open_creation(creation);
        self.enter_studio();
    }

    /// Enter confirms a paused reopened preview: the creation starts singing.
    pub(super) fn studio_confirm_opened(&mut self) {
        if let Some(spec) = self.studio_panel.confirm_opened() {
            self.set_studio_sound(Some(spec));
        }
    }

    /// Open a `.num` file from disk into the Studio, or say briefly why not.
    fn open_num_file(&mut self, path: &std::path::Path) {
        match numinous_core::StudioCreation::from_num_path(path) {
            Ok(creation) => {
                self.open_studio_creation(&creation);
                self.banner = Some(feedback::Banner::status("REOPENED  ENTER: PLAY", 90));
            }
            Err(error) => {
                let line = match error {
                    numinous_core::NumFileError::Io(_) => "COULD NOT READ THE .NUM FILE",
                    numinous_core::NumFileError::TooLarge => "THE .NUM FILE IS TOO LARGE",
                    numinous_core::NumFileError::Invalid(_) => "NOT A VALID .NUM CREATION",
                };
                self.banner = Some(feedback::Banner::status(line, feedback::REFUSAL_FRAMES));
            }
        }
    }

    /// A file dropped on the window: only a `.num` creation opens here.
    pub(super) fn open_dropped_file(&mut self, path: &std::path::Path) {
        // A scored run in progress is not abandoned by a stray drop; the
        // player finishes or leaves it themselves, then drops again.
        if self.gauntlet.is_some()
            || self.munch.is_some()
            || self.nim.is_some()
            || self.arcade.is_some()
            || self.session_viewer.is_open()
        {
            self.banner = Some(feedback::Banner::status(
                "FINISH THE GAME FIRST",
                feedback::REFUSAL_FRAMES,
            ));
            return;
        }
        let is_num = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("num"));
        if !is_num {
            self.banner = Some(feedback::Banner::status(
                "ONLY .NUM CREATIONS OPEN HERE",
                feedback::REFUSAL_FRAMES,
            ));
            return;
        }
        self.open_num_file(path);
    }

    /// The launch-argument front door: a `.num` path or a `numinous://` link.
    pub(super) fn open_start_input(&mut self, input: &str) {
        if input.starts_with("numinous://") {
            match numinous_core::StudioCreation::from_link(input) {
                Ok(creation) => {
                    self.open_studio_creation(&creation);
                    self.banner = Some(feedback::Banner::status("REOPENED  ENTER: PLAY", 90));
                }
                Err(_) => {
                    self.banner = Some(feedback::Banner::status(
                        "NOT A VALID NUMINOUS LINK",
                        feedback::REFUSAL_FRAMES,
                    ));
                }
            }
            return;
        }
        self.open_num_file(std::path::Path::new(input));
    }
}
