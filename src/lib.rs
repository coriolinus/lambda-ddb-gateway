use http::{Method, StatusCode};
use lambda_http::{Body, IntoResponse, Request, RequestExt, Response};
use lambda_runtime::{error::HandlerError, Context};
use lazy_static::lazy_static;
use maplit::hashmap;
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemInput, PutItemInput};
use std::env;

pub enum DispatchResult {
    IllegalMethod,
    UnknownPath,
    DynamoErr,
    Get(Option<String>),
    Unauthorized,
    InvalidBody,
    Set,
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
            Self::DynamoErr => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::Empty) // TODO: helpful body text about the dynamo problem
                .unwrap(),
            Self::Get(value) => match value {
                Some(v) => Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::Text(v))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::Empty)
                    .unwrap(),
            },
            Self::Unauthorized => Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::Empty) // TODO: helpful body text
                .unwrap(),
            Self::InvalidBody => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::Empty) // TODO: helpful body text
                .unwrap(),
            Self::Set => Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(Body::Empty) // TODO: helpful body text
                .unwrap(),
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

const ID: &'static str = "Id";
const VALUE: &'static str = "Value";
const TABLE: &'static str = "table";
const KEY: &'static str = "key";
const SECRET_TOKEN_ENV_VAR: &'static str = "SECRET_TOKEN";
const AUTHORIZATION: &'static str = "Authorization";
const TOKEN_PREFIX: &'static str = "Token: ";

lazy_static! {
    // using the default region fetches it, if set, from AWS_DEFAULT_REGION
    // or AWS_REGION environment variables. If unset or malformed, falls
    // back to us-east-1.
    static ref CLIENT: DynamoDbClient = DynamoDbClient::new(Region::default());

    // the required token is set in the lambda environment
    static ref SECRET_TOKEN: String = env::var(SECRET_TOKEN_ENV_VAR).unwrap_or_default();
}

/// get a key from a particular table and return its value
fn get(request: Request, _ctx: Context) -> Result<DispatchResult, HandlerError> {
    let pparams = request.path_parameters();
    let table: &str;
    let key: &str;
    match (pparams.get(TABLE), pparams.get(KEY)) {
        (Some(t), Some(k)) => {
            table = t;
            key = k;
        }
        _ => return Ok(DispatchResult::UnknownPath),
    }
    let value = match CLIENT
        .get_item({
            let mut gii = GetItemInput::default();
            gii.key = hashmap! {ID.to_string() => {
                let mut av = AttributeValue::default();
                av.s = Some(key.to_string());
                av
            }};
            gii.table_name = table.to_string();
            gii
        })
        .sync()
    {
        Ok(v) => v,
        Err(_) => return Ok(DispatchResult::DynamoErr),
    };

    Ok(DispatchResult::Get(
        value
            .item
            // TODO: can we consume av somehow so we can avoid the clone?
            .map(|v| v.get(VALUE).map(|av| av.s.clone()))
            // in the future, this map will be replaced with `.flatten()`,
            // which requires nightly for now.
            .map(|os| match os {
                Some(Some(s)) => s,
                _ => String::new(),
            }),
    ))
}

/// set a key in a given table to the provided string
fn set(request: Request, _ctx: Context) -> Result<DispatchResult, HandlerError> {
    let mut authorized = false;
    if let Some(auth_value) = request.headers().get(AUTHORIZATION) {
        if let Ok(auth) = auth_value.to_str() {
            if auth.starts_with(TOKEN_PREFIX) && &auth[TOKEN_PREFIX.len()..] == *SECRET_TOKEN {
                authorized = true;
            }
        }
    }
    if !authorized {
        return Ok(DispatchResult::Unauthorized);
    }

    // TODO: maybe extract these params in a separate function? This is just
    // copy-pasted code.
    let pparams = request.path_parameters();
    let table: &str;
    let key: &str;
    match (pparams.get(TABLE), pparams.get(KEY)) {
        (Some(t), Some(k)) => {
            table = t;
            key = k;
        }
        _ => return Ok(DispatchResult::UnknownPath),
    }

    // extract the request body as string
    let body: String;
    match request.into_body() {
        Body::Text(b) => body = b,
        _ => return Ok(DispatchResult::InvalidBody),
    }

    Ok(
        match CLIENT
            .put_item({
                let mut pii = PutItemInput::default();
                pii.item = hashmap! {
                    ID.to_string() => {
                    let mut av = AttributeValue::default();
                    av.s = Some(key.to_string());
                    av
                },
                VALUE.to_string() => {
                    let mut av = AttributeValue::default();
                    av.s = Some(body);
                    av
                }};
                pii.table_name = table.to_string();
                pii
            })
            .sync()
        {
            Ok(_) => DispatchResult::Set,
            Err(_) => DispatchResult::DynamoErr,
        },
    )
}
