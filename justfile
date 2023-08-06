tailwind_cmd := "npx tailwindcss -m -i input.css -o static/main.css"

tailwind:
	{{tailwind_cmd}} -w

run:
	OXITRAFFIC_DATA_DIR=dev cargo r

publish:
	cargo outdated --exit-code 1
	typos
	cargo test
	cargo sqlx prepare --check
	{{tailwind_cmd}}
	git push origin main
	cargo publish
