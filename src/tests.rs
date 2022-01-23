#![feature(async_closure)]

use crate::ms_tts::{new_websocket, MsTtsMsgRequest};
use crate::utils::{binary_search, random_string};
use bytes::{BufMut, Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use std::thread;
use std::time::Duration;
use tokio::fs::File;
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
fn test_get_ms_tts_token() {
    info!("ms tts websocket token: ");
    let token: [u8; 32] = [
        54, 65, 53, 65, 65, 49, 68, 52, 69, 65, 70, 70, 52, 69, 57, 70, 66, 51, 55, 69, 50, 51, 68,
        54, 56, 52, 57, 49, 68, 54, 70, 52,
    ];
    info!("{}", String::from_utf8(token.to_vec()).unwrap())
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

    let hh = new_websocket().await;

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

            let mut cache: HashMap<String, BytesMut> = HashMap::new();
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
                                // info!("到消息: {}", id);
                                if let Some(_i) = s.find("Path:turn.start") {
                                    cache.insert(id, BytesMut::new());
                                } else if let Some(_i) = s.find("Path:turn.end") {
                                    debug!("响应 {}， 结束", id);
                                    // File::create(format!("/tmp/{}.mp3", id)).await
                                    //     .write_all(&cache.get(&id).unwrap().to_vec())
                                    //     .unwrap();
                                    cache.remove(&id);
                                }
                            }
                            Message::Binary(s) => {
                                if s.starts_with(&crate::ms_tts::TAG_SOME_DATA_START) {
                                    let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                    let mut body = BytesMut::from(s.as_slice());
                                    let index =
                                        binary_search(&s, &crate::ms_tts::TAG_BODY_SPLIT).unwrap();
                                    let mut _head =
                                        body.split_to(index + crate::ms_tts::TAG_BODY_SPLIT.len());
                                    cache.get_mut(&id).unwrap().put(body);
                                    info!("二进制响应体 ,{}", id);
                                } else if s.starts_with(&crate::ms_tts::TAG_NONE_DATA_START) {
                                    let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                    info!("二进制响应体结束 TAG_NONE_DATA_START, {}", id);
                                } else {
                                    info!("其他二进制类型: {} ", unsafe {
                                        String::from_utf8_unchecked(s.to_vec())
                                    });
                                }
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

    let tag1 = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
    let tag1_2: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10];
    let tag2: [u8; 5] = [0, 128, 88, 45, 82];

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
    let test = MsTtsMsgRequest {
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

fn get_websocket() -> Result<(Sender<Bytes>, Receiver<Bytes>), Box<dyn std::error::Error>> {
    Ok(crossbeam_channel::bounded::<Bytes>(2000))
}

// #[tokio::test]
#[test]
fn test_error1() {
    init_log();
    info!("test_serialize");

    crate::RUNTIME.get_or_init(|| {
        let cpus = num_cpus::get() / 2;
        let cpus = if cpus < 1 { 1 } else { cpus };
        Builder::new_multi_thread()
            .thread_name("event-bus-thread")
            .worker_threads(cpus)
            .enable_all()
            .build()
            .unwrap()
    });

    info!("1");
    crate::RUNTIME.get().unwrap().spawn(async {
        info!("2");
        std::thread::sleep(Duration::from_secs(1));
        info!("3");
        crate::RUNTIME.get().unwrap().spawn(async move {
            info!("4");
            //std::thread::sleep(Duration::from_secs(1));
            info!("5");
        });
        info!("8");
    });
    info!("9");

    thread::sleep(Duration::from_secs(5));
}
