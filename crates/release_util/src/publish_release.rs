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

pub(crate) fn create_gh_release(dir: impl AsRef<Path>, tag: &str) -> anyhow::Result<()> {
    let repository_name = std::env::var("GITHUB_REPOSITORY")
        .context("Missing environment variable `GITHUB_REPOSITORY`")?
        .split('/')
        .next_back()
        .context("GITHUB_REPOSITORY is not a valid GITHUB_REPOSITORY")?
        .to_string();

    let tag_version = tag.trim_start_matches('v');

    std::process::Command::new("gh")
        .current_dir(dir)
        .arg("release")
        .arg("create")
        .arg("--generate-notes")
        .arg("--title")
        .arg(format!("{} {}", repository_name, tag_version))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to create GitHub release")?;

    Ok(())
}
