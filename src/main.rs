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
        /// The context parameter returned from a previous request
        #[arg(short, long)]
        context: Option<String>,
        /// Whether to return the raw response from the model
        #[arg(short, long)]
        raw: bool,
        /// The duration to keep the model loaded in memory following the request
        #[arg(short, long, default_value = "5m")]
        keep_alive: String,
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
                json_data["context"] = serde_json::from_str(context)
                    .with_context(|| format!("Failed to parse context as JSON: {}", context))?;
            }
            if *raw {
                json_data["raw"] = serde_json::Value::Bool(true);
            }
            json_data["keep_alive"] = serde_json::Value::String(keep_alive.clone());
            let response = request.json(&json_data).send().await?;
            if response.status().is_success() {
                let context_value = handle_stream(response).await?;
                if let Some(context) = context_value {
                    println!("Context: {}", serde_json::to_string_pretty(&context)?);
                }
            } else {
                eprintln!("Received HTTP {}", response.status());
            }
        }
        None => println!("No subcommand was used"),
    }
    Ok(())
}

async fn handle_stream(response: Response) -> Result<Option<Value>> {
    let mut stream = response.bytes_stream();
    let mut context_value = None;
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
        if let Some(context) = json.get("context") {
            context_value = Some(context.clone());
        }
    }
    println!(); // Add a newline after the last chunk
    Ok(context_value)
}
