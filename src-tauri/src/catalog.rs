//! Static action catalog for the button menu and the key pickers.

use crate::dto::{ActionCatalog, ActionCategory, ActionEntry, KeyEntry, ModifierEntry};
use shark_core::protocol::actions::{ACTIONS, Group};
use shark_core::protocol::keys::{MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN, hid_label};

const GROUPS: [(Group, &str); 6] = [
    (Group::Mouse, "mouse"),
    (Group::Dpi, "dpi"),
    (Group::Media, "media"),
    (Group::Browser, "browser"),
    (Group::Function, "function"),
    (Group::Shortcut, "shortcut"),
];

pub fn action_catalog() -> ActionCatalog {
    let categories = GROUPS
        .iter()
        .map(|(group, id)| ActionCategory {
            id,
            label: group.label(),
            actions: ACTIONS.iter().filter(|a| a.group == *group).map(|a| ActionEntry { id: a.id, label: a.label }).collect(),
        })
        .collect();
    // 0x04..=0xE7 covers letters, digits, punctuation, F-keys, navigation and modifiers; 0xF1..=0xF5 are macro mouse codes
    let keys = (0x04u8..=0xE7).chain(0xF1..=0xF5).map(|code| KeyEntry { code, label: hid_label(code) }).filter(|k| !k.label.starts_with("0x")).collect();
    let modifiers = [(MOD_CTRL, "Ctrl"), (MOD_SHIFT, "Shift"), (MOD_ALT, "Alt"), (MOD_WIN, "Win/Cmd")]
        .map(|(bit, label)| ModifierEntry { bit, label })
        .to_vec();
    ActionCatalog { categories, keys, modifiers }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_every_action_once() {
        let c = action_catalog();
        assert_eq!(c.categories.iter().map(|g| g.actions.len()).sum::<usize>(), ACTIONS.len());
        assert!(c.keys.iter().any(|k| k.code == 0x04 && k.label == "A"));
        assert!(c.keys.iter().any(|k| k.code == 0xF1));
    }
}
