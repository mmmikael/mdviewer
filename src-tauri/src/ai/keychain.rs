//! API keys live in the login keychain, never in settings.json or the page.

const SERVICE: &str = "com.mmmikael.mdviewer.ai";

#[cfg(target_os = "macos")]
mod imp {
    use security_framework::passwords;

    /// errSecItemNotFound
    const NOT_FOUND: i32 = -25300;

    pub fn get(service: &str, account: &str) -> Result<Option<String>, String> {
        match passwords::get_generic_password(service, account) {
            Ok(bytes) => Ok(String::from_utf8(bytes).ok()),
            Err(err) if err.code() == NOT_FOUND => Ok(None),
            Err(err) => Err(format!(
                "Couldn’t read the API key from the keychain: {err}"
            )),
        }
    }

    pub fn set(service: &str, account: &str, key: &str) -> Result<(), String> {
        passwords::set_generic_password(service, account, key.as_bytes())
            .map_err(|err| format!("Couldn’t save the API key in the keychain: {err}"))
    }

    pub fn delete(service: &str, account: &str) -> Result<(), String> {
        match passwords::delete_generic_password(service, account) {
            Err(err) if err.code() != NOT_FOUND => Err(format!(
                "Couldn’t remove the API key from the keychain: {err}"
            )),
            _ => Ok(()),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn get(_: &str, _: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    pub fn set(_: &str, _: &str, _: &str) -> Result<(), String> {
        Err("Keys can only be stored on macOS.".into())
    }
    pub fn delete(_: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Blocking: macOS may ask the user to allow access (e.g. after an update).
pub fn get(account: &str) -> Result<Option<String>, String> {
    imp::get(SERVICE, account)
}

pub fn set(account: &str, key: &str) -> Result<(), String> {
    imp::set(SERVICE, account, key)
}

pub fn delete(account: &str) -> Result<(), String> {
    imp::delete(SERVICE, account)
}
