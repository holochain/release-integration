//! Common utilities for release management in a Git repository.
//!
//! This module contains code that is common between this crate and the integration tests crate.

use anyhow::Context;
use git2::ObjectType;

/// Get the Git revision for a given tag in a repository.
pub fn get_revision_for_tag(repository: &git2::Repository, tag: &str) -> anyhow::Result<String> {
    let id = repository
        .revparse_single(format!("refs/tags/{}", tag).as_str())
        .context("Failed to find tag")?
        .peel_to_commit()
        .context("Failed to find commit")?
        .id();

    Ok(id.to_string())
}

/// Create a tag in the given repository.
///
/// - If the tag exists and already points to the current HEAD commit, it will not be created again.
/// - If the tag exists but points to a different commit, it will be updated to point to the current
///   HEAD commit.
/// - If the tag does not exist, it will be created pointing to the current HEAD commit.
pub fn tag(repository: &git2::Repository, tag: &str, message: &str) -> anyhow::Result<()> {
    let signature = repository.signature().context("Failed to get signature")?;
    let head = repository.head().context("Failed to get HEAD")?;
    let commit = head
        .peel(ObjectType::Commit)
        .context("Failed to peel HEAD to commit")?;

    let force = match get_revision_for_tag(repository, tag) {
        Ok(revision) => {
            if commit.id().to_string() == revision {
                println!("Tag '{}' already exists for commit {}", tag, commit.id());
                return Ok(());
            } else {
                println!(
                    "Updating existing tag '{}' to point to commit {}",
                    tag,
                    commit.id()
                );
                true
            }
        }
        Err(_) => {
            // If there was an error getting the tag, assume it doesn't exist and don't try
            // to force creating it.
            false
        }
    };

    repository
        .tag(tag, &commit, &signature, message, force)
        .context("Failed to create tag")?;

    Ok(())
}

/// Get the version from the output of `git-cliff` command.
///
/// `git-cliff` must have been called with the `--context` flag.
pub fn get_version_from_cliff_output(output: &[u8]) -> anyhow::Result<String> {
    let value = serde_json::from_slice::<Vec<serde_json::Value>>(output)
        .context("Unexpected output from git-cliff")?;

    Ok(value
        .first()
        .context("No value in git-cliff output list")?
        .as_object()
        .context("Expected an object in git-cliff output list")?
        .get("version")
        .context("Expected 'version' in git-cliff output")?
        .as_str()
        .context("Expected a string as the version in git-cliff output")?
        .to_string())
}

/// Get the current version from the given content.
///
/// The content is expected to be the content of a `Cargo.toml` file.
pub fn get_current_version_from_cargo_toml(content: &str) -> anyhow::Result<String> {
    let cargo_toml = toml::from_str::<toml::Value>(content).context("Invalid TOML")?;

    let get_version_from_table = |table: &toml::Value| -> anyhow::Result<String> {
        Ok(table
            .as_table()
            .context("Expected root to be a table")?
            .get("package")
            .context("Expected 'package' in Cargo.toml")?
            .as_table()
            .context("Expected 'package' to be a table")?
            .get("version")
            .context("Expected 'version' in package table")?
            .as_str()
            .context("Expected a string as the version in package table")?
            .to_string())
    };

    match cargo_toml
        .as_table()
        .context("Expected root to be a table")?
        .get("workspace")
    {
        Some(workspace) => get_version_from_table(workspace),
        None => get_version_from_table(&cargo_toml),
    }
}
