# release-integration

Integration of third-party release tools with Holochain repositories

## Overview

This repository provides a CLI tool for integrating third-party release tools for use in Holochain repositories. It 
operates in two stages, to first prepare a release and then to publish it. This allows the release preparation to be
reviewed, possibly updated, and then approved before publishing proceeds.

The preparation stage uses:
- [git-cliff](https://git-cliff.org/) to pick a new version number based on the commit history, and to generate a
  changelog from that history.
- [cargo-workspaces](https://github.com/pksunkara/cargo-workspaces) to update the version number in all relevant
  `Cargo.toml` files.
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) to check the semver compliance of the
  changes in the repository against the previous release.

The publishing stage uses:
- The [GitHub CLI](https://cli.github.com/) to determine if the HEAD of the current branch came from a pull request,
  and if it did, then whether the PR was labeled with `hra-release`.
- [git2](https://github.com/rust-lang/git2-rs) to tag the HEAD of the current branch with the new version number, and
  then to push that tag to the remote repository.
- The [GitHub CLI](https://cli.github.com/) again to create a GitHub release for the new version.

## Committing to a repository that uses this tool

When committing to a repository that uses this tool, you should follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) 
format. This will allow the tool to automatically determine the next version number based on the commit history.

To get an idea of the commit messages that `git-cliff` is configured to expect, you can check the [`pre-1.0-cliff.toml`](./pre-1.0-cliff.toml)  
file for the `git.commit_parsers` field. These patterns are used to parse commit messages and group them under 
categorized headings in the generated changelog.

Note that the `cocogitto` bot is enabled on all Holochain repositories, and will automatically reject commits that do
not follow the Conventional Commits format. However, just because a commit is compliant with Conventional Commits does
not necessarily mean it will show up how you expect in the changelog. It is preferred that commits use messages which
fit the patterns mentioned above.

Currently, Holochain repositories permit either "Rebase and merge" or "Squash and merge" as the merge strategy for pull 
requests. Unless you are working on a pull request that was opened before the introduction of this tool, it is 
recommended to use "Rebase and merge" as the merge strategy.

## Maintaining repositories that use this tool

There are a few important things to know when maintaining repositories that use this tool:
- Release tags are filtered by branch so that only tags that are relevant to the current branch are considered. This is 
  done to permit creating new releases from release branches, after newer versions have been published from the main 
  branch.
- Although the tool used to manage versions in `Cargo.toml` files (cargo-workspaces) is capable of understanding various
  strategies for versioning crates within a workspace, this tool only supports using a single version for all crates and
  it must be specified in the root `Cargo.toml` file. Use the `[workspace.package]` section to specify the version and 
  then reference that version in crates as `version.workspace = true`.

## Integrating the tool into a repository

This repository only provides a binary CLI tool, which is published to a release on this repository when the repository
is tagged. To integrate the tool, you need two workflows which are provided in the `holochain/actions` repository.

Note that the actions are versioned, and point to a specific version of this tool, as well as using specific versions of
the third-party tools that have been tested here. When updating either this tool, or the actions, you should try to keep
both in sync, and update documentation here accordingly.

The release [preparation workflow](https://github.com/holochain/actions/blob/main/.github/workflows/prepare-release.yml)
can be added to your repository with a workflow that looks like this:

```yaml
name: Prepare a release

on:
  workflow_dispatch:
    inputs:
      force_version:
        type: string
        description: "Specify the semver version for the next release, to override the default semver bump"
        default: ""
        required: false

jobs:
  call:
    uses: holochain/actions/.github/workflows/prepare-release.yml@v1.0.0
    with:
      cliff_config: "https://raw.githubusercontent.com/holochain/release-integration/refs/heads/main/pre-1.0-cliff.toml"
      force_version: ${{ inputs.force_version }}
    secrets:
      HRA2_GITHUB_TOKEN: ${{ secrets.HRA2_GITHUB_TOKEN }}
```

The release [publishing workflow](https://github.com/holochain/actions/blob/main/.github/workflows/publish-release.yml) 
can be added with a workflow that looks like this:

```yaml
name: Publish release

on:
  push:
    branches:
      - main
      - main-*
      - release/*
      - release-*

jobs:
  call:
    uses: holochain/actions/.github/workflows/publish-release.yml@v1.0.0
    secrets:
      HRA2_GITHUB_TOKEN: ${{ secrets.HRA2_GITHUB_TOKEN }}
      HRA2_CRATES_IO_TOKEN: ${{ secrets.HRA2_CRATES_IO_TOKEN }}
```

For this to work, the repository requires:
1. A Rust project which is either a library, or a workspace configured as described above.
2. The `hra-release` label. 
3. The `HRA2_GITHUB_TOKEN` which is used to grant the HRA2 user access to the repository to create pull requests and 
   releases.
4. The `HRA2_CRATES_IO_TOKEN` which is used to publish crates to crates.io. That token must have the "publish-new" and
   "publish-update" scopes.

Only requirement 1. needs to be done manually. The other requirements are [automated](https://github.com/holochain/hc-github-config).
Look for the `AddReleaseIntegrationSupport` function which is used to add the label and the secrets to a repository.

## Publishing a release using the workflows

In the most basic case, you can publish a release by finding the "Prepare a release" workflow in the "Actions" tab and
running it with no custom inputs. This will automatically determine the next version number based on the commit history,
and existing tags, then generate a PR with the changes needed to prepare the release. You can then review that PR and 
when it merges, the publishing will happen automatically.

For releases from release branches, the same workflow can be run, but you need to specify the branch to run from, and
the workflow must be present on that release branch.

When the version that you need to release is not the next version according to the commit history, you can override the 
default semver bump by specifying the `force_version` input when running the "Prepare a release" workflow. When forcing
a version:
- The version must be valid semver, or a semver tag. So either `0.2.0` or `v0.2.0` are valid.
- The `cargo-semver-checks` must still pass. So if you try to force a version that would violate semver against the 
  previous release, then the preparation will fail.

Note that there are three cases where you must force a version:
- When switching to a pre-release version. The only supported pre-release format is currently `-dev.X`, though this can
  be relaxed in the future if needed.
- When switching from a pre-release version to a stable version.
- After branching a release branch, and wanting to bump to a new version that is not the next semver version. For 
  example, if you have just created a `release-0.2` branch which contained the version `0.2.5`, then the next semver
  version would be `0.2.6`, but you want to release `0.3.0` from that branch, then you need to force the version to 
  `0.3.0`.

## Setting up a test environment

The tests in this repository need to run against real services, running locally. These are a crate registry and a Git
server.

To start these services, run:

```shell
docker compose up -d
```

There is a manual setup step needed for the Git server. Navigate to `http://localhost:3000` and press the 
"Install Gitea" button. When this process completes and you are redirected to the login page, you can proceed to the
automated steps.

Use the setup script:

```shell
nix develop -c ./scripts/run_setup.sh
```

If this script succeeds, you should find a git token in `./scripts/git_test_token.txt` and a crates token in 
`./scripts/crates_test_token.txt`.

>! **NOTE**: The tests are limited in what they can check. They can verify that we're using the third-party tools 
>  correctly, but they cannot verify the integration with GitHub. There are aspects of the integration like detecting
>  release pull requests, creating releases and GitHub-specific changelog content that cannot be tested in this 
>  repository. When making changes that impact these areas, please ensure that you test them against a real repository
>  before releasing your changes.

## Logging into the test services

- Access Gitea at `http://localhost:3000` and log in with the username `gituser` and the password `pass`.
- Access the crates registry at `http://localhost:8000` and log in with the username `admin` and the password `admin`.

## Running the tests

Ensure the services are up and running, then run the tests with:

```shell
nix develop -c cargo test
```

Once the tests have finished, you can see the state that they have created in the running services.

## Publishing the CLI

This repository doesn't have its own release automation. Please follow the following steps:
- Ensure the tests pass.
- Update the version in `crates/release_util/Cargo.toml` to the next version.
- Commit the changes and push them.
- Run `git tag -a "v0.X.Y" -m "v0.X.Y"` with an appropriate version.
- Push the tag with `git push origin v0.X.Y`.
- The release CI workflow will create a GitHub release, and attach the CLI binary to it.
- Now publish the CLI to crates.io with `cargo publish --package holochain_release_util`

