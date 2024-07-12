
WASM_DIR ?= wgol

.PHONY: test
test:
	make -C ${WASM_DIR} test

.PHONY: build
build:
	make -C ${WASM_DIR} build

.PHONY: npm-link
npm-link:
	make -C ${WASM_DIR} npm-link
	cd www && npm link wasm-game-of-life

.PHONY: serve
serve:
	cd web-server && cargo run
