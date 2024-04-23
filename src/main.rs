use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use futures_util::stream::StreamExt;
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::str;

/// A simple CLI tool to interact with an API
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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

        /// The system message to set the behavior of the model
        #[arg(short, long)]
        system: Option<String>,

        /// Template string
        #[arg(short, long)]
        template: Option<String>,

        /// Raw output flag
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        raw: bool,

        /// Keep connection alive
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        keep_alive: bool,

        /// File to read/write context JSON
        #[arg(long)]
        context_file: Option<String>,
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
    let Commands::ApiGenerate {
        model,
        prompt,
        system,
        template,
        raw,
        keep_alive,
        context_file,
    } = command;
    let mut context = None;
    if let Some(file_path) = context_file {
        if let Ok(file) = File::open(file_path) {
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents)?;
            context = Some(contents);
        }
    }

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
            .insert("system".to_string(), json!(system));
    }
    if let Some(template) = template {
        payload
            .as_object_mut()
            .unwrap()
            .insert("template".to_string(), json!(template));
    }
    if let Some(context) = context {
        payload["context"] = serde_json::from_str(&context)
            .with_context(|| format!("Failed to parse context JSON: {}", context))?;
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
        let context_value = handle_stream(response).await?;
        if let Some(file_path) = context_file {
            if let Some(context) = context_value {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(file_path)?;
                file.write_all(serde_json::to_string_pretty(&context)?.as_bytes())?;
            }
        }
        return Ok(());
    } else {
        return Err(anyhow::anyhow!("Received HTTP {}", response.status()));
    }
}

async fn handle_stream(response: Response) -> Result<Option<Value>> {
    let mut stream = response.bytes_stream();
    let mut context_value = None;
    let mut json_buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let text = str::from_utf8(&bytes)
            .with_context(|| format!("Failed to convert bytes to UTF-8 string: {:?}", bytes))?;

        json_buffer.push_str(text);

        loop {
            match serde_json::from_str::<Value>(&json_buffer) {
                Ok(json) => {
                    if json.get("done").and_then(Value::as_bool).unwrap_or(false) {
                        context_value = json.get("context").cloned();
                        break; // Indicate completion of the stream
                    }
                    if let Some(response_text) = json["response"].as_str() {
                        print!("{}", response_text);
                        std::io::stdout().flush().unwrap();
                    }
                    json_buffer.clear();
                }
                Err(e) => {
                    if e.is_eof() {
                        // Incomplete JSON, continue accumulating chunks
                        break;
                    } else {
                        // Invalid JSON, print the error and clear the buffer
                        eprintln!("Error parsing JSON: {}", e);
                        eprintln!("Problematic JSON text: {}", json_buffer);
                        json_buffer.clear();
                        break;
                    }
                }
            }
        }
    }
    println!(); // Add a newline after the last chunk
    Ok(context_value)
}
