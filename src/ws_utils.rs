#![allow(dead_code, unused)]
use tokio_tungstenite::{accept_async, tungstenite::Error};
use std::collections::HashMap;
use std::sync::Arc;
use chatgpt::types::*;
use cognitive_services_speech_sdk_rs::audio::*;
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use log::*;
use tiktoken_rs::p50k_base;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::*;
use warp::ws::*;
use crate::gpt_client::{create_client, getSystemMessage};
use crate::{helpers_recog, helpers_synth};
use crate::rs_generic_types::{BinaryAudioStreamWriter, IncomingMessage, Media};
use crate::redis_client::get_call_status;
use crate::telnyx::hangup_call;
use crate::warp::Filter;



pub struct CurrentModel {
    pub model_name: String,
    pub token_limit: i32,
}
pub fn get_curr_model() -> CurrentModel {

    let model = std::env::var("current_gpt_model").unwrap_or("gpt-3.5-turbo-16k-0613".to_string());
    let token_limit = match model.as_str() {
        "gpt-3.5-turbo-16k-0613" => 15500,
        _ => 6500,
    };

    let model = CurrentModel{
        model_name: model,
        token_limit
    };

    return  model
}

pub fn modify_chat_for_token_limitV2(mut chat_log: Vec<ChatMessage>, is8kmodel: bool) -> Vec<ChatMessage> {
    // Initialize text assembly for encoding
    let mut aggregated_text = String::new();
    let bpe_processor = p50k_base().expect("Failed to initialize BPE processor");

    for item in chat_log.iter() {
        aggregated_text.push_str(&format!("Position: {:?}, Msg: {}\n", item.role, item.content));
    }

    // Encode the aggregated text
    let bpe = p50k_base().unwrap();
    let token_sequence = bpe.encode_with_special_tokens(aggregated_text.as_str());
    let num_tokens = token_sequence.len() as i32;
    println!("GPT_Queue finished Token count: {:?}", num_tokens);

    // Determine the token limit based on the model
    let token_limit = get_curr_model().token_limit;

    // If the token count exceeds the limit
    if num_tokens >= token_limit {
        let total_len = chat_log.len();
        let mut initial_system_detected = false;
        let mut last_system_idx = None;

        // Identify the first and last 'System' messages
        for index in 0..total_len {
            if chat_log[index].role == Role::System {
                if !initial_system_detected {
                    initial_system_detected = true;
                } else {
                    last_system_idx = Some(index);
                }
            }
        }

        // Empty all 'System' messages except the first and last ones
        for index in 0..total_len {
            if chat_log[index].role == Role::System && Some(index) != last_system_idx && initial_system_detected {
                chat_log[index].content = String::new();
            }
        }
    }

    chat_log
}
pub fn modify_chat_for_token_limit(mut chat_log: Vec<ChatMessage>, curr_model: CurrentModel) -> Vec<ChatMessage> {
    // Initialize text assembly for encoding
    let mut aggregated_text = String::new();
    let bpe_processor = p50k_base().expect("Failed to initialize BPE processor");

    for item in chat_log.iter() {
        aggregated_text.push_str(&format!("Position: {:?}, Msg: {}\n", item.role, item.content));
    }

    // Encode the aggregated text
    let bpe = p50k_base().unwrap();
    let token_sequence = bpe.encode_with_special_tokens(aggregated_text.as_str());
    let num_tokens = token_sequence.len() as i32;
    println!("GPT_Queue finished Token count: {:?}", num_tokens);

    // Determine the token limit based on the model
    // let token_limit = if curr_model.model_name.contains( "3.5") {15500  } else { 6500 };
    let token_limit = get_curr_model().token_limit;

    // If the token count exceeds the limit
    if num_tokens >= token_limit {
        let total_len = chat_log.len();
        let mut initial_system_detected = false;
        let mut last_system_idx = None;

        // Identify the first and last 'System' messages
        for index in 0..total_len {
            if chat_log[index].role == Role::System {
                if !initial_system_detected {
                    initial_system_detected = true;
                } else {
                    last_system_idx = Some(index);
                }
            }
        }

        // Empty all 'System' messages except the first and last ones
        for index in 0..total_len {
            if chat_log[index].role == Role::System && Some(index) != last_system_idx && initial_system_detected {
                chat_log[index].content = String::new();
            }
        }
    }

    chat_log
}


pub fn create_chat_message(key:String, value: String) -> ChatMessage {
    if key == "user".to_string() {
        return  ChatMessage {
            role: Role::User,
            content:value,
        };
    } else if key == "assistant".to_string() {
        return  ChatMessage {
            role: Role::Assistant,
            content: value,
        }
    } else  {
        return  ChatMessage {
            role: Role::System,
            content: value,
        }
    }
}
pub fn get_latest_chat_history(chat_db: &sled::Db) -> Vec<ChatMessage> {
    let mut iter_history = chat_db.range::<&[u8], _>(..).rev().take(4);
    let mut history: Vec<ChatMessage> = Vec::new();
    for item in iter_history {
        let (key, value) = item.unwrap();
        if key == "user".as_bytes() {
            history.push(ChatMessage {
                role: Role::User,
                content: String::from_utf8(value.to_vec()).unwrap(),
            });
        } else if key == "assistant".as_bytes() {
            history.push(ChatMessage {
                role: Role::Assistant,
                content: String::from_utf8(value.to_vec()).unwrap(),
            });
        } else if key == "system".as_bytes() {
            history.push(ChatMessage {
                role: Role::System,
                content: String::from_utf8(value.to_vec()).unwrap(),
            });
        }
    }
    history
}
pub async fn append_to_file(call_id: &str, history:String) -> std::io::Result<()> {
    let filename = format!("chat_history_{}.txt", call_id);

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        // .truncate(true) // This line ensures the file is truncated before writing.
        .create(true)
        .open(filename)
        .await?;
    file.write_all(format!("Content: {}\n", history).as_bytes()).await?;

    Ok(())
}
pub fn is_valid_ssml(ssml_string: &str) -> bool {
    let pattern = r"<speak\b[^>]*\bxmlns:mstts\b[^>]*>";
    match regex::RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
    {
        Err(err) => {
            eprintln!("Error occurred while validating SSML: {:?}", err);
            false
        }
        Ok(re) => re.is_match(ssml_string),
    }
}