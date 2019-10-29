use http::{Method, StatusCode};
use lambda_http::{Body, IntoResponse, Request, RequestExt, Response};
use lambda_runtime::{error::HandlerError, Context};

pub enum DispatchResult {
    IllegalMethod,
    UnknownPath,
    Set,
    Get,
}

impl IntoResponse for DispatchResult {
    fn into_response(self) -> Response<Body> {
        match self {
            Self::IllegalMethod => Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::Empty) // TODO: add helpful body text
                .unwrap(),
            Self::UnknownPath => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::Empty) // TODO: add helpful body text
                .unwrap(),
            _ => unimplemented!(),
        }
    }
}

// we do this manually here instead of using a framework like Rocket or Warp
// because the use case here is extremely simple, and it's faster to do it
// by hand than to integrate more libraries.
//
// That said, it's obviously unusual to match first on the request method and
// only then to attempt to get the path. If we wanted to expand on this software,
// adding a real server dispatcher should be a high priority.
//
/// distribute requests according to their method and path
///
/// we handle only two functions:
///
///   GET /{table}/{key}
///   SET /{table}/{key}
pub fn dispatch(request: Request, ctx: Context) -> Result<impl IntoResponse, HandlerError> {
    match request.method() {
        &Method::GET => get(request, ctx),
        &Method::POST => set(request, ctx),
        _ => Ok(DispatchResult::IllegalMethod),
    }
}

/// get an API key from a particular table and return it
fn get(request: Request, _ctx: Context) -> Result<DispatchResult, HandlerError> {
    unimplemented!()
}

/// set a key to the provided byte stream
fn set(request: Request, _ctx: Context) -> Result<DispatchResult, HandlerError> {
    unimplemented!()
}
