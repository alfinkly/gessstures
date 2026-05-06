.PHONY: serv front docker

serv:
	cargo run -p ws_bridge -- -n demo-notes

docker:
	docker compose up --build -d

down:
	docker compose down

front:
	cd frontend && rm -rf .next && npm run dev
