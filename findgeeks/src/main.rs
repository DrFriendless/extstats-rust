// Claude helped write this code because the Rust doco is pretty impenetrable.

use lambda_runtime::{service_fn, Error, LambdaEvent};
use regex::Regex;
use serde_json::{json, Value};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlQueryResult};
use sqlx::{MySqlPool};
use std::{env::var, sync::LazyLock};

static CLEAN_GEEK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9 _!-]+").unwrap());
static CLEAN_DESIGNER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w '!-]+").unwrap());

#[tokio::main]
async fn main() -> Result<(), Error> {
    // pool is shared across invocations
    let pool = get_pool().await?;
    lambda_runtime::run(service_fn(move |event| {
        let pool = pool.clone(); // cheap: it's an Arc inside
        async move { handler(&pool, event).await }
    })).await
}

// https://github.com/aws/aws-lambda-rust-runtime
pub(crate) async fn handler(pool: &MySqlPool, event: LambdaEvent<Value>) -> Result<Value, Error> {
    let (event, _context) = event.into_parts();
    let raw_path: &str = event["rawPath"].as_str().unwrap_or("/r").into();
    let method = event["requestContext"]["http"]["method"].as_str().unwrap_or("");
    println!("{event}");

    if method == "GET" {
        if raw_path.eq("/r/findgeeks") {
            increment_rust_counter(pool).await?;
            let fragment = CLEAN_GEEK.replace_all(event["queryStringParameters"]["fragment"].as_str().unwrap_or(""), "").to_lowercase();
            let fpercent = format!("{fragment}%");
            let percentfpercent = format!("%{fragment}%");

            let sql = "select username from geeks where LOWER(username) like ? order by 1 limit 10";
            let mut matches: Vec<String> = sqlx::query_scalar(sql).bind(fpercent).fetch_all(pool).await?;
            if matches.is_empty() {
                matches = sqlx::query_scalar(sql).bind(percentfpercent).fetch_all(pool).await?;
            }
            let result = serde_json::to_string(&matches)?;
            Ok(json!({
                "statusCode": 200,
                "body": result,
                "headers": { "content-type": "application/json" },
            }))
        } else if raw_path.eq("/r/finddesigner") {
            increment_rust_counter(pool).await?;
            let bggid = match get_bggid_param(&event) {
                Ok(value) => value,
                Err(value) => return Ok(value),
            };
            let sql = "select name from designers where bggid = ?";
            let row: Option<String> = sqlx::query_scalar(sql)
                .bind(bggid)
                .fetch_optional(pool)
                .await?;
            let result: Value = match row {
                Some(name) => json!({ "bggid": bggid, "name": name }),
                None => json!({}),
            };
            Ok(json!({
                "statusCode": 200,
                "body": result.to_string(),
                "headers": { "content-type": "application/json" },
            }))
        } else if raw_path.eq("/r/finddesigners") {
            increment_rust_counter(pool).await?;
            let sql = "select name from designers where LOWER(name) like ? order by 1 limit 10";
            let fragment = CLEAN_DESIGNER.replace_all(event["queryStringParameters"]["fragment"].as_str().unwrap_or(""), "").to_lowercase();
            let fpercent = format!("{fragment}%");
            let percentfpercent = format!("%{fragment}%");

            let mut matches: Vec<String> = sqlx::query_scalar(sql).bind(fpercent).fetch_all(pool).await?;
            if matches.is_empty() { matches = sqlx::query_scalar(sql).bind(percentfpercent).fetch_all(pool).await?; }
            let result = serde_json::to_string(&matches)?;

            Ok(json!({
                "statusCode": 200,
                "body": result,
                "headers": { "content-type": "application/json" }
            }))
        } else if raw_path.eq("/r/findpublisher") {
            increment_rust_counter(pool).await?;
            let bggid = match get_bggid_param(&event) {
                Ok(value) => value,
                Err(value) => return Ok(value),
            };

            let sql = "select name from publishers where bggid = ?";
            let row: Option<String> = sqlx::query_scalar(sql)
                .bind(bggid)
                .fetch_optional(pool)
                .await?;
            let result: Value = match row {
                Some(row) => json!({ "bggid": bggid, "name": row }),
                None => json!({}),
            };

            Ok(json!({
                "statusCode": 200,
                "body": result.to_string(),
                "headers": { "content-type": "application/json" }
            }))
        } else if raw_path.eq("/r/findpublishers") {
            increment_rust_counter(pool).await?;
            let sql = "select name from publishers where LOWER(name) like ? order by 1 limit 10";
            let fragment = CLEAN_DESIGNER.replace_all(event["queryStringParameters"]["fragment"].as_str().unwrap_or(""), "").to_lowercase();
            let fpercent = format!("{fragment}%");
            let percentfpercent = format!("%{fragment}%");

            let mut matches: Vec<String> = sqlx::query_scalar(sql).bind(fpercent).fetch_all(pool).await?;
            if matches.is_empty() { matches = sqlx::query_scalar(sql).bind(percentfpercent).fetch_all(pool).await?; }
            let result = serde_json::to_string(&matches)?;

            Ok(json!({
                "statusCode": 200,
                "body": result,
                "headers": { "content-type": "application/json" }
            }))
        } else {
            Ok(json!({
            "statusCode": 404,
            "body": "Path not found"
        }))
        }
    } else if method == "POST" {
        if raw_path.eq("/r/count") {
            increment_rust_counter(pool).await?;
            let fragment = event["queryStringParameters"]["counts"].as_str().unwrap_or("").to_lowercase();
            for frag in fragment.split(',') {
                match frag {
                    "page" => { increment_page_views(pool).await?; },
                    "blog" => { increment_blog_views(pool).await?; },
                    "doco" => { increment_doco_views(pool).await?; },
                    "adv" => { increment_adventure_views(pool).await?; },
                    _ => { continue; }
                }
            }
            Ok(json!({
            "statusCode": 200,
            "headers": { "content-type": "application/json" }
        }))
        } else {
            Ok(json!({
            "statusCode": 404,
            "body": "Path not found"
        }))
        }
    } else {
        Ok(json!({
            "statusCode": 400,
            "body": "Unused method"
        }))
    }
}

async fn increment_rust_counter(pool: &MySqlPool) -> Result<MySqlQueryResult, sqlx::Error> {
    let sql = "update counters set rust_calls = rust_calls + 1";
    sqlx::query(sql).execute(pool).await
}

async fn increment_page_views(pool: &MySqlPool) -> Result<MySqlQueryResult, sqlx::Error> {
    let sql = "update counters set page_views = page_views + 1";
    sqlx::query(sql).execute(pool).await
}

async fn increment_blog_views(pool: &MySqlPool) -> Result<MySqlQueryResult, sqlx::Error> {
    let sql = "update counters set blog_views = blog_views + 1";
    sqlx::query(sql).execute(pool).await
}

async fn increment_doco_views(pool: &MySqlPool) -> Result<MySqlQueryResult, sqlx::Error> {
    let sql = "update counters set doco_views = doco_views + 1";
    sqlx::query(sql).execute(pool).await
}

async fn increment_adventure_views(pool: &MySqlPool) -> Result<MySqlQueryResult, sqlx::Error> {
    let sql = "update counters set adventure_views = adventure_views + 1";
    sqlx::query(sql).execute(pool).await
}

fn get_bggid_param(event: &Value) -> Result<i32, Value> {
    let bggid_s = event["queryStringParameters"]["bggid"].as_str().unwrap_or("");
    let bggid_r = bggid_s.parse::<i32>();
    if bggid_r.is_err() {
        return Err(json!({ "statusCode": 400, "body": "Bad bggid parameter" }));
    }
    Ok(bggid_r.unwrap())
}

async fn get_pool() -> Result<MySqlPool, Error> {
    let opts = MySqlConnectOptions::new()
        .host(&var("MYSQL_HOST")?)
        .username(&var("MYSQL_USERNAME")?)
        .password(&var("MYSQL_PASSWORD")?)
        .database(&var("MYSQL_DATABASE")?);
    Ok(MySqlPoolOptions::new()
        .max_connections(1)
        .connect_lazy_with(opts))
}
