use integration::{CargoWorkspaceModel, ChangelogConfig, CrateModel, TestHarness};

/// How the content of a commit body is rendered into the changelog.
///
/// With this test, we get:
/// - Lists in a commit body keep their structure instead of being flattened onto one line.
/// - Hard-wrapped lines are joined back together without leaving a run of spaces behind.
/// - List items are rendered as the author wrote them, so identifiers are not capitalised.
/// - Repository mechanics, such as the `# Conflicts:` block left behind by a conflicted rebase,
///   are not published as release notes.
/// - Rebase and squash artifacts that ended up as commit subjects are left out entirely.
///
/// Unlike the other tests in this file, this one never pushes, because changelog generation is
/// purely local. That keeps it runnable without the Gitea and registry services.
#[test]
fn commit_body_rendering() {
    let harness = TestHarness::new("changelog-commit-body");

    harness.add_standard_gitignore();

    //
    // A body with a lead-in paragraph followed by a hard-wrapped list.
    //
    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: add the first thing

Some lead-in prose that is
hard wrapped over two lines:

- an item that is itself hard wrapped
  onto a second line
- rcgen 0.14: CertifiedKey::key_pair renamed to signing_key

A trailing paragraph after the list.",
    );

    //
    // A body that is nothing but a list, with no lead-in to nest under.
    //
    harness.write_file_content("b.txt", "b");
    harness.commit(
        "*",
        "fix: correct the second thing

- alpha item
- beta item",
    );

    //
    // A body carrying the artifacts of a conflicted rebase, plus a line that merely opens with
    // an issue reference and must be kept.
    //
    harness.write_file_content("c.txt", "c");
    harness.commit(
        "*",
        "chore: tidy the third thing

Real body text.

#123 was the original report

# Conflicts:
#\tCargo.lock
#\tCargo.toml",
    );

    //
    // Subjects that are rebase and squash artifacts rather than descriptions of a change.
    //
    harness.write_file_content("d.txt", "d");
    harness.commit("*", "# This is a combination of 2 commits.");
    harness.write_file_content("e.txt", "e");
    harness.commit("*", "--fixup=31047affea8826bb03b4c4e2161da6c26241661d");
    harness.write_file_content("f.txt", "f");
    harness.commit("*", "fixup! feat: add the first thing");

    //
    // Generate the changelog
    //
    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    //
    // The list keeps its structure, nested under the lead-in it belongs to, and the paragraph
    // that follows the list returns to the outer level.
    //
    assert!(
        changelog.contains(
            "  - Some lead-in prose that is hard wrapped over two lines:
    - an item that is itself hard wrapped onto a second line
    - rcgen 0.14: CertifiedKey::key_pair renamed to signing_key
  - A trailing paragraph after the list."
        ),
        "Lead-in list was not rendered as a nested list. Changelog is:\n{changelog}"
    );

    //
    // A body that is only a list has nothing to nest under, so it stays at the outer level.
    //
    assert!(
        changelog.contains(
            "  - alpha item
  - beta item"
        ),
        "List without a lead-in was not rendered at the outer level. Changelog is:\n{changelog}"
    );

    //
    // Joining a hard-wrapped line must not leave the original indentation behind as extra spaces.
    //
    // Checked past the leading indentation, which is meaningful, so this only looks at content.
    let run_of_spaces = changelog
        .lines()
        .find(|line| line.trim_start().contains("  "));
    assert!(
        run_of_spaces.is_none(),
        "A hard-wrapped line was joined without dropping its indentation, leaving a run of spaces \
         in {run_of_spaces:?}. Changelog is:\n{changelog}"
    );

    //
    // Repository mechanics are not release notes, but an issue reference is.
    //
    assert!(
        !changelog.contains("Conflicts:"),
        "Merge conflict metadata was published. Changelog is:\n{changelog}"
    );
    assert!(
        !changelog.contains("Cargo.lock"),
        "Merge conflict file list was published. Changelog is:\n{changelog}"
    );
    assert!(
        changelog.contains("- #123 was the original report"),
        "A body line opening with an issue reference was mangled. Changelog is:\n{changelog}"
    );

    //
    // Artifact subjects describe no change, so they are dropped rather than grouped.
    //
    assert!(
        !changelog.contains("This is a combination"),
        "A squash artifact was published. Changelog is:\n{changelog}"
    );
    assert!(
        !changelog.contains("--fixup="),
        "A fixup artifact was published. Changelog is:\n{changelog}"
    );
    assert!(
        !changelog.contains("fixup!"),
        "A fixup commit was published. Changelog is:\n{changelog}"
    );
    assert!(
        !changelog.contains("Other Changes"),
        "Artifacts were grouped as Other Changes. Changelog is:\n{changelog}"
    );
}

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
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
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
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("## \\[[0.1.1]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
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
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("## \\[[0.1.1]"));
    assert!(changelog.contains("## \\[[0.1.2]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
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
        .add_crate(lib_crate, &[])
        .add_crate(bin_crate, &[]);

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
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
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
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("## \\[[0.1.1]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
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
/// - It's possible to switch to a pre-release version in a workspace.
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
        .add_crate(lib_crate, &[])
        .add_crate(bin_crate, &[]);

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
    assert!(changelog.contains("## \\[[0.6.0]"));
    assert!(changelog.contains("## \\[[0.6.1]"));
    assert!(changelog.contains("## \\[[0.7.0-dev.0]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
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
    assert!(changelog.contains("## \\[[0.6.0]"));
    assert!(changelog.contains("## \\[[0.6.1]"));
    assert!(changelog.contains("## \\[[0.7.0-dev.0]"));
    assert!(changelog.contains("## \\[[0.7.0-dev.1]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
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
    assert!(changelog.contains("## \\[[0.6.0]"));
    assert!(changelog.contains("## \\[[0.6.1]"));
    assert!(changelog.contains("## \\[[0.7.0-dev.0]"));
    assert!(changelog.contains("## \\[[0.7.0-dev.1]"));
    assert!(changelog.contains("## \\[[0.7.0]"));
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
    assert!(changelog.contains("## \\[[0.6.0]"));
    assert!(changelog.contains("## \\[[0.6.1]"));
    assert!(changelog.contains("## \\[[0.7.0-dev.0]"));
    assert!(changelog.contains("## \\[[0.7.0-dev.1]"));
    assert!(changelog.contains("## \\[[0.7.0]"));
    assert!(changelog.contains("## \\[[0.7.1]"));
    assert!(changelog.contains("Add new feature to test_bin"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.7.1");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);
}

/// A library crate with versions across multiple branches to check that the versioning continues
/// to behave as expected.
///
/// With this test, we get:
/// - That we can generate changelogs on release branches with correct versioning.
#[test]
fn version_across_release_branches() {
    let harness = TestHarness::new("changelog-release-branches");

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
    let lib_crate = CrateModel::new("test_lib", "0.1.0")
        .make_lib()
        .with_description("A test lib crate")
        .with_repository(harness.repository_url().as_str())
        .with_license("Apache-2.0");

    harness.add_crate(lib_crate);
    harness.verify_cargo_project("");
    harness.commit("*", "chore: Add crate");
    harness.push_branch("main");

    //
    // Generate the initial changelog
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.0");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(
        changelog.contains("## \\[[0.1.0]"),
        "Changelog actual content is:\n{changelog}"
    );
    assert!(changelog.contains("### Miscellaneous Tasks"));
    assert!(changelog.contains("Add crate"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.1.0");
    harness.tag(version.as_str(), version.as_str());
    harness.push_branch("main");
    harness.push_tag(version.as_str());

    //
    // Make a code change to the library
    //
    harness.write_file_content(
        "src/lib.rs",
        r#"pub fn add(a: i32, b: i32) -> i32 { a + b }"#,
    );
    harness.verify_cargo_project("");
    harness.commit("src/lib.rs", "chore: Add add function");
    harness.push_branch("main");

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.1");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("## \\[[0.1.1]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
    assert!(changelog.contains("Add add function"));

    //
    // Push a tag for the new version
    //
    harness.commit("*", "docs: Update changelog for v0.1.1");
    harness.tag(version.as_str(), version.as_str());
    harness.push_branch("main");
    harness.push_tag(version.as_str());

    //
    // Now create a release branch
    //
    harness.switch_branch("release/0.1.x");

    //
    // Switch back to main and make a change
    //
    harness.switch_branch("main");
    harness.write_file_content(
        "src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn subtract(a: i32, b: i32) -> i32 { a - b }",
    );
    harness.verify_cargo_project("");
    harness.commit("src/lib.rs", "feat: Add subtract function");
    harness.push_branch("main");

    //
    // Create a pre-release version
    //
    let version = harness.generate_changelog(
        ChangelogConfig::Pre1Point0Cliff,
        Some("v0.2.0-dev.0".to_string()),
    );
    assert_eq!(version, "v0.2.0-dev.0");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("## \\[[0.1.1]"));
    assert!(changelog.contains("## \\[[0.2.0-dev.0]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
    assert!(changelog.contains("Add subtract function"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.2.0-dev.0");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);

    //
    // Make a change to the library crate
    //
    harness.write_file_content(
        "src/lib.rs",
        "/// It adds numbers\npub fn add(a: i32, b: i32) -> i32 { a + b }",
    );
    harness.verify_cargo_project("");
    harness.commit("src/lib.rs", "docs: Add documentation to add function");
    harness.push_branch("main");

    //
    // Release the pre-release version as a stable version
    //
    let version =
        harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, Some("v0.2.0".to_string()));
    assert_eq!(version, "v0.2.0");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("## \\[[0.1.1]"));
    assert!(changelog.contains("## \\[[0.2.0-dev.0]"));
    assert!(changelog.contains("## \\[[0.2.0]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
    assert!(changelog.contains("Add subtract function"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.2.0");
    harness.tag(&version, &version);
    harness.push_branch("main");
    harness.push_tag(&version);

    //
    // Now switch back to the release branch and make a change
    //
    harness.switch_branch("release/0.1.x");
    harness.write_file_content(
        "src/lib.rs",
        "/// It adds numbers\npub fn add(a: i32, b: i32) -> i32 { a + b }",
    );
    harness.verify_cargo_project("");
    harness.commit("src/lib.rs", "docs: Add documentation to add function");
    harness.push_branch("release/0.1.x");

    //
    // Generate the changelog for the new version
    //
    let version = harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    assert_eq!(version, "v0.1.2");

    //
    // Check the changelog content
    //
    let changelog = harness.read_file_content("CHANGELOG.md");
    assert!(changelog.contains("## \\[[0.1.0]"));
    assert!(changelog.contains("## \\[[0.1.1]"));
    assert!(changelog.contains("## \\[[0.1.2]"));
    assert!(changelog.contains("### Miscellaneous Tasks"));
    assert!(changelog.contains("Add documentation to add function"));

    //
    // Push a tag for the version
    //
    harness.commit("*", "docs: Update changelog for v0.1.2");
    harness.tag(&version, &version);
    harness.push_branch("release/0.1.x");
    harness.push_tag(&version);
}
