//! Apply planning: which packets to send, in the order of spec 3.8.
//!
//! The original sends 0x0C on startup, profile switch, reset and import. Here 0x0C is only
//! emitted when the caller passes `send_resync = true`, which the UI does only after an
//! explicit confirmation. Switch and import just treat every group as dirty (`prev = None`).

use super::macros::build_macro;
use super::packets::{build_buttons, build_lighting, build_polling, build_resync, build_sensor};
use crate::profile::{ACTION_MACRO, BUTTON_COUNT, Profile};

fn changed(prev: Option<&Vec<u8>>, new: &Vec<u8>) -> bool {
    prev != Some(new)
}

/// Packets for `new`, comparing with `prev` (the last applied profile). `prev = None` marks
/// every group dirty. Order: 0x0C (opt-in), 0x04, 0x05, 0x06, 0x08, then 0x09 per macro button.
pub fn plan(prev: Option<&Profile>, new: &Profile, send_resync: bool) -> Vec<Vec<u8>> {
    plan_groups(prev, new, send_resync, [&build_sensor, &build_lighting, &build_polling, &build_buttons], true)
}

/// Group builder: one fixed-size packet per profile.
pub type Builder<'a> = &'a dyn Fn(&Profile) -> Vec<u8>;

/// `plan` with per-model builders for 0x04, 0x05, 0x06, 0x08 (in that order). `macros = false`
/// skips 0x09 for models whose macro format is not supported.
pub fn plan_groups(prev: Option<&Profile>, new: &Profile, send_resync: bool, groups: [Builder<'_>; 4], macros: bool) -> Vec<Vec<u8>> {
    let prev = if send_resync { None } else { prev };
    let mut out = Vec::new();
    if send_resync {
        out.push(build_resync());
    }
    for build in groups {
        let packet = build(new);
        if changed(prev.map(build).as_ref(), &packet) {
            out.push(packet);
        }
    }
    if !macros {
        return out;
    }
    for (i, button) in new.buttons.iter().enumerate().take(BUTTON_COUNT) {
        let Some(m) = button.macro_binding.as_ref().filter(|_| button.action_id == ACTION_MACRO) else { continue };
        let packet = build_macro(i, m);
        let old = prev
            .and_then(|p| p.buttons.get(i))
            .filter(|b| b.action_id == ACTION_MACRO)
            .and_then(|b| b.macro_binding.as_ref())
            .map(|pm| build_macro(i, pm));
        if changed(old.as_ref(), &packet) {
            out.push(packet);
        }
    }
    out
}
