set dotenv-load := true

run:
    dx serve

watch-css:
    npx tailwindcss -i ./main.css -o ./public/tailwind.css --watch
