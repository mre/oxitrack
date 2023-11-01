tailwind_cmd := "npx tailwindcss -i input.css -o static/main.css"
gzip_options := "-kf static/{logo.svg,main.css,{index,stats}.js{,.map}}"

build-static-dev:
	{{tailwind_cmd}}
	npx rspack --mode development
	gzip --fast {{gzip_options}}

alias r := run

# Run the binary
run: build-static-dev
	OXITRAFFIC_DATA_DIR=dev cargo r

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
	npx rspack
	gzip --best {{gzip_options}}
	cargo publish --allow-dirty
	git tag -a -m "release" "v$(cargo read-manifest | jaq -r '.version')"
	git push --follow-tags origin main
