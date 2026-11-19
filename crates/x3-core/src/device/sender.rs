//! Send rules of spec 2.2-2.4: priority, retries, macro chunking and sleeps.

use super::{LinkKind, Transport};
use crate::protocol::events::Event;
use crate::protocol::macros::chunk_macro;
use crate::protocol::{CMD_MACRO, CMD_POLLING};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const WRITE_GAP: Duration = Duration::from_millis(500);
pub const WIRED_CHUNK_GAP: Duration = Duration::from_millis(200);
pub const DONGLE_RETRY_GAP: Duration = Duration::from_millis(500);
pub const DONGLE_CHUNK_GAP: Duration = Duration::from_millis(1000);
pub const DONGLE_MAX_ATTEMPTS: usize = 10;
pub const HEARTBEAT_WATCHDOG: Duration = Duration::from_secs(6);

pub trait Sleeper: Send {
    fn sleep(&mut self, d: Duration);
}

pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// State shared between the event reader and the sender.
#[derive(Clone, Default)]
pub struct LinkState {
    ack: Arc<AtomicBool>,
    last_heartbeat: Arc<Mutex<Option<Instant>>>,
}

impl LinkState {
    /// Event 0x5010. inferred: any 0x5010 counts as an ack (expected value unknown, spec unknown #11)
    pub fn mark_ack(&self) {
        self.ack.store(true, Ordering::SeqCst);
    }

    pub fn clear_ack(&self) {
        self.ack.store(false, Ordering::SeqCst);
    }

    pub fn acked(&self) -> bool {
        self.ack.load(Ordering::SeqCst)
    }

    /// Event 0x4010 re-arms the 6 s watchdog and marks the dongle link alive.
    pub fn mark_heartbeat(&self) {
        if let Ok(mut g) = self.last_heartbeat.lock() {
            *g = Some(Instant::now());
        }
    }

    pub fn dongle_alive(&self) -> bool {
        self.last_heartbeat.lock().ok().and_then(|g| *g).is_some_and(|t| t.elapsed() < HEARTBEAT_WATCHDOG)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Sent,
    /// Not sent by design (polling rate over BLE).
    Skipped,
    Failed,
}

pub struct Sender {
    transports: Vec<Box<dyn Transport>>,
    link: LinkState,
    sleeper: Box<dyn Sleeper>,
}

impl Sender {
    pub fn new(link: LinkState, sleeper: Box<dyn Sleeper>) -> Self {
        Self { transports: Vec::new(), link, sleeper }
    }

    /// Replace the transport of `kind` (or remove it with `None`), keeping priority order.
    pub fn set_transport(&mut self, kind: LinkKind, transport: Option<Box<dyn Transport>>) {
        self.transports.retain(|t| t.kind() != kind);
        self.transports.extend(transport);
        self.transports.sort_by_key(|t| t.kind());
    }

    /// Run every transport's periodic read (battery on request-only devices).
    pub fn poll(&mut self) -> Vec<(LinkKind, Event)> {
        self.transports.iter_mut().filter_map(|t| Some((t.kind(), t.poll()?))).collect()
    }

    fn slot(&mut self, kind: LinkKind) -> Option<&mut Box<dyn Transport>> {
        self.transports.iter_mut().find(|t| t.kind() == kind)
    }

    pub fn send(&mut self, pkt: &[u8]) -> Outcome {
        if pkt.is_empty() {
            return Outcome::Failed;
        }
        let is_macro = pkt[0] == CMD_MACRO;
        let outcome = if self.send_wired(pkt, is_macro) || self.send_dongle(pkt, is_macro) {
            Outcome::Sent
        } else {
            self.send_ble(pkt)
        };
        if outcome == Outcome::Sent {
            self.sleeper.sleep(WRITE_GAP);
        }
        outcome
    }

    fn send_wired(&mut self, pkt: &[u8], is_macro: bool) -> bool {
        let chunks = if is_macro { chunk_macro(pkt, false) } else { Vec::new() };
        let Some(t) = self.transports.iter_mut().find(|t| t.kind() == LinkKind::Wired) else { return false };
        if !is_macro {
            return t.write(pkt).is_ok();
        }
        for chunk in &chunks {
            if t.write(chunk).is_err() {
                return false;
            }
            self.sleeper.sleep(WIRED_CHUNK_GAP);
        }
        !chunks.is_empty()
    }

    fn send_dongle(&mut self, pkt: &[u8], is_macro: bool) -> bool {
        if !self.link.dongle_alive() {
            return false;
        }
        let chunks = if is_macro { chunk_macro(pkt, true) } else { Vec::new() };
        let link = self.link.clone();
        link.clear_ack();
        let mut any_ok = false;
        for _ in 0..DONGLE_MAX_ATTEMPTS {
            let Some(t) = self.transports.iter_mut().find(|t| t.kind() == LinkKind::Dongle) else { return false };
            if is_macro {
                for chunk in &chunks {
                    any_ok |= t.write(chunk).is_ok();
                    self.sleeper.sleep(DONGLE_CHUNK_GAP);
                }
            } else {
                any_ok |= t.write(pkt).is_ok();
                self.sleeper.sleep(DONGLE_RETRY_GAP);
            }
            if link.acked() {
                break;
            }
        }
        any_ok
    }

    fn send_ble(&mut self, pkt: &[u8]) -> Outcome {
        let Some(t) = self.slot(LinkKind::Ble) else { return Outcome::Failed };
        if pkt[0] == CMD_POLLING {
            return Outcome::Skipped;
        }
        if t.write(pkt).is_ok() { Outcome::Sent } else { Outcome::Failed }
    }
}
