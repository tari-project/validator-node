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

### Template operations
```
cargo run -- template list
```

### Asset operations
```
cargo run -- asset list <template-id>
cargo run -- asset view <asset-id>
cargo run -- asset create <template-id> "asset name" --issuer pubkey
```

### Token operations
```
cargo run -- token list <asset-id>
cargo run -- token view <token-id>
```

### Instruction operations
```
cargo run -- instruction asset <asset-id> <contract-name> <data>
cargo run -- instruction token <token-id> <contract-name> <data>
cargo run -- instruction wallet <pubkey> <contract-name> <data>
```

### Api Access management
```
cargo run -- access grant api --pubkey XXX
cargo run -- access list
cargo run -- access revoke api --pubkey XXX
cargo run -- access --help
```

### Wallet operations
```
cargo run -- wallet create -n "animo assets"
cargo run -- wallet list
cargo run -- wallet info --pubkey XXX
```

### Wallet Access management
```
cargo run -- access grant wallet --pubkey XXX --wallet XXX
cargo run -- access revoke wallet --pubkey XXX --wallet XXX
cargo run -- access list
```
