use crate::prepare_release::{
    generate_changelog, get_next_version, get_released_version_tag, run_semver_checks, set_version,
};
use crate::publish_release::{create_gh_release, is_releasable_change, publish};
use crate::utils::{get_current_version_from_cargo_toml, get_revision_for_tag, push_tag, tag};
use anyhow::Context;
use std::fs::read_to_string;
use std::path::Path;

mod prepare_release;
mod publish_release;
pub mod utils;

pub const RELEASE_LABEL: &str = "hra-release";

/// Prepares changes for the next release.
///
/// - Runs semver checks on the current branch to ensure it is releasable with
///   the requested configuration.
/// - Generates a changelog using `git-cliff` based on the provided configuration.
/// - Sets the version in the `Cargo.toml` files to the next version determined by `git-cliff`.
pub fn prepare_release(
    dir: impl AsRef<Path>,
    cliff_config: String,
    force_version: Option<String>,
) -> anyhow::Result<()> {
    let repository = git2::Repository::open(&dir).context("Failed to open git repository")?;

    let force_tag = input_version_to_version_tag(force_version)?;

    // Generate the changelog and check what version it chose.
    generate_changelog(&dir, &cliff_config, &force_tag)?;
    let next_version_tag = get_next_version(&dir, &cliff_config, &force_tag)?;

    // Set the version in the Cargo.toml files.
    set_version(&dir, &next_version_tag)?;

    // Ensure the changes on the current branch pass semver checks.
    match get_released_version_tag(&dir, &cliff_config, &force_tag) {
        Ok(released_version_tag) => {
            println!("Retrieving revision for tag: {}", released_version_tag);
            let revision = get_revision_for_tag(&repository, &released_version_tag)?;
            run_semver_checks(&dir, &revision)?;
        }
        Err(e) => {
            eprintln!("No previous release found, skipping semver checks: {e:?}");
        }
    }

    Ok(())
}

/// Publishes a release if one is found.
///
/// - First checks whether the current HEAD commit is part of a releasable change. A change is
///   releasable if the commit was introduced by a PR that has the `hra-release` label.
/// - If a releasable change is found, it tags the current HEAD commit with the version from the
///   `Cargo.toml` file.
/// - Finally, it publishes the crates.
pub fn publish_release(
    dir: impl AsRef<Path>,
    git_token: String,
    danger_skip_releasable_changes_check: bool,
    danger_skip_create_gh_release: bool,
) -> anyhow::Result<()> {
    let repository = git2::Repository::open(&dir).context("Failed to open git repository")?;

    if !danger_skip_releasable_changes_check {
        let maybe_pr_number = is_releasable_change(&repository, &dir)?;
        let Some(pr_number) = maybe_pr_number else {
            println!("Not a releasable change, stopping.");
            return Ok(());
        };
        println!("Found releasable change with PR number: {}", pr_number);
    }

    let cargo_toml =
        read_to_string(dir.as_ref().join("Cargo.toml")).context("Failed to read Cargo.toml")?;
    let current_version = get_current_version_from_cargo_toml(&cargo_toml)
        .context("Failed to find version in Cargo.toml")?;
    let current_tag = format!("v{current_version}");

    tag(&repository, &current_tag, &current_tag).context("Failed to tag the release")?;
    println!("Tagged current HEAD with: {}", current_tag);

    push_tag(&repository, &git_token, &current_tag).context("Failed to push tag to remote")?;
    println!("Pushed tag to remote: {}", current_tag);

    publish(&dir).context("Failed to publish crates")?;

    if !danger_skip_create_gh_release {
        create_gh_release(&dir, &current_tag).context("Failed to create GitHub release")?;
    }

    Ok(())
}

pub(crate) fn input_version_to_version_tag(
    force_version: Option<String>,
) -> anyhow::Result<Option<String>> {
    let force_tag = match force_version {
        Some(input) if input.is_empty() => None,
        Some(maybe_version_tag) => match maybe_version_tag.strip_prefix("v") {
            Some(version) => {
                if semver::Version::parse(version).is_ok() {
                    Some(maybe_version_tag)
                } else {
                    anyhow::bail!("Invalid version format: {}", version);
                }
            }
            None => {
                if semver::Version::parse(&maybe_version_tag).is_ok() {
                    Some(format!("v{}", maybe_version_tag))
                } else {
                    anyhow::bail!("Invalid version format: {}", maybe_version_tag);
                }
            }
        },
        None => None,
    };

    Ok(force_tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_input_version_to_version_tag() {
        // Maps no input to None
        assert_eq!(input_version_to_version_tag(None).unwrap(), None);

        // Valid version with 'v' prefix remains unchanged
        assert_eq!(
            input_version_to_version_tag(Some("v1.2.3".to_string())).unwrap(),
            Some("v1.2.3".to_string())
        );

        // Valid version gets prefixed with 'v'
        assert_eq!(
            input_version_to_version_tag(Some("1.2.3".to_string())).unwrap(),
            Some("v1.2.3".to_string())
        );

        // Invalid semver is rejected
        assert!(input_version_to_version_tag(Some("invalid".to_string())).is_err());
        // Invalid semver with a 'v' prefix is rejected
        assert!(input_version_to_version_tag(Some("vinvalid".to_string())).is_err());
    }
}
