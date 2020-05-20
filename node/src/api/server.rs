use crate::{
    api::{middleware::*, routing},
    config::NodeConfig,
    db::utils::db::build_pool,
};
use actix_cors::Cors;
use actix_web::{http, middleware::Logger, web, App, HttpResponse, HttpServer, Scope};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};

pub const DEFAULT_PORT: u16 = 3001;
pub const DEFAULT_ADDR: Ipv4Addr = Ipv4Addr::LOCALHOST;

// Must be valid JSON
const LOGGER_FORMAT: &'static str = r#"{"level": "INFO", "target":"api::request", "remote_ip":"%a", "user_agent": "%{User-Agent}i", "request": "%r", "uri": "%U", "status_code": %s, "response_time": %D, "api_version":"%{x-app-version}o", "client_version": "%{X-API-Client-Version}i" }"#;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActixConfig {
    pub host: IpAddr,
    pub port: u16,
    pub workers: Option<usize>,
    pub backlog: Option<usize>,
    pub maxconn: Option<usize>,
}
impl Default for ActixConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_ADDR.into(),
            port: DEFAULT_PORT,
            workers: None,
            backlog: None,
            maxconn: None,
        }
    }
}
impl ActixConfig {
    fn addr(&self) -> impl ToSocketAddrs {
        (self.host, self.port)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: String,
}
impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: "*".to_string(),
        }
    }
}

pub async fn actix_main<F>(config: NodeConfig, scopes: F) -> anyhow::Result<()>
where F: (FnOnce() -> Vec<Scope>) + Clone + Send + 'static {
    let pool = web::Data::new(build_pool(&config.postgres)?);

    println!(
        "Server starting at {}",
        config.actix.addr().to_socket_addrs()?.next().unwrap()
    );

    let cors_config = config.cors.clone();
    let mut server = HttpServer::new(move || {
        let app = App::new()
            .app_data(pool.clone())
            .wrap({
                let mut cors = Cors::new();
                cors = match cors_config.allowed_origins.as_str() {
                    "*" => cors.send_wildcard(),
                    _ => cors.allowed_origin(&cors_config.allowed_origins),
                };
                cors.allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
                    .allowed_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::ACCEPT,
                        "X-API-Client-Version".parse::<http::header::HeaderName>().unwrap(),
                    ])
                    .allowed_header(http::header::CONTENT_TYPE)
                    .expose_headers(vec!["x-app-version"])
                    .max_age(3600)
                    .finish()
            })
            .wrap(Logger::new(LOGGER_FORMAT).exclude("/status"))
            .wrap(Authentication::new())
            .wrap(AppVersionHeader::new());

        // the problem we solving here is for every template scope we need to install distinct app_data with DB pool
        let with_templates = scopes.clone()()
            .into_iter()
            .fold(app, |app, scope| app.service(scope.app_data(pool.clone())));

        with_templates
            .configure(routing::routes)
            .default_service(web::get().to(|| HttpResponse::NotFound().json(json!({"error": "Not found"}))))
    })
    .bind(config.actix.addr())?;

    if let Some(workers) = config.actix.workers {
        server = server.workers(workers);
    }

    if let Some(backlog) = config.actix.backlog {
        server = server.backlog(backlog as i32);
    };

    if let Some(maxconn) = config.actix.maxconn {
        server = server.maxconn(maxconn);
    };

    server.run().await?;

    Ok(())
}
