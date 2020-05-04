# Tari Validator Node


## Cheatsheet

### Init DB
```
cargo run -- init
PG_DBNAME=validator_test cargo run -- init
```

### Toml config ~/.tari/config.toml
```
[validator]
postgres = { host = "localhost", user = "postgres", password = "password123" }
```

### Env vars overloading
- PG_DBNAME - database
- PG_USER - db user
- PG_PASSWORD - db password
- PG_HOST - db host
Tests expect same

### Migrate DBs
```
cargo run -- migrate
PG_DBNAME=validator_test cargo run -- migrate
```

### Reset DBs
```
cargo run -- wipe -y
PG_DBNAME=validator_test cargo run -- wipe
```

### Access management
```
cargo run -- access grant --pubkey XXX
cargo run -- access list
cargo run -- access revoke --pubkey XXX
cargo run -- access --help
```