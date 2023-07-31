tailwind_cmd := "npx tailwindcss -m -i input.css -o static/main.css"

tailwind:
	{{tailwind_cmd}} -w

run:
	OXITRAFFIC_DATA_DIR=dev cargo r

publish:
	typos
	cargo sqlx prepare --check
	{{tailwind_cmd}}
	git-cliff -o CHANGELOG.md
	git push origin main
	cargo publish
