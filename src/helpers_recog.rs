use cognitive_services_speech_sdk_rs::audio::{
    AudioConfig, AudioStreamFormat, PullAudioInputStream, PushAudioInputStream,
};
use cognitive_services_speech_sdk_rs::speech::{SpeechConfig, SpeechRecognizer, SpeechSynthesizer};
use log::*;
use std::{env, io};
use std::io::{Cursor, Read};
use base64;

use std::io::{Result};
use base64::{DecodeError, encode};
use futures_util::SinkExt;

/// convenience function to setup environment variables
/// subscription key is taken from external file
pub fn set_env_vars(_ms_key_file_path: &str) {
    let ms_key = env::var("MSSubscriptionKey").expect("MSSubscriptionKey not found in environment");
    // export  MSSubscriptionKey=xxxxxxxxxxx
    env::set_var("MSSubscriptionKey", ms_key);
    env::set_var("MSServiceRegion", "eastus");
    // env::set_var("RUST_LOG", "debug");
    // env::set_var("RUST_BACKTRACE", "full");
}
 pub fn set_callbacks_mux_lock(speech_recognizer: &mut SpeechRecognizer,speech_synthesizer: &mut SpeechSynthesizer) {
    info!("set_callbacks_mux_lock started");


}

pub fn set_callbacks(speech_recognizer: &mut SpeechRecognizer) {
    speech_recognizer
        .set_session_started_cb(|event| info!(">set_***************_session_started_cb {:?}", event))
        .unwrap_or_else(|_| panic!("Failed to set session_started_cb"));

    speech_recognizer
        .set_session_stopped_cb(|event| info!(">set_***************_session_stopped_cb {:?}", event))
        .unwrap_or_else(|_| panic!("Failed to set session_stopped_cb"));

    speech_recognizer
        .set_speech_start_detected_cb(|event| info!(">set_***************_speech_start_detected_cb {:?}", event))
        .unwrap_or_else(|_| panic!("Failed to set speech_start_detected_cb"));

    speech_recognizer
        .set_speech_end_detected_cb(|event| info!(">set_***************_speech_end_detected_cb {:?}", event))
        .unwrap_or_else(|_| panic!("Failed to set speech_end_detected_cb"));

    speech_recognizer
        .set_recognizing_cb(|event| info!(">set_***************_recognizing_cb {:?}", event.result.text))
        .unwrap_or_else(|_| panic!("Failed to set recognizing_cb"));

    speech_recognizer
        .set_recognized_cb(|event| info!(">set_***************_recognized_cb {:?}", event))
        .unwrap_or_else(|_| panic!("Failed to set recognized_cb"));

    speech_recognizer
        .set_canceled_cb(|event| info!(">set_***************_canceled_cb {:?}", event))
        .unwrap_or_else(|_| panic!("Failed to set canceled_cb"));
}

///creates speech recognizer from provided audio config and implicit speech config
/// created from MS subscription key hardcoded in sample file
pub fn speech_recognizer_from_audio_cfg(audio_config: AudioConfig) -> SpeechRecognizer {
    let subscription_key = env::var("MSSubscriptionKey");
    let service_region = env::var("MSServiceRegion");

    if subscription_key.is_err() || service_region.is_err() {
        error!("MSSubscriptionKey or MSServiceRegion not found in environment");
        // You can return a default value or handle the error in some other way.
        // For this example, I'll panic with a custom message.
        panic!("MSSubscriptionKey or MSServiceRegion not found in environment");
    }

    let mut speech_config = SpeechConfig::from_subscription(subscription_key.unwrap(), service_region.unwrap())
        .expect("Failed to create speech_config");
    // speech_config.set_speech_recognition_language("ar-SA".to_string()).expect("TODO: panic message");
    let speech_recognizer = SpeechRecognizer::from_config(speech_config, audio_config)
        .expect("Failed to create speech_recognizer");

    speech_recognizer
}
pub fn ulaw2linear2(u_val: u8) -> u8 {
    let u_val = !(u_val as u32);
    let mut t = ((u_val & 0x0F) << 3) + 0x84;
    t <<= (u_val & 0x70) >> 4;
    let result = if (u_val & 0x80) != 0 {
        (t - 0x84) as u8
    } else {
        (0x84u32.checked_sub(t).unwrap_or(0) as u8)
    };
    result
}
pub fn ulaw2linear(u_val: u8) -> i16 {
    let u_val = !(u_val as u32);
    let mut t = ((u_val & 0x0F) << 3) + 0x84;
    t <<= (u_val & 0x70) >> 4;
    let result = if (u_val & 0x80) != 0 {
        (t - 0x84) as i16
    } else {
        (0x84u32.checked_sub(t).unwrap_or(0) as i16)
    };
    result
}

pub  fn convert_base64_pcmu_to_pcm(base64_pcmu: &str) -> Vec<i16> {
    let pcmu_data = base64::decode(base64_pcmu).unwrap();
    pcmu_data.into_iter().map(ulaw2linear).collect()
}
pub  fn convert_base64_pcmu_to_pcm2(base64_pcmu: &str) -> Vec<u8> {
    let pcmu_data = base64::decode(base64_pcmu).unwrap();
    pcmu_data.into_iter().map(ulaw2linear2).collect()
}


/// creates speech recognizer from push input stream and MS speech subscription key
/// returns recognizer and also push stream so that data push can be initiated
pub fn speech_recognizer_from_push_stream() -> (SpeechRecognizer, PushAudioInputStream) {
    info!("creating speech recognizer");
    let wave_format = AudioStreamFormat::get_wave_format_pcm(8000, None, None).unwrap();
    let push_stream = PushAudioInputStream::create_push_stream_from_format(wave_format).unwrap();
    let audio_config = AudioConfig::from_stream_input(&push_stream).unwrap();
    info!("done creating speech recognizer");

    (speech_recognizer_from_audio_cfg(audio_config), push_stream)
}

/// creates speech recognizer from pull input stream and MS speech subscription key
/// returns recognizer and also pull stream so that data push can be initiated
// pub fn speech_recognizer_from_pull_stream() -> (SpeechRecognizer, PullAudioInputStream) {
//     let wave_format = AudioStreamFormat::get_wave_format_pcm(16000, None, None).unwrap();
//     let pull_stream = PullAudioInputStream::from_format(&wave_format).unwrap();
//     let audio_config = AudioConfig::from_stream_input(&pull_stream).unwrap();
//     (speech_recognizer_from_audio_cfg(audio_config), pull_stream)
// }

/// creates speech recognizer from wav input file and MS speech subscription key
pub fn speech_recognizer_from_wav_file(wav_file: &str) -> SpeechRecognizer {
    let audio_config = AudioConfig::from_wav_file_input(wav_file).unwrap();
    speech_recognizer_from_audio_cfg(audio_config)
}

/// creates speech recognizer from default mic settings and MS speech subscription key
pub fn speech_recognizer_default_mic() -> SpeechRecognizer {
    let audio_config = AudioConfig::from_default_microphone_input().unwrap();
    speech_recognizer_from_audio_cfg(audio_config)
}

pub enum PushStreamError {
    IoError(io::Error),
    DecodeError(DecodeError),
}

impl From<io::Error> for PushStreamError {
    fn from(err: io::Error) -> PushStreamError {
        PushStreamError::IoError(err)
    }
}

impl From<DecodeError> for PushStreamError {
    fn from(err: DecodeError) -> PushStreamError {
        PushStreamError::DecodeError(err)
    }
}
// pub fn push_base64_into_stream00(base64_audio_text: String, audio_push_stream: &mut PushAudioInputStream) -> Result<()> {
//     let mut original = Cursor::new(original);
//     let mut resampled = Cursor::new(Vec::new());
//
//     loop {
//         match original.read_i16::<LittleEndian>() {
//             Ok(x) => {
//                 // Write it twice
//                 resampled.write_i16::<LittleEndian>(x).unwrap();
//                 resampled.write_i16::<LittleEndian>(x).unwrap();
//             }
//             Err(_) => break,
//         }
//     }
//     Ok((resampled))
// }
pub fn push_base64_into_stream(base64_audio_text: &str, audio_push_stream: &mut PushAudioInputStream) {
    let cccc = convert_base64_pcmu_to_pcm(base64_audio_text);
    let bytes: Vec<u8> = unsafe {
        std::slice::from_raw_parts(cccc.as_ptr() as *const u8, cccc.len() * 2).to_vec()
    };
    audio_push_stream.write(&bytes).unwrap();
}
pub fn push_base64_to_bytes(base64_audio_text: &str) -> Vec<u8> {
    let cccc = convert_base64_pcmu_to_pcm(base64_audio_text);
    let bytes: Vec<u8> = unsafe {
        std::slice::from_raw_parts(cccc.as_ptr() as *const u8, cccc.len() * 2).to_vec()
    };
     return bytes;
}

pub fn push_bytes_vec_into_stream(bytes_vec: &[u8], audio_push_stream: &mut PushAudioInputStream) -> Result<()> {
    let chunk_size = 1000;
    audio_push_stream.write(bytes_vec).unwrap();

   Ok(())
}
pub fn push_file_into_stream(filename: &str,   audio_push_stream: &mut PushAudioInputStream) {
    let mut file = std::fs::File::open(filename).expect("TODO: error opening file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    audio_push_stream.write(buffer).unwrap();
    // audio_push_stream.deref()

}



pub fn load_and_encode_wav2(file_path: String) -> Result<String> {
    let mut file = std::fs::File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let encoded_data = encode(buffer);
    // println!("{:?}", encoded_data);
    Ok(encoded_data)
}
/// retrieves full path of file with filename
/// from examples/sample_files folder
pub fn get_sample_file(filename: &str) -> String {
    let mut dir = std::env::current_exe().unwrap();
    dir.pop();
    dir.pop();
    dir.pop();
    dir.pop();
    dir.push("examples");
    dir.push("sample_files");
    dir.push(filename);
    dir.into_os_string().into_string().unwrap()
}