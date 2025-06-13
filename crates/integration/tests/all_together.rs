//! Tests the `release-util` crate which combines the functionality that is being checked in the
//! other integration tests here.

use integration::{CargoWorkspaceModel, ChangelogConfig, CrateModel, TestHarness};

#[test]
fn release_a_library() {
    let harness = TestHarness::new("all-together");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.add_private_registry_cargo_toml();
    harness.write_file_content("README.md", "# all together library");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let library = CrateModel::new("all-together-lib", "0.0.1")
        .make_lib()
        .with_description("All together library")
        .with_repository(&harness.repository_url())
        .with_license("Apache-2.0");

    harness.add_crate(library);
    harness.verify_cargo_project(".");
    harness.commit("*", "chore: Add library");
    harness.push_branch("main");

    //
    // Prepare an initial release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, Some("v0.1.0".to_string()));

    //
    // Trust that the tool did its job for now and commit + push the changes
    //
    harness.commit("*", "chore: Prepare v0.1.0 release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.1.0\""),
        "Expected version 0.1.0 in Cargo.toml"
    );

    //
    // Publish the release
    //
    harness.run_publish_release();

    //
    // Try running the publish operation again, to see how it behaves. Want idempotent and no
    // errors here, though there may be in the logs.
    //
    harness.run_publish_release();

    //
    // Make a simple change to the library
    //
    harness.write_file_content("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    harness.verify_cargo_project(".");
    harness.commit("src/lib.rs", "chore: Add add function");
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, None);
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.1.1\""),
        "Expected version 0.1.1 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();

    //
    // Make an incompatible change to the library
    //
    harness.write_file_content(
        "src/lib.rs",
        "pub fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }",
    );
    harness.verify_cargo_project(".");
    harness.commit(
        "src/lib.rs",
        "chore: Update add function with three parameters",
    );
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(
        ChangelogConfig::Pre1Point0Cliff,
        Some("v0.2.0-dev.0".to_string()),
    );
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.2.0-dev.0\""),
        "Expected version 0.2.0-dev.0 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();

    //
    // Make another feature change to the library
    //
    harness.write_file_content("src/lib.rs", "pub fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }");
    harness.verify_cargo_project(".");
    harness.commit("src/lib.rs", "feat: Add subtract function");
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, None);
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.2.0-dev.1\""),
        "Expected version 0.2.0 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();

    //
    // Add documentation to the library
    //
    harness.write_file_content(
        "src/lib.rs",
        r#"/// Adds two numbers.
pub fn add(a: i32, b: i32, c: i32) -> i32 {
    a + b + c
}
/// Subtracts two numbers.
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}"#,
    );
    harness.verify_cargo_project(".");
    harness.commit(
        "src/lib.rs",
        "docs: Add documentation to add and subtract functions",
    );
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, Some("v0.2.0".to_string()));
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.2.0\""),
        "Expected version 0.2.0 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();
}

#[test]
fn release_a_workspace() {
    let harness = TestHarness::new("all-together-workspace");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.add_private_registry_cargo_toml();
    harness.write_file_content("README.md", "# all together workspace");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let library = CrateModel::new("all-together-ws-lib", "0.0.1")
        .make_lib()
        .with_description("All together ws library")
        .with_repository(&harness.repository_url())
        .with_license("Apache-2.0");

    let binary = CrateModel::new("all-together-ws-bin", "0.0.1")
        .with_description("All together ws binary")
        .with_repository(&harness.repository_url())
        .with_license("Apache-2.0");

    harness.add_workspace(
        CargoWorkspaceModel::default()
            .add_crate(library, &[])
            .add_crate(binary, &["all-together-ws-lib"]),
    );
    harness.verify_cargo_project(".");
    harness.commit("*", "chore: Add workspace");
    harness.push_branch("main");

    //
    // Prepare an initial release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, Some("v0.1.0".to_string()));

    //
    // Trust that the tool did its job for now and commit + push the changes
    //
    harness.commit("*", "chore: Prepare v0.1.0 release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.1.0\""),
        "Expected version 0.1.0 in Cargo.toml"
    );

    //
    // Publish the release
    //
    harness.run_publish_release();

    //
    // Try running the publish operation again, to see how it behaves. Want idempotent and no
    // errors here, though there may be in the logs.
    //
    harness.run_publish_release();

    //
    // Make a simple change to the library
    //
    harness.write_file_content(
        "crates/all-together-ws-lib/src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
    );
    harness.verify_cargo_project(".");
    harness.commit(
        "crates/all-together-ws-lib/src/lib.rs",
        "chore: Add add function",
    );
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, None);
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.1.1\""),
        "Expected version 0.1.1 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();

    //
    // Make an incompatible change to the library
    //
    harness.write_file_content(
        "crates/all-together-ws-lib/src/lib.rs",
        "pub fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }",
    );
    harness.verify_cargo_project(".");
    harness.commit(
        "crates/all-together-ws-lib/src/lib.rs",
        "chore: Update add function with three parameters",
    );
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(
        ChangelogConfig::Pre1Point0Cliff,
        Some("v0.2.0-dev.0".to_string()),
    );
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.2.0-dev.0\""),
        "Expected version 0.2.0-dev.0 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();

    //
    // Make another feature change to the library
    //
    harness.write_file_content("crates/all-together-ws-lib/src/lib.rs", "pub fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }");
    harness.verify_cargo_project(".");
    harness.commit(
        "crates/all-together-ws-lib/src/lib.rs",
        "feat: Add subtract function",
    );
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, None);
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.2.0-dev.1\""),
        "Expected version 0.2.0 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();

    //
    // Add documentation to the library
    //
    harness.write_file_content(
        "crates/all-together-ws-lib/src/lib.rs",
        r#"/// Adds two numbers.
pub fn add(a: i32, b: i32, c: i32) -> i32 {
    a + b + c
}
/// Subtracts two numbers.
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}"#,
    );
    harness.verify_cargo_project(".");
    harness.commit(
        "crates/all-together-ws-lib/src/lib.rs",
        "docs: Add documentation to add and subtract functions",
    );
    harness.push_branch("main");

    //
    // Prepare a new release
    //
    harness.run_prepare_release(ChangelogConfig::Pre1Point0Cliff, Some("v0.2.0".to_string()));
    harness.commit("*", "chore: Prepare next release");
    harness.push_branch("main");

    //
    // Now check that the expected version is set
    //
    let toml_content = harness.read_file_content("Cargo.toml");
    assert!(
        toml_content.contains("version = \"0.2.0\""),
        "Expected version 0.2.0 in Cargo.toml"
    );

    //
    // Publish the new release
    //
    harness.run_publish_release();
}
