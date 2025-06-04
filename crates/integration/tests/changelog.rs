use integration::{BumpType, ChangelogConfig, CrateModel, TestHarness};

/// A really simple library crate to check that changelog generation behaves as expected.
///
/// With this test, we get:
/// - Semver is respected for `chore:` and `feat:` commits when the crate is using a 0.x.y version.
#[test]
fn simple_library_changelog() {
    let harness = TestHarness::new("library");

    //
    // Initialize the repository
    //
    harness.write_file_content("README.md", "# hello world");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let new_crate = CrateModel::new("test", "0.1.0")
        .make_lib()
        .with_description("A test crate")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");

    harness.add_crate(new_crate);
    harness.commit("*", "chore: Add crate");
    harness.push_branch("main");

    //
    // Generate the initial changelog
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, BumpType::default());
    assert_eq!(version, "v0.1.0");

    // Check the changelog content
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.1.0]"));
    assert!(changelog.contains("### Changed"));
    assert!(changelog.contains("Add crate"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.1.0");
    harness.tag(version.as_str(), version.as_str());
    harness.push_branch("main");
    harness.push_tag(version.as_str());

    //
    // Make a simple change to the library
    //
    harness.write_file_content("src/lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");
    harness.commit("src/lib.rs", "chore: Add add function");
    harness.push_branch("main");

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, BumpType::default());
    assert_eq!(version, "v0.1.1");

    // Check the changelog content
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.1.0]"));
    assert!(changelog.contains("## [0.1.1]"));
    assert!(changelog.contains("### Changed"));
    assert!(changelog.contains("Add add function"));

    //
    // Push a tag for the new version
    //
    harness.commit("*", "docs: Update changelog for v0.1.1");
    harness.tag(version.as_str(), version.as_str());
    harness.push_branch("main");
    harness.push_tag(version.as_str());

    //
    // Make another change to the library and this time call it a feature
    //
    harness.write_file_content(
        "src/lib.rs",
        r#"fn add(a: i32, b: i32) -> i32 { a + b }
fn subtract(a: i32, b: i32) -> i32 { a - b }
"#,
    );
    harness.commit("src/lib.rs", "feat: Add subtract function");
    harness.push_branch("main");

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, BumpType::default());
    assert_eq!(version, "v0.1.2");

    // Check the changelog content
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.1.0]"));
    assert!(changelog.contains("## [0.1.1]"));
    assert!(changelog.contains("## [0.1.2]"));
    assert!(changelog.contains("### Changed"));
    assert!(changelog.contains("Add subtract function"));

    //
    // Push a tag for the new version
    //
    harness.commit("*", "docs: Update changelog for v0.1.2");
    harness.tag(version.as_str(), version.as_str());
    harness.push_branch("main");
    harness.push_tag(version.as_str());

    harness.retain();
}
