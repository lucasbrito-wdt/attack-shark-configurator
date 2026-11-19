//! In-app DPI encoding check (split from `state.rs`). Read-only: the stored 0x04 block is read over
//! the cable or 2.4G receiver on its own handle while the writer stays idle (like `read_back`), and the
//! verdict is persisted per model. A verified format equal to the model's encoding enables DPI writes.

use super::*;
use shark_core::platform::{DpiVerdict, ReadError};
use shark_core::storage::DpiEncoding;

impl Inner {
    /// The stored verdict confirms the encoding this model's DPI writes use.
    pub(super) fn dpi_verdict_ok(&self) -> bool {
        let stored = self.store.dpi_encoding.get(self.spec.id);
        !self.spec.dpi.verified && stored.is_some_and(|v| v.verified && v.format.as_deref() == Some(self.spec.dpi.encoding))
    }

    /// Queue the automatic check once per cable/receiver connection, when the identified model's DPI
    /// is unverified and no verdict is stored.
    pub(super) fn maybe_auto_verify(&mut self) {
        if !self.auto_verify || !self.identified || !self.links.iter().any(|l| *l != LinkKind::Ble) {
            return;
        }
        self.auto_verify = false;
        if !self.spec.dpi.verified && !self.store.dpi_encoding.contains_key(self.spec.id) {
            self.outbox.push(Emit::VerifyDpi);
        }
    }

    fn verification(&mut self, status: VerificationStatus, format: Option<&str>, reason: Option<String>) -> DpiVerification {
        let result = DpiVerification { status, format: format.map(str::to_string), reason };
        self.outbox.push(Emit::DpiVerification(DpiVerificationEvent { state: "done", result: Some(result.clone()) }));
        result
    }

    /// `Ok(Ok(spec))`: read `spec` outside the lock, then call `end_verify`. `Ok(Err(result))`: answered
    /// without touching the device.
    pub fn begin_verify(&mut self) -> ApiResult<Result<&'static ModelSpec, DpiVerification>> {
        if !self.links.is_empty() && !self.identified {
            return Err(ApiError::new("unsupported_model", "the connected model is not identified yet"));
        }
        if self.spec.dpi.verified {
            let reason = Some("the model's DPI encoding is verified on hardware".to_string());
            return Ok(Err(self.verification(VerificationStatus::Verified, Some(self.spec.dpi.encoding), reason)));
        }
        if !self.links.iter().any(|l| *l != LinkKind::Ble) {
            let reason = if self.links.is_empty() {
                "connect the cable or the 2.4G receiver"
            } else {
                "the Bluetooth read-back blob (family-b 4.3) is not decoded; connect the cable or the 2.4G receiver"
            };
            return Ok(Err(self.verification(VerificationStatus::UnsupportedLink, None, Some(reason.into()))));
        }
        if self.sending > 0 {
            return Err(ApiError::new("busy", "an apply or a read is in progress"));
        }
        self.sending += 1;
        self.outbox.push(Emit::DpiVerification(DpiVerificationEvent { state: "running", result: None }));
        Ok(Ok(self.spec))
    }

    /// End the check started by `begin_verify`. Only a judged block is persisted; link errors are not.
    pub fn end_verify(&mut self, spec: &'static ModelSpec, result: Result<DpiVerdict, ReadError>) -> DpiVerification {
        let (status, format, reason) = match result {
            Ok(v) => {
                let at = storage::utc_rfc3339(std::time::SystemTime::now());
                let format = v.format.map(str::to_string);
                let stored = DpiEncoding { verified: v.format.is_some(), format, at, firmware: v.firmware };
                self.store.dpi_encoding.insert(spec.id.to_string(), stored);
                if let Err(e) = self.save() {
                    eprintln!("could not save settings: {}", e.message);
                }
                self.refresh_caps();
                let mut reason = v.reason;
                if let Some(f) = v.format.filter(|f| *f != spec.dpi.encoding) {
                    let why = reason.unwrap_or_default();
                    reason = Some(format!("{why}; {} writes the {} encoding, not {f}, so DPI writes stay disabled", spec.name, spec.dpi.encoding));
                }
                let status = if v.format.is_some() { VerificationStatus::Verified } else { VerificationStatus::Inconclusive };
                (status, v.format, reason)
            }
            Err(ReadError::Unsupported(m)) => (VerificationStatus::UnsupportedLink, None, Some(m)),
            Err(ReadError::Timeout(m)) => (VerificationStatus::Timeout, None, Some(m)),
            Err(e) => (VerificationStatus::Inconclusive, None, Some(e.to_string())),
        };
        // after the verdict is stored, so an apply queued meanwhile already includes the DPI
        self.end_io();
        self.verification(status, format, reason)
    }
}
