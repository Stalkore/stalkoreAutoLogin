use super::*;
const FILE: &str =
    "[login]\nusername=fixture-user\npassword=fixture-password\nserver=1\ncharacter=2\npin=0123\n";
fn inputs(
    config: &PluginConfiguration,
    entries: &[(&str, &str)],
) -> Result<LoginInputs, &'static str> {
    config.resolve_login(50, |name| {
        entries
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    })
}

#[test]
fn file_alone_supplies_all_login_fields_and_keeps_pin_zeroes() {
    let config = PluginConfiguration::parse(FILE).unwrap();
    let fields = inputs(&config, &[]).unwrap();
    assert_eq!(fields.user.as_bytes(), b"fixture-user");
    assert_eq!(fields.password.as_bytes(), b"fixture-password");
    assert_eq!((fields.server, fields.character), (1, 2));
    assert_eq!(fields.pin.as_deref(), Some("0123"));
}
#[test]
fn every_environment_field_overrides_only_its_own_file_value() {
    let config = PluginConfiguration::parse(FILE).unwrap();
    let fields = inputs(
        &config,
        &[("RO_CHAR", "0"), ("RO_PASSWORD", "override-fixture")],
    )
    .unwrap();
    assert_eq!(fields.user.as_bytes(), b"fixture-user");
    assert_eq!(fields.password.as_bytes(), b"override-fixture");
    assert_eq!((fields.server, fields.character), (1, 0));
    assert_eq!(fields.pin.as_deref(), Some("0123"));
    let fields = inputs(
        &config,
        &[
            ("RO_USERNAME", "override-user"),
            ("RO_SERVER", "0"),
            ("RO_PIN", "9876"),
        ],
    )
    .unwrap();
    assert_eq!(fields.user.as_bytes(), b"override-user");
    assert_eq!(fields.password.as_bytes(), b"fixture-password");
    assert_eq!((fields.server, fields.character), (0, 2));
    assert_eq!(fields.pin.as_deref(), Some("9876"));
}
#[test]
fn invalid_or_empty_required_overrides_do_not_fall_back_to_valid_file() {
    let config = PluginConfiguration::parse(FILE).unwrap();
    for entry in [
        ("RO_CHAR", ""),
        ("RO_CHAR", "256"),
        ("RO_SERVER", "100"),
        ("RO_USERNAME", ""),
        ("RO_PASSWORD", ""),
        ("RO_PASSWORD", "bad\0fixture"),
    ] {
        assert!(inputs(&config, &[entry]).is_err());
    }
    assert!(inputs(&config, &[("RO_PASSWORD", &"x".repeat(64))]).is_err());
}
#[test]
fn selected_environment_values_can_replace_invalid_file_values() {
    let config =
        PluginConfiguration::parse("[login]\nusername=\npassword=\nserver=bad\ncharacter=bad\n")
            .unwrap();
    assert!(inputs(
        &config,
        &[
            ("RO_USERNAME", "fixture-user"),
            ("RO_PASSWORD", "fixture-password"),
            ("RO_SERVER", "0"),
            ("RO_CHAR", "0")
        ]
    )
    .is_ok());
}
#[test]
fn optional_pin_can_be_absent_or_explicitly_disabled() {
    let config = PluginConfiguration::parse(FILE).unwrap();
    assert!(inputs(&config, &[("RO_PIN", "")]).unwrap().pin.is_none());
    assert!(inputs(
        &PluginConfiguration::parse(&FILE.replace("pin=0123", "pin=")).unwrap(),
        &[]
    )
    .unwrap()
    .pin
    .is_none());
}
#[test]
fn punctuation_quotes_spaces_and_backslashes_are_literal_login_data() {
    let config = PluginConfiguration::parse("# comment\n[login]\nusername=fixture-user\npassword=\" #;=:\\ fixture ' \\ \"\nserver=0\ncharacter=0\npin='0123'\n").unwrap();
    let fields = inputs(&config, &[]).unwrap();
    assert_eq!(fields.password.as_bytes(), b" #;=:\\ fixture ' \\ ");
    assert_eq!(fields.pin.as_deref(), Some("0123"));
    let config =
        PluginConfiguration::parse(&FILE.replace("fixture-password", "p#ss;word=with:punctuation"))
            .unwrap();
    assert_eq!(
        inputs(&config, &[]).unwrap().password.as_bytes(),
        b"p#ss;word=with:punctuation"
    );
}
#[test]
fn legacy_settings_keep_defaults_bounds_and_inline_comments() {
    let config = PluginConfiguration::parse("dry_run=true\ntick_milliseconds=100 # old syntax\nautologin_max_attempts=4\n[login]\nusername=fixture\n").unwrap();
    assert!(config.dry_run());
    assert_eq!(config.tick_milliseconds(), 100);
    assert_eq!(config.recovery.max_attempts, 4);
    let config = PluginConfiguration::parse("").unwrap();
    assert!(!config.dry_run());
    assert_eq!(config.tick_milliseconds(), 50);
    for (value, expected) in [("1", 10), ("5000", 1000), ("invalid", 50)] {
        assert_eq!(
            PluginConfiguration::parse(&format!("tick_milliseconds={value}"))
                .unwrap()
                .tick_milliseconds(),
            expected
        );
    }
}
#[test]
fn errors_identify_configuration_without_exposing_values() {
    assert_eq!(
        PluginConfiguration::parse("[private-fixture-value").err(),
        Some("autoLogin.ini has invalid INI syntax")
    );
    let config = PluginConfiguration::parse("").unwrap();
    assert!(inputs(&config, &[])
        .err()
        .unwrap()
        .contains("RO_CHAR is missing"));
    assert!(PluginConfiguration::parse("dry_run=invalid").is_err());
    assert!(PluginConfiguration::parse("autologin_max_attempts=0").is_err());
}
#[test]
fn account_capacity_includes_its_terminating_nul_and_credentials_remain_bounded() {
    let config = PluginConfiguration::parse(FILE).unwrap();
    assert!(config.resolve_login(12, |_| None).is_err());
    assert!(config.resolve_login(13, |_| None).is_ok());
    assert!(inputs(&config, &[("RO_CHAR", "255")]).is_ok());
}

#[test]
fn utf8_bom_does_not_hide_dry_run_or_the_first_recovery_setting() {
    let config = PluginConfiguration::parse(&format!("\u{feff}dry_run=true\n{FILE}")).unwrap();
    assert!(config.dry_run());
    assert!(inputs(&config, &[]).is_ok());
    let config = PluginConfiguration::parse("\u{feff}autologin_max_attempts=7").unwrap();
    assert_eq!(config.recovery.max_attempts, 7);
}
