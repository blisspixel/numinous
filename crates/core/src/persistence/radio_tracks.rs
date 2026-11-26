//! Naming and atomic publication of locally generated radio tracks.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

use super::{
    PendingTempFile, allocate_temp_file, expected_temp_name, lock_local_state, private_file_options,
};

const STAGE: &str = "numinous-recording";

pub(super) fn is_stage(name: &OsStr) -> bool {
    expected_temp_name(Path::new(STAGE), name).as_deref() == Some(name)
}

fn track_name(station: &crate::Station, number: usize) -> String {
    format!("{}-{number:03}.wav", station.id)
}

/// Recognize only the exact spelling produced for a positive station number.
pub(super) fn generated_track(name: &OsStr) -> Option<(&'static crate::Station, usize)> {
    let name = name.to_str()?;
    let (id, digits) = name.strip_suffix(".wav")?.rsplit_once('-')?;
    let station = crate::station(id)?;
    let number = digits.parse::<usize>().ok()?;
    (number > 0 && name == track_name(station, number)).then_some((station, number))
}

fn station_by_id(id: &str) -> io::Result<&'static crate::Station> {
    crate::station(id).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unknown station"))
}

/// Next number for choosing a station's brief. A concurrent publication may
/// change the number before the recording is ready.
///
/// # Errors
///
/// Fails on an unknown station, an unreadable directory, or exhausted numbers.
pub fn next_number(dir: &Path, id: &str) -> io::Result<usize> {
    let station = station_by_id(id)?;
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(1),
        Err(error) => return Err(error),
    };
    let mut highest = 0;
    for entry in entries {
        if let Some((found, number)) = generated_track(&entry?.file_name())
            && found.id == station.id
        {
            highest = highest.max(number);
        }
    }
    highest
        .checked_add(1)
        .ok_or_else(|| io::Error::other("station track numbers exhausted"))
}

/// Finish a recording privately and publish it at a new numbered path under
/// the same cache lock used for erasure. A crash can leave an ordinary managed
/// cache sidecar; no partially encoded file enters the playlist. Success
/// means the completed recording is visible at the returned path. Unix
/// directory synchronization is best effort, as for the other player stores.
///
/// # Errors
///
/// Fails if the recording cannot be written or published. An occupied
/// destination is preserved. A completed stage is retained on publication errors.
pub fn publish(
    dir: &Path,
    id: &str,
    write_recording: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<PathBuf> {
    let station = station_by_id(id)?;
    let _lock = lock_local_state(dir)?;
    fs::create_dir_all(dir)?;
    let mut number = next_number(dir, station.id)?;
    let (temp, mut file) = allocate_temp_file(&dir.join(STAGE))?;
    let mut pending = PendingTempFile::new(temp.clone());
    write_recording(&mut file)?;
    file.sync_all()?;
    drop(file);
    loop {
        let path = dir.join(track_name(station, number));
        let published = match fs::hard_link(&temp, &path) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => {
                copy_wav_exclusively(&temp, &path)
            }
            result => result,
        };
        match published {
            Ok(()) => {
                #[cfg(unix)]
                if let Ok(parent) = File::open(dir) {
                    let _ = parent.sync_all();
                }
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && number < usize::MAX => {
                number += 1;
            }
            Err(error) => {
                pending.disarm();
                return Err(io::Error::new(
                    error.kind(),
                    format!(
                        "could not finish {} ({error}). The complete recording remains at {}",
                        path.display(),
                        temp.display()
                    ),
                ));
            }
        }
    }
}

/// Filesystems such as FAT do not support hard links. Create the destination
/// exclusively and make its RIFF signature visible only after every payload
/// byte is durable. A power loss may leave an unplayable numbered file. Its
/// complete private stage survives until successful publication or erasure.
fn copy_wav_exclusively(temp: &Path, path: &Path) -> io::Result<()> {
    let mut source = File::open(temp)?;
    let mut magic = [0; 4];
    source.read_exact(&mut magic)?;
    if &magic != b"RIFF" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "recording lacks a RIFF header",
        ));
    }
    let mut target = private_file_options().create_new(true).open(path)?;
    copy_wav_payload(&mut source, &mut target)
}

fn copy_wav_payload(source: &mut impl Read, target: &mut File) -> io::Result<()> {
    target.write_all(&[0; 4])?;
    io::copy(source, target)?;
    target.sync_all()?;
    target.rewind()?;
    target.write_all(b"RIFF")?;
    target.sync_all()
}

#[cfg(test)]
// Stereo PCM, one sample frame, including a valid RIFF length and data chunk.
pub(super) const SAMPLE_WAV: &[u8; 48] = b"RIFF\x28\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x44\xac\0\0\x10\xb1\x02\0\x04\0\x10\0data\x04\0\0\0\x01\0\x02\0";

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn exclusive_copy_publishes_the_wav_signature_last_and_preserves_occupied_paths() {
        let root = std::env::temp_dir().join(format!("numinous-radio-copy-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let temp = root.join("prepared");
        let dest = root.join("trance-001.wav");
        let wav = SAMPLE_WAV;
        fs::write(&temp, wav).unwrap();
        fs::write(&dest, b"prior recording").unwrap();
        assert_eq!(
            copy_wav_exclusively(&temp, &dest).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&dest).unwrap(), b"prior recording");
        fs::remove_file(&dest).unwrap();
        copy_wav_exclusively(&temp, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), wav);

        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("read interrupted"))
            }
        }
        let partial = root.join("trance-002.wav");
        let mut file = File::create(&partial).unwrap();
        let mut interrupted = Cursor::new(&wav[4..24]).chain(Broken);
        assert!(copy_wav_payload(&mut interrupted, &mut file).is_err());
        drop(file);
        let partial_bytes = fs::read(partial).unwrap();
        assert_eq!(&partial_bytes[..4], &[0; 4]);
        assert_eq!(&partial_bytes[4..], &wav[4..24]);
        assert_eq!(fs::read(&temp).unwrap(), wav);
        fs::write(&temp, b"oops").unwrap();
        assert_eq!(
            copy_wav_exclusively(&temp, &root.join("other"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert!(!root.join("other").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
