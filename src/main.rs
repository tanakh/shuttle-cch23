use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Multipart, Path, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Result},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use base64::Engine;
use futures_util::{
    future::{select_all, Either},
    stream_select, SinkExt as _, StreamExt as _,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use shuttle_runtime::CustomError;
use sqlx::{PgPool, QueryBuilder};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio_stream::wrappers::BroadcastStream;

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
    let (res,) = sqlx::query_as::<_, (i32,)>("SELECT 20231213")
        .fetch_one(&pool.pool)
        .await
        .map_err(|e| format!("sql error: {e:?}"))?;
    Ok(format!("{res}"))
}

async fn day13_18_reset(State(pool): State<Pool>) -> Result<impl IntoResponse> {
    let migrator = sqlx::migrate!();
    migrator
        .undo(&pool.pool, 0)
        .await
        .map_err(|e| format!("{e:?}"))?;
    migrator
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

#[derive(Deserialize, Debug)]
struct Region {
    id: i32,
    name: String,
}

async fn day13_18_orders(
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

async fn day18_regions(
    State(pool): State<Pool>,
    Json(regions): Json<Vec<Region>>,
) -> Result<impl IntoResponse> {
    let mut query_builder = QueryBuilder::new("INSERT INTO regions (id, name)");

    query_builder.push_values(regions, |mut b, region| {
        b.push_bind(region.id).push_bind(region.name);
    });

    let query = query_builder.build();

    query
        .execute(&pool.pool)
        .await
        .map_err(|e| format!("SQL error: {e:?}"))?;

    Ok(())
}

async fn day13_task2_orders_total(State(pool): State<Pool>) -> Result<impl IntoResponse> {
    let total = sqlx::query_as::<_, (i64,)>("SELECT SUM(quantity) FROM orders")
        .fetch_one(&pool.pool)
        .await
        .map_err(|e| format!("SQL error: {e:?}"))?;
    Ok(Json(json!({ "total": total })))
}

async fn day18_total(State(pool): State<Pool>) -> Result<impl IntoResponse> {
    let row = sqlx::query_as::<_, (String, i64)>(
        "
        SELECT
            regions.name AS region,
            SUM(orders.quantity) AS total
        FROM orders
        JOIN regions ON orders.region_id = regions.id
        GROUP BY orders.region_id, regions.id
        ORDER BY regions.name
    ",
    )
    .fetch_all(&pool.pool)
    .await
    .map_err(|e| format!("SQL error: {e:?}"))?;

    let res = row
        .into_iter()
        .map(|(region, total)| {
            json!({
                "region": region,
                "total": total,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(res))
}

async fn day13_task2_orders_popular(State(pool): State<Pool>) -> Result<impl IntoResponse> {
    let row = sqlx::query_as::<_, (String,)>(
        "
        SELECT gift_name
        FROM orders
        WHERE id = (SELECT MAX(id) FROM orders)
    ",
    )
    .fetch_all(&pool.pool)
    .await
    .map_err(|e| format!("SQL error: {e:?}"))?;

    let res = if row.len() == 1 {
        json!(row[0].0.clone())
    } else {
        json!(null)
    };

    Ok(Json(json!({"popular": res})))
}

async fn day18_top_list(
    Path(limit): Path<i32>,
    State(pool): State<Pool>,
) -> Result<impl IntoResponse> {
    let row = sqlx::query_as::<_, (String, Vec<String>)>(
        "
        SELECT
            sum.region_name AS region,
            ARRAY_REMOVE(
                (ARRAY_AGG(
                    sum.gift_name ORDER BY sum.quantity DESC, sum.gift_name ASC
                ))[:$1], NULL
            ) AS top_gifts
        FROM (
            SELECT
                regions.name AS region_name,
                orders.gift_name AS gift_name,
                SUM(orders.quantity) AS quantity
            FROM regions
            LEFT JOIN orders ON regions.id = orders.region_id
            GROUP BY regions.id, orders.gift_name
        ) AS sum
        GROUP BY sum.region_name
        ORDER BY sum.region_name ASC
    ",
    )
    .bind(limit)
    .fetch_all(&pool.pool)
    .await
    .map_err(|e| format!("SQL error: {e:?}"))?;

    let mut ret = vec![];

    for (region, top_gifts) in row {
        ret.push(json!({
            "region": region,
            "top_gifts": top_gifts,
        }));
    }

    Ok(Json(ret))
}

#[derive(Deserialize)]
struct Day14 {
    content: String,
}

async fn day14_task1(Json(input): Json<Day14>) -> Result<impl IntoResponse> {
    let resp = format!(
        r"<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    {}
  </body>
</html>",
        input.content
    );
    Ok(resp)
}

async fn day14_task2(Json(input): Json<Day14>) -> Result<impl IntoResponse> {
    let resp = format!(
        r"<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    {}
  </body>
</html>",
        html_escape::encode_double_quoted_attribute(&input.content)
    );
    Ok(resp)
}

#[derive(Deserialize)]
struct Day15 {
    input: String,
}

async fn day15_task1(Json(input): Json<Day15>) -> impl IntoResponse {
    let (code, resp) = if is_nice(&input.input) {
        (StatusCode::OK, "nice")
    } else {
        (StatusCode::BAD_REQUEST, "naughty")
    };

    (code, Json(json!({ "result": resp })))
}

fn is_nice(s: &str) -> bool {
    let vowels = s.chars().filter(|c| "aeiouy".contains(*c)).count();
    let twice = s
        .as_bytes()
        .windows(2)
        .any(|w| w[0] == w[1] && w[0].is_ascii_alphabetic());
    let err = ["ab", "cd", "pq", "xy"].iter().any(|p| s.contains(p));

    vowels >= 3 && twice && !err
}

async fn day15_task2(Json(input): Json<Day15>) -> impl IntoResponse {
    let s = &input.input;

    let len = s.len();
    let uppercase = s.chars().any(|c| c.is_uppercase());
    let lowercase = s.chars().any(|c| c.is_lowercase());
    let digit = s.chars().filter(|c| c.is_digit(10)).count();
    let sum = s
        .chars()
        .map(|c| if c.is_digit(10) { c } else { ' ' })
        .collect::<String>()
        .split_ascii_whitespace()
        .map(|w| w.parse::<i64>().unwrap())
        .sum::<i64>();

    let re_joy = regex::Regex::new(r"j.*o.*y").unwrap();
    let re_no_joy = &[
        regex::Regex::new(r"j.*y.*o").unwrap(),
        regex::Regex::new(r"o.*j.*y").unwrap(),
        regex::Regex::new(r"o.*y.*j").unwrap(),
        regex::Regex::new(r"y.*j.*o").unwrap(),
        regex::Regex::new(r"y.*o.*j").unwrap(),
    ];
    let joy = re_joy.is_match(s) && !re_no_joy.iter().any(|re| re.is_match(s));
    let rep = s.as_bytes().windows(3).any(|w| {
        w[0] == w[2] && w[0] != w[1] && w[0].is_ascii_alphabetic() && w[1].is_ascii_alphabetic()
    });
    let unicode = s.chars().any(|c| ('\u{2980}'..='\u{2BFF}').contains(&c));
    let emoji = s
        .chars()
        .any(|c| unic::emoji::char::is_emoji_presentation(c));
    let digest = sha256::digest(s.as_bytes());

    let (code, resp) = match () {
        _ if len < 8 => (400, "8 chars"),
        _ if !uppercase || !lowercase || digit == 0 => (400, "more types of chars"),
        _ if digit < 5 => (400, "55555"),
        _ if sum != 2023 => (400, "math is hard"),
        _ if !joy => (406, "not joyful enough"),
        _ if !rep => (451, "illegal: no sandwich"),
        _ if !unicode => (416, "outranged"),
        _ if !emoji => (426, "😳"),
        _ if !digest.ends_with("a") => (418, "not a coffee brewer"),
        _ => (200, "that's a nice password"),
    };

    (
        StatusCode::from_u16(code).unwrap(),
        Json(json!({
            "result": if code == 200 { "nice" } else { "naughty" },
            "reason": resp,
        })),
    )
}

async fn day19_task1(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(day19_task1_handle)
}

async fn day19_task1_handle(mut socket: WebSocket) {
    let mut started = false;

    while let Some(msg) = socket.recv().await {
        let Ok(msg) = msg else {
            return;
        };
        let Ok(msg) = msg.to_text() else {
            return;
        };

        if !started {
            if msg == "serve" {
                started = true;
            }
        } else {
            if msg == "ping" {
                if socket.send("pong".into()).await.is_err() {
                    // client disconnected
                    return;
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Tweet {
    user: String,
    message: String,
}

#[derive(Deserialize)]
struct TweetMessage {
    message: String,
}

#[derive(Clone, Default)]
struct TwitterState {
    views: Arc<AtomicUsize>,
    rooms: Arc<Mutex<HashMap<usize, Room>>>,
}

struct Room {
    tx: Sender<Tweet>,
}

impl TwitterState {
    fn join(&self, room: usize) -> (Sender<Tweet>, Receiver<Tweet>) {
        let mut room_lock = self.rooms.lock().unwrap();
        let room = room_lock.entry(room).or_insert_with(|| {
            let (tx, _rx) = tokio::sync::broadcast::channel(1_000_000);
            Room { tx }
        });
        (room.tx.clone(), room.tx.subscribe())
    }

    fn inc_views(&self) {
        self.views.fetch_add(1, Ordering::SeqCst);
    }

    fn reset_views(&self) {
        self.views.store(0, Ordering::SeqCst);
    }

    fn views(&self) -> usize {
        self.views.load(Ordering::SeqCst)
    }
}

async fn day19_task2_reset(State(state): State<TwitterState>) -> impl IntoResponse {
    eprintln!("reset views");
    state.reset_views();
}

async fn day19_task2_views(State(state): State<TwitterState>) -> impl IntoResponse {
    let views = state.views();
    eprintln!("views: {views}");
    format!("{views}")
}

async fn day19_task2(
    Path((room, user)): Path<(usize, String)>,
    State(state): State<TwitterState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| day19_task2_handle(room, user, state, socket))
}

async fn day19_task2_handle(room: usize, user: String, state: TwitterState, socket: WebSocket) {
    eprintln!("{user} enters room #{room}");

    let (tx, rx) = state.join(room);

    let rx = BroadcastStream::new(rx).map(Either::Right);

    let (mut socket_sink, socket_stream) = socket.split();

    let socket = socket_stream.map(Either::Left);
    let mut r = stream_select!(rx, socket);

    while let Some(msg) = r.next().await {
        match msg {
            Either::Left(msg) => {
                let Ok(msg) = msg else {
                    return;
                };
                let Ok(msg) = msg.to_text() else {
                    return;
                };
                let Ok(msg) = serde_json::from_str::<TweetMessage>(msg) else {
                    return;
                };
                if msg.message.len() > 128 {
                    continue;
                }
                tx.send(Tweet {
                    user: user.clone(),
                    message: msg.message,
                })
                .unwrap();
            }
            Either::Right(tweet) => {
                if let Ok(tweet) = tweet {
                    state.inc_views();
                    if socket_sink
                        .send(Message::Text(serde_json::to_string(&tweet).unwrap()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
    }
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
        .route("/13/reset", post(day13_18_reset))
        .route("/13/orders", post(day13_18_orders))
        .route("/13/orders/total", get(day13_task2_orders_total))
        .route("/13/orders/popular", get(day13_task2_orders_popular))
        .route("/14/unsafe", post(day14_task1))
        .route("/14/safe", post(day14_task2))
        .route("/15/nice", post(day15_task1))
        .route("/15/game", post(day15_task2))
        .route("/18/reset", post(day13_18_reset))
        .route("/18/orders", post(day13_18_orders))
        .route("/18/regions", post(day18_regions))
        .route("/18/regions/total", get(day18_total))
        .route("/18/regions/top_list/:limit", get(day18_top_list))
        .with_state(Pool { pool })
        .route("/19/ws/ping", get(day19_task1))
        .route("/19/reset", post(day19_task2_reset))
        .route("/19/views", get(day19_task2_views))
        .route("/19/ws/room/:room_id/user/:user_id", get(day19_task2))
        .with_state(TwitterState::default())
        .route("/", get(hello_world));
    Ok(router.into())
}
