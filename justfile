# Run `just --list` to see available recipes

_default:
	just --list
	
# Run the server in development mode
run:
	OXYTRACK_CONFIG_FILE=dev/config.toml cargo run

# Run the server with auto-reload on source/template changes
watch:
	watchexec -nr -w src -w templates -w static just run

# Run tests
test:
	cargo test

# Build release binary
build:
	cargo build --release

# Check for errors and warnings
check:
	cargo clippy

alias r := run
alias dev := run
alias w := watch
alias t := test