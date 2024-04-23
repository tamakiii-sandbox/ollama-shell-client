use anyhow::{Context, Result};
use futures_util::stream::StreamExt;
use reqwest::{Client, Response};
use serde_json::Value;
use std::io::Write;
use std::str;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": "llama3",
            "prompt": "Why is the sky blue?"
        }))
        .send()
        .await?;

    if response.status().is_success() {
        handle_stream(response).await?;
    } else {
        eprintln!("Received HTTP {}", response.status());
    }

    Ok(())
}

async fn handle_stream(response: Response) -> Result<()> {
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let text = str::from_utf8(&bytes)
            .with_context(|| format!("Failed to convert bytes to UTF-8 string: {:?}", bytes))?;
        let json: Value = serde_json::from_str(text)
            .with_context(|| format!("Failed to parse JSON from string: {}", text))?;
        if json.get("done").and_then(Value::as_bool).unwrap_or(false) {
            break; // Indicate completion of the stream
        }
        if let Some(response_text) = json["response"].as_str() {
            print!("{}", response_text);
            std::io::stdout().flush().unwrap();
        }
    }

    println!(); // Add a newline after the last chunk
    Ok(())
}
