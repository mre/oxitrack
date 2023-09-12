tailwind_cmd := "npx tailwindcss -m -i input.css -o static/main.css"

npm-up:
	npm update
	cp node_modules/chart.js/dist/chart.umd.js static
	cp node_modules/chartjs-adapter-date-fns/dist/chartjs-adapter-date-fns.bundle.min.js static
	{{tailwind_cmd}}

tailwind:
	{{tailwind_cmd}} -w

alias r := run

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
