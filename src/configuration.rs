//! File parsing and per-field environment precedence, without native operations.
mod login;
use crate::recovery::RecoverySettings;
use configparser::ini::Ini;
pub(crate) use login::LoginInputs;
use std::ffi::OsString;

/// One startup snapshot. Deliberately has no Debug implementation: it owns secrets.
pub struct PluginConfiguration {
    dry_run: bool,
    tick_milliseconds: u32,
    pub(crate) recovery: RecoverySettings,
    login: Ini,
}

impl PluginConfiguration {
    pub fn parse(contents: &str) -> Result<Self, &'static str> {
        let contents = contents.trim_start_matches('\u{feff}');
        let dry_run = match setting(contents, "dry_run") {
            None | Some("false") => false,
            Some("true") => true,
            Some(_) => return Err("autoLogin.ini dry_run must be true or false"),
        };
        let tick_milliseconds = setting(contents, "tick_milliseconds")
            .and_then(|value| value.parse::<u32>().ok())
            .map_or(50, |value| value.clamp(10, 1000));
        let recovery = RecoverySettings::parse(contents)?;
        let mut login = Ini::new_cs();
        let mut defaults = login.defaults();
        defaults.delimiters = vec!['='];
        login.load_defaults(defaults);
        // Password punctuation is data. Only whole-line comments are recognized
        // for login values; existing automation settings keep their old grammar.
        login.set_inline_comment_symbols(Some(&[]));
        login
            .read(contents.to_owned())
            .map_err(|_| "autoLogin.ini has invalid INI syntax")?;
        Ok(Self {
            dry_run,
            tick_milliseconds,
            recovery,
            login,
        })
    }
    pub fn dry_run(&self) -> bool {
        self.dry_run
    }
    pub fn tick_milliseconds(&self) -> u32 {
        self.tick_milliseconds
    }
    pub(crate) fn override_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }
    pub(crate) fn resolve_login(
        &self,
        capacity: u32,
        environment: impl Fn(&str) -> Option<OsString>,
    ) -> Result<LoginInputs, &'static str> {
        login::resolve(&self.login, capacity, environment)
    }
}

fn setting<'a>(config: &'a str, name: &str) -> Option<&'a str> {
    config
        .lines()
        .filter_map(|line| line.split('#').next()?.split_once('='))
        .find_map(|(key, value)| (key.trim() == name).then_some(value.trim()))
}

#[cfg(test)]
mod tests;
