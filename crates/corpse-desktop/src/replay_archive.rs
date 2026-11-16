//! Human-readable replay names. Filesystem time never enters simulation state.
use std::{io::Write, path::{Path, PathBuf}};
use corpse_core::Run;

fn stem(run: &Run, timestamp: &str) -> Result<String, String> {
    let date = timestamp.get(..10).ok_or("Replay date must be YYYY-MM-DD")?;
    if !date.bytes().enumerate().all(|(i, b)| if i == 4 || i == 7 { b == b'-' } else { b.is_ascii_digit() }) {
        return Err("Replay date must be YYYY-MM-DD".into());
    }
    let depth = if run.status == "won" { 0. } else { corpse_core::fast_mode::display_depth(run,run.depth) };
    Ok(format!("{date}-{:.0}-{depth:.0}", run.run_stats.as_ref().map_or(0., |stats| stats.blessings_received)))
}

/// Serialize elsewhere once. O(B + D) I/O for B bytes and D occupied names;
/// O(1) extra memory beyond the borrowed payload. Never replace an existing file.
pub fn write(directory: &Path, run: &Run, timestamp: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let stem = stem(run, timestamp)?;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    // Finish and sync a sibling temporary file before atomically publishing it.
    // hard_link is create-only on Windows and Unix, unlike rename's replacement.
    let (temp, mut file) = (1u64..).find_map(|n| {
        let temp = directory.join(format!(".{stem}.pending-{}-{n}", std::process::id()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => Some(Ok((temp, file))),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => None,
            Err(e) => Some(Err(e.to_string())),
        }
    }).ok_or("Replay temporary-name limit reached")??;
    let result = (|| {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        for n in 1u64.. {
            let name = if n == 1 { format!("{stem}.json") } else { format!("{stem}-{n}.json") };
            let path = directory.join(name);
            match std::fs::hard_link(&temp, &path) {
                Ok(()) => return Ok(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("Could not publish replay: {e}")),
            }
        }
        Err("Replay name limit reached".into())
    })();
    let _ = std::fs::remove_file(temp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run() -> Run {
        let mut run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut stats = corpse_core::model::RunStatistics::default();
        stats.blessings_received = 12.; run.run_stats = stats.into();
        run.depth = 17.; run
    }
    #[test]
    fn replay_names_use_sortable_date_blessings_and_terminal_depth() {
        let mut run = run();
        for status in ["playing", "dead"] {
            run.status = status.into();
            assert_eq!(stem(&run, "2026-11-10T23:59:59.000Z").unwrap(), "2026-11-10-12-17");
        }
        run.status = "won".into();
        assert_eq!(stem(&run, "2026-11-10T00:00:00.000Z").unwrap(), "2026-11-10-12-0");
        assert!(stem(&run, "../../bad/").is_err());
    }
    #[test]
    fn replay_duplicate_names_preserve_existing_and_legacy_bytes() {
        let dir = std::env::temp_dir().join(format!("corpse-replay-names-{}-{}", std::process::id(), super::super::utc_now().replace(':', "")));
        std::fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join("descent-legacy.json");
        std::fs::write(&legacy, b"legacy").unwrap();
        let run = run();
        let first = write(&dir, &run, "2026-11-10", b"first").unwrap();
        let second = write(&dir, &run, "2026-11-10", b"second").unwrap();
        let third = write(&dir, &run, "2026-11-10", b"third").unwrap();
        assert_eq!(first.file_name().unwrap(), "2026-11-10-12-17.json");
        assert_eq!(second.file_name().unwrap(), "2026-11-10-12-17-2.json");
        assert_eq!(third.file_name().unwrap(), "2026-11-10-12-17-3.json");
        assert_eq!(std::fs::read(first).unwrap(), b"first");
        assert_eq!(std::fs::read(second).unwrap(), b"second");
        assert_eq!(std::fs::read(legacy).unwrap(), b"legacy");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 4);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
