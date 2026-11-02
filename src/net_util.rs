use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn local_bind_addresses() -> std::io::Result<Vec<IpAddr>> {
    let mut addresses = BTreeSet::new();
    addresses.extend([
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
    ]);

    for interface in if_addrs::get_if_addrs()? {
        let ip = interface.ip();
        if !ip.is_unspecified() && (interface.is_oper_up() || ip.is_loopback()) {
            addresses.insert(ip);
        }
    }

    Ok(addresses.into_iter().collect())
}

pub fn is_local_bind_address(ip: IpAddr) -> bool {
    local_bind_addresses()
        .map(|addresses| addresses.contains(&ip))
        .unwrap_or_else(|_| ip.is_loopback())
}

pub fn format_bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if value < 1024 {
        return format!("{value} B");
    }

    let mut amount = value as f64;
    let mut unit = 0usize;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    if amount >= 100.0 {
        format!("{amount:.0} {}", UNITS[unit])
    } else if amount >= 10.0 {
        format!("{amount:.1} {}", UNITS[unit])
    } else {
        format!("{amount:.2} {}", UNITS[unit])
    }
}

pub fn format_rate(bytes_per_second: u64) -> String {
    let formatted = format_bytes(bytes_per_second);
    format!("{formatted}/s")
}

pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes_automatically() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(10 * 1024), "10.0 KB");
        assert_eq!(format_bytes(125 * 1024), "125 KB");
        assert_eq!(format_rate(1024 * 1024), "1.00 MB/s");
    }
}
