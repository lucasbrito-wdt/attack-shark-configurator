//! Read-only hardware probe. Never writes configuration: the only outgoing reports are the 0xA0
//! read requests (which stage data for GetFeature) and BLE notification subscriptions.
//!
//! ```text
//! cargo run -p x3-core --bin x3-probe                 # list matching HID interfaces
//! cargo run -p x3-core --bin x3-probe -- --read       # + firmware version and 0x04 read-back (USB)
//! cargo run -p x3-core --bin x3-probe -- --listen 10  # + print input reports / BLE notifications for 10 s
//! ```

use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use x3_core::device::manager::{DeviceManager, DeviceMsg};
use x3_core::platform::bk::frame::{self, CMD_BULK, CMD_READ, CMD_VERSION};
use x3_core::platform::{self, ModelSpec};
use x3_core::protocol::CMD_SENSOR;
use x3_core::protocol::events::force_event_log;

const READ_TRIES: usize = 10;
const READ_DELAY: Duration = Duration::from_millis(100);
const SENSOR_LEN: u8 = 0x38;
const BULK_LEN: usize = 0x80;

struct Args {
    read: bool,
    listen: Option<u64>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { read: false, listen: None };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--read" => args.read = true,
            "--listen" => {
                let secs = it.next().ok_or("--listen needs a number of seconds")?;
                args.listen = Some(secs.parse().map_err(|_| format!("invalid seconds: {secs}"))?);
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(args)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

fn watched(d: &DeviceInfo) -> bool {
    platform::usb_ids().iter().any(|(v, p, _)| *v == d.vendor_id() && *p == d.product_id())
}

fn list(api: &HidApi) -> Vec<&DeviceInfo> {
    let found: Vec<&DeviceInfo> = api.device_list().filter(|d| watched(d)).collect();
    if found.is_empty() {
        println!("no matching USB HID device (BLE links only show up with --listen)");
    }
    for d in &found {
        let model = platform::detect_usb(d.vendor_id(), d.product_id(), d.product_string()).map_or("shared receiver", |m| m.name);
        println!(
            "{:04X}:{:04X} page=0x{:04X} usage=0x{:04X} iface={} product={:?} model={model}",
            d.vendor_id(),
            d.product_id(),
            d.usage_page(),
            d.usage(),
            d.interface_number(),
            d.product_string().unwrap_or("?"),
        );
    }
    found
}

/// SetFeature `A0 <id> <len> ..`, wait for GetFeature 0xA0 ready, then GetFeature `id`.
fn read_back(dev: &HidDevice, id: u8, len: usize) -> Result<Vec<u8>, String> {
    dev.send_feature_report(&frame::read_request(id, len as u8)).map_err(|e| format!("read request: {e}"))?;
    let mut ready = false;
    for _ in 0..READ_TRIES {
        thread::sleep(READ_DELAY);
        let mut buf = [0u8; 8];
        buf[0] = CMD_READ;
        if let Ok(n) = dev.get_feature_report(&mut buf) {
            println!("  A0 status: {}", hex(&buf[..n]));
            if frame::read_ready(&buf[..n]) {
                ready = true;
                break;
            }
        }
    }
    if !ready {
        println!("  A0 never reported ready, reading anyway");
    }
    get_feature(dev, id, len)
}

fn get_feature(dev: &HidDevice, id: u8, len: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; len];
    buf[0] = id;
    let n = dev.get_feature_report(&mut buf).map_err(|e| format!("GetFeature 0x{id:02X}: {e}"))?;
    buf.truncate(n);
    Ok(buf)
}

fn print_sensor(resp: &[u8], spec: Option<&ModelSpec>) {
    println!("  0x04 raw: {}", hex(resp));
    let Some(stages) = frame::decode_sensor(resp) else {
        return println!("  0x04 reply too short or wrong report id");
    };
    if let Some(m) = spec {
        println!("  table model: {} (encoding {}, verified {})", m.name, m.dpi.encoding, m.dpi.verified);
    }
    for s in stages {
        println!("  stage {} enabled={} as family A (u-1): {} dpi | as variant T (R1): {:?} dpi", s.stage + 1, s.enabled, s.as_a, s.as_t);
    }
}

fn read_device(api: &HidApi, infos: &[&DeviceInfo]) {
    let mut pids: Vec<(u16, u16)> = infos.iter().map(|d| (d.vendor_id(), d.product_id())).collect();
    pids.dedup();
    for (vid, pid) in pids {
        let Some(codec) = platform::platform_of_vid(vid).and_then(platform::codec) else { continue };
        let route = codec.usb_route(pid);
        println!("\n== {vid:04X}:{pid:04X} read-back ==");
        let candidates = route.config.iter().filter_map(|(page, usage)| {
            infos.iter().find(|d| d.product_id() == pid && d.usage_page() == *page && d.usage() == *usage)
        });
        let Some(dev) = candidates.filter_map(|d| api.open_path(d.path()).ok()).next() else {
            println!("  no config interface could be opened");
            continue;
        };
        let version = match read_back(&dev, CMD_VERSION, 8) {
            Ok(v) => {
                println!("  0x0B raw: {}", hex(&v));
                if let Some(hint) = frame::version_model_hint(&v) {
                    println!("  C1 version hint: {hint}");
                }
                frame::parse_version(&v)
            }
            Err(e) => {
                println!("  version: {e}");
                None
            }
        };
        let spec = platform::detect_usb(vid, pid, dev.get_product_string().ok().flatten().as_deref());
        match version {
            Some(v) => println!("  firmware 0x{v:04X} ({}.{})", v >> 8, v & 0xFF),
            None => println!("  firmware unknown, trying the per-command read"),
        }
        if version.is_none_or(frame::split_read) {
            match read_back(&dev, CMD_SENSOR, usize::from(SENSOR_LEN)) {
                Ok(r) => print_sensor(&r, spec),
                Err(e) => println!("  0x04: {e}"),
            }
        } else {
            match get_feature(&dev, CMD_BULK, BULK_LEN) {
                Ok(r) => println!("  0x0A bulk (BLE A0 layout, undecoded): {}", hex(&r)),
                Err(e) => println!("  0x0A: {e}"),
            }
        }
    }
}

fn listen(secs: u64) {
    force_event_log();
    println!("\n== listening {secs} s (raw bytes on stderr, parsed messages below) ==");
    let manager = DeviceManager::start(Arc::new(|| ()));
    let end = Instant::now() + Duration::from_secs(secs);
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        match manager.messages.recv_timeout(left) {
            Ok(DeviceMsg::Detected(m)) => println!("Detected({} {}, code 0x{:02X}, dpi {} verified={})", m.id, m.name, m.code, m.dpi.encoding, m.dpi.verified),
            Ok(msg) => println!("{msg:?}"),
            Err(_) => {}
        }
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            eprintln!("usage: x3-probe [--read] [--listen <seconds>]");
            std::process::exit(2);
        }
    };
    match HidApi::new() {
        Ok(api) => {
            let found = list(&api);
            if args.read {
                read_device(&api, &found);
            }
        }
        Err(e) => eprintln!("HID init failed: {e}"),
    }
    if let Some(secs) = args.listen {
        listen(secs);
    }
}
