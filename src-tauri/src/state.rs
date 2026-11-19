//! App state machine: link tracking, battery honesty, apply bookkeeping and persistence.
//! Pure of Tauri: side effects go through the `send` closure and the `outbox`.

use crate::dto::*;
use crate::error::{ApiError, ApiResult};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use x3_core::device::manager::DeviceMsg;
use x3_core::device::sender::LinkState;
use x3_core::device::LinkKind;
use x3_core::model::Capabilities;
use x3_core::platform::battery;
use x3_core::platform::{self, ModelSpec, PlanError};
use x3_core::profile::{MAX_NAME_LEN, Profile};
use x3_core::protocol::events::{Event, log_line};
use x3_core::storage::{self, Store};

/// A Tauri event waiting to be emitted once the state lock is released.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Emit {
    Status(DeviceStatus),
    DpiStage(DpiStageChanged),
    Progress(ApplyProgress),
    Raw(DeviceEventRaw),
    DpiVerification(DpiVerificationEvent),
    /// Start the automatic DPI encoding check (runtime spawns it outside the lock).
    VerifyDpi,
}

pub struct Inner {
    pub store: Store,
    /// Detected model, else the persisted last-used one, else the X6.
    pub spec: &'static ModelSpec,
    pub caps: Capabilities,
    /// `X3_ALLOW_UNVERIFIED=1`: send DPI even with an encoding not verified on hardware.
    pub allow_unverified: bool,
    pub outbox: Vec<Emit>,
    path: Option<PathBuf>,
    send: Box<dyn Fn(Vec<Vec<u8>>) + Send>,
    link: LinkState,
    /// Last profile known to be on the device; `None` marks every group dirty.
    applied: Option<Profile>,
    in_flight: Option<Profile>,
    in_flight_skipped: Vec<SkippedGroup>,
    /// The in-flight apply goes over Bluetooth (no write acknowledgement, no read-back).
    in_flight_ble: bool,
    /// Groups skipped by the last apply and not yet written; they stay pending until a write succeeds.
    pending_groups: Vec<SkippedGroup>,
    /// A model was detected for the current links; until then apply answers `unsupported_model`.
    identified: bool,
    sending: usize,
    dirty: bool,
    /// A cable or receiver came up: check the DPI encoding once the model is identified.
    auto_verify: bool,
    links: BTreeSet<LinkKind>,
    model: Option<String>,
    heartbeat_awake: Option<bool>,
    battery_level: Option<u8>,
    /// Last 0..=100 byte of the BLE Battery Service (0x2A19); used only when no vendor level is.
    bas_level: Option<u8>,
    charging: Option<bool>,
    last_status: Option<DeviceStatus>,
    log_raw: bool,
}

pub fn clean_name(name: &str) -> ApiResult<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > MAX_NAME_LEN {
        return Err(ApiError::invalid(format!("name must have 1..={MAX_NAME_LEN} characters")));
    }
    Ok(n.to_string())
}

impl Inner {
    pub fn new(store: Store, path: Option<PathBuf>, link: LinkState, send: Box<dyn Fn(Vec<Vec<u8>>) + Send>) -> Self {
        let spec = store.model.as_deref().and_then(platform::by_id).unwrap_or_else(platform::default_model);
        Self {
            store,
            spec,
            caps: spec.capabilities(),
            allow_unverified: platform::allow_unverified(),
            outbox: Vec::new(),
            path,
            send,
            link,
            applied: None,
            in_flight: None,
            in_flight_skipped: Vec::new(),
            in_flight_ble: false,
            pending_groups: Vec::new(),
            identified: false,
            sending: 0,
            dirty: false,
            auto_verify: false,
            links: BTreeSet::new(),
            model: None,
            heartbeat_awake: None,
            battery_level: None,
            bas_level: None,
            charging: None,
            last_status: None,
            log_raw: std::env::var_os("X3_LOG_EVENTS").is_some_and(|v| v == "1"),
        }
    }

    pub fn save(&self) -> ApiResult<()> {
        self.path.as_deref().map_or(Ok(()), |p| storage::save_to(p, &self.store))?;
        Ok(())
    }

    // ---- status -------------------------------------------------------------------------

    fn is_sleeping(&self) -> bool {
        match self.links.iter().next() {
            Some(LinkKind::Dongle) => !self.link.dongle_alive() || self.heartbeat_awake == Some(false),
            Some(LinkKind::Ble) => self.heartbeat_awake == Some(false),
            _ => false,
        }
    }

    pub fn status(&self) -> DeviceStatus {
        let mode = match self.links.iter().next() {
            Some(LinkKind::Wired) => LinkMode::Wired,
            Some(LinkKind::Dongle) => LinkMode::Dongle,
            Some(LinkKind::Ble) => LinkMode::Ble,
            None => LinkMode::None,
        };
        DeviceStatus {
            connected: mode != LinkMode::None,
            mode,
            model: self.model.clone(),
            awake: mode != LinkMode::None && !self.is_sleeping(),
            battery: self.battery(mode),
        }
    }

    /// Model capabilities narrowed to what the primary link carries (e.g. a wired polling cap).
    fn refresh_caps(&mut self) {
        let mut caps = self.spec.capabilities();
        if let (Some(codec), Some(&link)) = (platform::codec(self.spec.platform), self.links.iter().next())
            && let Some(max) = codec.max_polling(self.spec, link)
        {
            caps.polling_rates.retain(|r| *r <= max);
        }
        caps.dpi_verified |= self.dpi_verdict_ok();
        self.caps = caps;
    }

    /// Queue a `device-status` event if anything visible changed (also called by the 1 s watchdog tick).
    pub fn refresh_status(&mut self) {
        let status = self.status();
        if self.last_status.as_ref() != Some(&status) {
            self.last_status = Some(status.clone());
            self.outbox.push(Emit::Status(status));
        }
    }

    // ---- device messages ----------------------------------------------------------------

    pub fn on_msg(&mut self, msg: DeviceMsg) {
        match msg {
            DeviceMsg::LinkUp(kind) => {
                self.links.insert(kind);
                self.applied = None;
                if kind != LinkKind::Wired {
                    self.heartbeat_awake = None;
                }
                self.auto_verify |= kind != LinkKind::Ble;
                self.refresh_caps();
            }
            DeviceMsg::LinkDown(kind) => {
                self.links.remove(&kind);
                self.charging = None;
                if self.links.is_empty() {
                    (self.model, self.heartbeat_awake, self.battery_level) = (None, None, None);
                    self.identified = false;
                }
                if kind == LinkKind::Ble {
                    self.bas_level = None;
                }
                self.refresh_caps();
            }
            DeviceMsg::DeviceName(name) => self.model = Some(name),
            DeviceMsg::Detected(spec) => self.on_detected(spec),
            DeviceMsg::Event(link, ev) => self.on_event(link, ev),
            DeviceMsg::SendDone { sent, skipped, failed } => self.on_send_done(sent, skipped, failed),
            DeviceMsg::Error(e) => eprintln!("device: {e}"),
        }
        self.maybe_auto_verify();
        self.refresh_status();
    }

    /// Capabilities follow the detected model, which is persisted as the offline default.
    fn on_detected(&mut self, spec: &'static ModelSpec) {
        (self.applied, self.identified) = (None, true);
        self.model.get_or_insert_with(|| spec.name.to_string());
        if spec.id == self.spec.id {
            return;
        }
        self.spec = spec;
        self.refresh_caps();
        self.store.model = Some(spec.id.to_string());
        if let Err(e) = self.save() {
            eprintln!("could not save settings: {}", e.message);
        }
    }

    fn on_event(&mut self, link: LinkKind, ev: Event) {
        if self.log_raw {
            self.outbox.push(Emit::Raw(DeviceEventRaw { event: format!("{ev:?}") }));
        }
        match ev {
            Event::DpiStage(v) if (1..=self.spec.dpi.stages).contains(&v) => {
                self.edit_both(|p| p.active_stage = v - 1);
                let dpi = self.caps.dpi_from_units(self.store.current().dpi_units[usize::from(v - 1)]);
                self.outbox.push(Emit::DpiStage(DpiStageChanged { stage: v - 1, dpi }));
            }
            Event::LightMode(m) if usize::from(m) < self.spec.lighting.modes.len() => self.edit_both(|p| p.light_mode = m),
            // only the vendor battery event feeds the battery; invalid levels keep the last value
            Event::Heartbeat { state, level } => {
                if let Some(r) = battery::decode(&self.spec.battery, link, state, level) {
                    self.heartbeat_awake = r.awake.or(self.heartbeat_awake);
                    self.charging = r.charging;
                    if let Some(percent) = r.percent {
                        self.battery_level = Some(percent);
                        self.remember_battery(link, percent);
                    }
                }
            }
            Event::BasBattery(byte) => self.on_bas(byte),
            Event::Unknown { kind, value } => log_line(&format!("unknown event type=0x{kind:04X} value=0x{value:04X}")),
            _ => {}
        }
    }

    /// A device-originated change goes to both the profile and the applied snapshot, so it is
    /// not reported as pending.
    fn edit_both(&mut self, f: impl Fn(&mut Profile)) {
        f(self.store.current_mut());
        if let Some(p) = &mut self.applied {
            f(p);
        }
        if let Err(e) = self.save() {
            eprintln!("could not save settings: {}", e.message);
        }
    }

    fn on_send_done(&mut self, sent: usize, skipped: usize, failed: usize) {
        self.sending = self.sending.saturating_sub(1);
        let snapshot = self.in_flight.take();
        let unconfirmed = std::mem::take(&mut self.in_flight_ble) && failed == 0;
        let (state, reason) = if failed == 0 {
            // a BLE write that lands is indistinguishable from one the firmware drops (observed on the X6)
            if !unconfirmed {
                self.applied = snapshot;
            }
            (ProgressState::Ok, (skipped > 0).then(|| "polling rate is not sent over Bluetooth".to_string()))
        } else {
            (ProgressState::Failed, Some(format!("{failed} packet(s) not delivered")))
        };
        let skipped_groups = std::mem::take(&mut self.in_flight_skipped);
        if failed == 0 || !skipped_groups.is_empty() {
            self.pending_groups = skipped_groups.clone();
        }
        self.outbox.push(Emit::Progress(ApplyProgress { state, packets: sent + skipped + failed, skipped, failed, attempt: None, reason, skipped_groups, unconfirmed }));
        if std::mem::take(&mut self.dirty)
            && failed == 0
            && let Err(e) = self.apply(false, false)
        {
            eprintln!("queued apply refused: {}", e.message);
        }
    }

    // ---- apply --------------------------------------------------------------------------

    pub fn pending_changes(&self) -> bool {
        platform::codec(self.spec.platform)
            .and_then(|c| c.plan(self.spec, self.applied.as_ref(), self.store.current(), false, true).ok())
            .is_some_and(|p| !p.is_empty())
            || !self.pending_groups.is_empty()
    }

    fn plan_error(&self, e: PlanError) -> ApiError {
        match e {
            PlanError::UnverifiedEncoding => ApiError::new(
                "unverified_encoding",
                format!("the {} DPI encoding is not verified on hardware", self.spec.name),
            ),
            PlanError::Unsupported => ApiError::new("unsupported_model", format!("no codec for {}", self.spec.name)),
        }
    }

    /// `full` rewrites every group; `resync` additionally sends 0x0C (callers must have a confirmation).
    /// DPI packets with an unverified encoding are left out (reported in `skipped_groups`) and the
    /// rest is sent. Fails with `unverified_encoding` only for a codec that cannot single them out.
    pub fn apply(&mut self, resync: bool, full: bool) -> ApiResult<ApplyResult> {
        let result = |status, packets| Ok(ApplyResult { status, packets, skipped_groups: Vec::new() });
        if self.links.is_empty() {
            return result(ApplyStatus::NoDevice, 0);
        }
        if self.is_sleeping() {
            return result(ApplyStatus::Asleep, 0);
        }
        if self.sending > 0 {
            self.dirty = true;
            return result(ApplyStatus::Queued, 0);
        }
        if !self.identified {
            return Err(ApiError::new("unsupported_model", "the connected model is not identified yet"));
        }
        let profile = self.store.current().clone();
        let codec = platform::codec(self.spec.platform).ok_or_else(|| self.plan_error(PlanError::Unsupported))?;
        let prev = if full { None } else { self.applied.as_ref() };
        let plan = |prev, allow| codec.plan(self.spec, prev, &profile, resync, allow).map_err(|e| self.plan_error(e));
        let mut packets = plan(prev, true)?;
        if !self.pending_groups.is_empty() && prev.is_some() {
            // the snapshot holds the skipped DPI, so the diff misses it: take it from a full plan
            packets = plan(None, true)?.into_iter().filter(|p| packets.contains(p) || codec.is_unverified_dpi(self.spec, p)).collect();
        }
        let mut skipped_groups = Vec::new();
        if !self.allow_unverified && !self.dpi_verdict_ok() {
            let before = packets.len();
            packets.retain(|p| !codec.is_unverified_dpi(self.spec, p));
            if packets.len() < before {
                skipped_groups.push(SkippedGroup { group: "dpi", code: "unverified_encoding" });
            } else {
                plan(prev, false)?;
            }
        }
        if packets.is_empty() {
            return Ok(ApplyResult { status: ApplyStatus::Nothing, packets: 0, skipped_groups });
        }
        let count = packets.len();
        self.in_flight = Some(profile);
        self.in_flight_skipped = skipped_groups.clone();
        self.in_flight_ble = self.links.iter().next() == Some(&LinkKind::Ble);
        self.sending += 1;
        let progress = ApplyProgress { state: ProgressState::Pending, packets: count, skipped: 0, failed: 0, attempt: None, reason: None, skipped_groups: skipped_groups.clone(), unconfirmed: false };
        self.outbox.push(Emit::Progress(progress));
        (self.send)(packets);
        Ok(ApplyResult { status: ApplyStatus::Queued, packets: count, skipped_groups })
    }

    // ---- views --------------------------------------------------------------------------

    pub fn profile_view(&self, id: usize) -> ApiResult<ProfileDto> {
        let p = self.store.profiles.get(id).ok_or_else(|| ApiError::not_found("profile"))?;
        Ok(profile_dto(p, id, &self.store.macros, &self.caps))
    }

    pub fn current_view(&self) -> ProfileDto {
        profile_dto(self.store.current(), self.store.selected, &self.store.macros, &self.caps)
    }

    pub fn summaries(&self) -> Vec<ProfileSummary> {
        self.store.profiles.iter().enumerate().map(|(id, p)| ProfileSummary { id, name: p.name.clone() }).collect()
    }

    pub fn macro_views(&self) -> Vec<MacroDto> {
        self.store.macros.iter().enumerate().map(|(i, m)| macro_dto(m, i)).collect()
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            status: self.status(),
            profile: self.current_view(),
            profiles: self.summaries(),
            macros: self.macro_views(),
            capabilities: self.caps.clone(),
            pending_changes: self.pending_changes(),
            pending_groups: self.pending_groups.clone(),
            dpi_encoding: self.store.dpi_encoding.get(self.spec.id).cloned(),
        }
    }

    // ---- profile list -------------------------------------------------------------------

    fn select_index(&mut self, id: usize) {
        self.store.selected = id;
        self.applied = None;
    }

    pub fn create_profile(&mut self, name: &str) -> ApiResult<()> {
        self.store.profiles.push(Profile::named(&clean_name(name)?));
        self.select_index(self.store.profiles.len() - 1);
        self.save()
    }

    pub fn rename_profile(&mut self, id: usize, name: &str) -> ApiResult<()> {
        let name = clean_name(name)?;
        self.store.profiles.get_mut(id).ok_or_else(|| ApiError::not_found("profile"))?.name = name;
        self.save()
    }

    pub fn delete_profile(&mut self, id: usize) -> ApiResult<()> {
        if id >= self.store.profiles.len() {
            return Err(ApiError::not_found("profile"));
        }
        if self.store.profiles.len() == 1 {
            return Err(ApiError::new("last_profile", "keep at least one profile"));
        }
        self.store.profiles.remove(id);
        let selected = if id < self.store.selected { self.store.selected - 1 } else { self.store.selected.min(self.store.profiles.len() - 1) };
        if id <= self.store.selected {
            self.applied = None;
        }
        self.store.selected = selected;
        self.save()
    }

    pub fn select_profile(&mut self, id: usize) -> ApiResult<()> {
        if id >= self.store.profiles.len() {
            return Err(ApiError::not_found("profile"));
        }
        self.select_index(id);
        self.save()
    }

    pub fn import_profile(&mut self, path: &str) -> ApiResult<()> {
        let profile = storage::import_profile(Path::new(path)).map_err(ApiError::invalid)?;
        self.store.profiles.push(profile);
        self.select_index(self.store.profiles.len() - 1);
        self.save()
    }

    pub fn export_profile(&self, id: usize, path: &str) -> ApiResult<()> {
        let p = self.store.profiles.get(id).ok_or_else(|| ApiError::not_found("profile"))?;
        Ok(storage::export_profile(Path::new(path), p)?)
    }

    /// Reset to factory defaults (keeps the name). Only the selected profile talks to the device,
    /// and only here the risky 0x0C resync is sent.
    pub fn reset_profile(&mut self, id: usize, confirm: bool) -> ApiResult<(ProfileDto, ApplyResult)> {
        if !confirm {
            return Err(ApiError::new("confirm_required", "reset sends the 0x0C resync command and needs confirm: true"));
        }
        if self.sending > 0 {
            return Err(ApiError::new("busy", "an apply is in progress"));
        }
        let old = self.store.profiles.get_mut(id).ok_or_else(|| ApiError::not_found("profile"))?;
        *old = Profile::named(&old.name.clone());
        self.save()?;
        let apply = if id == self.store.selected {
            self.applied = None;
            self.apply(true, true)?
        } else {
            ApplyResult { status: ApplyStatus::Nothing, packets: 0, skipped_groups: Vec::new() }
        };
        Ok((self.profile_view(id)?, apply))
    }

    // ---- settings, buttons, macros ------------------------------------------------------

    pub fn update_settings(&mut self, patch: &SettingsPatch) -> ApiResult<SettingsResult> {
        let caps = self.caps.clone();
        apply_patch(self.store.current_mut(), patch, &caps)?;
        self.save()?;
        let apply = self.apply(false, false)?;
        Ok(SettingsResult { profile: self.current_view(), apply })
    }

    pub fn set_button_action(&mut self, button: usize, action: &ActionRef) -> ApiResult<SettingsResult> {
        let library = self.store.macros.clone();
        set_action(self.store.current_mut(), button, action, &library)?;
        self.save()?;
        let apply = self.apply(false, false)?;
        Ok(SettingsResult { profile: self.current_view(), apply })
    }

    pub fn save_macro(&mut self, id: Option<usize>, input: &MacroInput) -> ApiResult<()> {
        let m = macro_from_input(input, self.caps.max_macro_events)?;
        match id {
            None => self.store.macros.push(m),
            Some(i) => *self.store.macros.get_mut(i).ok_or_else(|| ApiError::not_found("macro"))? = m,
        }
        self.save()
    }

    pub fn delete_macro(&mut self, id: usize) -> ApiResult<()> {
        if id >= self.store.macros.len() {
            return Err(ApiError::not_found("macro"));
        }
        self.store.macros.remove(id);
        self.save()
    }
}

#[path = "state_battery.rs"]
mod battery_state;
#[path = "state_verify.rs"]
mod verify;
#[path = "state_read.rs"]
pub(crate) mod read;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
