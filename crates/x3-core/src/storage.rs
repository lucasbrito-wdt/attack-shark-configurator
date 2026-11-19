//! Local JSON persistence in the OS config dir, plus profile and macro import/export.

use crate::profile::{Macro, Profile, sanitize_macro};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

const MAX_IMPORT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub profiles: Vec<Profile>,
    pub selected: usize,
    pub macros: Vec<Macro>,
    /// `ModelSpec::id` of the last detected model.
    pub model: Option<String>,
    /// Last accepted vendor battery reading per `ModelSpec::id`.
    pub last_battery: BTreeMap<String, LastBattery>,
    /// Verdict of the in-app DPI encoding check per `ModelSpec::id`.
    pub dpi_encoding: BTreeMap<String, DpiEncoding>,
}

impl Default for Store {
    fn default() -> Self {
        Self { profiles: vec![Profile::default()], selected: 0, macros: Vec::new(), model: None, last_battery: BTreeMap::new(), dpi_encoding: BTreeMap::new() }
    }
}

/// Link a battery reading arrived on (`usb` = cable).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatteryLink {
    Usb,
    Dongle,
    Ble,
}

/// Last accepted vendor battery reading of one model. Survives disconnects and restarts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastBattery {
    pub level: u8,
    pub charging: bool,
    pub link: BatteryLink,
    /// RFC 3339 UTC, e.g. `2026-11-19T14:03:07Z`.
    pub at: String,
}

/// Result of the read-only DPI encoding check. `verified` with a `format` (a `DpiSpec::encoding`
/// name) when the stored sensor block fits exactly one encoding; `false` and no format when inconclusive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DpiEncoding {
    pub verified: bool,
    pub format: Option<String>,
    /// RFC 3339 UTC.
    pub at: String,
    /// Firmware version read with the verdict (0x0B), if the device reported one.
    pub firmware: Option<u16>,
}

/// RFC 3339 UTC timestamp with second precision (civil-from-days, proleptic Gregorian).
pub fn utc_rfc3339(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

impl Store {
    pub fn sanitize(&mut self) {
        self.profiles.iter_mut().for_each(Profile::sanitize);
        self.macros.iter_mut().for_each(sanitize_macro);
        self.last_battery.retain(|_, b| b.level <= 100);
        if self.profiles.is_empty() {
            self.profiles.push(Profile::default());
        }
        self.selected = self.selected.min(self.profiles.len() - 1);
    }

    pub fn current(&self) -> &Profile {
        &self.profiles[self.selected]
    }

    pub fn current_mut(&mut self) -> &mut Profile {
        &mut self.profiles[self.selected]
    }
}

pub fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "lucasbrito", "attack-shark-x3").map(|d| d.config_dir().join("config.json"))
}

/// Write `value` as JSON atomically (temp file, then rename).
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value).map_err(io::Error::other)?)?;
    fs::rename(&tmp, path)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_IMPORT_BYTES {
        return Err("file is too large".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid file: {e}"))
}

pub fn load_from(path: &Path) -> Store {
    let mut store: Store = read_json(path).unwrap_or_default();
    store.sanitize();
    store
}

pub fn load() -> Store {
    config_path().map_or_else(Store::default, |p| load_from(&p))
}

pub fn save_to(path: &Path, store: &Store) -> io::Result<()> {
    write_json(path, store)
}

pub fn save(store: &Store) -> io::Result<()> {
    config_path().map_or(Ok(()), |p| save_to(&p, store))
}

pub fn export_profile(path: &Path, profile: &Profile) -> io::Result<()> {
    write_json(path, profile)
}

pub fn import_profile(path: &Path) -> Result<Profile, String> {
    let mut profile: Profile = read_json(path)?;
    profile.sanitize();
    Ok(profile)
}

pub fn export_macro(path: &Path, m: &Macro) -> io::Result<()> {
    write_json(path, m)
}

pub fn import_macro(path: &Path) -> Result<Macro, String> {
    let mut m: Macro = read_json(path)?;
    sanitize_macro(&mut m);
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ACTION_MACRO, MacroEvent};

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("x3-test-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join("file.json")
    }

    fn rich_profile() -> Profile {
        let mut p = Profile::named("Gaming");
        p.dpi_units[2] = 400;
        p.buttons[4].action_id = ACTION_MACRO;
        p.buttons[4].macro_binding =
            Some(Macro { events: vec![MacroEvent { key: 4, down: true, delay_ms: 30 }], ..Macro::default() });
        p
    }

    #[test]
    fn profile_json_roundtrip() {
        let p = rich_profile();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<Profile>(&json).unwrap(), p);
    }

    #[test]
    fn export_import_roundtrip_and_store_roundtrip() {
        let path = temp_file("roundtrip");
        let p = rich_profile();
        export_profile(&path, &p).unwrap();
        assert_eq!(import_profile(&path).unwrap(), p);
        let mut store =
            Store { profiles: vec![p.clone(), Profile::default()], selected: 1, macros: vec![Macro::default()], model: Some("x3-max".into()), ..Store::default() };
        let at = utc_rfc3339(UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000));
        store.last_battery.insert("x6".into(), LastBattery { level: 70, charging: false, link: BatteryLink::Dongle, at });
        save_to(&path, &store).unwrap();
        assert_eq!(load_from(&path), store);
    }

    #[test]
    fn rfc3339_utc() {
        let at = |s: u64| utc_rfc3339(UNIX_EPOCH + std::time::Duration::from_secs(s));
        assert_eq!(at(0), "1970-01-01T00:00:00Z");
        assert_eq!(at(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(at(4_102_444_799), "2099-12-31T23:59:59Z");
    }

    #[test]
    fn import_sanitizes_untrusted_values() {
        let path = temp_file("untrusted");
        fs::write(&path, r#"{"name":"","dpi_units":[9999,0,0,0,0,0,0,0],"polling_index":200,"active_stage":99,"buttons":[]}"#).unwrap();
        let p = import_profile(&path).unwrap();
        assert_eq!(p.dpi_units[0], 800);
        assert_eq!((p.polling_index, p.active_stage), (6, 7));
        assert_eq!(p.buttons.len(), 18);
        assert!(!p.name.is_empty());
    }

    #[test]
    fn import_rejects_garbage_and_load_falls_back() {
        let path = temp_file("garbage");
        fs::write(&path, "not json").unwrap();
        assert!(import_profile(&path).is_err());
        assert_eq!(load_from(&path), Store::default());
        assert!(import_profile(&path.with_file_name("missing.json")).is_err());
    }
}
