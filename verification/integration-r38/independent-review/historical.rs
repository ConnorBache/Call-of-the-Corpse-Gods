use corpse_core::{startup,storage};
use std::io::Write;
fn main(){for rev in 29..=37 {let mut r=startup::new_run_shell_revision("r38-historical-stable","Unblessed",false,rev as f64).unwrap();startup::ensure_floors(&mut r,40).unwrap();let raw=storage::serialize(&r).unwrap();println!("{rev}\t{raw}");std::io::stdout().flush().unwrap();}}
