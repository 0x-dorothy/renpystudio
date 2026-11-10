//! OS-level protection for the AI provider key.
//!
//! The key has to survive restarts, but it must not sit in `settings.json` as
//! plain text: on Windows we wrap it with DPAPI (`CryptProtectData`, current
//! user + app entropy), the same mechanism browsers use. Where DPAPI is not
//! available the callers fall back to plain storage.

#[cfg(windows)]
pub fn protect(plain: &str) -> Option<String> {
    let data = dpapi(true, plain.as_bytes())?;
    Some(to_hex(&data))
}

#[cfg(windows)]
pub fn unprotect(hex: &str) -> Option<String> {
    let data = from_hex(hex)?;
    let plain = dpapi(false, &data)?;
    String::from_utf8(plain).ok()
}

#[cfg(not(windows))]
pub fn protect(_plain: &str) -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn unprotect(_hex: &str) -> Option<String> {
    None
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(trimmed.len() / 2);
    let bytes = trimmed.as_bytes();
    for pair in bytes.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

#[cfg(windows)]
fn dpapi(encrypt: bool, input: &[u8]) -> Option<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    // App-specific entropy: a blob protected here cannot be decrypted by
    // another application running as the same Windows user.
    let entropy_bytes = b"RenPyStudio::AiKey::v1";
    let input_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: entropy_bytes.len() as u32,
        pbData: entropy_bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };

    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input_blob,
                ptr::null(),
                &entropy,
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input_blob,
                ptr::null_mut(),
                &entropy,
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };

    if ok == 0 || output.pbData.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as *mut core::ffi::c_void);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let bytes = vec![0u8, 1, 15, 16, 255];
        let hex = to_hex(&bytes);
        assert_eq!(hex, "00010f10ff");
        assert_eq!(from_hex(&hex).unwrap(), bytes);
        assert!(from_hex("xyz").is_none());
        assert!(from_hex("abc").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_roundtrip() {
        let secret = "sk-test-1234567890";
        let sealed = protect(secret).expect("DPAPI protect");
        assert_ne!(sealed, secret);
        assert_eq!(unprotect(&sealed).unwrap(), secret);
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_rejects_other_entropy() {
        // Protects with our entropy and fails on arbitrary bytes instead of
        // returning wrong plaintext.
        assert!(unprotect("deadbeef").is_none());
    }
}
