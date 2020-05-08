use crate::api::errors::{ApiError, *};
use actix_http::http::header::Header;
use actix_web::{dev, FromRequest, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use futures::future::{err, ok, Ready};
use jsonwebtoken::{dangerous_unsafe_decode, decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct AccessToken {
    pub sub: String,
    pub iss: String,
    pub exp: u64,
    pub issued: u64,
}

impl AccessToken {
    pub fn new(pubkey: String, issuer: String, expiry_in_minutes: i64) -> Self {
        let mut timer = SystemTime::now();
        timer += Duration::from_secs(expiry_in_minutes as u64 * 60);
        let exp = timer.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let issued = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        AccessToken {
            iss: issuer,
            sub: pubkey,
            exp,
            issued,
        }
    }
}

impl FromRequest for AccessToken {
    type Config = ();
    type Error = ApiError;
    type Future = Ready<Result<AccessToken, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        if let Ok(bearer_token) = Authorization::<Bearer>::parse(req) {
            let token = bearer_token.into_scheme();
            match dangerous_unsafe_decode::<AccessToken>(&token.token()) {
                Ok(token_without_verification) => {
                    match DecodingKey::from_rsa_pem(token_without_verification.claims.sub.as_bytes()) {
                        Ok(decoding_key) => {
                            match decode(&token.token(), &decoding_key, &Validation::new(Algorithm::RS512)) {
                                Ok(token) => ok(token.claims),
                                Err(_) => err(AuthError::unauthorized(
                                    "Invalid auth token: unable to verify signature",
                                )
                                .into()),
                            }
                        },
                        Err(_) => err(AuthError::unauthorized("Invalid auth token: invalid public key format").into()),
                    }
                },
                Err(_) => err(AuthError::unauthorized("Invalid auth token").into()),
            }
        } else {
            err(AuthError::unauthorized("Missing auth token").into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test::TestRequest;
    use jsonwebtoken::{encode, EncodingKey, Header as JwtHeader};

    #[actix_rt::test]
    async fn from_request() -> anyhow::Result<()> {
        let access_token = AccessToken::new(
            include_str!("../../test/data/example-public-key.pem").to_string(),
            "tari".to_string(),
            6000,
        );
        let token = encode(
            &JwtHeader::new(Algorithm::RS512),
            &access_token,
            &EncodingKey::from_rsa_pem(include_bytes!("../../test/data/example-private-key.pem")).unwrap(),
        )
        .unwrap();

        let request = TestRequest::with_header("authorization", format!("Bearer {}", token)).to_http_request();
        let access_token_from_request = AccessToken::from_request(&request, &mut dev::Payload::None).await?;
        assert_eq!(access_token_from_request, access_token);

        // Incorrectly signed JWT fails validation
        let token = encode(
            &JwtHeader::new(Algorithm::RS512),
            &access_token,
            &EncodingKey::from_rsa_pem(include_bytes!("../../test/data/example-private-key-invalid.pem")).unwrap(),
        )
        .unwrap();
        let request = TestRequest::with_header("authorization", format!("Bearer {}", token)).to_http_request();
        let response = AccessToken::from_request(&request, &mut dev::Payload::None).await;
        assert!(response.is_err());

        Ok(())
    }
}
