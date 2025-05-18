use std::time::Duration;
use proxy::create_resolver_fn;
use reqwest::Proxy;
use tokio::time::sleep;

mod proxy;

#[tokio::main]
async fn main() {
    let client = reqwest::ClientBuilder::new()
        .proxy(Proxy::custom(create_resolver_fn()))
        .build()
        .unwrap();

    // let client2 = reqwest::ClientBuilder::new()
    //     .proxy(Proxy::custom(create_resolver_fn()))
    //     .build()
    //     .unwrap();

    loop {
        match client.get("https://www.google.com").send().await {
            Ok(r) => println!("success"),
            Err(e) => println!("error")
        }
        // client2.get("https://www.google.com").send().await.unwrap();
        sleep(Duration::from_secs(1)).await;
    }
}
