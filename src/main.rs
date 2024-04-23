use reqwest::Client;
use std::error::Error;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    let request_body = r#"{
        "model": "llama2",
        "prompt": "Why is the sky blue?"
    }"#;

    let response = client.post(url).body(request_body).send().await?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let json_str = std::str::from_utf8(&chunk)?;
        println!("{}", json_str);
    }

    Ok(())
}
