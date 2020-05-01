use crate::db::models::*;
use rand::prelude::*;
use tokio_postgres::Client;

#[allow(dead_code)]
pub struct AccessBuilder<'a> {
    pub_key: String,
    client: &'a Client,
}

#[allow(dead_code)]
impl<'a> AccessBuilder<'a> {
    pub fn new(client: &'a Client) -> Self {
        let x: u32 = random();
        AccessBuilder {
            pub_key: format!("7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0{}", x).into(),
            client,
        }
    }

    pub fn with_pub_key(mut self, pub_key: String) -> Self {
        self.pub_key = pub_key;
        self
    }

    pub async fn finish(&self) -> anyhow::Result<Access> {
        let params = NewAccess {
            pub_key: self.pub_key.to_owned(),
        };
        Access::grant(params, self.client).await?;

        let query = SelectAccess {
            id: None,
            pub_key: Some(self.pub_key.to_owned()),
            include_deleted: None,
        };
        Ok(Access::select(query.clone(), self.client).await?.pop().unwrap())
    }
}
