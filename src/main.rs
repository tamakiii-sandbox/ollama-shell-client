use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use futures_util::stream::StreamExt;
use reqwest::{Client, Response};
use serde_json::Value;
use std::io::Write;
use std::str;

/// A simple CLI tool to interact with an API
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate text using the API
    ApiGenerate {
        /// The model to use
        #[arg(short, long)]
        model: String,
        /// The prompt to generate text from
        #[arg(short, long)]
        prompt: String,
        /// The system message to set the behavior of the model
        #[arg(short, long)]
        system: Option<String>,
        /// The template to use for generating text
        #[arg(short, long)]
        template: Option<String>,
        /// The context to provide to the model
        #[arg(short, long)]
        context: Option<String>,
        /// Whether to return the raw response from the model
        #[arg(short, long)]
        raw: bool,
        /// Whether to keep the connection alive for multiple requests
        #[arg(short, long)]
        keep_alive: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::ApiGenerate {
            model,
            prompt,
            system,
            template,
            context,
            raw,
            keep_alive,
        }) => {
            let client = Client::new();
            let request = client.post("http://localhost:11434/api/generate");
            let mut json_data = serde_json::json!({
                "model": model,
                "prompt": prompt,
            });
            if let Some(system) = system {
                json_data["system"] = serde_json::Value::String(system.clone());
            }
            if let Some(template) = template {
                json_data["template"] = serde_json::Value::String(template.clone());
            }
            if let Some(context) = context {
                json_data["context"] = serde_json::Value::String(context.clone());
            }
            if *raw {
                json_data["raw"] = serde_json::Value::Bool(true);
            }
            if *keep_alive {
                json_data["keep_alive"] = serde_json::Value::Bool(true);
            }
            let response = request.json(&json_data).send().await?;
            if response.status().is_success() {
                handle_stream(response).await?;
            } else {
                eprintln!("Received HTTP {}", response.status());
            }
        }
        None => println!("No subcommand was used"),
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
