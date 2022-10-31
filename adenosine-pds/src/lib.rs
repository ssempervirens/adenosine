
use anyhow::Result;
use log::{self, debug};
use warp::Filter;
use warp::reply::Response;
use std::collections::HashMap;

pub async fn run_server() -> Result<()> {

    // GET /
    let homepage = warp::path::end().map(|| "Not much to see here yet!");

    // GET /xrpc/some.method w/ query params
    let xrpc_some_get = warp::get()
        .and(warp::path!("xrpc" / "some.method"))
        .and(warp::query::<HashMap<String, String>>())
        .map(|query_params: HashMap<String, String>| {
            println!("query params: {:?}", query_params);
            // return query params as a JSON map object
            warp::reply::json(&query_params)
        });

    // POST /xrpc/other.method w/ query params
    let xrpc_other_post = warp::post()
        .and(warp::path!("xrpc" / "other.method"))
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::body::json())
        .map(|query_params: HashMap<String, String>, body_val: serde_json::Value| {
            println!("query params: {:?}", query_params);
            println!("body JSON: {}", body_val);
            // echo it back
            warp::reply::json(&body_val)
        });

    let routes = homepage.or(xrpc_some_get).or(xrpc_other_post).with(warp::log("adenosine-pds"));
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
    Ok(())
}

// TODO: tokio::task::spawn_blocking
