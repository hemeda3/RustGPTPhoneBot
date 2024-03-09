use std::collections::HashMap;
use std::env;
use chatgpt::client::ChatGPT;
use chatgpt::types::{ChatMessage, CompletionResponse, Role, ServerResponse};
use log::error;
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap};
use serde_json::{from_slice, json, Value};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use crate::gql_client::search_products_function;
use crate::rs_generic_types::{ChatRequest, FunctionParam, GptStreamCompletionResponse, GptStreamFunctionCall, GptStreamServerResponse, Message, SearchProductsFunctionArgs};
use crate::ws_utils::get_curr_model;


pub async fn call_gpt_without_function(_input_message: String, history: Vec<ChatMessage>) -> Option<GptStreamCompletionResponse> {
    let url = "https://api.openai.com/v1/chat/completions";
    let api_key = env::var("OPENAI_KEY").expect("OPENAI_KEY not found in environment");

    let mut cloned_history = history.clone();
    cloned_history.push(ChatMessage {
        role: Role::User,
        content: _input_message,
    });

    let messages: Vec<Message> = cloned_history.into_iter().map(|message| {
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            _ => "system",
        };
        Message {
            role: Option::from(role.to_string()),
            content: Option::from(message.content),
        }
    }).collect();

    let request = ChatRequest {
        model: get_curr_model().model_name,
        temperature: 0,
        messages,
        functions: vec![],
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(AUTHORIZATION, format!("Bearer {}", api_key).parse().unwrap());

    let client = reqwest::Client::new();
    match client.post(url)
        .headers(headers)
        .json(&request)
        .send()
        .await {
        Ok(res) => match res.json::<GptStreamServerResponse>().await {
            Ok(GptStreamServerResponse::Completion(completion)) => Some(completion),
            _ => None,
        },
        Err(_) => None,
    }
}
