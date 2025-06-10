#!/usr/bin/env sh

set -eu

script_dir=$(dirname "$0")

"$script_dir/setup_crates.sh"
"$script_dir/setup_git.sh"
