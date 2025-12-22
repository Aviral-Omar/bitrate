use std::fs;

pub fn get_default_network_interface() -> Option<String> {
    if let Ok(routes) = fs::read_to_string("/proc/net/route") {
        for line in routes.lines().skip(1) {
            // Skip header
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() > 1 && fields[1] == "00000000" {
                return Some(fields[0].to_string());
            }
        }
    }
    None
}

pub fn get_received_bytes(network_interface: &str) -> Option<u64> {
    let rx_bytes_path = format!("/sys/class/net/{}/statistics/rx_bytes", network_interface);
    if let Ok(received_bytes_str) = fs::read_to_string(rx_bytes_path) {
        return u64::from_str_radix(received_bytes_str.trim_end(), 10).ok();
    }
    None
}

pub fn get_sent_bytes(network_interface: &str) -> Option<u64> {
    let tx_bytes_path = format!("/sys/class/net/{}/statistics/tx_bytes", network_interface);
    if let Ok(sent_bytes_str) = fs::read_to_string(tx_bytes_path) {
        return u64::from_str_radix(sent_bytes_str.trim_end(), 10).ok();
    }
    None
}
