use futures_util::stream::StreamExt;
use reqwest::{Client, Error, Response};
use serde_json::Value;
use std::str;

#[tokio::main]
async fn main() -> Result<(), Error> {
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

async fn handle_stream(response: Response) -> Result<(), Error> {
    let mut stream = response.bytes_stream();
    let mut complete_response = String::new();

    while let Some(chunk) = stream.next().await {
        let text_chunk = process_chunk(chunk).unwrap();
        if let Some(text) = text_chunk {
            complete_response.push_str(&text);
        } else {
            break;
        }
    }

    println!("Final Response: {}", complete_response);
    Ok(())
}

/// Processes a single chunk of the response stream, returns the text to append,
/// or None if the stream is complete.
fn process_chunk(
    chunk: Result<bytes::Bytes, Error>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match chunk {
        Ok(bytes) => {
            let text = str::from_utf8(&bytes)?;
            let json: Value = serde_json::from_str(text)?;
            if json.get("done").and_then(Value::as_bool).unwrap_or(false) {
                return Ok(None); // Indicate completion of the stream
            }
            Ok(Some(json["response"].as_str().unwrap_or("").to_string()))
        }
        Err(e) => Err(Box::new(e)),
    }
}
