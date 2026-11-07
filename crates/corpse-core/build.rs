use std::{env, fs::{self, File, OpenOptions}, io::{self, Read, Write}, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
const NAME: &str = "mini-me-starter-combinations-v48.json";
const SIZES: [u64; 2] = [67_108_864, 49_777_121];

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("../../fixtures");
    println!("cargo:rerun-if-changed=build.rs");
    for suffix in ["", ".part1", ".part2"] {
        println!("cargo:rerun-if-changed={}", fixtures.join(format!("{NAME}{suffix}")).display());
    }
    restore(&fixtures, NAME, &SIZES).unwrap_or_else(|e| panic!("Cannot restore large fixture: {e}"));
}

// O(n) time, two fixed 64KiB buffers, O(n) temporary disk only on restoration.
// Tracked parts are canonical; preserve an existing file unless all bytes match.
fn matches_parts(destination: &Path, parts: &[PathBuf]) -> io::Result<bool> {
    let mut destination = File::open(destination)?;
    let mut expected = [0u8; 65536];
    let mut actual = [0u8; 65536];
    for path in parts {
        let mut part = File::open(path)?;
        loop {
            let size = part.read(&mut expected)?;
            if size == 0 { break; }
            match destination.read_exact(&mut actual[..size]) {
                Ok(()) => {},
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(false),
                Err(e) => return Err(e),
            }
            if actual[..size] != expected[..size] { return Ok(false); }
        }
    }
    Ok(destination.read(&mut actual[..1])? == 0)
}
fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidData, message) }
fn verify_existing(destination: &Path, parts: &[PathBuf]) -> io::Result<()> {
    if matches_parts(destination, parts)? { Ok(()) }
    else { Err(invalid("existing fixture differs from tracked parts; refusing to overwrite local changes")) }
}
struct Temporary(PathBuf);
impl Drop for Temporary { fn drop(&mut self) { let _ = fs::remove_file(&self.0); } }
fn restore(directory: &Path, name: &str, sizes: &[u64]) -> io::Result<()> {
    let destination = directory.join(name);
    let parts: Vec<_> = (1..=sizes.len()).map(|i| directory.join(format!("{name}.part{i}"))).collect();
    for (part, size) in parts.iter().zip(sizes) {
        if fs::metadata(part)?.len() != *size { return Err(invalid("fixture part length mismatch")); }
    }
    match fs::symlink_metadata(&destination) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() { return Err(invalid("existing fixture is not a regular file")); }
            return verify_existing(&destination, &parts);
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {},
        Err(e) => return Err(e),
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| invalid("system time precedes epoch"))?.as_nanos();
    let temporary_path = directory.join(format!(".{name}.{}.{nonce}.tmp", std::process::id()));
    let mut output = OpenOptions::new().write(true).create_new(true).open(&temporary_path)?;
    let temporary = Temporary(temporary_path);
    for (path, size) in parts.iter().zip(sizes) {
        let mut part = File::open(path)?;
        if io::copy(&mut part, &mut output)? != *size { return Err(invalid("fixture part changed during restoration")); }
    }
    output.flush()?;
    drop(output);
    if !matches_parts(&temporary.0, &parts)? { return Err(invalid("fixture parts changed during restoration")); }
    // Same-directory hard link publishes complete bytes atomically, without overwrite.
    // No rename fallback: rename can overwrite local edits racing with this build.
    match fs::hard_link(&temporary.0, &destination) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => verify_existing(&destination, &parts),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sandbox() -> Temporary {
        let path = PathBuf::from(env::var_os("TEST_TMPDIR").expect("TEST_TMPDIR"))
            .join(format!("test-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&path).unwrap(); Temporary(path)
    }
    fn setup() -> Temporary {
        let dir = sandbox(); fs::write(dir.0.join("f.part1"), b"abc").unwrap(); fs::write(dir.0.join("f.part2"), b"def").unwrap(); dir
    }
    #[test] fn roundtrip_and_idempotence() {
        let d=setup(); restore(&d.0,"f",&[3,3]).unwrap();
        assert_eq!(fs::read(d.0.join("f")).unwrap(),b"abcdef");
        let modified=fs::metadata(d.0.join("f")).unwrap().modified().unwrap();
        restore(&d.0,"f",&[3,3]).unwrap(); assert_eq!(fs::metadata(d.0.join("f")).unwrap().modified().unwrap(),modified);
        fs::remove_dir_all(&d.0).unwrap();
    }
    #[test] fn reject_local_changes() {
        for changed in [b"abcdeg".as_slice(), b"abc", b"abcdefg"] {
            let d=setup(); fs::write(d.0.join("f"),changed).unwrap(); assert!(restore(&d.0,"f",&[3,3]).is_err());
            assert_eq!(fs::read(d.0.join("f")).unwrap(),changed); fs::remove_dir_all(&d.0).unwrap();
        }
    }
    #[test] fn reject_truncated_or_missing_parts() {
        let d=setup(); assert!(restore(&d.0,"f",&[3,4]).is_err()); assert!(!d.0.join("f").exists());
        fs::remove_file(d.0.join("f.part2")).unwrap(); assert!(restore(&d.0,"f",&[3,3]).is_err()); fs::remove_dir_all(&d.0).unwrap();
    }
    #[test] fn no_temporary_files_remain() {
        let d=setup(); restore(&d.0,"f",&[3,3]).unwrap(); assert_eq!(fs::read_dir(&d.0).unwrap().count(),3); fs::remove_dir_all(&d.0).unwrap();
    }
}
