default:

.PHONY: preview
preview: doc/preview.gif

doc/preview.gif: prepare-preview doc/preview.tape doc/docker-compose.yaml src/*.rs src/components/*.rs
	docker compose -f doc/docker-compose.yaml run --build vhs ./doc/preview.tape

prepare-preview:
	docker compose -f doc/docker-compose.yaml up -d
	docker wait doc-docker-1

clean-preview:
	docker compose -f doc/docker-compose.yaml run --entrypoint /scripts/clean.sh docker
	docker compose -f doc/docker-compose.yaml down
	


.PHONY: tracing
tracing:
	docker compose -f tracing/docker-compose.yaml up -d

