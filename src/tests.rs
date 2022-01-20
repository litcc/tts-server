use crate::ms_tts::ms_tts_websocket;
use crate::utils::random_string;
use futures_util::{SinkExt, StreamExt};
use std::thread;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::*;

#[test]
fn test4() {
    thread::spawn(|| {
        for i in 1..10 {
            println!("hi number {} from the spawned thread!", i);
            thread::sleep(Duration::from_millis(1));
        }
    });

    for i in 1..5 {
        println!("hi number {} from the main thread!", i);
        thread::sleep(Duration::from_millis(1));
    }
}

// 测试是能模拟edge进行tts接口调用
#[tokio::main]
#[test]
async fn test_ms_tts_websocket() {
    let mut websocket: Option<WebSocketStream<TlsStream<TcpStream>>> = Option::None;

    let hh = ms_tts_websocket().await;

    match hh {
        Ok(mut socket) => {
            info!("连接成功");
            websocket = Some(socket);

            let (mut tx, mut rx) = websocket.unwrap().split();

            tx.send(Message::Ping(vec![])).await.unwrap();
            let msg1 = String::from("Path:speech.config\r\nContent-Type:application/json;charset=utf-8\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"},\"language\":{\"autoDetection\":false}}}}\r\n");

            let request_id = random_string(32);
            let msg2 = format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n<speak xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"http://www.w3.org/2001/mstts\" xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" version=\"1.0\" xml:lang=\"zh-CN\"><voice name=\"{}\"><s /><mstts:express-as style=\"{}\"><prosody rate=\"{}%\" pitch=\"{}%\">{}</prosody></mstts:express-as><s /></voice></speak>", request_id, "zh-CN-XiaoxiaoNeural", "general", "0", "0", "你好");

            tx.send(Message::Text(msg1)).await.unwrap();
            tx.send(Message::Text(msg2)).await.unwrap();

            loop {
                let msg = rx.next().await.unwrap();

                if let Ok(m) = msg {
                    info!("收到消息:{:?}", m);
                }
            }
            //let (write, read) = websocket.split();
        }
        Err(e) => {
            panic!("连接错误: {}", e);
        }
    }
}
