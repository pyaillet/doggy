#!/bin/sh

docker context create dind \
	--description "dind container" \
	--docker "host=tcp://dind-rootless:2376,ca=/certs/client/ca.pem,cert=/certs/client/cert.pem,key=/certs/client/key.pem"

export DOCKER_CONTEXT=dind

docker compose -f /scripts/docker-compose.yaml down

docker container rm -f $(docker container ls -aq)


