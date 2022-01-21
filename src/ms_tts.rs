use bytes::{BufMut, BytesMut};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::{Method, Uri, Version};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{client_async_with_config, WebSocketStream};

use crate::info;
use crate::utils::{binary_search, get_system_ca_config, random_string};

//log::set_max_level(LevelFilter::Trace);
//"202.89.233.100","202.89.233.101" speech.platform.bing.com

// "Path:audio\r\n"
static TAG_BODY_SPLIT: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10];
// �X-R
static TAG_SOME_DATA_START: [u8; 2] = [0, 128];
// gX-R
static TAG_NONE_DATA_START: [u8; 2] = [0, 103];

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct MsTtsRequest {
    // 待生成文本
    pub text: String,
    // 请求id
    pub request_id: String,
    // 发音人
    pub informant: String,
    // 音频风格
    pub style: String,
    // 语速
    pub rate: String,
    // 音调
    pub pitch: String,
    // 音频格式
    pub quality: String,
    // text_replace_list:Vec<String>,
    // phoneme_list:Vec<String>
}

pub(crate) fn register_service() {
    type WebsocketRt = SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>;
    crate::RUNTIME.get().unwrap().spawn_blocking(async || {
        let tx: Arc<Mutex<Option<WebsocketRt>>> = Arc::new(Mutex::new(None));
        let tts_receiver = crate::CHANNEL
            .get()
            .unwrap()
            .get("tts")
            .unwrap()
            .1
            .clone();

        loop {
            let msg = tts_receiver.recv();
            if let Ok(m) = msg {
                // info!("get message from event bus: {:?}", d);
                if tx.clone().lock().unwrap().is_none() {
                    debug!("websocket is not connected");
                    let mut result = ms_tts_websocket().await;
                    loop {
                        if let Ok(ref mut _socket2) = result {
                            let (tx_tmp, rx_tmp) = result.unwrap().split();
                            tx.clone().lock().unwrap().replace(tx_tmp);
                            let tmp1 = tx.clone();
                            crate::RUNTIME.get().unwrap().spawn_blocking(async move || {
                                let tx_r = tmp1.clone();
                                let mut rx_r = rx_tmp;
                                // let mut responeData:HashMap<String,B>
                                let mut cache: HashMap<String, BytesMut> = HashMap::new();
                                loop {
                                    let msg = rx_r.next().await.unwrap();
                                    match msg {
                                        Ok(m) => {
                                            // info!("收到消息:{:?}", m);
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
                                                        File::create(format!("/tmp/{}.mp3", id))
                                                            .unwrap()
                                                            .write_all(&cache.get(&id).unwrap().to_vec())
                                                            .unwrap();
                                                        cache.remove(&id);
                                                    }
                                                }
                                                Message::Binary(s) => {
                                                    if s.starts_with(&TAG_SOME_DATA_START) {
                                                        let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                                        let mut body = BytesMut::from(s.as_slice());
                                                        let index = binary_search(&s, &TAG_BODY_SPLIT).unwrap();
                                                        let mut _head = body.split_to(index + TAG_BODY_SPLIT.len());
                                                        cache.get_mut(&id).unwrap().put(body);
                                                        info!("二进制响应体 ,{}",id);
                                                    } else if s.starts_with(&TAG_NONE_DATA_START) {
                                                        let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                                        info!("二进制响应体结束 TAG_NONE_DATA_START, {}",id);
                                                    } else {
                                                        info!("其他二进制类型: {} ", unsafe { String::from_utf8_unchecked(s.to_vec()) });
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            info!("收到错误消息:{:?}", e);
                                            break;
                                        }
                                    }
                                }
                                *tx_r.lock().unwrap() = None;
                            });
                            break;
                        } else {
                            debug!("reconnection websocket");
                            result = ms_tts_websocket().await;
                        }
                    }
                }

                let request: MsTtsRequest = bincode::deserialize(&m[..]).unwrap();


                let tx_o = tx.clone();
                crate::RUNTIME.get().unwrap().spawn_blocking(async move || {
                    let mut tx_b = tx_o;
                    let msg1 = String::from("Path:speech.config\r\nContent-Type:application/json;charset=utf-8\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"},\"language\":{\"autoDetection\":false}}}}\r\n");

                    let request_id = random_string(32);
                    let msg2 = format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n<speak xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"http://www.w3.org/2001/mstts\" xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" version=\"1.0\" xml:lang=\"zh-CN\"><voice name=\"{}\"><s /><mstts:express-as style=\"{}\"><prosody rate=\"{}%\" pitch=\"{}%\">{}</prosody></mstts:express-as><s /></voice></speak>", request_id, "zh-CN-XiaoxiaoNeural", "general", "0", "0", "扣你及哇");
                    tx_b.lock().unwrap().unwrap().send(Message::Text(msg1)).await.unwrap();
                    tx_b.lock().unwrap().unwrap().send(Message::Text(msg2)).await.unwrap();
                });

                // if let Some(ref mut _socket) = websocket {
                //     debug!("send message to ms tts");
                // } else {
                //
                //     // (ref mut tx,ref mut rx)
                //     // let (ref mut tx,ref mut rx) = (&mut ().unwrap()).split();
                // }
                // if let Some(socket) = websocket {
                // } else {
                //     let result = ms_tts_websocket().await;
                // }
            }
        }
    });
}

///
/// 获取新的隧道连接
pub(crate) async fn ms_tts_websocket() -> Result<WebSocketStream<TlsStream<TcpStream>>, String> {
    let trusted_client_token = format!("6A5AA1D4EAFF4E9FB37E23D68491D6F4");
    let connect_id = random_string(32);
    let uri = Uri::builder()
        .scheme("wss")
        .authority("speech.platform.bing.com")
        .path_and_query(format!(
            "/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&ConnectionId={}",
            &trusted_client_token, &connect_id
        ))
        .build();
    if let Err(e) = uri {
        return Err(format!("uri 构建错误 {:?}", e));
    }
    let uri = uri.unwrap();

    let request_builder = Request::builder().uri(uri).method(Method::GET)
        .header("Sec-webSocket-Extension", "permessage-deflate")
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .header("Accept", "*/*")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.212 Safari/537.36 Edg/90.0.818.62")
        .header("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold")
        .version(Version::HTTP_11);
    let request = request_builder.body(());
    if let Err(e) = request {
        return Err(format!("request_builder 构建错误 {:?}", e));
    }
    let request = request.unwrap();

    let config = get_system_ca_config();
    let config = TlsConnector::from(Arc::new(config));
    let domain = tokio_rustls::rustls::ServerName::try_from("speech.platform.bing.com");

    if let Err(e) = domain {
        return Err(format!("dns解析错误 speech.platform.bing.com {:?}", e));
    }
    let domain = domain.unwrap();

    //let mut sock = TcpStream::connect("speech.platform.bing.com:443").await.unwrap();
    let sock = TcpStream::connect((Ipv4Addr::new(202, 89, 233, 100), 443)).await;

    if let Err(e) = sock {
        return Err(format!("tcp握手失败! 请检查网络! {:?}", e));
    }
    let sock = sock.unwrap();

    let tsl_stream = config.connect(domain, sock).await;

    if let Err(e) = tsl_stream {
        return Err(format!("tsl握手失败! {}", e));
    }
    let tsl_stream = tsl_stream.unwrap();

    let websocket = client_async_with_config(
        request,
        tsl_stream,
        Some(WebSocketConfig {
            max_send_queue: None,
            max_message_size: None,
            max_frame_size: None,
            accept_unmasked_frames: false,
        }),
    )
    .await;
    return match websocket {
        Ok(_websocket) => {
            info!("websocket 握手成功");
            Ok(_websocket.0)
        }
        Err(e2) => Err(format!("websocket 握手失败! {:?}", e2)),
    };
}
