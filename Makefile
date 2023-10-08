NODE_MODULES=node_modules/.package-lock.json
$(NODE_MODULES): package-lock.json
	npm i

.PHONY: check
check: $(NODE_MODULES)
	cargo clippy --all
	npm run --workspaces --if-present check

runtime: $(NODE_MODULES)
	npm run -w @tangramdotdev/runtime build

lsp: $(NODE_MODULES)
	npm run -w @tangramdotdev/lsp build
