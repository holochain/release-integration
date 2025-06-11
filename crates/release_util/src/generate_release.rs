use crate::utils::get_version_from_cliff_output;
use anyhow::Context;
use std::path::Path;

pub(crate) fn generate_changelog(
    dir: impl AsRef<Path>,
    cliff_config: &str,
    force_tag: &Option<String>,
) -> anyhow::Result<()> {
    println!("Generating changelog");

    let mut command = common_git_cliff_command(&dir, cliff_config, force_tag);

    command.arg("--unreleased").arg("--bump");

    if dir.as_ref().join("CHANGELOG.md").exists() {
        command.arg("--output");
    } else {
        command.arg("--prepend").arg("CHANGELOG.md");
    }

    if let Some(tag) = force_tag {
        command.arg("--tag").arg(tag);
    }

    let status = command.status().context("git-cliff failed to run")?;

    if !status.success() {
        anyhow::bail!("git-cliff command failed with status: {}", status);
    }

    Ok(())
}

pub(crate) fn get_next_version(
    dir: impl AsRef<Path>,
    cliff_config: &str,
    force_tag: &Option<String>,
) -> anyhow::Result<String> {
    println!("Retrieving next version");

    let mut command = common_git_cliff_command(&dir, cliff_config, force_tag);

    command
        .arg("--unreleased")
        .arg("--bump")
        .arg("--context")
        .stdout(std::process::Stdio::piped());

    if let Some(tag) = force_tag {
        command.arg("--tag").arg(tag);
    }

    let output = command.output().context("git-cliff failed to run")?;

    get_version_from_cliff_output(&output.stdout)
}

pub(crate) fn set_version(dir: impl AsRef<Path>, version: &str) -> anyhow::Result<()> {
    let version = version.trim_start_matches('v');
    println!("Setting version to {}", version);

    let mut command = std::process::Command::new("cargo");

    command
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .arg("workspaces")
        .arg("version")
        .arg("--no-git-commit")
        .arg("--no-git-tag")
        .arg("--no-git-push")
        .arg("--no-individual-tags")
        .arg("--yes")
        .arg("custom")
        .arg(version);

    let status = command
        .status()
        .context("Failed to run cargo workspaces version")?;

    if !status.success() {
        anyhow::bail!(
            "cargo workspaces version command failed with status: {}",
            status
        );
    }

    Ok(())
}

pub(crate) fn get_released_version_tag(
    dir: impl AsRef<Path>,
    cliff_config: &str,
    force_tag: &Option<String>,
) -> anyhow::Result<String> {
    println!("Retrieving released version tag");

    let mut command = common_git_cliff_command(&dir, cliff_config, force_tag);

    command
        .arg("--latest")
        .arg("--context")
        .stdout(std::process::Stdio::piped());

    let output = command.output().context("git-cliff failed to run")?;

    get_version_from_cliff_output(&output.stdout)
}

pub(crate) fn run_semver_checks(
    dir: impl AsRef<Path>,
    against_revision: &str,
) -> anyhow::Result<()> {
    let status = std::process::Command::new("cargo")
        .current_dir(dir)
        .arg("semver-checks")
        .arg("--workspace")
        .arg("--baseline-rev")
        .arg(against_revision)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to run cargo semver-checks")?;

    if !status.success() {
        anyhow::bail!("cargo semver-checks command failed with status: {}", status);
    }

    Ok(())
}

fn common_git_cliff_command(
    dir: impl AsRef<Path>,
    cliff_config: &str,
    force_tag: &Option<String>,
) -> std::process::Command {
    let mut command = std::process::Command::new("git-cliff");

    command
        .current_dir(dir)
        .stderr(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .arg("--use-branch-tags");

    if url::Url::parse(cliff_config).is_ok() {
        command.arg("--config-url").arg(cliff_config);
    } else {
        command.arg("--config").arg(cliff_config);
    }

    if let Some(tag) = force_tag {
        if !tag.contains("-dev") {
            command.arg("--tag-pattern").arg("^v\\d+.\\d+.\\d+$");
        }
    }

    command
}
