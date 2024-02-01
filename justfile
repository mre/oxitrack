tailwind_cmd := "npx tailwindcss -i input.css -o static/main.css"
gzip_args := "-kf static/{logo.svg,main.css,stats.js{,.map}}"

build-static-dev:
	{{tailwind_cmd}}
	npx esbuild --bundle --sourcemap --outdir=static ts/stats.ts
	gzip --fast {{gzip_args}}

alias r := run

# Run the binary
run: build-static-dev
	OXITRAFFIC_CONFIG_FILE=dev/config.toml cargo r

alias w := watch

# Run the binary in watch mode
watch:
	watchexec -nr -w src -w templates -w ts just r

# Initialize the project for development or compilation from source
init: && build-static-dev
	npm install

# Publish on crates.io
publish:
	npm outdated
	cargo outdated --exit-code 1
	typos
	cargo sqlx prepare --check
	cargo test

	{{tailwind_cmd}} -m
	npx esbuild --bundle --sourcemap --minify --outdir=static ts/stats.ts
	gzip --best {{gzip_args}}

	cargo publish --allow-dirty

	git tag -a -m "release" "v$(cargo read-manifest | jaq -r '.version')"
	git push --follow-tags origin main

	buildah build -t oxitraffic:latest .
	podman push localhost/oxitraffic:latest docker.io/mo8it/oxitraffic:v$(cargo read-manifest | jaq -r '.version')
	podman push localhost/oxitraffic:latest docker.io/mo8it/oxitraffic:latest
