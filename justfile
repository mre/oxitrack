tailwind_cmd := "npx tailwindcss -m -i input.css -o static/main.css"

npm-up:
	npm update
	cp node_modules/chart.js/dist/chart.umd.js static
	# Required by the zoom plugin
	cp node_modules/hammerjs/hammer.min.js static
	cp node_modules/chartjs-plugin-zoom/dist/chartjs-plugin-zoom.min.js static
	{{tailwind_cmd}}

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
