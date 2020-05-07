## Single use token lifecycle

Assumptions: AssetID is in Commit state, tokens might be created only by asset_states.authorized_signers

1. Venue (or other authorized_signers) issues Single-use Tokens via RPC `asset_call("issue_tokens", asset_id, {amount, price})` signed by pubkeyS
2. Validation node accepting call will validate and return array of token_ids for RPC call, tokens are created in `Prepare` state
3. Committee would validate `pubkeyS` is one of `asset_states.authorized_signers` and move tokens to `Commit` stage
4. User wants to obtain token, Application signs by pubkeyS and issues RPC `asset_call("buy_token", asset_id, {timeout, user_wallet_key})`
5. 1. Validation node accepting call will validate pubkeyS matching authorized_signers for asset_id, pick or create temporary_wallet_key, find not assigned token
5. 2. Validation node will amend token's json adding `{operation: "buy", temporary_wallet, user_key}`, move token to `Prepare` state
- Q: We should keep old token's state somewhere for the case of rollback, probably we just keep 2 token records? Keeping this locked by DB transaction is dangerous.
5. 3. Validation node will wait for confirmed transfer for `price_tari` tari's to `temporary_wallet_key` OR `timeout`
5. 4. If enough tari appears in `temporary_wallet_key` token's json is amended with `{transaction_id, owner: user_key}`, also `operation: "buy"` flag is removed, moving token to `Prepare` state
6. Validation node committee would reach consensus on PreCommit state, validating `transaction_id`, `owner`, `pubkeyS` and move token to `Commit` state writing down new token's hash in checkpoint summary
7. User checks in with token, Application issues `token_call("use_token", token_id, {user_key})` call
8. Validator node will ensure that `pubkeyS` is one of `authorized_signers` and `token.json.owner` is `user_key`, add to token's json `{token_used}` flag, moving it to `Prepare` state
- Q: Again probably we should have 2 records here, one is current `Commit` state another is new `Prepare` state? Keeping this locked by DB transaction is dangerous.
9. Validation node committee would validate `pubkeyS` and token integrity and move token to `Commit` state

## Layers

As part of Template's smart contract Validation Node will allow RPC calls on existing assets and tokens in addition to `create_asset` and `migrate_asset`, where operations would vary depending on template. To achieve that Validation Node might use layered protocols architecture, where message consist of fixed set of layer's headers and wrapped message for the next layer.

### Proposed flow:
1. Middleware - actix middleware - generic HTTP handler, will extract pubkey (from auth token?), validate that token is signed with pubkey and add pubkey to Context
2. Endpoint Handler - actix handler on routes `/asset_call/:id` and `/token_call/:id` - find matching templateID, decode RPC method signature and params, call matching trait implementation `asset_call(..)` or `token_call(..)`. Will pass context information, e.g. wallet pubkey, template data, request headers, JWT headers
3. Template router - template trait methods `asset_call` and `token_call` - will match procedure name passed as parameter to implementation, validate passed parameters correctness, can query any asset and token details within a given template and call procedure
4. Smart Contract initiation - procedure implementation running on single node accepting RPC - validates call parameters and amends `asset` and/or `tokens` correspondingly, moving to `Prepare` state
5. Smart Contract execution - procedure implementation running on comittee nodes moving item to `Committed` state
- *Option 1*: initiated by a token or asset data stored in additional json, might need to wait for a given condition to happen, e.g. `buy` for specific amount to appear in a `wallet`
- *Option 2*: initiated by serialized RPC signed by initiation node
