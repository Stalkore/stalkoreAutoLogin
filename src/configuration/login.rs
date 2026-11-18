//! Select first, then validate. Invalid environment values never fall back to file.
use crate::valid_credentials;
use configparser::ini::Ini;
use std::ffi::{CString, OsString};

pub(crate) struct LoginInputs {
    pub user: CString,
    pub password: CString,
    pub server: u32,
    pub character: u8,
    pub pin: Option<String>,
}

fn file_value(file: &Ini, key: &str) -> Option<String> {
    file.get("login", key).map(|value| {
        // Matching outer quotes preserve surrounding spaces. Their contents are
        // literal: do not unescape backslashes or interpolate environment names.
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value[1..value.len() - 1].to_owned()
        } else {
            value
        }
    })
}
fn selected(
    file: &Ini,
    environment: &impl Fn(&str) -> Option<OsString>,
    variable: &str,
    key: &str,
) -> Result<Option<String>, &'static str> {
    match environment(variable) {
        Some(value) => value
            .into_string()
            .map(Some)
            .map_err(|_| "a login environment override is not valid Unicode"),
        None => Ok(file_value(file, key)),
    }
}

pub(super) fn resolve(
    file: &Ini,
    capacity: u32,
    environment: impl Fn(&str) -> Option<OsString>,
) -> Result<LoginInputs, &'static str> {
    let character = selected(file, &environment, "RO_CHAR", "character")?
        .ok_or("RO_CHAR is missing; set it or [login].character in autoLogin.ini")?
        .parse::<u8>().map_err(|_| "RO_CHAR must be a zero-based character slot between 0 and 255 (or set [login].character)")?;
    let user = selected(file, &environment, "RO_USERNAME", "username")?
        .filter(|value| !value.is_empty())
        .and_then(|value| CString::new(value).ok())
        .ok_or("RO_USERNAME or [login].username is missing or invalid")?;
    let password = selected(file, &environment, "RO_PASSWORD", "password")?
        .filter(|value| !value.is_empty())
        .and_then(|value| CString::new(value).ok())
        .ok_or("RO_PASSWORD or [login].password is missing or invalid")?;
    if !valid_credentials(user.as_bytes(), password.as_bytes()) {
        return Err("credentials do not fit the native login fields");
    }
    if user.to_bytes_with_nul().len() > capacity as usize {
        return Err("username exceeds the native account-name field");
    }
    let server = selected(file, &environment, "RO_SERVER", "server")?
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value < 100)
        .ok_or("RO_SERVER or [login].server must be a zero-based index below 100")?;
    let pin = selected(file, &environment, "RO_PIN", "pin")?.filter(|value| !value.is_empty());
    Ok(LoginInputs {
        user,
        password,
        server,
        character,
        pin,
    })
}
