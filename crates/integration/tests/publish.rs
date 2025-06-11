use integration::{CargoWorkspaceModel, CrateModel, TestHarness};

/// Publish a simple library crate.
///
/// With this test, we get:
/// - All the test harness logic is correctly set up for publishing to a private registry.
/// - Cargo workspaces correctly publishes a library crate.
#[test]
fn publish_simple_library() {
    let harness = TestHarness::new("publish-library");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.add_private_registry_cargo_toml();
    harness.write_file_content("README.md", "# publish library");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let new_crate = CrateModel::new("test_publish", "0.1.0")
        .make_lib()
        .with_description("A test published crate")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");
    
    harness.add_crate(new_crate);
    harness.verify_cargo_project(".");
    harness.commit("*", "chore: Add crate");
    harness.push_branch("main");
    
    harness.publish();
    harness.check_index_clean();
}

/// Publish a workspace with internal dependencies.
///
/// With this test, we get:
/// - Cargo workspaces correctly discovers dependencies between crates and publishes them in the 
///   right order.
#[test]
fn publish_workspace() {
    let harness = TestHarness::new("publish-workspace");

    //
    // Initialize the repository
    //
    harness.add_standard_gitignore();
    harness.add_private_registry_cargo_toml();
    harness.write_file_content("README.md", "# publish workspace");
    harness.commit("README.md", "chore: Add README");
    harness.push_branch("main");

    //
    // Add Rust source code
    //
    let lib1 = CrateModel::new("test_publish_lib_1", "0.1.0")
        .make_lib()
        .with_description("Test published library 1")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");
    
    let lib2 = CrateModel::new("test_publish_lib_2", "0.1.0")
        .make_lib()
        .with_description("Test published library 2")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");
    
    let bin = CrateModel::new("test_publish_bin", "0.1.0")
        .with_description("Test published binary")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");

    let workspace = CargoWorkspaceModel::default()
        .add_crate(lib1, &[])
        .add_crate(lib2, &["test_publish_lib_1"])
        .add_crate(bin, &["test_publish_lib_1", "test_publish_lib_2"]);
    
    harness.add_workspace(workspace);
    harness.verify_cargo_project(".");
    harness.commit("*", "chore: Add workspace");
    harness.push_branch("main");

    harness.publish();
    harness.check_index_clean();
}

