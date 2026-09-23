use lambda_runtime::{LambdaEvent, Error, service_fn};
use serde_json::Value;
use sqlx::{MySql, MySqlPool, Row};
use std::env::var;
use serde_json::Value::String;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(handler)).await
}

// https://github.com/aws/aws-lambda-rust-runtime
pub(crate) async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let (event, _context) = event.into_parts();
    // println!("{:?}", event);
    let fragment = event["queryStringParameters"]["fragment"].as_str().unwrap_or("");
    let fpercent = format!("{fragment}%");
    let percentfpercent = format!("%{fragment}%");

    let sql = "select username from geeks where LOWER(username) like ? order by 1 limit 10";
    let rpool = get_pool().await;
    if rpool.is_ok() {
        let pool: sqlx::Pool<MySql> = rpool?;
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
        Ok(String(result))
    } else {
        Err(rpool.unwrap_err().into())
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
