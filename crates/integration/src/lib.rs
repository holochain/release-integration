use git2::{IndexAddOption, RemoteCallbacks, Repository, RepositoryInitOptions};
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

pub struct TestHarness {
    temp_dir: tempfile::TempDir,
    repository: Repository,
}

impl TestHarness {
    pub fn new(project_name: &str) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();

        let random_ext = nanoid::nanoid!(5);
        let origin_url = format!("http://localhost:3000/gituser/{project_name}-{random_ext}.git");
        let repository = Repository::init_opts(
            &temp_dir,
            &RepositoryInitOptions::new()
                .origin_url(origin_url.as_str())
                .initial_head("main"),
        )
        .unwrap();

        let mut config = repository.config().unwrap();
        config.set_str("user.name", "gituser").unwrap();
        config
            .set_str("user.email", "gituser@holochain.org")
            .unwrap();
        let mut index = repository.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let signature = repository.signature().unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "chore: Init",
                &repository.find_tree(tree_id).unwrap(),
                &[],
            )
            .unwrap();

        // Ensure that we aren't using any credential helper in this repository.
        // We want to authenticate with a username/token instead!
        let status = std::process::Command::new("git")
            .current_dir(temp_dir.path())
            .arg("config")
            .arg("credential.helper")
            .arg("")
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        assert!(status.success(), "Failed to set git credential helper");


        Self {
            temp_dir,
            repository,
        }
    }

    pub fn write_file_content(&self, relative_path: &str, content: &str) {
        let file_path = self.temp_dir.path().join(relative_path);
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(file_path, content).unwrap();
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

    pub fn push(&self, branch: &str) {
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
            .expect("Failed to push changes to remote");
    }
    
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
