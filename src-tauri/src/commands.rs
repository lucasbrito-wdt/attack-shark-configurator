//! Tauri commands. Argument names are camelCase on the JS side (Tauri converts them).

use crate::catalog::action_catalog;
use crate::dto::*;
use crate::error::ApiResult;
use crate::runtime::AppState;
use tauri::{AppHandle, Manager, State};

type S<'a> = State<'a, AppState>;

#[tauri::command]
pub async fn get_snapshot(app: AppHandle, state: S<'_>) -> ApiResult<Snapshot> {
    Ok(state.with(&app, |i| i.snapshot()))
}

#[tauri::command]
pub async fn list_profiles(app: AppHandle, state: S<'_>) -> ApiResult<Vec<ProfileSummary>> {
    Ok(state.with(&app, |i| i.summaries()))
}

#[tauri::command]
pub async fn create_profile(app: AppHandle, state: S<'_>, name: String) -> ApiResult<Snapshot> {
    state.with(&app, |i| i.create_profile(&name).map(|()| i.snapshot()))
}

#[tauri::command]
pub async fn rename_profile(app: AppHandle, state: S<'_>, id: usize, name: String) -> ApiResult<Snapshot> {
    state.with(&app, |i| i.rename_profile(id, &name).map(|()| i.snapshot()))
}

#[tauri::command]
pub async fn delete_profile(app: AppHandle, state: S<'_>, id: usize) -> ApiResult<Snapshot> {
    state.with(&app, |i| i.delete_profile(id).map(|()| i.snapshot()))
}

/// Switches the selected profile. Does not touch the device: `pendingChanges` becomes true until `apply_all`.
#[tauri::command]
pub async fn select_profile(app: AppHandle, state: S<'_>, id: usize) -> ApiResult<Snapshot> {
    state.with(&app, |i| i.select_profile(id).map(|()| i.snapshot()))
}

#[tauri::command]
pub async fn import_profile(app: AppHandle, state: S<'_>, path: String) -> ApiResult<Snapshot> {
    state.with(&app, |i| i.import_profile(&path).map(|()| i.snapshot()))
}

#[tauri::command]
pub async fn export_profile(app: AppHandle, state: S<'_>, id: usize, path: String) -> ApiResult<()> {
    state.with(&app, |i| i.export_profile(id, &path))
}

#[tauri::command]
pub async fn reset_profile(app: AppHandle, state: S<'_>, id: usize, confirm: bool) -> ApiResult<SettingsResult> {
    state.with(&app, |i| i.reset_profile(id, confirm).map(|(profile, apply)| SettingsResult { profile, apply }))
}

#[tauri::command]
pub async fn update_settings(app: AppHandle, state: S<'_>, patch: SettingsPatch) -> ApiResult<SettingsResult> {
    state.with(&app, |i| i.update_settings(&patch))
}

#[tauri::command]
pub async fn set_button_action(app: AppHandle, state: S<'_>, button: usize, action: ActionRef) -> ApiResult<SettingsResult> {
    state.with(&app, |i| i.set_button_action(button, &action))
}

#[tauri::command]
pub async fn list_actions() -> ApiResult<ActionCatalog> {
    Ok(action_catalog())
}

#[tauri::command]
pub async fn list_macros(app: AppHandle, state: S<'_>) -> ApiResult<Vec<MacroDto>> {
    Ok(state.with(&app, |i| i.macro_views()))
}

#[tauri::command]
pub async fn create_macro(app: AppHandle, state: S<'_>, input: MacroInput) -> ApiResult<Vec<MacroDto>> {
    state.with(&app, |i| i.save_macro(None, &input).map(|()| i.macro_views()))
}

#[tauri::command]
pub async fn update_macro(app: AppHandle, state: S<'_>, id: usize, input: MacroInput) -> ApiResult<Vec<MacroDto>> {
    state.with(&app, |i| i.save_macro(Some(id), &input).map(|()| i.macro_views()))
}

#[tauri::command]
pub async fn delete_macro(app: AppHandle, state: S<'_>, id: usize) -> ApiResult<Vec<MacroDto>> {
    state.with(&app, |i| i.delete_macro(id).map(|()| i.macro_views()))
}

/// Reads the device's current settings (0xA0 read-back). Read-only; applies wait until it ends.
#[tauri::command]
pub async fn read_back(app: AppHandle, state: S<'_>) -> ApiResult<ReadBackDto> {
    let spec = state.with(&app, |i| i.begin_read())?;
    let result = tauri::async_runtime::spawn_blocking(move || shark_core::device::hid::read_back(spec))
        .await
        .unwrap_or_else(|e| Err(shark_core::platform::ReadError::Io(format!("read-back task failed: {e}"))));
    state.with(&app, |i| i.end_read(result))
}

/// Read-only check of the model's DPI encoding (0xA0 read-back of the stored 0x04); the verdict is persisted.
#[tauri::command]
pub async fn verify_dpi_encoding(app: AppHandle, state: S<'_>) -> ApiResult<DpiVerification> {
    verify_dpi(&app, &state).await
}

async fn verify_dpi(app: &AppHandle, state: &AppState) -> ApiResult<DpiVerification> {
    let spec = match state.with(app, |i| i.begin_verify())? {
        Ok(spec) => spec,
        Err(done) => return Ok(done),
    };
    let result = tauri::async_runtime::spawn_blocking(move || shark_core::device::hid::verify_dpi(spec))
        .await
        .unwrap_or_else(|e| Err(shark_core::platform::ReadError::Io(format!("DPI check task failed: {e}"))));
    Ok(state.with(app, |i| i.end_verify(spec, result)))
}

/// The automatic check queued by the state (`Emit::VerifyDpi`) after a cable or receiver connects.
pub fn spawn_dpi_verification(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if let Err(e) = verify_dpi(&app, &state).await {
            eprintln!("automatic DPI check: {}", e.message);
        }
    });
}

/// Rewrites every group of the selected profile (no 0x0C).
#[tauri::command]
pub async fn apply_all(app: AppHandle, state: S<'_>) -> ApiResult<ApplyResult> {
    state.with(&app, |i| i.apply(false, true))
}
