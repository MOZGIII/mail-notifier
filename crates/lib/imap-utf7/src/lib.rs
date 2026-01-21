//! IMAP modified UTF-7 encoding helpers.
#![allow(unsafe_code)]

/// A borrowed IMAP modified UTF-7 string.
///
/// This is a wrapper around a `str` that is guaranteed to be valid IMAP
/// modified UTF-7 **and** to decode into valid UTF-16 (and therefore valid
/// UTF-8) when using [`decode`](ImapUtf7Str::decode).
#[repr(transparent)]
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ImapUtf7Str(str);

/// An owned IMAP modified UTF-7 string.
///
/// This stores the encoded mailbox name and guarantees it is valid IMAP
/// modified UTF-7, including valid UTF-16 in any encoded sections.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ImapUtf7String {
    /// Inner owned string.
    inner: String,
}

/// Errors returned while validating IMAP modified UTF-7.
///
/// This enum currently mirrors `ImapUtf7DecodeError`, but is kept separate
/// so callers can distinguish validation-only fallibility from decode-time
/// fallibility in public APIs.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ImapUtf7ValidateError {
    /// Encountered a non-ASCII character in the encoded input.
    #[error("non-ASCII character at byte offset {0}")]
    NonAscii(usize),

    /// An encoded sequence started with '&' but did not terminate with '-'.
    #[error("unterminated encoded sequence at byte offset {0}")]
    UnterminatedSequence(usize),

    /// Encountered an invalid base64 character at the given byte offset.
    #[error("invalid base64 character at byte offset {0}")]
    InvalidBase64Char(usize),

    /// Base64 input length is invalid for IMAP modified UTF-7.
    #[error("invalid base64 length at byte offset {0}")]
    InvalidBase64Length(usize),

    /// Base64 decoded bytes length is not even, invalid for UTF-16BE.
    #[error("invalid UTF-16 length at byte offset {0}")]
    InvalidUtf16Length(usize),

    /// Decoded UTF-16 sequence is invalid.
    #[error("invalid UTF-16 sequence")]
    InvalidUtf16,
}

/// Errors returned while decoding IMAP modified UTF-7.
///
/// Kept private because decoding is infallible for validated values.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
enum ImapUtf7DecodeError {
    /// Encountered a non-ASCII character in the encoded input.
    #[error("non-ASCII character at byte offset {0}")]
    NonAscii(usize),

    /// An encoded sequence started with '&' but did not terminate with '-'.
    #[error("unterminated encoded sequence at byte offset {0}")]
    UnterminatedSequence(usize),

    /// Encountered an invalid base64 character at the given byte offset.
    #[error("invalid base64 character at byte offset {0}")]
    InvalidBase64Char(usize),

    /// Base64 input length is invalid for IMAP modified UTF-7.
    #[error("invalid base64 length at byte offset {0}")]
    InvalidBase64Length(usize),

    /// Base64 decoded bytes length is not even, invalid for UTF-16BE.
    #[error("invalid UTF-16 length at byte offset {0}")]
    InvalidUtf16Length(usize),

    /// Decoded UTF-16 sequence is invalid.
    #[error("invalid UTF-16 sequence")]
    InvalidUtf16,
}

impl ImapUtf7Str {
    /// Validate an IMAP modified UTF-7 string.
    ///
    /// This performs **full validation**:
    /// - ASCII-only input.
    /// - Proper `&...-` sequences.
    /// - Modified UTF-7 base64 alphabet and length checks.
    /// - Decoded UTF-16 must be well-formed (no unpaired surrogates).
    ///
    /// On success, returns a borrowed wrapper tied to the input lifetime.
    pub fn new(encoded: &str) -> Result<&Self, ImapUtf7ValidateError> {
        validate_imap_utf7(encoded)?;
        Ok(unsafe { Self::from_str_unchecked(encoded) })
    }

    /// Borrow a validated IMAP modified UTF-7 string without checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure `encoded` is valid IMAP modified UTF-7 **and**
    /// that all encoded sections decode to **valid UTF-16**. If this is not
    /// upheld, subsequent operations like [`decode`](ImapUtf7Str::decode) may
    /// return errors or cause UB when other invariants are assumed.
    pub unsafe fn from_str_unchecked(encoded: &str) -> &Self {
        unsafe { &*(encoded as *const str as *const ImapUtf7Str) }
    }

    /// Return the underlying encoded string slice.
    pub const fn as_str(&self) -> &str {
        &self.0
    }

    /// Decode this IMAP modified UTF-7 string into UTF-8.
    ///
    /// # Panics
    ///
    /// Panics if the instance was constructed with unchecked APIs using
    /// invalid input.
    pub fn decode(&self) -> String {
        decode_imap_utf7(self.as_str()).expect("ImapUtf7Str::decode: invalid unchecked input")
    }

    /// Convert this wrapper into an owned `ImapUtf7String`.
    ///
    /// The resulting owned value preserves the same validation invariants.
    pub fn to_owned_string(&self) -> ImapUtf7String {
        ImapUtf7String {
            inner: self.as_str().to_string(),
        }
    }
}

impl ImapUtf7String {
    /// Encode a UTF-8 mailbox name into IMAP modified UTF-7.
    ///
    /// The output is guaranteed to be valid IMAP modified UTF-7 and to decode
    /// back into the original UTF-8 string.
    pub fn from_utf8(input: &str) -> Self {
        Self {
            inner: encode_imap_utf7(input),
        }
    }

    /// Create a wrapper from an already encoded IMAP modified UTF-7 string.
    ///
    /// Performs full validation (syntax + decoded UTF-16 validity).
    pub fn from_imap_utf7(encoded: String) -> Result<Self, ImapUtf7ValidateError> {
        validate_imap_utf7(encoded.as_str())?;
        Ok(Self { inner: encoded })
    }

    /// Borrow as an `ImapUtf7Str` wrapper.
    ///
    /// This is safe because all constructors validate the stored string.
    pub fn as_imap_utf7_str(&self) -> &ImapUtf7Str {
        // SAFETY: `self.inner` is validated on construction or encoding.
        unsafe { ImapUtf7Str::from_str_unchecked(self.inner.as_str()) }
    }

    /// Return the underlying encoded string slice.
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Decode this IMAP modified UTF-7 string into UTF-8.
    ///
    /// # Panics
    ///
    /// Panics if the instance was constructed with unchecked APIs using
    /// invalid input.
    pub fn decode(&self) -> String {
        decode_imap_utf7(self.inner.as_str())
            .expect("ImapUtf7String::decode: invalid unchecked input")
    }
}

impl std::fmt::Display for ImapUtf7String {
    /// Format the decoded UTF-8 string.
    ///
    /// Returns a formatting error only if the internal value was constructed
    /// via unchecked APIs with invalid data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.decode().fmt(f)
    }
}

impl std::fmt::Display for ImapUtf7Str {
    /// Format the decoded UTF-8 string slice.
    ///
    /// Returns a formatting error only if the value was constructed via
    /// unchecked APIs with invalid data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.decode().fmt(f)
    }
}

impl std::ops::Deref for ImapUtf7String {
    /// Dereference to the encoded `ImapUtf7Str` slice.
    ///
    /// This is safe because the internal string is validated on construction.
    type Target = ImapUtf7Str;

    fn deref(&self) -> &Self::Target {
        // SAFETY: `self.inner` is validated on construction or encoding.
        unsafe { ImapUtf7Str::from_str_unchecked(self.inner.as_str()) }
    }
}

impl AsRef<ImapUtf7Str> for ImapUtf7String {
    fn as_ref(&self) -> &ImapUtf7Str {
        self.as_imap_utf7_str()
    }
}

impl AsRef<str> for ImapUtf7String {
    fn as_ref(&self) -> &str {
        self.inner.as_str()
    }
}

impl AsRef<str> for ImapUtf7Str {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for ImapUtf7String {
    type Error = ImapUtf7ValidateError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Full validation (syntax + UTF-16 correctness).
        validate_imap_utf7(value)?;
        Ok(Self {
            inner: value.to_string(),
        })
    }
}

impl std::str::FromStr for ImapUtf7String {
    type Err = ImapUtf7ValidateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Delegates to the fully validating TryFrom.
        ImapUtf7String::try_from(s)
    }
}

impl From<&ImapUtf7Str> for ImapUtf7String {
    fn from(value: &ImapUtf7Str) -> Self {
        value.to_owned_string()
    }
}

impl std::borrow::Borrow<ImapUtf7Str> for ImapUtf7String {
    /// Borrow as an IMAP modified UTF-7 string slice.
    fn borrow(&self) -> &ImapUtf7Str {
        self.as_imap_utf7_str()
    }
}

impl std::borrow::Borrow<str> for ImapUtf7String {
    /// Borrow as a plain string slice.
    fn borrow(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::borrow::Borrow<str> for ImapUtf7Str {
    /// Borrow as a plain string slice.
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::borrow::ToOwned for ImapUtf7Str {
    type Owned = ImapUtf7String;

    /// Create an owned IMAP modified UTF-7 string.
    ///
    /// The owned value preserves the same validation invariants.
    fn to_owned(&self) -> Self::Owned {
        self.to_owned_string()
    }
}

/// Encode a UTF-8 string into IMAP modified UTF-7.
fn encode_imap_utf7(input: &str) -> String {
    if input
        .bytes()
        .all(|byte| (0x20..=0x7e).contains(&byte) && byte != b'&')
    {
        return input.to_string();
    }

    let mut out = String::new();
    let mut buf: Vec<u16> = Vec::new();

    let flush = |buf: &mut Vec<u16>, out: &mut String| {
        if buf.is_empty() {
            return;
        }

        let mut bytes = Vec::with_capacity(buf.len() * 2);
        for code_unit in buf.drain(..) {
            bytes.push((code_unit >> 8) as u8);
            bytes.push((code_unit & 0xff) as u8);
        }

        out.push('&');
        out.push_str(&encode_mutf7_base64(&bytes));
        out.push('-');
    };

    for ch in input.chars() {
        let code = ch as u32;
        let is_direct = (0x20..=0x7e).contains(&code) && ch != '&';

        if is_direct {
            flush(&mut buf, &mut out);
            out.push(ch);
            continue;
        }

        if ch == '&' {
            flush(&mut buf, &mut out);
            out.push_str("&-");
            continue;
        }

        let mut tmp = [0u16; 2];
        for unit in ch.encode_utf16(&mut tmp) {
            buf.push(*unit);
        }
    }

    flush(&mut buf, &mut out);
    out
}

/// Validate IMAP modified UTF-7 input without allocating.
fn validate_imap_utf7(input: &str) -> Result<(), ImapUtf7ValidateError> {
    let mut chars = input.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if !ch.is_ascii() {
            return Err(ImapUtf7ValidateError::NonAscii(idx));
        }

        if ch != '&' {
            continue;
        }

        match chars.peek() {
            Some((_, '-')) => {
                chars.next();
            }
            _ => {
                let seq_start = idx;
                let mut has_end = false;
                let mut seq_len = 0usize;
                let mut seq_end = idx + 1;
                for (inner_idx, inner_ch) in chars.by_ref() {
                    if inner_ch == '-' {
                        has_end = true;
                        break;
                    }

                    if !is_mutf7_base64_char(inner_ch) {
                        return Err(ImapUtf7ValidateError::InvalidBase64Char(inner_idx));
                    }

                    seq_len += 1;
                    seq_end = inner_idx + inner_ch.len_utf8();
                }

                if !has_end {
                    return Err(ImapUtf7ValidateError::UnterminatedSequence(seq_start));
                }

                if seq_len % 4 == 1 {
                    return Err(ImapUtf7ValidateError::InvalidBase64Length(seq_end));
                }

                validate_mutf7_base64_utf16(&input[seq_start + 1..seq_end], seq_end)?;
            }
        }
    }

    Ok(())
}

/// Decode IMAP modified UTF-7 input into UTF-8.
fn decode_imap_utf7(input: &str) -> Result<String, ImapUtf7DecodeError> {
    let mut out = String::new();
    let mut chars = input.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        if !ch.is_ascii() {
            return Err(ImapUtf7DecodeError::NonAscii(idx));
        }

        if ch != '&' {
            out.push(ch);
            continue;
        }

        match chars.peek() {
            Some((_, '-')) => {
                chars.next();
                out.push('&');
            }
            _ => {
                let seq_start = idx + 1;
                let mut seq_end = seq_start;
                let mut has_end = false;
                for (inner_idx, inner_ch) in chars.by_ref() {
                    if inner_ch == '-' {
                        has_end = true;
                        break;
                    }

                    if !is_mutf7_base64_char(inner_ch) {
                        return Err(ImapUtf7DecodeError::InvalidBase64Char(inner_idx));
                    }

                    seq_end = inner_idx + inner_ch.len_utf8();
                }

                if !has_end {
                    return Err(ImapUtf7DecodeError::UnterminatedSequence(seq_start - 1));
                }

                let bytes = decode_mutf7_base64_slice(&input[seq_start..seq_end])?;
                if bytes.len() % 2 != 0 {
                    return Err(ImapUtf7DecodeError::InvalidUtf16Length(seq_end));
                }

                let mut utf16 = Vec::with_capacity(bytes.len() / 2);
                for chunk in bytes.chunks(2) {
                    let code = ((chunk[0] as u16) << 8) | (chunk[1] as u16);
                    utf16.push(code);
                }

                let decoded =
                    String::from_utf16(&utf16).map_err(|_| ImapUtf7DecodeError::InvalidUtf16)?;
                out.push_str(&decoded);
            }
        }
    }

    Ok(out)
}

/// Return true when the character is in the IMAP modified UTF-7 base64 alphabet.
const fn is_mutf7_base64_char(ch: char) -> bool {
    matches!(ch, 'A'..='Z' | 'a'..='z' | '0'..='9' | '+' | ',')
}

/// Decode IMAP modified UTF-7 base64 data into raw bytes.
fn decode_mutf7_base64_slice(input: &str) -> Result<Vec<u8>, ImapUtf7DecodeError> {
    let mut sextets = Vec::with_capacity(input.len());
    for (idx, ch) in input.char_indices() {
        let val = match ch {
            'A'..='Z' => (ch as u8) - b'A',
            'a'..='z' => (ch as u8) - b'a' + 26,
            '0'..='9' => (ch as u8) - b'0' + 52,
            '+' => 62,
            ',' => 63,
            _ => return Err(ImapUtf7DecodeError::InvalidBase64Char(idx)),
        };
        sextets.push(val);
    }

    if sextets.len() % 4 == 1 {
        return Err(ImapUtf7DecodeError::InvalidBase64Length(input.len()));
    }

    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 <= sextets.len() {
        let b0 = sextets[i];
        let b1 = sextets[i + 1];
        let b2 = sextets[i + 2];
        let b3 = sextets[i + 3];
        out.push((b0 << 2) | (b1 >> 4));
        out.push((b1 << 4) | (b2 >> 2));
        out.push((b2 << 6) | b3);
        i += 4;
    }

    let rem = sextets.len() - i;
    if rem == 2 {
        let b0 = sextets[i];
        let b1 = sextets[i + 1];
        out.push((b0 << 2) | (b1 >> 4));
    } else if rem == 3 {
        let b0 = sextets[i];
        let b1 = sextets[i + 1];
        let b2 = sextets[i + 2];
        out.push((b0 << 2) | (b1 >> 4));
        out.push((b1 << 4) | (b2 >> 2));
    }

    Ok(out)
}

/// Validate IMAP modified UTF-7 base64 data and decoded UTF-16 without allocating.
fn validate_mutf7_base64_utf16(input: &str, seq_end: usize) -> Result<(), ImapUtf7ValidateError> {
    let mut sextets = [0u8; 4];
    let mut sextet_len = 0usize;
    let mut pending_byte: Option<u8> = None;
    let mut pending_high_surrogate: Option<u16> = None;

    let mut push_byte = |byte: u8| -> Result<(), ImapUtf7ValidateError> {
        if let Some(high) = pending_byte.take() {
            let code_unit = ((high as u16) << 8) | (byte as u16);
            match code_unit {
                0xD800..=0xDBFF => {
                    if pending_high_surrogate.is_some() {
                        return Err(ImapUtf7ValidateError::InvalidUtf16);
                    }
                    pending_high_surrogate = Some(code_unit);
                }
                0xDC00..=0xDFFF => {
                    if pending_high_surrogate.take().is_none() {
                        return Err(ImapUtf7ValidateError::InvalidUtf16);
                    }
                }
                _ => {
                    if pending_high_surrogate.is_some() {
                        return Err(ImapUtf7ValidateError::InvalidUtf16);
                    }
                }
            }
        } else {
            pending_byte = Some(byte);
        }

        Ok(())
    };

    for (idx, ch) in input.char_indices() {
        let val = match ch {
            'A'..='Z' => (ch as u8) - b'A',
            'a'..='z' => (ch as u8) - b'a' + 26,
            '0'..='9' => (ch as u8) - b'0' + 52,
            '+' => 62,
            ',' => 63,
            _ => return Err(ImapUtf7ValidateError::InvalidBase64Char(idx)),
        };

        sextets[sextet_len] = val;
        sextet_len += 1;

        if sextet_len == 4 {
            let b0 = sextets[0];
            let b1 = sextets[1];
            let b2 = sextets[2];
            let b3 = sextets[3];
            push_byte((b0 << 2) | (b1 >> 4))?;
            push_byte((b1 << 4) | (b2 >> 2))?;
            push_byte((b2 << 6) | b3)?;
            sextet_len = 0;
        }
    }

    if sextet_len == 1 {
        return Err(ImapUtf7ValidateError::InvalidBase64Length(seq_end));
    }

    if sextet_len == 2 {
        let b0 = sextets[0];
        let b1 = sextets[1];
        push_byte((b0 << 2) | (b1 >> 4))?;
    } else if sextet_len == 3 {
        let b0 = sextets[0];
        let b1 = sextets[1];
        let b2 = sextets[2];
        push_byte((b0 << 2) | (b1 >> 4))?;
        push_byte((b1 << 4) | (b2 >> 2))?;
    }

    if pending_byte.is_some() {
        return Err(ImapUtf7ValidateError::InvalidUtf16Length(seq_end));
    }

    if pending_high_surrogate.is_some() {
        return Err(ImapUtf7ValidateError::InvalidUtf16);
    }

    Ok(())
}

/// Encode bytes using the IMAP modified UTF-7 base64 alphabet (no padding).
fn encode_mutf7_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,";

    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        let idx0 = (b0 >> 2) as usize;
        let idx1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let idx2 = (((b1 & 0x0f) << 2) | (b2 >> 6)) as usize;
        let idx3 = (b2 & 0x3f) as usize;
        out.push(ALPHABET[idx0] as char);
        out.push(ALPHABET[idx1] as char);
        out.push(ALPHABET[idx2] as char);
        out.push(ALPHABET[idx3] as char);
        i += 3;
    }

    match bytes.len() - i {
        1 => {
            let b0 = bytes[i];
            let idx0 = (b0 >> 2) as usize;
            let idx1 = ((b0 & 0x03) << 4) as usize;
            out.push(ALPHABET[idx0] as char);
            out.push(ALPHABET[idx1] as char);
        }
        2 => {
            let b0 = bytes[i];
            let b1 = bytes[i + 1];
            let idx0 = (b0 >> 2) as usize;
            let idx1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
            let idx2 = ((b1 & 0x0f) << 2) as usize;
            out.push(ALPHABET[idx0] as char);
            out.push(ALPHABET[idx1] as char);
            out.push(ALPHABET[idx2] as char);
        }
        _ => {}
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{ImapUtf7Str, ImapUtf7String};

    #[test]
    fn encode_ascii() {
        let encoded = ImapUtf7String::from_utf8("INBOX");
        assert_eq!(encoded.as_str(), "INBOX");
    }

    #[test]
    fn encode_ampersand() {
        let encoded = ImapUtf7String::from_utf8("A&B");
        assert_eq!(encoded.as_str(), "A&-B");
    }

    #[test]
    fn encode_nbsp() {
        let encoded = ImapUtf7String::from_utf8("Project\u{00A0}Notes");
        assert_eq!(encoded.as_str(), "Project&AKA-Notes");
    }

    #[test]
    fn decode_literal_ampersand() {
        let encoded = ImapUtf7Str::new("&-").unwrap();
        let decoded = encoded.decode();
        assert_eq!(decoded, "&");
    }

    #[test]
    fn decode_nbsp() {
        let encoded = ImapUtf7Str::new("Project&AKA-Notes").unwrap();
        let decoded = encoded.decode();
        assert_eq!(decoded, "Project\u{00A0}Notes");
    }

    #[test]
    fn reject_non_ascii() {
        let err = ImapUtf7Str::new("тест").unwrap_err();
        assert!(matches!(err, super::ImapUtf7ValidateError::NonAscii(_)));
    }

    #[test]
    fn reject_unterminated_sequence() {
        let err = ImapUtf7Str::new("Bad&AAA").unwrap_err();
        assert!(matches!(
            err,
            super::ImapUtf7ValidateError::UnterminatedSequence(_)
        ));
    }

    #[test]
    fn reject_invalid_base64_char() {
        let err = ImapUtf7Str::new("Bad&AA=-").unwrap_err();
        assert!(matches!(
            err,
            super::ImapUtf7ValidateError::InvalidBase64Char(_)
        ));
    }

    #[test]
    fn reject_invalid_base64_length() {
        let err = ImapUtf7Str::new("Bad&A-").unwrap_err();
        assert!(matches!(
            err,
            super::ImapUtf7ValidateError::InvalidBase64Length(_)
        ));
    }

    #[test]
    fn reject_invalid_utf16_length() {
        let err = ImapUtf7Str::new("Bad&AA-").unwrap_err();
        assert!(matches!(
            err,
            super::ImapUtf7ValidateError::InvalidUtf16Length(_)
        ));
    }

    #[test]
    fn reject_invalid_utf16_surrogate() {
        let err = ImapUtf7Str::new("Bad&2AA-").unwrap_err();
        assert!(matches!(err, super::ImapUtf7ValidateError::InvalidUtf16));
    }

    #[test]
    fn display_decodes_utf7() {
        let encoded = ImapUtf7Str::new("Project&AKA-Notes").unwrap();
        assert_eq!(encoded.to_string(), "Project\u{00A0}Notes");
    }
}
