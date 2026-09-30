//! A minimal ZIP writer (stored, no compression) for .docx packages: local headers, the
//! central directory and its end record, with CRC-32s. Word, LibreOffice and Pages read
//! stored entries fine; the files are small text.

/// Builds a ZIP of `(path, bytes)` entries, in order.
pub fn store(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    // 1980-01-01 00:00, the DOS epoch.
    let (time, date) = (0u16, (1u16 << 5) | 1);
    for (name, data) in entries {
        let crc = crc32fast::hash(data);
        let offset = out.len() as u32;
        let size = data.len() as u32;
        let name_bytes = name.as_bytes();
        // Local file header.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 names
        out.extend_from_slice(&0u16.to_le_bytes()); // stored
        out.extend_from_slice(&time.to_le_bytes());
        out.extend_from_slice(&date.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);
        // Its central directory record.
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // made by
        central.extend_from_slice(&20u16.to_le_bytes()); // needed
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&time.to_le_bytes());
        central.extend_from_slice(&date.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 12]); // extra, comment, disk, attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let cd_offset = out.len() as u32;
    let n = entries.len() as u16;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // disk numbers
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads back the entries (stored only), as a check of the structure.
    fn read(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
        let u16at = |i: usize| u16::from_le_bytes([zip[i], zip[i + 1]]) as usize;
        let u32at =
            |i: usize| u32::from_le_bytes([zip[i], zip[i + 1], zip[i + 2], zip[i + 3]]) as usize;
        let end = zip.len() - 22;
        assert_eq!(u32at(end), 0x0605_4b50);
        let (n, mut at) = (u16at(end + 10), u32at(end + 16));
        (0..n)
            .map(|_| {
                assert_eq!(u32at(at), 0x0201_4b50);
                let (size, name_len, off) = (u32at(at + 20), u16at(at + 28), u32at(at + 42));
                let name = String::from_utf8(zip[at + 46..at + 46 + name_len].to_vec()).unwrap();
                at += 46 + name_len;
                let data_at = off + 30 + u16at(off + 26);
                let data = zip[data_at..data_at + size].to_vec();
                assert_eq!(crc32fast::hash(&data), u32at(off + 14) as u32);
                (name, data)
            })
            .collect()
    }

    #[test]
    fn entries_round_trip() {
        let e = vec![
            ("[Content_Types].xml".to_string(), b"<Types/>".to_vec()),
            (
                "word/document.xml".to_string(),
                "Gypsum – board".as_bytes().to_vec(),
            ),
        ];
        assert_eq!(read(&store(&e)), e);
        // The CRC-32 of "123456789" is the standard check value.
        assert_eq!(crc32fast::hash(b"123456789"), 0xCBF4_3926);
    }
}
