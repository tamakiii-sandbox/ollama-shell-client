// src/main.rs
use reqwest::Error;
use serde_json::json; // Ensure we use the JSON macro for easy payload construction

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create an instance of the client
    let client = reqwest::Client::new();

    // Prepare the POST request with the JSON payload
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&json!({
            "model": "llama3",
            "prompt": "Why is the sky blue?"
        }))
        .send()
        .await?;

    // Print the HTTP status of the response
    println!("Status: {}", response.status());

    // Optionally, we could check and print the response body here:
    // let body = response.text().await?;
    // println!("Body: {}", body);

    Ok(())
}
