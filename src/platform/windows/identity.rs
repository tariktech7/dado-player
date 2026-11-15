use windows_sys::Win32::{
    NetworkManagement::IpHelper::{GetAdaptersInfo, IP_ADAPTER_INFO},
    System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ},
};
pub fn machine_id() -> Option<String> {
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut text = [0u16; 128];
    let mut size = std::mem::size_of_val(&text) as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            wide("SOFTWARE\\Microsoft\\Cryptography").as_ptr(),
            wide("MachineGuid").as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            text.as_mut_ptr().cast(),
            &mut size,
        )
    };
    (rc == 0)
        .then(|| {
            String::from_utf16_lossy(
                &text[..text.iter().position(|c| *c == 0).unwrap_or(text.len())],
            )
        })
        .filter(|s| !s.is_empty())
}
pub fn mac_addresses() -> Vec<String> {
    let mut size = 0u32;
    unsafe {
        GetAdaptersInfo(std::ptr::null_mut(), &mut size);
    }
    if size == 0 || size > 1024 * 1024 {
        return Vec::new();
    }
    let mut buffer = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
    let mut adapter = buffer.as_mut_ptr().cast::<IP_ADAPTER_INFO>();
    if unsafe { GetAdaptersInfo(adapter, &mut size) } != 0 {
        return Vec::new();
    }
    let mut rows: Vec<(bool, String)> = Vec::new();
    for _ in 0..256 {
        if adapter.is_null() {
            break;
        }
        let a = unsafe { &*adapter };
        let description = a
            .Description
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| *b as u8 as char)
            .collect::<String>()
            .to_lowercase();
        let virtual_adapter = [
            "virtual",
            "wi-fi direct",
            "wifi direct",
            "hyper-v",
            "vmware",
            "bluetooth",
            "tunnel",
            "vpn",
        ]
        .iter()
        .any(|name| description.contains(name));
        if matches!(a.Type, 6 | 71)
            && !virtual_adapter
            && a.AddressLength == 6
            && a.Address[..6].iter().any(|b| *b != 0)
        {
            let value = a.Address[..6]
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":");
            let has_ip = a
                .IpAddressList
                .IpAddress
                .String
                .iter()
                .take_while(|b| **b != 0)
                .map(|b| *b as u8 as char)
                .collect::<String>()
                .parse::<std::net::Ipv4Addr>()
                .is_ok_and(|ip| !ip.is_unspecified());
            let has_gateway = a
                .GatewayList
                .IpAddress
                .String
                .iter()
                .take_while(|b| **b != 0)
                .map(|b| *b as u8 as char)
                .collect::<String>()
                .parse::<std::net::Ipv4Addr>()
                .is_ok_and(|ip| !ip.is_unspecified());
            if !rows.iter().any(|(_, mac)| *mac == value) {
                rows.push((has_ip && has_gateway, value));
            }
        }
        adapter = a.Next;
    }
    // Display and register the same primary adapter; Wi-Fi Direct duplicates
    // must not look like four different activation identities.
    rows.sort_by_key(|(active, _)| !*active);
    rows.into_iter().map(|(_, mac)| mac).collect()
}
