# Run `just --list` to see available recipes

_default:
	just --list

# Compile everything from source (including static files). Requires `npm` and `gzip`. `cargo install oxitraffic` is more recommended and only requires Rust
build:
	npm install
	just _build-static-prod
	cargo build -r

# Initialize the project for development. Use `run` or `watch` afterwards
init: && _build-static-dev
	npm install

# Run the binary
run: _build-static-dev
	OXITRAFFIC_CONFIG_FILE=dev/config.toml cargo r

# Run the binary in watch mode
watch:
	watchexec -nr -w src -w templates -w ts just r

# Publish on crates.io
publish: _build-static-prod
	npm outdated
	cargo upgrades
	typos
	cargo sqlx prepare --check
	cargo test

	cargo publish --allow-dirty

	git tag -a -m "release" "v$(cargo read-manifest | jaq -r '.version')"
	git push --follow-tags origin main

	buildah build -t oxitraffic:latest .
	podman push localhost/oxitraffic:latest docker.io/mo8it/oxitraffic:v$(cargo read-manifest | jaq -r '.version')
	podman push localhost/oxitraffic:latest docker.io/mo8it/oxitraffic:latest

alias r := run
alias w := watch

tailwind_cmd := "npx tailwindcss -i input.css -o static/main.css"
gzip_args := "-kf static/{logo.svg,main.css,stats.js{,.map}}"

_build-static-dev:
	{{tailwind_cmd}}
	npx esbuild --bundle --sourcemap --outdir=static ts/stats.ts
	gzip --fast {{gzip_args}}

_build-static-prod:
	{{tailwind_cmd}} -m
	npx esbuild --bundle --sourcemap --minify --outdir=static ts/stats.ts
	gzip --best {{gzip_args}}
