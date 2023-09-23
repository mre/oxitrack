tailwind_cmd := "npx tailwindcss -m -i input.css -o static/main.css"

alias r := run

# Run the binary
run:
	OXITRAFFIC_DATA_DIR=dev cargo r

# Run tailwindcss in watch mode
tailwind:
	{{tailwind_cmd}} -w

# Run rspack in development and watch mode
rspack:
	npx rspack --mode development --watch

# Initialize the project for development or compilation from source
init:
	npm install
	{{tailwind_cmd}}
	npx rspack

# Publish on crates.io
publish:
	npm outdated
	cargo outdated --exit-code 1
	typos
	{{tailwind_cmd}}
	npx rspack
	cargo sqlx prepare --check
	cargo test
	cargo publish
	git push origin main
