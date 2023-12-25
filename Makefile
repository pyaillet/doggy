default:

.PHONY: preview
preview: doc/preview.gif

doc/preview.gif: doc/preview.tape doc/docker-compose.yaml target/x86_64-unknown-linux-gnu/release/doggy
	docker compose -f doc/docker-compose.yaml run --build vhs ./doc/preview.tape

target/x86_64-unknown-linux-gnu/release/doggy: src/*.rs src/components/*.rs
	RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-gnu

.PHONY: tracing
tracing:
	docker compose -f tracing/docker-compose.yaml up -d

