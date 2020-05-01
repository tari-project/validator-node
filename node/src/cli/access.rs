use crate::{
    config::NodeConfig,
    db::{
        models::{Access, NewAccess, SelectAccess},
        pool::build_pool,
    },
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum AccessCommands {
    /// Allow access for public key
    Grant {
        /// Public key of a wallet or node
        #[structopt(short = "k", long)]
        pubkey: String,
    },
    /// List access tokens
    List,
    /// Revoke access for public key
    Revoke {
        /// Public key of a wallet or node
        #[structopt(short = "k", long)]
        pubkey: Option<String>,
    },
}

impl AccessCommands {
    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let db = build_pool(&node_config.postgres)?;
        let client = db.get().await?;

        match self {
            Self::Grant { pubkey } => {
                let updated = Access::grant(NewAccess { pub_key: pubkey }, &client).await?;
                println!("Granted {}", updated);
            },
            Self::List => {
                let access = Access::select(SelectAccess::default(), &client).await?;
                for rec in access {
                    println!("{}", rec)
                }
            },
            Self::Revoke { pubkey } => {
                let updated = Access::revoke(
                    SelectAccess {
                        id: None,
                        pub_key: pubkey,
                        include_deleted: None,
                    },
                    &client,
                )
                .await?;
                println!("Revoked {}", updated);
            },
        };
        Ok(())
    }
}
