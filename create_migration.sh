#!/bin/bash

docker run \
    --user $(id -u):$(id -g) \
    --rm \
    --volume /usr/share/zoneinfo:/usr/share/zoneinfo:ro \
    --volume $(pwd)/migrations:$(pwd)/migrations \
    migrate/migrate:v4.18.3 \
    create -dir $(pwd)/migrations -ext sql initial
