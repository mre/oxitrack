tailwind_bin := "npx tailwindcss"

tailwind:
	{{tailwind_bin}} -mw -i input.css -o static/main.css

run:
	OXITRAFFIC_DATA_DIR=dev cargo r

publish:
	cargo sqlx prepare
	cargo publish
