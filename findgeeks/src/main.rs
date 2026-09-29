use lambda_runtime::{LambdaEvent, Error, service_fn};
use serde_json::{json, Value};
use sqlx::{MySql, MySqlPool, Row};
use std::env::var;
use regex::Regex;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(handler)).await
}

// https://github.com/aws/aws-lambda-rust-runtime
pub(crate) async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let (event, _context) = event.into_parts();
    let raw_path: &str = event["rawPath"].as_str().unwrap_or("/r").into();

    if raw_path.eq("/r/findgeeks") {
        let re = Regex::new(r"[^A-Za-z0-9 _!-]+")?;
        println!("{:?}", event);
        let fragment = re.replace_all(event["queryStringParameters"]["fragment"].as_str().unwrap_or(""), "").to_string();
        let fpercent = format!("{fragment}%");
        let percentfpercent = format!("%{fragment}%");

        let pool_r = get_pool().await;
        if pool_r.is_err() {
            return Ok(json!({ "statusCode": 500, "body": "Unable to connect to database" }));
        }
        let pool: sqlx::Pool<MySql> = pool_r?;
        let sql = "select username from geeks where LOWER(username) like ? order by 1 limit 10";
        let mut matches: Vec<sqlx::mysql::MySqlRow> = sqlx::query(sql).bind(fpercent).fetch_all(&pool).await?;
        if matches.is_empty() {
            matches = sqlx::query(sql).bind(percentfpercent).fetch_all(&pool).await?;
        }
        let data: Vec<&str> = matches.iter().map(|row| row.get::<&str, &str>("username")).collect();
        let mut parts: Vec<&str> = vec!("[");
        for d in data {
            parts.push("\"");
            parts.push(d);
            parts.push("\"");
            parts.push(", ");
        }
        if parts.len() > 1 {
            parts.pop();
        }
        parts.push("]");
        let result = parts.join("");
        Ok(json!({
            "statusCode": 200,
            "body": result
        }))
    } else if raw_path.eq("/r/finddesigner") {
        let bggid_s = event["queryStringParameters"]["bggid"].as_str().unwrap_or("");
        let bggid_r = bggid_s.parse::<i32>();
        if bggid_r.is_err() {
            return Ok(json!({ "statusCode": 400, "body": "Bad bggid parameter" }));
        }
        let pool_r = get_pool().await;
        if pool_r.is_err() {
            return Ok(json!({ "statusCode": 500, "body": "Unable to connect to database" }));
        }
        let bggid = bggid_r.unwrap();
        let row = sqlx::query("select bggid, name from designers where bggid = ?")
            .bind(bggid)
            .fetch_optional(&pool_r.unwrap())
            .await?;
        let result: Value = match row {
            Some(row) => json!({ "bggid": bggid, "name": row.get::<String, _>("name") }),
            None => json!({}),
        };
        Ok(json!({
            "statusCode": 200,
            "body": result.to_string()
        }))
    } else {
        Ok(json!({
            "statusCode": 404,
            "body": "Path not found"
        }))
    }
}

async fn get_pool() -> Result<sqlx::Pool<MySql>, sqlx::Error> {
    let user = var("MYSQL_USERNAME").unwrap();
    let password = var("MYSQL_PASSWORD").unwrap();
    let host = var("MYSQL_HOST").unwrap();
    let database = var("MYSQL_DATABASE").unwrap();
    let url = format!("mysql://{user}:{password}@{host}/{database}");
    MySqlPool::connect(&url).await
}
