use crate::types::*;
use rand::prelude::random;

impl AssetID {
    pub fn test_from_template(template_id: TemplateID) -> Self {
        let hash = format!("{:032X}", random::<u32>());
        Self::new( template_id, 0, RaidID::default(), hash )
    }
}

impl TokenID {
    const TEST_NODE_ID: [u8; 6] = [0, 1, 2, 3, 4, 5];
    pub fn test_from_asset(asset_id: &AssetID) -> Self {
        TokenID::new(asset_id, Self::TEST_NODE_ID).unwrap()
    }
}