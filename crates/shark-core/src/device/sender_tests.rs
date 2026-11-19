use super::sender::*;
use super::{DeviceError, LinkKind, Transport};
use crate::profile::{Macro, MacroEvent, Profile};
use crate::protocol::macros::build_macro;
use crate::protocol::packets::build_polling;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
enum Rec {
    Write(LinkKind, Vec<u8>),
    Sleep(u64),
}

type Log = Arc<Mutex<Vec<Rec>>>;

struct Mock {
    kind: LinkKind,
    log: Log,
    fail: bool,
    /// Mark the link acked after this many writes.
    ack_after: Option<(usize, LinkState)>,
    writes: usize,
}

impl Transport for Mock {
    fn kind(&self) -> LinkKind {
        self.kind
    }

    fn write(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        self.log.lock().unwrap().push(Rec::Write(self.kind, data.to_vec()));
        self.writes += 1;
        if let Some((n, link)) = &self.ack_after
            && self.writes >= *n
        {
            link.mark_ack();
        }
        if self.fail { Err(DeviceError("mock failure".into())) } else { Ok(()) }
    }
}

struct MockSleeper(Log);

impl Sleeper for MockSleeper {
    fn sleep(&mut self, d: Duration) {
        self.0.lock().unwrap().push(Rec::Sleep(d.as_millis() as u64));
    }
}

fn mock(kind: LinkKind, log: &Log) -> Mock {
    Mock { kind, log: log.clone(), fail: false, ack_after: None, writes: 0 }
}

fn setup() -> (Sender, LinkState, Log) {
    let log: Log = Arc::default();
    let link = LinkState::default();
    (Sender::new(link.clone(), Box::new(MockSleeper(log.clone()))), link, log)
}

fn macro_packet() -> Vec<u8> {
    let m = Macro { events: vec![MacroEvent { key: 4, down: true, delay_ms: 50 }], ..Macro::default() };
    build_macro(0, &m)
}

fn writes(log: &Log) -> Vec<(LinkKind, Vec<u8>)> {
    log.lock()
        .unwrap()
        .iter()
        .filter_map(|r| if let Rec::Write(k, d) = r { Some((*k, d.clone())) } else { None })
        .collect()
}

fn sleeps(log: &Log) -> Vec<u64> {
    log.lock().unwrap().iter().filter_map(|r| if let Rec::Sleep(ms) = r { Some(*ms) } else { None }).collect()
}

#[test]
fn wired_single_write_then_gap() {
    let (mut s, _, log) = setup();
    s.set_transport(LinkKind::Wired, Some(Box::new(mock(LinkKind::Wired, &log))));
    let pkt = build_polling(&Profile::default());
    assert_eq!(s.send(&pkt), Outcome::Sent);
    assert_eq!(*log.lock().unwrap(), vec![Rec::Write(LinkKind::Wired, pkt), Rec::Sleep(500)]);
}

#[test]
fn wired_macro_sends_three_chunks_with_200ms_gaps() {
    let (mut s, _, log) = setup();
    s.set_transport(LinkKind::Wired, Some(Box::new(mock(LinkKind::Wired, &log))));
    assert_eq!(s.send(&macro_packet()), Outcome::Sent);
    let w = writes(&log);
    assert_eq!(w.len(), 3);
    for (i, (_, c)) in w.iter().enumerate() {
        assert_eq!((c.len(), c[1], c[3]), (64, 0x40, i as u8));
    }
    assert_eq!(sleeps(&log), vec![200, 200, 200, 500]);
}

#[test]
fn wired_failure_falls_through_to_dongle() {
    let (mut s, link, log) = setup();
    let mut wired = mock(LinkKind::Wired, &log);
    wired.fail = true;
    let mut dongle = mock(LinkKind::Dongle, &log);
    dongle.ack_after = Some((1, link.clone()));
    s.set_transport(LinkKind::Dongle, Some(Box::new(dongle)));
    s.set_transport(LinkKind::Wired, Some(Box::new(wired)));
    link.mark_heartbeat();
    let pkt = build_polling(&Profile::default());
    assert_eq!(s.send(&pkt), Outcome::Sent);
    let kinds: Vec<LinkKind> = writes(&log).iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![LinkKind::Wired, LinkKind::Dongle]);
}

#[test]
fn dongle_stops_at_first_ack() {
    let (mut s, link, log) = setup();
    let mut dongle = mock(LinkKind::Dongle, &log);
    dongle.ack_after = Some((3, link.clone()));
    s.set_transport(LinkKind::Dongle, Some(Box::new(dongle)));
    link.mark_heartbeat();
    assert_eq!(s.send(&build_polling(&Profile::default())), Outcome::Sent);
    assert_eq!(writes(&log).len(), 3);
    assert_eq!(sleeps(&log), vec![500, 500, 500, 500]);
}

#[test]
fn dongle_caps_at_ten_attempts_without_ack() {
    let (mut s, link, log) = setup();
    s.set_transport(LinkKind::Dongle, Some(Box::new(mock(LinkKind::Dongle, &log))));
    link.mark_heartbeat();
    s.send(&build_polling(&Profile::default()));
    assert_eq!(writes(&log).len(), 10);
}

#[test]
fn dongle_macro_uses_1s_chunk_gaps_and_0c_on_last_chunk() {
    let (mut s, link, log) = setup();
    let mut dongle = mock(LinkKind::Dongle, &log);
    dongle.ack_after = Some((3, link.clone()));
    s.set_transport(LinkKind::Dongle, Some(Box::new(dongle)));
    link.mark_heartbeat();
    assert_eq!(s.send(&macro_packet()), Outcome::Sent);
    let w = writes(&log);
    assert_eq!(w.len(), 3);
    assert_eq!((w[0].1[1], w[1].1[1], w[2].1[1]), (0x40, 0x40, 0x0C));
    assert_eq!(sleeps(&log), vec![1000, 1000, 1000, 500]);
}

#[test]
fn dongle_is_gated_on_heartbeat() {
    let (mut s, _, log) = setup();
    s.set_transport(LinkKind::Dongle, Some(Box::new(mock(LinkKind::Dongle, &log))));
    assert_eq!(s.send(&build_polling(&Profile::default())), Outcome::Failed);
    assert!(writes(&log).is_empty());
}

#[test]
fn ble_writes_whole_packet_and_never_polling() {
    let (mut s, _, log) = setup();
    s.set_transport(LinkKind::Ble, Some(Box::new(mock(LinkKind::Ble, &log))));
    assert_eq!(s.send(&build_polling(&Profile::default())), Outcome::Skipped);
    assert!(writes(&log).is_empty());
    let pkt = macro_packet();
    assert_eq!(s.send(&pkt), Outcome::Sent);
    assert_eq!(writes(&log), vec![(LinkKind::Ble, pkt)]);
}

#[test]
fn priority_is_wired_then_dongle_then_ble() {
    let (mut s, link, log) = setup();
    for kind in [LinkKind::Ble, LinkKind::Dongle, LinkKind::Wired] {
        s.set_transport(kind, Some(Box::new(mock(kind, &log))));
    }
    link.mark_heartbeat();
    s.send(&build_polling(&Profile::default()));
    let kinds: Vec<LinkKind> = writes(&log).iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![LinkKind::Wired]);
    s.set_transport(LinkKind::Wired, None);
    log.lock().unwrap().clear();
    link.mark_ack();
    s.send(&build_polling(&Profile::default()));
    assert!(writes(&log).iter().all(|(k, _)| *k == LinkKind::Dongle));
}

#[test]
fn no_transport_fails() {
    let (mut s, _, _) = setup();
    assert_eq!(s.send(&[0x06, 9]), Outcome::Failed);
}

struct Polled(LinkKind);

impl Transport for Polled {
    fn kind(&self) -> LinkKind {
        self.0
    }

    fn write(&mut self, _: &[u8]) -> Result<(), DeviceError> {
        Ok(())
    }

    fn poll(&mut self) -> Option<crate::protocol::events::Event> {
        Some(crate::protocol::events::Event::Heartbeat { state: 3, level: 55 })
    }
}

#[test]
fn poll_reads_only_transports_that_answer() {
    let log: Log = Arc::default();
    let mut s = Sender::new(LinkState::default(), Box::new(MockSleeper(log.clone())));
    s.set_transport(LinkKind::Ble, Some(Box::new(Mock { kind: LinkKind::Ble, log, fail: false, ack_after: None, writes: 0 })));
    assert!(s.poll().is_empty(), "the default transport has no periodic read");
    s.set_transport(LinkKind::Wired, Some(Box::new(Polled(LinkKind::Wired))));
    assert_eq!(s.poll(), [(LinkKind::Wired, crate::protocol::events::Event::Heartbeat { state: 3, level: 55 })]);
}
