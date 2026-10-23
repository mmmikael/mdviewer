//! API keys live in the login keychain, never in settings.json or the page.

const SERVICE: &str = "com.mmmikael.mdviewer.ai";

#[cfg(target_os = "macos")]
mod imp {
    use security_framework::passwords;

    /// errSecItemNotFound
    const NOT_FOUND: i32 = -25300;
    /// The user clicked Deny (userCanceledErr), or access wasn't granted
    /// (errSecAuthFailed, errSecInteractionNotAllowed).
    const DENIED: [i32; 3] = [-128, -25293, -25308];

    fn message(action: &str, err: security_framework::base::Error) -> String {
        if DENIED.contains(&err.code()) {
            format!(
                "MDViewer wasn’t allowed to {action} the keychain. Try again, and choose Always Allow when macOS asks."
            )
        } else {
            format!("Couldn’t {action} the keychain: {err}")
        }
    }

    pub fn get(service: &str, account: &str) -> Result<Option<String>, String> {
        match passwords::get_generic_password(service, account) {
            Ok(bytes) => Ok(String::from_utf8(bytes).ok()),
            Err(err) if err.code() == NOT_FOUND => Ok(None),
            Err(err) => Err(message("read your API key from", err)),
        }
    }

    pub fn set(service: &str, account: &str, key: &str) -> Result<(), String> {
        passwords::set_generic_password(service, account, key.as_bytes())
            .map_err(|err| message("save your API key in", err))
    }

    pub fn delete(service: &str, account: &str) -> Result<(), String> {
        match passwords::delete_generic_password(service, account) {
            Err(err) if err.code() != NOT_FOUND => Err(message("remove your API key from", err)),
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

/// Blocking: macOS may ask the user to allow access. MDViewer is signed
/// without an Apple developer identity, so it asks again after each update.
pub fn get(account: &str) -> Result<Option<String>, String> {
    imp::get(SERVICE, account)
}

pub fn set(account: &str, key: &str) -> Result<(), String> {
    imp::set(SERVICE, account, key)
}

pub fn delete(account: &str) -> Result<(), String> {
    imp::delete(SERVICE, account)
}
