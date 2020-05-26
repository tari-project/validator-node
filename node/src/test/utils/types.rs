use crate::types::*;
use rand::prelude::random;

pub struct Test<T>(std::marker::PhantomData<T>);

impl Test<TemplateID> {
    /// Predefined TemplateID for tests
    pub fn new() -> TemplateID {
        65536.into()
    }
}

impl Test<AssetID> {
    /// Generate random test [AssetID] on [Test<TemplateID>]
    pub fn new() -> AssetID {
        Self::from_template(Test::<TemplateID>::new())
    }

    /// Generate random test [AssetID] on provided TemplateID
    pub fn from_template(template_id: TemplateID) -> AssetID {
        let hash = format!("{:032X}", random::<u32>());
        AssetID::new(template_id, 0, RaidID::default(), hash)
    }
}

impl Test<TokenID> {
    /// Generate random test [AssetID] on [Test<TemplateID>]
    pub fn new() -> TokenID {
        Self::from_asset(&Test::<AssetID>::new())
    }

    pub fn from_asset(asset_id: &AssetID) -> TokenID {
        TokenID::new(asset_id, &Test::<NodeID>::new()).unwrap()
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
