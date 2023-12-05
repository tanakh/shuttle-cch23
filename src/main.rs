use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

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

#[derive(Deserialize)]
struct Reindeer {
    name: String,
    strength: i64,
    #[serde(default)]
    speed: f64,
    #[serde(default)]
    height: i64,
    #[serde(default)]
    antler_width: i64,
    #[serde(default)]
    snow_magic_power: i64,
    #[serde(default)]
    favorite_food: String,
    #[serde(rename = "cAnD13s_3ATeN-yesT3rdAy")]
    #[serde(default)]
    candies: i64,
}

async fn day4_task1(Json(payload): Json<Vec<Reindeer>>) -> impl IntoResponse {
    let sum = payload.iter().fold(0, |a, b| a + b.strength);
    format!("{sum}")
}

async fn day4_task2(Json(payload): Json<Vec<Reindeer>>) -> Json<serde_json::Value> {
    let fastest = payload
        .iter()
        .max_by(|a, b| a.speed.partial_cmp(&b.speed).unwrap())
        .unwrap();
    let tallest = payload
        .iter()
        .max_by(|a, b| a.height.cmp(&b.height))
        .unwrap();
    let magician = payload
        .iter()
        .max_by(|a, b| a.snow_magic_power.cmp(&b.snow_magic_power))
        .unwrap();
    let consumer = payload
        .iter()
        .max_by(|a, b| a.candies.cmp(&b.candies))
        .unwrap();

    Json(json!({
        "fastest": format!("Speeding past the finish line with a strength of {} is {}", fastest.strength, fastest.name),
        "tallest": format!("{} is standing tall with his {} cm wide antlers", tallest.name, tallest.antler_width),
        "magician": format!("{} could blast you away with a snow magic power of {}", magician.name, magician.snow_magic_power),
        "consumer": format!("{} ate lots of candies, but also some {}", consumer.name, consumer.favorite_food),
    }))
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/1/*nums", get(day1))
        .route("/-1/error", get(error))
        .route("/4/strength", post(day4_task1))
        .route("/4/contest", post(day4_task2))
        .route("/", get(hello_world));
    Ok(router.into())
}
