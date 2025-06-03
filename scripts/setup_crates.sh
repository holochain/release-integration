#!/usr/bin/env sh

set -eu

script_dir=$(dirname "$0")

rm "$script_dir/session.json" || true
rm "$script_dir/crates_test_token.txt" || true

http POST http://localhost:8000/api/v1/user/login \
  --session="$script_dir/session.json" \
  Content-Type:application/json \
  user=admin \
  pwd=admin

http POST http://localhost:8000/api/v1/user/add_token \
  --session="$script_dir/session.json" \
  --body \
  Content-Type:application/json \
  name=test_token | jq -r '.token' > ./scripts/crates_test_token.txt

rm "$script_dir/session.json"
