// src/cors.rs
use rocket::http::Method;
use rocket::Config;
use rocket_cors::{AllowedHeaders, AllowedMethods, AllowedOrigins, Cors, CorsOptions, Error};

pub fn create_cors() -> Result<Cors, Error> {
    let allowed_origins =
        AllowedOrigins::some_regex(&["http://localhost.*", "chrome-extension://.*"]);

    let allowed_methods: AllowedMethods = vec![
        Method::Get,
        Method::Post,
        Method::Delete,
        Method::Options, // Important for CORS preflight
    ]
    .into_iter()
    .map(From::from)
    .collect();

    CorsOptions {
        allowed_origins,
        allowed_methods,
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept", "Content-Type"]),
        allow_credentials: true,
        expose_headers: ["Content-Type", "X-Custom-Header"]
            .iter()
            .map(|s| (*s).into())
            .collect(),
        max_age: Some(3600),
        ..Default::default()
    }
    .to_cors()
}
