use crate::api::errors::ApiError;
use actix_web::{web::Data, HttpResponse};
use deadpool::Status as DeadpoolStatus;
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct Status {
    pub max_size: usize,
    pub size: usize,
    pub available: isize,
}

impl From<DeadpoolStatus> for Status {
    fn from(deadpool_status: DeadpoolStatus) -> Status {
        Status {
            max_size: deadpool_status.max_size,
            size: deadpool_status.size,
            available: deadpool_status.available,
        }
    }
}

pub async fn check(db: Data<Pool>) -> Result<HttpResponse, ApiError> {
    let status: Status = db.status().into();
    Ok(HttpResponse::Ok().json(json!(status)))
}
