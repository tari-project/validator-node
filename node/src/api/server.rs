use crate::{
    api::{middleware::*, routing},
    config::NodeConfig,
    consensus::ConsensusProcessor,
    db::utils::db::build_pool,
    wallet::WalletStore,
};
use actix_cors::Cors;
use actix_web::{http, middleware::Logger, web, App, HttpResponse, HttpServer, Scope};
use serde_json::json;
use std::{net::ToSocketAddrs, sync::Arc, sync::mpsc};
use tokio::sync::Mutex;

// Must be valid JSON
const LOGGER_FORMAT: &'static str = r#"{"level": "INFO", "target":"api::request", "remote_ip":"%a", "user_agent": "%{User-Agent}i", "request": "%r", "uri": "%U", "status_code": %s, "response_time": %D, "api_version":"%{x-app-version}o", "client_version": "%{X-API-Client-Version}i" }"#;

pub async fn actix_main<F>(config: NodeConfig, scopes: F) -> anyhow::Result<()>
where F: (FnOnce() -> Vec<Scope>) + Clone + Send + 'static {
    let pool = Arc::new(build_pool(&config.postgres)?);
    let wallets = WalletStore::init(config.wallets_keys_path.clone())?;
    let wallets = Arc::new(Mutex::new(wallets));
    let config_arc = Arc::new(config.clone());

    println!(
        "Server starting at {}",
        config.actix.addr().to_socket_addrs()?.next().unwrap()
    );

    let mut consensus_processor = ConsensusProcessor::new(config.clone());
    let (kill_sender, kill_receiver) = mpsc::channel::<()>();
    // TODO: spawn consensus processors in separate Runtime
    actix_rt::spawn(async move {
        consensus_processor.start(kill_receiver).await;
    });

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
        let with_templates = scopes.clone()().into_iter().fold(app, |app, scope| {
            app.service(
                scope
                //TODO: abstract this configuration, make it reusable in tests too
                    .app_data(pool.clone())
                    .app_data(config_arc.clone())
                    .app_data(wallets.clone()),
            )
        });

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
    kill_sender.send(())?;

    Ok(())
}
