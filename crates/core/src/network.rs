//! Validated controller configuration and the private-network routing policy.
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prefix {
    pub address: IpAddr,
    pub bits: u8,
}
impl Prefix {
    pub fn parse(input: &str) -> Option<Self> {
        let (address, bits) = input.split_once('/')?;
        let address: IpAddr = address.parse().ok()?;
        let bits = bits.parse().ok()?;
        (bits <= if address.is_ipv4() { 32 } else { 128 }).then_some(Self { address, bits })
    }
    pub fn network(self) -> IpAddr {
        match self.address {
            IpAddr::V4(ip) => IpAddr::V4(Ipv4Addr::from(
                u32::from(ip) & u32::MAX.checked_shl(32 - u32::from(self.bits)).unwrap_or(0),
            )),
            IpAddr::V6(ip) => IpAddr::V6(Ipv6Addr::from(
                u128::from(ip)
                    & u128::MAX
                        .checked_shl(128 - u32::from(self.bits))
                        .unwrap_or(0),
            )),
        }
    }
    pub fn contains(self, address: IpAddr) -> bool {
        address.is_ipv4() == self.address.is_ipv4()
            && Self {
                address,
                bits: self.bits,
            }
            .network()
                == self.network()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub target: Prefix,
    pub via: Option<IpAddr>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    pub mac: u64,
    pub mtu: usize,
    pub addresses: Vec<Prefix>,
    pub routes: Vec<Route>,
    pub dns: Vec<IpAddr>,
    pub dns_requested: bool,
    pub domain: String,
}
fn unicast(ip: IpAddr) -> bool {
    !ip.is_unspecified() && !ip.is_multicast() && !ip.is_loopback()
}
impl Configuration {
    pub fn parse(mac: u64, mtu: u32, text: &str) -> Result<Self, &'static str> {
        if mac == 0 || mac >> 48 != 0 || (mac >> 40) & 1 != 0 || text.len() > 32768 {
            return Err("Invalid controller configuration.");
        }
        let mut result = Self {
            mac,
            mtu: (mtu as usize).clamp(1280, 2800),
            addresses: Vec::new(),
            routes: Vec::new(),
            dns: Vec::new(),
            dns_requested: false,
            domain: String::new(),
        };
        for line in text.lines() {
            let words: Vec<_> = line.split_ascii_whitespace().collect();
            match words.as_slice() {
                ["a", value] => {
                    let p = Prefix::parse(value).ok_or("Invalid assigned address.")?;
                    if !unicast(p.address) || p.bits == 0 || result.addresses.len() >= 32 {
                        return Err("Invalid assigned address.");
                    }
                    if !result.addresses.contains(&p) {
                        result.addresses.push(p);
                    }
                }
                ["r", value, via] => {
                    let mut p = Prefix::parse(value).ok_or("Invalid managed route.")?;
                    if p.bits == 0 {
                        continue;
                    } // Exit routing requires a future explicit opt-in.
                    p.address = p.network();
                    let via = if *via == "-" {
                        None
                    } else {
                        Some(
                            via.parse::<IpAddr>()
                                .map_err(|_| "Invalid route gateway.")?,
                        )
                    };
                    if result.routes.len() >= 128
                        || via.is_some_and(|ip| !unicast(ip) || ip.is_ipv4() != p.address.is_ipv4())
                    {
                        return Err("Invalid managed route.");
                    }
                    result.routes.push(Route { target: p, via });
                }
                ["d", value] => {
                    result.dns_requested = true;
                    let ip: IpAddr = value.parse().map_err(|_| "Invalid managed DNS server.")?;
                    if !unicast(ip) || result.dns.len() >= 8 {
                        return Err("Invalid managed DNS server.");
                    }
                    if !result.dns.contains(&ip) {
                        result.dns.push(ip);
                    }
                }
                ["s", value]
                    if value.len() <= 253
                        && value.split('.').all(|label| {
                            !label.is_empty()
                                && label.len() <= 63
                                && label
                                    .bytes()
                                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                        }) =>
                {
                    result.domain = value.to_string()
                }
                _ => return Err("Invalid controller configuration."),
            }
        }
        if result.addresses.is_empty() {
            return Err("The controller has not assigned an IP address.");
        }
        for p in &result.addresses {
            let mut target = *p;
            target.address = target.network();
            if !result
                .routes
                .iter()
                .any(|r| r.target == target && r.via.is_none())
            {
                result.routes.push(Route { target, via: None });
            }
        }
        result.routes.retain(|r| {
            result
                .addresses
                .iter()
                .any(|p| p.address.is_ipv4() == r.target.address.is_ipv4())
        });
        let dns = result.dns.clone();
        result.dns = dns
            .into_iter()
            .filter(|ip| result.next_hop(*ip).is_some())
            .collect();
        Ok(result)
    }
    pub fn source_for(&self, destination: IpAddr) -> Option<IpAddr> {
        self.addresses
            .iter()
            .find(|p| p.contains(destination))
            .or_else(|| {
                self.addresses
                    .iter()
                    .find(|p| p.address.is_ipv4() == destination.is_ipv4())
            })
            .map(|p| p.address)
    }
    pub fn next_hop(&self, destination: IpAddr) -> Option<IpAddr> {
        self.routes
            .iter()
            .filter(|r| r.target.contains(destination))
            .max_by_key(|r| r.target.bits)
            .map(|r| r.via.unwrap_or(destination))
    }
    pub fn android_text(&self, revision: u64) -> String {
        let mut s = format!("revision {revision}\nmtu {}\n", self.mtu);
        for p in &self.addresses {
            s += &format!("a {}/{}\n", p.address, p.bits);
        }
        for r in &self.routes {
            s += &format!("r {}/{}\n", r.target.address, r.target.bits);
        }
        // Android uses a route to this single address for AI_ADDRCONFIG, even
        // when only private IPv6 is required. Export the host route to Android
        // without adding an Internet exit to the packet adapter's route policy.
        // See docs/architecture.md for the measured failure and AOSP reference.
        let ipv6_probe = IpAddr::V6(Ipv6Addr::new(0x2000, 0, 0, 0, 0, 0, 0, 0));
        if self.addresses.iter().any(|p| p.address.is_ipv6()) && self.next_hop(ipv6_probe).is_none()
        {
            s += &format!("r {ipv6_probe}/128\n");
        }
        for ip in &self.dns {
            s += &format!("d {ip}\n");
        }
        if !self.domain.is_empty() {
            s += &format!("s {}\n", self.domain);
        }
        s
    }
    pub fn subscriptions(&self) -> Vec<(u64, u32)> {
        let mut s = vec![(0xffffffffffff, 0), (0x333300000001, 0)];
        for p in &self.addresses {
            match p.address {
                IpAddr::V4(ip) => s.push((0xffffffffffff, u32::from(ip))),
                IpAddr::V6(ip) => s.push((0x3333ff000000 | (u128::from(ip) as u64 & 0xffffff), 0)),
            }
        }
        s.sort_unstable();
        s.dedup();
        s
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_dns_differs_from_configured_but_unroutable_dns() {
        let absent = Configuration::parse(0x020000000001, 1280, "a 192.0.2.1/24").unwrap();
        assert!(!absent.dns_requested);
        let unreachable =
            Configuration::parse(0x020000000001, 1280, "a 192.0.2.1/24\nd 198.51.100.53").unwrap();
        assert!(unreachable.dns_requested);
        assert!(unreachable.dns.is_empty());
    }

    #[test]
    fn longest_route_gateway_and_exit_opt_in() {
        let c=Configuration::parse(0x020000000001,2800,"a 192.0.2.2/24\nr 198.51.100.0/24 192.0.2.1\nr 0.0.0.0/0 192.0.2.1\nd 192.0.2.53\ns example.test\n").unwrap();
        assert_eq!(
            c.next_hop("198.51.100.9".parse().unwrap()),
            Some("192.0.2.1".parse().unwrap())
        );
        assert_eq!(c.next_hop("203.0.113.1".parse().unwrap()), None);
        assert!(!c.android_text(1).contains("0.0.0.0/0"));
    }
    #[test]
    fn malformed_controller_input_fails_closed() {
        for text in [
            "a 127.0.0.1/8",
            "a 192.0.2.2/99",
            "a 192.0.2.2/24\ns bad/domain",
            "a 192.0.2.2/24\nr 198.51.100.0/24 ::1",
        ] {
            assert!(Configuration::parse(0x020000000001, 1280, text).is_err());
        }
    }
    #[test]
    fn ipv6_prefix_and_arp_subscriptions() {
        let c =
            Configuration::parse(0x020000000001, 1280, "a 2001:db8::2/64\na 192.0.2.2/24").unwrap();
        assert_eq!(
            c.next_hop("2001:db8::1234".parse().unwrap()),
            Some("2001:db8::1234".parse().unwrap())
        );
        assert!(
            c.subscriptions()
                .contains(&(0xffffffffffff, u32::from(Ipv4Addr::new(192, 0, 2, 2))))
        );
        assert!(c.subscriptions().contains(&(0x3333ff000002, 0)));
    }
    #[test]
    fn android_ipv6_probe_route_does_not_enable_an_exit() {
        let config = Configuration::parse(
            0x020000000001,
            1280,
            "a 2001:db8::2/64\nr ::/0 2001:db8::1\nd 2001:db8::53",
        )
        .unwrap();
        let android = config.android_text(1);
        assert!(android.contains("r 2000::/128\n"));
        assert!(!android.contains("r ::/0\n"));
        assert_eq!(config.next_hop("2000::".parse().unwrap()), None);
        assert_eq!(
            config.next_hop("2606:4700:4700::1111".parse().unwrap()),
            None
        );
        assert_eq!(
            config.next_hop("2001:db8::53".parse().unwrap()),
            Some("2001:db8::53".parse().unwrap())
        );
        let ipv4 = Configuration::parse(0x020000000001, 1280, "a 192.0.2.2/24").unwrap();
        assert!(!ipv4.android_text(1).contains("2000::"));
    }
    #[test]
    fn controller_route_to_android_probe_is_not_duplicated() {
        let config = Configuration::parse(
            0x020000000001,
            1280,
            "a 2001:db8::2/64\nr 2000::/128 2001:db8::1",
        )
        .unwrap();
        assert_eq!(config.android_text(1).matches("r 2000::/128\n").count(), 1);
        assert_eq!(
            config.next_hop("2000::".parse().unwrap()),
            Some("2001:db8::1".parse().unwrap())
        );
    }
}
