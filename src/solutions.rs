use anyhow::{Context, Result};
use base64::Engine;

/// Spoiler shield, not security: the key ships with the repo. It only keeps
/// solutions from being read by accident while browsing the tree.
/// Must match KEY in scripts/solutions_codec.py.
const XOR_KEY: &[u8] = b"clings-spoiler-shield";

fn xor(data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect()
}

/// Decode a .c.enc payload (base64 over XOR-ed bytes) back to C source.
pub fn decode(encoded: &str) -> Result<String> {
    let compact: String = encoded.split_whitespace().collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(compact)
        .context("Solution file is corrupted (invalid base64)")?;
    String::from_utf8(xor(&bytes)).context("Solution file is corrupted (invalid UTF-8)")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(plain: &str) -> String {
        base64::engine::general_purpose::STANDARD.encode(xor(plain.as_bytes()))
    }

    #[test]
    fn roundtrip() {
        let src = "#include <stdio.h>\nint main(void) { return 0; }\n";
        assert_eq!(decode(&encode(src)).unwrap(), src);
    }

    #[test]
    fn decode_ignores_line_wrapping() {
        let src = "int x;\n";
        let enc = encode(src);
        let wrapped = format!("{}\n{}\n", &enc[..4], &enc[4..]);
        assert_eq!(decode(&wrapped).unwrap(), src);
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode("not!!valid@@base64").is_err());
    }
}
