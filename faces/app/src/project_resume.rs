//! Cabinet preview of one kept project.
//!
//! Core reads the chain and the journal. This module only formats what the
//! window shows. Journal text stays in the journal. Nothing here writes a
//! file or awards a journey.

use std::path::Path;

use numinous_core::{
    CreationStatus, EvidenceStatus, NextPreview, Raster, ReceiptCheck, RoomStatus, StudioCreation,
    display_safe, try_load_journal_file, try_load_project_file,
};

/// How the embedded creation can be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreationState {
    /// The revision stored no creation.
    Missing,
    /// The stored text does not reopen as a Studio creation.
    Incompatible,
    /// The stored creation reopened.
    Present,
}

/// Lines for the plate, plus whether Enter may start a creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plate {
    pub(crate) lines: Vec<String>,
    pub(crate) creation_state: CreationState,
}

/// A preview the cabinet can show. `creation` is set only when it reopened.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoadedProject {
    pub(crate) plate: Plate,
    pub(crate) creation: Option<StudioCreation>,
}

/// Whether the Cabinet should offer the kept question.
///
/// A missing file is not an offer. A chain that cannot be read is an offer,
/// so the player can see that refusal instead of a hidden file.
pub(crate) fn chain_available(path: &Path) -> bool {
    match try_load_project_file(path) {
        Ok(chain) => !chain.is_empty(),
        Err(_) => true,
    }
}

/// Read the latest revision. The files are not written.
///
/// # Errors
///
/// Returns a short sentence when the chain is empty or either file cannot
/// be read. The sentence does not include a path.
pub(crate) fn load(project_path: &Path, journal_path: &Path) -> Result<LoadedProject, String> {
    let chain =
        try_load_project_file(project_path).map_err(|_| "Could not read the project chain")?;
    if chain.is_empty() {
        return Err("project chain is empty".to_string());
    }
    let journal = try_load_journal_file(journal_path).map_err(|_| "Could not read the journal")?;
    let revision = chain
        .revisions()
        .last()
        .ok_or_else(|| "project chain is empty".to_string())?;
    let preview = chain
        .preview(None, &journal, ReceiptCheck::NotSupplied)
        .map_err(|_| "Could not read the project chain")?;
    let creation = match preview.creation.status {
        CreationStatus::Present => revision
            .creation_num()
            .and_then(|num| StudioCreation::from_capsule(num).ok()),
        CreationStatus::Missing | CreationStatus::Incompatible => None,
    };
    let creation_state = if creation.is_some() {
        CreationState::Present
    } else if preview.creation.status == CreationStatus::Missing {
        CreationState::Missing
    } else {
        CreationState::Incompatible
    };
    let mut lines = vec![
        format!("Question: {}", display_safe(&preview.question)),
        format!("Will return: {}", preview.will_return),
        next_line(&preview.next),
        format!("Creation: {}", creation_word(creation_state)),
    ];
    if let Some(period) = preview.creation.period_text.as_deref() {
        lines.push(format!("Period: {}", display_safe(period)));
    }
    for room in &preview.rooms {
        if room.status == RoomStatus::Present {
            continue;
        }
        lines.push(format!(
            "Room: {} {}",
            display_safe(&room.id),
            room_word(room.status)
        ));
    }
    for evidence in &preview.evidence {
        if evidence.status == EvidenceStatus::Present {
            continue;
        }
        lines.push(format!(
            "Evidence: {} {} {}",
            evidence.kind,
            evidence_word(evidence.status),
            display_safe(&evidence.digest_hex)
        ));
    }
    lines.push(match creation_state {
        CreationState::Present => "Enter starts the creation. Esc leaves.".to_string(),
        CreationState::Missing | CreationState::Incompatible => {
            "Esc leaves. Nothing is written.".to_string()
        }
    });
    Ok(LoadedProject {
        plate: Plate {
            lines,
            creation_state,
        },
        creation,
    })
}

/// Draw the plate on a quiet band at the top of the frame.
pub(crate) fn draw(raster: &mut Raster, lines: &[String], width: usize, height: usize) {
    if lines.is_empty() || width == 0 || height == 0 {
        return;
    }
    let scale = (width as i32 / 300).clamp(1, 3);
    let columns = ((width as i32 - 20) / (6 * scale)).max(8) as usize;
    let mut wrapped = Vec::new();
    for line in lines {
        wrapped.extend(numinous_core::wrap_text(line, columns));
    }
    let shown: Vec<&str> = wrapped.iter().take(8).map(String::as_str).collect();
    if shown.is_empty() {
        return;
    }
    let line_height = 8 * scale;
    let top = 8;
    let band_bottom = top + shown.len() as i32 * line_height + 8;
    raster.clear_rows(top, band_bottom.min(height as i32));
    for (index, line) in shown.iter().enumerate() {
        numinous_core::draw_text(
            raster,
            line,
            10,
            top + index as i32 * line_height,
            scale,
            '#',
        );
    }
}

fn next_line(next: &NextPreview) -> String {
    match next {
        NextPreview::Ready(call) => format!("Next: {}", call.tool),
        NextPreview::Incompatible(incompatible) => {
            format!("Next: incompatible {}", display_safe(&incompatible.reason))
        }
    }
}

fn creation_word(state: CreationState) -> &'static str {
    match state {
        CreationState::Missing => "missing",
        CreationState::Incompatible => "incompatible",
        CreationState::Present => "present",
    }
}

fn room_word(status: RoomStatus) -> &'static str {
    match status {
        RoomStatus::Present => "present",
        RoomStatus::Missing => "missing",
        RoomStatus::Incompatible => "incompatible",
    }
}

fn evidence_word(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Present => "present",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Corrected => "corrected",
        EvidenceStatus::Collided => "collided",
        EvidenceStatus::Incompatible => "incompatible",
    }
}
