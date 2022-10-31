#!/usr/bin/env bash

__tg_alias_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." >/dev/null 2>&1 && pwd )"


if [[ "$#" == 1 && "$1" == "--no-build" ]]; then
	alias tg="$__tg_alias_dir/target/debug/tg"
else
	# Run function in subshell
	__tg_alias_run() (
		set -e
		(cd "$__tg_alias_dir"; cargo build --package tangram_cli --bin tg --quiet)
		"$__tg_alias_dir/target/debug/tg" "$@"
	)

	alias tg=__tg_alias_run
fi

