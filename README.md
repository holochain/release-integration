# release-integration

Integration of third-party release tools with Holochain repositories

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

