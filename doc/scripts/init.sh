#!/bin/sh

docker context create dind \
	--description "dind container" \
	--docker "host=tcp://dind-rootless:2376,ca=/certs/client/ca.pem,cert=/certs/client/cert.pem,key=/certs/client/key.pem"

export DOCKER_CONTEXT=dind
docker container run -d --name d1 ubuntu:22.04 sleep infinity
docker container run -d --name d2 ubuntu:22.04 sleep infinity
docker container run -d whoami:v1.10
docker container run -d -p 80:80 nginx:stable

docker container run alpine:edge bash

docker image pull mysql:8.2.0
docker image pull debian:bookworm

docker-compose -f /scripts/docker-compose.yaml up -d

docker container ls

