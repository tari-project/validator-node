use structopt::StructOpt;

pub mod access;
pub use access::AccessCommands;
pub mod wallets;
pub use wallets::WalletCommands;
pub mod templates;
pub use templates::TemplateCommands;
pub mod assets;
pub use assets::AssetCommands;

#[derive(StructOpt, Debug)]
pub enum Commands {
    /// Init configs and create the database, also running migrations
    Init,
    /// Start node
    Start,
    /// Run the migrations
    Migrate,
    /// API access management
    Access(AccessCommands),
    /// Manage wallets
    Wallet(WalletCommands),
    /// Work with template
    Template(TemplateCommands),
    /// Work with template
    Asset(AssetCommands),
    /// Recreate and migrate database,  *DANGER!* it will wipe all data
    Wipe {
        /// Don't prompt for confirmation
        #[structopt(short)]
        y: bool,
    },
}
impl Default for Commands {
    fn default() -> Self {
        Commands::Start
    }
}
