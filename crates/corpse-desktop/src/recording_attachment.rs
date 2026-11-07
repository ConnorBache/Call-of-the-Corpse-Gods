//! Bounded recording attachment checks. Not a whole-file integrity proof: an
//! uncooperative earlier in-place edit outside these samples is checked on reopen.
use std::{fs::{File,OpenOptions},io::{Read,Seek,SeekFrom},path::Path};
const SAMPLE:usize=64;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]struct Identity(u64,u64);
/// O(1) OS metadata; Unix dev/inode identify the open file, not its pathname.
#[cfg(unix)]fn identity(file:&File)->std::io::Result<Identity>{use std::os::unix::fs::MetadataExt;let m=file.metadata()?;Ok(Identity(m.dev(),m.ino()))}
/// O(1) stable Win32 handle identity, including volume serial and64-bit file index.
/// No unstable Rust MetadataExt::file_index API or additional dependency is used.
#[cfg(windows)]fn identity(file:&File)->std::io::Result<Identity>{use std::os::windows::io::AsRawHandle;#[repr(C)]struct Information{words:[u32;13]}#[link(name="kernel32")]unsafe extern "system"{fn GetFileInformationByHandle(handle:*mut std::ffi::c_void,info:*mut Information)->i32;}let mut info=Information{words:[0;13]};if unsafe{GetFileInformationByHandle(file.as_raw_handle(),&mut info)}==0{return Err(std::io::Error::last_os_error())}Ok(Identity(info.words[7]as u64,((info.words[11]as u64)<<32)|info.words[12]as u64))}
#[cfg(not(any(unix,windows)))]fn identity(_: &File)->std::io::Result<Identity>{Err(std::io::Error::other("Recording file identity is unsupported on this platform"))}
#[derive(Clone,Copy)]pub(super) struct IdentityWitness(Option<Identity>);
impl IdentityWitness{
 /// O(1), retains file identity without retaining a handle across replacement.
 pub fn from_file(file:&File)->std::io::Result<Self>{identity(file).map(|id|Self(Some(id)))}
 /// O(1), detects replacement/unlink while allowing own partial writes to change length.
 pub fn check_identity(&self,path:&Path)->Result<(),String>{match File::open(path){Ok(file)=>{if Some(identity(&file).map_err(|e|e.to_string())?)!=self.0{return Err("Recording was replaced or created externally; preserved until explicit reopen".into())}Ok(())},Err(e)if e.kind()==std::io::ErrorKind::NotFound&&self.0.is_none()=>Ok(()),Err(e)=>Err(format!("Recording path changed during recovery; explicit reopen required: {e}"))}}
 /// O(1), absence is also an expected identity during an initial-create retry.
 pub fn absent()->Self{Self(None)}

}
pub(super) enum AppendFailure{Write(String),Changed(String)}
pub(super) struct Attachment{file:File,id:Identity,len:u64,head:Vec<u8>,tail:Vec<u8>}
impl Attachment{
 /// O(file bytes) once on attachment; exact readback validates what was opened.
 /// Input is the actual checkpoint just written, never a cloned append prefix.
 pub fn open_verified(path:&Path,expected:&[u8])->Result<Self,String>{
 #[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.attach_full_validation");
let mut file=OpenOptions::new().read(true).append(true).open(path).map_err(|e|e.to_string())?;let id=identity(&file).map_err(|e|e.to_string())?;if file.metadata().map_err(|e|e.to_string())?.len()!=expected.len()as u64{return Err("Recording changed while attaching; reopen required".into())}let mut actual=Vec::new();(&mut file).take(expected.len()as u64+1).read_to_end(&mut actual).map_err(|e|e.to_string())?;if actual!=expected{return Err("Recording bytes changed while attaching; preserved".into())}let mut a=Self{file,id,len:expected.len()as u64,head:expected[..expected.len().min(SAMPLE)].to_vec(),tail:expected[expected.len().saturating_sub(SAMPLE)..].to_vec()};a.check(path)?;Ok(a)}
 /// O(1) bounded I/O: identity/length + at most128 sampled bytes, not a full scan.
 pub fn check(&mut self,path:&Path)->Result<(),String>{
 #[cfg(feature="runtime-timing")]let _span=corpse_core::timing::span("recording.bounded_check");
let fail=||"Recording identity, length, or boundary changed; file preserved. Explicitly reopen/reload or choose a new save path.".to_string();let visible=File::open(path).map_err(|e|format!("Cannot inspect recording path; preserved: {e}"))?;if identity(&visible).map_err(|e|e.to_string())?!=self.id||visible.metadata().map_err(|e|e.to_string())?.len()!=self.len||identity(&self.file).map_err(|e|e.to_string())?!=self.id||self.file.metadata().map_err(|e|e.to_string())?.len()!=self.len{return Err(fail())}let mut bytes=vec![0;self.head.len()];self.file.seek(SeekFrom::Start(0)).map_err(|e|e.to_string())?;self.file.read_exact(&mut bytes).map_err(|e|e.to_string())?;if bytes!=self.head{return Err(fail())}bytes.resize(self.tail.len(),0);self.file.seek(SeekFrom::Start(self.len-self.tail.len()as u64)).map_err(|e|e.to_string())?;self.file.read_exact(&mut bytes).map_err(|e|e.to_string())?;if bytes!=self.tail{return Err(fail())}Ok(())}
 /// O(1), detached identity token survives checkpoint/write failure without a data handle.
 pub fn witness(&self)->IdentityWitness{IdentityWitness(Some(self.id))}
 /// O(1), acknowledged expected length, independent of filesystem block layout.
 pub fn len(&self)->u64{self.len}
 /// O(new bytes) + sync callback; updates only bounded samples, then checks the
 /// published endpoint. Caller invalidates attachment on every error, retaining receipts.
 pub fn append_with(&mut self,path:&Path,bytes:&[u8],write:impl FnOnce(&mut File,&[u8])->std::io::Result<()>)->Result<(),AppendFailure>{let next=self.len.checked_add(bytes.len()as u64).ok_or_else(||AppendFailure::Write("Recording length overflow".into()))?;write(&mut self.file,bytes).map_err(|e|AppendFailure::Write(format!("Recording append failed; prior complete records remain recoverable: {e}")))?;if self.head.len()<SAMPLE{let count=(SAMPLE-self.head.len()).min(bytes.len());self.head.extend_from_slice(&bytes[..count]);}if bytes.len()>=SAMPLE{self.tail.clear();self.tail.extend_from_slice(&bytes[bytes.len()-SAMPLE..]);}else{self.tail.extend_from_slice(bytes);let excess=self.tail.len().saturating_sub(SAMPLE);self.tail.drain(..excess);}self.len=next;self.check(path).map_err(AppendFailure::Changed)}
}
