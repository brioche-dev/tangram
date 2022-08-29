#!/usr/bin/env bash

# Get the directory of the tangram repo
__tg_alias_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." >/dev/null 2>&1 && pwd )"

# Run function in subshell
__tg_alias_run() (
	set -e
	(cd "$__tg_alias_dir"; cargo build --package tangram --bin tangram --quiet)
	"$__tg_alias_dir/target/debug/tangram" "$@"
)

alias tg=__tg_alias_run


