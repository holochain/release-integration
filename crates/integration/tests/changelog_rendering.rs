//! Changelog rendering tests.
//!
//! The tests in this module must be runnable locally and in CI so cannot rely on external services.

use integration::{ChangelogConfig, TestHarness};

#[test]
fn nested_list_depth_is_flattened_by_one_level() {
    let harness = TestHarness::new("changelog-nested-list");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: add a nested list

Lead in:

- parent item
  - child item
- second parent",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "  - Lead in:
    - parent item
    - child item
    - second parent"
        ),
        "Nested list rendering changed. Changelog is:\n{changelog}"
    );
}

#[test]
fn list_without_lead_in_stays_at_outer_level() {
    let harness = TestHarness::new("changelog-list-no-lead-in");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "fix: correct the second thing

- alpha item
- beta item",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "  - alpha item
  - beta item"
        ),
        "List without a lead-in was not rendered at the outer level. Changelog is:\n{changelog}"
    );
}

#[test]
fn merge_conflict_metadata_is_dropped() {
    let harness = TestHarness::new("changelog-merge-conflict");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "chore: tidy the third thing

Real body text.

# Conflicts:
#\tCargo.lock
#\tCargo.toml",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains("Real body text."),
        "The real body text was dropped along with the conflict metadata. Changelog is:\n{changelog}"
    );
    assert!(
        !changelog.contains("Conflicts:"),
        "Merge conflict metadata was published. Changelog is:\n{changelog}"
    );
    assert!(
        !changelog.contains("Cargo.lock"),
        "Merge conflict file list was published. Changelog is:\n{changelog}"
    );
}

#[test]
fn subject_opening_with_issue_reference_is_kept() {
    let harness = TestHarness::new("changelog-issue-ref-subject");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit("*", "#123: fix the startup path");

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains("- Fix the startup path"),
        "A subject opening with an issue reference was skipped. Changelog is:\n{changelog}"
    );
}

#[test]
fn body_line_opening_with_issue_reference_is_kept() {
    let harness = TestHarness::new("changelog-issue-ref-body");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "chore: tidy the third thing

#123 was the original report",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains("- #123 was the original report"),
        "A body line opening with an issue reference was mangled. Changelog is:\n{changelog}"
    );
}

#[test]
fn squash_summary_subject_is_dropped() {
    let harness = TestHarness::new("changelog-squash-summary");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit("*", "# This is a combination of 2 commits.");

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        !changelog.contains("This is a combination"),
        "A squash artifact was published. Changelog is:\n{changelog}"
    );
}

#[test]
fn fixup_reference_subject_is_dropped() {
    let harness = TestHarness::new("changelog-fixup-ref");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit("*", "--fixup=31047affea8826bb03b4c4e2161da6c26241661d");

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        !changelog.contains("--fixup="),
        "A fixup artifact was published. Changelog is:\n{changelog}"
    );
}

/// The commit body is written as a sub-list under the message due to Markdown styling constraints.
#[test]
fn commit_body_is_rendered_as_sublist() {
    let harness = TestHarness::new("changelog-body");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: some summary of the changes

More information about the changes.",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "- Some summary of the changes
  - More information about the changes."
        ),
        "Body was not nested under the heading. Changelog is:\n{changelog}"
    );
}

/// The commit body is written as a sub-list under the message due to Markdown styling constraints.
///
/// Make sure that wrapped lines are rendered on the same line
#[test]
fn commit_body_with_a_long_line_is_rendered_as_one_line() {
    let harness = TestHarness::new("changelog-long-body");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: some summary of the changes

More information about the changes that is really long and will overflow and therefore
needs to be written across two lines.",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "- Some summary of the changes
  - More information about the changes that is really long and will overflow and therefore needs to be written across two lines.",
        ),
        "Wrapped body was not written on one line. Changelog is:\n{changelog}"
    );
}

/// The commit body is written as a sub-list under the message due to Markdown styling constraints.
///
/// This test checks that the body is correctly rendered as a sub-list, even if that body has a list
/// itself.
#[test]
fn trailing_paragraph_after_list_is_nested_under_it() {
    let harness = TestHarness::new("changelog-trailing-paragraph");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: add the first thing

Some lead-in prose:

- an item
- another item

A trailing paragraph after the list.",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "- Add the first thing
  - Some lead-in prose:
    - an item
    - another item
  - A trailing paragraph after the list."
        ),
        "Trailing paragraph was not nested under the list. Changelog is:\n{changelog}"
    );
}

/// Commits marked as breaking should be marked as such in the Changelog.
#[test]
fn breaking_change_without_explanation() {
    let harness = TestHarness::new("changelog-breaking");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit("*", "feat!: added a breaking change");

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains("- \\[**BREAKING**\\] Added a breaking change"),
        "Breaking change was not rendered as such. Changelog is:\n{changelog}"
    );

    assert!(
        !changelog.contains("**Breaking Change**"),
        "No explanation was given so it should not be rendered. Changelog is:\n{changelog}"
    );
}

/// Commits marked as breaking and with an explanation should be marked as such in the Changelog.
#[test]
fn breaking_change_with_explanation() {
    let harness = TestHarness::new("changelog-breaking-explanation");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat!: added a breaking change

BREAKING CHANGE: This explains why it is breaking",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains("- \\[**BREAKING**\\] Added a breaking change"),
        "Breaking change was not rendered as such. Changelog is:\n{changelog}"
    );

    assert!(
        changelog.contains("  - **Breaking Change**: This explains why it is breaking"),
        "An explanation was given but was not rendered. Changelog is:\n{changelog}"
    );
}

#[test]
fn breaking_change_explanation_with_bulleted_list_renders_as_a_sublist() {
    let harness = TestHarness::new("changelog-breaking-explanation-list");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat!: added a breaking change

BREAKING CHANGE: This breaks things for the following reasons:
- first reason
- second reason",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "  - **Breaking Change**: This breaks things for the following reasons:
    - first reason
    - second reason"
        ),
        "Explanation was not rendered with a nested sublist. Changelog is:\n{changelog}"
    );

    assert!(
        !changelog.contains("@@ CLIFF LIST ITEM @@"),
        "Internal list-item marker leaked into the changelog. Changelog is:\n{changelog}"
    );
}

/// Commits not marked as breaking with the exclamation `!` but do have a breaking change
/// explanation should be marked as breaking in the Changelog.
#[test]
fn breaking_change_without_mark_but_with_explanation() {
    let harness = TestHarness::new("changelog-breaking-explanation-no-mark");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: a breaking change but without marking with `!`

BREAKING CHANGE: This explains why it is breaking",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains("- \\[**BREAKING**\\] A breaking change but without marking with `!`"),
        "Breaking change was not rendered as such. Changelog is:\n{changelog}"
    );

    assert!(
        changelog.contains("  - **Breaking Change**: This explains why it is breaking"),
        "An explanation was given but was not rendered. Changelog is:\n{changelog}"
    );
}

/// List items as part of a commit body can overflow onto the next line, these must be rendered on
/// the same line.
#[test]
fn list_item_with_a_wrapping_line_renders_as_a_single_line() {
    let harness = TestHarness::new("changelog-list-item-continuation");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: add the first thing

Lead-in prose:

- More information about the changes that is really long and will overflow and therefore
needs to be written across two lines.
- Second item.",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "- Add the first thing
  - Lead-in prose:
    - More information about the changes that is really long and will overflow and therefore needs to be written across two lines.
    - Second item."
        ),
        "Continuation paragraph did not stay nested under its list item. Changelog is:\n{changelog}"
    );
}

#[test]
fn fixup_commit_ignored() {
    let harness = TestHarness::new("changelog-fixup");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit("*", "feat: add the first thing");
    harness.write_file_content("b.txt", "b");
    harness.commit("*", "fixup! feat: add the first thing");

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        !changelog.contains("feat: add the first thing"),
        "A fixup commit was added to the changelog. Changelog is:\n{changelog}"
    );
}

#[test]
fn amend_commit_ignored() {
    let harness = TestHarness::new("changelog-amend");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit("*", "feat: add the first thing");
    harness.write_file_content("b.txt", "b");
    harness.commit("*", "amend! feat: add the first thing");

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        !changelog.contains("feat: add the first thing"),
        "An amend commit was added to the changelog. Changelog is:\n{changelog}"
    );
}

#[test]
fn squash_commit_ignored() {
    let harness = TestHarness::new("changelog-squash");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit("*", "feat: add the first thing");
    harness.write_file_content("b.txt", "b");
    harness.commit("*", "squash! feat: add the first thing");

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        !changelog.contains("feat: add the first thing"),
        "A squash commit was added to the changelog. Changelog is:\n{changelog}"
    );
}

/// A body paragraph starting with a bare dash run not followed by a space, such as a CLI flag name,
/// is not a list marker and must be processed like any other paragraph.
#[test]
fn body_paragraph_starting_with_a_flag_name() {
    let harness = TestHarness::new("changelog-flag-paragraph");

    harness.add_standard_gitignore();

    harness.write_file_content("a.txt", "a");
    harness.commit(
        "*",
        "feat: add the first thing

--verbose flag enables logging",
    );

    harness.generate_changelog(ChangelogConfig::Pre1Point0Cliff, None);
    let changelog = harness.read_file_content("CHANGELOG.md");

    assert!(
        changelog.contains(
            "- Add the first thing
  - --verbose flag enables logging"
        ),
        "Leading dashes on a non-list paragraph were not stripped. Changelog is:\n{changelog}"
    );
}
