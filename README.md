# Rust 🦀 and WebAssembly 🕸

## Tutorial Document

https://rustwasm.github.io/docs/book/game-of-life/debugging.html


### setup

```sh
cargo install wasm-pack
cargo install cargo-generate

curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
source ~/.bashrc
nvm install --lts
nvm use --lts
```

### Tips

Chromeの場合はキャッシュが効いて更新してもwasm関係が更新されないことがある。
Devtool -> Network -> Disable cacheをonにしてキャッシュを使わせないことで防げる
