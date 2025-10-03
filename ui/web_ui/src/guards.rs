// web_server/src/guards.rs
use lifestats_core::app_context::AppContext;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use std::sync::Arc;

pub struct Context(pub Arc<AppContext>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Context {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let state = request.rocket().state::<Context>().unwrap();
        Outcome::Success(Context(state.0.clone()))
    }
}
