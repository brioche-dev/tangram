check:
	cargo clippy --all
	npm run --workspaces --if-present check

clean:
	umount ~/.tangram/artifacts; rm -rf ~/.tangram

lsp:
	npm run -w @tangramdotdev/lsp build

runtime:
	npm run -w @tangramdotdev/runtime build

serve_dev:
	cargo run -- serve
