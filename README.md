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

### Logging config ~/.tari/log4rs.yml

See example [config/log4rs.yml.example](config/log4rs.yml.example)

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
cargo run -- instruction status <instruction-id>
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
cargo run -- wallet create "animo assets"
cargo run -- wallet list
cargo run -- wallet view <pubkey>
cargo run -- wallet topup <pubkey> <amount>
```

### Wallet Access management
```
cargo run -- access grant wallet --pubkey XXX --wallet XXX
cargo run -- access revoke wallet --pubkey XXX --wallet XXX
cargo run -- access list
```


### Single Use Token example:

```
> tvnc asset create 1.0 "Kyiv Barbarian Pub" --issuer issuer_key
Asset created! Details:
...
asset_id                  "0000000100000000000000000000000.00000000000000000000000000000F1E"

> tvnc instruction asset 0000000100000000000000000000000.00000000000000000000000000000F1E issue_tokens '{"amount":10}'
Root Id                                   Status     Params
 **  77674150-a4d1-11ea-8034-000102030405 Pending    {"IssueTokens":{"amount":10,"token_ids":null}}

> tvnc token list 0000000100000000000000000000000.00000000000000000000000000000C77
Id                                                                                               IssueNumber          Status
0000000100000000000000000000000.00000000000000000000000000000F1E776DD2D6A4D111EA8035000102030405 1                    Available
...

> tvnc instruction token 0000000100000000000000000000000.00000000000000000000000000000F1E776DD2D6A4D111EA8035000102030405 sell_token '{"price": 1, "user_pubkey": "new_owner", "timeout_secs": 300}'
Root Id                                   Status     Params
 **  c63f6d5c-a4d1-11ea-8040-000102030405 Processing {"SellToken":{"price":1,"timeout_secs":300,"user_pubkey":"new_owner"}}
     c649f5ba-a4d1-11ea-8041-000000000000 Pending    {"SellTokenLock":{"wallet_key":"100cf9ffe39a5c7b6201910ace22e4a1d0e6bd22ab59f616a2d18cdfe8ea2b4e"}}

> tvnc instruction  status c63f6d5c-a4d1-11ea-8040-000102030405
Root Id                                   Status     Params
 **  c63f6d5c-a4d1-11ea-8040-000102030405 Processing {"SellToken":{"price":1,"timeout_secs":300,"user_pubkey":"new_owner"}}
     c649f5ba-a4d1-11ea-8041-000000000000 Commit     {"SellTokenLock":{"wallet_key":"100cf9ffe39a5c7b6201910ace22e4a1d0e6bd22ab59f616a2d18cdfe8ea2b4e"}}

> tvnc wallet balance 100cf9ffe39a5c7b6201910ace22e4a1d0e6bd22ab59f616a2d18cdfe8ea2b4e 1
Field                     Value
balance                   1
...

> tvnc instruction  status c63f6d5c-a4d1-11ea-8040-000102030405
Root Id                                   Status     Params
 **  c63f6d5c-a4d1-11ea-8040-000102030405 Commit     {"SellToken":{"price":1,"timeout_secs":300,"user_pubkey":"new_owner"}}
     c649f5ba-a4d1-11ea-8041-000000000000 Commit     {"SellTokenLock":{"wallet_key":"100cf9ffe39a5c7b6201910ace22e4a1d0e6bd22ab59f616a2d18cdfe8ea2b4e"}}

> tvnc instruction token redeem 0000000100000000000000000000000.00000000000000000000000000000F1E776DD2D6A4D111EA8035000102030405 'null'
...

> tvnc token view 0000000100000000000000000000000.00000000000000000000000000000F1E776DD2D6A4D111EA8035000102030405
Field                     Value
additional_data_json      {"owner_pubkey":"issuer_key","used":true}

```
