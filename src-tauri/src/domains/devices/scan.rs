//! Find Matrix devices on the workstation's own IPv4 subnets.
//!
//! The scan stays on private addresses of local interfaces, and never wider
//! than a /24. Identification is the documented `device-basic-config` read.
//! No password is sent during the scan.

use std::collections::BTreeSet;
use std::net::Ipv4Addr;

use serde::Serialize;

use crate::matrix::{FoundMatrixDevice, MatrixAdapter};

const MAX_HOSTS: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDevice {
    pub host: String,
    pub port: u16,
    pub name: Option<String>,
}

pub async fn scan_local_matrix_devices(matrix: &MatrixAdapter) -> Vec<DiscoveredDevice> {
    let hosts = local_scan_targets();
    tracing::info!(candidates = hosts.len(), "network_scan_started");
    let found = matrix.find_matrix_devices(&hosts).await;
    tracing::info!(found = found.len(), "network_scan_finished");
    found.into_iter().map(to_discovered).collect()
}

fn to_discovered(device: FoundMatrixDevice) -> DiscoveredDevice {
    DiscoveredDevice {
        host: device.host,
        port: device.port,
        name: device.name,
    }
}

pub fn local_scan_targets() -> Vec<Ipv4Addr> {
    let mut targets = BTreeSet::new();
    for (address, mask) in local_ipv4_interfaces() {
        for host in hosts_to_probe(address, mask) {
            if targets.len() >= MAX_HOSTS {
                return targets.into_iter().collect();
            }
            targets.insert(host);
        }
    }
    targets.into_iter().collect()
}

pub fn is_private_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168)
}

/// Hosts on this interface, excluding the workstation itself.
///
/// A mask wider than /24 is narrowed to the interface's own /24.
pub fn hosts_to_probe(address: Ipv4Addr, netmask: Ipv4Addr) -> Vec<Ipv4Addr> {
    if !is_private_ipv4(address) {
        return Vec::new();
    }
    let mask = u32::from(netmask);
    if !mask_is_contiguous(mask) {
        return Vec::new();
    }
    let prefix = mask.leading_ones();
    if prefix > 30 {
        return Vec::new();
    }
    let effective = if prefix < 24 { 0xffff_ff00 } else { mask };
    let ip = u32::from(address);
    let network = ip & effective;
    let broadcast = network | !effective;
    let mut hosts = Vec::new();
    let mut cursor = network.wrapping_add(1);
    while cursor < broadcast && hosts.len() < 254 {
        if cursor != ip {
            hosts.push(Ipv4Addr::from(cursor));
        }
        cursor = cursor.wrapping_add(1);
    }
    hosts
}

fn mask_is_contiguous(mask: u32) -> bool {
    mask != 0 && mask.leading_ones() + mask.trailing_zeros() == 32
}

#[cfg(unix)]
fn local_ipv4_interfaces() -> Vec<(Ipv4Addr, Ipv4Addr)> {
    unsafe { interface_addresses() }
}

#[cfg(not(unix))]
fn local_ipv4_interfaces() -> Vec<(Ipv4Addr, Ipv4Addr)> {
    Vec::new()
}

#[cfg(unix)]
unsafe fn interface_addresses() -> Vec<(Ipv4Addr, Ipv4Addr)> {
    use std::os::raw::c_void;

    #[repr(C)]
    struct IfAddrs {
        next: *mut IfAddrs,
        _name: *mut c_void,
        flags: u32,
        addr: *mut c_void,
        netmask: *mut c_void,
        _dst: *mut c_void,
        _data: *mut c_void,
    }

    extern "C" {
        fn getifaddrs(ifap: *mut *mut IfAddrs) -> i32;
        fn freeifaddrs(ifa: *mut IfAddrs);
    }

    const IFF_UP: u32 = 0x1;
    const IFF_LOOPBACK: u32 = 0x8;

    let mut head: *mut IfAddrs = std::ptr::null_mut();
    if getifaddrs(&mut head) != 0 || head.is_null() {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut cursor = head;
    let mut seen = 0;
    while !cursor.is_null() && seen < 128 {
        seen += 1;
        let flags = (*cursor).flags;
        if flags & IFF_UP != 0 && flags & IFF_LOOPBACK == 0 {
            if let (Some(address), Some(mask)) =
                (read_ipv4((*cursor).addr), read_ipv4((*cursor).netmask))
            {
                found.push((address, mask));
            }
        }
        cursor = (*cursor).next;
    }
    freeifaddrs(head);
    found
}

#[cfg(unix)]
unsafe fn read_ipv4(sockaddr: *mut std::os::raw::c_void) -> Option<Ipv4Addr> {
    if sockaddr.is_null() {
        return None;
    }
    let bytes = sockaddr as *const u8;
    let family = if cfg!(target_os = "linux") {
        std::ptr::read_unaligned(bytes as *const u16)
    } else {
        std::ptr::read_unaligned(bytes.add(1)) as u16
    };
    if family != 2 {
        return None;
    }
    let octets = std::ptr::read_unaligned(bytes.add(4) as *const [u8; 4]);
    Some(Ipv4Addr::from(octets))
}

#[cfg(test)]
mod tests {
    use super::{hosts_to_probe, is_private_ipv4};
    use std::net::Ipv4Addr;

    #[test]
    fn private_ranges_are_the_only_scan_targets() {
        assert!(is_private_ipv4(Ipv4Addr::new(192, 168, 1, 20)));
        assert!(is_private_ipv4(Ipv4Addr::new(10, 1, 2, 3)));
        assert!(is_private_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(!is_private_ipv4(Ipv4Addr::new(172, 32, 0, 1)));
        assert!(!is_private_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn a_slash_24_excludes_the_workstation_network_and_broadcast() {
        let hosts = hosts_to_probe(
            Ipv4Addr::new(192, 168, 1, 20),
            Ipv4Addr::new(255, 255, 255, 0),
        );
        assert_eq!(hosts.len(), 253);
        assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 1, 20)));
        assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 1, 0)));
        assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 1, 255)));
        assert!(hosts.contains(&Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn a_wide_mask_stays_inside_the_interface_slash_24() {
        let hosts = hosts_to_probe(Ipv4Addr::new(10, 4, 8, 5), Ipv4Addr::new(255, 0, 0, 0));
        assert_eq!(hosts.len(), 253);
        assert!(hosts.iter().all(|host| host.octets()[2] == 8));
        assert!(!hosts.contains(&Ipv4Addr::new(10, 4, 8, 5)));
    }

    #[test]
    fn a_public_address_is_not_probed() {
        assert!(hosts_to_probe(
            Ipv4Addr::new(203, 0, 113, 10),
            Ipv4Addr::new(255, 255, 255, 0)
        )
        .is_empty());
    }
}
