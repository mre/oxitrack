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
	npm outdated
	cargo outdated --exit-code 1
	typos
	{{tailwind_cmd}}
	cargo sqlx prepare --check
	cargo test
	cargo publish
	git push origin main
