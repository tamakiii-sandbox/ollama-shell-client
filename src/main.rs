use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use futures_util::stream::StreamExt;
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;
use std::str;

/// A simple CLI tool to interact with an API
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Supported system types
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum System {
    Base,
    Enhanced,
}

impl std::fmt::Display for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            System::Base => write!(f, "base"),
            System::Enhanced => write!(f, "enhanced"),
        }
    }
}

/// API generation command options
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

        /// Specify system type
        #[arg(short, long, value_enum)]
        system: Option<System>,

        /// Template string
        #[arg(short, long)]
        template: Option<String>,

        /// Context as JSON
        #[arg(short, long)]
        context: Option<String>,

        /// Output file to save the context from the API response
        #[arg(long)]
        context_out: Option<String>,

        /// Raw output flag
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        raw: bool,

        /// Keep connection alive
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        keep_alive: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(command) => call_api_generate(command).await?,
        None => println!("No subcommand was used"),
    }

    Ok(())
}

async fn call_api_generate(command: &Commands) -> Result<()> {
    match command {
        Commands::ApiGenerate {
            model,
            prompt,
            system,
            template,
            context,
            context_out,
            raw,
            keep_alive,
        } => {
            let client = Client::new();
            let mut payload = json!({
                "model": model,
                "prompt": prompt,
            });

            // Optionally add other parameters if they are provided
            if let Some(system) = system {
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("system".to_string(), json!(system.to_string()));
            }
            if let Some(template) = template {
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("template".to_string(), json!(template));
            }
            if let Some(context) = context {
                let context_json: Value = serde_json::from_str(context)
                    .with_context(|| format!("Failed to parse context JSON: {}", context))?;
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("context".to_string(), context_json);
            }
            if *raw {
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("raw".to_string(), json!(true));
            }
            if *keep_alive {
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("keep_alive".to_string(), json!(true));
            }

            let response = client
                .post("http://localhost:11434/api/generate")
                .json(&payload)
                .send()
                .await?;

            if response.status().is_success() {
                if let Some(context) = handle_stream(response).await? {
                    if let Some(context_out) = context_out {
                        save_context_to_file(&context, context_out)?;
                    }
                }
                return Ok(());
            } else {
                return Err(anyhow::anyhow!("Received HTTP {}", response.status()));
            }
        }
    }
}

async fn handle_stream(response: Response) -> Result<Option<Value>> {
    let mut stream = response.bytes_stream();
    let mut context_value = None;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let text = str::from_utf8(&bytes)
            .with_context(|| format!("Failed to convert bytes to UTF-8 string: {:?}", bytes))?;
        match serde_json::from_str::<Value>(text) {
            Ok(json) => {
                if json.get("done").and_then(Value::as_bool).unwrap_or(false) {
                    context_value = json.get("context").cloned();
                    break; // Indicate completion of the stream
                }
                if let Some(response_text) = json["response"].as_str() {
                    print!("{}", response_text);
                    std::io::stdout().flush().unwrap();
                }
            }
            Err(e) => {
                eprintln!("Error parsing JSON: {}", e);
            }
        }
    }
    println!(); // Add a newline after the last chunk
    Ok(context_value)
}

fn save_context_to_file(context: &Value, path: &str) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(context.to_string().as_bytes())?;
    Ok(())
}
