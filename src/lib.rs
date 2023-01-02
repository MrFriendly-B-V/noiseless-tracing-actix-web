use std::borrow::Cow;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{Error, ResponseError};
use actix_web::http::{Method, StatusCode, Version};
use tracing::Span;
use tracing_actix_web::RootSpanBuilder;

/// Adaptation of [tracing_actix_web::DefaultRootSpanBuilder] which provides less noisy output
#[derive(Clone, Copy)]
pub struct NoiselessRootSpanBuilder;

impl RootSpanBuilder for NoiselessRootSpanBuilder {
    #[allow(unused)] // They're not unused, CLion just doesnt know that
    fn on_request_start(request: &ServiceRequest) -> Span {
        let http_route: Cow<'static, str> = request
            .match_pattern()
            .map(Into::into)
            .unwrap_or_else(|| "default".into());
        let http_method = http_method_str(&request.method());
        let http_flavor = http_flavor(request.version());

        let span = ::tracing::span!(
                    ::tracing::Level::INFO,
                    "HTTP request",
                    http.method = %http_method,
                    http.route = %http_route,
                    http.flavor = %http_flavor,
                    http.status_code = ::tracing::field::Empty,
                    exception.message = ::tracing::field::Empty,
                    exception.details = ::tracing::field::Empty,
                );
        span
    }

    fn on_request_end<B: MessageBody>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        match &outcome {
            Ok(response) => {
                if let Some(error) = response.response().error() {
                    // use the status code already constructed for the outgoing HTTP response
                    handle_error(span, response.status(), error.as_response_error());
                } else {
                    let code: i32 = response.response().status().as_u16().into();
                    span.record("http.status_code", code);
                    span.record("otel.status_code", "OK");
                }
            }
            Err(error) => {
                let response_error = error.as_response_error();
                handle_error(span, response_error.status_code(), response_error);
            }
        };
    }
}

fn handle_error(span: Span, status_code: StatusCode, response_error: &dyn ResponseError) {
    // pre-formatting errors is a workaround for https://github.com/tokio-rs/tracing/issues/1565
    let display = format!("{response_error}");
    let debug = format!("{response_error:?}");
    span.record("exception.message", &tracing::field::display(display));
    span.record("exception.details", &tracing::field::display(debug));
    let code: i32 = status_code.as_u16().into();

    span.record("http.status_code", code);
}

fn http_flavor(version: Version) -> Cow<'static, str> {
    match version {
        Version::HTTP_09 => "0.9".into(),
        Version::HTTP_10 => "1.0".into(),
        Version::HTTP_11 => "1.1".into(),
        Version::HTTP_2 => "2.0".into(),
        Version::HTTP_3 => "3.0".into(),
        other => format!("{other:?}").into(),
    }
}

fn http_method_str(method: &Method) -> Cow<'static, str> {
    match method {
        &Method::OPTIONS => "OPTIONS".into(),
        &Method::GET => "GET".into(),
        &Method::POST => "POST".into(),
        &Method::PUT => "PUT".into(),
        &Method::DELETE => "DELETE".into(),
        &Method::HEAD => "HEAD".into(),
        &Method::TRACE => "TRACE".into(),
        &Method::CONNECT => "CONNECT".into(),
        &Method::PATCH => "PATCH".into(),
        other => other.to_string().into(),
    }
}