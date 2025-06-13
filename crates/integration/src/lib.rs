use git2::{BranchType, IndexAddOption, RemoteCallbacks, Repository, RepositoryInitOptions};
use holochain_release_util::utils::push_tag;
use holochain_release_util::{prepare_release, publish_release};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn git_token() -> String {
    static TOKEN: OnceLock<String> = OnceLock::new();
    TOKEN
        .get_or_init(|| {
            std::fs::read_to_string("../../scripts/git_test_token.txt")
                .unwrap()
                .trim()
                .to_string()
        })
        .clone()
}

fn crates_token() -> String {
    static TOKEN: OnceLock<String> = OnceLock::new();
    TOKEN
        .get_or_init(|| {
            std::fs::read_to_string("../../scripts/crates_test_token.txt")
                .unwrap()
                .trim()
                .to_string()
        })
        .clone()
}

pub enum ChangelogConfig {
    Pre1Point0Cliff,
}

impl ChangelogConfig {
    pub fn path(&self) -> PathBuf {
        let dir = std::env::current_dir().unwrap();
        match self {
            ChangelogConfig::Pre1Point0Cliff => {
                dir.join("../../pre-1.0-cliff.toml").canonicalize().unwrap()
            }
        }
    }
}

pub struct TestHarness {
    random_id: String,
    temp_dir: tempfile::TempDir,
    repository: Repository,
}

impl TestHarness {
    pub fn new(project_name: &str) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();

        let random_id = nanoid::nanoid!(
            5,
            &[
                '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
                'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v',
                'w', 'x', 'y', 'z',
            ]
        );
        let origin_url = format!("http://localhost:3000/gituser/{project_name}-{random_id}.git");
        println!(
            "Creating repository with origin: {}",
            temp_dir.path().display()
        );
        let repository = Repository::init_opts(
            &temp_dir,
            RepositoryInitOptions::new()
                .origin_url(origin_url.as_str())
                .initial_head("main"),
        )
        .unwrap();

        let mut config = repository.config().unwrap();
        config.set_str("user.name", "gituser").unwrap();
        config
            .set_str("user.email", "gituser@holochain.org")
            .unwrap();
        config.set_str("credential.helper", "").unwrap();
        config.set_str("pager.branch", "false").unwrap();
        config.set_str("commit.gpgSign", "false").unwrap();
        config.set_str("tag.gpgSign", "false").unwrap();
        let mut index = repository.index().unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let signature = repository.signature().unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "chore: init",
                &repository.find_tree(tree_id).unwrap(),
                &[],
            )
            .unwrap();

        Self {
            random_id,
            temp_dir,
            repository,
        }
    }

    pub fn repository_url(&self) -> String {
        let origin = self
            .repository
            .find_remote("origin")
            .expect("Failed to find remote 'origin'");
        origin.url().unwrap().to_string()
    }

    pub fn write_file_content(&self, relative_path: &str, content: &str) {
        let file_path = self.temp_dir.path().join(relative_path);
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(file_path, content).unwrap();
    }

    pub fn read_file_content(&self, relative_path: &str) -> String {
        let file_path = self.temp_dir.path().join(relative_path);
        std::fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Failed to read file '{}'", relative_path))
    }

    pub fn commit(&self, pattern: &str, message: &str) {
        let mut index = self.repository.index().unwrap();
        index
            .add_all([pattern], IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = index.write_tree().unwrap();
        let tree = self.repository.find_tree(tree).unwrap();

        let head = self.repository.head().unwrap();

        let signature = self.repository.signature().unwrap();
        self.repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[&head.peel_to_commit().unwrap()],
            )
            .unwrap();
    }

    pub fn tag(&self, tag: &str, message: &str) {
        holochain_release_util::utils::tag(&self.repository, tag, message)
            .expect("Failed to create tag");
    }

    pub fn push_branch(&self, branch: &str) {
        let mut remote = self
            .repository
            .find_remote("origin")
            .expect("Failed to find remote 'origin'");

        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(Self::make_cb());
        push_opts.remote_push_options(&["repo.private=false"]);

        remote
            .push(
                &[format!("refs/heads/{branch}:refs/heads/{branch}")],
                Some(&mut push_opts),
            )
            .expect("Failed to push branch to remote");
    }

    pub fn push_tag(&self, tag: &str) {
        push_tag(&self.repository, &git_token(), tag).unwrap();
    }

    pub fn switch_branch(&self, branch: &str) {
        let head = self.repository.head().unwrap().peel_to_commit().unwrap();
        let target_branch = match self.repository.branch(branch, &head, false) {
            Ok(branch) => branch,
            Err(_) => {
                self.repository
                    .branches(Some(BranchType::Local))
                    .unwrap()
                    .find(|br| br.as_ref().unwrap().0.name().unwrap().unwrap() == branch)
                    .expect("Branch not found")
                    .unwrap()
                    .0
            }
        };

        let oid = target_branch.into_reference().target().unwrap();
        let tree = self.repository.find_object(oid, None).unwrap();

        self.repository
            .checkout_tree(&tree, None)
            .unwrap_or_else(|_| {
                panic!("Failed to switch to branch '{}'", branch);
            });
        self.repository
            .set_head(&format!("refs/heads/{branch}"))
            .unwrap();
    }

    pub fn check_index_clean(&self) {
        let diff = self.repository.diff_index_to_workdir(None, None).unwrap();
        assert_eq!(
            0,
            diff.deltas().count(),
            "Index is not clean, there are uncommitted changes"
        );
    }

    pub fn get_revision_for_tag(&self, tag: &str) -> String {
        holochain_release_util::utils::get_revision_for_tag(&self.repository, tag)
            .expect("Failed to get revision for tag")
    }

    pub fn add_standard_gitignore(&self) {
        self.write_file_content(
            ".gitignore",
            r#"target/
            "#,
        );

        self.commit(".gitignore", "chore: add standard .gitignore");
    }

    pub fn add_private_registry_cargo_toml(&self) {
        let token = crates_token();

        self.write_file_content(
            "./.cargo/config.toml",
            &format!(
                r#"[registries.dev-registry]
index = "sparse+http://localhost:8000/api/v1/crates/"
credential-provider = ["cargo:token"]
token = "{token}"

[registry]
default = "dev-registry"

[source.crates-io]
replace-with = "dev-registry-source"

[source.dev-registry-source]
registry = "sparse+http://localhost:8000/api/v1/crates/"
        "#
            ),
        );

        self.commit("./.cargo/config.toml", "chore: add private registry config");
    }

    pub fn add_crate(&self, crate_model: CrateModel) {
        self.write_file_content(
            "Cargo.toml",
            &format!(
                r#"[package]
name = "{}_{}"
version = "{}"
edition = "2024"
{}
{}
{}
        publish = ["dev-registry"]
"#,
                crate_model.name,
                self.random_id,
                crate_model.version,
                if let Some(description) = &crate_model.description {
                    format!("description = \"{}\"", description)
                } else {
                    String::new()
                },
                if let Some(repository) = &crate_model.repository {
                    format!("repository = \"{}\"", repository)
                } else {
                    String::new()
                },
                if let Some(license) = &crate_model.license {
                    format!("license = \"{}\"", license)
                } else {
                    String::new()
                },
            ),
        );

        self.add_crate_src_at_path(&crate_model, self.temp_dir.path());
    }

    pub fn add_workspace(&self, workspace_model: CargoWorkspaceModel) {
        let workspace_version = workspace_model
            .crates
            .first()
            .as_ref()
            .expect("No crates in workspace")
            .0
            .version
            .clone();
        self.write_file_content(
            "Cargo.toml",
            &format!(
                r#"[workspace]
members = [
    {}
]
resolver = "3"

[workspace.package]
version = "{}"
edition = "2024"
{}
{}

[workspace.dependencies]
{}
"#,
                workspace_model
                    .crates
                    .iter()
                    .map(|c| format!("\"crates/{}\"", c.0.name))
                    .collect::<Vec<_>>()
                    .join(",\n    "),
                workspace_version,
                if let Some(repository) = workspace_model
                    .crates
                    .first()
                    .as_ref()
                    .expect("No crates in workspace")
                    .0
                    .repository
                    .as_ref()
                {
                    format!("repository = \"{}\"", repository)
                } else {
                    String::new()
                },
                if let Some(license) = workspace_model
                    .crates
                    .first()
                    .as_ref()
                    .expect("No crates in workspace")
                    .0
                    .license
                    .as_ref()
                {
                    format!("license = \"{}\"", license)
                } else {
                    String::new()
                },
                workspace_model
                    .crates
                    .iter()
                    .map(|c| format!(
                        "{}_{} = {{ version = \"{}\", path = \"crates/{}\", registry = \"dev-registry\" }}",
                        c.0.name, self.random_id, workspace_version, c.0.name
                    ))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        );

        for (crate_model, workspace_dependencies) in &workspace_model.crates {
            self.write_file_content(
                &format!("crates/{}/Cargo.toml", crate_model.name),
                &format!(
                    r#"[package]
name = "{}_{}"
{}
version.workspace = true
edition.workspace = true
{}
{}

[dependencies]
{}
        "#,
                    crate_model.name,
                    self.random_id,
                    if let Some(description) = &crate_model.description {
                        format!("description = \"{}\"", description)
                    } else {
                        String::new()
                    },
                    if crate_model.repository.is_some() {
                        "repository.workspace = true".to_string()
                    } else {
                        String::new()
                    },
                    if crate_model.license.is_some() {
                        "license.workspace = true".to_string()
                    } else {
                        String::new()
                    },
                    workspace_dependencies
                        .iter()
                        .map(|dep| format!("{}_{}.workspace = true", dep, self.random_id))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
            );

            self.add_crate_src_at_path(
                crate_model,
                self.temp_dir.path().join("crates").join(&crate_model.name),
            );
        }
    }

    pub fn verify_cargo_project(&self, path: &str) {
        let path = self.temp_dir.path().join(path);

        let exit_status = std::process::Command::new("cargo")
            .current_dir(&path)
            .arg("clippy")
            .arg("--all-targets")
            .arg("--")
            .arg("--deny")
            .arg("warnings")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap()
            .wait();

        assert!(
            exit_status.unwrap().success(),
            "Cargo clippy failed for project at {}",
            path.display()
        );
    }

    pub fn git_status(&self) {
        let output = std::process::Command::new("git")
            .current_dir(self.temp_dir.path())
            .arg("status")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("Failed to run git status")
            .wait()
            .unwrap();

        assert!(output.success(), "git status failed");
    }

    pub fn generate_changelog(
        &self,
        changelog_config: ChangelogConfig,
        force_tag: Option<String>,
    ) -> String {
        let configure_command = |command: &mut std::process::Command| {
            command
                .current_dir(self.temp_dir.path())
                .arg("--config")
                .arg(changelog_config.path())
                .arg("--use-branch-tags")
                .arg("--unreleased");

            if let Some(tag) = &force_tag {
                if !tag.contains("-dev") {
                    command.arg("--tag-pattern").arg("^v\\d+.\\d+.\\d+$");
                }

                command.arg("--tag").arg(tag);
            }

            command.arg("--bump");
        };

        let mut command = std::process::Command::new("git-cliff");
        configure_command(&mut command);

        if !self.temp_dir.path().join("CHANGELOG.md").exists() {
            command.arg("--output");
        } else {
            command.arg("--prepend").arg("CHANGELOG.md");
        }

        let exit_status = command
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(exit_status.success(), "git-cliff command failed");

        let mut command = std::process::Command::new("git-cliff");
        configure_command(&mut command);

        let output = command
            .arg("--context")
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();

        holochain_release_util::utils::get_version_from_cliff_output(&output.stdout)
            .expect("Failed to get version from git-cliff output")
    }

    pub fn set_version(&self, version: &str, push: bool) {
        let mut command = std::process::Command::new("cargo");

        command
            .current_dir(self.temp_dir.path())
            .arg("workspaces")
            .arg("version");

        if !push {
            command.arg("--no-git-push");
        }

        command
            .arg("--no-individual-tags")
            .arg("--message")
            .arg("chore: Release %v")
            .arg("--yes")
            .arg("custom")
            .arg(version.trim_start_matches('v'))
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    pub fn get_current_version_from_workspace_cargo_toml(&self) -> String {
        let content = self.read_file_content("Cargo.toml");
        holochain_release_util::utils::get_current_version_from_cargo_toml(&content)
            .expect("Failed to get current version from Cargo.toml")
    }

    pub fn get_current_version_from_git_cliff(
        &self,
        changelog_config: ChangelogConfig,
        force_tag: Option<String>,
    ) -> String {
        let configure_command = |command: &mut std::process::Command| {
            command
                .current_dir(self.temp_dir.path())
                .arg("--config")
                .arg(changelog_config.path())
                .arg("--use-branch-tags")
                .arg("--latest");

            if let Some(tag) = &force_tag {
                if !tag.contains("-dev") {
                    command.arg("--tag-pattern").arg("^v\\d+.\\d+.\\d+$");
                }
            }
        };

        let mut command = std::process::Command::new("git-cliff");
        configure_command(&mut command);

        let output = command
            .arg("--context")
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();

        let value = serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();

        value
            .as_array()
            .unwrap()
            .first()
            .unwrap()
            .as_object()
            .unwrap()
            .get("version")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    pub fn run_semver_checks(&self, against_revision: &str) -> bool {
        let status = std::process::Command::new("cargo")
            .current_dir(self.temp_dir.path())
            .arg("semver-checks")
            .arg("--workspace")
            .arg("--baseline-rev")
            .arg(against_revision)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        status.success()
    }

    pub fn publish(&self) {
        std::process::Command::new("cargo")
            .current_dir(self.temp_dir.path())
            .arg("workspaces")
            .arg("publish")
            .arg("--allow-branch")
            .arg("main*")
            .arg("--publish-as-is")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    pub fn run_prepare_release(
        &self,
        changelog_config: ChangelogConfig,
        force_version: Option<String>,
    ) {
        let cliff_config = std::env::current_dir()
            .unwrap()
            .join(changelog_config.path())
            .to_str()
            .unwrap()
            .to_string();

        prepare_release(self.temp_dir.path(), cliff_config, force_version).unwrap();
    }

    pub fn run_publish_release(&self) {
        publish_release(self.temp_dir.path(), git_token(), true, true).unwrap();
    }

    /// Retain the temporary directory and print its path.
    ///
    /// Useful for debugging the state of the repository after tests. Alternatively, you can see
    /// the state of the repository on Gitea.
    pub fn retain(self) {
        let path = self.temp_dir.keep();
        println!("Repository retained at: {}", path.display());
    }

    fn add_crate_src_at_path(&self, crate_model: &CrateModel, path: impl AsRef<Path>) {
        let source_path = path.as_ref().join("src");
        let entry_path = if crate_model.binary {
            source_path.join("main.rs")
        } else {
            source_path.join("lib.rs")
        };

        if let Some(content) = &crate_model.content {
            self.write_file_content(entry_path.to_str().unwrap(), content);
        } else if crate_model.binary {
            self.write_file_content(
                entry_path.to_str().unwrap(),
                "fn main() { println!(\"hello binary\"); }",
            );
        } else {
            self.write_file_content(
                entry_path.to_str().unwrap(),
                "// This is a placeholder for the crate content",
            );
        }
    }

    fn make_cb<'a>() -> RemoteCallbacks<'a> {
        let mut cb = RemoteCallbacks::new();
        cb.credentials(|url, username, allowed_types| {
            println!(
                "Server wants {:?} for user {:?} to url {}",
                allowed_types, username, url
            );
            let token = git_token();
            let created = git2::Cred::userpass_plaintext("gituser", token.as_str())?;
            Ok(created)
        });
        cb
    }
}

pub struct CrateModel {
    pub name: String,
    pub version: String,
    pub binary: bool,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub content: Option<String>,
}

impl CrateModel {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            binary: true,
            description: None,
            repository: None,
            license: None,
            content: None,
        }
    }

    pub fn make_lib(mut self) -> Self {
        self.binary = false;
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn with_repository(mut self, repository: &str) -> Self {
        self.repository = Some(repository.to_string());
        self
    }

    pub fn with_license(mut self, license: &str) -> Self {
        self.license = Some(license.to_string());
        self
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.content = Some(content.to_string());
        self
    }
}

#[derive(Default)]
pub struct CargoWorkspaceModel {
    crates: Vec<(CrateModel, Vec<String>)>,
}

impl CargoWorkspaceModel {
    pub fn add_crate(mut self, crate_model: CrateModel, workspace_dependencies: &[&str]) -> Self {
        self.crates.push((
            crate_model,
            workspace_dependencies
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>(),
        ));
        self
    }
}
