## Single use token lifecycle

Assumptions: AssetID is in Commit state, tokens might be created only by asset_states.authorized_signers

1. Venue (or other authorized_signers) issues Single-use Tokens via RPC `issue_tokens(asset_id, token_ids)` signed by pubkeyS
2. Validation node accepting call will validate and return array of token_ids for RPC call, tokens are created in `Prepare` state
3. Committee would validate `pubkeyS` is one of `asset_states.authorized_signers` and move tokens to `Commit` stage
4. User obtains token, Application signs by pubkeyS and issues RPC `transfer_token(token_id, user_pubkey)`
5. Validation node accepting call will validate pubkeyS matching authorized_signers for asset_id, pick or create temporary_wallet_key, find not assigned token, moves token to `Prepare` state
6. Validation node committee would reach consensus on PreCommit state, validating `pubkeyS`... and will move token to `Commit` state writing down new token's hash in checkpoint summary
7. User checks in with token, Application issues `use_token(token_id, user_key)` call
8. Validator node will ensure that `pubkeyS` is one of `authorized_signers` and `token.json.owner` is `user_key`, add to token's json `{token_used}` flag, moving it to `Prepare` state
9. Validation node committee would validate `pubkeyS` and token integrity and move token to `Commit` state

### Actix implementation:

1. Middleware - actix middleware - generic HTTP handler, will extract pubkey (from auth token?), validate that token is signed with pubkey and add pubkey to Context
2. Endpoint Handler - actix handler on routes `/asset_call/:id` and `/token_call/:id` - find matching templateID, decode RPC method signature and params, call matching trait implementation `asset_call(..)` or `token_call(..)`. Will pass context information, e.g. wallet pubkey, template data, request headers, JWT headers
3. Template router - template trait methods `asset_call` and `token_call` - will match procedure name passed as parameter to implementation, validate passed parameters correctness, can query any asset and token details within a given template and call procedure
4. Smart Contract initiation - procedure implementation running on single node accepting RPC - validates call parameters and amends `asset` and/or `tokens` correspondingly, moving to `Prepare` state
5. Smart Contract execution - procedure implementation running on comittee nodes moving item to `Committed` state initiated by serialized RPC signed by initiation node
