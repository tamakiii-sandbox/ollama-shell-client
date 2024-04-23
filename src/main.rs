// src/main.rs
use reqwest::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": "llama3",
            "prompt": "Why is the sky blue?"
        }))
        .send()
        .await?;

    println!("Status: {}", res.status());
    Ok(())
}
