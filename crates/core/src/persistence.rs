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
const MAX_JOURNEY_FILE_BYTES: u64 = 64 * 1024;
const MAX_LOCK_FILE_BYTES: u64 = 4 * 1024;
const MAX_SCOREBOARD_FILE_BYTES: u64 = 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
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
    if after.last_daily != before.last_daily && after.last_daily != 0 {
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
}

struct PendingPersistLock {
    path: PathBuf,
    token: String,
    armed: bool,
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
                        let _ = fs::remove_file(&lock_path);
                        return Err(error);
                    }
                    let pending = PendingPersistLock::new(lock_path.clone(), token);
                    writeln!(file, "pid {}", std::process::id())?;
                    writeln!(file, "started_unix_secs {}", current_unix_secs())?;
                    file.sync_all()?;
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
}

impl Drop for PersistLock {
    fn drop(&mut self) {
        remove_lock_if_owned(&self.path, &self.token);
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

#[cfg(windows)]
fn backup_path_for(path: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(file_name(path));
    name.push(format!(
        ".{}.{}.bak",
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
    if age.is_none_or(|age| age < LOCK_STALE_AFTER_SECS) {
        return false;
    }
    let Some(pid) = text.as_ref().and_then(|text| {
        text.lines().find_map(|line| {
            line.trim()
                .strip_prefix("pid ")
                .and_then(|value| value.parse::<u32>().ok())
        })
    }) else {
        return true;
    };
    !process_may_be_running(pid)
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

#[cfg(not(any(target_os = "linux", windows)))]
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
    for _ in 0..16 {
        let temp = temp_path_for(path);
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => {
                write_and_commit(path, &temp, file, bytes)?;
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("could not allocate temp file for {}", path.display()),
    ))
}

fn write_and_commit(path: &Path, temp: &Path, mut file: File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    match fs::rename(temp, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            #[cfg(windows)]
            if should_retry_windows_replace(&error) {
                return replace_existing_windows(path, temp);
            }
            let _ = fs::remove_file(temp);
            Err(error)
        }
    }
}

#[cfg(windows)]
fn should_retry_windows_replace(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
    )
}

#[cfg(windows)]
fn replace_existing_windows(path: &Path, temp: &Path) -> io::Result<()> {
    let mut last_error = None;
    for _ in 0..16 {
        let backup = backup_path_for(path);
        match fs::rename(path, &backup) {
            Ok(()) => match fs::rename(temp, path) {
                Ok(()) => {
                    let _ = fs::remove_file(&backup);
                    return Ok(());
                }
                Err(error) => {
                    let restore = fs::rename(&backup, path);
                    return Err(restore.err().unwrap_or(error));
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) if should_retry_windows_replace(&error) => {
                last_error = Some(error);
                thread::sleep(LOCK_SLEEP);
                continue;
            }
            Err(error) => return Err(error),
        }

        match fs::rename(temp, path) {
            Ok(()) => return Ok(()),
            Err(error) if should_retry_windows_replace(&error) => {
                last_error = Some(error);
                thread::sleep(LOCK_SLEEP);
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
        Journey, Scoreboard, load_journey_file, load_scoreboard_file, persist_journey_delta,
        record_score_file, remove_persisted_file,
    };
    use std::io;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "numinous_persistence_{name}_{}_{}.txt",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
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
    fn lock_drop_does_not_remove_replacement_lock() {
        let path = temp_file("replacement_lock");
        let lock_path = super::lock_path_for(&path);
        let old_lock = super::PersistLock {
            path: lock_path.clone(),
            token: "old-token".to_string(),
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
