run LOG_LEVEL="debug":
  RUST_LOG={{LOG_LEVEL}} cargo run

debug: (run "trace")

lint:
    cargo clippy --all-targets --all-features -- -Dwarnings
