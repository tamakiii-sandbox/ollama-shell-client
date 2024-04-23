use futures_util::stream::StreamExt;
use reqwest::{Client, Error, Response};
use serde_json::{json, Value};
use std::str;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Client::new();
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&json!({
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

async fn handle_stream(response: Response) -> Result<(), Error> {
    let mut stream = response.bytes_stream();
    let mut final_response = String::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                if let Ok(text) = str::from_utf8(&bytes) {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if let Some(response_text) = json["response"].as_str() {
                            final_response.push_str(response_text);
                        }
                        if json.get("done").and_then(Value::as_bool).unwrap_or(false) {
                            println!("Final Response: {}", final_response);
                            break;
                        }
                    }
                }
            }
            Err(e) => eprintln!("Stream error: {}", e),
        }
    }

    Ok(())
}
