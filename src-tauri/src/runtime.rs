//! Glue between the pure state machine and Tauri: shared state, the device pump and event emission.

use crate::state::{Emit, Inner};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, PoisonError};
use tauri::Manager;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use x3_core::device::manager::DeviceManager;
use x3_core::storage;

pub struct AppState(Mutex<Inner>);

impl AppState {
    /// Run `f` under the lock, then emit the queued events with the lock released.
    pub fn with<R>(&self, app: &AppHandle, f: impl FnOnce(&mut Inner) -> R) -> R {
        let (result, events) = {
            let mut inner = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let result = f(&mut inner);
            inner.refresh_status();
            (result, std::mem::take(&mut inner.outbox))
        };
        emit_all(app, events);
        result
    }
}

fn emit_all(app: &AppHandle, events: Vec<Emit>) {
    for e in events {
        // a failed emit only means no window is listening
        let _ = match e {
            Emit::Status(p) => {
                crate::tray::update(app, &p);
                app.emit("device-status", p)
            }
            Emit::DpiStage(p) => app.emit("dpi-stage-changed", p),
            Emit::Progress(p) => app.emit("apply-progress", p),
            Emit::Raw(p) => app.emit("device-event-raw", p),
            Emit::DpiVerification(p) => app.emit("dpi-verification", p),
            Emit::VerifyDpi => {
                crate::commands::spawn_dpi_verification(app);
                Ok(())
            }
        };
    }
}

/// Start the device threads and the pump that feeds device messages into the state.
pub fn start(app: &AppHandle) {
    let manager = DeviceManager::start(Arc::new(|| {}));
    let link = manager.shared.link.clone();
    let writer = manager.shared.clone();
    let send = Box::new(move |packets| writer.send_packets(packets));
    let inner = Inner::new(storage::load(), storage::config_path(), link, send);
    app.manage(AppState(Mutex::new(inner)));
    let handle = app.clone();
    std::thread::spawn(move || loop {
        let pump_state = handle.state::<AppState>();
        match manager.messages.recv_timeout(Duration::from_secs(1)) {
            Ok(msg) => pump_state.with(&handle, |i| i.on_msg(msg)),
            // the 1 s tick re-evaluates the dongle heartbeat watchdog
            Err(RecvTimeoutError::Timeout) => pump_state.with(&handle, |_| ()),
            Err(RecvTimeoutError::Disconnected) => break,
        }
    });
}
