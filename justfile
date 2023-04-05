# List all recipes.
default:
  @just --list

# Build dev WASM.
build-dev-wasm:
  cargo build --target wasm32-unknown-unknown --profile dev

# Build release WASM.
build-release-wasm:
  cargo build --target wasm32-unknown-unknown --profile release-wasm
  wasm-bindgen --out-dir ./out --target web ./target/wasm32-unknown-unknown/release-wasm/rust-boids.wasm
  tar czf rust-boids-wasm.tar.gz --directory=./out .
  mv rust-boids-wasm.tar.gz ./out

# Run dev WASM.
run-dev-wasm:
  cargo run --target wasm32-unknown-unknown --profile dev

# Run release WASM.
run-release-wasm:
  cargo run --target wasm32-unknown-unknown --profile release-wasm
