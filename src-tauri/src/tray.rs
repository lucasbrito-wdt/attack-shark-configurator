//! System tray / menu bar battery indicator: a runtime-drawn battery glyph, title, tooltip and menu.

use crate::dto::{BatterySource, DeviceStatus, LinkMode};
use crate::tray_glyph;
use crate::tray_i18n::{Key, Locale, fill, tr};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use x3_core::storage::{BatteryLink, LastBattery};

const TRAY_ID: &str = "battery";
pub const LOW_LEVEL: u8 = 20;
const REFRESH: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Usb,
    Dongle,
    Ble,
}

impl Link {
    fn label(self, loc: Locale) -> &'static str {
        tr(loc, match self {
            Link::Usb => Key::LinkUsb,
            Link::Dongle => Key::LinkDongle,
            Link::Ble => Key::LinkBle,
        })
    }
}

/// Last battery reading persisted by the core, shown when there is no live level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Known {
    pub level: u8,
    pub link: Link,
    pub at: SystemTime,
}

impl Known {
    fn from_stored(l: &LastBattery) -> Option<Self> {
        let link = match l.link {
            BatteryLink::Usb => Link::Usb,
            BatteryLink::Dongle => Link::Dongle,
            BatteryLink::Ble => Link::Ble,
        };
        Some(Self { level: l.level, link, at: parse_rfc3339_utc(&l.at)? })
    }
}

/// Inverse of `x3_core::storage::utc_rfc3339` (`YYYY-MM-DDTHH:MM:SSZ`, UTC only).
pub fn parse_rfc3339_utc(s: &str) -> Option<SystemTime> {
    let b = s.as_bytes();
    if b.len() != 20 || !s.is_ascii() || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' || b[19] != b'Z' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| s[r].parse::<i64>().ok();
    let (y, m, d, hh, mm, ss) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss).ok()?;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

/// Everything the tray needs to render; independent of Tauri so it can be unit tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub connected: bool,
    pub link: Option<Link>,
    pub model: Option<String>,
    pub level: Option<u8>,
    pub charging: bool,
    /// Level comes from the standard BLE Battery Service: label the source, no charge state.
    pub via_bas: bool,
    pub known: Option<Known>,
}

impl View {
    pub fn from_status(s: &DeviceStatus) -> Self {
        let link = match s.mode {
            LinkMode::Wired => Some(Link::Usb),
            LinkMode::Dongle => Some(Link::Dongle),
            LinkMode::Ble => Some(Link::Ble),
            LinkMode::None => None,
        };
        Self {
            connected: s.connected,
            link,
            model: s.model.clone(),
            level: s.battery.level,
            charging: s.battery.charging.unwrap_or(false),
            via_bas: s.battery.source == BatterySource::BleBas,
            known: s.battery.last_known.as_ref().and_then(Known::from_stored),
        }
    }

    pub fn live(&self) -> Option<u8> {
        self.level.filter(|_| self.connected)
    }
}

pub fn age_text(age: Duration, loc: Locale) -> String {
    let s = age.as_secs();
    let (key, n) = match s {
        0..=59 => return tr(loc, Key::AgeNow).into(),
        60..=3599 => (Key::AgeMin, s / 60),
        3600..=86_399 => (Key::AgeHour, s / 3600),
        _ => (Key::AgeDay, s / 86_400),
    };
    fill(tr(loc, key), &[("n", &n.to_string())])
}

pub fn title(v: &View, loc: Locale) -> String {
    if let Some(l) = v.live() {
        let low = if l <= LOW_LEVEL && !v.charging { "!" } else { "" };
        return format!("{l}%{low}{}", if v.charging { format!(" {}", tr(loc, Key::TitleCharging)) } else { String::new() });
    }
    match v.known {
        Some(k) => format!("~{}%", k.level),
        None => "--".into(),
    }
}

/// One-sentence description, used for the tooltip and the disabled menu line.
pub fn sentence(v: &View, now: SystemTime, loc: Locale) -> String {
    if let Some(l) = v.live() {
        if v.via_bas {
            return fill(tr(loc, Key::LiveVia), &[("level", &l.to_string()), ("source", Link::Ble.label(loc))]);
        }
        let key = if v.charging { Key::LiveCharging } else { Key::Live };
        return fill(tr(loc, key), &[("level", &l.to_string())]);
    }
    match v.known {
        Some(k) => {
            let age = age_text(now.duration_since(k.at).unwrap_or_default(), loc);
            let text = fill(tr(loc, Key::Stale), &[("level", &k.level.to_string()), ("link", k.link.label(loc)), ("age", &age)]);
            if v.link == Some(Link::Ble) { format!("{text} {}", tr(loc, Key::BleNote)) } else { text }
        }
        None if v.connected => tr(loc, Key::NotReported).into(),
        None => tr(loc, Key::Disconnected).into(),
    }
}

fn menu_line(v: &View, now: SystemTime, loc: Locale) -> String {
    let mut parts = vec![sentence(v, now, loc).trim_end_matches('.').to_string()];
    parts.extend(v.model.clone());
    parts.extend(v.link.filter(|_| v.connected).map(|l| l.label(loc).to_string()));
    parts.join(" - ")
}

struct TrayState {
    tray: TrayIcon,
    status_item: MenuItem<tauri::Wry>,
    open: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
    view: Mutex<View>,
    locale: Mutex<Locale>,
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn render(app: &AppHandle) {
    let Some(st) = app.try_state::<TrayState>() else { return };
    let view = st.view.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let loc = *st.locale.lock().unwrap_or_else(PoisonError::into_inner);
    let now = SystemTime::now();
    let template = cfg!(target_os = "macos");
    let g = tray_glyph::render(&view, template);
    // tray calls fail only when the tray is being torn down
    let _ = st.tray.set_icon(Some(Image::new_owned(g.rgba, g.w, g.h)));
    // set_icon drops the template flag on macOS, so it must be re-applied every time
    #[cfg(target_os = "macos")]
    let _ = st.tray.set_icon_as_template(true);
    let _ = st.tray.set_tooltip(Some(sentence(&view, now, loc)));
    #[cfg(target_os = "macos")]
    let _ = st.tray.set_title(Some(title(&view, loc)));
    let _ = st.status_item.set_text(menu_line(&view, now, loc));
    let _ = st.open.set_text(tr(loc, Key::OpenApp));
    let _ = st.quit.set_text(tr(loc, Key::Quit));
}

/// Push a new device status to the tray. Cheap; safe to call from any thread.
pub fn update(app: &AppHandle, status: &DeviceStatus) {
    if let Some(st) = app.try_state::<TrayState>() {
        *st.view.lock().unwrap_or_else(PoisonError::into_inner) = View::from_status(status);
        render(app);
    }
}

/// Switch the tray language (`pt-BR`, `en`, `es`; anything else is English) and re-render.
#[tauri::command]
pub fn set_locale(app: AppHandle, locale: String) {
    if let Some(st) = app.try_state::<TrayState>() {
        *st.locale.lock().unwrap_or_else(PoisonError::into_inner) = Locale::parse(&locale);
        render(&app);
    }
}

/// Build the tray icon and start the per-minute refresh of the relative age.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let view = View { connected: false, link: None, model: None, level: None, charging: false, via_bas: false, known: None };
    let loc = Locale::system();
    let status_item = MenuItem::with_id(app, "status", menu_line(&view, SystemTime::now(), loc), false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", tr(loc, Key::OpenApp), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", tr(loc, Key::Quit), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&status_item, &PredefinedMenuItem::separator(app)?, &open, &quit])?;

    let g = tray_glyph::render(&view, cfg!(target_os = "macos"));
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::new_owned(g.rgba, g.w, g.h))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_window(tray.app_handle());
            }
        });
    #[cfg(target_os = "macos")]
    let builder = builder.icon_as_template(true);
    let tray = builder.build(app)?;

    app.manage(TrayState { tray, status_item, open, quit, view: Mutex::new(view), locale: Mutex::new(loc) });
    render(app);
    let handle = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(REFRESH);
        render(&handle);
    });
    Ok(())
}

#[cfg(test)]
#[path = "tray_tests.rs"]
mod tests;
