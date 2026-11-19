//! Wire DTOs shared with the web UI. Must match `ui/src/api/types.ts` exactly (camelCase).

use crate::error::{ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use x3_core::model::Capabilities;
use x3_core::profile::{self, ACTION_MACRO, ACTION_SHORTCUT, BUTTON_COUNT, DPI_STAGES, Macro, MacroEvent, Profile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkMode {
    Wired,
    Dongle,
    Ble,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BatterySource {
    VendorEvent,
    /// BLE Battery Service (0x2A19): level only, no charge state.
    BleBas,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    /// Percent; `None` until a vendor battery event arrives, or when the link never reports it.
    pub level: Option<u8>,
    /// From the vendor charge state; family A: the wired PID is present. `None` = unknown.
    pub charging: Option<bool>,
    pub source: BatterySource,
    /// Why `level` is `None` although connected: `"not-reported-over-ble"`.
    pub reason: Option<&'static str>,
    /// Last accepted vendor reading of this model; kept across disconnects and restarts.
    pub last_known: Option<x3_core::storage::LastBattery>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub connected: bool,
    pub mode: LinkMode,
    /// Name as reported by the device (e.g. "X6 Mouse2"), if known.
    pub model: Option<String>,
    pub awake: bool,
    pub battery: Battery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiStage {
    pub index: usize,
    /// 0 = unused slot.
    pub dpi: u32,
    pub enabled: bool,
    /// `#RRGGBB`
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lighting {
    pub mode: u8,
    pub brightness: u8,
    pub speed: u8,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ActionRef {
    Preset { id: u8 },
    Shortcut { key: u8, mods: u8 },
    /// `macro_id` indexes the macro library; `None` when the bound copy matches no library entry.
    Macro { macro_id: Option<usize>, name: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonBinding {
    pub button: usize,
    pub action: ActionRef,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDto {
    pub id: usize,
    pub name: String,
    pub dpi_stages: Vec<DpiStage>,
    /// 0-based.
    pub active_stage: u8,
    /// Hz.
    pub polling: u16,
    /// Lift-off distance in mm.
    pub lod: u8,
    /// `None` when the model does not have the setting (see the `supports*` / range capabilities).
    pub ripple_control: Option<bool>,
    pub angle_snap: Option<bool>,
    pub motion_sync: Option<bool>,
    pub debounce: Option<u8>,
    pub sleep_minutes: Option<u8>,
    pub deep_sleep_minutes: Option<u8>,
    pub lighting: Lighting,
    pub buttons: Vec<ButtonBinding>,
}

/// One DPI stage as read from the device; `None` = not decoded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiStageRead {
    pub index: usize,
    pub dpi: Option<u32>,
    pub enabled: Option<bool>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LightingRead {
    pub mode: Option<u8>,
    pub brightness: Option<u8>,
    pub speed: Option<u8>,
    pub color: Option<String>,
}

/// `read_back` result: the profile shape with every field nullable (`None` = not decoded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadBackDto {
    /// Firmware version (family B `0x0B`), `None` on C1 or when unanswered.
    pub firmware: Option<u16>,
    pub dpi_stages: Vec<DpiStageRead>,
    pub active_stage: Option<u8>,
    pub polling: Option<u16>,
    pub lod: Option<u8>,
    pub ripple_control: Option<bool>,
    pub angle_snap: Option<bool>,
    pub motion_sync: Option<bool>,
    pub debounce: Option<u8>,
    pub sleep_minutes: Option<u8>,
    pub deep_sleep_minutes: Option<u8>,
    pub lighting: LightingRead,
    /// The 0x08 button map is not decoded: always `None`.
    pub buttons: Option<Vec<ButtonBinding>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Inconclusive,
    UnsupportedLink,
    Timeout,
}

/// `verify_dpi_encoding` result; `format` is a `capabilities.dpiEncoding` name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiVerification {
    pub status: VerificationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `dpi-verification` event: `running` when a check starts, `done` with its result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiVerificationEvent {
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<DpiVerification>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoopMode {
    Count,
    UntilKey,
    WhileHeld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroEventDto {
    pub key: u8,
    pub down: bool,
    pub delay_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroDto {
    pub id: usize,
    pub name: String,
    pub events: Vec<MacroEventDto>,
    pub loop_mode: LoopMode,
    pub loop_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroInput {
    pub name: String,
    pub events: Vec<MacroEventDto>,
    pub loop_mode: LoopMode,
    pub loop_count: u8,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DpiStagePatch {
    pub index: usize,
    pub dpi: Option<u32>,
    pub enabled: Option<bool>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LightingPatch {
    pub mode: Option<u8>,
    pub brightness: Option<u8>,
    pub speed: Option<u8>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub dpi_stages: Option<Vec<DpiStagePatch>>,
    pub active_stage: Option<u8>,
    pub polling: Option<u16>,
    pub lod: Option<u8>,
    pub ripple_control: Option<bool>,
    pub angle_snap: Option<bool>,
    pub motion_sync: Option<bool>,
    pub debounce: Option<u8>,
    pub sleep_minutes: Option<u8>,
    pub deep_sleep_minutes: Option<u8>,
    pub lighting: Option<LightingPatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplyStatus {
    /// Packets were handed to the writer; the outcome arrives as `apply-progress`.
    Queued,
    /// Device already matches the profile.
    Nothing,
    NoDevice,
    Asleep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub status: ApplyStatus,
    pub packets: usize,
    /// Groups left out of this apply; the rest was sent (partial success).
    pub skipped_groups: Vec<SkippedGroup>,
}

/// A packet group apply did not send, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedGroup {
    /// `"dpi"`
    pub group: &'static str,
    /// `"unverified_encoding"`
    pub code: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResult {
    pub profile: ProfileDto,
    pub apply: ApplyResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: DeviceStatus,
    pub profile: ProfileDto,
    pub profiles: Vec<ProfileSummary>,
    pub macros: Vec<MacroDto>,
    pub capabilities: Capabilities,
    /// True when the selected profile differs from what is known to be on the device.
    pub pending_changes: bool,
    /// Groups skipped by the last apply (unverified DPI) that the device has not received yet.
    pub pending_groups: Vec<SkippedGroup>,
    /// Stored verdict of the DPI encoding check for this model, `None` before any check.
    pub dpi_encoding: Option<x3_core::storage::DpiEncoding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressState {
    Pending,
    /// Part of the contract; the sender does not report retries yet (docs/API.md).
    #[allow(dead_code)]
    Retry,
    Ok,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProgress {
    pub state: ProgressState,
    pub packets: usize,
    /// Packets not sent by design (polling rate over Bluetooth).
    pub skipped: usize,
    pub failed: usize,
    /// Retry number for `retry`.
    pub attempt: Option<u32>,
    pub reason: Option<String>,
    /// Groups this apply left out on purpose (same as `ApplyResult.skippedGroups`).
    pub skipped_groups: Vec<SkippedGroup>,
    /// Sent over Bluetooth, where the mouse acknowledges nothing: the profile stays pending.
    pub unconfirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiStageChanged {
    /// 0-based.
    pub stage: u8,
    pub dpi: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceEventRaw {
    /// Debug text of the parsed event; only emitted with `X3_LOG_EVENTS=1`.
    pub event: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionEntry {
    pub id: u8,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCategory {
    pub id: &'static str,
    pub label: &'static str,
    pub actions: Vec<ActionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEntry {
    /// HID keyboard usage, or a macro mouse code (0xF1..0xF5).
    pub code: u8,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierEntry {
    pub bit: u8,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCatalog {
    pub categories: Vec<ActionCategory>,
    pub keys: Vec<KeyEntry>,
    pub modifiers: Vec<ModifierEntry>,
}

pub fn hex_color(c: profile::Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

pub fn parse_color(s: &str) -> ApiResult<profile::Rgb> {
    let hex = s.strip_prefix('#').filter(|h| h.len() == 6 && h.is_ascii());
    let byte = |i: usize| hex.and_then(|h| u8::from_str_radix(&h[i..i + 2], 16).ok());
    match (byte(0), byte(2), byte(4)) {
        (Some(r), Some(g), Some(b)) => Ok([r, g, b]),
        _ => Err(ApiError::invalid(format!("invalid color {s:?}, expected #RRGGBB"))),
    }
}

pub fn polling_hz(index: u8, caps: &Capabilities) -> u16 {
    let last = caps.polling_rates.len().saturating_sub(1);
    caps.polling_rates.get(usize::from(index).min(last)).copied().unwrap_or(1000)
}

fn action_ref(b: &profile::ButtonConfig, library: &[Macro]) -> ActionRef {
    match b.action_id {
        ACTION_SHORTCUT => ActionRef::Shortcut { key: b.shortcut_key, mods: b.shortcut_mods },
        ACTION_MACRO => ActionRef::Macro {
            macro_id: b.macro_binding.as_ref().and_then(|m| library.iter().position(|l| l == m)),
            name: b.macro_binding.as_ref().map(|m| m.name.clone()),
        },
        id => ActionRef::Preset { id },
    }
}

pub fn profile_dto(p: &Profile, id: usize, library: &[Macro], caps: &Capabilities) -> ProfileDto {
    ProfileDto {
        id,
        name: p.name.clone(),
        dpi_stages: (0..DPI_STAGES)
            .map(|i| DpiStage { index: i, dpi: caps.dpi_from_units(p.dpi_units[i].min(units_max(caps))), enabled: p.stage_enabled[i], color: hex_color(p.stage_colors[i]) })
            .collect(),
        active_stage: p.active_stage,
        polling: polling_hz(p.polling_index, caps),
        lod: p.lift_off + 1,
        ripple_control: caps.supports_ripple_control.then_some(p.ripple_control),
        angle_snap: caps.supports_angle_snap.then_some(p.angle_snap),
        motion_sync: caps.supports_motion_sync.then_some(p.motion_sync),
        debounce: caps.debounce.as_ref().map(|_| p.debounce),
        sleep_minutes: caps.sleep_minutes.as_ref().map(|_| p.sleep_minutes),
        deep_sleep_minutes: caps.deep_sleep_minutes.as_ref().map(|_| p.deep_sleep_minutes),
        lighting: Lighting { mode: p.light_mode, brightness: p.brightness, speed: p.light_speed, color: hex_color(p.light_color) },
        buttons: p
            .buttons
            .iter()
            .enumerate()
            .map(|(button, b)| ButtonBinding { button, action: action_ref(b, library), label: x3_core::protocol::actions::label(b) })
            .collect(),
    }
}

pub fn macro_dto(m: &Macro, id: usize) -> MacroDto {
    MacroDto {
        id,
        name: m.name.clone(),
        events: m.events.iter().map(|e| MacroEventDto { key: e.key, down: e.down, delay_ms: e.delay_ms }).collect(),
        loop_mode: match m.play_mode {
            profile::PLAY_UNTIL_KEY => LoopMode::UntilKey,
            profile::PLAY_WHILE_HELD => LoopMode::WhileHeld,
            _ => LoopMode::Count,
        },
        loop_count: m.loop_count,
    }
}

pub fn macro_from_input(input: &MacroInput, max_events: usize) -> ApiResult<Macro> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > profile::MAX_NAME_LEN {
        return Err(ApiError::invalid(format!("macro name must have 1..={} characters", profile::MAX_NAME_LEN)));
    }
    if input.events.len() > max_events {
        return Err(ApiError::invalid(format!("a macro holds at most {max_events} events")));
    }
    if input.events.iter().any(|e| e.key == 0) || input.loop_count == 0 {
        return Err(ApiError::invalid("events need a key and loopCount must be at least 1"));
    }
    Ok(Macro {
        name: name.to_string(),
        events: input.events.iter().map(|e| MacroEvent { key: e.key, down: e.down, delay_ms: e.delay_ms }).collect(),
        play_mode: match input.loop_mode {
            LoopMode::Count => profile::PLAY_COUNT,
            LoopMode::UntilKey => profile::PLAY_UNTIL_KEY,
            LoopMode::WhileHeld => profile::PLAY_WHILE_HELD,
        },
        loop_count: input.loop_count,
    })
}

/// Slider units of the model's top DPI (the profile may hold more after a model switch).
fn units_max(caps: &Capabilities) -> u16 {
    caps.units_from_dpi(caps.dpi_max).unwrap_or(u16::MAX)
}

#[path = "dto_patch.rs"]
mod patch;
pub use patch::{apply_patch, set_action};

#[cfg(test)]
#[path = "dto_tests.rs"]
mod tests;
