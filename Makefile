default:

.PHONY: preview
preview: doc/preview.gif

doc/preview.gif: doc/preview.tape doc/docker-compose.yaml target/release/doggy
	docker-compose -f doc/docker-compose.yaml run vhs ./doc/preview.tape

