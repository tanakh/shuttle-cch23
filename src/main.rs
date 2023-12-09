use std::collections::HashMap;

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Result},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use base64::Engine;
use serde::{de::DeserializeOwned, Deserialize};
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

async fn day4_task2(Json(payload): Json<Vec<Reindeer>>) -> impl IntoResponse {
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

async fn day6(body: String) -> impl IntoResponse {
    let elf_on_a_shelf = body.match_indices("elf on a shelf").count();

    Json(json!({
        "elf": body.match_indices("elf").count(),
        "elf on a shelf": elf_on_a_shelf,
        "shelf with no elf on it": body.match_indices("shelf").count() - elf_on_a_shelf,
    }))
}

fn get_value_from_cookie<T: DeserializeOwned>(jar: &CookieJar, name: &str) -> Option<T> {
    let s = jar.get(name)?.value();
    let decoded = base64::prelude::BASE64_STANDARD.decode(s).ok()?;
    let decoded = String::from_utf8_lossy(&decoded);
    Some(serde_json::from_str(&decoded).ok()?)
}

async fn day7_task1(jar: CookieJar) -> impl IntoResponse {
    let input = get_value_from_cookie::<serde_json::Value>(&jar, "recipe").unwrap();
    Json(input)
}

async fn day7_task2_3(jar: CookieJar) -> impl IntoResponse {
    let mut input =
        get_value_from_cookie::<HashMap<String, HashMap<String, i64>>>(&jar, "recipe").unwrap();

    let recipe = std::mem::take(input.get_mut("recipe").unwrap());
    let mut pantry = std::mem::take(input.get_mut("pantry").unwrap());

    let mut cookies = i64::MAX;

    for (ingred, amount) in &recipe {
        cookies = cookies.min(pantry.get(ingred).unwrap_or(&0) / amount);
    }

    for (ingred, amount) in &recipe {
        if amount * cookies > 0 {
            *pantry.get_mut(ingred).unwrap() -= amount * cookies;
        }
    }

    Json(json!({"cookies": cookies, "pantry": pantry}))
}

async fn pokeapi(id: u64) -> Result<HashMap<String, serde_json::Value>> {
    Ok(
        reqwest::get(format!("https://pokeapi.co/api/v2/pokemon/{id}/"))
            .await
            .map_err(|_| "pokeapi error")?
            .json::<HashMap<String, serde_json::Value>>()
            .await
            .map_err(|_| "invalid json")?,
    )
}

async fn day8_task1(Path(id): Path<u64>) -> Result<impl IntoResponse> {
    let pokemon = pokeapi(id).await?;
    let weight = pokemon.get("weight").unwrap().as_u64().unwrap();
    Ok(format!("{}", weight as f64 / 10.0))
}

async fn day8_task2(Path(id): Path<u64>) -> Result<impl IntoResponse> {
    let pokemon = pokeapi(id).await?;
    let weight = pokemon.get("weight").unwrap().as_u64().unwrap();
    let h = 10.0_f64;
    let g = 9.825;
    let v = (2.0 * g * h).sqrt();
    let f = weight as f64 / 10.0 * v;
    Ok(format!("{f:.12}"))
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/1/*nums", get(day1))
        .route("/-1/error", get(error))
        .route("/4/strength", post(day4_task1))
        .route("/4/contest", post(day4_task2))
        .route("/6", post(day6))
        .route("/7/decode", get(day7_task1))
        .route("/7/bake", get(day7_task2_3))
        .route("/8/weight/:id", get(day8_task1))
        .route("/8/drop/:id", get(day8_task2))
        .route("/", get(hello_world));
    Ok(router.into())
}
