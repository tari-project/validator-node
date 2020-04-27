use super::config::NodeConfig;
use crate::db::pool::build_pool;
use actix_web::{web, App, HttpResponse, HttpServer};
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};

#[derive(Clone, Serialize, Deserialize)]
pub struct ActixConfig {
    pub host: IpAddr,
    pub port: u16,
    pub workers: Option<usize>,
}
impl Default for ActixConfig {
    fn default() -> Self {
        Self {
            host: [127, 0, 0, 1].into(),
            port: 3001,
            workers: None,
        }
    }
}
impl ActixConfig {
    fn addr(&self) -> impl ToSocketAddrs {
        (self.host, self.port)
    }
}

#[actix_rt::main]
pub async fn actix_main(config: NodeConfig) -> anyhow::Result<()> {
    let pool = web::Data::new(build_pool(config.clone()));

    println!(
        "Server starting at {}",
        config.actix.addr().to_socket_addrs()?.next().unwrap()
    );

    let server = HttpServer::new(move || {
        App::new().app_data(pool.clone()).service(
            web::resource("/status").to(|db: web::Data<Pool>| HttpResponse::Ok().body(format!("{:?}", db.status()))),
        )
    })
    .bind(config.actix.addr())?;
    match config.actix.workers {
        Some(workers) => server.workers(workers),
        None => server,
    }
    .run()
    .await?;

    Ok(())
}
