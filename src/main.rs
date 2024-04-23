use futures_util::stream::StreamExt; // Correct import for StreamExt
use reqwest::{Client, Error, Response}; // Use Bytes, a more appropriate type for handling byte arrays

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

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    println!("Chunk: {}", text);
                }
            }
            Err(e) => eprintln!("Stream error: {}", e),
        }
    }

    Ok(())
}
