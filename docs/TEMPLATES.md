## Single use token example

Assumptions: AssetID is registered with creator_sig, tokens might be created only by creator matching pubkey

1. Venue (or asset owner) issues Single-use Tokens via RPC issue_single_use_token(asset_id, price_tari, issuer_sig, provider_pubkey, amount)
// provider_sig - is a pubkey which can later mark token as used
1. Validation node accepting call will validate call and return array of token_ids for RPC call, tokens are created in Prepare state
2. Validation node committee would validate asset_id belongs to venue validating that asset.creator_sig == token.issuer_sig and move token to Commit stage
3. User wants to buy token issuing RPC buy_token(asset_id, user_wallet_sig, price_tari)
4. Validation node accepting call will validate call and pick or create temporary wallet and any of not assigned tokens for asset_id, amend token adding temporary_wallet, user_wallet for token (all those fields are in additional json), and move token in Prepare state (OR should we rather tentatively close previous token record and create new in Prepare state)
5. Validation node committee would validate signature and wait for confirmed transfer for price_tari tari's to temporary_wallet, adding transfer transaction_id to token details and assigning token to user_wallet, moving token to PreCommit state
6. Validation node committee would reach consensus on precommit state, making sure that transaction id is valid, will move token to Commit state writing down new token's hash in checkpoint summary
7. User is using token issuing use_token(token_id, provider_sig, user_wallet_sign) call
8. Validator node will validate call and amend token with token_used and provider_sig flags moving it to Prepare state
9. Validation node committee would validate provider_sig and token integrity and move token to Commit state

## Layers

Validation node will support communication via HTTP allowing RPC calls, where RPC calls would vary depending on template. To achieve that Validation Node might use layered protocols architecture, where message consist of fixed set of layer's headers and wrapped message for the next layer.

### Proposed layers:
- Application layer - handle RPC calls
- Routing Layer - locate matching trait implementation based on TemplateID header and RPC method signature, compose and pass additional context information, e.g. wallet pubkey, template data, request headers - this will be actix handler for the rpc endpoint
- HTTP layer - provide generic endpoint to issue RPC calls, enrich context with information from JWT, next layer implementation might be chosen based on endpoint parameters (or maybe we would have single routing layer implementation) - this will be actix middleware


### Authorization by layers:
- Application layer - business logic authorization based on rpc calls params and context, e.g. given pubkey is authorized to issue tokens on given asset, based on asset details
- Routing layer - pubkey access to TemplateID and any other common validations based on information stored in database
- HTTP layer - pubkey access to API, JWT token integrity