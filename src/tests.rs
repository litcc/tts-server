use crate::ms_tts::{ms_tts_websocket, MsTtsRequest};
use crate::utils::{binary_search, random_string};
use bytes::{BufMut, Bytes};
use futures_util::{SinkExt, StreamExt};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::*;

// 测试日志
fn init_log() {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%f)} {t} {T} thread_{I} {h({l})} - {m}{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        // .logger(Logger::builder()
        //     .appender("file")
        //     .additive(true)
        //     .build("app", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();

    log4rs::init_config(config).unwrap();
}

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
#[tokio::test]
async fn test_ms_tts_websocket() {
    init_log();
    info!("test_ms_tts_websocket");

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

    let mut websocket: Option<WebSocketStream<TlsStream<TcpStream>>> = Option::None;

    let hh = ms_tts_websocket().await;

    match hh {
        Ok(socket) => {
            info!("连接成功");
            websocket = Some(socket);
            let (mut tx, mut rx) = websocket.unwrap().split();

            tx.send(Message::Ping(vec![])).await.unwrap();
            let msg1 = String::from("Path:speech.config\r\nContent-Type:application/json;charset=utf-8\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"},\"language\":{\"autoDetection\":false}}}}\r\n");

            let request_id = random_string(32);
            let msg2 = format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n<speak xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"http://www.w3.org/2001/mstts\" xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" version=\"1.0\" xml:lang=\"zh-CN\"><voice name=\"{}\"><s /><mstts:express-as style=\"{}\"><prosody rate=\"{}%\" pitch=\"{}%\">{}</prosody></mstts:express-as><s /></voice></speak>", request_id, "zh-CN-XiaoxiaoNeural", "general", "0", "0", "扣你及哇");

            info!("第一次发送请求");
            tx.send(Message::Text(msg1)).await.unwrap();
            tx.send(Message::Text(msg2)).await.unwrap();

            RUNTIME.get().unwrap().spawn_blocking(async move || {
                thread::sleep(Duration::from_secs(60));
                let msg3 = String::from("Path:speech.config\r\nContent-Type:application/json;charset=utf-8\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"},\"language\":{\"autoDetection\":false}}}}\r\n");

                let request_id2 = random_string(32);
                let msg4 = format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n<speak xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"http://www.w3.org/2001/mstts\" xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" version=\"1.0\" xml:lang=\"zh-CN\"><voice name=\"{}\"><s /><mstts:express-as style=\"{}\"><prosody rate=\"{}%\" pitch=\"{}%\">{}</prosody></mstts:express-as><s /></voice></speak>", request_id2, "zh-CN-XiaoxiaoNeural", "general", "0", "0", "13123131");

                info!("第二次发送请求");
                tx.send(Message::Text(msg3)).await.unwrap();
                tx.send(Message::Text(msg4)).await.unwrap();
            });

            let tag_body_split: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10]; // "Path:audio\r\n"
            let tag_some_data_start = [0, 128]; // �X-R
            let tag_none_data_start = [0, 103]; // gX-R
            loop {
                let msg = rx.next().await.unwrap();

                match msg {
                    Ok(m) => {
                        match m {
                            Message::Ping(s) => {
                                info!("收到ping消息: {:?}", s);
                                // tx.send(Message::Pong(s)).await.unwrap();
                            }
                            Message::Pong(s) => {
                                info!("收到pong消息: {:?}", s);
                            }
                            Message::Close(s) => {
                                info!("收到close消息: {:?}", s);
                                break;
                            }
                            Message::Text(s) => {
                                let id = s[12..44].to_string();
                                info!("收到消息: {}", id);
                            }
                            Message::Binary(s) => {
                                if s.starts_with(&tag_some_data_start) {
                                    let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                    let mut body = BytesMut::from(s.as_slice());
                                    let index = binary_search(&s, &tag_body_split).unwrap();
                                    let mut body_new = body.split_to(index + tag_body_split.len());

                                    info!("二进制响应体 ,{}", id);
                                } else if s.starts_with(&tag_none_data_start) {
                                    let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                    info!("二进制响应体结束 tag_none_data_start, {}", id);
                                } else {
                                    info!("其他二进制类型: {} ", unsafe {
                                        String::from_utf8_unchecked(s.to_vec())
                                    });
                                }

                                // let mut df = BytesMut::from(s.as_slice());
                                //
                                // let index = binary_search(&s,&tag1);
                                //
                                // if let Some(i) = index {
                                //     let mut b = df.split_to(i + tag1.len());
                                //
                                //     info!("头长度: {} ", b.len());
                                //     info!("消息长度: {} ", df.len());
                                //     let l5 = b.split_to(5);
                                //     info!("收到二进制消息前5位: {:?} ", l5.to_vec());
                                //     info!("收到二进制消息: {:} \n\n", unsafe { String::from_utf8_unchecked(l5.to_vec()) });
                                // }else {
                                //     let l5 = df.split_to(5);
                                //     info!("收到二进制消息前5位: {:?} ", l5.to_vec());
                                //     info!("收到二进制消息: {:} \n\n", unsafe { String::from_utf8_unchecked(df.to_vec()) });
                                //
                                // }
                            }
                        }
                    }
                    Err(e) => {
                        info!("收到error消息: {:?}", e);
                        break;
                    }
                }
            }
            //let (write, read) = websocket.split();
        }
        Err(e) => {
            panic!("连接错误: {}", e);
        }
    }
}

// impl Bytes {
//     fn find(&self) -> usize {
//         -1
//     }
// }

#[tokio::test]
async fn test_bytes() {
    init_log();
    info!("test_ms_tts_websocket");

    let tag_some_data_start = [0, 128];
    let tag_none_data_start = [0, 103];

    let tag1 = "Path:audio\r\n";
    let test1: [u8; 850] = [
        0, 128, 88, 45, 82, 101, 113, 117, 101, 115, 116, 73, 100, 58, 101, 57, 100, 56, 97, 99,
        98, 55, 100, 101, 57, 49, 57, 52, 102, 55, 57, 50, 102, 98, 49, 97, 50, 54, 56, 55, 98, 56,
        101, 52, 101, 97, 13, 10, 67, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101, 58, 97,
        117, 100, 105, 111, 47, 109, 112, 101, 103, 13, 10, 88, 45, 83, 116, 114, 101, 97, 109, 73,
        100, 58, 56, 48, 52, 68, 53, 54, 67, 57, 48, 67, 65, 68, 52, 50, 65, 49, 65, 67, 55, 70,
        50, 66, 68, 51, 69, 50, 70, 56, 56, 48, 65, 68, 13, 10, 80, 97, 116, 104, 58, 97, 117, 100,
        105, 111, 13, 10, 255, 243, 100, 196, 114, 28, 225, 150, 205, 246, 194, 70, 124, 161, 106,
        197, 38, 19, 27, 117, 38, 73, 19, 224, 130, 2, 121, 231, 205, 139, 224, 223, 50, 99, 81,
        173, 15, 46, 48, 173, 151, 50, 126, 143, 84, 197, 122, 79, 4, 208, 253, 108, 118, 198, 245,
        66, 149, 189, 242, 132, 33, 16, 213, 57, 129, 14, 114, 10, 208, 136, 236, 71, 35, 178, 109,
        53, 116, 151, 73, 142, 101, 118, 98, 32, 52, 20, 45, 109, 153, 179, 37, 182, 123, 255, 235,
        255, 255, 252, 247, 43, 172, 133, 69, 112, 224, 203, 182, 127, 255, 255, 177, 109, 65, 4,
        140, 66, 131, 161, 0, 128, 101, 223, 249, 26, 130, 40, 95, 104, 235, 120, 229, 143, 226,
        178, 130, 20, 227, 204, 103, 97, 73, 0, 228, 198, 255, 243, 100, 196, 129, 27, 34, 122,
        193, 238, 131, 196, 92, 105, 4, 19, 41, 150, 201, 101, 67, 239, 94, 172, 36, 51, 138, 201,
        204, 206, 15, 158, 113, 98, 195, 227, 56, 119, 16, 214, 35, 26, 196, 116, 168, 149, 50, 24,
        164, 99, 145, 232, 183, 206, 206, 67, 125, 82, 238, 169, 118, 99, 181, 74, 236, 179, 181,
        117, 237, 74, 255, 255, 255, 232, 81, 44, 204, 37, 3, 17, 69, 48, 141, 52, 155, 59, 197, 2,
        182, 5, 80, 44, 104, 78, 239, 246, 15, 120, 144, 120, 80, 58, 48, 88, 201, 219, 194, 96,
        42, 163, 36, 29, 169, 233, 102, 206, 43, 132, 64, 17, 67, 203, 147, 1, 190, 38, 78, 95, 37,
        220, 191, 57, 111, 7, 225, 163, 6, 102, 1, 128, 40, 1, 140, 161, 255, 243, 100, 196, 151,
        28, 226, 130, 213, 246, 195, 4, 118, 134, 175, 52, 218, 32, 214, 67, 253, 114, 161, 171,
        59, 229, 108, 90, 239, 86, 204, 235, 82, 211, 12, 209, 13, 46, 100, 12, 169, 200, 71, 198,
        74, 95, 155, 26, 154, 198, 168, 211, 7, 177, 8, 172, 166, 119, 255, 183, 255, 255, 255,
        240, 78, 7, 67, 167, 136, 163, 255, 250, 215, 250, 139, 173, 164, 129, 68, 61, 38, 144,
        179, 162, 42, 140, 90, 221, 181, 138, 197, 16, 41, 98, 154, 217, 123, 141, 106, 173, 57,
        189, 101, 210, 23, 68, 172, 209, 187, 187, 182, 203, 92, 151, 241, 219, 10, 31, 217, 174,
        15, 61, 69, 17, 146, 19, 85, 163, 73, 169, 47, 172, 225, 132, 8, 217, 220, 184, 19, 45, 75,
        211, 74, 236, 255, 243, 100, 196, 166, 27, 154, 94, 201, 254, 120, 197, 76, 45, 206, 95,
        87, 221, 168, 215, 171, 219, 187, 125, 180, 52, 172, 213, 40, 114, 148, 81, 195, 40, 70,
        86, 170, 190, 195, 18, 68, 90, 183, 251, 117, 251, 76, 100, 8, 240, 33, 99, 146, 40, 168,
        48, 75, 255, 211, 132, 76, 54, 165, 13, 123, 142, 147, 36, 30, 197, 215, 23, 0, 53, 100,
        84, 105, 145, 131, 14, 106, 70, 37, 240, 219, 116, 128, 77, 231, 81, 85, 43, 154, 132, 69,
        160, 168, 219, 58, 127, 0, 132, 165, 232, 132, 37, 152, 182, 85, 105, 84, 85, 50, 16, 28,
        124, 110, 14, 228, 159, 164, 12, 16, 68, 116, 38, 4, 80, 90, 50, 219, 131, 102, 48, 82, 48,
        137, 36, 5, 63, 65, 73, 47, 101, 255, 243, 100, 196, 186, 30, 194, 118, 226, 62, 194, 68,
        186, 170, 187, 87, 66, 12, 234, 50, 99, 72, 73, 101, 37, 99, 45, 148, 85, 165, 149, 141,
        74, 81, 141, 74, 242, 191, 190, 149, 79, 210, 213, 115, 90, 147, 218, 13, 22, 61, 48, 153,
        145, 34, 50, 237, 174, 230, 10, 144, 148, 106, 113, 140, 165, 127, 249, 102, 117, 213, 190,
        215, 171, 51, 247, 188, 20, 229, 33, 65, 202, 242, 255, 255, 255, 254, 94, 103, 221, 115,
        59, 178, 21, 102, 13, 91, 9, 57, 18, 67, 84, 96, 10, 253, 161, 196, 199, 50, 19, 235, 80,
        129, 140, 50, 186, 237, 88, 173, 192, 2, 34, 73, 166, 12, 97, 175, 60, 79, 155, 13, 134,
        24, 147, 69, 83, 85, 180, 95, 37, 86, 86, 243, 8, 180,
    ];
    let tag1_2: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10];
    let tag2: [u8; 5] = [0, 128, 88, 45, 82];

    let agz = unsafe { String::from_utf8_unchecked(test1.to_vec()) };
    // let agz = BytesMut::from(test1.as_slice());
    // agz.find();

    let kk = binary_search(&test1, &tag1_2);
    info!("binary_search index: {:?}", kk);
    //let x = test1.binary_search()

    info!("tag1: {:?}", tag1.as_bytes());
    info!("tag2: {}", unsafe {
        String::from_utf8_unchecked(tag2.to_vec())
    });

    let mut b = BytesMut::new();

    // b.put(&b"123"[..]);
    // b.reserve(2);
    // b.put_slice(b"xy");
    // info!("{:?}",b);
    // info!("{:?}",b.capacity());
}

#[tokio::test]
async fn test_serialize() {
    init_log();
    info!("test_serialize");
    let test = MsTtsRequest {
        text: "123".to_string(),
        request_id: "123".to_string(),
        informant: "123".to_string(),
        style: "123".to_string(),
        rate: "123".to_string(),
        pitch: "123".to_string(),
        quality: "123".to_string(),
    };
    let encoded: Vec<u8> = bincode::serialize(&test).unwrap();

    info!("test: {:?}", encoded);
    //let decoded: MsTtsRequest = bincode::deserialize(&encoded[..]).unwrap();
    //let adsf:Vec<u8> = postcard::to_allocvec(&test).unwrap();
}
