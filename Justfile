set dotenv-load := true

run:
    dx serve --hot-reload

watch-css:
    npx tailwindcss -i ./main.css -o ./public/tailwind.css --watch
