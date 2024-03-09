use reqwest::Client;
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::error::Error;
use base64;
use futures_util::stream::StreamExt;

pub async fn fetch_and_encode_audio_stream(input_text: &str) -> Option<String> {
    let http_client = match Client::builder().build() {
        Ok(client) => client,
        Err(_) => {
            println!("Failed to build HTTP client");
            return None;
        }
    };

    let mut http_headers = HeaderMap::new();

    http_headers.insert("xi-api-key", "".parse().unwrap());

    http_headers.insert("Content-Type", "application/json".parse().unwrap());


    let escaped_text = serde_json::to_string(&input_text).unwrap_or_else(|e| {
        println!("Failed to escape special characters: {:?}", e);
        return String::from("");
    });

    let payload_formatted = format!(r#"{{ "text": {}, "model_id": "eleven_multilingual_v1" }}"#, escaped_text);

    let json_result = serde_json::from_str::<serde_json::Value>(&payload_formatted);
    let json_value = match json_result {
        Ok(val) => val,
        Err(e) => {
            println!("Failed to parse JSON payload: {:?}, Original text: {:?}", e, input_text);
            return None;
        },
    };

    let req = http_client.request(reqwest::Method::POST, "https://api.elevenlabs.io/v1/text-to-speech/jsiKUHLIa2UYhMkRlIx2/stream")
        .headers(http_headers)
        .json(&json_value);
    let res = match req.send().await {
        Ok(response) => response,
        Err(_) => {
            println!("Failed to send request");
            return None;
        },
    };

    let mut collected_bytes = Vec::new();
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                // println!("Received chunk: {:?}", chunk);
                collected_bytes.extend_from_slice(&chunk);
            },
            Err(_) => {
                println!("Failed to receive chunk");
                return None;
            },
        }
    }

    let base64_encoded = base64::encode(collected_bytes);
    // println!("Base64 encoded string: {}", base64_encoded);

    Some(base64_encoded)
}


// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     let xxx =fetch_and_encode_audio_stream("hello").await;
//
//     println!("{:?}", xxx);
//     Ok(())
// }