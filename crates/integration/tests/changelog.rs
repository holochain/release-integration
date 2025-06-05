use integration::{BumpType, CargoWorkspaceModel, ChangelogConfig, CrateModel, TestHarness};

/// A really simple library crate to check that changelog generation behaves as expected.
///
/// With this test, we get:
/// - Semver is respected for `chore:` and `feat:` commits when the crate is using a 0.x.y version.
/// - The versions in the Cargo.toml are ignored, it's only the tags that matter.
#[test]
fn simple_library_changelog() {
    let harness = TestHarness::new("changelog-library");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.write_file_content("README.md", "# library");
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
    harness.verify_cargo_project(".");
    harness.commit("*", "chore: Add crate");
    harness.push_branch("main");

    //
    // Generate the initial changelog
    //
    let version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        BumpType::default(),
        None,
        "",
    );
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
    let version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        BumpType::default(),
        None,
        "",
    );
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
    let version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        BumpType::default(),
        None,
        "",
    );
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
}

/// A simple workspace with one library and one binary crate to check that changelog generation
/// behaves as expected.
///
/// With this test, we get:
/// - Changelogs can be generated in a monorepo with multiple crates.
#[test]
fn simple_workspace_changelog() {
    let harness = TestHarness::new("changelog-workspace");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.write_file_content("README.md", "# simple workspace");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let lib_crate = CrateModel::new("test_lib", "0.1.0")
        .make_lib()
        .with_description("A test lib crate")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");

    let bin_crate = CrateModel::new("test_bin", "")
        .with_description("A test bin crate")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");

    let workspace = CargoWorkspaceModel::default()
        .add_crate(lib_crate)
        .add_crate(bin_crate);

    harness.add_workspace(workspace);
    harness.verify_cargo_project("crates/test_lib");
    harness.verify_cargo_project("crates/test_bin");
    harness.commit("*", "chore: Add workspace");
    harness.push_branch("main");

    //
    // Update the library crate
    //
    harness.write_file_content(
        "crates/test_lib/src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
    );
    harness.commit("crates/test_lib/", "chore: Add add function to test_lib");
    harness.push_branch("main");

    //
    // Generate changelogs for both crates
    //
    let lib_version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        BumpType::default(),
        None,
        "crates/test_lib",
    );
    assert_eq!(lib_version, "v0.1.0");
    let bin_version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        BumpType::default(),
        None,
        "crates/test_bin",
    );
    assert_eq!(bin_version, "v0.1.0");

    //
    // Check the changelog content
    //
    let lib_changelog = harness.read_file_content("crates/test_lib/CHANGELOG.md");
    assert!(lib_changelog.contains("## [0.1.0]"));
    assert!(lib_changelog.contains("### Changed"));
    assert!(lib_changelog.contains("Add workspace"));
    assert!(lib_changelog.contains("Add add function to test_lib"));

    let bin_changelog = harness.read_file_content("crates/test_bin/CHANGELOG.md");
    assert!(bin_changelog.contains("## [0.1.0]"));
    assert!(bin_changelog.contains("### Changed"));
    assert!(bin_changelog.contains("Add workspace"));
    assert!(!bin_changelog.contains("Add add function to test_lib"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelogs for v0.1.0");
    harness.tag(&lib_version, &lib_version);
    harness.push_branch("main");
    harness.push_tag(&lib_version);

    //
    // Make a change to the library and binary crates
    //
    harness.write_file_content(
        "crates/test_lib/src/lib.rs",
        r#"pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn subtract(a: i32, b: i32) -> i32 { a - b }
"#,
    );
    harness.verify_cargo_project("");
    harness.commit(
        "crates/test_lib/",
        "feat: Add subtract function to test_lib",
    );

    harness.write_file_content(
        "crates/test_bin/src/main.rs",
        r#"fn main() { println!("Hello from test_bin!"); }"#,
    );
    harness.verify_cargo_project("");

    harness.commit("crates/test_bin/", "feat: Update main function in test_bin");
    harness.push_branch("main");

    //
    // Generate changelogs for both crates again
    //
    let lib_version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        BumpType::default(),
        None,
        "crates/test_lib",
    );
    assert_eq!(lib_version, "v0.1.1");

    let bin_version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        BumpType::default(),
        None,
        "crates/test_bin",
    );
    assert_eq!(bin_version, "v0.1.1");

    //
    // Check the changelog content for both crates
    //
    let lib_changelog = harness.read_file_content("crates/test_lib/CHANGELOG.md");
    println!("lib_changelog:\n{}", lib_changelog);
    assert!(lib_changelog.contains("## [0.1.0]"));
    assert!(lib_changelog.contains("## [0.1.1]"));
    assert!(lib_changelog.contains("### Changed"));
    assert!(lib_changelog.contains("Add subtract function to test_lib"));

    let bin_changelog = harness.read_file_content("crates/test_bin/CHANGELOG.md");
    assert!(bin_changelog.contains("## [0.1.0]"));
    assert!(bin_changelog.contains("## [0.1.1]"));
    assert!(bin_changelog.contains("### Changed"));
    assert!(bin_changelog.contains("Update main function in test_bin"));

    //
    // Push a tag for each version
    //
    harness.commit("*", "docs: Update changelogs for v0.1.1");
    harness.tag(&lib_version, &lib_version);
    harness.push_branch("main");
    harness.push_tag(&lib_version);
}
