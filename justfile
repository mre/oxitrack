# Run `just --list` to see available recipes

server_user := "root"
server_ip   := "46.225.7.147"
remote_dir  := "/data/coolify/applications/oxytrack/data"

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

# Deploy by pushing to main — Coolify auto-deploys on git push
deploy:
	git push

# Pull the live database from the server to backups/
db-backup:
	mkdir -p backups
	scp {{server_user}}@{{server_ip}}:{{remote_dir}}/oxytrack.db backups/oxytrack_$(date +%Y%m%d_%H%M%S).db

# Push local oxytrack.db to the server (use carefully — overwrites live data)
db-push:
	@echo "WARNING: this will overwrite the live database. Waiting 3s, press Ctrl-C to abort."
	sleep 3
	scp dev/oxytrack.db {{server_user}}@{{server_ip}}:{{remote_dir}}/oxytrack.db

# Tail live container logs via SSH
logs:
	ssh {{server_user}}@{{server_ip}} "docker logs -f --tail=50 \$(docker ps --filter name=oxytrack --format '{{{{.Names}}}}' | head -1)"