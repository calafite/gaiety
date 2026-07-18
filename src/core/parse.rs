pub(crate) fn parse_version(string: &str) -> Result<semver::Version, semver::Error> {
    if let Ok(value) = semver::Version::parse(string) {
        return Ok(value);
    }

    let (base, remainder) = match string.find(['-', '+']) {
        Some(index) => string.split_at(index),
        None => (string, ""),
    };

    let padded_base = match base.split('.').count() {
        1 => format!("{}.0.0", base),
        2 => format!("{}.0", base),
        _ => base.to_string(),
    };

    semver::Version::parse(&format!("{}{}", padded_base, remainder))
}

#[test]
fn test_parse_version() {
    assert_eq!(
        parse_version("1.2.3").unwrap(),
        semver::Version::new(1, 2, 3)
    );
    assert_eq!(parse_version("1.2").unwrap(), semver::Version::new(1, 2, 0));
    assert_eq!(parse_version("1").unwrap(), semver::Version::new(1, 0, 0));
    assert_eq!(
        parse_version("1.2-alpha").unwrap(),
        semver::Version::parse("1.2.0-alpha").unwrap()
    );
}
