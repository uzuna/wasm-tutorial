
.PHONY: test
test:
	wasm-pack test --firefox --headless

.PHONY: build
build:
	wasm-pack build

.PHONY: serve
serve:
	cd www && npm run start
