# RPC calls authorization

## Participants

- Validation Node owner - provides validation CPU power and collateral
- Application owner - provides valuable service to user, using TemplateID smart contract
- User - customer of services, considering that user also has tari wallet

## Authorization approach for stage 1

1. Validation node owner creates wallet, then grants access to that wallet for some pubkeyA (application owner), considering that validation node can process TemplateID contracts for application
2. Application will accept and validate all user requests, whenever call needs to be made to smart contract it will sign with its own pubkeyA and pass to predefined Validation node
3. Validation Node will validate that request is signed with private key corresponding to pubkeyA
4. Asset RPC calls permissions granted via asset.authorized_signers and validated on RPC layer
5. Token level permissions validated on RPC layer

## Scaling to distributed RPC calls

1. Validation node owner creates wallet, then grants access to that wallet for some pubkeyA (application owner), considering that validation node can process TemplateID contracts.
2. Application owner (pubkeyA) grants access-token for a user (pubkeyU) to access given TemplateID
3. User would need to obtain access token from application owner (e.g. access token {pubkeyU; pubkeyA; templateID})
4. User would trigger RPC call supplying JWT token with access-token field (that whole token should be signed and short living to ensure authenticity). JWT token will be recognized on any node where pubkeyA is registered.

## Benefits of layered authorization

In general it's possible to perform authorization on application layer only, as that's the only layer which has all the context and business information. Though for production ready version common auth checks, like JWT token integrity validation, might be performed on early stage. Doing simple integrity checks earlier helps to reuse those checks between trait implementations, also minimizing resource usage by malicious callers. With layered protocol architecture (see [TEMPLATES.md](./TEMPLATES.md)) each layer can only perform checks based on information available to that layer.
