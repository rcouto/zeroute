//! App-private settings for optional ZeroTier underlay endpoint hints.

use crate::settings::write_private;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

const FILE_NAME: &str = "peer-hints-v1";
pub(crate) const MAX_BYTES: usize = 64 * 1024;
const MAX_PEERS: usize = 128;
const MAX_ENDPOINTS: usize = 8;
const INVALID: &str = "Peer hints contain an invalid or duplicate peer or endpoint.";
const READ_FAILED: &str = "Saved peer hints could not be read.";
const SAVE_FAILED: &str = "Peer hints could not be saved.";

pub(crate) type PeerHints = BTreeMap<u64, Vec<SocketAddr>>;

fn is_unicast(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [first, ..] = ip.octets();
            first != 0 && first < 224 && !ip.is_loopback()
        }
        IpAddr::V6(ip) => {
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_multicast()
                && ip.to_ipv4().is_none()
        }
    }
}

// Public hints may be used on any underlay. Conservatively exclude special-use
// ranges rather than letting a scope label enable a private destination.
fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [first, second, third, _] = ip.octets();
            !ip.is_private()
                && !ip.is_link_local()
                && !ip.is_documentation()
                && !matches!(
                    (first, second, third),
                    (100, 64..=127, _) | (192, 0, 0) | (192, 88, 99) | (198, 18..=19, _)
                )
        }
        IpAddr::V6(ip) => {
            let [first, second, ..] = ip.segments();
            (0x2000..0x4000).contains(&first)
                && !matches!(
                    (first, second),
                    (0x2001, 0..=0x01ff | 0x0db8) | (0x2002, _) | (0x3fff, 0..=0x0fff)
                )
        }
    }
}

fn parse(bytes: &[u8]) -> Result<PeerHints, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err(INVALID);
    }
    let mut lines = std::str::from_utf8(bytes)
        .map_err(|_| INVALID)?
        .lines()
        .peekable();
    if lines.next() != Some("1") {
        return Err(INVALID);
    }
    let mut peers = HashSet::new();
    let mut hints = PeerHints::new();
    while let Some(node) = lines.next() {
        if node.len() != 10
            || !node
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(INVALID);
        }
        let node = u64::from_str_radix(node, 16).map_err(|_| INVALID)?;
        if !peers.insert(node) || peers.len() > MAX_PEERS {
            return Err(INVALID);
        }
        let mut seen = HashSet::new();
        let mut public = Vec::new();
        while lines.peek().is_some_and(|line| line.contains('\t')) {
            let mut fields = lines.next().ok_or(INVALID)?.split('\t');
            let scope = fields.next().ok_or(INVALID)?;
            let ip: IpAddr = fields.next().ok_or(INVALID)?.parse().map_err(|_| INVALID)?;
            let port: u16 = fields.next().ok_or(INVALID)?.parse().map_err(|_| INVALID)?;
            let address = SocketAddr::new(ip, port);
            if fields.next().is_some()
                || port == 0
                || !is_unicast(ip)
                || !seen.insert(address)
                || seen.len() > MAX_ENDPOINTS
            {
                return Err(INVALID);
            }
            match scope {
                "lan" => {}
                "public" if is_public(ip) => public.push(address),
                _ => return Err(INVALID),
            }
        }
        if seen.is_empty() {
            return Err(INVALID);
        }
        if !public.is_empty() {
            hints.insert(node, public);
        }
    }
    Ok(hints)
}

pub(crate) fn load(root: &Path) -> Result<Option<PeerHints>, &'static str> {
    let file = match File::open(root.join(FILE_NAME)) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(READ_FAILED),
    };
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| READ_FAILED)?;
    parse(&bytes).map(Some)
}

pub(crate) fn store(root: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    let hints = parse(bytes)?;
    let mut public_records = String::from("1\n");
    for (node, endpoints) in hints {
        writeln!(public_records, "{node:010x}").map_err(|_| SAVE_FAILED)?;
        for address in endpoints {
            writeln!(
                public_records,
                "public\t{}\t{}",
                address.ip(),
                address.port()
            )
            .map_err(|_| SAVE_FAILED)?;
        }
    }
    write_private(root, FILE_NAME, public_records.as_bytes()).map_err(|_| SAVE_FAILED)
}

pub(crate) fn remove(root: &Path) -> Result<(), &'static str> {
    match fs::remove_file(root.join(FILE_NAME)) {
        Ok(()) => File::open(root)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| SAVE_FAILED),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(SAVE_FAILED),
    }
}
