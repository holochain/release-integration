use integration::{CargoWorkspaceModel, ChangelogConfig, CrateModel, TestHarness};

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
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
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
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
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
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
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
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.0");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.1.0]"));
    assert!(changelog.contains("### Changed"));
    assert!(changelog.contains("Add workspace"));
    assert!(changelog.contains("Add add function to test_lib"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelogs for v0.1.0");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);

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
    // Generate an updated changelog
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.1");

    //
    // Check the changelog content for both crates
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.1.0]"));
    assert!(changelog.contains("## [0.1.1]"));
    assert!(changelog.contains("### Changed"));
    assert!(changelog.contains("Add subtract function to test_lib"));

    //
    // Push a tag for each version
    //
    harness.commit("*", "docs: Update changelogs for v0.1.1");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);
}

/// A workspace that needs to produce pre-release versions.
///
/// With this test, we get:
/// - It's possible for switch to a pre-release version in a workspace.
/// - Can switch back to a release version after pre-release versions.
/// - Correctly gather changes, ignoring pre-release versions, to produce an aggregated change set.
#[test]
fn pre_release_from_workspace() {
    let harness = TestHarness::new("changelog-pre-release-workspace");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.write_file_content("README.md", "# pre-release workspace");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let lib_crate = CrateModel::new("test_lib", "0.6.1")
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
    // Create a version history
    //
    let version =
        harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, Some("v0.6.0".to_string()));
    assert_eq!(version, "v0.6.0");

    harness.commit("*", "docs: Update changelog for v0.6.0");
    harness.tag("v0.6.0", "v0.6.0");
    harness.push_branch("main");
    harness.push_tag("v0.6.0");

    harness.write_file_content("b.txt", "0.6.1 content");
    harness.commit("*", "chore: Add 0.6.1 content");

    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.6.1");

    harness.commit("*", "docs: Update changelog for v0.6.1");
    harness.tag("v0.6.1", "v0.6.1");
    harness.push_branch("main");
    harness.push_tag("v0.6.1");

    //
    // Update the library and binary crates
    //
    harness.write_file_content(
        "crates/test_lib/src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
    );
    harness.commit("crates/test_lib/", "chore: Add add function to test_lib");

    harness.write_file_content(
        "crates/test_bin/src/main.rs",
        r#"fn main() { println!("Hello from test_bin!"); }"#,
    );
    harness.commit(
        "crates/test_bin/",
        "chore: Update main function in test_bin",
    );

    harness.push_branch("main");

    //
    // Generate changelog
    //
    let version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        Some("v0.7.0-dev.0".to_string()),
    );
    assert_eq!(version, "v0.7.0-dev.0");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.6.0]"));
    assert!(changelog.contains("## [0.6.1]"));
    assert!(changelog.contains("## [0.7.0-dev.0]"));
    assert!(changelog.contains("### Changed"));
    assert!(changelog.contains("Add workspace"));
    assert!(changelog.contains("Add add function to test_lib"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelogs for v0.7.0-dev.0");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);

    //
    // Make a change to just the library crate
    //
    harness.write_file_content(
        "crates/test_lib/src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn add2(a: i32, b: i32) -> i32 { a + b }",
    );
    harness.commit("crates/test_lib/", "chore: Add add2 function to test_lib");
    harness.push_branch("main");

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.7.0-dev.1");

    //
    // Check the changelog content for both crates
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.6.0]"));
    assert!(changelog.contains("## [0.6.1]"));
    assert!(changelog.contains("## [0.7.0-dev.0]"));
    assert!(changelog.contains("## [0.7.0-dev.1]"));
    assert!(changelog.contains("### Changed"));
    assert!(changelog.contains("Add add2 function to test_lib"));

    //
    // Push a tag
    //
    harness.commit("*", "docs: Update changelogs for v0.7.0-dev.1");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);

    //
    // Have to make a change if we want to switch to a release version
    //
    harness.write_file_content("a.txt", "0.7.0 content");
    harness.commit("*", "chore: Add 0.7.0 content");
    harness.push_branch("main");

    //
    // Switch to a release version
    //
    let version =
        harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, Some("v0.7.0".to_string()));
    assert_eq!(version, "v0.7.0");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.6.0]"));
    assert!(changelog.contains("## [0.6.1]"));
    assert!(changelog.contains("## [0.7.0-dev.0]"));
    assert!(changelog.contains("## [0.7.0-dev.1]"));
    assert!(changelog.contains("## [0.7.0]"));
    assert!(changelog.contains("Update main function in test_bin"));
    assert!(changelog.contains("Add add2 function to test_lib"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.7.0");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);

    //
    // Make further changes to the binary crate
    //
    harness.write_file_content(
        "crates/test_bin/src/main.rs",
        r#"fn main() { println!("Hello from test_bin!"); println!("New feature!"); }"#,
    );
    harness.commit("crates/test_bin/", "feat: Add new feature to test_bin");
    harness.push_branch("main");

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.7.1");

    //
    // Check the changelog content the new version
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## [0.6.0]"));
    assert!(changelog.contains("## [0.6.1]"));
    assert!(changelog.contains("## [0.7.0-dev.0]"));
    assert!(changelog.contains("## [0.7.0-dev.1]"));
    assert!(changelog.contains("## [0.7.0]"));
    assert!(changelog.contains("## [0.7.1]"));
    assert!(changelog.contains("Add new feature to test_bin"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.7.1");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);
}
