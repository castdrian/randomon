const TABLE: [u16; 256] = build_table();

const fn build_table() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut index = 0;
    while index < 256 {
        let mut value = index as u16;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 1 != 0 {
                (value >> 1) ^ 0xA001
            } else {
                value >> 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

pub fn calculate(data: &[u8]) -> u16 {
    data.iter().fold(0xFFFFu16, |crc, &byte| {
        (crc >> 8) ^ TABLE[((crc ^ byte as u16) & 0xFF) as usize]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_upstream_literal_entries() {
        assert_eq!(TABLE[0], 0x0000);
        assert_eq!(TABLE[1], 0xC0C1);
        assert_eq!(TABLE[2], 0xC181);
        assert_eq!(TABLE[3], 0x0140);
        assert_eq!(TABLE[0x80], 0xA001);
        assert_eq!(TABLE[0xFF], 0x4040);
    }

    #[test]
    fn modbus_check_value() {
        assert_eq!(calculate(b"123456789"), 0x4B37);
    }
}
