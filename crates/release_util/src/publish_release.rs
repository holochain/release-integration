use crate::RELEASE_LABEL;
use anyhow::Context;
use std::path::Path;

/// Checks if the current HEAD commit is part of a merged pull request that is releasable.
///
/// Determined by the presence of the `hra-release` label on the pull request that this change came
/// from.
pub(crate) fn is_releasable_change(
    repository: &git2::Repository,
    dir: impl AsRef<Path>,
) -> anyhow::Result<Option<u64>> {
    let head = repository
        .head()
        .context("Failed to get HEAD reference")?
        .peel_to_commit()
        .context("Failed to retrieve HEAD commit")?;

    let output = std::process::Command::new("gh")
        .current_dir(&dir)
        .arg("pr")
        .arg("list")
        .arg("--search")
        .arg(head.id().to_string())
        .arg("--state")
        .arg("merged")
        .arg("--json")
        .arg("id,number,labels")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("Failed to run `gh pr list`")?;

    let matches = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout)
        .context("Failed to parse `gh pr list` output")?;

    if matches.len() == 1 {
        let values = &matches[0]
            .as_object()
            .context("Expected a JSON object value as PR list output")?;

        let pr_number = values
            .get("number")
            .context("Missing 'number' in PR data")?
            .as_number()
            .context("Expected a number as the PR number value")?
            .as_u64()
            .expect("PR number should be a valid u64");

        println!(
            "Have labels for PR #{}: {:?}",
            pr_number,
            values.get("labels")
        );

        let labels = values
            .get("labels")
            .context("Missing 'labels' in PR data")?
            .as_array()
            .context("Expected an array for labels")?
            .iter()
            .map(|v| {
                v.as_object()
                    .context("Expected label object")?
                    .get("name")
                    .context("Expected label to have a name")?
                    .as_str()
                    .context("Expected label name to be a string")
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        if labels.contains(&RELEASE_LABEL) {
            println!(
                "Found releasable PR #{} with 'hra-release' label",
                pr_number
            );
            return Ok(Some(pr_number));
        } else {
            println!(
                "PR #{} is not releasable due to missing 'hra-release' label",
                pr_number
            );
        }
    }

    println!("No releasable PR found for the current HEAD commit.");
    Ok(None)
}

pub(crate) fn publish(dir: impl AsRef<Path>) -> anyhow::Result<()> {
    let status = std::process::Command::new("cargo")
        .current_dir(dir)
        .arg("workspaces")
        .arg("publish")
        .arg("--allow-branch")
        .arg("(main|release)*")
        .arg("--publish-as-is")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to run workspace publish")?;

    if !status.success() {
        anyhow::bail!("Failed to publish workspace");
    }

    Ok(())
}

/// Parse the version from a release tag, which may or may not have a `v` prefix.
fn parse_version_tag(tag: &str) -> anyhow::Result<semver::Version> {
    semver::Version::parse(tag.trim_start_matches('v'))
        .with_context(|| format!("Release tag is not a valid version: {tag}"))
}

/// Checks whether a release tag refers to a pre-release version.
///
/// Determined by the semver pre-release field, so any pre-release format is recognised, not just
/// the `-dev.X` format that is currently used for Holochain releases.
fn is_prerelease_tag(tag: &str) -> anyhow::Result<bool> {
    Ok(!parse_version_tag(tag)?.pre.is_empty())
}

/// Get the tag of the repository's current latest release, if it has one.
///
/// Pre-releases and drafts are never the latest release, so they are not considered here.
fn get_latest_release_tag(dir: impl AsRef<Path>) -> anyhow::Result<Option<String>> {
    let output = std::process::Command::new("gh")
        .current_dir(dir)
        .arg("release")
        .arg("view")
        .arg("--json")
        .arg("tagName")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("Failed to run `gh release view`")?;

    // The command fails when the repository has no releases yet, which is not an error here.
    if !output.status.success() {
        println!("No current latest release found, this release will become the latest.");
        return Ok(None);
    }

    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .context("Failed to parse `gh release view` output")?;

    Ok(Some(
        value
            .as_object()
            .context("Expected a JSON object as release view output")?
            .get("tagName")
            .context("Missing 'tagName' in release data")?
            .as_str()
            .context("Expected the tag name to be a string")?
            .to_string(),
    ))
}

/// Checks whether a release tag should become the repository's latest release.
///
/// GitHub marks every new release as the latest one unless told otherwise, which is wrong when
/// releasing from a release branch after a newer version has been released from another branch.
/// For example, releasing `v0.3.7` when `v0.4.2` is already out should leave `v0.4.2` as the
/// latest release.
fn should_be_latest_release(tag: &str, current_latest_tag: Option<&str>) -> anyhow::Result<bool> {
    let version = parse_version_tag(tag)?;

    let Some(current_latest_tag) = current_latest_tag else {
        return Ok(true);
    };

    // Only the tag being released has to be a valid version. If the current latest release was not
    // created by this tool, then leave it to GitHub to decide what the latest release should be.
    match parse_version_tag(current_latest_tag) {
        Ok(current_latest_version) => Ok(version > current_latest_version),
        Err(e) => {
            println!("Ignoring the current latest release: {e:?}");
            Ok(true)
        }
    }
}

pub(crate) fn create_gh_release(dir: impl AsRef<Path>, tag: &str) -> anyhow::Result<()> {
    let repository_name = std::env::var("GITHUB_REPOSITORY")
        .context("Missing environment variable `GITHUB_REPOSITORY`")?
        .split('/')
        .next_back()
        .context("GITHUB_REPOSITORY is not a valid GITHUB_REPOSITORY")?
        .to_string();

    let tag_version = tag.trim_start_matches('v');

    let mut command = std::process::Command::new("gh");

    command
        .current_dir(&dir)
        .arg("release")
        .arg("create")
        .arg(tag)
        .arg("--generate-notes")
        .arg("--title")
        .arg(format!("{} {}", repository_name, tag_version))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if is_prerelease_tag(tag)? {
        println!("Creating {} as a pre-release", tag);
        // GitHub will not select a pre-release as the latest release, but say so explicitly rather
        // than relying on that.
        command.arg("--prerelease").arg("--latest=false");
    } else {
        let current_latest_tag = get_latest_release_tag(&dir)?;
        if !should_be_latest_release(tag, current_latest_tag.as_deref())? {
            println!(
                "Creating {} without making it the latest release, {} is newer",
                tag,
                current_latest_tag.unwrap_or_default()
            );
            command.arg("--latest=false");
        }
    }

    let status = command
        .status()
        .context("Failed to create GitHub release")?;

    if !status.success() {
        anyhow::bail!(
            "Failed to create GitHub release, gh exited with: {}",
            status
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_tag_is_not_a_prerelease() {
        assert!(!is_prerelease_tag("v1.2.3").unwrap());
    }

    #[test]
    fn dev_tag_is_a_prerelease() {
        assert!(is_prerelease_tag("v1.2.3-dev.0").unwrap());
    }

    #[test]
    fn rc_tag_is_a_prerelease() {
        assert!(is_prerelease_tag("v1.2.3-rc.1").unwrap());
    }

    #[test]
    fn build_metadata_alone_is_not_a_prerelease() {
        assert!(!is_prerelease_tag("v1.2.3+build.5").unwrap());
    }

    #[test]
    fn tag_without_v_prefix_is_accepted() {
        assert!(is_prerelease_tag("1.2.3-dev.0").unwrap());
    }

    #[test]
    fn tag_that_is_not_semver_is_rejected() {
        assert!(is_prerelease_tag("vnot-a-version").is_err());
    }

    #[test]
    fn newer_release_becomes_latest() {
        assert!(should_be_latest_release("v0.4.3", Some("v0.4.2")).unwrap());
    }

    #[test]
    fn patch_for_an_older_release_line_does_not_become_latest() {
        assert!(!should_be_latest_release("v0.3.7", Some("v0.4.2")).unwrap());
    }

    #[test]
    fn first_release_of_a_repository_becomes_latest() {
        assert!(should_be_latest_release("v0.1.0", None).unwrap());
    }

    #[test]
    fn unrecognised_current_latest_release_leaves_the_release_as_latest() {
        assert!(should_be_latest_release("v1.0.0", Some("nightly")).unwrap());
    }
}
