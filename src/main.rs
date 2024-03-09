#![allow(dead_code)]
extern crate warp;

use std::collections::HashMap;
use std::env;
use log::info;
use serde_json::Value;
use warp::Filter;
use warp::ws::Ws;
use rs_generic_types::TelnyxInboundMessage;
use crate::telnyx::handle_call_events;
mod helpers_recog;
mod gpt_client;
mod helpers_synth;
mod telnyx;
mod ws_server_v2;
mod redis_client;
mod rs_generic_types;
mod ws_utils;
mod gcp_google_client;
mod gpt_stream_httpv2;
mod gql_client;
mod constants;
mod gpt_without_function;
mod eleven_labs;

use crate::ws_server_v2::handle_connection;
#[tokio::main]
async fn main() {

    info!("*****Welcome GPT VOIP Bot Server*****");
    // export WS_URL=SHOULD COME AFTER FIRST DEPLOY TO CLOUD RUN IS THE HOSTNAME WSS://hostname-gcp-cloudrun
    //export should_spawn_azure_task = true/false enable 11 lab voic chat
    //export current_gpt_model= gpt-3.5-turbo-16k-0613 or gpt4 will iimpact token limit to clean the history of messages
    // export TELNYX_API_KEY=

    // export REDIS_URI=redis- .redislabs.com
    // export REDIS_PASSWORD=

    // export OPENAI_KEY=sk-

    // export MSServiceRegion=eastus
    // export DYLD_FALLBACK_FRAMEWORK_PATH=~/speechsdk/MicrosoftCognitiveServicesSpeech.xcframework/macos-arm64_x86_64 // only macos for linux it's set in docker
    // export MACOS_SPEECHSDK_ROOT=/Users/ xxxx/speechsdk

    // 3612  export  MSSubscriptionKey=
    env::set_var("TELNYX_API_KEY", "xxxxxxxxxxxxxx");
    env::set_var("TELNYX_API_URL", "https://api.telnyx.com");
    env::set_var("REDIS_URI", "xxxxxxxxxxxxxx");
    env::set_var("REDIS_PORT", "12037xxxxxxx");
    env::set_var("REDIS_PASSWORD", "xxxxxxx");
    env::set_var("OPENAI_KEY", "sk-xxxxxxx");
    env::set_var("MSServiceRegion", "eastus");
    // env::set_var("DYLD_FALLBACK_FRAMEWORK_PATH", "~/speechsdk/MicrosoftCognitiveServicesSpeech.xcframework/macos-arm64_x86_64");
    // env::set_var("MACOS_SPEECHSDK_ROOT", "/Users/xxxx/speechsdk");
    env::set_var("MSSubscriptionKey", "");

    env::set_var("RUST_LOG", "info");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    // tokio::spawn(run_mobile_ws_des_server());// remove after making Websocket uses http due to GCP only support 8080 port, so rust will switch after handshake from htttp to websocket wss(htts) or ws(http)
    let route = warp::post()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("webhook"))
        .and(warp::path::tail())
        .and(warp::body::json::<Value>()) // Capture the JSON as a serde_json Value
        .and_then(move |_tail: warp::path::Tail, json_request: Value| async move {
            if let Ok(data) = serde_json::from_value::<TelnyxInboundMessage>(json_request) {
                let _ = handle_call_events(data).await;
            }
            Ok::<_, warp::Rejection>("Webhook message processed")
        });


    let ws_route = warp::ws()
        .and(warp::query::<HashMap<String, String>>())
        .map(|ws: Ws, query_params: HashMap<String, String>| {
            let call_id = query_params.get("call_id").unwrap_or(&String::from("No call_id")).clone();
            info!("opening websocket for Call ID: {}", call_id);
            ws.on_upgrade(move |websocket| async {
                if let Err(e) = handle_connection(websocket, call_id).await {
                    info!("Error handling websocket connection: {}", e);
                }
            })
        });
    let combined = route.or(ws_route);
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a number");
    warp::serve(combined)
        .run(([0, 0, 0, 0], port))
        .await;
}

