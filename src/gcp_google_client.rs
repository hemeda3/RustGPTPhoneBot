
use google_cognitive_apis::speechtotext::recognizer::Recognizer;
use log::*;
use std::fs::{self, File};
use std::io::Read;
use google_cognitive_apis::api::grpc::google::cloud::speechtotext::v1::{ StreamingRecognitionConfig,RecognitionConfig };
use google_cognitive_apis::api::rest::google::cloud::speechtotext::v1::AudioEncoding;
pub async fn create_gcp_speech_recognizer() -> Result<Recognizer, Box<dyn std::error::Error>> {
    let credentials = fs::read_to_string("src/firebase.json")?;
    info!("create_gcp_speech_recognizer started");
    let streaming_config = StreamingRecognitionConfig {
        config: Some(RecognitionConfig {
            encoding: AudioEncoding::MULAW as i32,
            sample_rate_hertz: 8000,
            audio_channel_count: 1,
            enable_separate_recognition_per_channel: false,
            language_code: "en-US".to_string(),
            max_alternatives: 1,
            profanity_filter: false,
            speech_contexts: vec![],
            enable_word_time_offsets: false,
            enable_automatic_punctuation: false,
            diarization_config: None,
            metadata: None,
            model: "".to_string(),
            use_enhanced: true,
        }),
        single_utterance: false,
        interim_results: false,
    };

    let recognizer = Recognizer::create_streaming_recognizer(credentials, streaming_config, None)
        .await.unwrap();
    info!("create_gcp_speech_recognizer done OK ");

    Ok(recognizer)
}
