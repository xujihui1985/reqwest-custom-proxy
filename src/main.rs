use proxy::create_auto_proxy_fn;
use reqwest::Proxy;
use std::time::Duration;
use tokio::time::sleep;
use log::{info, error}; // Added for logging
// env_logger will be used directly in main

mod proxy;

#[tokio::main]
async fn main() {
    env_logger::init(); // Initialize env_logger

    let client = reqwest::ClientBuilder::new()
        .proxy(Proxy::custom(create_auto_proxy_fn()))
        .build()
        .unwrap();

    loop {
        match client.get("https://www.google.com").send().await {
            Ok(_r) => info!("Request successful"), // Changed from println!
            Err(_e) => error!("Request error: {:?}", _e), // Changed from println!
        }
        // client2.get("https://www.google.com").send().await.unwrap();
        sleep(Duration::from_secs(1)).await;
    }
}
