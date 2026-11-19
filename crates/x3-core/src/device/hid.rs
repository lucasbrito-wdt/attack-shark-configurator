//! hidapi transport for the wired and receiver links of every registered model.

use super::{DeviceError, LinkKind, Transport};
use crate::platform::bk::frame;
use crate::platform::{self, DpiVerdict, ModelSpec, PlatformCodec, ReadBack, ReadError, ReadFn, UsbRoute};
use crate::protocol::events::Event;
use crate::protocol::{USAGE_PAGE_CONFIG, USAGE_PAGE_EVENTS};
use hidapi::{HidApi, HidDevice};
use std::thread;
use std::time::Duration;

/// SetFeature -> GetFeature settle time (XVI vendor: 30 ms).
const QUERY_GAP: Duration = Duration::from_millis(30);
/// Feature reply size for queries: 64-byte payload plus the report ID.
const QUERY_LEN: usize = 65;
const ACK_TRIES: usize = 10;
/// 0xA0 ready polls and their gap (family-b 3.6: 10 tries, 100 ms).
const READ_TRIES: usize = 10;
const READ_DELAY: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IfaceUsage {
    pub usage_page: u16,
    pub usage: u16,
}

/// Indices (into the enumerated interfaces of one VID/PID) to use for each role.
#[derive(Debug, PartialEq, Eq)]
pub struct Selection {
    pub config: Vec<usize>,
    pub events: Vec<usize>,
}

/// Match by HIDP_CAPS usage (spec 1.2): config = 0x000B/0, events = 0x000A/0.
/// Fallback when nothing matches (Linux hidraw may report 0/0 for every interface): use every
/// interface; the transport pins the one where `send_feature_report` succeeds.
pub fn select_interfaces(ifaces: &[IfaceUsage]) -> Selection {
    select_for(ifaces, &[(USAGE_PAGE_CONFIG, 0)], &[(USAGE_PAGE_EVENTS, 0)])
}

/// Per role, the interfaces of the first (usage page, usage) preference that exists; every interface
/// when none does.
pub fn select_for(ifaces: &[IfaceUsage], config: &[(u16, u16)], events: &[(u16, u16)]) -> Selection {
    let pick = |prefs: &[(u16, u16)]| -> Vec<usize> {
        prefs
            .iter()
            .map(|&(page, usage)| {
                ifaces.iter().enumerate().filter(|(_, i)| i.usage_page == page && i.usage == usage).map(|(n, _)| n).collect::<Vec<_>>()
            })
            .find(|found| !found.is_empty())
            .unwrap_or_else(|| (0..ifaces.len()).collect())
    };
    Selection { config: pick(config), events: pick(events) }
}

pub struct HidTransport {
    kind: LinkKind,
    devices: Vec<HidDevice>,
    pinned: Option<usize>,
    pid: u16,
    output_reports: bool,
    codec: &'static dyn PlatformCodec,
}

impl HidTransport {
    fn write_one(&self, device: &HidDevice, data: &[u8]) -> Result<(), DeviceError> {
        if !self.output_reports {
            let Some(len) = self.codec.ack_reply_len() else {
                return device.send_feature_report(data).map_err(|e| DeviceError(e.to_string()));
            };
            for _ in 0..ACK_TRIES {
                device.send_feature_report(data).map_err(|e| DeviceError(e.to_string()))?;
                thread::sleep(QUERY_GAP);
                if get_feature(device, data[0], len).is_some_and(|r| self.codec.is_ack(data, &r)) {
                    return Ok(());
                }
            }
            return Err(DeviceError(format!("no ack after {ACK_TRIES} tries")));
        }
        // family-b unknowns #7/#9: which FA65 / X3 PRO collection takes the output report is not settled
        if !crate::platform::allow_unverified() {
            return Err(DeviceError("writes over the 4K link are unverified; set X3_ALLOW_UNVERIFIED=1".into()));
        }
        let frames = self.codec.frame(self.pid, data);
        if frames.is_empty() {
            return Err(DeviceError("this packet is not supported on a framed (4K) link".into()));
        }
        for f in frames {
            device.write(&f).map_err(|e| DeviceError(e.to_string()))?;
        }
        Ok(())
    }
}

fn get_feature(device: &HidDevice, report_id: u8, len: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len];
    buf[0] = report_id;
    let n = device.get_feature_report(&mut buf).ok()?;
    buf.truncate(n);
    Some(buf)
}

impl HidTransport {
    /// SetFeature the framed `request`, wait, then GetFeature the reply (on the pinned interface first).
    fn query(&self, request: &[u8]) -> Option<Vec<u8>> {
        let frames = self.codec.frame(self.pid, request);
        let report_id = frames.last()?.first().copied()?;
        let order = self.pinned.into_iter().chain((0..self.devices.len()).filter(|i| Some(*i) != self.pinned));
        order.map(|i| &self.devices[i]).find_map(|d| {
            if !frames.iter().all(|f| d.send_feature_report(f).is_ok()) {
                return None;
            }
            thread::sleep(QUERY_GAP);
            get_feature(d, report_id, QUERY_LEN)
        })
    }

    /// Resolve the model behind a shared PID, if the codec has an identity query for it.
    pub fn identify(&self) -> Option<&'static ModelSpec> {
        self.codec.identify(&self.query(&self.codec.identify_request(self.pid)?)?)
    }
}

impl Transport for HidTransport {
    fn kind(&self) -> LinkKind {
        self.kind
    }

    fn write(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        let candidates: Vec<usize> = self.pinned.map_or_else(|| (0..self.devices.len()).collect(), |p| vec![p]);
        let mut last = DeviceError("no HID interface".into());
        for i in candidates {
            match self.write_one(&self.devices[i], data) {
                Ok(()) => {
                    self.pinned = Some(i);
                    return Ok(());
                }
                Err(e) => last = DeviceError(e.to_string()),
            }
        }
        Err(last)
    }

    fn poll(&mut self) -> Option<Event> {
        let reply = self.query(&self.codec.battery_request()?)?;
        self.codec.parse_battery_reply(&reply)
    }
}

/// SetFeature `A0 <id> <len> ..`, poll GetFeature 0xA0 until ready, then GetFeature `id` (family-b 3.6).
/// The request only stages data for reading; it never changes the configuration.
fn read_feature(device: &HidDevice, id: u8, len: usize) -> Result<Vec<u8>, ReadError> {
    let len8 = u8::try_from(len).map_err(|_| ReadError::Decode(format!("read length {len} does not fit the request")))?;
    device.send_feature_report(&frame::read_request(id, len8)).map_err(|e| ReadError::Io(format!("read request 0x{id:02X}: {e}")))?;
    let ready = (0..READ_TRIES).any(|_| {
        thread::sleep(READ_DELAY);
        get_feature(device, frame::CMD_READ, 8).is_some_and(|r| frame::read_ready(&r))
    });
    if !ready {
        return Err(ReadError::Timeout(format!("0x{id:02X} was not ready after {READ_TRIES} polls")));
    }
    get_feature(device, id, len).ok_or_else(|| ReadError::Io(format!("GetFeature 0x{id:02X} failed")))
}

/// Read the current settings of `spec` over its cable or 2.4G receiver, on a separate handle (the
/// caller must keep the writer idle meanwhile). Each config interface candidate is tried in turn.
pub fn read_back(spec: &ModelSpec) -> Result<ReadBack, ReadError> {
    if !spec.read_back {
        return Err(ReadError::Unsupported(format!("{} has no read-back", spec.name)));
    }
    with_reader(spec, &mut |codec, read| codec.read_back(spec, read))
}

/// Read-only DPI encoding check of `spec` (`PlatformCodec::verify_dpi`), same link rules as `read_back`.
pub fn verify_dpi(spec: &ModelSpec) -> Result<DpiVerdict, ReadError> {
    with_reader(spec, &mut |codec, read| codec.verify_dpi(spec, read))
}

type Reader<'a, T> = dyn FnMut(&dyn PlatformCodec, &mut ReadFn<'_>) -> Result<T, ReadError> + 'a;

fn with_reader<T>(spec: &ModelSpec, f: &mut Reader<'_, T>) -> Result<T, ReadError> {
    let codec = platform::codec(spec.platform).ok_or_else(|| ReadError::Unsupported(format!("no codec for {}", spec.name)))?;
    let api = HidApi::new().map_err(|e| ReadError::Io(format!("HID init failed: {e}")))?;
    let vid = spec.ids.vid;
    let pid = [spec.ids.wired, spec.ids.dongle].into_iter().flatten().find(|&p| is_present(&api, vid, p));
    let pid = pid.ok_or_else(|| ReadError::Unsupported("read-back needs the cable or the 2.4G receiver".into()))?;
    let UsbRoute { config, events, output_reports } = codec.usb_route(pid);
    if output_reports {
        return Err(ReadError::Unsupported("read-back over the framed (4K) link is not decoded".into()));
    }
    let infos: Vec<_> = api.device_list().filter(|d| d.vendor_id() == vid && d.product_id() == pid).collect();
    let usages: Vec<IfaceUsage> = infos.iter().map(|d| IfaceUsage { usage_page: d.usage_page(), usage: d.usage() }).collect();
    let mut last = ReadError::Io("no config interface could be opened".into());
    for i in select_for(&usages, config, events).config {
        let Ok(device) = api.open_path(infos[i].path()) else { continue };
        match f(codec, &mut |id, len| read_feature(&device, id, len)) {
            Err(e @ ReadError::Io(_)) => last = e,
            other => return other,
        }
    }
    Err(last)
}

pub struct HidLink {
    pub transport: HidTransport,
    /// Handles to read device events from.
    pub readers: Vec<HidDevice>,
    /// USB product string of the first interface.
    pub product: Option<String>,
}

pub fn is_present(api: &HidApi, vid: u16, pid: u16) -> bool {
    api.device_list().any(|d| d.vendor_id() == vid && d.product_id() == pid)
}

pub fn open_link(api: &HidApi, kind: LinkKind, vid: u16, pid: u16, codec: &'static dyn PlatformCodec) -> Result<HidLink, DeviceError> {
    let infos: Vec<_> = api.device_list().filter(|d| d.vendor_id() == vid && d.product_id() == pid).collect();
    let usages: Vec<IfaceUsage> =
        infos.iter().map(|d| IfaceUsage { usage_page: d.usage_page(), usage: d.usage() }).collect();
    let UsbRoute { config, events, output_reports } = codec.usb_route(pid);
    let selection = select_for(&usages, config, events);
    let mut last_error = DeviceError("device not found".into());
    let mut open = |indices: &[usize]| -> Vec<HidDevice> {
        indices
            .iter()
            .filter_map(|i| match api.open_path(infos[*i].path()) {
                Ok(d) => Some(d),
                Err(e) => {
                    last_error = DeviceError(e.to_string());
                    None
                }
            })
            .collect()
    };
    let devices = open(&selection.config);
    let readers = open(&selection.events);
    if devices.is_empty() {
        return Err(last_error);
    }
    let product = infos.first().and_then(|d| d.product_string().map(str::to_owned));
    Ok(HidLink { transport: HidTransport { kind, devices, pinned: None, pid, output_reports, codec }, readers, product })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(usage_page: u16, usage: u16) -> IfaceUsage {
        IfaceUsage { usage_page, usage }
    }

    #[test]
    fn selects_by_usage_page() {
        let s = select_interfaces(&[u(0x01, 0x02), u(0x0B, 0), u(0x0A, 0)]);
        assert_eq!(s, Selection { config: vec![1], events: vec![2] });
    }

    #[test]
    fn falls_back_to_all_interfaces_when_usage_is_unknown() {
        let s = select_interfaces(&[u(0, 0), u(0, 0)]);
        assert_eq!(s, Selection { config: vec![0, 1], events: vec![0, 1] });
    }

    #[test]
    fn preferences_are_tried_in_order() {
        let ifaces = [u(0xFF00, 1), u(0xFF01, 2), u(0x0A, 0)];
        let s = select_for(&ifaces, &[(0xFF01, 2), (0xFF00, 1)], &[(0xFF00, 1), (0x0A, 0)]);
        assert_eq!(s, Selection { config: vec![1], events: vec![0] });
        let s = select_for(&ifaces, &[(0x0B, 0), (0xFF00, 1)], &[(0x0A, 0)]);
        assert_eq!(s, Selection { config: vec![0], events: vec![2] });
    }

    #[test]
    fn usage_must_be_zero() {
        let s = select_interfaces(&[u(0x0B, 1), u(0x0B, 0)]);
        assert_eq!(s.config, vec![1]);
    }
}
