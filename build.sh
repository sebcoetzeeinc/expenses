#!/bin/bash

VERSION=$(awk -F'"' '/version = / {print $2}' Cargo.toml | head -n 1)
TAG="registry.sebastiancoetzee.com/expenses:$VERSION"

docker build -t $TAG .
docker image push $TAG
