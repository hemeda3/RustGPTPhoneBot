use std::collections::HashMap;
use std::env;
use chatgpt::client::ChatGPT;
use chatgpt::types::{ChatMessage, CompletionResponse, Role, ServerResponse};
use log::{error, info};
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap};
use serde_json::{from_slice, json, Value};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use crate::constants::FAQS;
use crate::gpt_without_function::call_gpt_without_function;
use crate::gql_client::search_products_function;
use crate::rs_generic_types::{ChatRequest, FaqFunctionArgs, FunctionParam, GptStreamCompletionResponse, GptStreamFunctionCall, GptStreamServerResponse, Message, SearchProductsFunctionArgs};
use crate::ws_utils::{create_chat_message, get_curr_model};


pub async fn call_gpt_with_function( _input_message: String, history: Vec<ChatMessage>) -> Option<GptStreamCompletionResponse> {
    let get_faq_answer_function = FunctionParam {
        name: Some("fetch_answer_to_frequently_asked_questions".to_string()),
        description: Some("Retrieve detailed responses for shoppers/SDM FAQs ".to_string()),
        parameters: Some(json!({
        "type": "object",
        "properties": {
            "faq_question": { "type": "string", "description": "The question you seek an answer to about shoppers drug mart " },
            "additional_context": { "type": "string", "description": "Any extra context to improve the answer " }
        },
        "required": ["faq_question"]
    }))
    };
    let get_current_weather_function =   FunctionParam {
        name:Some( "get_current_weather".to_string()),
        description: Some("Get the current weather in a given location".to_string()),
        parameters: Some(json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" },
                "unit": { "type": "string", "enum": ["celsius", "fahrenheit"] }
            },
            "required": ["location"]
        }))};
    let search_products_function = FunctionParam {
        name: Some("search_products_function".to_string()),
        description: Some("Search products in shoppers drug mart using array of queryPrams".to_string()),
        parameters: Some(json!({
        "type": "object",
        "properties": {
            "userSearchTerm": { "type": "string" },
            "queryPrams": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["queryPrams"]
    }))
    };

    let find_flyers_by_postal_function = FunctionParam {
        name: Some("find_flyers_by_postal".to_string()),
        description: Some("Fetches flyers by postal code from a specific store (e.g., Shoppers Drug Mart). Requires postal code as an input parameter.".to_string()),
        parameters: Some(json!({
        "type": "object",
        "properties": {
            "postal_code": { "type": "string" }
        },
        "required": ["postal_code"]
    })),
    };

    let url = "https://api.openai.com/v1/chat/completions";
    let api_key = env::var("OPENAI_KEY").expect("OPENAI_KEY not found in environment");

    let mut cloned_history = history.clone();

    cloned_history.push(ChatMessage{
        role: Role::User,
        content: _input_message

    });
    let functions = vec![

        get_current_weather_function,
        get_faq_answer_function,
        search_products_function

    ];

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
        functions,

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
        Ok(res) => {
            // let resp = res.text().await;
            match res.json::<GptStreamServerResponse>().await {
                Ok(GptStreamServerResponse::Completion(completion)) => {
                    println!("HTTP Status: Completion {:?}", completion);
                    Some(completion)
                },
                Ok(GptStreamServerResponse::Error { error }) => {
                    println!("HTTP Status: Error {:?}", error);
                    None
                },
                Err(e) => {
                    println!("Decode error: {:?}", e);
                    None
                }
            }
        },
        Err(e) => {
            println!("ServerResponse error3: {:?}", e); // Add this
            None
        },
    }

}


pub async fn call_function_dynamically(func_call: &GptStreamFunctionCall) -> Option<String> {
    match func_call.name.as_str() {
        "get_current_weather" => handle_get_current_weather(func_call),
        "search_products_function" => handle_search_products_function(func_call).await,
        "get_faq_answer_function" => handle_get_faq_answer_function(func_call).await,
        _ => None,
    }
}
fn handle_get_current_weather(func_call: &GptStreamFunctionCall) -> Option<String> {
    let args_res: serde_json::Result<HashMap<String, String>> = serde_json::from_str(&func_call.arguments);
    match args_res {
        Ok(_) => Some("The weather is good".to_string()),
        Err(_) => None,
    }
}
async fn handle_search_products_function(func_call: &GptStreamFunctionCall) -> Option<String> {
    let args_res: serde_json::Result<SearchProductsFunctionArgs> = serde_json::from_str(&func_call.arguments);
    match args_res {
        Ok(a) => search_products_function(a.queryPrams).await,
        Err(_) => None,
    }
}
async fn handle_get_faq_answer_function(func_call: &GptStreamFunctionCall) -> Option<String> {
    // Deserialize to FaqFunctionArgs
    let args_res: serde_json::Result<FaqFunctionArgs> = serde_json::from_str(&func_call.arguments);
    info!("handle_get_faq_answer_function Deserialization successful: {:?}", args_res);

    match args_res {
        Ok(faq_args) => {
            info!("handle_get_faq_answer_function Deserialization successful: {:?}", faq_args);
            let query_pram = &faq_args.faq_question.unwrap_or_default(); // Handle None as you wish
            let mut history: Vec<ChatMessage> = Vec::new();
            history.push(create_chat_message("system".to_string(), FAQS.to_string()));
            info!("handle_get_faq_answer_function Query and history prepared: {}, {:?}", query_pram, history);

            if let Some(gpt_response) = call_gpt_without_function(query_pram.to_string(), history).await {
                info!("handle_get_faq_answer_function GPT Response: {:?}", gpt_response);
                handle_first_choice(&gpt_response)
            } else {
                error!("handle_get_faq_answer_function GPT Response is None");
                None
            }
        },
        Err(e) => {
            error!("handle_get_faq_answer_function Deserialization failed: {:?}", e);
            None
        },
    }
}


fn handle_first_choice(gpt_response: &GptStreamCompletionResponse) -> Option<String> {
    if gpt_response.choices.len() > 0 {
        if let Some(first_choice) = gpt_response.choices.first() {
            let message = &first_choice.message;
            if message.function_call.is_some() {
                error!("Only 1 level of function call allowed");
            }
            message.content.clone()
        } else {
            default_error_response()
        }
    } else {
        default_error_response()
    }
}

fn default_error_response() -> Option<String> {
    Some(String::from("Sorry something went wrong"))
}