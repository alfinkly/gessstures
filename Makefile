.PHONY: serv front docker superset

serv:
	cargo run -p ws_bridge -- -n demo-notes

docker:
	docker compose up --build -d

down:
	docker compose down

superset:
	cd superset && docker compose up -d

superset-down:
	cd superset && docker compose down

front:
	cd frontend && rm -rf .next && npm run dev
