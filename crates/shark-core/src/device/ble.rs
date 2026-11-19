//! Bluetooth LE transport: write to characteristic 0xFEE3, subscribe to 0xFEE4 (spec 2.5).

use super::manager::{DeviceMsg, Shared};
use super::{DeviceError, LinkKind, Transport};
use crate::platform::{self, PlatformCodec, bk::BkCodec};
use crate::protocol::events::{Event, log_event_bytes, log_line};
use btleplug::api::bleuuid::uuid_from_u16;
use btleplug::api::{Central, CharPropFlags, Characteristic, Manager as _, Peripheral as _, RetrievePeripheralsOptions, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;
use std::thread;
use std::time::Duration;
use tokio::runtime::{Handle, Runtime};

const WRITE_UUID: u16 = 0xFEE3;
const NOTIFY_UUID: u16 = 0xFEE4;
const HID_SERVICE_UUID: u16 = 0x1812;
const BATTERY_SERVICE_UUID: u16 = 0x180F;
const BATTERY_LEVEL_UUID: u16 = 0x2A19;
const SCAN_TIME: Duration = Duration::from_secs(3);
const RETRY_INTERVAL: Duration = Duration::from_secs(5);
const CONNECTED_POLL: Duration = Duration::from_secs(2);

pub struct BleTransport {
    runtime: Handle,
    peripheral: Peripheral,
    characteristic: Characteristic,
}

impl Transport for BleTransport {
    fn kind(&self) -> LinkKind {
        LinkKind::Ble
    }

    /// One whole-packet write with response (closest match to the original reliable write).
    fn write(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        self.runtime
            .block_on(self.peripheral.write(&self.characteristic, data, WriteType::WithResponse))
            .map_err(|e| DeviceError(e.to_string()))
    }
}

pub fn spawn(shared: Shared) {
    thread::spawn(move || match Runtime::new() {
        Ok(rt) => {
            let handle = rt.handle().clone();
            rt.block_on(run(shared, handle));
        }
        Err(e) => shared.notify(DeviceMsg::Error(format!("BLE runtime failed: {e}"))),
    });
}

async fn run(shared: Shared, handle: Handle) {
    let manager = match Manager::new().await {
        Ok(m) => m,
        Err(e) => return shared.notify(DeviceMsg::Error(format!("BLE unavailable: {e}"))),
    };
    let mut connected: Option<Peripheral> = None;
    let mut last_error = String::new();
    loop {
        if let Some(p) = &connected {
            if p.is_connected().await.unwrap_or(false) {
                tokio::time::sleep(CONNECTED_POLL).await;
                continue;
            }
            connected = None;
            shared.set_transport(LinkKind::Ble, None);
            shared.notify(DeviceMsg::LinkDown(LinkKind::Ble));
        }
        match connect(&manager, &shared, &handle).await {
            Ok(Some(p)) => {
                connected = Some(p);
                shared.notify(DeviceMsg::LinkUp(LinkKind::Ble));
            }
            Ok(None) => {}
            Err(e) if e != last_error => {
                shared.notify(DeviceMsg::Error(format!("BLE: {e}")));
                last_error = e;
            }
            Err(_) => {}
        }
        tokio::time::sleep(RETRY_INTERVAL).await;
    }
}

/// What a peripheral exposes after service discovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exposed {
    pub battery_level: bool,
}

/// Selection rule of the original app: no name filter, a peripheral is the mouse when it exposes
/// both FEE3 (write) and FEE4 (notify). Pure over the characteristic UUID list.
pub fn select_characteristics(uuids: &[Uuid]) -> Option<Exposed> {
    let has = |short: u16| uuids.contains(&uuid_from_u16(short));
    (has(WRITE_UUID) && has(NOTIFY_UUID)).then_some(Exposed { battery_level: has(BATTERY_LEVEL_UUID) })
}

/// Standard Battery Level (0x2A19): one byte, 0..=100.
pub fn parse_battery_level(data: &[u8]) -> Option<u8> {
    match data {
        [level] if *level <= 100 => Some(*level),
        _ => None,
    }
}

fn is_permission_error(e: &str) -> bool {
    let e = e.to_lowercase();
    ["permission", "authoriz", "denied", "not allowed", "unauthori"].iter().any(|k| e.contains(k))
}

/// Logged once, unconditionally: without Bluetooth permission nothing else will work.
fn report_permission_once(e: &str) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        eprintln!(
            "Bluetooth permission problem: {e}. On macOS allow Bluetooth for this app or its terminal in \
             System Settings > Privacy & Security > Bluetooth."
        );
    });
}

/// Returns `(peripheral, from_system_retrieval)`; retrieved ones are already connected HID/battery devices.
async fn candidates(adapter: &Adapter) -> Result<Vec<(Peripheral, bool)>, btleplug::Error> {
    // already-connected peripherals first (macOS does not advertise connected mice)
    let mut found = Vec::new();
    for service in [HID_SERVICE_UUID, BATTERY_SERVICE_UUID] {
        let options = RetrievePeripheralsOptions { identifiers: None, services: Some(vec![uuid_from_u16(service)]) };
        found.extend(adapter.retrieve_peripherals(options).await.unwrap_or_default().into_iter().map(|p| (p, true)));
    }
    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(SCAN_TIME).await;
    found.extend(adapter.peripherals().await?.into_iter().map(|p| (p, false)));
    adapter.stop_scan().await.ok();
    let mut seen = Vec::new();
    found.retain(|(p, _)| {
        let id = p.id();
        let fresh = !seen.contains(&id);
        seen.push(id);
        fresh
    });
    Ok(found)
}

async fn connect(manager: &Manager, shared: &Shared, handle: &Handle) -> Result<Option<Peripheral>, String> {
    let adapter = manager.adapters().await.map_err(|e| e.to_string())?.into_iter().next().ok_or("no Bluetooth adapter")?;
    let list = match candidates(&adapter).await {
        Ok(l) => l,
        Err(e) => {
            if is_permission_error(&e.to_string()) {
                report_permission_once(&e.to_string());
            }
            return Err(e.to_string());
        }
    };
    for (p, retrieved) in list {
        let props = p.properties().await.ok().flatten();
        let name = props.as_ref().and_then(|pr| pr.local_name.clone()).unwrap_or_default();
        let connected = p.is_connected().await.unwrap_or(false);
        let advertised = props.map(|pr| pr.services).unwrap_or_default();
        log_line(&format!("ble candidate: name={name:?} id={} connected={connected} retrieved={retrieved} services={advertised:?}", p.id()));
        // never connect to unrelated scan results: only system-retrieved, connected or HID-advertising ones
        if !retrieved && !connected && !advertised.contains(&uuid_from_u16(HID_SERVICE_UUID)) {
            continue;
        }
        if let Some(transport) = open_peripheral(p.clone(), shared, handle, connected, &name).await {
            shared.set_transport(LinkKind::Ble, Some(Box::new(transport)));
            if !name.is_empty() {
                shared.notify(DeviceMsg::DeviceName(name));
            }
            return Ok(Some(p));
        }
    }
    Ok(None)
}

async fn open_peripheral(p: Peripheral, shared: &Shared, handle: &Handle, was_connected: bool, name: &str) -> Option<BleTransport> {
    if !was_connected
        && let Err(e) = p.connect().await
    {
        log_line(&format!("ble connect failed: {e}"));
        return None;
    }
    let found = open_discovered(&p, shared, handle, name).await;
    if found.is_none() && !was_connected {
        p.disconnect().await.ok();
    }
    found
}

async fn open_discovered(p: &Peripheral, shared: &Shared, handle: &Handle, name: &str) -> Option<BleTransport> {
    if let Err(e) = p.discover_services().await {
        log_line(&format!("ble discover_services failed: {e}"));
        return None;
    }
    for service in p.services() {
        let chars: Vec<String> = service.characteristics.iter().map(|c| format!("{} {:?}", c.uuid, c.properties)).collect();
        log_line(&format!("ble service {}: {chars:?}", service.uuid));
    }
    let chars = p.characteristics();
    let uuids: Vec<Uuid> = chars.iter().map(|c| c.uuid).collect();
    let exposed = select_characteristics(&uuids);
    log_line(&format!("ble exposes FEE3+FEE4: {}", exposed.is_some()));
    let exposed = exposed?;
    let write = chars.iter().find(|c| c.uuid == uuid_from_u16(WRITE_UUID))?.clone();
    let notify = chars.iter().find(|c| c.uuid == uuid_from_u16(NOTIFY_UUID))?;
    let subscribed = p.subscribe(notify).await;
    log_line(&format!("ble subscribe FEE4: {subscribed:?}"));
    subscribed.ok()?;
    // The original registers a value-changed handler on every notifiable characteristic (spec 4.3.3).
    for c in chars.iter().filter(|c| c.uuid != notify.uuid && c.properties.intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)) {
        log_line(&format!("ble subscribe {}: {:?}", c.uuid, p.subscribe(c).await));
    }
    let mut stream = match p.notifications().await {
        Ok(s) => s,
        Err(e) => {
            log_line(&format!("ble notifications stream failed: {e}"));
            return None;
        }
    };
    let shared = shared.clone();
    // FEE3/FEE4 is the BK vendor channel: the advertised name picks the model, else the first notification's code
    let mut detected = platform::detect_ble(Some(name), None);
    if let Some(spec) = detected {
        shared.notify(DeviceMsg::Detected(spec));
    }
    // read after detection so the level is remembered under the right model
    if exposed.battery_level
        && let Some(level) = chars.iter().find(|c| c.uuid == uuid_from_u16(BATTERY_LEVEL_UUID))
    {
        // the app decides what a raw byte means (0..=100 is a level); vendor events win over it
        match p.read(level).await {
            Ok(v) => {
                log_line(&format!("bas_battery={:?} (read {v:02X?})", parse_battery_level(&v)));
                if let [byte] = v[..] {
                    shared.dispatch(LinkKind::Ble, Event::BasBattery(byte));
                }
            }
            Err(e) => log_line(&format!("bas_battery read failed: {e}")),
        }
    }
    tokio::spawn(async move {
        while let Some(n) = stream.next().await {
            log_event_bytes(&format!("ble notify {}", n.uuid), &n.value);
            if n.uuid == uuid_from_u16(BATTERY_LEVEL_UUID) {
                log_line(&format!("bas_battery={:?}", parse_battery_level(&n.value)));
                if let [byte] = n.value[..] {
                    shared.dispatch(LinkKind::Ble, Event::BasBattery(byte));
                }
                continue;
            }
            if detected.is_none() && n.uuid == uuid_from_u16(NOTIFY_UUID) {
                detected = platform::detect_ble(None, Some(&n.value));
                if let Some(spec) = detected {
                    shared.notify(DeviceMsg::Detected(spec));
                }
            }
            if let Some(ev) = BkCodec.parse_notification(&n.value) {
                shared.dispatch(LinkKind::Ble, ev);
            }
        }
    });
    Some(BleTransport { runtime: handle.clone(), peripheral: p.clone(), characteristic: write })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(shorts: &[u16]) -> Vec<Uuid> {
        shorts.iter().map(|s| uuid_from_u16(*s)).collect()
    }

    #[test]
    fn selects_by_fee3_and_fee4_regardless_of_name() {
        assert_eq!(select_characteristics(&ids(&[0xFEE3, 0xFEE4])), Some(Exposed { battery_level: false }));
        assert_eq!(select_characteristics(&ids(&[0x2A4D, 0xFEE4, 0x2A19, 0xFEE3])), Some(Exposed { battery_level: true }));
    }

    #[test]
    fn rejects_peripherals_missing_either_characteristic() {
        assert_eq!(select_characteristics(&ids(&[0xFEE3])), None);
        assert_eq!(select_characteristics(&ids(&[0xFEE4, 0x2A19])), None);
        assert_eq!(select_characteristics(&[]), None);
    }

    #[test]
    fn battery_level_is_one_byte_0_to_100() {
        assert_eq!(parse_battery_level(&[100]), Some(100));
        assert_eq!(parse_battery_level(&[0]), Some(0));
        assert_eq!(parse_battery_level(&[101]), None);
        assert_eq!(parse_battery_level(&[]), None);
        assert_eq!(parse_battery_level(&[50, 0]), None);
    }

    #[test]
    fn permission_errors_are_recognized() {
        assert!(is_permission_error("Bluetooth authorization denied"));
        assert!(!is_permission_error("no adapter"));
    }
}
