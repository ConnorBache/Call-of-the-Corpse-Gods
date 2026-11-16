//! Bounded, single-file incremental save framing. This module performs no I/O,
//! simulation mutation, checkpoint decoding, or durability operations.
//!
//! Complete frames are borrowed from the input. A storage caller MUST validate
//! typed command batches in `parse`'s callback, then replay them in order into a
//! freshly decoded checkpoint, rejecting the entire load on any replay error.
//! The caller must bound file reads to MAX_FILE_BYTES before allocating a file
//! buffer; `parse` also checks the supplied buffer. CRC32 detects accidental
//! corruption, not malicious modification or authenticity.
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8] = b"CCG-INCREMENTAL-SAVE/1\n";
pub const REPLAY_BUILD_ID: &str = "corpse-r39-snapshot-overwrite-v1";
pub const LEGACY_REPLAY_BUILD_ID: &str = "corpse-r39-incremental-v1";
pub const SIMULATION_REVISION: u32 = 39;
pub const SHORE_REPLAY_BUILD_ID: &str = "corpse-r40-shore-snapshot-overwrite-v1";
pub const SHORE_SIMULATION_REVISION: u32 = 40;
pub const TIMEWEAVE_REPLAY_BUILD_ID: &str = "corpse-r41-timeweave-v1";
pub const TIMEWEAVE_SIMULATION_REVISION: u32 = 41;
pub const FORGIVING_REPLAY_BUILD_ID: &str = "corpse-r42-forgiving-start-v1";
pub const FORGIVING_SIMULATION_REVISION: u32 = 42;
pub const ITEM_ACTION_REPLAY_BUILD_ID: &str = "corpse-r43-item-action-v1";
pub const ITEM_ACTION_SIMULATION_REVISION: u32 = 43;
pub const BLEED_REPLAY_BUILD_ID: &str = "corpse-r44-bleed-v1";
pub const BLEED_SIMULATION_REVISION: u32 = 44;
pub const NOVA_REPLAY_BUILD_ID: &str = "corpse-r45-nova-energy-v1";
pub const NOVA_SIMULATION_REVISION: u32 = 45;
pub const SHINE_REPLAY_BUILD_ID: &str = "corpse-r46-shine-suppression-v1";
pub const SHINE_SIMULATION_REVISION: u32 = 46;
pub const ROSTER_REPLAY_BUILD_ID: &str = "corpse-r50-ability-roster-v1";
pub const ROSTER_SIMULATION_REVISION: u32 = 50;
pub const ROUNDING_REPLAY_BUILD_ID: &str = "corpse-r51-nearest-damage-v1";
pub const ROUNDING_SIMULATION_REVISION: u32 = 51;
pub const DARKNESS_REPLAY_BUILD_ID: &str = "corpse-r52-absolute-darkness-v1";
pub const DARKNESS_SIMULATION_REVISION: u32 = 52;
pub const METADATA_R52_BUILD_ID: &str = "corpse-r52-absolute-darkness-metadata-v2";
pub const METADATA_R51_BUILD_ID: &str = "corpse-r51-nearest-damage-metadata-v2";
pub const METADATA_R50_BUILD_ID: &str = "corpse-r50-ability-roster-metadata-v2";
pub const METADATA_R46_BUILD_ID: &str = "corpse-r46-shine-suppression-metadata-v2";
pub const METADATA_R45_BUILD_ID: &str = "corpse-r45-nova-energy-metadata-v2";
pub const METADATA_R44_BUILD_ID: &str = "corpse-r44-bleed-metadata-v2";
pub const METADATA_R43_BUILD_ID: &str = "corpse-r43-item-action-metadata-v2";
pub const METADATA_R42_BUILD_ID: &str = "corpse-r42-forgiving-start-metadata-v2";
pub const METADATA_R39_BUILD_ID: &str = "corpse-r39-snapshot-metadata-v2";
pub const METADATA_R40_BUILD_ID: &str = "corpse-r40-shore-metadata-v2";
pub const METADATA_R41_BUILD_ID: &str = "corpse-r41-timeweave-metadata-v2";
pub const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FRAME_PAYLOAD_BYTES: usize = MAX_FILE_BYTES;
pub const MAX_HEADER_BYTES: usize = 1024;
pub const MAX_BATCH_PAYLOAD_BYTES: usize = 256 * 1024;
/// Maximum command-batch records; the initial checkpoint is additional.
/// Extended-tail trial: 65,536 borrowed descriptors (roughly2MiB on64-bit),
/// still bounded independently by the unchanged32MiB total file cap. At most
///32 receipts per record means2,097,152 commands; replay CPU is not time-bounded.
pub const MAX_RECORDS: usize = 65_536;
const FRAME_HEADER_LEN: usize = 55;
const TRAILER_LEN: usize = 14;
const CHECKPOINT: u8 = b'C';
const BATCH: u8 = b'B';

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Header {
    pub replay_build: String,
    pub simulation_revision: u32,
}
impl Header {
    pub fn current() -> Self {
        Self { replay_build: REPLAY_BUILD_ID.into(), simulation_revision: SIMULATION_REVISION }
    }
    /// O(1), choose framing without upgrading the run's simulation rules.
    pub fn for_revision(revision: Option<f64>) -> Option<Self> {
        match revision {
            Some(v) if v == SIMULATION_REVISION as f64 => Some(Self::current()),
            Some(v) if v == SHORE_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: SHORE_REPLAY_BUILD_ID.into(), simulation_revision: SHORE_SIMULATION_REVISION,
            }),
            Some(v) if v == TIMEWEAVE_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: TIMEWEAVE_REPLAY_BUILD_ID.into(), simulation_revision: TIMEWEAVE_SIMULATION_REVISION,
            }),
            Some(v) if v == FORGIVING_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: FORGIVING_REPLAY_BUILD_ID.into(), simulation_revision: FORGIVING_SIMULATION_REVISION,
            }),
            Some(v) if v == ITEM_ACTION_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: ITEM_ACTION_REPLAY_BUILD_ID.into(), simulation_revision: ITEM_ACTION_SIMULATION_REVISION,
            }),
            Some(v) if v == BLEED_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: BLEED_REPLAY_BUILD_ID.into(), simulation_revision: BLEED_SIMULATION_REVISION,
            }),
            Some(v) if v == NOVA_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: NOVA_REPLAY_BUILD_ID.into(), simulation_revision: NOVA_SIMULATION_REVISION,
            }),
            Some(v) if v == SHINE_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: SHINE_REPLAY_BUILD_ID.into(), simulation_revision: SHINE_SIMULATION_REVISION,
            }),
            Some(v) if v == DARKNESS_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: DARKNESS_REPLAY_BUILD_ID.into(), simulation_revision: DARKNESS_SIMULATION_REVISION,
            }),
            Some(v) if v == ROUNDING_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: ROUNDING_REPLAY_BUILD_ID.into(), simulation_revision: ROUNDING_SIMULATION_REVISION,
            }),
            Some(v) if v == ROSTER_SIMULATION_REVISION as f64 => Some(Self {
                replay_build: ROSTER_REPLAY_BUILD_ID.into(), simulation_revision: ROSTER_SIMULATION_REVISION,
            }),
            _ => None,
        }
    }
    /// New operation grammar; mechanics revision and frame/CRC format unchanged.
    /// Old readers reject these build IDs rather than ignoring metadata records.
    pub fn for_metadata_revision(revision: Option<f64>) -> Option<Self> {
        let replay_build = match revision {
            Some(39.0) => METADATA_R39_BUILD_ID,
            Some(40.0) => METADATA_R40_BUILD_ID,
            Some(41.0) => METADATA_R41_BUILD_ID,
            Some(42.0) => METADATA_R42_BUILD_ID,
            Some(43.0) => METADATA_R43_BUILD_ID,
            Some(44.0) => METADATA_R44_BUILD_ID,
            Some(45.0) => METADATA_R45_BUILD_ID,
            Some(46.0) => METADATA_R46_BUILD_ID,
            Some(50.0) => METADATA_R50_BUILD_ID,
            Some(51.0) => METADATA_R51_BUILD_ID,
            Some(52.0) => METADATA_R52_BUILD_ID,
            _ => return None,
        };
        Some(Self { replay_build: replay_build.into(), simulation_revision: revision.unwrap() as u32 })
    }
    /// O(identifier bytes), grammar selector after exact pair admission.
    pub fn metadata_operations(&self) -> bool {
        matches!((self.replay_build.as_str(), self.simulation_revision),
            (METADATA_R39_BUILD_ID,39) | (METADATA_R40_BUILD_ID,40) | (METADATA_R41_BUILD_ID,41) | (METADATA_R42_BUILD_ID,42) | (METADATA_R43_BUILD_ID,43) | (METADATA_R44_BUILD_ID,44) | (METADATA_R45_BUILD_ID,45) | (METADATA_R46_BUILD_ID,46) | (METADATA_R50_BUILD_ID,50) | (METADATA_R51_BUILD_ID,51) | (METADATA_R52_BUILD_ID,52))
    }
    /// O(identifier bytes), admits only exact build/revision pairs before replay.
    pub fn legacy_revert_policy(&self) -> Result<bool, String> {
        match (self.replay_build.as_str(), self.simulation_revision) {
            (LEGACY_REPLAY_BUILD_ID, SIMULATION_REVISION) => Ok(true),
            (REPLAY_BUILD_ID, SIMULATION_REVISION) | (SHORE_REPLAY_BUILD_ID, SHORE_SIMULATION_REVISION)
                | (TIMEWEAVE_REPLAY_BUILD_ID, TIMEWEAVE_SIMULATION_REVISION)
                | (FORGIVING_REPLAY_BUILD_ID, FORGIVING_SIMULATION_REVISION)
                | (ITEM_ACTION_REPLAY_BUILD_ID, ITEM_ACTION_SIMULATION_REVISION)
                | (BLEED_REPLAY_BUILD_ID, BLEED_SIMULATION_REVISION)
                | (NOVA_REPLAY_BUILD_ID, NOVA_SIMULATION_REVISION)
                | (SHINE_REPLAY_BUILD_ID, SHINE_SIMULATION_REVISION)
                | (ROSTER_REPLAY_BUILD_ID, ROSTER_SIMULATION_REVISION)
                | (ROUNDING_REPLAY_BUILD_ID, ROUNDING_SIMULATION_REVISION)
                | (DARKNESS_REPLAY_BUILD_ID, DARKNESS_SIMULATION_REVISION)
                | (METADATA_R39_BUILD_ID,39) | (METADATA_R40_BUILD_ID,40) | (METADATA_R41_BUILD_ID,41) | (METADATA_R42_BUILD_ID,42) | (METADATA_R43_BUILD_ID,43) | (METADATA_R44_BUILD_ID,44) | (METADATA_R45_BUILD_ID,45) | (METADATA_R46_BUILD_ID,46) | (METADATA_R50_BUILD_ID,50) | (METADATA_R51_BUILD_ID,51) | (METADATA_R52_BUILD_ID,52) => Ok(false),
            _ => Err(invalid("unsupported replay build/simulation revision pair")),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    /// Bytes through the last complete, validated frame. An incomplete suffix
    /// must be durably removed or checkpoint-replaced BEFORE another append.
    pub valid_len: usize,
    pub next_sequence: u64,
    pub last_checksum: u32,
    pub recovered_tail: bool,
    pub record_count: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record<'a> {
    pub sequence: u64,
    pub checksum: u32,
    pub payload: &'a str,
}
#[derive(Debug)]
pub struct Parsed<'a> {
    pub header: Header,
    /// Original SaveEnvelope JSON bytes, unchanged by framing.
    pub checkpoint: &'a str,
    pub records: Vec<Record<'a>>,
    pub metadata: Metadata,
}
#[derive(Debug)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    pub next_sequence: u64,
    pub last_checksum: u32,
}

/// Recognize the journal family (including unknown versions) so a version error
/// is never silently retried as a legacy JSON save. O(1), no allocation.
pub fn is_journal(raw: &[u8]) -> bool { raw.starts_with(b"CCG-INCREMENTAL-SAVE/") }

fn invalid(message: &str) -> String { format!("Invalid incremental save: {message}") }
fn validate_header(header: &Header) -> Result<(), String> {
    if header.replay_build.is_empty() || header.replay_build.len() > 128 {
        return Err(invalid("replay build identifier length"));
    }
    Ok(())
}
fn checksum(kind: u8, sequence: u64, previous: u32, payload: &[u8]) -> u32 {
    let mut hash = crc32fast::Hasher::new();
    hash.update(b"CCG-INCREMENTAL-FRAME/1\0");
    hash.update(&[kind]);
    hash.update(&sequence.to_be_bytes());
    hash.update(&previous.to_be_bytes());
    hash.update(&(payload.len() as u32).to_be_bytes());
    hash.update(payload);
    hash.finalize()
}
fn frame_len(payload_len: usize) -> Result<usize, String> {
    if payload_len == 0 || payload_len > MAX_FRAME_PAYLOAD_BYTES {
        return Err(invalid("frame payload size limit"));
    }
    FRAME_HEADER_LEN.checked_add(payload_len).and_then(|n| n.checked_add(TRAILER_LEN))
        .filter(|&n| n <= MAX_FILE_BYTES).ok_or_else(|| invalid("frame size limit"))
}
fn append_frame(out: &mut Vec<u8>, kind: u8, sequence: u64, previous: u32, payload: &str) -> Result<u32, String> {
    let length = frame_len(payload.len())?;
    if out.len().checked_add(length).is_none_or(|n| n > MAX_FILE_BYTES) {
        return Err(invalid("file size limit"));
    }
    let crc = checksum(kind, sequence, previous, payload.as_bytes());
    // Header CRC protects the declared length even if the payload is torn.
    // kind SP seq16 SP prev8 SP length8 SP payload_crc8 SP header_crc8 LF.
    let prefix = format!("{} {sequence:016x} {previous:08x} {:08x} {crc:08x}", kind as char, payload.len());
    let header_crc = crc32fast::hash(prefix.as_bytes());
    let header = format!("{prefix} {header_crc:08x}\n");
    debug_assert_eq!(header.len(), FRAME_HEADER_LEN);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(payload.as_bytes());
    out.extend_from_slice(format!("\nEND {crc:08x}\n").as_bytes());
    Ok(crc)
}

/// Exact maximum checkpoint bytes after this bounded header and framing.
/// O(header bytes) time and temporary memory; at most MAX_HEADER_BYTES.
pub fn checkpoint_limit(header: &Header) -> Result<usize, String> {
    validate_header(header)?;
    let metadata = serde_json::to_vec(header).map_err(|e| invalid(&e.to_string()))?;
    if metadata.len() > MAX_HEADER_BYTES { return Err(invalid("compatibility header size limit")); }
    Ok(MAX_FILE_BYTES - MAGIC.len() - metadata.len() - 1 - FRAME_HEADER_LEN - TRAILER_LEN)
}

/// Frame original checkpoint bytes without changing their JSON spelling.
/// O(checkpoint bytes + header bytes) time and output memory. No Run clone, diff,
/// serialization or hash of current state beyond CRC of bytes already given.
/// Callers retain the existing temp/write/sync/rename checkpoint protocol.
pub fn encode_initial(checkpoint: &str, header: &Header) -> Result<Encoded, String> {
    validate_header(header)?;
    let metadata = serde_json::to_vec(header).map_err(|e| invalid(&e.to_string()))?;
    if metadata.len() > MAX_HEADER_BYTES { return Err(invalid("compatibility header size limit")); }
    let total = MAGIC.len().checked_add(metadata.len()).and_then(|n| n.checked_add(1))
        .and_then(|n| frame_len(checkpoint.len()).ok().and_then(|f| n.checked_add(f)))
        .filter(|&n| n <= MAX_FILE_BYTES).ok_or_else(|| invalid("file size limit"))?;
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&metadata);
    bytes.push(b'\n');
    let previous = crc32fast::hash(&bytes);
    let last_checksum = append_frame(&mut bytes, CHECKPOINT, 0, previous, checkpoint)?;
    Ok(Encoded { bytes, next_sequence: 1, last_checksum })
}

/// O(payload bytes) time and output memory. The caller enforces the cumulative
/// file/record limits using its verified Metadata before append and syncs the
/// new bytes before acknowledging success. Sequence zero belongs to checkpoint.
pub fn encode_append(sequence: u64, previous: u32, payload: &str) -> Result<Encoded, String> {
    if sequence == 0 || sequence > MAX_RECORDS as u64 { return Err(invalid("command sequence limit")); }
    if payload.len() > MAX_BATCH_PAYLOAD_BYTES { return Err(invalid("command batch size limit")); }
    let mut bytes = Vec::with_capacity(frame_len(payload.len())?);
    let last_checksum = append_frame(&mut bytes, BATCH, sequence, previous, payload)?;
    Ok(Encoded { bytes, next_sequence: sequence + 1, last_checksum })
}

fn hex(bytes: &[u8]) -> Result<u64, String> {
    bytes.iter().try_fold(0u64, |value, &b| {
        let digit = match b { b'0'..=b'9' => b - b'0', b'a'..=b'f' => b - b'a' + 10,
            _ => return Err(invalid("frame header hex spelling")) };
        value.checked_mul(16).and_then(|v| v.checked_add(digit as u64))
            .ok_or_else(|| invalid("frame header number overflow"))
    })
}
/// Validate even a partial header's present syntax. Invalid garbage is not
/// treated as a torn write. Incomplete *valid prefixes* may be recovered.
fn header_prefix(bytes: &[u8], expected_kind: u8) -> Result<(), String> {
    for (i, &b) in bytes.iter().take(FRAME_HEADER_LEN).enumerate() {
        let valid = match i {
            0 => b == expected_kind,
            1 | 18 | 27 | 36 | 45 => b == b' ',
            54 => b == b'\n',
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        };
        if !valid { return Err(invalid("frame header syntax or kind")); }
    }
    Ok(())
}

/// O(min(header bytes,1024)) time/memory; rejects oversized files before parsing.
/// This is compatibility selection ONLY, not an integrity certificate. Call parse
/// with the selected exact header before decoding checkpoint or executing commands.
pub fn inspect_header(raw: &[u8]) -> Result<Header,String> {
    read_header(raw).map(|(header,_)|header)
}
/// O(bounded header bytes); shared strict schema/duplicate-field validation.
fn read_header(raw: &[u8]) -> Result<(Header,usize),String> {
    if raw.len() > MAX_FILE_BYTES { return Err(invalid("file size limit")); }
    if !raw.starts_with(MAGIC) { return Err(invalid("unsupported journal magic or version")); }
    let metadata_start = MAGIC.len();
    let available = &raw[metadata_start..];
    let metadata_len = available.iter().take(MAX_HEADER_BYTES + 1).position(|&b| b == b'\n')
        .ok_or_else(|| invalid("missing or oversized compatibility header"))?;
    let metadata_end = metadata_start + metadata_len;
    let header: Header = serde_json::from_slice(&raw[metadata_start..metadata_end])
        .map_err(|e| invalid(&format!("compatibility header: {e}")))?;
    validate_header(&header)?;
    Ok((header,metadata_end+1))
}

/// O(N + V) time for N supplied bytes and V validator work; O(R + header) codec
/// memory for R <= MAX_RECORDS borrowed records. No payload-size allocation by
/// framing. The required validator runs only on complete CRC/link-valid batches
/// and MUST reject malformed/unsupported commands without mutating live state.
/// Any complete corrupt record fails the entire parse. Only a syntactically
/// possible incomplete final record after a complete checkpoint is recoverable.
pub fn parse<'a, F>(raw: &'a [u8], expected: &Header, mut validate_batch: F) -> Result<Parsed<'a>, String>
where F: FnMut(&str) -> Result<(), String> {
    validate_header(expected)?;
    let (header,metadata_end) = read_header(raw)?;
    if header.replay_build != expected.replay_build { return Err(invalid("unsupported replay build")); }
    if header.simulation_revision != expected.simulation_revision { return Err(invalid("unsupported simulation revision")); }
    let mut cursor = metadata_end;
    let mut previous = crc32fast::hash(&raw[..cursor]);
    let mut next_sequence = 0;
    let mut checkpoint = None;
    let mut records = Vec::new();
    let mut recovered_tail = false;
    loop {
        if cursor == raw.len() { break; }
        let kind = if checkpoint.is_none() { CHECKPOINT } else { BATCH };
        let remaining = &raw[cursor..];
        header_prefix(remaining, kind)?;
        if remaining.len() < FRAME_HEADER_LEN {
            recovered_tail = true;
            break;
        }
        let header_crc = hex(&remaining[46..54])? as u32;
        if crc32fast::hash(&remaining[..45]) != header_crc { return Err(invalid("frame header CRC mismatch")); }
        let sequence = hex(&remaining[2..18])?;
        let linked = hex(&remaining[19..27])? as u32;
        let payload_len = hex(&remaining[28..36])? as usize;
        let declared_crc = hex(&remaining[37..45])? as u32;
        if sequence != next_sequence { return Err(invalid("nonmonotonic frame sequence")); }
        if linked != previous { return Err(invalid("previous frame checksum mismatch")); }
        if sequence > MAX_RECORDS as u64 { return Err(invalid("record count limit")); }
        if kind == BATCH && payload_len > MAX_BATCH_PAYLOAD_BYTES { return Err(invalid("command batch size limit")); }
        let full_len = frame_len(payload_len)?;
        let payload_end = FRAME_HEADER_LEN + payload_len;
        if remaining.len() < payload_end {
            recovered_tail = true;
            break;
        }
        let payload_bytes = &remaining[FRAME_HEADER_LEN..payload_end];
        let actual_crc = checksum(kind, sequence, linked, payload_bytes);
        if actual_crc != declared_crc { return Err(invalid("frame CRC mismatch")); }
        let trailer = format!("\nEND {actual_crc:08x}\n");
        let trailer_available = (remaining.len() - payload_end).min(TRAILER_LEN);
        if remaining[payload_end..payload_end + trailer_available] != trailer.as_bytes()[..trailer_available] {
            return Err(invalid("frame trailer mismatch"));
        }
        if remaining.len() < full_len {
            recovered_tail = true;
            break;
        }
        let payload = std::str::from_utf8(payload_bytes).map_err(|_| invalid("complete payload is not UTF8"))?;
        if kind == CHECKPOINT {
            checkpoint = Some(payload);
        } else {
            validate_batch(payload).map_err(|e| invalid(&format!("command batch: {e}")))?;
            records.push(Record { sequence, checksum: actual_crc, payload });
        }
        cursor += full_len;
        previous = actual_crc;
        next_sequence += 1;
    }
    let checkpoint = checkpoint.ok_or_else(|| invalid("incomplete or missing initial checkpoint"))?;
    Ok(Parsed { header, checkpoint, metadata: Metadata { valid_len: cursor, next_sequence,
        last_checksum: previous, recovered_tail, record_count: records.len() }, records })
}

#[cfg(test)]
mod tests {
    use super::*;
    const CHECKPOINT_JSON: &str = "{\"nativeSaveVersion\":1,\"session\":\"fixture\",\"run\":{\"text\":\"literal \\\"quote\\\" and \\n\"}}";
    const WAIT: &str = "{\"commands\":[{\"type\":\"wait\"}]}";
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Batch { commands: Vec<TestCommand> }
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
    enum TestCommand { Wait, Choose { id: String } }
    fn validate(raw: &str) -> Result<(), String> {
        let batch: Batch = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        if batch.commands.is_empty() || batch.commands.len() > 32 { return Err("command count".into()); }
        // Serde internally tagged unit variants may ignore extra fields even
        // with deny_unknown_fields. Storage must make key admission explicit.
        let input: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        for (command, original) in batch.commands.iter().zip(input["commands"].as_array().unwrap()) {
            let canonical = serde_json::to_value(command).map_err(|e| e.to_string())?;
            let a = original.as_object().ok_or("command object")?;
            let b = canonical.as_object().unwrap();
            if a.len() != b.len() || a.keys().any(|key| !b.contains_key(key)) { return Err("unknown command fields".into()); }
            if let TestCommand::Choose { id } = command { assert!(!id.is_empty()); }
        }
        Ok(())
    }
    fn initial() -> Encoded { encode_initial(CHECKPOINT_JSON, &Header::current()).unwrap() }
    fn parsed(bytes: &[u8]) -> Result<Parsed<'_>, String> { parse(bytes, &Header::current(), validate) }
    fn with_batch(payload: &str) -> (Encoded, Vec<u8>) {
        let first = initial();
        let frame = encode_append(first.next_sequence, first.last_checksum, payload).unwrap();
        let mut bytes = first.bytes.clone(); bytes.extend_from_slice(&frame.bytes);
        (first, bytes)
    }
    fn raw_frame(kind: u8, seq: u64, prev: u32, payload: &[u8]) -> Vec<u8> {
        let crc = checksum(kind, seq, prev, payload);
        let prefix = format!("{} {seq:016x} {prev:08x} {:08x} {crc:08x}", kind as char, payload.len());
        let mut out = format!("{prefix} {:08x}\n", crc32fast::hash(prefix.as_bytes())).into_bytes();
        out.extend_from_slice(payload);
        out.extend_from_slice(format!("\nEND {crc:08x}\n").as_bytes()); out
    }
    fn refresh_header_crc(frame: &mut [u8]) {
        let crc = format!("{:08x}", crc32fast::hash(&frame[..45]));
        frame[46..54].copy_from_slice(crc.as_bytes());
    }
    #[test]
    fn framing_preserves_checkpoint_and_escaped_utf8_batch_exactly() {
        let payload = "{\"commands\":[{\"type\":\"choose\",\"id\":\"☃ 🧪 newline\\n quote\\\" slash\\\\ END\\n\"}]}";
        let (first, bytes) = with_batch(payload);
        assert!(std::str::from_utf8(&bytes).is_ok());
        let restored = parsed(&bytes).unwrap();
        assert_eq!(restored.checkpoint.as_bytes(), CHECKPOINT_JSON.as_bytes());
        assert_eq!(restored.records[0].payload, payload);
        assert_eq!(restored.metadata.valid_len, bytes.len());
        assert_eq!(restored.metadata.next_sequence, 2);
        assert_eq!(restored.metadata.record_count, 1);
        assert!(!restored.metadata.recovered_tail);
        assert_eq!(restored.records[0].sequence, first.next_sequence);
        let offset = restored.checkpoint.as_ptr() as usize - bytes.as_ptr() as usize;
        assert!(offset < bytes.len()); // checkpoint borrows, not a reserialization.
    }
    #[test]
    fn incomplete_initial_file_is_never_recovered_without_checkpoint() {
        let first = initial();
        for end in 0..first.bytes.len() { assert!(parsed(&first.bytes[..end]).is_err(), "cut {end}"); }
        assert!(parsed(&first.bytes).is_ok());
    }
    #[test]
    fn every_final_frame_truncation_recovers_only_last_complete_frame() {
        let (first, bytes) = with_batch("{\"commands\":[{\"type\":\"choose\",\"id\":\"☃🧪\"}]}");
        for end in first.bytes.len()..bytes.len() {
            let restored = parsed(&bytes[..end]).unwrap_or_else(|e| panic!("cut {end}: {e}"));
            assert_eq!(restored.metadata.valid_len, first.bytes.len(), "cut {end}");
            assert_eq!(restored.metadata.next_sequence, 1);
            assert!(restored.records.is_empty());
            assert_eq!(restored.metadata.recovered_tail, end != first.bytes.len());
        }
        assert_eq!(parsed(&bytes).unwrap().records.len(), 1);
    }
    #[test]
    fn every_single_byte_frame_corruption_is_rejected_not_tail_recovered() {
        let (first, bytes) = with_batch(WAIT);
        for index in first.bytes.len()..bytes.len() {
            let mut changed = bytes.clone(); changed[index] ^= 1;
            assert!(parsed(&changed).is_err(), "byte {index}");
        }
    }
    #[test]
    fn complete_invalid_json_command_and_extra_fields_are_rejected() {
        for payload in ["{", "null", "{\"commands\":[]}", "{\"commands\":[{\"type\":\"unknown\"}]}",
            "{\"commands\":[{\"type\":\"wait\",\"extra\":1}]}", "{\"commands\":[{\"type\":\"wait\"}],\"extra\":1}"] {
            let (_, bytes) = with_batch(payload);
            assert!(parsed(&bytes).is_err(), "{payload}");
        }
    }
    #[test]
    fn wrong_sequence_link_kind_and_repeated_checkpoint_are_rejected() {
        let first = initial();
        for frame in [raw_frame(BATCH, 2, first.last_checksum, WAIT.as_bytes()),
            raw_frame(BATCH, 1, first.last_checksum ^ 1, WAIT.as_bytes()),
            raw_frame(CHECKPOINT, 1, first.last_checksum, WAIT.as_bytes())] {
            let mut bytes = first.bytes.clone(); bytes.extend(frame); assert!(parsed(&bytes).is_err());
        }
    }
    #[test]
    fn compatibility_build_revision_format_and_header_bytes_are_validated() {
        let mut other = Header::current(); other.replay_build.push_str("-next");
        assert!(parsed(&encode_initial(CHECKPOINT_JSON, &other).unwrap().bytes).unwrap_err().contains("build"));
        other = Header::current(); other.simulation_revision += 1;
        assert!(parsed(&encode_initial(CHECKPOINT_JSON, &other).unwrap().bytes).unwrap_err().contains("revision"));
        let mut version = initial().bytes; version[MAGIC.len() - 2] = b'2';
        assert!(is_journal(&version)); assert!(parsed(&version).unwrap_err().contains("version"));
        assert!(!is_journal(CHECKPOINT_JSON.as_bytes()));
        // Valid JSON metadata whitespace modification must also break the first link.
        let mut bytes = initial().bytes; bytes.insert(MAGIC.len(), b' ');
        assert!(parsed(&bytes).unwrap_err().contains("checksum"));
    }
    #[test]
    fn complete_non_utf8_payload_is_rejected_but_torn_utf8_tail_is_safe() {
        let first = initial();
        let mut bytes = first.bytes.clone(); bytes.extend(raw_frame(BATCH, 1, first.last_checksum, &[0xff]));
        assert!(parsed(&bytes).unwrap_err().contains("UTF8"));
    }
    #[test]
    fn header_length_corruption_is_not_mistaken_for_torn_payload() {
        let (first, mut bytes) = with_batch(WAIT);
        let offset = first.bytes.len(); bytes[offset + 28] = b'1';
        assert!(parsed(&bytes).unwrap_err().contains("header CRC"));
        // Even a checksum-consistent oversized declaration is rejected before allocation.
        bytes[offset + 28..offset + 36].copy_from_slice(b"ffffffff");
        refresh_header_crc(&mut bytes[offset..]);
        assert!(parsed(&bytes).unwrap_err().contains("size limit"));
    }
    #[test]
    fn bounds_reject_file_batch_header_and_record_oversize() {
        assert!(parsed(&vec![b' '; MAX_FILE_BYTES + 1]).unwrap_err().contains("file size"));
        assert!(encode_append(1, 0, &"x".repeat(MAX_BATCH_PAYLOAD_BYTES + 1)).is_err());
        assert!(encode_append(0, 0, WAIT).is_err());
        assert!(encode_append(MAX_RECORDS as u64 + 1, 0, WAIT).is_err());
        let mut header = MAGIC.to_vec(); header.extend(vec![b' '; MAX_HEADER_BYTES + 1]); header.push(b'\n');
        assert!(parsed(&header).unwrap_err().contains("compatibility header"));
        assert!(checkpoint_limit(&Header::current()).unwrap() < MAX_FILE_BYTES);
        let mut bytes = initial().bytes;
        let mut previous = parsed(&bytes).unwrap().metadata.last_checksum;
        for sequence in 1..=MAX_RECORDS {
            let frame = encode_append(sequence as u64, previous, WAIT).unwrap();
            previous = frame.last_checksum; bytes.extend(frame.bytes);
        }
        assert_eq!(parsed(&bytes).unwrap().records.len(), MAX_RECORDS);
        bytes.extend(raw_frame(BATCH, MAX_RECORDS as u64 + 1, previous, WAIT.as_bytes()));
        assert!(parsed(&bytes).unwrap_err().contains("record count"));
    }
    #[test]
    fn malformed_partial_header_or_trailer_is_not_ignored() {
        let mut bytes = initial().bytes; bytes.extend_from_slice(b"garbage");
        assert!(parsed(&bytes).is_err());
        let (first, mut bytes) = with_batch(WAIT); bytes.pop();
        *bytes.last_mut().unwrap() = b'z';
        assert!(parsed(&bytes).is_err());
        assert!(bytes.len() > first.bytes.len());
    }
    #[test]
    fn incomplete_batch_never_reaches_validator_and_valid_batches_arrive_in_order() {
        let (first, bytes) = with_batch(WAIT);
        let mut calls = 0;
        parse(&bytes[..bytes.len() - 1], &Header::current(), |_| { calls += 1; Ok(()) }).unwrap();
        assert_eq!(calls, 0);
        let state = parsed(&bytes).unwrap().metadata;
        let second = encode_append(state.next_sequence, state.last_checksum, WAIT).unwrap();
        let mut all = bytes; all.extend(second.bytes);
        let restored = parse(&all, &Header::current(), |_| { calls += 1; Ok(()) }).unwrap();
        assert_eq!(calls, 2); assert_eq!(restored.records[0].sequence, first.next_sequence);
        assert_eq!(restored.records[1].sequence, 2);
    }
    /// O(R*P), demonstrates framing has no32-command/1MiB rollover requirement.
    #[test]
    fn extended_tail_over_32_records_and_one_mib_preserves_payloads() {
        let first=initial();let mut bytes=first.bytes.clone();let mut previous=first.last_checksum;
        let payload=serde_json::json!({"commands":[{"type":"choose","id":"x".repeat(32*1024)}]}).to_string();
        for sequence in 1..=40 {let frame=encode_append(sequence,previous,&payload).unwrap();previous=frame.last_checksum;bytes.extend(frame.bytes);}
        assert!(bytes.len()-first.bytes.len()>1024*1024);
        let restored=parsed(&bytes).unwrap();assert_eq!(restored.records.len(),40);
        assert_eq!(restored.metadata.next_sequence,41);assert_eq!(restored.metadata.last_checksum,previous);
        assert_eq!(restored.metadata.valid_len,bytes.len());assert!(!restored.metadata.recovered_tail);
        for (index,record) in restored.records.iter().enumerate(){assert_eq!(record.sequence,index as u64+1);assert_eq!(record.payload,payload);}
    }
    /// O(R*P) fixture construction and bounded representative tail cuts.
    #[test]
    fn extended_tail_corruption_fails_and_incomplete_last_frame_recovers_prefix() {
        let first=initial();let mut bytes=first.bytes;let mut previous=first.last_checksum;let mut offsets=vec![];
        let payload=serde_json::json!({"commands":[{"type":"choose","id":"y".repeat(32*1024)}]}).to_string();
        for sequence in 1..=40 {offsets.push(bytes.len());let frame=encode_append(sequence,previous,&payload).unwrap();previous=frame.last_checksum;bytes.extend(frame.bytes);}
        for index in [offsets[0]+FRAME_HEADER_LEN,offsets[19]+FRAME_HEADER_LEN,bytes.len()-2] {let mut corrupt=bytes.clone();corrupt[index]^=1;assert!(parsed(&corrupt).is_err(),"corrupt byte {index}");}
        let last=offsets[39];
        for cut in [last,last+1,last+FRAME_HEADER_LEN-1,last+FRAME_HEADER_LEN,last+FRAME_HEADER_LEN+payload.len()/2,bytes.len()-1] {
            let mut calls=0;let restored=parse(&bytes[..cut],&Header::current(),|raw|{calls+=1;validate(raw)}).unwrap();
            assert_eq!(calls,39);assert_eq!(restored.records.len(),39);assert_eq!(restored.metadata.valid_len,last);assert_eq!(restored.metadata.next_sequence,40);
        }
    }
    /// O(header+checkpoint bytes); existing replay policy identifiers remain distinct.
    #[test]
    fn legacy_build_remains_explicitly_selected_for_long_tail_reader() {
        let legacy=Header{replay_build:LEGACY_REPLAY_BUILD_ID.into(),simulation_revision:SIMULATION_REVISION};
        let original=encode_initial(CHECKPOINT_JSON,&legacy).unwrap();
        assert_eq!(inspect_header(&original.bytes).unwrap(),legacy);
        assert!(parse(&original.bytes,&Header::current(),validate).unwrap_err().contains("replay build"));
        assert_eq!(parse(&original.bytes,&legacy,validate).unwrap().checkpoint,CHECKPOINT_JSON);
    }

}

#[cfg(test)]
mod revision_pair_tests {
    use super::*;
    /// O(1), finite pair matrix and malformed revision selection.
    #[test]
    fn revision_headers_select_and_admit_only_exact_pairs() {
        for (build, revision, legacy) in [(LEGACY_REPLAY_BUILD_ID,39,true),(REPLAY_BUILD_ID,39,false),(SHORE_REPLAY_BUILD_ID,40,false),(TIMEWEAVE_REPLAY_BUILD_ID,41,false),(FORGIVING_REPLAY_BUILD_ID,42,false),(ITEM_ACTION_REPLAY_BUILD_ID,43,false),(BLEED_REPLAY_BUILD_ID,44,false),(NOVA_REPLAY_BUILD_ID,45,false),(SHINE_REPLAY_BUILD_ID,46,false)] {
            let h=Header{replay_build:build.into(),simulation_revision:revision};
            assert_eq!(h.legacy_revert_policy().unwrap(),legacy);
            for other in [39,40,41,42,43,44,45,46,47] {
                if other!=revision {
                    let wrong=Header{simulation_revision:other,..h.clone()};
                    assert!(wrong.legacy_revert_policy().is_err());
                }
            }
        }
        assert_eq!(Header::for_revision(Some(39.0)),Some(Header::current()));
        assert_eq!(Header::for_revision(Some(40.0)).unwrap().replay_build,SHORE_REPLAY_BUILD_ID);
        assert_eq!(Header::for_revision(Some(41.0)).unwrap().replay_build,TIMEWEAVE_REPLAY_BUILD_ID);
        assert_eq!(Header::for_revision(Some(42.0)).unwrap().replay_build,FORGIVING_REPLAY_BUILD_ID);
        assert_eq!(Header::for_revision(Some(43.0)).unwrap().replay_build,ITEM_ACTION_REPLAY_BUILD_ID);
        assert_eq!(Header::for_revision(Some(44.0)).unwrap().replay_build,BLEED_REPLAY_BUILD_ID);
        assert_eq!(Header::for_revision(Some(45.0)).unwrap().replay_build,NOVA_REPLAY_BUILD_ID);
        assert_eq!(Header::for_revision(Some(46.0)).unwrap().replay_build,SHINE_REPLAY_BUILD_ID);
        for rev in [None,Some(38.0),Some(47.0),Some(40.5),Some(41.5),Some(f64::NAN),Some(f64::INFINITY)] {
            assert!(Header::for_revision(rev).is_none());
        }
    }
}
