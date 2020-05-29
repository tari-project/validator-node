## Pipeline

Template engine is responsible for reactive processing of instructions inline with pipeline:
![Instructions pipeline MVP](../instructions/pipeline.mmd.svg)

## Objects

- Template - Tari uses templates to define the behaviour for its smart contracts. Template is a super trait, which represents set of traits defining smart contracts as well as hashable identifier, so that it can be located.
- TemplateRunner - executor of template instructions, it crawls through queue of incoming instructions for templates
- TemplateContext - is execution context for the template
- AssetTemplateContext is execution context for asset, able to execute instruction on asset
- TokenTemplateContext is execution context for token, able to execute instruction on token

### Actix implementation:

1. Middleware - actix middleware - generic HTTP handler, will extract pubkey (from auth token?), validate that token is signed with pubkey and add pubkey to Context
2. Endpoint Handler - actix handler on routes `/asset_call/:id` and `/token_call/:id` - find matching templateID, decode RPC method signature and params, call matching trait implementation `asset_call(..)` or `token_call(..)`. Will pass context information, e.g. wallet pubkey, template data, request headers, JWT headers
3. Template router - template trait methods `asset_call` and `token_call` - will match procedure name passed as parameter to implementation, validate passed parameters correctness, can query any asset and token details within a given template and call procedure
4. Smart Contract initiation - procedure implementation running on single node accepting RPC - validates call parameters and amends `asset` and/or `tokens` correspondingly, moving to `Prepare` state
5. Smart Contract execution - procedure implementation running on comittee nodes moving item to `Committed` state initiated by serialized RPC signed by initiation node
