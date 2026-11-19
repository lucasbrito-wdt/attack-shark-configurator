//! Validation of incoming settings patches and button actions (split from `dto.rs`).

use super::*;

fn dpi_rule(caps: &Capabilities) -> String {
    match caps.dpi_high_step_from {
        Some(t) => format!("dpi must be {}..={t} in steps of {}, then up to {} in steps of 100", caps.dpi_min, caps.dpi_step, caps.dpi_max),
        None => format!("dpi must be {}..={} in steps of {}", caps.dpi_min, caps.dpi_max, caps.dpi_step),
    }
}

fn within(name: &str, v: u32, min: u32, max: u32) -> ApiResult<u8> {
    if (min..=max).contains(&v) { Ok(v as u8) } else { Err(ApiError::invalid(format!("{name} must be {min}..={max}"))) }
}

/// `v` within the model's range for `name`; refused when the model does not have the setting.
fn ranged(name: &str, v: u8, range: Option<&shark_core::model::Range>) -> ApiResult<u8> {
    let r = range.ok_or_else(|| ApiError::invalid(format!("this model has no {name} setting")))?;
    within(name, v.into(), r.min, r.max)
}

fn flag(name: &str, v: Option<bool>, supported: bool, current: bool) -> ApiResult<bool> {
    match v {
        Some(_) if !supported => Err(ApiError::invalid(format!("this model has no {name} setting"))),
        Some(on) => Ok(on),
        None => Ok(current),
    }
}

/// Validate and apply `patch` to a copy; `p` changes only when everything is valid.
pub fn apply_patch(p: &mut Profile, patch: &SettingsPatch, caps: &Capabilities) -> ApiResult<()> {
    let mut n = p.clone();
    for s in patch.dpi_stages.iter().flatten() {
        if s.index >= DPI_STAGES {
            return Err(ApiError::invalid(format!("dpi stage index must be 0..={}", DPI_STAGES - 1)));
        }
        if let Some(dpi) = s.dpi {
            n.dpi_units[s.index] = caps.units_from_dpi(dpi).ok_or_else(|| ApiError::invalid(dpi_rule(caps)))?;
        }
        if let Some(on) = s.enabled {
            n.stage_enabled[s.index] = on;
        }
        if let Some(c) = &s.color {
            n.stage_colors[s.index] = parse_color(c)?;
        }
    }
    if let Some(v) = patch.active_stage {
        n.active_stage = within("activeStage", v.into(), 0, DPI_STAGES as u32 - 1)?;
    }
    if let Some(hz) = patch.polling {
        let idx = caps.polling_rates.iter().position(|r| *r == hz).ok_or_else(|| ApiError::invalid("unsupported polling rate"))?;
        n.polling_index = idx as u8;
    }
    if let Some(mm) = patch.lod {
        n.lift_off = caps.lod_options.iter().position(|o| *o == mm).ok_or_else(|| ApiError::invalid("unsupported lod"))? as u8;
    }
    n.ripple_control = flag("rippleControl", patch.ripple_control, caps.supports_ripple_control, n.ripple_control)?;
    n.angle_snap = flag("angleSnap", patch.angle_snap, caps.supports_angle_snap, n.angle_snap)?;
    n.motion_sync = flag("motionSync", patch.motion_sync, caps.supports_motion_sync, n.motion_sync)?;
    if let Some(v) = patch.debounce {
        n.debounce = ranged("debounce", v, caps.debounce.as_ref())?;
    }
    if let Some(v) = patch.sleep_minutes {
        n.sleep_minutes = ranged("sleepMinutes", v, caps.sleep_minutes.as_ref())?;
    }
    if let Some(v) = patch.deep_sleep_minutes {
        n.deep_sleep_minutes = ranged("deepSleepMinutes", v, caps.deep_sleep_minutes.as_ref())?;
    }
    if let Some(l) = &patch.lighting {
        if let Some(v) = l.mode {
            if caps.lighting_modes.is_empty() {
                return Err(ApiError::invalid("this model has no lighting modes"));
            }
            n.light_mode = within("lighting.mode", v.into(), 0, caps.lighting_modes.len() as u32 - 1)?;
        }
        if let Some(v) = l.brightness {
            n.brightness = within("lighting.brightness", v.into(), caps.brightness.min, caps.brightness.max)?;
        }
        if let Some(v) = l.speed {
            n.light_speed = within("lighting.speed", v.into(), caps.light_speed.min, caps.light_speed.max)?;
        }
        if let Some(c) = &l.color {
            n.light_color = parse_color(c)?;
        }
    }
    *p = n;
    Ok(())
}

/// Set button `index` to `action`. Macro bindings copy the library entry (same as the original app).
pub fn set_action(p: &mut Profile, index: usize, action: &ActionRef, library: &[Macro]) -> ApiResult<()> {
    if index >= BUTTON_COUNT {
        return Err(ApiError::invalid(format!("button must be 0..={}", BUTTON_COUNT - 1)));
    }
    let mut b = profile::ButtonConfig::default();
    match action {
        ActionRef::Preset { id } => {
            if *id == ACTION_SHORTCUT || *id == ACTION_MACRO || shark_core::protocol::actions::info(*id).is_none() {
                return Err(ApiError::invalid(format!("unknown preset action 0x{id:02X}")));
            }
            b.action_id = *id;
        }
        ActionRef::Shortcut { key, mods } => {
            if *key == 0 || *mods > 0x0F {
                return Err(ApiError::invalid("shortcut needs a key and mods in 0..=15"));
            }
            (b.action_id, b.shortcut_key, b.shortcut_mods) = (ACTION_SHORTCUT, *key, *mods);
        }
        ActionRef::Macro { macro_id, .. } => {
            let m = macro_id.and_then(|i| library.get(i)).ok_or_else(|| ApiError::not_found("macro"))?;
            b.action_id = ACTION_MACRO;
            b.macro_binding = Some(m.clone());
        }
    }
    p.buttons[index] = b;
    Ok(())
}
