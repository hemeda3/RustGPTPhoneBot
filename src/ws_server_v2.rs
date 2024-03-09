#![allow(dead_code, unused)]
use tokio_tungstenite::{accept_async, connect_async, MaybeTlsStream, tungstenite::Error, WebSocketStream};
use std::collections::HashMap;
use std::sync::Arc;
use bytes::BytesMut;
use std::io::Read;
use chatgpt::types::*;
use cognitive_services_speech_sdk_rs::audio::*;
use cognitive_services_speech_sdk_rs::speech::{SpeechRecognizer, SpeechSynthesisResult};
use futures_util::{AsyncReadExt, pin_mut, SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use google_cognitive_apis::speechtotext::recognizer::Recognizer;
use log::*;
use reqwest::header::HeaderMap;
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::time::Duration;
use tiktoken_rs::p50k_base;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::*;
use warp::ws::*;
use websocket::futures::Future;
use crate::gpt_client::{create_client, getSystemMessage, get_system_message_shopping_assistant};
use crate::gpt_stream_httpv2::{call_function_dynamically, call_gpt_with_function};
use crate::{helpers_recog, helpers_synth};
use crate::constants::FAQS;
use crate::gcp_google_client::create_gcp_speech_recognizer;
use crate::gpt_without_function::call_gpt_without_function;
use crate::rs_generic_types::{BinaryAudioStreamWriter, IncomingMessage, Media};
use crate::redis_client::get_call_status;
use crate::telnyx::hangup_call;
use crate::warp::Filter;
use crate::ws_utils::{append_to_file, CurrentModel, get_curr_model, is_valid_ssml,modify_chat_for_token_limit, modify_chat_for_token_limitV2};
use crate::eleven_labs::fetch_and_encode_audio_stream;
use tungstenite::Result as TungsteniteResult;
use serde::{Deserialize, Serialize};

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
pub async fn handle_connection(ws_stream: WebSocket, call_id: String) -> Result<(), Error> {
    handle_incoming_messages(ws_stream, &call_id).await;
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    audio: Option<String>,
}
async fn fetch_and_output_audio(
    receiving_stream: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Option<AudioMessage> {
    if let Some(Ok(incoming_msg)) = receiving_stream.next().await {
        // println!("Successfully deserialized: {:?}", incoming_msg.clone());

        if incoming_msg.is_text() {
            let text_content =  incoming_msg.to_string() ;

            match serde_json::from_str::<AudioMessage>(text_content.as_str()) {
                Ok(audio_msg) => {
                    println!("Successfully deserialized: {:?}", audio_msg);
                    return Some(audio_msg);
                }
                Err(_) => {
                    println!("Failed to deserialize or server message: {}", text_content);
                }
            }
        }
    }
    None
}


#[derive(Serialize)]
struct InitializationMessage<'a> {
    text: &'a str,
    voice_opts: Option<VoiceConfig>,
    #[serde(rename = "xi-api-key")]
    api_key: Option<&'a str>,
    should_trigger: Option<bool>,
}

#[derive(Serialize)]
struct VoiceConfig {
    stability: f32,
    similar_enhance: bool,
}

pub async fn handle_incoming_messages(ws_stream: WebSocket, call_id: &String) {
    let callid_clone = call_id.clone();
    let callid_clone_for_hangup = call_id.clone();
    let callid_clone_base1 = call_id.clone();
    let callid_clone_for_elvenLabs = call_id.clone();
    let (mut tx_gcp_bytes_stream, mut rx_gcp_bytes_stream) = tokio::sync::mpsc::channel(32);
    let tx_gcp_bytes_stream_clone = tx_gcp_bytes_stream.clone();

    let stream_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let stream_id_clone3 = Arc::clone(&stream_id);
    let (mut user_ws_tx, mut user_ws_rx) = ws_stream.split();
    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx_send_channel_original_mut_speech_synth_text_convert_tossml_send_it_, mut rx_receive_channel_original_mut_speech_synth_listening_here_will_take_text_convert_tossml_send_it_) = mpsc::unbounded_channel::<Message>();
    let (mut speech_recognizer,  mut audio_push_stream) = helpers_recog::speech_recognizer_from_push_stream();
    let (speech_synthesizer, _) = helpers_synth::speech_synthesizer();
    let stream_id_clone1 = Arc::clone(&stream_id);
    let stream_id_clone2 = Arc::clone(&stream_id);

    let shared_sink = Arc::new(Mutex::new(user_ws_tx));
    let call_id_clone = call_id.clone();
    let call_id_clone_for_redis = call_id.clone();
    let (tx_gpt, mut rx_gpt) = mpsc::unbounded_channel::<String>();
    let tx_gpt_clone = tx_gpt.clone();
    let tx_gpt_clone_2 = tx_gpt.clone();
    let tx_gpt_clone_for_unprocessed = tx_gpt.clone();
    let stream_id_clone2 = Arc::clone(&stream_id);
    let (tx_trigger, mut rx_trigger) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
    let call_id_clone_for_processing = call_id_clone_for_redis.clone();
    let mut recognizer = create_gcp_speech_recognizer().await.unwrap();
    let mut audio_sender = recognizer.take_audio_sink().unwrap();
    let mut audio_sender_clone = audio_sender.clone();

    let handle_gpc0 = tokio::spawn(async move {

        while let Some(bytes_streaming) = rx_gcp_bytes_stream.recv().await {

            let streaming_request = Recognizer::streaming_request_from_bytes(bytes_streaming);
            match audio_sender.try_send(streaming_request) {
                Ok(_) => {

                },
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        error!("Channel closed");
                    hangup_call(call_id_clone.clone()).await;
                        break;
                    }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        error!("Channel full");
                    hangup_call(call_id_clone.clone()).await;
                    break;
                }

            }

        }
    });

    let handle_gpc2=  tokio::spawn(async move {
        let mut stream = recognizer.streaming_recognize_async_stream().await;
        pin_mut!(stream); // needed for iteration

        while let Some(val) = stream.next().await {
            match val {

                Ok(stream) =>{
                    println!("create_gcp_speech_recognizer stream-errors {:?}", stream);
                    if stream.error.is_none() {
                        let results = stream.results;
                        for result in results.into_iter() {
                            if result.is_final {
                                let text = &result.alternatives[0].transcript;
                                let tx_result = if !text.is_empty() {
                                    info!("create_gcp_speech_recognizer tx_result IS OK TODO ");
                                    tx_gpt.send(text.to_string())
                                } else {
                                    info!("create_gcp_speech_recognizer tx_result tx_gpt WARNING empty transcript TODO  ");
                                    Ok(())
                                };
                                match tx_result {
                                    Err(_) => warn!("create_gcp_speech_recognizer Error sending {}", text),
                                    Ok(_) => info!("create_gcp_speech_recognizer OK sent {}", text),
                                }
                            } else {
                                info!("create_gcp_speech_recognizer OK but not transcript result is not final");

                            }
                        }
                    } else {
                        error!("create_gcp_speech_recognizer Error sending ");
                    }
                }
                Err(e) => {
                    error!("create_gcp_speech_recognizer: errors in GCP stream");
                }
            }

        }
    });

    let handle_websocket=tokio::spawn(async move {
        // let mut buffer = Vec::new();
       let batch_size = 1024;
       tokio::spawn(async move {
            loop {
                // Sleep for 10 seconds
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                // Your data to send, replace this with the actual data
                let data_to_send = vec![1];

                // Send data
                if tx_gcp_bytes_stream_clone.send(data_to_send).await.is_err() {
                    eprintln!("An error occurred while sending data.");
                }
            }
        });
        while let Some(result) = user_ws_rx.next().await {

            let msg_option = match result {
                Ok(msg) => Some(msg),
                Err(e) => {
                    eprintln!("user_ws_rx websocket {:?}",  e);
                    None
                }
            };
            if let Some(msg_raw) = msg_option {
                let msg = get_serde_message(&callid_clone, msg_raw.clone()).await;
                let msg2 = msg.clone();
                if let Some(inc_msg) = msg {
                    if let Some(event) = inc_msg.event {
                        let media_clone = inc_msg.media.clone();
                        if event == "start" {
                            if let Some(streamId) = inc_msg.stream_id {
                                *stream_id_clone2.lock().await = Some(streamId);
                                let hello_start = "trigger_chat_gpt";
                                if let Err(e) = tx_gpt_clone.send(hello_start.to_string()) {
                                    error!(" error  in user_ws_rx sending set_recognized_cb result to GPT channel  {} ", "hello_start error ")
                                }
                            }
                        } else if event == "stop" {
                            if let Some(streamId) = inc_msg.stream_id {
                                println!(" stop stream_idstream_idstream");
                                // speech_recognizer
                                //     .stop_continuous_recognition_async()
                                //     .await
                                //     .unwrap();
                                // audio_push_stream.close_stream().expect("TODO: panic message close_stream");
                            }
                        }  else if event == "media" {


                            if let Some(media_clone_1) = media_clone {
                                if media_clone_1.track.clone().unwrap() != "inbound" {
                                    // println!("media stream event: {:#?}", media_clone_1.clone());
                                }
                            }
                            if let Some(media) = inc_msg.media {

                                if let Some(audio_as_base64) = media.payload {
                                    // append_to_file("audio recorded in file", audio_as_base64.clone().to_string()).await;
                                    tx_gcp_bytes_stream.send(base64::decode(audio_as_base64).unwrap()).await.unwrap();

                                }
                            }
                        } else {
                            println!(" not stop + not start + not media ?   stream id {:?} ",msg2);

                        }
                    }
                }
            } else {
                error!(" None not expected please debug this bug ")
            }

        }


    });
    let handle_synth1_11_labs=  tokio::task::spawn(async move {


        let callid_clone_1= callid_clone_for_elvenLabs.clone();
            let callid_clone_1 = callid_clone_base1.clone();
            while let Some(message) = rx_receive_channel_original_mut_speech_synth_listening_here_will_take_text_convert_tossml_send_it_.recv().await {
                let message_cloned_for_hangup = message.clone();
                let message_cloned_for_ssm_check = message.clone();


                println!("handle_synth1_11_labshandle_synth1_11_labshandle_synth1_11_labshandle_synth1_11_labshandle_synth1_11_labs 11");

                 let audio_message = fetch_and_encode_audio_stream(message.to_str().unwrap()).await;
                // let audio_message = fetch_and_output_audio(&mut receiver_port).await;
                println!("handle_synth1_11_labshandle_synth1_11_labshandle_synth1_11_labshandle_synth1_11_labshandle_synth1_11_labs 555");

                match audio_message {
                    Some(audio) => {
                        // Do something with the audio message
                        println!("Received and processed audio message: ");

                            let stream_id_unlocked = stream_id_clone1.lock().await;
                            let stream_id_unlocked_ref = stream_id_unlocked.as_ref();
                            let call_id_clone = callid_clone_1.clone();
                            let call_id_clone2 = callid_clone_1.clone();
                            if let Some(stream_id_unlocked_ref) = stream_id_unlocked_ref {
                                // println!("got result! from speech_synthesizer  len of result.audio_data {:?}, stream_id ", result.audio_data);
                                let media_instance = Media {
                                    track: None,
                                    chunk: None,
                                    timestamp: None,
                                    payload: Some(audio),
                                    stream_id: Some(stream_id_unlocked_ref.to_string())
                                };
                                let media_instance_envoloped = IncomingMessage {
                                    event: Some(String::from("media")),
                                    sequence_number: None,
                                    stop: None,
                                    // sourceOfMessage: Some(String::from("synthesizer")),
                                    start: None,
                                    media: Some(media_instance),
                                    stream_id: Some(stream_id_unlocked_ref.to_string()),
                                    version: None,
                                };
                                let json_string = serde_json::to_string(&media_instance_envoloped).unwrap();
                                info!("speech_synthesizer:after serde_json the msg,   done converting text to audio, sending[started] to customer's call, it should say something now ");

                                // send_messagev2(shared_sink.clone(), json_string).await;
                                let mut locked_sink = shared_sink.lock().await;
                                let txt = json_string.clone();
                                match locked_sink.send(Message::text(txt)).await {
                                    Ok(_) => {
                                        info!("Message sent successfully");
                                        info!("speech_synthesizer11: done converting text to audio, sending [done] to customer's call");

                                    }
                                    Err(e) => {
                                        error!("speech_synthesizer33: websocket send error: {}", e);
                                    }
                                }

                            } else {
                                error!("speech_synthesizer: can't send to telnyx websocket due to empty stream_id");
                            }
                    }
                    None => {
                        // Handle the absence of an audio message
                        println!("No audio message received or an error occurred.");
                    }
                }
            }

            });

    // let handle_synth1=  tokio::task::spawn(async move {
    //     let callid_clone_1 = callid_clone_base1.clone();
    //     while let Some(message) = rx_receive_channel_original_mut_speech_synth_listening_here_will_take_text_convert_tossml_send_it_.recv().await {
    //         let message_cloned_for_hangup = message.clone();
    //         let message_cloned_for_ssm_check = message.clone();
    //
    //         if is_valid_ssml(message_cloned_for_ssm_check.to_str().unwrap()) {
    //             match speech_synthesizer.speak_ssml_async(message.to_str().unwrap()).await {
    //                 Err(err) => error!("speak_text_async error {:?}", err),
    //                 Ok(result) => {
    //                     let stream_id_unlocked = stream_id_clone1.lock().await;
    //                     let stream_id_unlocked_ref = stream_id_unlocked.as_ref();
    //                     let call_id_clone = callid_clone_1.clone();
    //                     let call_id_clone2 = callid_clone_1.clone();
    //                     if let Some(stream_id_unlocked_ref) = stream_id_unlocked_ref {
    //                         // println!("got result! from speech_synthesizer  len of result.audio_data {:?}, stream_id ", result.audio_data);
    //                         let media_instance = Media {
    //                             track: None,
    //                             chunk: None,
    //                             timestamp: None,
    //                             payload: Some(base64::encode(result.audio_data)),
    //                             stream_id: Some(stream_id_unlocked_ref.to_string())
    //                         };
    //                         let media_instance_envoloped = IncomingMessage {
    //                             event: Some(String::from("media")),
    //                             sequence_number: None,
    //                             stop: None,
    //                             // sourceOfMessage: Some(String::from("synthesizer")),
    //                             start: None,
    //                             media: Some(media_instance),
    //                             stream_id: Some(stream_id_unlocked_ref.to_string()),
    //                             version: None,
    //                         };
    //                         let json_string = serde_json::to_string(&media_instance_envoloped).unwrap();
    //                         info!("speech_synthesizer:after serde_json the msg,   done converting text to audio, sending[started] to customer's call, it should say something now");
    //
    //                         // send_messagev2(shared_sink.clone(), json_string).await;
    //                         let mut locked_sink = shared_sink.lock().await;
    //                         let txt = json_string.clone();
    //                         match locked_sink.send(Message::text(txt)).await {
    //                             Ok(_) => {
    //                                 info!("Message sent successfully");
    //                                 info!("speech_synthesizer11: done converting text to audio, sending [done] to customer's call");
    //                             }
    //                             Err(e) => {
    //                                 error!("speech_synthesizer33: websocket send error: {}", e);
    //                             }
    //                         }
    //                     } else {
    //                         error!("speech_synthesizer: can't send to telnyx websocket due to empty stream_id");
    //                     }
    //                     if message_cloned_for_hangup.is_text() {
    //                         let msg_for_hang = message_cloned_for_hangup.to_str().unwrap();
    //                         if msg_for_hang.to_lowercase().contains("goodbye") || msg_for_hang.to_lowercase().contains("Have a great day") {
    //                             let callid_clone_for_hangup_inside = callid_clone_for_hangup.clone();
    //                             tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    //                                 println!("recognized GPT say: goodbye will hang up ");
    //                                     hangup_call(callid_clone_for_hangup_inside).await;
    //                                     continue;
    //                         }
    //                     }
    //                 }
    //             }
    //         } else {
    //             match speech_synthesizer.speak_text_async(message.to_str().unwrap()).await {
    //                 Err(err) => error!("speak_text_async error {:?}", err),
    //                 Ok(result) => {
    //                     let stream_id_unlocked = stream_id_clone1.lock().await;
    //                     let stream_id_unlocked_ref = stream_id_unlocked.as_ref();
    //                     let call_id_clone = callid_clone_1.clone();
    //                     let call_id_clone2 = callid_clone_1.clone();
    //                     if let Some(stream_id_unlocked_ref) = stream_id_unlocked_ref {
    //                         // println!("got result! from speech_synthesizer  len of result.audio_data {:?}, stream_id ", result.audio_data);
    //                         let media_instance = Media {
    //                             track: None,
    //                             chunk: None,
    //                             timestamp: None,
    //                             payload: Some(base64::encode(result.audio_data)),
    //                             stream_id: Some(stream_id_unlocked_ref.to_string())
    //                         };
    //
    //                         let media_instance_envoloped = IncomingMessage {
    //                             event: Some(String::from("media")),
    //                             sequence_number: None,
    //                             stop: None,
    //                             // sourceOfMessage: Some(String::from("synthesizer")),
    //                             start: None,
    //                             media: Some(media_instance),
    //                             stream_id: Some(stream_id_unlocked_ref.to_string()),
    //                             version: None,
    //                         };
    //                         let json_string = serde_json::to_string(&media_instance_envoloped).unwrap();
    //                         info!(" speech_synthesizer  should say something now {:?}", &json_string.len());
    //                         // send_messagev2(shared_sink.clone(), json_string).await;
    //                         let mut locked_sink = shared_sink.lock().await;
    //                         let txt = json_string.clone();
    //                         match locked_sink.send(Message::text(txt)).await {
    //                             Ok(_) => {
    //                                 info!("Message sent successfully0");
    //                                 info!("speech_synthesizer01: done converting text to audio, sending [done] to customer's call");
    //                             }
    //                             Err(e) => {
    //                                 error!("speech_synthesizer01: websocket send error: {}", e);
    //                             }
    //                         }
    //                     } else {
    //                         error!("speech_synthesizer: can't send to telnyx websocket due to empty stream_id");
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // });


    let handle_gpt1=tokio::task::spawn(async move {
        let mut history: Vec<ChatMessage> = Vec::new();

        let system_msg= format!("{} \n\n\n {} ", get_system_message_shopping_assistant(),  FAQS.to_string());
        history.push(create_chat_message("system".to_string(), system_msg.to_string()));
        while let Some(user_talk_text) = rx_gpt.recv().await {
            println!("user_talk_text.len() ==0 continue {}", user_talk_text);
            if user_talk_text.len() == 0 {
                continue
            }
            if user_talk_text == "send_message_in_the_buffer_only" {
            } else if user_talk_text == "trigger_chat_gpt_call.playback.ended" {
                info!("###GPT_Queue finished trigger_chat_gpt_call.playback.ended   {}",history.clone().len());
            } else if user_talk_text == "trigger_chat_gpt_call.playback.started" {
                info!("###GPT_Queue finished trigger_chat_gpt_call.playback.started   {}",history.clone().len());
            } else if user_talk_text == "trigger_chat_gpt" {
                let  mut_create_client = create_client().await;

                if let Some(gpt_response) = call_gpt_with_function( user_talk_text.to_string().clone(), history.clone()).await {
                    if gpt_response.choices.len() > 0 {
                        let choices = gpt_response.choices;
                        if let Some(first_choice) = choices.first() {
                            let message = &first_choice.message;
                            if let Some(function_call) = &message.function_call{
                                let result = call_function_dynamically(function_call).await;
                                println!("################################|====|##############################################");
                                println!("##############################################################################");
                                println!("###GPT_Queue call_function_dynamically Function 000response: {:?}", result);
                                println!("##############################################################################");
                                println!("#################################|=====|#############################################");
                                // now we get what product user wants to buy get prices
                                if let Some(call_function_result) = result{
                                    let template = format!("functionResponse======{:?}",call_function_result);
                                    // TODO enable function call
                                    tx_gpt_clone_2.send(template);
                                    continue
                                } else {
                                    // TODO enable function call
                                    let template = format!("functionResponse======Error");
                                    tx_gpt_clone_2.send(template);
                                    continue
                                }
                            } else {
                                let gpt_response = &message.content;
                                if let Some(response) = gpt_response {
                                    let gpt_response =response.to_string().clone();
                                    let gpt_response_1 = response.to_string().clone();
                                    let gpt_response_2 = response.to_string().clone();
                                    let ch_msg = create_chat_message("assistant".to_string(), gpt_response.clone());
                                    history.push(ch_msg);
                                    info!("###GPT_Queue trigger_chat_gpt building history of prev conversations we have # {} conversation {:#?} ",history.clone().len(),history.clone() );
                                    if let Err(_disconnected) = tx_send_channel_original_mut_speech_synth_text_convert_tossml_send_it_.send(Message::text(gpt_response_2)) {
                                        println!("###GPT_Queue trigger_chat_gpt _disconnected from speech recognition do to error {:?}", _disconnected.0);
                                    }
                                    info!("###GPT_Queue finished trigger_chat_gpt from GPT resposen will send GPT response to speech_synthesizer to convert it to audio to send it  to phone call  {}",history.clone().len());
                                } else {
                                    error!("###GPT_Queue  didn't respond  {}",history.clone().len());
                                }
                            }
                        } else {
                            println!("###GPT_Queue choices.first()  is empty");
                        }
                    } else {
                        println!("###GPT_Queue gpt_response.choices.len()  is empty");
                    }
                } else {
                    println!("###GPT_Queue call_gpt_with_function is error ");
                }
            } else {
                println!("before   match get_call_statu {:?}",user_talk_text);

                match get_call_status(call_id_clone_for_redis.clone()) {
                    Ok(status) =>  {
                        if status == "call.playback.ended" {
                            let text_to_speak = user_talk_text.clone();
                            let text_to_speak2 = user_talk_text.clone();

                            history=  modify_chat_for_token_limit(history.clone(), get_curr_model());
                            info!("###GPT_Queue: bot is not talking and received recognized_text: {}",text_to_speak2);
                            let mut ch_msg1 = create_chat_message("user".to_string(), text_to_speak.clone());

                            if text_to_speak.contains("functionResponse======Error") {
                                ch_msg1 = create_chat_message("system".to_string(), "Sorry Something went wrong, please try again later".to_string().clone());
                            } else if text_to_speak.contains("functionResponse======"){
                                if let Some(start) = text_to_speak.find("functionResponse======") {
                                    let output_str = &text_to_speak[start + "functionResponse======".len()..];
                                    ch_msg1 = create_chat_message("system".to_string(), output_str.to_string().clone());
                                } else {
                                    ch_msg1 = create_chat_message("system".to_string(), "Please try again ".to_string().clone());
                                }
                            }

                            history.push(ch_msg1);
                            let  mut_create_client = create_client().await;
                            let cloned_history_for_gpt = history.clone();
                            if let Some(gpt_response) = call_gpt_with_function( text_to_speak.to_string().clone(), history.clone()).await {
                                if gpt_response.choices.len() > 0 {
                                    let choices = gpt_response.choices;
                                    if let Some(first_choice) = choices.first() {
                                        let message = &first_choice.message;
                                        if let Some(function_call) = &message.function_call{
                                            // now we get what product user wants to buy get prices
                                            let result = call_function_dynamically(function_call).await;
                                            println!("################################|====|##############################################");
                                            println!("##############################################################################");
                                            println!("###GPT_Queue call_function_dynamically Function 000response00: {:?}", result);
                                            println!("##############################################################################");
                                            println!("#################################|=====|#############################################");
                                            if let Some(call_function_result) = result{
                                                let template = format!("functionResponse======{:?}",call_function_result);
                                                tx_gpt_clone_2.send(template);
                                                continue
                                            } else {
                                                let template = format!("functionResponse======Error");
                                                tx_gpt_clone_2.send(template);
                                                continue
                                            }

                                        }
                                        if let Some( gpt_response) =  &message.content {
                                            let gpt_response = gpt_response.clone();
                                            let gpt_response_1 = gpt_response.clone();
                                            let gpt_response_2 = gpt_response.clone();
                                            let ch_msg = create_chat_message("assistant".to_string(), gpt_response.clone());
                                            history.push(ch_msg);
                                            info!("###GPT_Queue  building history of prev conversations we have # {} conversation {:#?} ",history.clone().len(),history.clone() );
                                            if let Err(_disconnected) = tx_send_channel_original_mut_speech_synth_text_convert_tossml_send_it_.send(Message::text(gpt_response_2)) {
                                                println!("###GPT_Queue_disconnected from speech recognition do to error {:?}", _disconnected.0);
                                            }
                                            info!("###GPT_Queue finished from GPT resposen will send GPT response to speech_synthesizer to convert it to audio to send it  to phone call  {}",history.clone().len());
                                        } else {
                                            println!("###GPT_Queue  &message.content   is empty");
                                        }
                                    } else {
                                        println!("###GPT_Queue gpt_response.choices.first  is empty");
                                    }
                                } else {
                                    println!("###GPT_Queue gpt_response.choices.first  is empty");
                                }
                            } else {
                                error!("###GPT_Queue call_gpt_with_function return null or empty or erro response ")
                            }
                        } else  if status == "call.playback.started" {
                            info!("###Sending_recognized_text_GPT built history of conversations done {}",history.clone().len());
                            history.push(create_chat_message("user".to_string(), user_talk_text.clone().to_string()));

                            error!("###Sending_recognized_text_GPT ignore customer recognized text because the bot is still talking please wait, do we need to send it to queue? {} ",user_talk_text.clone());
                            // if let Err(e) = tx_gpt_clone_for_unprocessed.send(user_talk_text) {
                            //     Handle error here
                            // warn!(" error sending set_recognized_cb result to GPT channel  ")
                            // }
                        } else {
                            error!("###Sending_recognized_text_GPT maybe call hangup/or just started, I will  remove event from queue ");
                        }
                    }
                    Err(e) => {
                        info!("###Sending_recognized_text_GPT  redis_result get_call_status error {:?}", e);
                    },
                }
            }
        }
    });

    // let send_every_10s = tokio::spawn(async move {
    //     loop {
    //         // Sleep for 10 seconds
    //         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //
    //         // Your data to send, replace this with the actual data
    //         let data_to_send = vec![1];
    //
    //         // Send data
    //         if tx_gcp_bytes_stream_clone.send(data_to_send).await.is_err() {
    //             eprintln!("An error occurred while sending data.");
    //         }
    //     }
    // });
    let handle_gpc0_result = handle_gpc2.await;

    let should_spawn_azure_task = std::env::var("should_spawn_azure_task")
        .map(|val| val.to_lowercase() == "true")
        .unwrap_or(true);  // Default to true if the env var is not set or if it couldn't be read


    if handle_gpc0_result.is_ok() {
        let _ = tokio::try_join!(handle_gpc0, handle_websocket, handle_gpt1, handle_synth1_11_labs);
    }



}


pub async fn get_serde_message(streams_id: &String, msg: Message) -> Option<IncomingMessage> {
    if msg.is_text() {
        if let Ok(incoming_message) = serde_json::from_str::<IncomingMessage>(msg.to_str().unwrap()) {
            return Some(incoming_message);
        } else {
            error!("get_serde_message is not text  {:?}", msg.clone());
        }
    }
    None
}