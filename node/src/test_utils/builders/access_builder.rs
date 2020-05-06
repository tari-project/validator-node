use crate::db::models::*;
use rand::prelude::*;
use tokio_postgres::Client;

#[allow(dead_code)]
pub struct AccessBuilder {
    pub pub_key: String,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for AccessBuilder {
    fn default() -> Self {
        let x: u32 = random();
        Self {
            pub_key: format!("7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0{}", x).into(),
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl AccessBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<Access> {
        let params = NewAccess {
            pub_key: self.pub_key.to_owned(),
        };
        Access::grant(params, client).await?;

        let query = SelectAccess {
            id: None,
            pub_key: Some(self.pub_key.to_owned()),
            include_deleted: None,
        };
        Ok(Access::select(query.clone(), client).await?.pop().unwrap())
    }
}
