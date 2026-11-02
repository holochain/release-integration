//! Changelog rendering tests.
//!
//! The tests in this module must be runnable locally and in CI so cannot rely on external services.

use integration::{ChangelogConfig, TestHarness};

/// How the content of a commit body is rendered into the changelog.
///
/// With this test, we get:
/// - Lists in a commit body keep their structure instead of being flattened onto one line.
/// - Hard-wrapped lines are joined back together without leaving a run of spaces behind.
/// - List items are rendered as the author wrote them, so identifiers are not capitalised.
/// - Repository mechanics, such as the `# Conflicts:` block left behind by a conflicted rebase,
///   are not published as release notes.
/// - Rebase and squash artifacts that ended up as commit subjects are left out entirely.
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
    // A list nested under another list. Depth is not carried through, so this records the
    // one level of flattening rather than asserting a nesting we do not produce.
    //
    harness.write_file_content("g.txt", "g");
    harness.commit(
        "*",
        "feat: add a nested list

Lead in:

- parent item
  - child item
- second parent",
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
    // Not an artifact: the `#` opens an issue reference rather than a comment git wrote.
    harness.write_file_content("h.txt", "h");
    harness.commit("*", "#123: fix the startup path");
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
    // Depth beyond the first level is not preserved, so a child item comes out as a sibling of
    // its parent. This is asserted so the limitation is visible and cannot change unnoticed.
    // The content itself must survive either way.
    //
    assert!(
        changelog.contains(
            "  - Lead in:
    - parent item
    - child item
    - second parent"
        ),
        "Nested list rendering changed. Changelog is:\n{changelog}"
    );

    //
    // A subject is only an artifact when the `#` is a comment git wrote, so an issue reference
    // must survive as a subject, not just inside a body. It is rendered without the `#123:`
    // because git-cliff reads that as the conventional commit type and the template prints only
    // the description, which is how every other type is handled too.
    //
    assert!(
        changelog.contains("- Fix the startup path"),
        "A subject opening with an issue reference was skipped. Changelog is:\n{changelog}"
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

    // The Other Changes group itself is not checked for absence. It legitimately holds the
    // issue-reference subject above, which is an unconventional commit rather than an artifact.
    // The assertions above already establish that no artifact reaches the changelog at all.
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
