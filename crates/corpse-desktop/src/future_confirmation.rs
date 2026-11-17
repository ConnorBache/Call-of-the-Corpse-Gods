//! UI-only consent gate. No simulation writes until the existing command dispatch.
use corpse_core::{Command, Run};
use sdl2::keyboard::Keycode;

pub fn message(run: &Run, command: &Command) -> Option<&'static str> {
    if !matches!(command, Command::Ability { id, .. } if id == "morae-see-the-future")
        || !run.player.abilities.iter().any(|id| id == "morae-see-the-future") {
        return None;
    }
    Some(if run.future.is_some() {
        "Return to your saved state and undo everything since?"
    } else {
        "Spend all your energy to see the future?"
    })
}

/// Y returns the queued command exactly once; N/Escape discard it; other keys retain it.
pub fn answer(prompt: &mut Option<(Command, String)>, key: Keycode) -> Option<Command> {
    match key {
        Keycode::Y => prompt.take().map(|(command, _)| command),
        Keycode::N | Keycode::Escape => { *prompt = None; None }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::{Field, future_anchor::FutureAnchor};
    fn command() -> Command { Command::Ability { id: "morae-see-the-future".into(), target: Field::Missing, x: Field::Missing, y: Field::Missing } }
    #[test]
    fn future_confirmation_distinguishes_capture_and_return_without_mutation() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.player.abilities.push("morae-see-the-future".into()); run.future=None;
        let before=run.clone();
        assert_eq!(message(&run,&command()),Some("Spend all your energy to see the future?"));
        assert_eq!(run,before);
        run.future=Some(FutureAnchor::from_text("test anchor".into()));
        let before=run.clone();
        assert_eq!(message(&run,&command()),Some("Return to your saved state and undo everything since?"));
        assert_eq!(run,before);
        assert_eq!(message(&run,&Command::Wait),None);
    }
    #[test]
    fn future_confirmation_cancel_and_confirm_both_stages() {
        for text in ["Spend all your energy to see the future?", "Return to your saved state and undo everything since?"] {
            for key in [Keycode::N,Keycode::Escape] {
                let mut prompt=Some((command(),text.into()));
                assert!(answer(&mut prompt,key).is_none()); assert!(prompt.is_none());
                assert!(answer(&mut prompt,Keycode::Y).is_none());
            }
            let mut prompt=Some((command(),text.into()));
            assert!(answer(&mut prompt,Keycode::Space).is_none()); assert!(prompt.is_some());
            assert_eq!(answer(&mut prompt,Keycode::Y),Some(command()));
            assert!(prompt.is_none()); assert!(answer(&mut prompt,Keycode::Y).is_none());
        }
    }
}
