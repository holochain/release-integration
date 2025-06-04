use git2::{IndexAddOption, ObjectType, RemoteCallbacks, Repository, RepositoryInitOptions};
use std::path::PathBuf;
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

#[derive(Default)]
pub enum BumpType {
    #[default]
    Auto,
    Minor,
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
        let mut index = repository.index().unwrap();
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

        let source_path = self.temp_dir.path().join("src");
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

    pub fn generate_changelog(
        &self,
        changelog_config: ChangelogConfig,
        bump_type: BumpType,
    ) -> String {
        let configure_command = |command: &mut std::process::Command| {
            command
                .current_dir(self.temp_dir.path())
                .arg("--config")
                .arg(changelog_config.path())
                .arg("--use-branch-tags");

            match bump_type {
                BumpType::Auto => command.arg("--bump"),
                BumpType::Minor => command.arg("--bumpminor").arg("minor"),
            };
        };

        let mut command = std::process::Command::new("git-cliff");
        configure_command(&mut command);

        let exit_status = command
            .arg("--output")
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
