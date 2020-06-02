use crate::types::*;
use rand::prelude::random;

#[derive(Clone)]
pub struct Test<T>(std::marker::PhantomData<T>);

impl Test<TemplateID> {
    /// Predefined TemplateID for tests
    pub fn new() -> TemplateID {
        65536.into()
    }
}

impl Test<AssetID> {
    /// Generate random test [AssetID] on [Test<TemplateID>]
    #[allow(dead_code)]
    pub fn new() -> AssetID {
        Self::from_template(Test::<TemplateID>::new())
    }

    /// Generate random test [AssetID] on provided TemplateID
    pub fn from_template(template_id: TemplateID) -> AssetID {
        let hash = format!("{:032X}", random::<u64>());
        AssetID::new(template_id, 0, Test::<RaidID>::new(), hash)
    }
}

impl Test<TokenID> {
    /// Generate random test [AssetID] on [Test<TemplateID>]
    #[allow(dead_code)]
    pub fn new() -> TokenID {
        Self::from_asset(&Test::<AssetID>::new())
    }

    pub fn from_asset(asset_id: &AssetID) -> TokenID {
        TokenID::new(asset_id, &Test::<NodeID>::new()).unwrap()
    }
}

impl Test<RaidID> {
    /// Generate random test [AssetID] on [Test<TemplateID>]
    pub fn new() -> RaidID {
        let raw = format!("{:015X}", random::<u32>());
        RaidID::from_base58(raw.as_str()).unwrap_or(RaidID::default())
    }
}

impl Test<NodeID> {
    pub fn new() -> NodeID {
        NodeID([0, 1, 2, 3, 4, 5])
    }
}

impl Test<InstructionID> {
    /// Generate new unique test instruction
    pub fn new() -> InstructionID {
        InstructionID::new(Test::<NodeID>::new()).unwrap()
    }
}

impl Test<Pubkey> {
    /// Generate new unique test instruction
    pub fn new() -> Pubkey {
        let x: u64 = random();
        format!("7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0{:X}", x).into()
    }
}

use tari_common::ConfigBootstrap;

lazy_static::lazy_static! {
    static ref BOOTSTRAP: ConfigBootstrap = ConfigBootstrap {
        base_path: Test::<TempDir>::get_path_buf(),
        ..Default::default()
    };
}

impl Test<ConfigBootstrap> {
    pub fn get() -> &'static ConfigBootstrap {
        &BOOTSTRAP
    }
}

use tari_test_utils::random::string;
use tempdir::TempDir;

lazy_static::lazy_static! {
    static ref TEMP_DIR: TempDir = TempDir::new(string(8).as_str()).unwrap();
}

impl Test<TempDir> {
    pub fn get_path_buf() -> std::path::PathBuf {
        TEMP_DIR.path().to_path_buf()
    }
}

use crate::template::Template;

#[derive(Clone)]
pub struct TestTemplate;
impl Template for TestTemplate {
    type AssetContracts = ();
    type TokenContracts = ();

    fn id() -> TemplateID {
        Test::<TemplateID>::new()
    }
}
