//! Device read-back bookkeeping (split from `state.rs`). The read runs on its own HID handle outside
//! the lock; meanwhile the writer stays idle (applies are deferred like during another apply).

use super::*;
use x3_core::platform::{ReadBack, ReadError};

impl Inner {
    /// Check that a read-back can run now and mark it running. Returns the model to read.
    pub fn begin_read(&mut self) -> ApiResult<&'static ModelSpec> {
        if !self.links.iter().any(|l| *l != LinkKind::Ble) {
            return Err(ApiError::new("unsupported", "read-back needs the cable or the 2.4G receiver"));
        }
        if !self.identified {
            return Err(ApiError::new("unsupported_model", "the connected model is not identified yet"));
        }
        if !self.spec.read_back {
            return Err(ApiError::new("unsupported", format!("{} has no read-back", self.spec.name)));
        }
        if self.sending > 0 {
            return Err(ApiError::new("busy", "an apply is in progress"));
        }
        self.sending += 1;
        Ok(self.spec)
    }

    /// End the read started by `begin_read`; an apply requested meanwhile runs now.
    pub fn end_read(&mut self, result: Result<ReadBack, ReadError>) -> ApiResult<ReadBackDto> {
        self.end_io();
        let code = match &result {
            Err(ReadError::Unsupported(_)) => "unsupported",
            Err(ReadError::Timeout(_)) => "timeout",
            Err(ReadError::Decode(_)) => "decode_failed",
            Err(ReadError::Io(_)) | Ok(_) => "io",
        };
        result.map(|r| read_back_dto(&r, &self.caps)).map_err(|e| ApiError::new(code, e.to_string()))
    }
}

impl Inner {
    /// Release the writer held by a read; an apply requested meanwhile runs now.
    pub(super) fn end_io(&mut self) {
        self.sending = self.sending.saturating_sub(1);
        if std::mem::take(&mut self.dirty)
            && let Err(e) = self.apply(false, false)
        {
            eprintln!("queued apply refused: {}", e.message);
        }
    }
}

pub fn read_back_dto(r: &ReadBack, caps: &Capabilities) -> ReadBackDto {
    ReadBackDto {
        firmware: r.firmware,
        dpi_stages: (0..r.dpi_units.len())
            .map(|i| DpiStageRead {
                index: i,
                dpi: r.dpi_units[i].map(|u| caps.dpi_from_units(u)),
                enabled: r.stage_enabled[i],
                color: r.stage_colors[i].map(hex_color),
            })
            .collect(),
        active_stage: r.active_stage,
        polling: r.polling_index.map(|i| polling_hz(i, caps)),
        lod: r.lift_off.map(|l| l + 1),
        ripple_control: r.ripple_control,
        angle_snap: r.angle_snap,
        motion_sync: r.motion_sync,
        debounce: r.debounce,
        sleep_minutes: r.sleep_minutes,
        deep_sleep_minutes: r.deep_sleep_minutes,
        lighting: LightingRead { mode: r.light_mode, brightness: r.brightness, speed: r.light_speed, color: r.light_color.map(hex_color) },
        buttons: None,
    }
}
