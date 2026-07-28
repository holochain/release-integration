//! Changelog rendering tests.
//!
//! Unlike the other tests in this crate, nothing here pushes to a remote or publishes a crate,
//! because changelog generation is entirely local. That means this file, and only this file, can
//! run without the Gitea and registry services, so CI runs it on every pull request.

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
