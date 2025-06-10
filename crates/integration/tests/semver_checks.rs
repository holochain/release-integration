use integration::{CargoWorkspaceModel, ChangelogConfig, CrateModel, TestHarness};

/// Run semver checks against a simple library crate.
///
/// With this test, we get:
/// - Semver checks catch breaking API changes in a library.
/// - Breaking API changes are permitted when the semver version is incremented appropriately.
#[test]
#[ignore]
fn check_semver_simple_library() {
    let harness = TestHarness::new("check-semver-library");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.write_file_content("README.md", "# check semver library");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let new_crate = CrateModel::new("test_sem", "0.1.0")
        .make_lib()
        .with_description("A test semver crate")
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

    //
    // Set the crate version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();
    harness.push_branch("main");
    harness.push_tag("v0.1.0");

    //
    // Make a simple change to the library
    //
    harness.write_file_content("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    harness.commit("src/lib.rs", "chore: Add add function");
    harness.push_branch("main");

    //
    // Discover the baseline revision
    //
    let current_version = harness.get_current_version_from_workspace_cargo_toml();
    let revision = harness.get_revision_for_tag(&format!("v{}", current_version));

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.1");

    //
    // Set the new version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();

    //
    // Run semver checks
    //
    let passed = harness.run_semver_checks(&revision);
    assert!(passed, "Semver checks should have for v0.1.1");

    //
    // Push the new version
    //
    harness.push_branch("main");
    harness.push_tag("v0.1.1");

    //
    // Make a breaking change to the library
    //
    harness.write_file_content(
        "src/lib.rs",
        "pub fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }",
    );
    harness.commit("src/lib.rs", "feat: Add function now adds 3 numbers");
    harness.push_branch("main");

    //
    // Discover the baseline revision
    //
    let current_version = harness.get_current_version_from_workspace_cargo_toml();
    let revision = harness.get_revision_for_tag(&format!("v{}", current_version));

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.2");
    harness.commit("CHANGELOG.md", "docs: Update changelog for v0.1.2");
    harness.push_branch("main");

    //
    // Set the new version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();
    harness.push_tag("v0.1.2");

    //
    // Run semver checks
    //
    let passed = harness.run_semver_checks(&revision);
    assert!(
        !passed,
        "Semver checks should have failed for v0.1.2 due to breaking change"
    );

    //
    // Make further changes
    //
    harness.write_file_content(
        "src/lib.rs",
        "/// An add function\npub fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }",
    );
    harness.commit("src/lib.rs", "docs: Describe the add function");
    harness.push_branch("main");

    //
    // Force a version bump to an appropriate version to permit the breaking change
    //
    let version =
        harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, Some("v0.2.0".to_string()));
    assert_eq!(version, "v0.2.0");

    //
    // Set the new version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();

    //
    // Run semver checks
    //
    let passed = harness.run_semver_checks(&revision);
    assert!(
        passed,
        "Semver checks should have passed for v0.2.0 after forcing a version bump"
    );

    //
    // Push the new version
    //
    harness.push_branch("main");
    harness.push_tag("v0.2.0");
}

/// Run semver checks against a workspace with multiple crates.
///
/// With this test, we get:
/// - Semver checks catch breaking API changes in a workspace.
#[test]
fn check_semver_workspace() {
    let harness = TestHarness::new("check-semver-workspace");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.write_file_content("README.md", "# check semver workspace");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let lib_crate = CrateModel::new("test_lib_sem", "0.1.0")
        .make_lib()
        .with_description("A test versioned lib crate")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");

    let bin_crate = CrateModel::new("test_bin_sem", "")
        .with_description("A test versioned bin crate")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");

    let workspace = CargoWorkspaceModel::default()
        .add_crate(lib_crate)
        .add_crate(bin_crate);

    harness.add_workspace(workspace);
    harness.verify_cargo_project("crates/test_lib_sem");
    harness.verify_cargo_project("crates/test_bin_sem");
    harness.commit("*", "chore: Add workspace");
    harness.push_branch("main");

    //
    // Generate the initial changelog
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.0");
    harness.commit("CHANGELOG.md", "chore: Update changelog for v0.1.0");
    harness.push_branch("main");

    //
    // Set the workspace version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();
    harness.push_branch("main");
    harness.push_tag("v0.1.0");

    //
    // Update the library and binary crates
    //
    harness.write_file_content(
        "crates/test_lib_sem/src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
    );
    harness.commit(
        "crates/test_lib_sem/",
        "chore: Add add function to test_lib_sem",
    );

    harness.write_file_content(
        "crates/test_bin_sem/src/main.rs",
        r#"fn main() { println!("Hello from test_bin!"); }"#,
    );
    harness.commit(
        "crates/test_bin_sem/",
        "chore: Update main function in test_bin_sem",
    );

    harness.verify_cargo_project("");
    harness.push_branch("main");

    //
    // Discover the baseline revision
    //
    let current_version = harness.get_current_version_from_workspace_cargo_toml();
    let revision = harness.get_revision_for_tag(&format!("v{}", current_version));

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.1");
    harness.commit("CHANGELOG.md", "chore: Update changelog for v0.1.1");

    //
    // Set the new version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();

    //
    // Run semver checks
    //
    let passed = harness.run_semver_checks(&revision);
    assert!(passed, "Semver checks should have passed for v0.1.1");

    //
    // Push the new version
    //
    harness.push_branch("main");
    harness.push_tag("v0.1.1");

    //
    // Make an incompatible change to the library crate
    //
    harness.write_file_content(
        "crates/test_lib_sem/src/lib.rs",
        "pub fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }",
    );
    harness.commit(
        "crates/test_lib_sem/src/lib.rs",
        "feat: Add function now adds 3 numbers",
    );
    harness.push_branch("main");

    //
    // Discover the baseline revision
    //
    let current_version = harness.get_current_version_from_workspace_cargo_toml();
    let revision = harness.get_revision_for_tag(&format!("v{}", current_version));

    //
    // Generate changelog, switching to pre-release for the next version
    //
    let version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        Some("v0.2.0-dev.0".to_string()),
    );
    assert_eq!(version, "v0.2.0-dev.0");
    harness.commit("CHANGELOG.md", "chore: Update changelog for v0.2.0-dev.0");
    harness.push_branch("main");

    //
    // Set the new version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();
    harness.verify_cargo_project("");

    //
    // Run semver checks
    //
    let passed = harness.run_semver_checks(&revision);
    assert!(passed, "Semver checks should have passed for v0.1.1");

    //
    // Push the new version
    //
    harness.push_branch("main");
    harness.push_tag("v0.2.0-dev.0");

    //
    // Make an incompatible change to the library crate, which should be permitted on pre-release
    // versions.
    //
    harness.write_file_content(
        "crates/test_lib_sem/src/lib.rs",
        "pub fn add2(a: i32, b: i32, c: i32) -> i32 { a + b + c }",
    );
    harness.commit("crates/test_lib_sem/src/lib.rs", "feat: Rename add to add2");
    harness.push_branch("main");

    //
    // Discover the baseline revision
    //
    let current_version = harness.get_current_version_from_workspace_cargo_toml();
    let revision = harness.get_revision_for_tag(&format!("v{}", current_version));

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.2.0-dev.1");
    harness.commit("CHANGELOG.md", "chore: Update changelog for v0.2.0-dev.1");
    harness.push_branch("main");

    //
    // Set the new version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();

    //
    // Run semver checks
    //
    let passed = harness.run_semver_checks(&revision);
    assert!(passed, "Semver checks should have passed for v0.2.0-dev.1");

    //
    // Update the library crate with documentation
    //
    harness.write_file_content(
        "crates/test_lib_sem/src/lib.rs",
        "/// Add three numbers\npub fn add2(a: i32, b: i32, c: i32) -> i32 { a + b + c }",
    );
    harness.commit(
        "crates/test_lib_sem/src/lib.rs",
        "docs: Add documentation to add2 function in test_lib_sem",
    );
    harness.push_branch("main");

    //
    // Discover the baseline revision
    //
    let current_version = harness.get_current_version_from_git_cliff(
        ChangelogConfig::Pre1Point0Cliff,
        Some("v0.2.0".to_string()),
    );
    println!("Current version: {}", current_version);
    let revision = harness.get_revision_for_tag(&current_version);
    println!("Revision: {}", revision);

    //
    // Switch to a release version
    //
    let version =
        harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, Some("v0.2.0".to_string()));
    assert_eq!(version, "v0.2.0");

    //
    // Set the new version
    //
    harness.set_version(&version, false);
    harness.check_index_clean();

    //
    // Run semver checks
    //
    let passed = harness.run_semver_checks(&revision);
    assert!(
        passed,
        "Semver checks should have passed for v0.2.0 after switching to a release version"
    );

    //
    // Push the new version
    //
    harness.push_branch("main");
    harness.push_tag("v0.2.0");
}
