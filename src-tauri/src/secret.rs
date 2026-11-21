//! The Riot API key, kept for the signed-in user by the system: encrypted with DPAPI in
//! `riot-key.bin` (the data folder) on Windows, in the login keychain on macOS.

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};

    fn file() -> std::path::PathBuf {
        crate::paths::config_dir().join("riot-key.bin")
    }

    fn run(data: &[u8], protect: bool) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            let ok = if protect {
                CryptProtectData(&input, windows::core::w!("Mattofessor Riot API key"), None, None, None, 0, &mut out)
            } else {
                CryptUnprotectData(&input, None, None, None, None, 0, &mut out)
            };
            ok.ok()?;
            let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
            Some(bytes)
        }
    }

    pub fn load() -> Option<Vec<u8>> {
        run(&std::fs::read(file()).ok()?, false)
    }

    pub fn save(key: &[u8]) -> anyhow::Result<()> {
        let blob = run(key, true).ok_or_else(|| anyhow::anyhow!("DPAPI could not encrypt it"))?;
        Ok(std::fs::write(file(), blob)?)
    }

    pub fn clear() {
        let _ = std::fs::remove_file(file());
    }

    #[cfg(test)]
    #[test]
    fn dpapi_roundtrip() {
        let blob = run(b"RGAPI-test", true).unwrap();
        assert_ne!(blob, b"RGAPI-test");
        assert_eq!(run(&blob, false).unwrap(), b"RGAPI-test");
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

    /// Debug builds keep their own item, as they keep their own data folder.
    const SERVICE: &str = if cfg!(debug_assertions) { "Mattofessor-dev" } else { "Mattofessor" };
    const ACCOUNT: &str = "riot-api-key";

    pub fn load() -> Option<Vec<u8>> {
        get_generic_password(SERVICE, ACCOUNT).ok()
    }

    pub fn save(key: &[u8]) -> anyhow::Result<()> {
        Ok(set_generic_password(SERVICE, ACCOUNT, key)?)
    }

    pub fn clear() {
        let _ = delete_generic_password(SERVICE, ACCOUNT);
    }
}

/// The stored key, if any.
pub fn load() -> Option<String> {
    imp::load().and_then(|b| String::from_utf8(b).ok()).filter(|k| !k.is_empty())
}

pub fn save(key: &str) -> anyhow::Result<()> {
    imp::save(key.as_bytes())
}

pub fn clear() {
    imp::clear()
}
