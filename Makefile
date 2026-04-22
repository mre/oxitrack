SERVER_USER := root
SERVER_IP   := 46.225.7.147
REMOTE_DIR  := /data/coolify/applications/vg91n6ofqwle2ws39q8py1tt/data
TIMESTAMP   := $(shell date +%Y%m%d_%H%M%S)

.DEFAULT_GOAL := help

.PHONY: help run watch test build check deploy db-backup db-push logs r dev w t

help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} /^[a-zA-Z0-9_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

run: ## Run the server in development mode
	OXITRACK_CONFIG_FILE=dev/config.toml cargo run

watch: ## Run the server with auto-reload on source/template changes
	watchexec -nr -w src -w templates -w static make run

test: ## Run tests
	cargo test

build: ## Build release binary
	cargo build --release

check: ## Check for errors and warnings
	cargo clippy

deploy: ## Deploy by pushing to main — Coolify auto-deploys on git push
	git push

db-backup: ## Pull the live database from the server to backups/
	mkdir -p backups
	scp $(SERVER_USER)@$(SERVER_IP):$(REMOTE_DIR)/oxitrack.db backups/oxitrack_$(TIMESTAMP).db

db-push: ## Push local oxitrack.db to the server (use carefully — overwrites live data)
	@echo "WARNING: this will overwrite the live database. Waiting 3s, press Ctrl-C to abort."
	sleep 3
	scp dev/oxitrack.db $(SERVER_USER)@$(SERVER_IP):$(REMOTE_DIR)/oxitrack.db

logs: ## Tail live container logs via SSH
	ssh $(SERVER_USER)@$(SERVER_IP) "docker logs -f --tail=50 $$(docker ps --filter name=oxitrack --format '{{.Names}}' | head -1)"

r: run
dev: run
w: watch
t: test