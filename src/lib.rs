use http::{Method, StatusCode};
use lambda_http::{Body, IntoResponse, Request, RequestExt, Response};
use lambda_runtime::{error::HandlerError, Context};
use lazy_static::lazy_static;
use maplit::hashmap;
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemInput};

pub enum DispatchResult {
    IllegalMethod,
    UnknownPath,
    DynamoErr,
    Get(Option<String>),
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

const ID: &'static str = "Id";
const VALUE: &'static str = "Value";
const TABLE: &'static str = "table";
const KEY: &'static str = "key";

lazy_static! {
    // using the default region fetches it, if set, from AWS_DEFAULT_REGION
    // or AWS_REGION environment variables. If unset or malformed, falls
    // back to us-east-1.
    static ref CLIENT: DynamoDbClient = DynamoDbClient::new(Region::default());
}

/// get an API key from a particular table and return it
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

/// set a key to the provided byte stream
fn set(request: Request, _ctx: Context) -> Result<DispatchResult, HandlerError> {
    unimplemented!()
}
