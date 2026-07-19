use std::fmt::Write;

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

pub(crate) fn decode<const N: usize>(input: &str) -> Option<[u8; N]> {
    if input.len() != N * 2 || !input.is_ascii() {
        return None;
    }
    let bytes = input.as_bytes();
    let mut output = [0_u8; N];
    for (index, pair) in bytes.chunks_exact(2).enumerate() {
        output[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(output)
}

const fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_is_canonical_and_exact_length() {
        assert_eq!(super::encode(&[0, 1, 15, 16, 255]), "00010f10ff");
        assert_eq!(super::decode::<2>("00ff"), Some([0, 255]));
        assert_eq!(super::decode::<2>("00FF"), None);
        assert_eq!(super::decode::<2>("0ff"), None);
        assert_eq!(super::decode::<2>("zzzz"), None);
    }
}
