use crate::{
    config::NodeConfig,
    db::{
        models::{Access, AccessResource, NewAccess, SelectAccess},
        utils::db::db_client,
    },
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum AccessCommands {
    /// Allow access for public key
    Grant(AccessType),
    /// List access tokens
    List,
    /// Revoke access for public key
    Revoke(AccessType),
}

#[derive(StructOpt, Debug)]
pub enum AccessType {
    /// Access to API
    Api {
        /// Public key of api user
        #[structopt(short = "k", long)]
        pubkey: String,
    },
    /// Access to Wallet funds
    Wallet {
        /// Public key of api user
        #[structopt(short = "k", long)]
        pubkey: String,
        /// Public key of a Wallet owned by a node
        #[structopt(short = "w", long)]
        wallet: String,
    },
}

impl AccessCommands {
    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let client = db_client(&node_config).await?;
        match self {
            Self::Grant(access_type) => {
                let updated = Access::grant(NewAccess::from(access_type), &client).await?;
                println!("Granted {}", updated);
            },
            Self::List => {
                let access = Access::select(SelectAccess::default(), &client).await?;
                for rec in access {
                    println!("{}", rec)
                }
            },
            Self::Revoke(access_type) => {
                let updated = Access::revoke(SelectAccess::from(access_type), &client).await?;
                println!("Revoked {}", updated);
            },
        };
        Ok(())
    }
}

impl From<AccessType> for NewAccess {
    fn from(access: AccessType) -> Self {
        match access {
            AccessType::Api { pubkey } => NewAccess {
                pub_key: pubkey,
                resource: AccessResource::Api,
                ..NewAccess::default()
            },
            AccessType::Wallet { pubkey, wallet } => NewAccess {
                pub_key: pubkey,
                resource: AccessResource::Wallet,
                resource_key: Some(wallet),
                ..NewAccess::default()
            },
        }
    }
}

impl From<AccessType> for SelectAccess {
    fn from(access: AccessType) -> Self {
        match access {
            AccessType::Api { pubkey } => SelectAccess {
                pub_key: Some(pubkey),
                resource: AccessResource::Api,
                ..SelectAccess::default()
            },
            AccessType::Wallet { pubkey, wallet } => SelectAccess {
                pub_key: Some(pubkey),
                resource: AccessResource::Wallet,
                resource_key: Some(wallet),
                ..SelectAccess::default()
            },
        }
    }
}
