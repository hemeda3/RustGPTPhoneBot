#![allow(dead_code, unused)]

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use base64::{decode, encode};
use chatgpt::prelude::ChatMessage;
use chatgpt::types::Role;
use cognitive_services_speech_sdk_rs::audio::{AudioConfig, AudioStreamFormat, PushAudioInputStream, PushAudioOutputStreamCallbacks};
use cognitive_services_speech_sdk_rs::speech::SpeechConfig;
use futures_util::TryFutureExt;
use futures_util::sink::SinkExt;
use futures_util::stream::SplitSink;
use futures_util::StreamExt;
use log::{error, info, warn};
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_json::value::Index;
use sled::Db;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex, MutexGuard, RwLock};
use tokio::time::{Duration, sleep};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{accept_async, tungstenite::Error};
use tokio_tungstenite::WebSocketStream;
use warp::Filter;
use warp::ws::{WebSocket, Ws};

use crate::{helpers_recog, helpers_synth};
use crate::gpt_client::{chat_gpt_process_conversation_directed2, create_client, getSystemMessage};
use crate::redis_client::{get_call_status, update_call_status};
use crate::telnyx::{get_call_info, hangup_call, send_dtmf};

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct MediaFormat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u8>,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct Start {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub  user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_control_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_format: Option<MediaFormat>,
}
#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct Stop {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_control_id: Option<String>,
}
#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct Media {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
}
#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct IncomingMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // sourceOfMessage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Start>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Media>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}


pub struct BinaryAudioStreamWriter {
    pub   final_audio_data: Vec<u8>,
    pub   sender: mpsc::Sender<Vec<u8>>,
}

impl BinaryAudioStreamWriter {
    pub fn new(sender: mpsc::Sender<Vec<u8>>) -> Self {
        BinaryAudioStreamWriter {
            final_audio_data: vec![],
            sender,
        }
    }
}

impl PushAudioOutputStreamCallbacks for BinaryAudioStreamWriter {
    fn write(&mut self, data_buffer: &[u8]) -> u32 {
        info!("BinaryAudioStreamWriter::write called");
        let mut data_buffer_vec = data_buffer.to_vec();
        self.final_audio_data.append(&mut data_buffer_vec);
        data_buffer_vec.len() as u32
    }

    fn close(&mut self) {
        info!("BinaryAudioStreamWriter::close called");
        let audio_bytes = self.final_audio_data.clone();
        // self.sender.send(audio_bytes).unwrap();
    }
}




#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct TelnyxPayload {
    pub call_control_id: Option<String>,
    pub call_leg_id: Option<String>,
    pub call_session_id: Option<String>,
    pub client_state: Option<String>,
    pub connection_id: Option<String>,
    pub direction: Option<String>,
    pub from: Option<String>,
    pub state: Option<String>,
    pub to: Option<String>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct TelnyxData {
    pub event_type: Option<String>,
    pub id: Option<String>,
    pub occurred_at: Option<String>,
    pub payload: Option<TelnyxPayload>,
    pub record_type: Option<String>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct TelnyxMeta {
    pub attempt: Option<i32>,
    pub delivered_to: Option<String>,
}




#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseWrapper {
    data: Option<DataWrapper>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataWrapper {
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct CallInfoResponse {
    pub data: CallInfoData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CallInfoData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_control_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_leg_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_alive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_duration: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_type: Option<String>,
}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Usage {
    pub completion_tokens: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Choice {
    pub finish_reason: Option<String>,
    pub index: Option<i32>,
    pub logprobs: Option<serde_json::Value>,
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TextCompletion {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<i64>,
    pub model: Option<String>,
    pub choices: Option<Vec<Choice>>,
    pub usage: Option<Usage>,
    pub warning: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub temperature: i32,
    pub messages: Vec<Message>,
    pub functions: Vec<FunctionParam>,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}



#[derive(Debug, Deserialize)]
pub struct GptStreamFunctionCall {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct GptStreamMessage {
    pub role: String,
    pub content: Option<String>,
    pub(crate) function_call: Option<GptStreamFunctionCall>,
}

#[derive(Debug, Deserialize)]
pub struct GptStreamChoice {
    pub index: i32,
    pub(crate) message: GptStreamMessage,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct GptStreamCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub(crate) choices: Vec<GptStreamChoice>,
}

#[derive(Debug, Deserialize)]
pub struct GptStreamCompletionError {
    // your fields here
    /// Message, describing the error
    pub message: String,
    /// The type of error. Example: `server_error`
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GptStreamServerResponse {
    Error { error: GptStreamCompletionError },
    Completion(GptStreamCompletionResponse),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchProductsFunctionArgs {
    pub queryPrams: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GraphQLResponse {
    pub data: Option<Data>,

}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Product {
    pub badges: Option<Vec<Badge>>,
    pub brandName: Option<String>,
    pub id: Option<String>,
    pub isOnSale: Option<bool>,
    pub isOutOfStock: Option<bool>,
    pub name: Option<String>,
    pub optimumPoints: Option<OptimumPoints>,
    pub price: Option<Price>,
    pub salePrice: Option<SalePrice>,
    pub smallThumbnail: Option<String>,
    pub url: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SalePrice {
    pub formattedPrice: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Badge {
    pub uid: Option<String>,
    pub name: Option<String>,
    pub badge_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OptimumPoints {
    pub base: Option<i32>,
    pub bonus: Option<i32>,
    pub optimumPromoText: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Price {
    pub formattedPrice: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Pagination {
    pub currentPage: Option<i32>,
    pub pageSize: Option<i32>,
    pub totalPages: Option<i32>,
    pub totalResults: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SearchData {
    pub categoryType: Option<String>,
    pub pagination: Option<Pagination>,
    pub products: Option<Vec<Product>>,
    pub totalFiltersCount: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Data {
    pub searchData: Option<SearchData>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct TelnyxInboundMessage {
    pub data: Option<TelnyxData>,
    pub meta: Option<TelnyxMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqFunctionArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faq_question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}
