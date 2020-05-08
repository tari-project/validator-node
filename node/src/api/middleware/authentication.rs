use crate::api::{errors::ApiError, models::AccessToken};
use actix_http::error::ResponseError;
use actix_service::Service;
use actix_web::{
    dev::{MessageBody, Payload, ServiceRequest, ServiceResponse, Transform},
    error,
    FromRequest,
    HttpRequest,
};
use futures::future::{ok, Ready};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

#[derive(Clone, Debug, PartialEq)]
pub struct AuthenticationContext {
    pubkey: String,
}

pub trait RequestAuthenticationContext {
    fn authentication_context(&self) -> Result<AuthenticationContext, ApiError>;
}

impl RequestAuthenticationContext for HttpRequest {
    fn authentication_context(&self) -> Result<AuthenticationContext, ApiError> {
        let access_token = AccessToken::from_request(&self, &mut Payload::None).into_inner()?;
        Ok(AuthenticationContext {
            pubkey: access_token.sub,
        })
    }
}

pub struct Authentication;

impl Authentication {
    pub fn new() -> Authentication {
        Authentication {}
    }
}

impl<S, B> Transform<S> for Authentication
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = error::Error> + 'static,
    B: MessageBody,
{
    type Error = S::Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type InitError = ();
    type Request = S::Request;
    type Response = S::Response;
    type Transform = AuthenticationService<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationService::new(service))
    }
}

#[derive(Clone)]
pub struct AuthenticationService<S> {
    service: Rc<RefCell<S>>,
}

impl<S> AuthenticationService<S> {
    fn new(service: S) -> Self {
        Self {
            service: Rc::new(RefCell::new(service)),
        }
    }
}

impl<S, B> Service for AuthenticationService<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = error::Error> + 'static,
    B: MessageBody,
{
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    type Request = S::Request;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(error::Error::from)
    }

    fn call(&mut self, request: Self::Request) -> Self::Future {
        let mut service = self.service.clone();

        // Ignore requests to the status endpoint
        if request.uri() == "/status" {
            Box::pin(async move { service.borrow_mut().call(request).await })
        } else {
            let (http_request, payload) = request.into_parts();

            let authentication_context: Result<AuthenticationContext, ApiError> = http_request.authentication_context();

            match authentication_context {
                Ok(authentication_context) => {
                    http_request.extensions_mut().insert(authentication_context);
                    let request = ServiceRequest::from_parts(http_request, payload)
                        .unwrap_or_else(|_| unreachable!("Failed to recompose request in AuthenticationService::call"));
                    Box::pin(async move { service.borrow_mut().call(request).await })
                },
                Err(error) => Box::pin(async move {
                    Ok(ServiceResponse::<B>::new(
                        http_request,
                        error.error_response().into_body(),
                    ))
                }),
            }
        }
    }
}
