check:
	cargo clippy --all
	npm run --workspaces --if-present check

clean:
	umount ~/.tangram/artifacts; rm -rf ~/.tangram

lsp:
	npm run -w @tangramdotdev/lsp build

orb_clean:
	orb sh -c "umount /home/$USER/.tangram/artifacts; rm -rf /home/$USER/.tangram;"

orb_serve_dev:
	cargo build --target aarch64-unknown-linux-gnu && orb sh -c "./target/aarch64-unknown-linux-gnu/debug/tg server run"

runtime:
	npm run -w @tangramdotdev/runtime build

serve_dev:
	TANGRAM_TRACING=tangram_http=info cargo run -- server run

tg +ARGS:
	cargo run -- {{ARGS}}
