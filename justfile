tailwind_cmd := "npx tailwindcss -m -i input.css -o static/main.css && gzip -kf --best static/main.css"
rspack_cmd := "npx rspack && gzip -kf --best static/stats.js{,.map}"

alias r := run

# Run the binary
run:
	OXITRAFFIC_DATA_DIR=dev cargo r

# Run tailwind in watch mode
tailwind:
	watchexec -r -w templates "{{tailwind_cmd}}"

# Run rspack in watch mode
rspack:
	watchexec -r -w ts "{{rspack_cmd}}"

# Initialize the project for development or compilation from source
init:
	npm install
	{{tailwind_cmd}}
	{{rspack_cmd}}

# Publish on crates.io
publish:
	npm outdated
	cargo outdated --exit-code 1
	typos
	{{tailwind_cmd}}
	{{rspack_cmd}}
	cargo sqlx prepare --check
	cargo test
	cargo publish --allow-dirty
	git push origin main
