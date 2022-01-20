//#![feature(get_mut_unchecked)]
// #![windows_subsystem = "windows"]

use crate::ms_tts::ms_tts_websocket;
use bytes::{Buf, BytesMut};
use crossbeam_channel::{Receiver, Sender};
pub use log::*;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio::runtime::{Builder, Runtime};
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::WebSocketStream;

mod controller;
mod log_utils;
pub mod ms_tts;
#[cfg(test)]
mod tests;
pub mod utils;
//mod event_bus;

static TTS_QUEUE: OnceCell<HashMap<String, (Sender<BytesMut>, Receiver<BytesMut>)>> =
    OnceCell::new();

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

#[tokio::main]
async fn main() {
    log_utils::init_log();
    // env_logger::init();
    info!("Hello, world!");

    TTS_QUEUE.get_or_init(|| {
        let mut tts_map = HashMap::new();
        tts_map.insert("tts".to_string(), crossbeam_channel::bounded(2000));
        tts_map.insert("control".to_string(), crossbeam_channel::bounded(2000));
        tts_map
    });

    RUNTIME.get_or_init(|| {
        let cpus = num_cpus::get() / 2;
        let cpus = if cpus < 1 { 1 } else { cpus };
        Builder::new_multi_thread()
            .thread_name("event-bus-thread")
            .worker_threads(cpus)
            .enable_all()
            .build()
            .unwrap()
    });


    RUNTIME.get().unwrap().spawn_blocking(move || {
        let mut websocket: Option<WebSocketStream<TlsStream<TcpStream>>> = Option::None;
        let tts_receiver = TTS_QUEUE.get().unwrap().get("tts").unwrap().1.clone();
        loop {
            let msg = tts_receiver.recv();
            if let Ok(m) = msg {
                // info!("get message from event bus: {:?}", d);
                if let Some(ref mut socket) = websocket {


                }
                // if let Some(socket) = websocket {
                // } else {
                //     let result = ms_tts_websocket().await;
                // }
            }
        }
    });

    //crossbeam_channel::unbounded() // 无限制队列大小
    //crossbeam_channel::bounded(2000) // 指定队列大小

    //
    // let hh = ms_tts_websocket().await;
    //
    // match hh {
    //     Ok(mut socket) => {
    //         info!("连接成功");
    //         websocket = Some(socket);
    //
    //         let (mut tx, mut rx) = websocket.unwrap().split();
    //
    //         tx.send(Message::Ping(vec![])).await.unwrap();
    //         let msg1 = String::from("Path:speech.config\r\nContent-Type:application/json;charset=utf-8\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"},\"language\":{\"autoDetection\":false}}}}\r\n");
    //
    //         let request_id = random_string(32);
    //         let msg2 = format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n<speak xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"http://www.w3.org/2001/mstts\" xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" version=\"1.0\" xml:lang=\"zh-CN\"><voice name=\"{}\"><s /><mstts:express-as style=\"{}\"><prosody rate=\"{}%\" pitch=\"{}%\">{}</prosody></mstts:express-as><s /></voice></speak>", request_id, "zh-CN-XiaoxiaoNeural", "general", "0", "0", "你好");
    //
    //         tx.send(Message::Text(msg1)).await.unwrap();
    //         tx.send(Message::Text(msg2)).await.unwrap();
    //
    //         loop {
    //             let msg = rx.next().await.unwrap();
    //
    //             if let Ok(m) = msg {
    //                 info!("收到消息:{:?}", m);
    //             }
    //         }
    //         //let (write, read) = websocket.split();
    //     }
    //     Err(e) => {
    //         println!("连接错误: {}", e)
    //     }
    // }
}
