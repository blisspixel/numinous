//! Local persistence helpers shared by the App, CLI, and MCP faces.
//!
//! The domain types stay pure: [`Journey`] and [`Scoreboard`] still parse and
//! serialize text without owning file I/O. This module owns the defensive local
//! file behavior every face needs: one lock file per state file,
//! merge-before-write semantics, and same-directory temp writes before commit.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{Journey, Scoreboard};

const LOCK_RETRIES: usize = 2500;
const LOCK_SLEEP: Duration = Duration::from_millis(2);
const LOCK_STALE_AFTER_SECS: u64 = 30 * 60;
/// A much shorter grace for a lock whose holder process is confidently gone. A
/// real writer holds the lock for milliseconds, so a lock older than this from a
/// dead PID is certainly abandoned; recovering it here (instead of waiting the
/// full staleness window) means a hard crash blocks other writers for seconds,
/// not half an hour.
const LOCK_DEAD_PID_GRACE_SECS: u64 = 10;
const MAX_JOURNEY_FILE_BYTES: u64 = 64 * 1024;
const MAX_LOCK_FILE_BYTES: u64 = 4 * 1024;
const MAX_SCOREBOARD_FILE_BYTES: u64 = 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const MAX_MANAGED_CACHE_ENTRIES: usize = 4096;
const MAX_MANAGED_SIDECARS: usize = 4096;

/// Resolved locations for every Numinous-managed local state store.
///
/// User-selected exports and the installed application tree are deliberately
/// outside this set because their locations and ownership have separate
/// lifecycles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalStatePaths {
    /// Journey progress text file.
    pub journey: PathBuf,
    /// Shared high-score text file.
    pub scores: PathBuf,
    /// Player-owned local Cairn draft text file.
    pub cairn: PathBuf,
    /// Flat directory of generated radio WAV files.
    pub radio_cache: PathBuf,
    /// App crash diagnostic text file.
    pub crash_log: PathBuf,
}

/// Metadata for one expected regular state file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFileInventory {
    /// Resolved selected path.
    pub path: PathBuf,
    /// Whether any filesystem object exists at the path.
    pub exists: bool,
    /// Regular-file length, or zero for a missing or unexpected object.
    pub bytes: u64,
    /// False when the selected path is a directory, link, or other unexpected
    /// object. Erasure rejects such an object instead of following it.
    pub managed_file: bool,
    /// Adjacent lock, recovery, or orphan temporary files owned by this store.
    pub sidecar_files: usize,
    /// Bytes in recognized adjacent sidecar files.
    pub sidecar_bytes: u64,
    /// Whether sidecar inspection stopped at its bounded directory-entry cap.
    pub sidecar_scan_capped: bool,
}

/// Journey facts useful for a transparent local-state preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalJourneyInventory {
    /// Journey file metadata.
    pub file: LocalFileInventory,
    /// Unique catalog rooms recorded in the Journey.
    pub rooms_entered: usize,
    /// Recorded Journey wins.
    pub wins: u32,
    /// Recorded Journey plays.
    pub plays: u32,
    /// Recorded hidden discoveries.
    pub secrets_heard: u32,
}

/// Score-table facts useful for a transparent local-state preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalScoresInventory {
    /// Score file metadata.
    pub file: LocalFileInventory,
    /// Valid bounded score entries.
    pub entries: usize,
}

/// Local Cairn facts. Bundled founding stones are application content and do
/// not contribute to this player-owned draft count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCairnInventory {
    /// Local Cairn file metadata.
    pub file: LocalFileInventory,
    /// Valid player-owned draft lines, excluding bundled founding stones.
    pub local_drafts: usize,
}

/// Metadata for the flat generated-radio cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCacheInventory {
    /// Resolved generated-radio directory path.
    pub path: PathBuf,
    /// Whether any filesystem object exists at the path.
    pub exists: bool,
    /// Total bytes across recognized direct WAV files.
    pub bytes: u64,
    /// Recognized direct WAV files.
    pub files: usize,
    /// Direct entries that are not regular WAV files.
    pub unexpected_entries: usize,
    /// Whether inspection stopped at the bounded entry cap.
    pub truncated: bool,
    /// Adjacent transaction lock or recovery marker files.
    pub sidecar_files: usize,
    /// Bytes in adjacent transaction lock or recovery marker files.
    pub sidecar_bytes: u64,
    /// Whether adjacent temporary-file inspection reached its cap.
    pub sidecar_scan_capped: bool,
}

/// Truthful point-in-time inventory of Numinous-managed local state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalStateInventory {
    /// Journey inventory and bounded progress counts.
    pub journey: LocalJourneyInventory,
    /// Score inventory and bounded entry count.
    pub scores: LocalScoresInventory,
    /// Local Cairn inventory and valid draft count.
    pub cairn: LocalCairnInventory,
    /// Generated-radio cache inventory.
    pub radio_cache: LocalCacheInventory,
    /// App crash-log inventory.
    pub crash_log: LocalFileInventory,
}

impl LocalStateInventory {
    /// Bytes in regular managed files and recognized cached WAV files.
    #[must_use]
    pub fn total_managed_bytes(&self) -> u64 {
        self.journey
            .file
            .bytes
            .saturating_add(self.journey.file.sidecar_bytes)
            .saturating_add(self.scores.file.bytes)
            .saturating_add(self.scores.file.sidecar_bytes)
            .saturating_add(self.cairn.file.bytes)
            .saturating_add(self.cairn.file.sidecar_bytes)
            .saturating_add(self.radio_cache.bytes)
            .saturating_add(self.radio_cache.sidecar_bytes)
            .saturating_add(self.crash_log.bytes)
            .saturating_add(self.crash_log.sidecar_bytes)
    }

    /// Number of managed stores that still exist, including an unexpected
    /// object at one of the managed paths.
    #[must_use]
    pub fn managed_residue_count(&self) -> usize {
        [
            self.journey.file.exists || self.journey.file.sidecar_files != 0,
            self.scores.file.exists || self.scores.file.sidecar_files != 0,
            self.cairn.file.exists || self.cairn.file.sidecar_files != 0,
            self.radio_cache.exists || self.radio_cache.sidecar_files != 0,
            self.crash_log.exists || self.crash_log.sidecar_files != 0,
        ]
        .into_iter()
        .filter(|exists| *exists)
        .count()
    }
}

/// Exact stores selected for one confirmed erasure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalStateEraseSelection {
    /// Erase Journey progress.
    pub journey: bool,
    /// Erase shared scores.
    pub scores: bool,
    /// Erase player-owned local Cairn drafts.
    pub cairn: bool,
    /// Erase recognized generated radio tracks.
    pub radio_cache: bool,
    /// Erase the App crash diagnostic.
    pub crash_log: bool,
}

impl LocalStateEraseSelection {
    /// Select every Numinous-managed local store.
    #[must_use]
    pub const fn complete() -> Self {
        Self {
            journey: true,
            scores: true,
            cairn: true,
            radio_cache: true,
            crash_log: true,
        }
    }
}

/// A confirmed erasure stopped at one named store. Earlier stores in the
/// fixed deletion order may already be absent, so callers should inspect again
/// before reporting residue.
#[derive(Debug)]
pub struct LocalStateEraseError {
    target: &'static str,
    source: io::Error,
}

/// Exclusive process-local persistence guard for one managed state root.
///
/// A producer that writes the generated-radio cache holds this same guard as
/// complete erasure, preventing publication and deletion from overlapping.
pub struct LocalStateLock {
    inner: PersistLock,
}

/// Acquire the shared lock for one managed state path or directory.
pub fn lock_local_state(path: &Path) -> io::Result<LocalStateLock> {
    PersistLock::acquire(path).map(|inner| LocalStateLock { inner })
}

impl LocalStateLock {
    fn release(self) -> io::Result<()> {
        self.inner.release()
    }
}

impl LocalStateEraseError {
    /// Human-readable name of the store whose erasure failed.
    #[must_use]
    pub const fn target(&self) -> &'static str {
        self.target
    }
}

impl std::fmt::Display for LocalStateEraseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "could not erase {}: {}",
            self.target, self.source
        )
    }
}

impl std::error::Error for LocalStateEraseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

fn sidecar_name_parts(path: &Path) -> (String, String, String) {
    let base = file_name(path).to_string_lossy().into_owned();
    (
        format!("{base}.lock"),
        format!("{base}.lock.recover"),
        format!(".{base}."),
    )
}

fn inspect_managed_sidecars(path: &Path) -> io::Result<(usize, u64, bool)> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let (lock_name, recovery_name, temp_prefix) = sidecar_name_parts(path);
    let mut files = 0_usize;
    let mut bytes = 0_u64;
    for exact_name in [&lock_name, &recovery_name] {
        match fs::symlink_metadata(parent.join(exact_name)) {
            Ok(metadata) => {
                files += 1;
                if metadata.file_type().is_file() {
                    bytes = bytes.saturating_add(metadata.len());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok((files, bytes, false));
        }
        Err(error) => return Err(error),
    };
    let mut capped = false;
    for (index, entry) in entries.enumerate() {
        if index == MAX_MANAGED_SIDECARS {
            capped = true;
            break;
        }
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&temp_prefix) && name.ends_with(".tmp") {
            files += 1;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_file() {
                bytes = bytes.saturating_add(metadata.len());
            }
        }
    }
    Ok((files, bytes, capped))
}

fn inspect_managed_file(path: &Path) -> io::Result<LocalFileInventory> {
    let resolved_path = resolved_path_for_comparison(path)?;
    let (sidecar_files, sidecar_bytes, sidecar_scan_capped) = inspect_managed_sidecars(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let managed_file = metadata.file_type().is_file();
            Ok(LocalFileInventory {
                path: resolved_path,
                exists: true,
                bytes: if managed_file { metadata.len() } else { 0 },
                managed_file,
                sidecar_files,
                sidecar_bytes,
                sidecar_scan_capped,
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(LocalFileInventory {
            path: resolved_path,
            exists: false,
            bytes: 0,
            managed_file: true,
            sidecar_files,
            sidecar_bytes,
            sidecar_scan_capped,
        }),
        Err(error) => Err(error),
    }
}

fn inspect_managed_cache(path: &Path) -> io::Result<LocalCacheInventory> {
    let resolved_path = resolved_path_for_comparison(path)?;
    let (sidecar_files, sidecar_bytes, sidecar_scan_capped) = inspect_managed_sidecars(path)?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LocalCacheInventory {
                path: resolved_path,
                exists: false,
                bytes: 0,
                files: 0,
                unexpected_entries: 0,
                truncated: false,
                sidecar_files,
                sidecar_bytes,
                sidecar_scan_capped,
            });
        }
        Err(error) => return Err(error),
    };
    if !metadata.file_type().is_dir() {
        return Ok(LocalCacheInventory {
            path: resolved_path,
            exists: true,
            bytes: 0,
            files: 0,
            unexpected_entries: 1,
            truncated: false,
            sidecar_files,
            sidecar_bytes,
            sidecar_scan_capped,
        });
    }

    let mut bytes = 0_u64;
    let mut files = 0_usize;
    let mut unexpected_entries = 0_usize;
    let mut truncated = false;
    for (index, entry) in fs::read_dir(path)?.enumerate() {
        if index == MAX_MANAGED_CACHE_ENTRIES {
            truncated = true;
            break;
        }
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_file() && is_generated_radio_file(&entry.file_name()) {
            files += 1;
            bytes = bytes.saturating_add(metadata.len());
        } else {
            unexpected_entries += 1;
        }
    }
    Ok(LocalCacheInventory {
        path: resolved_path,
        exists: true,
        bytes,
        files,
        unexpected_entries,
        truncated,
        sidecar_files,
        sidecar_bytes,
        sidecar_scan_capped,
    })
}

fn is_generated_radio_file(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    crate::radio::STATIONS.iter().any(|station| {
        name.strip_prefix(station.id)
            .and_then(|suffix| suffix.strip_prefix('-'))
            .and_then(|suffix| suffix.strip_suffix(".wav"))
            .is_some_and(|number| {
                !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
            })
    })
}

fn resolved_path_for_comparison(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name())
        && parent.exists()
    {
        return fs::canonicalize(parent).map(|parent| parent.join(name));
    }
    Ok(absolute)
}

fn validate_local_state_paths(paths: &LocalStatePaths) -> io::Result<()> {
    let files = [
        ("journey", &paths.journey),
        ("scores", &paths.scores),
        ("Cairn drafts", &paths.cairn),
        ("crash log", &paths.crash_log),
    ]
    .map(|(name, path)| Ok((name, resolved_path_for_comparison(path)?)))
    .into_iter()
    .collect::<io::Result<Vec<_>>>()?;
    for (index, (left_name, left)) in files.iter().enumerate() {
        for (right_name, right) in files.iter().skip(index + 1) {
            if left == right {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{left_name} and {right_name} resolve to the same path"),
                ));
            }
        }
    }
    let cache = resolved_path_for_comparison(&paths.radio_cache)?;
    for (name, file) in &files {
        if file.starts_with(&cache) || cache.starts_with(file) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} and radio cache paths overlap"),
            ));
        }
    }
    Ok(())
}

/// Inspect every Numinous-managed local state store without changing it.
pub fn inspect_local_state(paths: &LocalStatePaths) -> io::Result<LocalStateInventory> {
    let journey_file = inspect_managed_file(&paths.journey)?;
    let scores_file = inspect_managed_file(&paths.scores)?;
    let cairn_file = inspect_managed_file(&paths.cairn)?;
    let journey = if journey_file.managed_file {
        load_journey_file(&paths.journey)
    } else {
        Journey::default()
    };
    let scores = if scores_file.managed_file {
        load_scoreboard_file(&paths.scores)
    } else {
        Scoreboard::default()
    };
    let local_drafts = if cairn_file.managed_file {
        crate::cairn::local_bequest_count(&paths.cairn)
    } else {
        0
    };
    Ok(LocalStateInventory {
        journey: LocalJourneyInventory {
            file: journey_file,
            rooms_entered: journey.visited.len(),
            wins: journey.wins,
            plays: journey.plays,
            secrets_heard: journey.secrets,
        },
        scores: LocalScoresInventory {
            file: scores_file,
            entries: scores.entries.len(),
        },
        cairn: LocalCairnInventory {
            file: cairn_file,
            local_drafts,
        },
        radio_cache: inspect_managed_cache(&paths.radio_cache)?,
        crash_log: inspect_managed_file(&paths.crash_log)?,
    })
}

fn reject_unmanaged_file(path: &Path) -> io::Result<()> {
    let inventory = inspect_managed_file(path)?;
    if inventory.managed_file {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a regular managed file", path.display()),
        ))
    }
}

fn preflight_managed_file(path: &Path) -> io::Result<()> {
    reject_unmanaged_file(path)?;
    let _temps = orphan_temp_files(path)?;
    Ok(())
}

fn remove_managed_file_locked(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    let parent = open_parent_directory(path)?;
    remove_orphan_temp_files(path)?;
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    #[cfg(unix)]
    let _sync_result = parent.sync_all();
    Ok(())
}

fn managed_cache_entries(path: &Path) -> io::Result<Option<Vec<PathBuf>>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a managed cache directory", path.display()),
        ));
    }
    let mut entries = Vec::new();
    for (index, entry) in fs::read_dir(path)?.enumerate() {
        if index == MAX_MANAGED_CACHE_ENTRIES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} exceeds the managed cache entry cap", path.display()),
            ));
        }
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.file_type().is_file() || !is_generated_radio_file(&entry.file_name()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} contains an entry not owned by the generated-radio cache and was not changed",
                    path.display()
                ),
            ));
        }
        entries.push(entry.path());
    }
    Ok(Some(entries))
}

fn remove_managed_cache_locked(path: &Path) -> io::Result<()> {
    remove_orphan_temp_files(path)?;
    let Some(entries) = managed_cache_entries(path)? else {
        return Ok(());
    };
    for entry in entries {
        fs::remove_file(entry)?;
    }
    fs::remove_dir(path)
}

fn erase_file_target(
    selected: bool,
    target: &'static str,
    path: &Path,
) -> Result<(), LocalStateEraseError> {
    if !selected {
        return Ok(());
    }
    remove_managed_file_locked(path).map_err(|source| LocalStateEraseError { target, source })
}

fn acquire_erasure_locks(
    paths: &LocalStatePaths,
    selection: LocalStateEraseSelection,
) -> Result<Vec<LocalStateLock>, LocalStateEraseError> {
    let mut selected = Vec::new();
    for (enabled, target, path) in [
        (selection.journey, "journey", &paths.journey),
        (selection.scores, "scores", &paths.scores),
        (selection.cairn, "Cairn drafts", &paths.cairn),
        (selection.radio_cache, "radio cache", &paths.radio_cache),
        (selection.crash_log, "crash log", &paths.crash_log),
    ] {
        if enabled {
            let resolved =
                resolved_path_for_comparison(path).map_err(|source| LocalStateEraseError {
                    target: "local-state path layout",
                    source,
                })?;
            selected.push((resolved, target, path));
        }
    }
    selected.sort_by(|left, right| left.0.cmp(&right.0));
    let mut locks = Vec::with_capacity(selected.len());
    for (_, target, path) in selected {
        locks.push(
            lock_local_state(path).map_err(|source| LocalStateEraseError { target, source })?,
        );
    }
    Ok(locks)
}

fn preflight_selected_state(
    paths: &LocalStatePaths,
    selection: LocalStateEraseSelection,
) -> Result<(), LocalStateEraseError> {
    for (enabled, target, path) in [
        (selection.journey, "journey", &paths.journey),
        (selection.scores, "scores", &paths.scores),
        (selection.cairn, "Cairn drafts", &paths.cairn),
        (selection.crash_log, "crash log", &paths.crash_log),
    ] {
        if enabled {
            preflight_managed_file(path)
                .map_err(|source| LocalStateEraseError { target, source })?;
        }
    }
    if selection.radio_cache {
        orphan_temp_files(&paths.radio_cache).map_err(|source| LocalStateEraseError {
            target: "radio cache",
            source,
        })?;
        managed_cache_entries(&paths.radio_cache).map_err(|source| LocalStateEraseError {
            target: "radio cache",
            source,
        })?;
    }
    Ok(())
}

/// Erase the exact selected local stores, then return a fresh residue inventory.
///
/// The generated-radio directory is attempted first because an unexpected
/// entry must fail closed before any personal record is removed. Remaining
/// stores then proceed from ancillary diagnostics to primary Journey state.
/// Every selected lock is released with checked cleanup before residue is
/// inspected, so a successful receipt includes no transaction sidecars.
pub fn erase_local_state(
    paths: &LocalStatePaths,
    selection: LocalStateEraseSelection,
) -> Result<LocalStateInventory, LocalStateEraseError> {
    validate_local_state_paths(paths).map_err(|source| LocalStateEraseError {
        target: "local-state path layout",
        source,
    })?;
    let locks = acquire_erasure_locks(paths, selection)?;
    preflight_selected_state(paths, selection)?;
    if selection.radio_cache {
        remove_managed_cache_locked(&paths.radio_cache).map_err(|source| LocalStateEraseError {
            target: "radio cache",
            source,
        })?;
    }
    erase_file_target(selection.crash_log, "crash log", &paths.crash_log)?;
    erase_file_target(selection.cairn, "Cairn drafts", &paths.cairn)?;
    erase_file_target(selection.scores, "scores", &paths.scores)?;
    erase_file_target(selection.journey, "journey", &paths.journey)?;
    for lock in locks.into_iter().rev() {
        lock.release().map_err(|source| LocalStateEraseError {
            target: "transaction lock cleanup",
            source,
        })?;
    }
    inspect_local_state(paths).map_err(|source| LocalStateEraseError {
        target: "post-erasure inventory",
        source,
    })
}

/// Load a Journey file, repairing malformed text through [`Journey::from_text`].
#[must_use]
pub fn load_journey_file(path: &Path) -> Journey {
    try_load_journey_file(path).unwrap_or_default()
}

/// Load a score file, repairing malformed text through [`Scoreboard::from_text`].
#[must_use]
pub fn load_scoreboard_file(path: &Path) -> Scoreboard {
    try_load_scoreboard_file(path).unwrap_or_default()
}

fn try_load_journey_file(path: &Path) -> io::Result<Journey> {
    match read_local_text_bounded(path, MAX_JOURNEY_FILE_BYTES) {
        Ok(text) => Ok(Journey::from_text(&text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Journey::default()),
        Err(error) => Err(error),
    }
}

fn try_load_scoreboard_file(path: &Path) -> io::Result<Scoreboard> {
    match read_local_text_bounded(path, MAX_SCOREBOARD_FILE_BYTES) {
        Ok(text) => Ok(Scoreboard::from_text(&text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Scoreboard::default()),
        Err(error) => Err(error),
    }
}

/// Persist the delta from `before` to `after`, merged onto the newest file.
///
/// Counters are applied as saturating deltas; token sets add only tokens gained
/// since `before`; daily streaks are replayed on top of the latest record. The
/// returned Journey is the merged state that was written.
pub fn persist_journey_delta(
    path: &Path,
    before: &Journey,
    after: &Journey,
) -> io::Result<Journey> {
    if before == after {
        return Ok(load_journey_file(path));
    }
    let _lock = PersistLock::acquire(path)?;
    let mut latest = try_load_journey_file(path)?;
    merge_journey_delta(before, after, &mut latest);
    atomic_write(path, latest.to_text().as_bytes())?;
    Ok(latest)
}

/// Record one score under the shared high-score rules.
///
/// Returns true only when the score was a new record and therefore written.
pub fn record_score_file(path: &Path, key: &str, score: i64) -> io::Result<bool> {
    let _lock = PersistLock::acquire(path)?;
    let mut board = try_load_scoreboard_file(path)?;
    let changed = board.record(key, score);
    if changed {
        atomic_write(path, board.to_text().as_bytes())?;
    }
    Ok(changed)
}

/// Remove a persisted state file while holding its lock.
///
/// Missing files are already forgotten and are treated as success.
pub fn remove_persisted_file(path: &Path) -> io::Result<()> {
    let _lock = PersistLock::acquire(path)?;
    #[cfg(unix)]
    let parent = open_parent_directory(path)?;
    match fs::remove_file(path) {
        Ok(()) => {
            #[cfg(unix)]
            let _sync_result = parent.sync_all();
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn orphan_temp_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let (_, _, temp_prefix) = sidecar_name_parts(path);
    let mut temps = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index == MAX_MANAGED_SIDECARS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "could not bound adjacent temporary-file cleanup for {}",
                    path.display()
                ),
            ));
        }
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !(name.starts_with(&temp_prefix) && name.ends_with(".tmp")) {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.file_type().is_file() && !metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} is not a removable state temporary file",
                    entry.path().display()
                ),
            ));
        }
        temps.push(entry.path());
    }
    Ok(temps)
}

fn remove_orphan_temp_files(path: &Path) -> io::Result<()> {
    for temp in orphan_temp_files(path)? {
        fs::remove_file(temp)?;
    }
    Ok(())
}

/// Append one record while holding the file's persistence lock, rejecting a
/// write that would make the file exceed `max_bytes`.
pub(crate) fn append_local_file_bounded(
    path: &Path,
    bytes: &[u8],
    max_bytes: u64,
) -> io::Result<()> {
    let _lock = PersistLock::acquire(path)?;
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    let appended_bytes = u64::try_from(bytes.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "local persistence record is too large",
        )
    })?;
    let resulting_bytes = file
        .metadata()?
        .len()
        .checked_add(appended_bytes)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "local persistence file size overflow",
            )
        })?;
    if resulting_bytes > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "local persistence file is too large",
        ));
    }
    file.write_all(bytes)
}

fn merge_journey_delta(before: &Journey, after: &Journey, latest: &mut Journey) {
    for id in after.visited.difference(&before.visited) {
        latest.visit(id);
    }
    for id in after.chosen.difference(&before.chosen) {
        latest.chosen.insert(id.clone());
    }
    latest.wins = latest
        .wins
        .saturating_add(after.wins.saturating_sub(before.wins));
    latest.secrets = latest
        .secrets
        .saturating_add(after.secrets.saturating_sub(before.secrets));
    latest.plays = latest
        .plays
        .saturating_add(after.plays.saturating_sub(before.plays));
    // Only advance the daily record. `record_daily` is not monotone (it resets
    // the streak whenever the day is not exactly the next one), so replaying a
    // stale or out-of-order delta whose day is at or behind what another writer
    // already recorded would move `last_daily` backward and destroy a longer
    // streak. Guarding on strict advance keeps the record monotone.
    if after.last_daily > latest.last_daily {
        let _ = latest.record_daily(after.last_daily);
    }
}

fn read_local_text_bounded(path: &Path, max_bytes: u64) -> io::Result<String> {
    let file = File::open(path)?;
    if file
        .metadata()
        .map(|metadata| metadata.len() > max_bytes)
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "local persistence file is too large",
        ));
    }
    let mut text = String::new();
    file.take(max_bytes + 1).read_to_string(&mut text)?;
    if text.len() as u64 > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "local persistence file is too large",
        ));
    }
    Ok(text)
}

struct PersistLock {
    path: PathBuf,
    token: String,
    armed: bool,
}

struct PendingPersistLock {
    path: PathBuf,
    token: String,
    armed: bool,
}

struct PendingTempFile {
    path: PathBuf,
    armed: bool,
}

impl PendingTempFile {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PendingTempFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

impl PendingPersistLock {
    fn new(path: PathBuf, token: String) -> Self {
        Self {
            path,
            token,
            armed: true,
        }
    }

    fn into_lock(mut self) -> PersistLock {
        self.armed = false;
        PersistLock {
            path: self.path.clone(),
            token: self.token.clone(),
            armed: true,
        }
    }
}

impl Drop for PendingPersistLock {
    fn drop(&mut self) {
        if self.armed {
            remove_lock_if_owned(&self.path, &self.token);
        }
    }
}

impl PersistLock {
    fn acquire(path: &Path) -> io::Result<Self> {
        ensure_parent(path)?;
        let lock_path = lock_path_for(path);
        for _ in 0..LOCK_RETRIES {
            let recovery_path = recovery_path_for(&lock_path);
            if recovery_path.exists() {
                let _ = recover_stale_marker(&recovery_path);
                thread::sleep(LOCK_SLEEP);
                continue;
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    let token = lock_token();
                    if let Err(error) = writeln!(file, "token {token}") {
                        drop(file);
                        let _ = fs::remove_file(&lock_path);
                        return Err(error);
                    }
                    let pending = PendingPersistLock::new(lock_path.clone(), token);
                    let metadata_result = (|| -> io::Result<()> {
                        writeln!(file, "pid {}", std::process::id())?;
                        writeln!(file, "started_unix_secs {}", current_unix_secs())?;
                        file.sync_all()
                    })();
                    drop(file);
                    metadata_result?;
                    if recovery_path_for(&lock_path).exists() {
                        drop(pending);
                        thread::sleep(LOCK_SLEEP);
                        continue;
                    }
                    return Ok(pending.into_lock());
                }
                Err(error) if should_retry_lock(&error) => {
                    if !recover_stale_lock(&lock_path) {
                        thread::sleep(LOCK_SLEEP);
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("timed out waiting for {}", lock_path.display()),
        ))
    }

    fn release(mut self) -> io::Result<()> {
        release_lock_if_owned(&self.path, &self.token)?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for PersistLock {
    fn drop(&mut self) {
        if self.armed {
            remove_lock_if_owned(&self.path, &self.token);
        }
    }
}

struct RecoveryMarker {
    path: PathBuf,
}

impl RecoveryMarker {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for RecoveryMarker {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn lock_path_for(path: &Path) -> PathBuf {
    let mut name = file_name(path);
    name.push(".lock");
    path.with_file_name(name)
}

fn recovery_path_for(lock_path: &Path) -> PathBuf {
    let mut name = file_name(lock_path);
    name.push(".recover");
    lock_path.with_file_name(name)
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(file_name(path));
    name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    path.with_file_name(name)
}

fn file_name(path: &Path) -> OsString {
    path.file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("numinous-state"))
}

fn ensure_parent(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn should_retry_lock(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::AlreadyExists
        || (cfg!(windows) && error.kind() == io::ErrorKind::PermissionDenied)
}

fn recover_stale_lock(lock_path: &Path) -> bool {
    let recovery_path = recovery_path_for(lock_path);
    let Ok(mut marker) = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&recovery_path)
    else {
        return false;
    };
    let _guard = RecoveryMarker::new(recovery_path);
    let _ = writeln!(marker, "pid {}", std::process::id());
    let _ = writeln!(marker, "started_unix_secs {}", current_unix_secs());
    let _ = marker.sync_all();
    lock_is_recoverable(lock_path) && fs::remove_file(lock_path).is_ok()
}

fn recover_stale_marker(recovery_path: &Path) -> bool {
    lock_is_recoverable(recovery_path) && fs::remove_file(recovery_path).is_ok()
}

fn remove_lock_if_owned(lock_path: &Path, token: &str) {
    if lock_token_matches(lock_path, token) {
        let _ = fs::remove_file(lock_path);
    }
}

fn release_lock_if_owned(lock_path: &Path, token: &str) -> io::Result<()> {
    match fs::symlink_metadata(lock_path) {
        Ok(_) if lock_token_matches(lock_path, token) => fs::remove_file(lock_path),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("lock ownership changed for {}", lock_path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn lock_token_matches(lock_path: &Path, token: &str) -> bool {
    read_lock_text_lossy(lock_path)
        .map(|text| {
            text.lines()
                .any(|line| line.trim().strip_prefix("token ") == Some(token))
        })
        .unwrap_or(false)
}

fn lock_is_recoverable(lock_path: &Path) -> bool {
    let text = match read_lock_text_lossy(lock_path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::InvalidData => None,
        Err(_) => return false,
    };
    let started = text.as_ref().and_then(|text| {
        text.lines().find_map(|line| {
            line.trim()
                .strip_prefix("started_unix_secs ")
                .and_then(|value| value.parse::<u64>().ok())
        })
    });
    let age = started
        .map(|started| current_unix_secs().saturating_sub(started))
        .or_else(|| lock_modified_age_secs(lock_path));
    let pid = text.as_ref().and_then(|text| {
        text.lines().find_map(|line| {
            line.trim()
                .strip_prefix("pid ")
                .and_then(|value| value.parse::<u32>().ok())
        })
    });
    match pid {
        // The holder is confidently gone (`process_may_be_running` returns true
        // on any uncertainty, so a false is reliable and PID reuse is guarded).
        // Recover after a short grace rather than the full staleness window: a
        // hard crash leaks the lock, and the old age-first gate then blocked
        // every writer for up to 30 minutes.
        Some(pid) if !process_may_be_running(pid) => {
            age.is_some_and(|age| age >= LOCK_DEAD_PID_GRACE_SECS)
        }
        // The holder may still be alive: never steal an active lock.
        Some(_) => false,
        // No PID recorded: fall back to the staleness window alone.
        None => age.is_some_and(|age| age >= LOCK_STALE_AFTER_SECS),
    }
}

fn read_lock_text_lossy(lock_path: &Path) -> io::Result<String> {
    let file = File::open(lock_path)?;
    if file
        .metadata()
        .map(|metadata| metadata.len() > MAX_LOCK_FILE_BYTES)
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "persistence lock file is too large",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_LOCK_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_LOCK_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "persistence lock file is too large",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn lock_modified_age_secs(lock_path: &Path) -> Option<u64> {
    fs::metadata(lock_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age.as_secs())
}

fn lock_token() -> String {
    format!(
        "{}-{}-{}",
        std::process::id(),
        current_unix_secs(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(target_os = "linux")]
fn process_may_be_running(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(windows)]
fn process_may_be_running(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    let Ok(output) = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
    else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    String::from_utf8_lossy(&output.stdout).contains(&format!("\",\"{pid}\","))
}

#[cfg(target_os = "macos")]
fn process_may_be_running(pid: u32) -> bool {
    let own_pid = std::process::id();
    if pid == own_pid {
        return true;
    }
    let selection = format!("{pid},{own_pid}");
    let Ok(output) = std::process::Command::new("ps")
        .args(["-p", &selection, "-o", "pid="])
        .env("LC_ALL", "C")
        .output()
    else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    let mut own_pid_is_listed = false;
    let mut target_pid_is_listed = false;
    for listed_pid in String::from_utf8_lossy(&output.stdout)
        .split_ascii_whitespace()
        .filter_map(|value| value.parse::<u32>().ok())
    {
        own_pid_is_listed |= listed_pid == own_pid;
        target_pid_is_listed |= listed_pid == pid;
    }
    !own_pid_is_listed || target_pid_is_listed
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn process_may_be_running(_pid: u32) -> bool {
    true
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    ensure_parent(path)?;
    #[cfg(unix)]
    let parent = open_parent_directory(path)?;
    let (temp, file) = allocate_temp_file(path)?;
    #[cfg(unix)]
    {
        write_and_commit(path, &temp, file, bytes, || parent.sync_all())
    }
    #[cfg(not(unix))]
    {
        write_and_commit(path, &temp, file, bytes, || Ok(()))
    }
}

fn allocate_temp_file(path: &Path) -> io::Result<(PathBuf, File)> {
    for _ in 0..16 {
        let temp = temp_path_for(path);
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => return Ok((temp, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("could not allocate temp file for {}", path.display()),
    ))
}

fn write_and_commit(
    path: &Path,
    temp: &Path,
    mut file: File,
    bytes: &[u8],
    sync_parent: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let mut pending = PendingTempFile::new(temp.to_path_buf());
    let prepare_result = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    prepare_result?;
    replace_temp_file(path, temp)?;
    pending.disarm();
    // The rename is the commit point for callers that merge deltas. A parent
    // sync failure after that point cannot be reported as an uncommitted write:
    // retrying the same delta would apply its counters twice. Attempt the
    // metadata barrier, then preserve the committed result on failure.
    let _sync_result = sync_parent();
    Ok(())
}

fn replace_temp_file(path: &Path, temp: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        retry_atomic_replace_with(path, || fs::rename(temp, path))
    }
    #[cfg(not(windows))]
    {
        fs::rename(temp, path)
    }
}

#[cfg(unix)]
fn open_parent_directory(path: &Path) -> io::Result<File> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)
}

#[cfg(any(windows, test))]
fn should_retry_atomic_replace(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
    )
}

#[cfg(any(windows, test))]
fn retry_atomic_replace_with(
    path: &Path,
    mut replace: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    let mut last_error = None;
    for attempt in 0..16 {
        match replace() {
            Ok(()) => return Ok(()),
            Err(error) if should_retry_atomic_replace(&error) => {
                last_error = Some(error);
                if attempt + 1 < 16 {
                    thread::sleep(LOCK_SLEEP);
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("could not replace {}", path.display()),
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        Journey, LocalStateEraseSelection, LocalStatePaths, Scoreboard, erase_local_state,
        inspect_local_state, load_journey_file, load_scoreboard_file, persist_journey_delta,
        record_score_file, remove_persisted_file,
    };
    use std::fs::File;
    use std::io;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    #[cfg(windows)]
    use std::sync::{Arc, atomic::AtomicBool};
    use std::thread;
    use std::time::Duration;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct LiveChild(std::process::Child);

    impl LiveChild {
        fn spawn() -> Self {
            #[cfg(windows)]
            let mut command = {
                let mut command = Command::new("ping");
                command.args(["-n", "30", "127.0.0.1"]);
                command
            };
            #[cfg(not(windows))]
            let mut command = {
                let mut command = Command::new("sleep");
                command.arg("30");
                command
            };
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            Self(command.spawn().expect("spawn live child"))
        }

        fn id(&self) -> u32 {
            self.0.id()
        }

        fn stop_and_wait(&mut self) {
            if self.0.try_wait().expect("inspect live child").is_none() {
                self.0.kill().expect("stop live child");
            }
            self.0.wait().expect("reap live child");
        }
    }

    impl Drop for LiveChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "numinous_persistence_{name}_{}_{}.txt",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn complete_local_erasure_inventory_covers_every_managed_store() {
        let root = temp_file("complete_local_erasure").with_extension("");
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("scores.txt"),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        std::fs::create_dir_all(&paths.radio_cache).expect("fixture directories");
        std::fs::write(
            &paths.journey,
            b"visited lorenz\nwins 1\nsecrets 2\nplays 3\n",
        )
        .expect("journey fixture");
        std::fs::write(&paths.scores, b"50\tmunch seed:1 board:0\n").expect("score fixture");
        std::fs::write(&paths.cairn, b"a tester\tthere is no last prime\n").expect("cairn fixture");
        std::fs::write(paths.radio_cache.join("trance-001.wav"), b"RIFFfixture")
            .expect("radio fixture");
        std::fs::write(&paths.crash_log, b"bounded crash receipt").expect("crash fixture");
        let journey_temp = root.join(".journey.txt.999.1.tmp");
        std::fs::write(&journey_temp, b"orphaned partial state").expect("temp fixture");
        let cache_temp = root.join(".radio.999.1.tmp");
        std::fs::write(&cache_temp, b"orphaned partial track").expect("cache temp fixture");

        let before = inspect_local_state(&paths).expect("inspect complete fixture");
        assert_eq!(before.journey.rooms_entered, 1);
        assert_eq!(before.journey.wins, 1);
        assert_eq!(before.scores.entries, 1);
        assert_eq!(before.cairn.local_drafts, 1);
        assert_eq!(before.radio_cache.files, 1);
        assert_eq!(before.radio_cache.unexpected_entries, 0);
        assert_eq!(before.radio_cache.sidecar_files, 1);
        assert_eq!(before.journey.file.sidecar_files, 1);
        assert!(before.total_managed_bytes() > 0);

        let after = erase_local_state(&paths, LocalStateEraseSelection::complete())
            .expect("complete erasure");
        assert_eq!(after.total_managed_bytes(), 0);
        assert_eq!(after.managed_residue_count(), 0);
        for path in [
            &paths.journey,
            &paths.scores,
            &paths.cairn,
            &paths.radio_cache,
            &paths.crash_log,
        ] {
            assert!(!path.exists(), "{} must be absent", path.display());
        }
        assert!(!journey_temp.exists(), "orphan state temp must be absent");
        assert!(!cache_temp.exists(), "orphan cache temp must be absent");
        let _ = std::fs::remove_dir(&root);
    }

    #[test]
    fn local_inventory_reports_absolute_resolved_paths() {
        let relative_root = PathBuf::from(format!(
            ".numinous_inventory_paths_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let paths = LocalStatePaths {
            journey: relative_root.join("journey.txt"),
            scores: relative_root.join("scores.txt"),
            cairn: relative_root.join("cairn.txt"),
            radio_cache: relative_root.join("radio"),
            crash_log: relative_root.join("crash.log"),
        };

        let inventory = inspect_local_state(&paths).expect("inspect relative paths");

        for path in [
            &inventory.journey.file.path,
            &inventory.scores.file.path,
            &inventory.cairn.file.path,
            &inventory.radio_cache.path,
            &inventory.crash_log.path,
        ] {
            assert!(path.is_absolute(), "{} must be absolute", path.display());
        }
        assert!(!relative_root.exists(), "inventory must not create state");
    }

    #[test]
    fn radio_cache_inventory_includes_and_releases_transaction_sidecars() {
        let root = temp_file("cache_sidecar_inventory").with_extension("");
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("scores.txt"),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        let guard = super::lock_local_state(&paths.radio_cache).expect("cache writer lock");

        let while_locked = inspect_local_state(&paths).expect("inspect held cache lock");
        assert_eq!(while_locked.radio_cache.sidecar_files, 1);
        assert!(while_locked.radio_cache.sidecar_bytes > 0);
        assert_eq!(while_locked.managed_residue_count(), 1);

        guard.release().expect("checked cache lock release");
        let released = inspect_local_state(&paths).expect("inspect released cache lock");
        assert_eq!(released.radio_cache.sidecar_files, 0);
        assert_eq!(released.radio_cache.sidecar_bytes, 0);
        assert_eq!(released.managed_residue_count(), 0);
        std::fs::remove_dir(&root).expect("fixture cleanup");
    }

    #[test]
    fn cache_erasure_fails_closed_on_a_non_directory_root() {
        let root = temp_file("unexpected_cache_entry").with_extension("");
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("scores.txt"),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        std::fs::create_dir_all(&root).expect("fixture root");
        std::fs::write(&paths.radio_cache, b"not a directory").expect("unexpected cache root");
        let error = erase_local_state(&paths, LocalStateEraseSelection::complete())
            .expect_err("non-directory cache root must block deletion");
        assert_eq!(error.target(), "radio cache");
        assert!(paths.radio_cache.is_file());
        std::fs::remove_dir_all(&root).expect("fixture cleanup");
    }

    #[test]
    fn cache_erasure_preserves_unrecognized_regular_files() {
        let root = temp_file("unexpected_cache_file").with_extension("");
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("scores.txt"),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        std::fs::create_dir_all(&paths.radio_cache).expect("cache fixture");
        let unexpected = paths.radio_cache.join("notes.txt");
        std::fs::write(&unexpected, b"not generated audio").expect("unexpected fixture");
        let error = erase_local_state(&paths, LocalStateEraseSelection::complete())
            .expect_err("unrecognized file must block cache deletion");
        assert_eq!(error.target(), "radio cache");
        assert_eq!(
            std::fs::read(&unexpected).expect("unexpected file remains"),
            b"not generated audio"
        );
        std::fs::remove_dir_all(&root).expect("fixture cleanup");
    }

    #[test]
    fn overlapping_managed_paths_fail_before_any_erasure() {
        let root = temp_file("overlapping_paths").with_extension("");
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("radio").join("trance-001.wav"),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        std::fs::create_dir_all(&paths.radio_cache).expect("fixture cache");
        std::fs::write(&paths.journey, b"plays 1\n").expect("journey fixture");
        std::fs::write(&paths.scores, b"score fixture").expect("overlap fixture");
        let error = erase_local_state(
            &paths,
            LocalStateEraseSelection {
                journey: true,
                scores: false,
                cairn: false,
                radio_cache: true,
                crash_log: false,
            },
        )
        .expect_err("overlap must fail before deletion");
        assert_eq!(error.target(), "local-state path layout");
        assert!(
            paths.journey.exists(),
            "selected journey remains on failure"
        );
        assert!(paths.scores.exists(), "unselected score alias remains");
        std::fs::remove_dir_all(&root).expect("fixture cleanup");
    }

    #[test]
    fn duplicate_file_store_paths_fail_before_any_erasure() {
        let root = temp_file("duplicate_paths").with_extension("");
        std::fs::create_dir_all(&root).expect("fixture root");
        let shared = root.join("shared.txt");
        std::fs::write(&shared, b"plays 1\n").expect("shared fixture");
        let paths = LocalStatePaths {
            journey: shared.clone(),
            scores: shared.clone(),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        let error = erase_local_state(
            &paths,
            LocalStateEraseSelection {
                journey: true,
                scores: false,
                cairn: false,
                radio_cache: false,
                crash_log: false,
            },
        )
        .expect_err("duplicate stores must fail before deletion");
        assert_eq!(error.target(), "local-state path layout");
        assert_eq!(
            std::fs::read(&shared).expect("shared state remains"),
            b"plays 1\n"
        );
        std::fs::remove_dir_all(&root).expect("fixture cleanup");
    }

    #[test]
    fn complete_erasure_preflights_every_store_before_mutation() {
        let root = temp_file("complete_preflight").with_extension("");
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("scores.txt"),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        std::fs::create_dir_all(&paths.journey).expect("invalid Journey object");
        std::fs::write(&paths.scores, b"50\tfixture\n").expect("score fixture");
        std::fs::write(&paths.cairn, b"Ada\ttruth\n").expect("Cairn fixture");
        std::fs::create_dir(&paths.radio_cache).expect("cache fixture");
        std::fs::write(paths.radio_cache.join("trance-001.wav"), b"RIFF").expect("radio fixture");
        std::fs::write(&paths.crash_log, b"diagnostic").expect("crash fixture");

        let error = erase_local_state(&paths, LocalStateEraseSelection::complete())
            .expect_err("invalid Journey object blocks the transaction");
        assert_eq!(error.target(), "journey");
        assert!(paths.journey.is_dir());
        assert_eq!(
            std::fs::read(&paths.scores).expect("scores remain"),
            b"50\tfixture\n"
        );
        assert_eq!(
            std::fs::read(&paths.cairn).expect("Cairn remains"),
            b"Ada\ttruth\n"
        );
        assert!(paths.radio_cache.join("trance-001.wav").is_file());
        assert_eq!(
            std::fs::read(&paths.crash_log).expect("crash log remains"),
            b"diagnostic"
        );
        std::fs::remove_dir_all(&root).expect("fixture cleanup");
    }

    #[test]
    fn cache_erasure_waits_for_the_shared_writer_lock() {
        let root = temp_file("cache_writer_lock").with_extension("");
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("scores.txt"),
            cairn: root.join("cairn.txt"),
            radio_cache: root.join("radio"),
            crash_log: root.join("crash.log"),
        };
        std::fs::create_dir_all(&paths.radio_cache).expect("cache fixture");
        std::fs::write(paths.radio_cache.join("trance-001.wav"), b"RIFF").expect("track fixture");
        let guard = super::lock_local_state(&paths.radio_cache).expect("writer lock");
        let worker_paths = paths.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            started_tx.send(()).expect("signal start");
            let result = erase_local_state(
                &worker_paths,
                LocalStateEraseSelection {
                    journey: false,
                    scores: false,
                    cairn: false,
                    radio_cache: true,
                    crash_log: false,
                },
            );
            done_tx.send(result).expect("report erasure");
        });
        started_rx.recv().expect("worker started");
        assert!(
            done_rx.recv_timeout(Duration::from_millis(25)).is_err(),
            "erasure must wait while a writer owns the cache"
        );
        drop(guard);
        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("erasure resumes")
            .expect("cache erasure succeeds");
        worker.join().expect("worker joined");
        assert!(!paths.radio_cache.exists());
        std::fs::remove_dir(&root).expect("fixture cleanup");
    }

    fn exited_process_id() -> u32 {
        #[cfg(windows)]
        let mut child = Command::new("cmd")
            .args(["/C", "exit 0"])
            .spawn()
            .expect("spawn exited child");
        #[cfg(not(windows))]
        let mut child = Command::new("sh")
            .args(["-c", "true"])
            .spawn()
            .expect("spawn exited child");
        let pid = child.id();
        child.wait().expect("wait child");
        pid
    }

    #[test]
    fn journey_deltas_merge_against_latest_file() {
        let path = temp_file("journey_merge");
        let base = Journey::default();
        let mut first = base.clone();
        first.visit("lorenz");
        first.play();
        persist_journey_delta(&path, &base, &first).expect("first write");

        let mut second = base.clone();
        second.visit("julia");
        second.win();
        let merged = persist_journey_delta(&path, &base, &second).expect("second write");

        assert!(merged.visited.contains("lorenz"));
        assert!(merged.visited.contains("julia"));
        assert_eq!(merged.plays, 1);
        assert_eq!(merged.wins, 1);
        assert_eq!(load_journey_file(&path), merged);
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn stale_journey_deltas_do_not_resurrect_forgotten_state() {
        let path = temp_file("journey_forget");
        let mut old = Journey::default();
        old.visit("lorenz");
        old.plays = 5;
        persist_journey_delta(&path, &Journey::default(), &old).expect("old write");
        remove_persisted_file(&path).expect("forget");

        let mut after = old.clone();
        after.visit("julia");
        after.play();
        let merged = persist_journey_delta(&path, &old, &after).expect("new delta");

        assert!(!merged.visited.contains("lorenz"));
        assert!(merged.visited.contains("julia"));
        assert_eq!(merged.plays, 1);
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn concurrent_journey_deltas_do_not_lose_updates() {
        let path = temp_file("journey_concurrent");
        let base = Journey::default();
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                let base = base.clone();
                thread::spawn(move || {
                    let mut after = base.clone();
                    after.visit(&format!("room-{i}"));
                    after.play();
                    persist_journey_delta(&path, &base, &after).expect("persist delta");
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("thread joined");
        }

        let merged = load_journey_file(&path);
        assert_eq!(merged.visited.len(), 8);
        assert_eq!(merged.plays, 8);
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn score_records_merge_under_lock() {
        let path = temp_file("scores_merge");
        assert!(record_score_file(&path, "munch seed:1 board:0", 20).expect("write"));
        assert!(!record_score_file(&path, "munch seed:1 board:0", 10).expect("worse"));
        assert!(record_score_file(&path, "quiz seed:2 rounds:3", 2).expect("second key"));

        let board = load_scoreboard_file(&path);
        assert_eq!(board.entries["munch seed:1 board:0"], 20);
        assert_eq!(board.entries["quiz seed:2 rounds:3"], 2);
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn stale_lock_file_is_recovered_before_write() {
        let path = temp_file("stale_lock");
        let lock = super::lock_path_for(&path);
        let stale_started = super::current_unix_secs()
            .saturating_sub(super::LOCK_STALE_AFTER_SECS)
            .saturating_sub(1);
        std::fs::write(&lock, format!("started_unix_secs {stale_started}\n")).expect("stale lock");

        assert!(record_score_file(&path, "key", 1).expect("write after stale lock"));

        assert_eq!(load_scoreboard_file(&path).entries["key"], 1);
        assert!(!lock.exists());
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn dead_process_stale_lock_file_is_recovered_before_write() {
        let path = temp_file("dead_process_stale_lock");
        let lock = super::lock_path_for(&path);
        let stale_started = super::current_unix_secs()
            .saturating_sub(super::LOCK_STALE_AFTER_SECS)
            .saturating_sub(1);
        std::fs::write(
            &lock,
            format!(
                "pid {}\ntoken stale\nstarted_unix_secs {stale_started}\n",
                exited_process_id()
            ),
        )
        .expect("dead process stale lock");

        assert!(record_score_file(&path, "key", 1).expect("write after dead stale lock"));

        assert_eq!(load_scoreboard_file(&path).entries["key"], 1);
        assert!(!lock.exists());
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn recent_dead_process_lock_is_recovered_without_the_full_stale_wait() {
        let path = temp_file("recent_dead_lock");
        let lock = super::lock_path_for(&path);
        // A lock only seconds old, well under the 30-minute staleness window, but
        // held by a dead process, must recover after the short grace so a hard
        // crash does not block writers for half an hour.
        let recent_started = super::current_unix_secs()
            .saturating_sub(super::LOCK_DEAD_PID_GRACE_SECS)
            .saturating_sub(1);
        std::fs::write(
            &lock,
            format!(
                "pid {}\ntoken stale\nstarted_unix_secs {recent_started}\n",
                exited_process_id()
            ),
        )
        .expect("recent dead lock");

        assert!(record_score_file(&path, "key", 1).expect("write after recent dead lock"));
        assert_eq!(load_scoreboard_file(&path).entries["key"], 1);
        assert!(!lock.exists());
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn a_stale_daily_delta_does_not_move_the_streak_backward() {
        use crate::journey::Journey;
        // latest already recorded day 101 with a streak; a stale or out-of-order
        // delta carrying an earlier day must not regress last_daily or reset the
        // streak another writer already earned.
        let mut latest = Journey::default();
        let _ = latest.record_daily(100);
        let _ = latest.record_daily(101);
        let (day_before, streak_before) = (latest.last_daily, latest.streak);
        assert!(streak_before >= 2, "the setup earns a streak to protect");

        let before = Journey::default();
        let mut after = Journey::default();
        let _ = after.record_daily(100);

        super::merge_journey_delta(&before, &after, &mut latest);
        assert_eq!(latest.last_daily, day_before, "the day must not regress");
        assert_eq!(latest.streak, streak_before, "the streak must be preserved");
    }

    #[test]
    fn stale_recovery_marker_is_removed_before_acquire() {
        let path = temp_file("stale_recovery_marker");
        let lock = super::lock_path_for(&path);
        let marker = super::recovery_path_for(&lock);
        let stale_started = super::current_unix_secs()
            .saturating_sub(super::LOCK_STALE_AFTER_SECS)
            .saturating_sub(1);
        std::fs::write(&marker, format!("started_unix_secs {stale_started}\n"))
            .expect("stale recovery marker");

        assert!(record_score_file(&path, "key", 1).expect("write after stale marker"));

        assert_eq!(load_scoreboard_file(&path).entries["key"], 1);
        assert!(!marker.exists());
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn current_process_stale_lock_is_not_recovered() {
        let path = temp_file("current_process_lock");
        let lock = super::lock_path_for(&path);
        let stale_started = super::current_unix_secs()
            .saturating_sub(super::LOCK_STALE_AFTER_SECS)
            .saturating_sub(1);
        std::fs::write(
            &lock,
            format!(
                "pid {}\ntoken active\nstarted_unix_secs {stale_started}\n",
                std::process::id()
            ),
        )
        .expect("current process lock");

        assert!(!super::recover_stale_lock(&lock));
        assert!(lock.exists());

        std::fs::remove_file(lock).expect("cleanup lock");
    }

    #[test]
    fn foreign_live_process_stale_lock_is_not_recovered() {
        let path = temp_file("foreign_process_lock");
        let lock = super::lock_path_for(&path);
        let mut child = LiveChild::spawn();
        let stale_started = super::current_unix_secs()
            .saturating_sub(super::LOCK_STALE_AFTER_SECS)
            .saturating_sub(1);
        std::fs::write(
            &lock,
            format!(
                "pid {}\ntoken active\nstarted_unix_secs {stale_started}\n",
                child.id()
            ),
        )
        .expect("foreign process lock");

        assert!(!super::recover_stale_lock(&lock));
        assert!(lock.exists());

        child.stop_and_wait();
        std::fs::remove_file(lock).expect("cleanup lock");
    }

    #[test]
    fn lock_drop_does_not_remove_replacement_lock() {
        let path = temp_file("replacement_lock");
        let lock_path = super::lock_path_for(&path);
        let old_lock = super::PersistLock {
            path: lock_path.clone(),
            token: "old-token".to_string(),
            armed: true,
        };
        std::fs::write(&lock_path, "token replacement-token\n").expect("replacement lock");

        drop(old_lock);

        assert_eq!(
            std::fs::read_to_string(&lock_path).expect("replacement remains"),
            "token replacement-token\n"
        );
        std::fs::remove_file(lock_path).expect("cleanup replacement lock");
    }

    #[test]
    fn lock_drop_does_not_read_unbounded_replacement_lock() {
        let path = temp_file("oversized_replacement_lock");
        let lock_path = super::lock_path_for(&path);
        let old_lock = super::PersistLock {
            path: lock_path.clone(),
            token: "old-token".to_string(),
            armed: true,
        };
        let file = std::fs::File::create(&lock_path).expect("oversized replacement lock");
        file.set_len(super::MAX_LOCK_FILE_BYTES + 1)
            .expect("make oversized lock");

        drop(old_lock);

        assert_eq!(
            std::fs::metadata(&lock_path)
                .expect("replacement metadata")
                .len(),
            super::MAX_LOCK_FILE_BYTES + 1
        );
        std::fs::remove_file(lock_path).expect("cleanup replacement lock");
    }

    #[test]
    fn malformed_stale_lock_file_is_recovered_before_write() {
        let path = temp_file("malformed_stale_lock");
        let lock = super::lock_path_for(&path);
        let stale_started = super::current_unix_secs()
            .saturating_sub(super::LOCK_STALE_AFTER_SECS)
            .saturating_sub(1);
        let mut bytes = format!("started_unix_secs {stale_started}\npid\n").into_bytes();
        bytes.push(0xFF);
        std::fs::write(&lock, bytes).expect("malformed stale lock");

        assert!(record_score_file(&path, "key", 1).expect("write after malformed stale lock"));

        assert_eq!(load_scoreboard_file(&path).entries["key"], 1);
        assert!(!lock.exists());
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn concurrent_score_records_do_not_lose_keys() {
        let path = temp_file("scores_concurrent");
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                thread::spawn(move || {
                    record_score_file(&path, &format!("key:{i}"), i as i64).expect("record score");
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("thread joined");
        }

        let board = load_scoreboard_file(&path);
        assert_eq!(board.entries.len(), 8);
        assert_eq!(board.entries["key:7"], 7);
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn score_writer_waits_for_a_short_held_lock() {
        let path = temp_file("scores_waits_for_lock");
        let held_lock = super::PersistLock::acquire(&path).expect("hold lock");
        let writer_path = path.clone();
        let writer = thread::spawn(move || {
            record_score_file(&writer_path, "delayed", 1).expect("wait for lock");
        });

        thread::sleep(Duration::from_millis(700));
        drop(held_lock);
        writer.join().expect("writer joined");

        let board = load_scoreboard_file(&path);
        assert_eq!(board.entries["delayed"], 1);
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn atomic_replace_retries_without_hiding_the_destination() {
        let path = temp_file("atomic_replace_destination");
        let temp = temp_file("atomic_replace_source");
        std::fs::write(&path, b"old").expect("old destination");
        std::fs::write(&temp, b"new").expect("new temp");
        let mut attempts = 0;

        super::retry_atomic_replace_with(&path, || {
            assert_eq!(
                std::fs::read(&path).expect("destination remains readable"),
                b"old"
            );
            attempts += 1;
            if attempts < 3 {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "simulated transient sharing violation",
                ))
            } else {
                std::fs::rename(&temp, &path)
            }
        })
        .expect("atomic replace eventually succeeds");

        assert_eq!(attempts, 3);
        assert_eq!(std::fs::read(&path).expect("new destination"), b"new");
        assert!(!temp.exists());
        remove_persisted_file(&path).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn windows_sharing_retry_keeps_the_destination_readable() {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_SHARE_READ: u32 = 0x0000_0001;
        const FILE_SHARE_WRITE: u32 = 0x0000_0002;

        let path = temp_file("windows_sharing_destination");
        let temp = temp_file("windows_sharing_source");
        std::fs::write(&path, b"old").expect("old destination");
        std::fs::write(&temp, b"new").expect("new temp");
        let mut blocker = Some(
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .open(&path)
                .expect("open handle that denies replacement"),
        );

        let stop = Arc::new(AtomicBool::new(false));
        let missing = Arc::new(AtomicBool::new(false));
        let reads = Arc::new(AtomicUsize::new(0));
        let (ready_tx, ready_rx) = mpsc::channel();
        let reader_path = path.clone();
        let reader_stop = Arc::clone(&stop);
        let reader_missing = Arc::clone(&missing);
        let reader_reads = Arc::clone(&reads);
        let reader = thread::spawn(move || {
            let mut ready_tx = Some(ready_tx);
            while !reader_stop.load(Ordering::Relaxed) {
                match std::fs::read(&reader_path) {
                    Ok(bytes) => {
                        assert!(bytes == b"old" || bytes == b"new");
                        reader_reads.fetch_add(1, Ordering::Relaxed);
                        if let Some(sender) = ready_tx.take() {
                            sender.send(()).expect("signal first read");
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        reader_missing.store(true, Ordering::Relaxed);
                    }
                    Err(error) => panic!("unexpected concurrent read error: {error}"),
                }
                thread::yield_now();
            }
        });
        ready_rx.recv().expect("reader observes old destination");
        let mut attempts = 0;

        super::retry_atomic_replace_with(&path, || {
            attempts += 1;
            let result = std::fs::rename(&temp, &path);
            if attempts == 1 {
                assert!(result.is_err(), "sharing handle must block first replace");
                drop(blocker.take());
            }
            result
        })
        .expect("replace after sharing handle closes");
        stop.store(true, Ordering::Relaxed);
        reader.join().expect("reader joined");

        assert!(attempts >= 2);
        assert!(reads.load(Ordering::Relaxed) > 0);
        assert!(!missing.load(Ordering::Relaxed));
        assert_eq!(std::fs::read(&path).expect("new destination"), b"new");
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn failed_precommit_write_removes_its_temp_file() {
        let path = temp_file("failed_write_destination");
        let temp = temp_file("failed_write_temp");
        std::fs::write(&path, b"old").expect("old destination");
        std::fs::write(&temp, b"partial").expect("temp placeholder");
        let read_only = File::open(&temp).expect("open temp without write access");

        let _error = super::write_and_commit(&path, &temp, read_only, b"new", || Ok(()))
            .expect_err("read-only temp must reject the write");

        assert_eq!(std::fs::read(&path).expect("old destination"), b"old");
        assert!(!temp.exists(), "failed temp must not become an orphan");
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn postcommit_sync_failure_does_not_turn_a_committed_delta_into_an_error() {
        let path = temp_file("postcommit_sync_destination");
        let temp = temp_file("postcommit_sync_temp");
        let base = Journey::default();
        let mut first = base.clone();
        first.play();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .expect("create temp");

        super::write_and_commit(&path, &temp, file, first.to_text().as_bytes(), || {
            Err(io::Error::other("simulated postcommit sync failure"))
        })
        .expect("the visible commit remains successful");

        let mut second = first.clone();
        second.play();
        let merged = persist_journey_delta(&path, &first, &second).expect("next delta");
        assert_eq!(merged.plays, 2, "the committed first delta is not replayed");
        remove_persisted_file(&path).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn test_filesystem_accepts_parent_directory_sync() {
        let path = temp_file("parent_sync_support");
        std::fs::write(&path, b"state").expect("state file");
        super::open_parent_directory(&path)
            .expect("open parent directory")
            .sync_all()
            .expect("sync parent directory");
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn pending_lock_drop_removes_the_owned_error_path_lock() {
        let path = temp_file("pending_lock_cleanup");
        let lock = super::lock_path_for(&path);
        let token = "pending-token".to_string();
        std::fs::write(&lock, format!("token {token}\n")).expect("pending lock");

        drop(super::PendingPersistLock::new(lock.clone(), token));

        assert!(!lock.exists());
    }

    #[test]
    fn oversized_journey_file_loads_as_default_but_write_preserves_file() {
        let path = temp_file("journey_oversized");
        let file = std::fs::File::create(&path).expect("oversized placeholder");
        file.set_len(super::MAX_JOURNEY_FILE_BYTES + 1)
            .expect("make sparse oversized journey");

        assert_eq!(load_journey_file(&path), Journey::default());

        let base = Journey::default();
        let mut after = Journey::default();
        after.play();
        let error =
            persist_journey_delta(&path, &base, &after).expect_err("oversized is not overwritten");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::metadata(&path).expect("metadata").len(),
            super::MAX_JOURNEY_FILE_BYTES + 1
        );
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn oversized_score_file_loads_as_default_but_write_preserves_file() {
        let path = temp_file("scores_oversized");
        let file = std::fs::File::create(&path).expect("oversized placeholder");
        file.set_len(super::MAX_SCOREBOARD_FILE_BYTES + 1)
            .expect("make sparse oversized scoreboard");

        assert_eq!(load_scoreboard_file(&path), Scoreboard::default());
        let error =
            record_score_file(&path, "munch seed:1 board:0", 20).expect_err("oversized preserved");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::metadata(&path).expect("metadata").len(),
            super::MAX_SCOREBOARD_FILE_BYTES + 1
        );
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn invalid_utf8_persistence_file_is_not_overwritten_from_default() {
        let path = temp_file("journey_invalid_utf8");
        std::fs::write(&path, b"plays=9\nvisited=lorenz\n\xFF").expect("invalid utf8");
        let base = Journey::default();
        let mut after = Journey::default();
        after.play();

        let error =
            persist_journey_delta(&path, &base, &after).expect_err("invalid utf8 blocks write");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::read(&path).expect("original bytes"),
            b"plays=9\nvisited=lorenz\n\xFF"
        );
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn invalid_utf8_score_file_is_not_overwritten_from_default() {
        let path = temp_file("score_invalid_utf8");
        std::fs::write(&path, b"key\t9\n\xFF").expect("invalid utf8 score");

        let error = record_score_file(&path, "munch seed:1 board:0", 20)
            .expect_err("invalid utf8 blocks score write");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::read(&path).expect("original bytes"),
            b"key\t9\n\xFF"
        );
        remove_persisted_file(&path).expect("cleanup");
    }

    #[test]
    fn remove_persisted_file_is_idempotent() {
        let path = temp_file("remove");
        record_score_file(&path, "key", 1).expect("write score");
        assert_ne!(load_scoreboard_file(&path), Scoreboard::default());
        remove_persisted_file(&path).expect("first remove");
        remove_persisted_file(&path).expect("second remove");
        assert_eq!(load_scoreboard_file(&path), Scoreboard::default());
    }
}
