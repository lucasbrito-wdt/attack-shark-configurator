//! Background threads: discovery (hot-plug polling), event readers and the writer.

use super::hid::{self, HidLink};
use super::sender::{LinkState, Outcome, Sender, ThreadSleeper};
use super::{LinkKind, Transport};
use crate::platform::{self, ModelSpec, PlatformCodec};
use crate::protocol::events::{Event, log_event_bytes};
use hidapi::{HidApi, HidDevice};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender as ChannelSender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(1500);
const ARRIVAL_SETTLE: Duration = Duration::from_millis(1000);
const READ_TIMEOUT_MS: i32 = 20;
/// Battery poll period on request-only devices (XVI vendor: 2 s).
const BATTERY_POLL: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub enum DeviceMsg {
    LinkUp(LinkKind),
    LinkDown(LinkKind),
    /// Parsed device event and the link it arrived on.
    Event(LinkKind, Event),
    /// Model identified by PID, name hint or the first report on a shared receiver.
    Detected(&'static ModelSpec),
    /// Product / advertised name reported by the device (e.g. "X6 Mouse2").
    DeviceName(String),
    SendDone { sent: usize, skipped: usize, failed: usize },
    Error(String),
}

pub(crate) enum WriterCmd {
    SetTransport(LinkKind, Option<Box<dyn Transport>>),
    Send(Vec<Vec<u8>>),
}

pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// Everything a background thread needs to talk to the app.
#[derive(Clone)]
pub struct Shared {
    pub(crate) writer: ChannelSender<WriterCmd>,
    pub(crate) messages: ChannelSender<DeviceMsg>,
    pub link: LinkState,
    pub(crate) wake: Waker,
}

impl Shared {
    pub(crate) fn notify(&self, msg: DeviceMsg) {
        // a closed channel means the app is shutting down
        let _ = self.messages.send(msg);
        (self.wake)();
    }

    /// Route a parsed event to the link state (ack, heartbeat) and to the UI.
    pub(crate) fn dispatch(&self, link: LinkKind, event: Event) {
        match event {
            Event::Ack(_) => self.link.mark_ack(),
            Event::Heartbeat { .. } => self.link.mark_heartbeat(),
            _ => {}
        }
        self.notify(DeviceMsg::Event(link, event));
    }

    /// Queue packets for the writer thread (never blocks the caller).
    pub fn send_packets(&self, packets: Vec<Vec<u8>>) {
        let _ = self.writer.send(WriterCmd::Send(packets));
    }

    pub(crate) fn set_transport(&self, kind: LinkKind, transport: Option<Box<dyn Transport>>) {
        let _ = self.writer.send(WriterCmd::SetTransport(kind, transport));
    }
}

pub struct DeviceManager {
    pub shared: Shared,
    pub messages: Receiver<DeviceMsg>,
}

impl DeviceManager {
    pub fn start(wake: Waker) -> Self {
        let (writer_tx, writer_rx) = mpsc::channel();
        let (msg_tx, msg_rx) = mpsc::channel();
        let shared = Shared { writer: writer_tx, messages: msg_tx, link: LinkState::default(), wake };
        spawn_writer(shared.clone(), writer_rx);
        let hid_shared = shared.clone();
        thread::spawn(move || discovery_loop(&hid_shared));
        #[cfg(feature = "ble")]
        if std::env::var_os("X3_NO_BLE").is_none() {
            super::ble::spawn(shared.clone());
        }
        Self { shared, messages: msg_rx }
    }

    /// Queue packets for the writer thread (never blocks the UI).
    pub fn send(&self, packets: Vec<Vec<u8>>) {
        self.shared.send_packets(packets);
    }
}

fn spawn_writer(shared: Shared, commands: Receiver<WriterCmd>) {
    thread::spawn(move || {
        let mut sender = Sender::new(shared.link.clone(), Box::new(ThreadSleeper));
        let mut next_poll = Instant::now() + BATTERY_POLL;
        loop {
            // polls share the writer thread so they never interleave with a config write
            let cmd = match commands.recv_timeout(next_poll.saturating_duration_since(Instant::now())) {
                Ok(cmd) => cmd,
                Err(RecvTimeoutError::Timeout) => {
                    for (kind, ev) in sender.poll() {
                        shared.dispatch(kind, ev);
                    }
                    next_poll = Instant::now() + BATTERY_POLL;
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            };
            match cmd {
                WriterCmd::SetTransport(kind, t) => sender.set_transport(kind, t),
                WriterCmd::Send(packets) => {
                    let (mut sent, mut skipped, mut failed) = (0, 0, 0);
                    for p in &packets {
                        match sender.send(p) {
                            Outcome::Sent => sent += 1,
                            Outcome::Skipped => skipped += 1,
                            Outcome::Failed => failed += 1,
                        }
                    }
                    shared.notify(DeviceMsg::SendDone { sent, skipped, failed });
                }
            }
        }
    });
}

/// One open USB link: which device it is and how to stop its reader.
struct OpenLink {
    vid: u16,
    pid: u16,
    stop: Arc<AtomicBool>,
}

fn discovery_loop(shared: &Shared) {
    let mut api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => return shared.notify(DeviceMsg::Error(format!("HID init failed: {e}"))),
    };
    let watched = platform::usb_ids();
    let mut open: HashMap<LinkKind, OpenLink> = HashMap::new();
    loop {
        if let Err(e) = api.refresh_devices() {
            shared.notify(DeviceMsg::Error(format!("HID enumeration failed: {e}")));
        }
        if let Some(kind) = open.iter().find(|(_, l)| !hid::is_present(&api, l.vid, l.pid)).map(|(k, _)| *k) {
            close_link(shared, kind, &mut open);
        }
        for &(vid, pid, kind) in &watched {
            if !open.contains_key(&kind) && hid::is_present(&api, vid, pid) {
                thread::sleep(ARRIVAL_SETTLE);
                open_link(shared, &api, kind, (vid, pid), &mut open);
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn open_link(shared: &Shared, api: &HidApi, kind: LinkKind, (vid, pid): (u16, u16), open: &mut HashMap<LinkKind, OpenLink>) {
    let Some(codec) = platform::platform_of_vid(vid).and_then(platform::codec) else { return };
    match hid::open_link(api, kind, vid, pid, codec) {
        Ok(HidLink { transport, readers: handles, product }) => {
            let stop = Arc::new(AtomicBool::new(false));
            // a codec with an identity query (V4 shared PIDs) is never guessed from the PID
            let detected = match codec.identify_request(pid) {
                Some(_) => transport.identify(),
                None => platform::detect_usb(vid, pid, product.as_deref()),
            };
            spawn_reader(shared.clone(), kind, handles, stop.clone(), (pid, codec, detected.is_none()));
            open.insert(kind, OpenLink { vid, pid, stop });
            shared.set_transport(kind, Some(Box::new(transport)));
            if let Some(spec) = detected {
                shared.notify(DeviceMsg::Detected(spec));
            }
            shared.notify(DeviceMsg::LinkUp(kind));
            if let Some(name) = product.filter(|n| !n.is_empty()) {
                shared.notify(DeviceMsg::DeviceName(name));
            }
        }
        Err(e) => shared.notify(DeviceMsg::Error(format!("{} open failed: {e}", kind.label()))),
    }
}

fn close_link(shared: &Shared, kind: LinkKind, open: &mut HashMap<LinkKind, OpenLink>) {
    if let Some(link) = open.remove(&kind) {
        link.stop.store(true, Ordering::SeqCst);
    }
    shared.set_transport(kind, None);
    shared.notify(DeviceMsg::LinkDown(kind));
}

/// `(pid, codec, bind)`: with `bind`, the first input report picks the model (shared receiver rule).
fn spawn_reader(shared: Shared, kind: LinkKind, devices: Vec<HidDevice>, stop: Arc<AtomicBool>, link: (u16, &'static dyn PlatformCodec, bool)) {
    let (pid, codec, mut bind) = link;
    let log_raw = std::env::var_os("X3_LOG_RAW").is_some();
    thread::spawn(move || {
        if devices.is_empty() {
            return;
        }
        let mut buf = [0u8; 64];
        while !stop.load(Ordering::SeqCst) {
            for d in &devices {
                let Ok(n) = d.read_timeout(&mut buf, READ_TIMEOUT_MS) else { return };
                if n == 0 {
                    continue;
                }
                if log_raw {
                    eprintln!("raw input report: {:02X?}", &buf[..n]);
                }
                log_event_bytes("usb", &buf[..n]);
                if bind && let Some(spec) = codec.detect_from_report(pid, &buf[..n]) {
                    bind = false;
                    shared.notify(DeviceMsg::Detected(spec));
                }
                if let Some(ev) = codec.parse_report(&buf[..n]) {
                    shared.dispatch(kind, ev);
                }
            }
        }
    });
}
