//! Shared by launcher and updater: Authenticode appends a certificate table after the overlay.
//! This only locates the package; Windows verifies the signature's trust.
pub fn footer_offset(data: &[u8]) -> Result<usize, String> {
    fn u32_at(data: &[u8], at: usize) -> Option<usize> {
        Some(u32::from_le_bytes(data.get(at..at.checked_add(4)?)?.try_into().ok()?) as usize)
    }
    fn footer_before(data: &[u8], end: usize, padding: usize) -> Option<usize> {
        for pad in 0..=padding {
            let start = end.checked_sub(64 + pad)?;
            if data.get(start..start + 8) == Some(b"DADOPK01")
                && data.get(start + 64..end)?.iter().all(|b| *b == 0)
            {
                return Some(start);
            }
        }
        None
    }
    if let Some(at) = footer_before(data, data.len(), 0) {
        return Ok(at);
    }
    let invalid = || "Kein gültiges eingebettetes Paket gefunden.".to_owned();
    if data.get(0..2) != Some(b"MZ") {
        return Err(invalid());
    }
    let pe = u32_at(data, 0x3c).ok_or_else(invalid)?;
    if data.get(pe..pe.saturating_add(4)) != Some(b"PE\0\0") {
        return Err(invalid());
    }
    let optional = pe.checked_add(24).ok_or_else(invalid)?;
    let magic = data
        .get(optional..optional.saturating_add(2))
        .ok_or_else(invalid)?;
    let directories = match magic {
        [0x0b, 0x02] => 112,
        [0x0b, 0x01] => 96,
        _ => return Err(invalid()),
    };
    let security = optional
        .checked_add(directories + 4 * 8)
        .ok_or_else(invalid)?;
    let cert_at = u32_at(data, security).ok_or_else(invalid)?;
    let cert_len = u32_at(data, security + 4).ok_or_else(invalid)?;
    if cert_at < security + 8
        || !cert_at.is_multiple_of(8)
        || cert_len < 8
        || cert_at.checked_add(cert_len) != Some(data.len())
    {
        return Err(invalid());
    }
    footer_before(data, cert_at, 7).ok_or_else(invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unsigned() -> Vec<u8> {
        let mut data = vec![0u8; 1027 + 64];
        data[..2].copy_from_slice(b"MZ");
        data[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
        data[128..132].copy_from_slice(b"PE\0\0");
        data[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        data[1027..1035].copy_from_slice(b"DADOPK01");
        data
    }
    fn signed() -> Vec<u8> {
        let mut data = unsigned();
        while !data.len().is_multiple_of(8) {
            data.push(0);
        }
        let at = data.len() as u32;
        let security = 152 + 112 + 4 * 8;
        data[security..security + 4].copy_from_slice(&at.to_le_bytes());
        data[security + 4..security + 8].copy_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&[16, 0, 0, 0, 0, 2, 2, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
        data
    }
    #[test]
    fn unsigned_and_certificate_overlay_locate_the_same_package() {
        assert_eq!(footer_offset(&unsigned()).unwrap(), 1027);
        assert_eq!(footer_offset(&signed()).unwrap(), 1027);
    }
    #[test]
    fn rejects_truncation_and_unexplained_trailing_bytes() {
        let mut data = signed();
        data.pop();
        assert!(footer_offset(&data).is_err());
        let mut data = unsigned();
        data.push(1);
        assert!(footer_offset(&data).is_err());
        assert!(footer_offset(&[]).is_err());
    }
    #[test]
    fn rejects_out_of_bounds_certificate_offsets() {
        let mut data = signed();
        data[296..300].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(footer_offset(&data).is_err());
    }
}
