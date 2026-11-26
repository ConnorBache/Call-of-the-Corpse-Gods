//! Desktop entry points shared by release arguments, menu and debug-state admission.
pub const DEV_TOOLS: bool = cfg!(feature = "dev-tools");

/// Release supports ordinary startup overrides, not fixture execution, arbitrary
/// starting floors, renderer captures, bestiary unlocks, or benchmark scripts.
/// Each accepted switch consumes exactly its own value.
pub fn check_args(args: &[String]) -> Result<(), String> {
    if DEV_TOOLS { return Ok(()); }
    let mut args = args.iter().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--software-renderer" => {},
            "--seed" | "--save" | "--replay" | "--replay-dir" | "--profile" | "--appearance" => {
                if args.next().is_none_or(|value| value.starts_with("--")) {
                    return Err(format!("Missing value for {arg}."));
                }
            }
            _ => return Err(format!("Unsupported launch option: {arg}.")),
        }
    }
    Ok(())
}

pub fn check_session(dev: bool, synthetic: bool, commands: &[corpse_core::Command]) -> Result<(), String> {
    if !DEV_TOOLS && (dev || synthetic
        || commands.iter().any(|command| matches!(command, corpse_core::Command::Dev { .. })))
    {
        return Err("Developer sessions cannot be opened in this build.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn argv(flags: &[&str]) -> Vec<String> {
        ["corpse-desktop"].into_iter().chain(flags.iter().copied()).map(str::to_owned).collect()
    }

    #[cfg(not(feature = "dev-tools"))]
    #[test]
    fn release_rejects_dev_arguments_before_opening_the_app() {
        assert_eq!(crate::run_app(argv(&["--debug-floor", "7"])).unwrap_err(),
            "Unsupported launch option: --debug-floor.");
    }

    #[test]
    fn desktop_save_and_replay_import_admit_player_files_and_gate_debug_provenance() {
        let path = std::env::temp_dir().join(format!("corpse-release-entry-{}.json", std::process::id()));
        let raw = include_str!("../../../fixtures/avatar-room-v1.json");
        let engine = corpse_core::temporal::Engine::load(raw).unwrap();
        for (kind, allowed) in [(crate::session_store::Kind::Genuine, true),
            (crate::session_store::Kind::Fixture, DEV_TOOLS)] {
            std::fs::write(&path, crate::session_store::encode(&engine, kind).unwrap()).unwrap();
            assert_eq!(crate::load_with_kind(path.to_str().unwrap()).is_ok(), allowed);
        }

        let run = corpse_core::startup::create_avatar_run("0-1-2:release-entry-policy").unwrap();
        let mut record = corpse_core::replay::ReplayRecord::from_run(&run);
        std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        let mut browser = crate::replays::Replays::new(path.parent().unwrap());
        assert!(browser.load(&path).is_ok());
        if !DEV_TOOLS {
            record.commands.push(corpse_core::Command::Dev { action: "test".into() });
            std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
            assert_eq!(browser.load(&path).unwrap_err(), "Developer sessions cannot be opened in this build.");
            assert!(browser.session.unwrap().record.commands.is_empty());
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn release_keeps_player_args_and_requires_opt_in_for_dev_tools() {
        assert!(check_args(&argv(&[])).is_ok());
        assert!(check_args(&argv(&["--seed", "0-1-2:real-run", "--software-renderer", "--replay", "replay.json"])).is_ok());
        for (flag, val) in [("--playtest-floor","12"), ("--debug-floor","12"), ("--fixture","run.json"),
            ("--generated","true"), ("--render-game","capture.png"), ("--manual-entry","rat"),
            ("--debug-stash","true"), ("--bestiary-preview-all","true"), ("--timing-test","true")] {
            assert_eq!(check_args(&argv(&[flag,val])).is_ok(), DEV_TOOLS, "{flag}");
        }
        assert_eq!(check_args(&argv(&["--seed", "--debug-floor", "12"])).is_ok(), DEV_TOOLS);
        assert_eq!(check_session(true, false, &[]).is_ok(), DEV_TOOLS);
        assert_eq!(check_session(false, true, &[]).is_ok(), DEV_TOOLS);
        assert!(check_session(false, false, &[]).is_ok());
        assert_eq!(check_session(false, false, &[corpse_core::Command::Dev { action: "qa".into() }]).is_ok(), DEV_TOOLS);
    }
}
