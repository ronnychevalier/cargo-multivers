//! CLI tests

#[test]
fn trycmd() {
    trycmd::TestCases::new().case("tests/cli/*.toml");
}
