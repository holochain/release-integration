use git2::{
    BranchType, IndexAddOption, ObjectType, RemoteCallbacks, Repository, RepositoryInitOptions,
};
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
    temp_dir: tempfile::TempDir,
    repository: Repository,
}

impl TestHarness {
    pub fn new(project_name: &str) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();

        let random_ext = nanoid::nanoid!(5);
        let origin_url = format!("http://localhost:3000/gituser/{project_name}-{random_ext}.git");
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
        let signature = self.repository.signature().unwrap();
        let head = self.repository.head().unwrap();
        let commit = head.peel(ObjectType::Commit).unwrap();

        self.repository
            .tag(tag, &commit, &signature, message, false)
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
        let mut remote = self
            .repository
            .find_remote("origin")
            .expect("Failed to find remote 'origin'");

        let mut push_opts = git2::PushOptions::new();
        push_opts.remote_callbacks(Self::make_cb());

        remote
            .push(
                &[format!("refs/tags/{tag}:refs/tags/{tag}")],
                Some(&mut push_opts),
            )
            .expect("Failed to push tag to remote");
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

    pub fn add_standard_gitignore(&self) {
        self.write_file_content(
            ".gitignore",
            r#"target/
            "#,
        );

        self.commit(".gitignore", "chore: add standard .gitignore");
    }

    pub fn add_crate(&self, crate_model: CrateModel) {
        self.write_file_content(
            "Cargo.toml",
            &format!(
                r#"[package]
name = "{}"
version = "{}"
edition = "2024"
{}
{}
{}
        "#,
                crate_model.name,
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
"#,
                workspace_model
                    .crates
                    .iter()
                    .map(|c| format!("\"crates/{}\"", c.name))
                    .collect::<Vec<_>>()
                    .join(",\n    "),
                workspace_model
                    .crates
                    .first()
                    .as_ref()
                    .expect("No crates in workspace")
                    .version,
                if let Some(repository) = workspace_model
                    .crates
                    .first()
                    .as_ref()
                    .expect("No crates in workspace")
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
                    .license
                    .as_ref()
                {
                    format!("license = \"{}\"", license)
                } else {
                    String::new()
                },
            ),
        );

        for crate_model in &workspace_model.crates {
            self.write_file_content(
                &format!("crates/{}/Cargo.toml", crate_model.name),
                &format!(
                    r#"[package]
name = "{}"
{}
version.workspace = true
edition.workspace = true
{}
{}
        "#,
                    crate_model.name,
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
    pub crates: Vec<CrateModel>,
}

impl CargoWorkspaceModel {
    pub fn add_crate(mut self, crate_model: CrateModel) -> Self {
        self.crates.push(crate_model);
        self
    }
}
