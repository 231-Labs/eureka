/// Heuristic: treat buffer as cleartext STL if it looks like ASCII or binary STL; otherwise assume encrypted.
pub fn is_file_encrypted(data: &[u8]) -> bool {
    if data.len() < 5 {
        return true;
    }

    let header = String::from_utf8_lossy(&data[..5]);
    if header.starts_with("solid") {
        return false;
    }

    if data.len() > 84 {
        let triangle_count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]);
        if triangle_count > 0 && triangle_count < 1_000_000 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_file_encrypted_ascii_stl() {
        let stl_data = b"solid cube\n  facet normal 0 0 1\n";
        assert!(!is_file_encrypted(stl_data));
    }

    #[test]
    fn test_is_file_encrypted_encrypted_data() {
        let encrypted_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00];
        assert!(is_file_encrypted(&encrypted_data));
    }
}
