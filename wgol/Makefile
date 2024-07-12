include ../common.mk

.PHONY: test
test:
	wasm-pack test --firefox --headless

.PHONY: build
build:
	wasm-pack build -d ${WWW_DIR}/wgol

.PHONY: npm-link
npm-link:
	cd pkg && npm link
