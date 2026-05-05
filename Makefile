.PHONY: serv front docker db

db:
	docker compose up -d postgres

serv: db
	cargo run -p ws_bridge -- -n demo-notes

docker:
	docker compose up --build

front:
	cd frontend && rm -rf .next && npm run dev