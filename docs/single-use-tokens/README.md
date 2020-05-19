# Single use tokens

Template ID | 1

## States

Single use tokens are analogue of tickets, which can be issued, sold, transferred and used. Each token state is token programmable state coupled with possible transaction-lock.

- Programmable states for Single Use Tokens:
![States diagram visualizes transitions](states-diagram.mmd.svg)

## Operations

### Issue tokens

1. Asset issuer or authorized signers (later `issuer`) calling `issue_tokens`
2. VN ensures signatures, data integrity and other criteria (TBD) are valid
3. Tokens created in `Active`(`Available`) token state

### Sell token

Is a multi-stage transaction involding locking token record until given conditions met.

1. Issuer is calling `sell_token`(`TokenID`, `price`, `new owner`)
2. Initiating VN validates token data integrity and starts `contract-transaction` in `Open` state to acquire `timed-lock`
  1. Creates temporary wallet (`tempWallet`) to accept payment
  2. Submits instruction for a `timed-lock` bound to `tempWallet` and `contract-transaction` to the committee
  3. Upon receipt of consensus from committee it will release `tempWallet` with signed consensus to client for payment
3. Initiating VN keeps monitoring `tempWallet`, once `price` amount appears:
  1. Initiating VN will create BL transactions to distribute tari's according to Asset settings
  2. Initiating VN will submit `contract-transaction` in `Commit` state moving token to `Active` state with `new owner`
4. VN committee will validate all conditions met and provide consensus resolution `contract-transaction` as `Commit`
5. If any of above stages fail all the tari transfers should be cancelled or reverted

#### Sell token sequence diagram option 1:
![sell token sequence option 1](sell-token-sequence-option-1.mmd.svg)

#### Sell token sequence diagram option 4:
![sell token sequence option 1](sell-token-sequence-option-4.mmd.svg)

### Transfer token

The flow is similar to [Selling tokens] except that it might be triggered by token owner:

1. Owner is calling `transfer_token`(`TokenID`, `price`, `new owner`)
2. Leader VN validates token data integrity and ownership and starts new `contract-transaction`:
... other steps same as [Sell token](Sell token)

### Redeem token and Unredeem token

Both are single-step transactions.

1. Issuer is calling `redeem_token`(`TokenID`)
2. VN will ensure token integrity and that one other open transactions involiving token [^1]
  1. Leader VN will transition [^1] token to `Active`(`Used`) state changing owner back to asset `issuer`
  2. Leader VN will move `contract-transaction` to `PreCommit` state and publish transaction to committee
3. VN committee validate all conditions and mark `contract-transaction` as `Commit`
