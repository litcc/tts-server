use bytes::{BufMut, Bytes, BytesMut};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::convert::TryFrom;
use std::net::Ipv4Addr;
use std::sync::Arc;

use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::time;

use tokio::net::TcpStream;
use tokio::time::sleep;
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
pub(crate) static TAG_BODY_SPLIT: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10];
// �X-R
pub(crate) static TAG_SOME_DATA_START: [u8; 2] = [0, 128];
// gX-R
pub(crate) static TAG_NONE_DATA_START: [u8; 2] = [0, 103];

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct MsTtsMsgRequest {
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

impl MsTtsMsgRequest {
    #[inline]
    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(bincode::serialize(self).unwrap())
    }

    #[inline]
    pub fn from_bytes(bytes: Bytes) -> MsTtsMsgRequest {
        let data: MsTtsMsgRequest = bincode::deserialize(&bytes[..]).unwrap();
        data
    }
}

type WebsocketRt = SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>;
type WebsocketRt2 = SplitStream<WebSocketStream<TlsStream<TcpStream>>>;

pub(crate) fn register_service() {
    debug!("register_service");
    crate::RUNTIME.get().unwrap().handle().spawn(async {
        let tx: Arc<Mutex<Option<WebsocketRt>>> = Arc::new(Mutex::new(None));
        let mut msg_receiver = crate::CHANNEL.get().unwrap().get("value").unwrap().receiver.clone();

        loop {
            let msg = msg_receiver.lock().await.recv().await;
            debug!("channel recv msg");
            match msg {
                Some(m) => {
                    debug!("start handle msg");

                    let is_some = tx.clone().lock().await.is_some();

                    if !is_some {
                        debug!("websocket is not connected");
                        let mut result = new_websocket().await;
                        'outer: loop {
                            // 'outer:
                            debug!("进入循环，防止websocket连接失败");
                            let result_bool = result.is_ok();

                            if result_bool {
                                debug!("websocket连接成功");
                                let (mut tx_tmp, mut rx_tmp) = result.unwrap().split();
                                *tx.clone().lock().await = Some(tx_tmp);
                                let tx_tmp1 = Arc::clone(&tx);
                                debug!("启动消息处理线程");
                                //crate::RUNTIME.get().unwrap().spawn
                                //tokio::spawn
                                crate::RUNTIME.get().unwrap().handle().spawn(async move {
                                    debug!("消息处理线程启动成功");
                                    let tx_r = tx_tmp1.clone();
                                    let mut rx_r = rx_tmp;
                                    let mut cache: HashMap<String, BytesMut> = HashMap::new();
                                    debug!("开始获取websocket返回值");
                                    loop {
                                        let msg = rx_r.next().await.unwrap();
                                        match msg {
                                            Ok(m) => {
                                                info!("收到消息");
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
                                                            File::create(format!("tmp/{}.mp3", id)).await
                                                                .unwrap().write_all(&cache.get(&id).unwrap().to_vec()).await.unwrap();
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
                                                // websocket 错误的话就会断开连接
                                                break;
                                            }
                                        }
                                    }
                                    *tx_r.lock().await = None;
                                });
                                sleep(Duration::from_secs(1));
                                debug!("准备跳出循环");
                                break 'outer; //
                            } else {
                                debug!("reconnection websocket");
                                sleep(Duration::from_secs(1));
                                result = new_websocket().await;
                            }
                        }
                        debug!("循环已跳出");
                    }


                    debug!("存在websocket连接，继续处理");
                    let request: MsTtsMsgRequest = MsTtsMsgRequest::from_bytes(m);
                    // crate::RUNTIME.get().unwrap().spawn(async move {
                    //
                    // });
                    debug!("发送请求: {:?}", request);
                    let msg1 = String::from("Path:speech.config\r\nContent-Type:application/json;charset=utf-8\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"},\"language\":{\"autoDetection\":false}}}}\r\n");

                    // let request_id = random_string(32);
                    let msg2 = format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n<speak xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"http://www.w3.org/2001/mstts\" xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" version=\"1.0\" xml:lang=\"zh-CN\"><voice name=\"{}\"><s /><mstts:express-as style=\"{}\"><prosody rate=\"{}%\" pitch=\"{}%\">{}</prosody></mstts:express-as><s /></voice></speak>", request.request_id, "zh-CN-XiaoxiaoNeural", "general", "0", "0", request.text);

                    // 向 websocket 发送消息
                    tx.clone().lock().await.as_mut().unwrap().send(Message::Text(msg1)).await.unwrap();
                    tx.clone().lock().await.as_mut().unwrap().send(Message::Text(msg2)).await.unwrap();

                    info!("发送成功");
                }
                None => {
                    error!("出错");
                }
            }
        }
    });
    sleep(Duration::from_secs(1));
}

static MS_TTS_TOKEN: Lazy<String> = Lazy::new(|| {
    String::from_utf8(
        [
            54, 65, 53, 65, 65, 49, 68, 52, 69, 65, 70, 70, 52, 69, 57, 70, 66, 51, 55, 69, 50, 51,
            68, 54, 56, 52, 57, 49, 68, 54, 70, 52,
        ]
            .to_vec(),
    )
        .unwrap()
});

///
/// 获取新的隧道连接
pub(crate) async fn new_websocket() -> Result<WebSocketStream<TlsStream<TcpStream>>, String> {
    let connect_id = random_string(32);
    let uri = Uri::builder()
        .scheme("wss")
        .authority("speech.platform.bing.com")
        .path_and_query(format!(
            "/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&ConnectionId={}",
            MS_TTS_TOKEN.as_str(),
            &connect_id
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
