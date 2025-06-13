use anyhow::Context;
use clap::{Parser, Subcommand};
use holochain_release_util::{prepare_release, publish_release};
use std::path::PathBuf;

#[derive(Parser)]
pub struct ReleaseUtilCli {
    /// The directory to run the command in.
    ///
    /// Defaults to the current directory.
    #[arg(long, default_value = ".")]
    dir: PathBuf,

    #[command(subcommand)]
    command: ReleaseUtilCommand,
}

#[derive(Subcommand)]
pub enum ReleaseUtilCommand {
    /// Prepare changes for the next release.
    ///
    /// Picks or accepts the next semver version, generates a changelog, and sets the version in the
    /// Cargo project.
    Prepare {
        /// The location of a `git-cliff` configuration file.
        ///
        /// This can either be a path to a file or a URL to a file.
        #[arg(long)]
        cliff_config: String,

        /// Force the release version, rather than letting the tool pick the next semver version.
        ///
        /// This should be used when switching to the next pre-release version or when switching
        /// to a new release version from a pre-release version.
        ///
        /// The code will treat an empty string the same as `None`, so it is safe to provide this
        /// argument without a value.
        #[arg(long)]
        force_version: Option<String>,
    },

    /// Publish a release if one is found.
    Publish,
}

fn main() -> anyhow::Result<()> {
    println!("Starting release-util...");
    let cli = ReleaseUtilCli::parse();

    match cli.command {
        ReleaseUtilCommand::Prepare {
            cliff_config,
            force_version,
        } => {
            prepare_release(cli.dir, cliff_config, force_version)?;
        }
        ReleaseUtilCommand::Publish => {
            let token = std::env::var("GH_TOKEN").context("Missing GH_TOKEN env var")?;
            publish_release(cli.dir, token, false, false)?;
        }
    }

    Ok(())
}
