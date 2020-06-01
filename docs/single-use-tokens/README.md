# Single use tokens

Template ID | 1

## States

Single use tokens are analogue of tickets, which can be issued, sold, transferred and used. Each token state is token programmable state coupled with possible transaction-lock.

- Programmable states for Single Use Tokens:
![States diagram visualizes transitions](states-diagram.mmd.svg)

## Operations

### Issue tokens

1. Asset issuer or authorized signers (later `issuer`) calling `issue_tokens`
2. VN ensures signatures, data integrity and creates Instruction `issue_tokens` in `Processing` state
3. Tokens created in `Active` token state, instruction moved to `Pending` state
4. Consensus reached, Instruction moved to `Commit` state

### Sell token

Is a multi-stage transaction involding locking token record until given conditions met.

1. Issuer is calling `sell_token`(`TokenID`, `price`, `new owner`, `timeout`)
2. Initiating VN validates token data integrity and creates `Instruction` in `Processing` state `sell-token`
3. Initiating VN validates creates sub-`Instruction` `sell-token-lock`
  1. Creates temporary wallet (`tempWallet`) to accept payment
  2. Upon receipt of consensus from committee `tempWallet` is available to client in `sell-token-lock` `Instruction` params
  3. Initiating VN keeps monitoring `tempWallet`, and once `price` amount appears:
  4. Initiating VN will create BL transactions to distribute tari's according to Asset settings
  5. Initiating VN will submit `sell-token` to `Pending` state, moving token to `Active` state with `new owner`
  6. If `timeout` reached before `Instruction` moves to `Invalid` state
4. VN committee will validate all conditions met and provide consensus resolution `contract-transaction` as `Commit`
5. If any of above stages fail all the tari transfers should be cancelled or reverted, instruction and subinstruction marked `Invalid`

#### Sell token sequence diagram:
![sell token sequence MVP](sell-token-sequence-mvp.mmd.svg)

### Transfer token

The flow is similar to [Selling tokens] except that it might be triggered by token owner:

1. Owner is calling `transfer_token`(`TokenID`, `new owner`)
2. VN ensures signatures, data integrity and creates Instruction `transfer_token` in `Processing` state
3. Tokens updated with `new owner`, instruction moved to `Pending` state
4. Consensus reached, Instruction moved to `Commit` state

### Redeem token and Unredeem (N/A yet) token

Both are single-step transactions.

1. Issuer is calling `redeem_token`(`TokenID`)
2. VN will ensure token integrity and create `redeem-token` Instruction in `Processing` state
2. VN will change token owner to asset `issuer` move Instruction to `Pending` state
3. VN committee validate all conditions and mark `redeem-token` as `Commit`
