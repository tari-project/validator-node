use crate::api::controllers::status;
use actix_web::web;

pub fn routes(app: &mut web::ServiceConfig) {
    // Please try to keep in alphabetical order
    app.service(web::resource("/status").route(web::get().to(status::check)));
}
