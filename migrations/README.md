To create a migration:

```
docker run \
    --user $(id -u):$(id -g) \
    --rm \
    --volume /usr/share/zoneinfo:/usr/share/zoneinfo:ro \
    --volume $(pwd)/migrations:$(pwd)/migrations \
    migrate/migrate:4 \
    create -dir $(pwd)/migrations -ext sql my_migration_name
```

To run a migration:

```
docker run \
    --user $(id -u):$(id -g) \
    --rm \
    --volume /usr/share/zoneinfo:/usr/share/zoneinfo:ro \
    --volume $(pwd)/migrations:$(pwd)/migrations \
    migrate/migrate:4 \
    -source=file:/$(pwd)/migrations \
    -database "postgres://my_user@my_host:5432/expenses?password=my_password&sslmode=disable" \
    up
```


