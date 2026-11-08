//! Client mods overriding the vanilla key mappings each tick (`native_key_mapping_state`): a mod
//! that remaps attack, use, movement and sneak to other keys, or blocks them while its own text box
//! has focus, as a `KeyMapping.isDown` mixin would.
use super::{Game, Result, gameplay_has_keyboard};
use crate::mod_api;
use horizon_client::actions::Action;
use winit::event::MouseButton;

/// The mappings a mod is asked about, attack and use first.
const OVERRIDABLE: [Action; 9] = [
    Action::Attack,
    Action::Use,
    Action::Forward,
    Action::Left,
    Action::Back,
    Action::Right,
    Action::Jump,
    Action::Sneak,
    Action::Sprint,
];

impl Game<'_> {
    /// Ask the mods what each vanilla mapping is this tick, while playing with no screen: the
    /// movement mappings take the answer as their held state (`HeldKeys::held`) and so do sneak
    /// and sprint wherever the game reads them, with the keys left as they are. Attack and use
    /// are mouse buttons (clicks, the dig, the use repeat), so `Some(true)` presses the button
    /// for the mod, a click first and then held, and it lets go when the answer ends. It does not
    /// mask the player's own button; `native_interaction_key` cancels what an attack or use does.
    /// Nothing is asked of the mods when none are installed, or off the world.
    pub(super) fn tick_key_mapping_overrides(&mut self) -> Result<()> {
        if mod_api::mods().is_empty() {
            return Ok(());
        }
        let playing = gameplay_has_keyboard(
            self.screen,
            self.title.is_some(),
            self.screen_over_playing(),
        ) && self.player.is_some()
            && !self.dialog_open();
        for action in OVERRIDABLE {
            let mut down = None;
            if playing {
                self.native_key_mapping_state(action.name(), &mut down);
            }
            let button = match action {
                Action::Attack => MouseButton::Left,
                Action::Use => MouseButton::Right,
                _ => {
                    self.keys.set_override(action, down);
                    continue;
                }
            };
            let slot = usize::from(action == Action::Use);
            let want = down == Some(true);
            let held = std::mem::replace(&mut self.mod_buttons[slot], want);
            if let Some(pressed) = button_edge(held, want, playing) {
                self.play_mouse(button, pressed)?;
            }
        }
        Ok(())
    }
}

/// What a mod's answer does to its attack or use button: `Some(true)` presses it, `Some(false)`
/// lets go, `None` leaves it. A screen that opened already let the mappings go
/// (`KeyMapping.releaseAll`), so there is nothing left to release then.
fn button_edge(held: bool, want: bool, playing: bool) -> Option<bool> {
    match (held, want) {
        (false, true) => Some(true),
        (true, false) => playing.then_some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mod_presses_once_and_lets_go_when_its_answer_ends() {
        assert_eq!(button_edge(false, true, true), Some(true));
        assert_eq!(button_edge(true, true, true), None, "held, no new click");
        assert_eq!(button_edge(true, false, true), Some(false));
        assert_eq!(button_edge(false, false, true), None);
    }

    #[test]
    fn a_screen_has_already_released_what_a_mod_held() {
        assert_eq!(button_edge(true, false, false), None);
    }

    #[test]
    fn only_the_nine_mappings_are_offered() {
        let names: Vec<_> = OVERRIDABLE.iter().map(|a| a.name()).collect();
        assert_eq!(
            names,
            [
                "key.attack",
                "key.use",
                "key.forward",
                "key.left",
                "key.back",
                "key.right",
                "key.jump",
                "key.sneak",
                "key.sprint"
            ]
        );
    }
}
