extern crate log;
extern crate serde_json;

use std::collections::HashMap;
use std::env;
use log::error;
use log::info;
use reqwest;
use serde_json::Value;

use crate::rs_generic_types::{CallInfoResponse, ResponseWrapper, TelnyxData, TelnyxInboundMessage};
use crate::redis_client::update_call_status;

pub async fn get_call_info(call_control_id: String) -> Option<CallInfoResponse> {
    let telnyx_api_url = std::env::var("TELNYX_API_URL").expect("TELNYX_API_URL not set in environment");
    let telnyx_api_key = std::env::var("TELNYX_API_KEY").expect("TELNYX_API_KEY not set in environment");
    let api_url = format!("{}/v2/calls/{}", telnyx_api_url, call_control_id);

    let client = reqwest::Client::builder().build().ok()?;
    let request_builder = client.get(api_url)
        .header("Authorization", format!("Bearer {}", telnyx_api_key));

    let response = request_builder.send().await.ok()?;

    if response.status().is_success() {
        let call_info_response: CallInfoResponse = response.json().await.ok()?;
        println!("call info done {:?}", call_info_response);
        Some(call_info_response)
    } else {
        // Handle the error response here
        let response_text = response.text().await.unwrap(); // Unwrapping the Result
        error!("send_dtmf {:?}", response_text);
        None
    }
}


pub async fn send_dtmf(call_control_id: String) ->  Option<()> {

    let telnyx_api_url = std::env::var("TELNYX_API_URL").unwrap_or("https://api.telnyx.com".to_string());
    let telnyx_api_key = std::env::var("TELNYX_API_KEY").expect("TELNYX_API_KEY is not set");
    let api_url = format!("{}/v2/calls/{}/actions/send_dtmf", telnyx_api_url, call_control_id);

    let mut map = HashMap::<String, Value>::new();
    map.insert("digits".to_string(), Value::String("1111".to_string()));
    map.insert("duration_millis".to_string(), Value::Number(500.into()));

    let client = reqwest::Client::builder().build().ok()?;
    let mut request_builder = client.post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", telnyx_api_key))
        .header("Accept", "application/json")
        .json(&map);

    let response = request_builder.send().await.ok()?;

    if response.status().is_success() {
        let response_wrapper: ResponseWrapper = response.json().await.ok()?;
        println!("send_dtmf info done {:?}", response_wrapper);
        Some(())
    } else {
        // Handle the error response here
        let response_text = response.text().await.unwrap(); // Unwrapping the Result

        error!("send_dtmf {:?}", response_text);
        None
    }


}


pub async fn hangup_call(call_control_id: String) ->  Option<()> {

    let telnyx_api_url = std::env::var("TELNYX_API_URL").unwrap_or("https://api.telnyx.com".to_string());
    let telnyx_api_key = std::env::var("TELNYX_API_KEY").expect("TELNYX_API_KEY is not set");
    let api_url = format!("{}/v2/calls/{}/actions/hangup", telnyx_api_url, call_control_id);

    let mut map = HashMap::<String, Value>::new();
    map.insert("digits".to_string(), Value::String("1www2WABCDw9".to_string()));
    map.insert("duration_millis".to_string(), Value::Number(500.into()));

    let client = reqwest::Client::builder().build().ok()?;
    let mut request_builder = client.post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", telnyx_api_key))
        .header("Accept", "application/json")
        .json(&map);

    let response = request_builder.send().await.ok()?;

    if response.status().is_success() {
        let response_wrapper: ResponseWrapper = response.json().await.ok()?;
        println!("hangup_call info done {:?}", response_wrapper);
        Some(())
    } else {
        // Handle the error response here
        let response_text = response.text().await.unwrap(); // Unwrapping the Result

        error!("hangup_call error {:?}", response_text);
        None
    }


}
pub async fn handle_call_events(data: TelnyxInboundMessage) -> Result<(), Box<dyn std::error::Error>> {
    match &data.data {
        Some(data) => {
            let data_cloned_for_reids = data.clone();
            let call_control_id = data.payload.as_ref().and_then(|payload| payload.call_control_id.clone());
            let call_control_id_reids = call_control_id.clone();

            if let Some(call_control_id_reids_val) = call_control_id_reids{
                if let Some (event_type_for_redis) =  data_cloned_for_reids.event_type{
                    if event_type_for_redis == "call.playback.started" {
                        if let Err(e) = update_call_status(call_control_id_reids_val, event_type_for_redis) {
                            println!("redis_result {:?}", e);
                        } else {
                            println!("redis_result success playback started");
                        }
                    } else  if event_type_for_redis == "call.answered" {
                        if let Err(e) = update_call_status(call_control_id_reids_val, "call.playback.ended".to_string()) {
                            println!("redis_result {:?}", e);
                        } else {
                            println!("redis_result success playback ended ");
                        }
                    } else  if event_type_for_redis == "call.playback.ended" {
                        if let Err(e) = update_call_status(call_control_id_reids_val, event_type_for_redis) {
                            println!("redis_result {:?}", e);
                        } else {
                            println!("redis_result success playback ended ");
                        }
                    } else  if event_type_for_redis == "call.hangup" {
                        if let Err(e) = update_call_status(call_control_id_reids_val, event_type_for_redis) {
                            println!("redis_result {:?}", e);
                        } else {
                            println!("redis_result success playback ended ");
                        }
                    }

                }
            }
            match data.event_type.as_deref() {
                Some("call.initiated") => {
                    let telnyx_api_url = std::env::var("TELNYX_API_URL").unwrap_or("https://api.telnyx.com".to_string());
                    let telnyx_api_key = std::env::var("TELNYX_API_KEY").expect("TELNYX_API_KEY is not set");
                    let api_url = format!("{}/v2/calls/{}/actions/answer", telnyx_api_url, call_control_id.as_ref().unwrap());


                    let  map = Some(HashMap::<String,String>::new());
                    let client = reqwest::Client::builder().build()?;
                    let mut request_builder = client.post(api_url)
                        .header("Content-Type", "application/json")
                        .header("Authorization", format!("Bearer {}", telnyx_api_key))
                        .header("Accept", "application/json");
                    if let Some(json_map) = map {
                        request_builder = request_builder.json(&json_map);
                    }
                    let send_res_call_ini_answer = request_builder.send().await?;
                    println!("send_res_call_ini_answer ResponseDetails11  {:?}", &send_res_call_ini_answer.status());
                }
                Some("streaming.failed") => println!("streaming.failed {:?}", call_control_id),
                Some("streaming.stopped") => println!("streaming.stopped {:?}", call_control_id),
                Some("call.answered3") => println!("{:?}", call_control_id),
                Some("call.answered") => {
                    println!("111handle_call_answered_call_start_stream_get_audio_delivered_as64encoded_audio_AKA_RTP called");
                    let handle_call_answered_result = handle_call_answered_call_start_stream_get_audio_delivered_as64encoded_audio_AKA_RTP(call_control_id.as_ref().unwrap().as_str())
                        .await;
                    println!("handle_call_answered_result Yess {:?}", handle_call_answered_result);
                }
                Some("call.bridged") => {
                    // Handle "call.bridged" event
                }
                Some("call.hangup") => log_output(format!("Call hangup: {}\n", call_control_id.as_ref().unwrap()), "#0000FF"),
                Some("call.fork.started") => log_output(format!("Call fork started: {}\n", serde_json::to_string(&data)?), "#0000FF"),
                Some("call.fork.stopped") => log_output(format!("Call fork stopped: {}\n", serde_json::to_string(&data)?), "#0000FF"),
                Some("call.playback.started") => {
                    log_output(format!("Call playback started: {}\n", serde_json::to_string(&data)?), "#0000FF");
                    handle_event(data)?; // call the function with your eventJson
                }
                Some("call.playback.ended") => {
                    log_output(format!("Call playback ended: {}\n", serde_json::to_string(&data)?), "#0000FF");
                    let _telnyx_data =
                    handle_event(data)?; // call the function with your eventJson
                    // sendMP3ToWebSocket('hash.mp3', data.payload.call_control_id, ws, 10);
                }
                _ => log_output(format!("Call unknown: {}\n", serde_json::to_string(&data)?), "#0000FF"),
            }
        }
        None => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Missing data in message"))),
    }
    Ok(())
}

/// here we get the call id, then we inform telnyx about where to send the audio track (inbound => coming from phone talker);
/// once audio delviered to ws server, it will recoginize it then it will try to send it back to???? web socket same
pub async fn handle_call_answered_call_start_stream_get_audio_delivered_as64encoded_audio_AKA_RTP(
    call_control_id: &str,
) -> Result<(), reqwest::Error> {
    let ws_url = env::var("WS_URL")
        .unwrap_or_else(|_| "wss://606a-142-198-199-243.ngrok-free.app".to_string());

    let telnyx_api_url = std::env::var("TELNYX_API_URL").unwrap_or("https://api.telnyx.com".to_string());
    let telnyx_api_key = std::env::var("TELNYX_API_KEY").expect("TELNYX_API_KEY is not set");
    info!("Call answered: {}", call_control_id);
    info!("Connecting call leg to websocket: {}", call_control_id);
    let ws_uri = format!("{}?trigger=call.answered&call_id={}", ws_url, call_control_id);
    let api_url = format!("{}/v2/calls/{}/actions/streaming_start", telnyx_api_url, call_control_id);
    info!("wsUri {}", ws_uri);
    info!("apiUrl {}", api_url);
    let mut amap = HashMap::<String,String>::new();
    amap.insert("stream_track".to_string(), "inbound_track".to_string());
    amap.insert("stream_url".to_string(), ws_uri.to_string());

    let map = Some(amap);
    let client = reqwest::Client::builder().build()?;
    let mut request_builder = client.post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", telnyx_api_key))
        .header("Accept", "application/json");

    if let Some(json_map) = map {
        request_builder = request_builder.json(&json_map);
    }
    let send_res_call_ini_answer = request_builder.send().await?;
    println!("send_res_call_ini_answer ResponseDetails22  {:#?}", send_res_call_ini_answer);
    Ok(())
}

pub fn handle_event(data: &TelnyxData) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}


pub fn log_output(message: String, _color: &str) {
    // log::info!("{}", message);
}
