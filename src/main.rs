use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Router};

async fn hello_world() -> &'static str {
    "Hello, world!"
}

async fn error() -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
}

async fn day1(Path(nums): Path<String>) -> impl IntoResponse {
    let val = nums
        .split('/')
        .map(|num| num.parse::<i64>().unwrap())
        .fold(0, |a, b| a ^ b)
        .pow(3);
    format!("{val}")
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/1/*nums", get(day1))
        .route("/-1/error", get(error))
        .route("/", get(hello_world));
    Ok(router.into())
}
