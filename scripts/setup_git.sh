#!/usr/bin/env sh

set -eu

script_dir=$(dirname "$0")

rm "$script_dir/git_test_token.txt" || true

docker compose exec --user 1000:1000 gitea \
  gitea admin user create --admin --username gituser --password pass --email gituser@holochain.org

docker compose exec --user 1000:1000 gitea \
  gitea admin user generate-access-token --username gituser --token-name test_token2 --scopes write:repository,read:user --raw > "$script_dir/git_test_token.txt"
