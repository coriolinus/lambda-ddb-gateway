use lambda_ddb_gateway::dispatch;
use lambda_http::lambda;

fn main() {
    lambda!(dispatch);
}
