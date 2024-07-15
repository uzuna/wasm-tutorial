# Rust 🦀 and WebAssembly 🕸

## Tutorial Document

https://rustwasm.github.io/docs/book/game-of-life/debugging.html


### setup and start

```sh
cargo install wasm-pack
cargo install cargo-generate

# terminal 1 start server
make serve

# terminal 2 build wasm
make build
```

### Tips

Chromeの場合はキャッシュが効いて更新してもwasm関係が更新されないことがある。
Devtool -> Network -> Disable cacheをonにしてキャッシュを使わせないことで防げる
