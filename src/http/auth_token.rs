use poem::{
    http::{header::AUTHORIZATION, StatusCode},
    FromRequest,
};

pub struct AuthToken(pub String);

impl<'a> FromRequest<'a> for AuthToken {
    fn from_request(
        req: &'a poem::Request,
        _body: &mut poem::RequestBody,
    ) -> impl std::future::Future<Output = poem::Result<Self>> + Send {
        Self::from_request_without_body(req)
    }
    fn from_request_without_body(
        req: &'a poem::Request,
    ) -> impl std::future::Future<Output = poem::Result<Self>> + Send {
        async move {
            log::info!("[AuthToken] check headers {:?}", req.headers());
            if let Some(header) = req.header(AUTHORIZATION) {
                return Ok(AuthToken(header.to_string()));
            }
            Err(poem::Error::from_string(
                "Missing AUTHORIZATION",
                StatusCode::UNAUTHORIZED,
            ))
        }
    }
}
