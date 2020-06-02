use structopt::StructOpt;

pub mod access;
pub use access::AccessCommands;
pub mod assets;
pub use assets::AssetCommands;
pub mod instructions;
pub use instructions::InstructionCommands;
pub mod templates;
pub use templates::TemplateCommands;
pub mod tokens;
pub use tokens::TokenCommands;
pub mod wallets;
pub use wallets::WalletCommands;

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
    /// Instruction commands
    Instruction(InstructionCommands),
    /// Token commands
    Token(TokenCommands),
    // TODO: Demo: cargo run  -- instruction asset 0000000100000000000000000000000.0000000000000000000000 issue_tokens
    // --data '{"number": 6}' TODO: Demo: cargo run  -- instruction token sell_token --data '{"owner_pubkey":
    // pubkey, "price": 100.0, "timeout": }' --autopick walletPubkey, token_id
    // TODO: Demo: cargo run  -- instruction wallet set_balance walletPubkey 100.0
    // TODO: Demo: cargo run  -- instruction token redeem_token
    // 0000000100000000000000000000000.0000000000000000000000.000000000..01
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
