#[derive(Contracts, Serialize, Deserialize, Clone)]
#[contracts(template="SingleUseTokenTemplate",token)]
pub enum TokenContracts {
    #[contract(method="sell_token")]
    SellToken(SellTokenParams),
    #[contract(method="sell_token_lock")]
    SellTokenLock(SellTokenLockParams),
    #[contract(method="transfer_token")]
    TransferToken(TransferTokenParams),
}
