.PHONY: serv front

serv:
	cargo run -p ws_bridge -- -n demo-notes

front:
	cd frontend && rm -rf .next && npm run dev