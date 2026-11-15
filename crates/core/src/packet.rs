//! Bounded IP/Ethernet adaptation, ARP/NDP and an actual DNS health exchange.
use crate::network::Configuration;
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone)]
pub struct Frame {
    pub destination: u64,
    pub kind: u32,
    pub bytes: Vec<u8>,
}
struct Pending {
    hop: IpAddr,
    packet: Vec<u8>,
    expires: u64,
}
struct Neighbor {
    mac: u64,
    seen: u64,
}
struct Probe {
    server: IpAddr,
    source: IpAddr,
    id: u16,
    question: Vec<u8>,
}
#[derive(Default)]
struct Counters {
    frames_out: u64,
    frames_in: u64,
    invalid_ip: u64,
    source_drop: u64,
    route_drop: u64,
}
pub struct Bridge {
    counters: Counters,
    pub config: Configuration,
    pub frames: VecDeque<Frame>,
    pub incoming: VecDeque<Vec<u8>>,
    pub dns_ready: bool,
    neighbors: HashMap<IpAddr, Neighbor>,
    pending: VecDeque<Pending>,
    asked: HashMap<IpAddr, u64>,
    probe: Option<Probe>,
    next_probe: u64,
    last_dns: Option<u64>,
    nonce: u16,
}
fn mac_bytes(mac: u64) -> [u8; 6] {
    let b = mac.to_be_bytes();
    b[2..].try_into().unwrap()
}
fn mac_value(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0, |v, b| (v << 8) | u64::from(*b))
}
fn checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    for p in bytes.chunks(2) {
        sum += u32::from(p[0]) << 8 | u32::from(*p.get(1).unwrap_or(&0));
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
fn transport_checksum(source: IpAddr, destination: IpAddr, protocol: u8, bytes: &[u8]) -> u16 {
    let mut b = Vec::with_capacity(40 + bytes.len());
    match (source, destination) {
        (IpAddr::V4(s), IpAddr::V4(d)) => {
            b.extend(s.octets());
            b.extend(d.octets());
            b.extend([0, protocol]);
            b.extend((bytes.len() as u16).to_be_bytes());
        }
        (IpAddr::V6(s), IpAddr::V6(d)) => {
            b.extend(s.octets());
            b.extend(d.octets());
            b.extend((bytes.len() as u32).to_be_bytes());
            b.extend([0, 0, 0, protocol]);
        }
        _ => return 1,
    }
    b.extend(bytes);
    checksum(&b)
}
struct Ip<'a> {
    source: IpAddr,
    destination: IpAddr,
    protocol: u8,
    transport: &'a [u8],
    length: usize,
}
fn ip(packet: &[u8]) -> Option<Ip<'_>> {
    match packet.first()? >> 4 {
        4 if packet.len() >= 20 => {
            let header = usize::from(packet[0] & 15) * 4;
            let length = usize::from(u16::from_be_bytes([packet[2], packet[3]]));
            if header < 20
                || length < header
                || length > packet.len()
                || checksum(packet.get(..header)?) != 0
            {
                return None;
            }
            let fragmented = u16::from_be_bytes([packet[6], packet[7]]) & 0x3fff != 0;
            Some(Ip {
                source: IpAddr::V4(Ipv4Addr::new(
                    packet[12], packet[13], packet[14], packet[15],
                )),
                destination: IpAddr::V4(Ipv4Addr::new(
                    packet[16], packet[17], packet[18], packet[19],
                )),
                protocol: if fragmented { 0 } else { packet[9] },
                transport: &packet[header..length],
                length,
            })
        }
        6 if packet.len() >= 40 => {
            let length = 40 + usize::from(u16::from_be_bytes([packet[4], packet[5]]));
            if length > packet.len() {
                return None;
            }
            Some(Ip {
                source: IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(&packet[8..24]).ok()?)),
                destination: IpAddr::V6(Ipv6Addr::from(
                    <[u8; 16]>::try_from(&packet[24..40]).ok()?,
                )),
                protocol: packet[6],
                transport: &packet[40..length],
                length,
            })
        }
        _ => None,
    }
}
fn ipv6(source: Ipv6Addr, destination: Ipv6Addr, protocol: u8, hop: u8, payload: &[u8]) -> Vec<u8> {
    let mut p = vec![0u8; 40];
    p[0] = 0x60;
    p[4..6].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    p[6] = protocol;
    p[7] = hop;
    p[8..24].copy_from_slice(&source.octets());
    p[24..40].copy_from_slice(&destination.octets());
    p.extend(payload);
    p
}
fn solicited(ip: Ipv6Addr) -> Ipv6Addr {
    Ipv6Addr::from(0xff0200000000000000000001ff000000u128 | (u128::from(ip) & 0xffffff))
}
fn multicast_mac(ip: IpAddr) -> Option<u64> {
    match ip {
        IpAddr::V4(v) if v.is_multicast() => {
            Some(0x01005e000000 | (u64::from(u32::from(v)) & 0x7fffff))
        }
        IpAddr::V4(v) if v == Ipv4Addr::BROADCAST => Some(0xffffffffffff),
        IpAddr::V6(v) if v.is_multicast() => {
            Some(0x333300000000 | (u128::from(v) as u64 & 0xffffffff))
        }
        _ => None,
    }
}
impl Bridge {
    pub fn new(config: Configuration, nonce: u16) -> Self {
        Self {
            counters: Counters::default(),
            config,
            frames: VecDeque::new(),
            incoming: VecDeque::new(),
            dns_ready: false,
            neighbors: HashMap::new(),
            pending: VecDeque::new(),
            asked: HashMap::new(),
            probe: None,
            next_probe: 0,
            last_dns: None,
            nonce,
        }
    }
    fn frame(&mut self, destination: u64, kind: u32, bytes: Vec<u8>) {
        if self.frames.len() < 128 {
            self.counters.frames_out += 1;
            self.frames.push_back(Frame {
                destination,
                kind,
                bytes,
            });
        }
    }
    pub fn invalidate_dns(&mut self) {
        self.dns_ready = false;
        self.last_dns = None;
        self.probe = None;
        self.next_probe = 0;
    }
    pub fn diagnostics(&self) -> String {
        format!(
            "Overlay TX {} RX {}; drops invalid={} source={} route={}; neighbor queue={}",
            self.counters.frames_out,
            self.counters.frames_in,
            self.counters.invalid_ip,
            self.counters.source_drop,
            self.counters.route_drop,
            self.pending.len()
        )
    }
    pub fn outgoing(&mut self, packet: &[u8], now: u64) {
        let Some(info) = ip(packet) else {
            self.counters.invalid_ip += 1;
            return;
        };
        if info.length > self.config.mtu {
            self.counters.invalid_ip += 1;
            return;
        }
        if !self
            .config
            .addresses
            .iter()
            .any(|p| p.address == info.source)
        {
            self.counters.source_drop += 1;
            return;
        }
        let packet = &packet[..info.length];
        let kind = if info.destination.is_ipv4() {
            0x0800
        } else {
            0x86dd
        };
        if let Some(mac) = multicast_mac(info.destination) {
            self.frame(mac, kind, packet.to_vec());
            return;
        }
        let Some(hop) = self.config.next_hop(info.destination) else {
            self.counters.route_drop += 1;
            return;
        };
        if let Some(n) = self
            .neighbors
            .get(&hop)
            .filter(|n| now.saturating_sub(n.seen) < 60000)
        {
            self.frame(n.mac, kind, packet.to_vec());
            return;
        }
        if self.pending.len() < 64 {
            self.pending.push_back(Pending {
                hop,
                packet: packet.to_vec(),
                expires: now.saturating_add(5000),
            });
            self.resolve(hop, now);
        }
    }
    fn resolve(&mut self, hop: IpAddr, now: u64) {
        if self
            .asked
            .get(&hop)
            .is_some_and(|t| now.saturating_sub(*t) < 1000)
        {
            return;
        }
        if self.asked.len() >= 256 {
            self.asked.clear();
        }
        self.asked.insert(hop, now);
        match (self.config.source_for(hop), hop) {
            (Some(IpAddr::V4(source)), IpAddr::V4(target)) => {
                let mut b = vec![0, 1, 8, 0, 6, 4, 0, 1];
                b.extend(mac_bytes(self.config.mac));
                b.extend(source.octets());
                b.extend([0; 6]);
                b.extend(target.octets());
                self.frame(0xffffffffffff, 0x0806, b);
            }
            (Some(IpAddr::V6(source)), IpAddr::V6(target)) => {
                let destination = solicited(target);
                let mut b = vec![135, 0, 0, 0, 0, 0, 0, 0];
                b.extend(target.octets());
                b.extend([1, 1]);
                b.extend(mac_bytes(self.config.mac));
                let sum = transport_checksum(source.into(), destination.into(), 58, &b);
                b[2..4].copy_from_slice(&sum.to_be_bytes());
                self.frame(
                    multicast_mac(destination.into()).unwrap(),
                    0x86dd,
                    ipv6(source, destination, 58, 255, &b),
                );
            }
            _ => (),
        }
    }
    fn learn(&mut self, address: IpAddr, mac: u64, now: u64) {
        if mac == 0
            || mac >> 48 != 0
            || (mac >> 40) & 1 != 0
            || address.is_unspecified()
            || address.is_multicast()
        {
            return;
        }
        if self.neighbors.len() >= 256
            && let Some(oldest) = self
                .neighbors
                .iter()
                .min_by_key(|(_, n)| n.seen)
                .map(|(ip, _)| *ip)
        {
            self.neighbors.remove(&oldest);
        }
        self.neighbors.insert(address, Neighbor { mac, seen: now });
        self.asked.remove(&address);
        let mut pending = std::mem::take(&mut self.pending);
        while let Some(p) = pending.pop_front() {
            if p.expires <= now {
                continue;
            }
            if p.hop == address {
                self.outgoing(&p.packet, now);
            } else {
                self.pending.push_back(p);
            }
        }
    }
    pub fn receive(&mut self, source: u64, destination: u64, kind: u32, packet: &[u8], now: u64) {
        self.counters.frames_in += 1;
        if source == self.config.mac
            || source >> 48 != 0
            || (source >> 40) & 1 != 0
            || packet.len() > self.config.mtu.max(1280)
            || (destination != self.config.mac && (destination >> 40) & 1 == 0)
        {
            return;
        }
        if kind == 0x0806 {
            self.arp(source, packet, now);
            return;
        }
        if kind != 0x0800 && kind != 0x86dd {
            return;
        }
        let Some(info) = ip(packet) else { return };
        if (kind == 0x0800) != info.destination.is_ipv4() {
            return;
        }
        if kind == 0x86dd
            && info.protocol == 58
            && matches!(info.transport.first(), Some(135 | 136))
        {
            self.ndp(source, packet, now);
            return;
        }
        if !self
            .config
            .addresses
            .iter()
            .any(|p| p.address == info.destination)
            && !info.destination.is_multicast()
        {
            return;
        }
        if self.dns_response(&info, now) {
            return;
        }
        if self.incoming.len() < 64 {
            self.incoming.push_back(packet[..info.length].to_vec());
        }
    }
    fn arp(&mut self, source: u64, b: &[u8], now: u64) {
        if b.len() < 28 || b[..6] != [0, 1, 8, 0, 6, 4] || mac_value(&b[8..14]) != source {
            return;
        }
        let operation = u16::from_be_bytes([b[6], b[7]]);
        let sender = Ipv4Addr::new(b[14], b[15], b[16], b[17]);
        let target = Ipv4Addr::new(b[24], b[25], b[26], b[27]);
        if !matches!(operation, 1 | 2)
            || !self
                .config
                .addresses
                .iter()
                .any(|p| p.address == IpAddr::V4(target))
        {
            return;
        }
        self.learn(sender.into(), source, now);
        if operation == 1 {
            let mut reply = vec![0, 1, 8, 0, 6, 4, 0, 2];
            reply.extend(mac_bytes(self.config.mac));
            reply.extend(target.octets());
            reply.extend(mac_bytes(source));
            reply.extend(sender.octets());
            self.frame(source, 0x0806, reply);
        }
    }
    fn ndp(&mut self, source_mac: u64, packet: &[u8], now: u64) {
        let Some(info) = ip(packet) else { return };
        let b = info.transport;
        if packet[7] != 255
            || b.len() < 24
            || b[1] != 0
            || transport_checksum(info.source, info.destination, 58, b) != 0
        {
            return;
        }
        let target = Ipv6Addr::from(<[u8; 16]>::try_from(&b[8..24]).unwrap());
        if target.is_multicast() || target.is_unspecified() {
            return;
        }
        let mut option = 24;
        let mut link = None;
        while option < b.len() {
            if option + 2 > b.len() || b[option + 1] == 0 {
                return;
            }
            let size = usize::from(b[option + 1]) * 8;
            if option + size > b.len() {
                return;
            }
            if b[option] == if b[0] == 135 { 1 } else { 2 } {
                if size != 8 {
                    return;
                }
                link = Some(mac_value(&b[option + 2..option + 8]));
            }
            option += size;
        }
        if link.is_some_and(|mac| mac != source_mac) {
            return;
        }
        if b[0] == 136 {
            if !info.source.is_unspecified()
                && self.pending.iter().any(|p| p.hop == IpAddr::V6(target))
            {
                self.learn(target.into(), source_mac, now);
            }
            return;
        }
        if !self
            .config
            .addresses
            .iter()
            .any(|p| p.address == IpAddr::V6(target))
        {
            return;
        }
        let IpAddr::V6(sender) = info.source else {
            return;
        };
        if !sender.is_unspecified() {
            self.learn(sender.into(), source_mac, now);
        }
        let destination = if sender.is_unspecified() {
            "ff02::1".parse().unwrap()
        } else {
            sender
        };
        let mut reply = vec![
            136,
            0,
            0,
            0,
            if sender.is_unspecified() { 0x20 } else { 0x60 },
            0,
            0,
            0,
        ];
        reply.extend(target.octets());
        reply.extend([2, 1]);
        reply.extend(mac_bytes(self.config.mac));
        let sum = transport_checksum(target.into(), destination.into(), 58, &reply);
        reply[2..4].copy_from_slice(&sum.to_be_bytes());
        self.frame(
            if sender.is_unspecified() {
                multicast_mac(destination.into()).unwrap()
            } else {
                source_mac
            },
            0x86dd,
            ipv6(target, destination, 58, 255, &reply),
        );
    }
    pub fn tick(&mut self, now: u64) {
        self.pending.retain(|p| p.expires > now);
        let hops: Vec<_> = self.pending.iter().map(|p| p.hop).collect();
        for hop in hops {
            self.resolve(hop, now);
        }
        if self.last_dns.is_some_and(|t| now.saturating_sub(t) > 30000) {
            self.dns_ready = false;
        }
        if now < self.next_probe {
            return;
        }
        self.next_probe = now.saturating_add(if self.dns_ready { 15000 } else { 2000 });
        let Some(server) = self.config.dns.first().copied() else {
            return;
        };
        let Some(source) = self.config.source_for(server) else {
            return;
        };
        self.nonce = self.nonce.wrapping_add(1);
        let mut question = Vec::new();
        for label in self.config.domain.split('.').filter(|s| !s.is_empty()) {
            question.push(label.len() as u8);
            question.extend(label.as_bytes());
        }
        question.extend([0, 0, 6, 0, 1]);
        let mut dns = vec![0; 12];
        dns[..2].copy_from_slice(&self.nonce.to_be_bytes());
        dns[2] = 1;
        dns[5] = 1;
        dns.extend(&question);
        let mut udp = vec![0; 8];
        udp[..2].copy_from_slice(&55345u16.to_be_bytes());
        udp[2..4].copy_from_slice(&53u16.to_be_bytes());
        udp[4..6].copy_from_slice(&((8 + dns.len()) as u16).to_be_bytes());
        udp.extend(dns);
        let sum = transport_checksum(source, server, 17, &udp);
        udp[6..8].copy_from_slice(&if sum == 0 { u16::MAX } else { sum }.to_be_bytes());
        let packet = match (source, server) {
            (IpAddr::V4(s), IpAddr::V4(d)) => {
                let mut p = vec![0; 20];
                p[0] = 0x45;
                p[2..4].copy_from_slice(&((20 + udp.len()) as u16).to_be_bytes());
                p[8] = 64;
                p[9] = 17;
                p[12..16].copy_from_slice(&s.octets());
                p[16..20].copy_from_slice(&d.octets());
                let sum = checksum(&p);
                p[10..12].copy_from_slice(&sum.to_be_bytes());
                p.extend(udp);
                p
            }
            (IpAddr::V6(s), IpAddr::V6(d)) => ipv6(s, d, 17, 64, &udp),
            _ => return,
        };
        self.probe = Some(Probe {
            server,
            source,
            id: self.nonce,
            question,
        });
        self.outgoing(&packet, now);
    }
    fn dns_response(&mut self, info: &Ip<'_>, now: u64) -> bool {
        let Some(p) = &self.probe else { return false };
        let b = info.transport;
        if info.protocol != 17
            || info.source != p.server
            || info.destination != p.source
            || b.len() < 20
            || b[..2] != 53u16.to_be_bytes()
            || b[2..4] != 55345u16.to_be_bytes()
        {
            return false;
        }
        let length = usize::from(u16::from_be_bytes([b[4], b[5]]));
        if length < 20 || length > b.len() {
            return false;
        }
        let b = &b[..length];
        if (info.source.is_ipv6() || b[6..8] != [0, 0])
            && transport_checksum(info.source, info.destination, 17, b) != 0
        {
            return false;
        }
        let d = &b[8..];
        if d[..2] != p.id.to_be_bytes()
            || d[2] & 0xfa != 0x80
            || !matches!(d[3] & 15, 0 | 3)
            || d[4..6] != [0, 1]
            || d.get(12..12 + p.question.len()) != Some(p.question.as_slice())
        {
            return false;
        }
        self.dns_ready = true;
        self.last_dns = Some(now);
        self.probe = None;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        Configuration::parse(
            0x020000000001,
            1280,
            "a 192.0.2.2/24\na 2001:db8::2/64\nd 192.0.2.53\ns example.test",
        )
        .unwrap()
    }
    fn v4(destination: Ipv4Addr) -> Vec<u8> {
        let mut p = vec![0; 20];
        p[0] = 0x45;
        p[2..4].copy_from_slice(&20u16.to_be_bytes());
        p[8] = 64;
        p[12..16].copy_from_slice(&[192, 0, 2, 2]);
        p[16..20].copy_from_slice(&destination.octets());
        let c = checksum(&p);
        p[10..12].copy_from_slice(&c.to_be_bytes());
        p
    }
    #[test]
    fn arp_resolution_flushes_pending_packet() {
        let mut b = Bridge::new(config(), 1);
        let packet = v4(Ipv4Addr::new(192, 0, 2, 9));
        b.outgoing(&packet, 0);
        assert_eq!(b.frames.pop_front().unwrap().kind, 0x806);
        assert_eq!(b.pending.len(), 1);
        let mut reply = vec![0, 1, 8, 0, 6, 4, 0, 2];
        reply.extend(mac_bytes(0x020000000009));
        reply.extend([192, 0, 2, 9]);
        reply.extend(mac_bytes(b.config.mac));
        reply.extend([192, 0, 2, 2]);
        b.receive(0x020000000009, b.config.mac, 0x806, &reply, 1);
        assert!(b.pending.is_empty());
        assert_eq!(b.frames.pop_front().unwrap().bytes, packet);
    }
    #[test]
    fn ndp_resolution_uses_solicited_multicast_and_checks_checksum() {
        let mut b = Bridge::new(config(), 1);
        let p = ipv6(
            "2001:db8::2".parse().unwrap(),
            "2001:db8::9".parse().unwrap(),
            59,
            64,
            &[],
        );
        b.outgoing(&p, 0);
        let f = b.frames.pop_front().unwrap();
        assert_eq!(f.destination, 0x3333ff000009);
        let info = ip(&f.bytes).unwrap();
        assert_eq!(
            transport_checksum(info.source, info.destination, 58, info.transport),
            0
        );
        assert_eq!(info.transport[0], 135);
    }
    #[test]
    fn queues_are_bounded_and_expire() {
        let mut b = Bridge::new(config(), 1);
        for _ in 0..1000 {
            b.outgoing(&v4(Ipv4Addr::new(192, 0, 2, 9)), 0);
        }
        assert_eq!(b.pending.len(), 64);
        assert_eq!(b.frames.len(), 1);
        b.tick(5001);
        assert!(b.pending.len() <= 1);
    }
    #[test]
    fn malformed_ip_never_enters_queue() {
        let mut b = Bridge::new(config(), 1);
        for n in 0..80 {
            b.outgoing(&vec![0xff; n], 0);
        }
        assert!(b.pending.is_empty());
        assert!(b.frames.is_empty());
    }
    #[test]
    fn dns_is_not_ready_without_a_valid_exchange() {
        let mut b = Bridge::new(config(), 1);
        b.tick(0);
        assert!(!b.dns_ready);
        assert!(b.probe.is_some());
        assert!(
            b.pending
                .iter()
                .any(|p| p.hop == "192.0.2.53".parse::<IpAddr>().unwrap())
        );
    }
    #[test]
    fn ipv6_neighbor_exchange_delivers_the_original_packet() {
        let mut a = Bridge::new(
            Configuration::parse(0x020000000001, 1280, "a 2001:db8::1/64").unwrap(),
            1,
        );
        let mut b = Bridge::new(
            Configuration::parse(0x020000000002, 1280, "a 2001:db8::2/64").unwrap(),
            2,
        );
        let packet = ipv6(
            "2001:db8::1".parse().unwrap(),
            "2001:db8::2".parse().unwrap(),
            59,
            64,
            &[],
        );
        a.outgoing(&packet, 0);
        let ns = a.frames.pop_front().unwrap();
        b.receive(a.config.mac, ns.destination, ns.kind, &ns.bytes, 1);
        let na = b.frames.pop_front().unwrap();
        a.receive(b.config.mac, na.destination, na.kind, &na.bytes, 2);
        let data = a.frames.pop_front().unwrap();
        b.receive(a.config.mac, data.destination, data.kind, &data.bytes, 3);
        assert_eq!(b.incoming.pop_front().unwrap(), packet);
    }
    #[test]
    fn dns_requires_matching_reply_and_expires_without_fresh_answers() {
        let mut b = Bridge::new(config(), 17);
        b.learn("192.0.2.53".parse().unwrap(), 0x020000000053, 0);
        b.tick(0);
        let query = b.frames.pop_front().unwrap();
        let mut reply = query.bytes;
        let src: [u8; 4] = reply[12..16].try_into().unwrap();
        let dst: [u8; 4] = reply[16..20].try_into().unwrap();
        reply[12..16].copy_from_slice(&dst);
        reply[16..20].copy_from_slice(&src);
        reply[20..22].copy_from_slice(&53u16.to_be_bytes());
        reply[22..24].copy_from_slice(&55345u16.to_be_bytes());
        reply[30] = 0x81;
        reply[31] = 0x80;
        reply[26..28].fill(0);
        let sum = transport_checksum(
            Ipv4Addr::from(dst).into(),
            Ipv4Addr::from(src).into(),
            17,
            &reply[20..],
        );
        reply[26..28].copy_from_slice(&sum.to_be_bytes());
        reply[10..12].fill(0);
        let sum = checksum(&reply[..20]);
        reply[10..12].copy_from_slice(&sum.to_be_bytes());
        let mut invalid = reply.clone();
        invalid[29] ^= 1;
        b.receive(0x020000000053, b.config.mac, 0x0800, &invalid, 1);
        assert!(!b.dns_ready);
        b.receive(0x020000000053, b.config.mac, 0x0800, &reply, 2);
        assert!(b.dns_ready);
        assert!(b.probe.is_none());
        b.invalidate_dns();
        assert!(!b.dns_ready);
        b.receive(0x020000000053, b.config.mac, 0x0800, &reply, 3);
        assert!(
            !b.dns_ready,
            "a response from the old network cannot restore readiness"
        );
        b.tick(4);
        assert!(b.probe.is_some());
        b.receive(0x020000000053, b.config.mac, 0x0800, &reply, 5);
        assert!(
            !b.dns_ready,
            "the old transaction cannot satisfy the new probe"
        );
        b.tick(31003);
        assert!(!b.dns_ready);
        b.receive(0x020000000053, b.config.mac, 0x0800, &reply, 31004);
        assert!(!b.dns_ready);
    }
}
