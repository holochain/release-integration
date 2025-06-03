#!/usr/bin/env sh

set -eu

script_dir=$(dirname "$0")

rm "$script_dir/git_test_token.txt" || true

docker compose exec --user 1000:1000 gitea \
  gitea admin user create --admin --username gituser --password pass --email gituser@holochain.org

docker compose exec --user 1000:1000 gitea \
  gitea admin user generate-access-token --username gituser --token-name test_token --scopes write:repository,read:user --raw > "$script_dir/git_test_token.txt"

docker compose cp gitea:/data/gitea/conf/app.ini "$script_dir/app.ini"
initool set "$script_dir/app.ini" repository ENABLE_PUSH_CREATE_USER true > "$script_dir/app.ini.new" \
  && mv "$script_dir/app.ini.new" "$script_dir/app.ini"

docker compose cp "$script_dir/app.ini" gitea:/data/gitea/conf/app.ini

rm "$script_dir/app.ini"

docker compose restart gitea
