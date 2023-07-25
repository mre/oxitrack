tailwind_bin := "npx tailwindcss"

tailwind:
	{{tailwind_bin}} -mw -i input.css -o static/main.css
