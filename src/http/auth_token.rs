use poem::{
    http::{header::AUTHORIZATION, StatusCode},
    FromRequest,
};

pub struct AuthToken(pub Option<String>);

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
            let token = req.header(AUTHORIZATION).map(|h| h.to_owned());
            Ok(AuthToken(token))
        }
    }
}
