use crate::{
    api::{middleware::*, routing},
    config::NodeConfig,
    consensus::ConsensusProcessor,
    db::utils::db::build_pool,
    metrics::Metrics,
    template::{actix_web_impl::ActixTemplate, single_use_tokens::SingleUseTokenTemplate, TemplateRunner},
};
use actix::Addr;
use actix_cors::Cors;
use actix_web::{http, middleware::Logger, web, App, HttpResponse, HttpServer};
use futures::{
    future::{select, Either},
    pin_mut,
};
use serde_json::json;
use std::{
    net::ToSocketAddrs,
    sync::{mpsc, Arc},
};
use tokio::sync::oneshot::Sender;

// Must be valid JSON
const LOGGER_FORMAT: &'static str = r#"{"level": "INFO", "target":"api::request", "remote_ip":"%a", "user_agent": "%{User-Agent}i", "request": "%r", "uri": "%U", "status_code": %s, "response_time": %D, "api_version":"%{x-app-version}o", "client_version": "%{X-API-Client-Version}i" }"#;

pub async fn actix_main(
    config: NodeConfig,
    metrics_addr: Option<Addr<Metrics>>,
    mut kill_console: Sender<()>,
) -> anyhow::Result<()>
{
    let pool = Arc::new(build_pool(&config.postgres)?);

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

    // TODO: so far predefined templates only... make templates runners configurable from main
    let sut_runner = TemplateRunner::<SingleUseTokenTemplate>::create(pool.clone(), config.clone(), metrics_addr);
    let sut_context = sut_runner.start();

    let cors_config = config.cors.clone();
    let mut server = HttpServer::new(move || {
        let app = App::new()
            .app_data(web::Data::new(pool.clone()))
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
            // TODO: Should we not be using a JWT but rather something more custom?
            //.wrap(Authentication::new())
            .wrap(AppVersionHeader::new());

        // the problem we solving here is for every template scope we need to install distinct app_data with DB pool
        // TODO: abstract this configuration, make it reusable in tests too
        let scopes = SingleUseTokenTemplate::actix_scopes();
        let with_templates = scopes
            .into_iter()
            .fold(app, |app, scope| app.service(scope.data(sut_context.clone())));

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

    let server = server.run();
    let console_closed_fut = kill_console.closed();
    pin_mut!(console_closed_fut);

    match select(server, console_closed_fut).await {
        Either::Left((Err(err), _)) => {
            log::error!("Actix web server exit with error: {}", err);
            let _ = kill_sender.send(());
            return Err(err)?;
        },
        Either::Left((Ok(_), _)) => {
            let _ = kill_sender.send(());
        },
        Either::Right((_, server)) => {
            server.stop(true).await;
            let _ = kill_sender.send(());
        },
    }

    Ok(())
}
