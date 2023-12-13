use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, RwLock},
};

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Result},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use base64::Engine;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use shuttle_runtime::CustomError;
use sqlx::{postgres::PgPoolOptions, Executor as _, PgPool, QueryBuilder, Row as _};

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
    let count =
        |s: &str, pat: &str| -> usize { (0..s.len()).filter(|i| s[*i..].starts_with(pat)).count() };

    let elf_on_a_shelf = count(&body, "elf on a shelf");

    Json(json!({
        "elf": count(&body, "elf"),
        "elf on a shelf": elf_on_a_shelf,
        "shelf with no elf on it": count(&body, "shelf") - elf_on_a_shelf,
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
        if *amount != 0 {
            cookies = cookies.min(pantry.get(ingred).unwrap_or(&0) / amount);
        }
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

async fn day11_task2(mut multipart: Multipart) -> Result<impl IntoResponse> {
    while let Some(field) = multipart.next_field().await? {
        if field.name() != Some("image") {
            continue;
        }

        let bytes = field.bytes().await?;
        let mut reader = image::io::Reader::new(Cursor::new(bytes));
        reader.set_format(image::ImageFormat::Png);

        let image = reader.decode().map_err(|e| format!("{e:?}"))?;
        let image = image
            .as_rgb8()
            .ok_or_else(|| format!("unsupported color format"))?;

        let mut red_pixels = 0;
        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = image.get_pixel(x, y);
                let r = pixel[0] as u32;
                let g = pixel[1] as u32;
                let b = pixel[2] as u32;

                if r > g + b {
                    red_pixels += 1;
                }
            }
        }

        return Ok(format!("{red_pixels}"));
    }

    Err("no image found")?
}

async fn day12_task1_post(State(state): State<Arc<RwLock<AppState>>>, Path(key): Path<String>) {
    let mut lock = state.write().unwrap();
    lock.day12.insert(key, time::Instant::now());
}

async fn day12_task1_get(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse> {
    let lock = state.read().unwrap();

    if let Some(time) = lock.day12.get(&key) {
        Ok(format!(
            "{:?}",
            time.elapsed().as_seconds_f64().floor() as i64
        ))
    } else {
        Err("key not found")?
    }
}

async fn day12_task2(Json(ulids): Json<Vec<String>>) -> Result<impl IntoResponse> {
    let ret = ulids
        .into_iter()
        .map(|s| -> Result<_> {
            let ulid = ulid::Ulid::from_string(&s).map_err(|e| format!("{e}"))?;
            Ok(uuid::Uuid::from_u128(ulid.0))
        })
        .rev()
        .collect::<Result<Vec<uuid::Uuid>>>()?;
    Ok(Json(ret))
}

async fn day12_task3(
    Path(weekday): Path<String>,
    Json(ulids): Json<Vec<String>>,
) -> Result<impl IntoResponse> {
    let weekday: u8 = weekday.parse().map_err(|_| "invalid weekday")?;

    let mut christmas_eve = 0;
    let mut weekday_cnt = 0;
    let mut in_the_future = 0;
    let mut lsb_is_1 = 0;

    for s in ulids {
        let ulid = ulid::Ulid::from_string(&s).map_err(|e| format!("{e}"))?;
        let ts = ulid.datetime();
        let epoch = ts
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|_| "invalid timestamp")?;
        let dt = time::OffsetDateTime::from_unix_timestamp_nanos(epoch.as_nanos() as i128)
            .map_err(|e| format!("{e}"))?;

        if dt.month() as u8 == 12 && dt.day() == 24 {
            christmas_eve += 1;
        }

        if dt.weekday().number_days_from_monday() == weekday {
            weekday_cnt += 1;
        }

        if dt > time::OffsetDateTime::now_utc() {
            in_the_future += 1;
        }

        if ulid.0 & 1 == 1 {
            lsb_is_1 += 1;
        }
    }

    Ok(Json(json!({
        "christmas eve": christmas_eve,
        "weekday": weekday_cnt,
        "in the future": in_the_future,
        "LSB is 1": lsb_is_1,
    })))
}

async fn day13_task1(State(pool): State<Pool>) -> Result<String> {
    let row = sqlx::query("SELECT 20231213")
        .fetch_one(&pool.pool)
        .await
        .map_err(|e| format!("sql error: {e:?}"))?;
    let res: i32 = row.get(0);
    Ok(format!("{res}"))
}

async fn day13_task2_reset(State(pool): State<Pool>) -> Result<impl IntoResponse> {
    sqlx::migrate!()
        .undo(&pool.pool, 0)
        .await
        .map_err(|e| format!("{e:?}"))?;
    sqlx::migrate!()
        .run(&pool.pool)
        .await
        .map_err(|e| format!("{e:?}"))?;

    Ok(())
}

#[derive(Deserialize, Debug)]
struct Order {
    id: i32,
    region_id: i32,
    gift_name: String,
    quantity: i32,
}

async fn day13_task2_orders(
    State(pool): State<Pool>,
    Json(orders): Json<Vec<Order>>,
) -> Result<impl IntoResponse> {
    let mut query_builder =
        QueryBuilder::new("INSERT INTO orders (id, region_id, gift_name, quantity)");

    query_builder.push_values(orders, |mut b, order| {
        b.push_bind(order.id)
            .push_bind(order.region_id)
            .push_bind(order.gift_name)
            .push_bind(order.quantity);
    });

    let query = query_builder.build();

    query
        .execute(&pool.pool)
        .await
        .map_err(|e| format!("SQL error: {e:?}"))?;

    Ok(())
}

async fn day13_task2_orders_total(State(pool): State<Pool>) -> Result<impl IntoResponse> {
    let row = sqlx::query("SELECT SUM(quantity) FROM orders")
        .fetch_one(&pool.pool)
        .await
        .map_err(|e| format!("SQL error: {e:?}"))?;
    let res: i64 = row.get(0);
    Ok(Json(json!({ "total": res })))
}

async fn day13_task2_orders_popular(State(pool): State<Pool>) -> Result<impl IntoResponse> {
    let row = sqlx::query("SELECT gift_name FROM orders WHERE id = (SELECT MAX(id) FROM orders)")
        .fetch_all(&pool.pool)
        .await
        .map_err(|e| format!("SQL error: {e:?}"))?;

    let res = if row.len() == 1 {
        let res: String = row[0].get(0);
        json!(res)
    } else {
        json!(null)
    };

    Ok(Json(json!({"popular": res})))
}

#[derive(Default)]
struct AppState {
    day12: HashMap<String, time::Instant>,
}

#[derive(Clone)]
struct Pool {
    pool: PgPool,
}

#[shuttle_runtime::main]
async fn main(#[shuttle_shared_db::Postgres] pool: PgPool) -> shuttle_axum::ShuttleAxum {
    dbg!(sqlx::migrate!())
        .run(&pool)
        .await
        .map_err(CustomError::new)?;

    let shared_state = Arc::new(RwLock::new(AppState::default()));

    let router = Router::new()
        .route("/-1/error", get(error))
        .route("/1/*nums", get(day1))
        .route("/4/strength", post(day4_task1))
        .route("/4/contest", post(day4_task2))
        .route("/6", post(day6))
        .route("/7/decode", get(day7_task1))
        .route("/7/bake", get(day7_task2_3))
        .route("/8/weight/:id", get(day8_task1))
        .route("/8/drop/:id", get(day8_task2))
        .nest_service("/11/assets", tower_http::services::ServeDir::new("assets"))
        .route("/11/red_pixels", post(day11_task2))
        .route("/12/save/:key", post(day12_task1_post))
        .route("/12/load/:key", get(day12_task1_get))
        .route("/12/ulids", post(day12_task2))
        .route("/12/ulids/:weekday", post(day12_task3))
        .with_state(shared_state)
        .route("/13/sql", get(day13_task1))
        .route("/13/reset", post(day13_task2_reset))
        .route("/13/orders", post(day13_task2_orders))
        .route("/13/orders/total", get(day13_task2_orders_total))
        .route("/13/orders/popular", get(day13_task2_orders_popular))
        .with_state(Pool { pool })
        .route("/", get(hello_world));
    Ok(router.into())
}
