use crate::utils::Result;
use encoding_rs::Encoding;

pub fn decode_bytes(bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() {
        return Ok(String::new());
    }

    // Check BOM
    if bytes.starts_with(b"\xef\xbb\xbf") {
        return Ok(String::from_utf8_lossy(&bytes[3..]).to_string());
    }

    // Scan initial bytes for XML encoding declaration: <?xml ... encoding="..." ?>
    let sample_len = bytes.len().min(1024);
    let sample_str = String::from_utf8_lossy(&bytes[..sample_len]);

    let mut detected_encoding_name = "utf-8";
    if let Some(xml_decl_start) = sample_str.find("<?xml") {
        if let Some(xml_decl_end) = sample_str[xml_decl_start..].find(">") {
            let decl = &sample_str[xml_decl_start..xml_decl_start + xml_decl_end];
            if let Some(enc_idx) = decl.to_lowercase().find("encoding=") {
                let rest = &decl[enc_idx + 9..];
                let quote_char = rest.chars().next().unwrap_or('"');
                if quote_char == '"' || quote_char == '\'' {
                    if let Some(end_quote) = rest[1..].find(quote_char) {
                        let enc_str = &rest[1..1 + end_quote];
                        detected_encoding_name = enc_str;
                    }
                }
            }
        }
    }

    let encoding =
        Encoding::for_label(detected_encoding_name.as_bytes()).unwrap_or(encoding_rs::UTF_8);
    let (cow, _, _had_errors) = encoding.decode(bytes);
    Ok(cow.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_bom() {
        let bytes = b"\xef\xbb\xbfHello world";
        assert_eq!(decode_bytes(bytes).unwrap(), "Hello world");
    }

    #[test]
    fn test_xml_encoding_declaration() {
        let (win1251_bytes, _, _) = encoding_rs::WINDOWS_1251
            .encode("<?xml version=\"1.0\" encoding=\"windows-1251\"?><book>Привет</book>");
        let decoded = decode_bytes(&win1251_bytes).unwrap();
        assert!(decoded.contains("Привет"));
    }
}
