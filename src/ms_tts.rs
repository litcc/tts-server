use crate::ServerArea;
use bytes::{BufMut, Bytes, BytesMut};
use clap::Parser;
use event_bus::message::IMessage;
use fancy_regex::Regex;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use itertools::Itertools;
use log::{debug, info, trace, warn};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use rand::Rng;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::{Method, Uri, Version};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{client_async_with_config, WebSocketStream};

use crate::utils::{binary_search, get_system_ca_config, random_string};

// "Path:audio\r\n"
pub(crate) static TAG_BODY_SPLIT: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10];
// �X-R
pub(crate) static TAG_SOME_DATA_START: [u8; 2] = [0, 128];
// gX-R
pub(crate) static TAG_NONE_DATA_START: [u8; 2] = [0, 103];

pub(crate) static MS_TTS_CONFIG: OnceCell<MsTtsConfig> = OnceCell::new();

pub(crate) static MS_TTS_SERVER_CHINA_LIST: [&str; 7] = [
    // 北京节点
    "202.89.233.100",
    "202.89.233.101",
    "202.89.233.102",
    "202.89.233.103",
    "202.89.233.104",
    // 国内其他地点
    "47.95.21.44",
    "182.61.148.24",

    //	国内无法访问
    // "171.117.98.148",
    // "103.36.193.41",
    // "159.75.112.15",
    // "149.129.90.244",
    // "111.229.238.112",
];

pub(crate) static MS_TTS_SERVER_CHINA_HK_LIST: [&str; 12] = [
    // 北京节点
    "149.129.121.248",
    "103.200.112.245",
    "47.90.51.125",
    "61.239.177.5",
    "149.129.88.238",
    "103.68.61.91",
    "47.75.141.93",
    "34.96.186.48",
    "47.240.87.168",
    "47.57.114.186",
    "150.109.51.247",
    "20.205.113.91",
];

pub(crate) static MS_TTS_SERVER_CHINA_TW_LIST: [&str; 12] = [
    "114.46.156.231",
    "34.81.240.201",
    "34.80.106.199",
    "35.234.55.34",
    "130.211.254.124",
    "35.221.158.89",
    "104.199.252.57",
    "114.46.224.80",
    "114.46.192.185",
    "35.194.227.105",
    "114.46.184.44",
    "114.46.186.185",
];


pub(crate) static MS_TTS_QUALITY_LIST: [&str; 32] = [
    "audio-16khz-128kbitrate-mono-mp3",
    "audio-16khz-16bit-32kbps-mono-opus",
    "audio-16khz-16kbps-mono-siren",
    "audio-16khz-32kbitrate-mono-mp3",
    "audio-16khz-64kbitrate-mono-mp3",
    "audio-24khz-160kbitrate-mono-mp3",
    "audio-24khz-16bit-24kbps-mono-opus",
    "audio-24khz-16bit-48kbps-mono-opus",
    "audio-24khz-48kbitrate-mono-mp3",
    "audio-24khz-96kbitrate-mono-mp3	",
    "audio-48khz-192kbitrate-mono-mp3",
    "audio-48khz-96kbitrate-mono-mp3",
    "ogg-16khz-16bit-mono-opus",
    "ogg-24khz-16bit-mono-opus",
    "ogg-48khz-16bit-mono-opus",
    "raw-16khz-16bit-mono-pcm",
    "raw-16khz-16bit-mono-truesilk",
    "raw-24khz-16bit-mono-pcm",
    "raw-24khz-16bit-mono-truesilk",
    "raw-48khz-16bit-mono-pcm",
    "raw-8khz-16bit-mono-pcm",
    "raw-8khz-8bit-mono-alaw",
    "raw-8khz-8bit-mono-mulaw",
    "riff-16khz-16bit-mono-pcm",
    // "riff-16khz-16kbps-mono-siren",/*弃用*/
    "riff-24khz-16bit-mono-pcm",
    "riff-48khz-16bit-mono-pcm",
    "riff-8khz-16bit-mono-pcm",
    "riff-8khz-8bit-mono-alaw",
    "riff-8khz-8bit-mono-mulaw",
    "webm-16khz-16bit-mono-opus",
    "webm-24khz-16bit-24kbps-mono-opus",
    "webm-24khz-16bit-mono-opus",
];


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

impl Into<Vec<u8>> for MsTtsMsgRequest {
    #[inline]
    fn into(self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Into<event_bus::message::Body> for MsTtsMsgRequest {
    #[inline]
    fn into(self) -> event_bus::message::Body {
        event_bus::message::Body::ByteArray(self.into())
    }
}

type WebsocketRt = SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>;

static SOCKET_TX: OnceCell<Arc<Mutex<Option<WebsocketRt>>>> = OnceCell::new();


pub(crate) struct MsTtsCache {
    pub(crate) data: BytesMut,
    pub(crate) reply: IMessage,
}

// &'static mut HashMap<String, Mutex<MsTtsCache>>
static MS_TTS_DATA_CACHE: Lazy<Arc<HashMap<String, Mutex<MsTtsCache>>>> = Lazy::new(|| {
    let mut kk = HashMap::new();
    Arc::new(kk)
});

static MS_TTS_GET_NEW: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));


pub(crate) async fn register_service() {
    debug!("register_service");

    crate::GLOBAL_EB.consumer("tts_ms", |fn_msg| async move {
        let id = random_string(5);
        let eb_msg = fn_msg.msg.clone();
        let eb = Arc::clone(&fn_msg.eb);
        let ll = Bytes::from(eb_msg.body().await.as_bytes().expect("event_bus[ms-tts]: body is not bytes").to_vec());
        let request = MsTtsMsgRequest::from_bytes(ll);
        let tx_socket = Arc::clone(SOCKET_TX.get_or_init(|| {
            Arc::new(Mutex::new(None))
        }));

        if !MS_TTS_GET_NEW.load(Ordering::Relaxed) && !tx_socket.clone().lock().await.is_some() {
            MS_TTS_GET_NEW.store(true, Ordering::Release);
            debug!("websocket is not connected");
            let mut result = new_websocket().await;
            'outer: loop {
                // 'outer:
                trace!("进入循环，防止websocket连接失败");
                let result_bool = result.is_ok();

                if result_bool {
                    trace!("websocket连接成功");
                    let (tx_tmp, rx_tmp) = result.unwrap().split();
                    *tx_socket.clone().lock().await = Some(tx_tmp);
                    let tx_tmp1 = Arc::clone(&tx_socket);
                    trace!("启动消息处理线程");
                    eb.runtime.spawn(async move {
                        let tx_r = tx_tmp1.clone();
                        let mut rx_r = rx_tmp;
                        loop {
                            let msg = rx_r.next().await.unwrap();
                            match msg {
                                Ok(m) => {
                                    trace!("收到消息");
                                    match m {
                                        Message::Ping(s) => {
                                            trace!("收到ping消息: {:?}", s);
                                        }
                                        Message::Pong(s) => {
                                            trace!("收到pong消息: {:?}", s);
                                        }
                                        Message::Close(s) => {
                                            debug!("被动断开连接: {:?}", s);
                                            break;
                                        }
                                        Message::Text(s) => {
                                            let id = s[12..44].to_string();
                                            // info!("到消息: {}", id);
                                            if let Some(_i) = s.find("Path:turn.start") {
                                                // MS_TTS_DATA_CACHE.lock().await.insert(id, BytesMut::new());
                                            } else if let Some(_i) = s.find("Path:turn.end") {
                                                trace!("响应 {}， 结束", id);
                                                let data = unsafe { Arc::get_mut_unchecked(&mut MS_TTS_DATA_CACHE.clone()).remove(&id) };
                                                if let Some(data) = data {
                                                    debug!("结束请求: {}",id);
                                                    let data = data.lock().await;
                                                    data.reply.reply(data.data.to_vec().into()).await;
                                                    // eb_msg.reply(data.to_vec().into()).await;
                                                } else {
                                                    trace!("响应 不存在回复");
                                                }
                                                // File::create(format!("tmp/{}.mp3", id)).await
                                                //     .unwrap().write_all(&cache.get(&id).unwrap().to_vec()).await.unwrap();
                                                // ;
                                            }
                                        }
                                        Message::Binary(s) => {
                                            if s.starts_with(&TAG_SOME_DATA_START) {
                                                let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                                let mut body = BytesMut::from(s.as_slice());
                                                let index = binary_search(&s, &TAG_BODY_SPLIT).unwrap();
                                                let mut _head = body.split_to(index + TAG_BODY_SPLIT.len());
                                                unsafe { Arc::get_mut_unchecked(&mut MS_TTS_DATA_CACHE.clone()).get_mut(&id).unwrap().lock().await.data.put(body) };
                                                trace!("二进制响应体 ,{}",id);
                                            } else if s.starts_with(&TAG_NONE_DATA_START) {
                                                let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                                                trace!("二进制响应体结束 TAG_NONE_DATA_START, {}",id);
                                            } else {
                                                trace!("其他二进制类型: {} ", unsafe { String::from_utf8_unchecked(s.to_vec()) });
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    // trace!("收到错误消息:{:?}", e);
                                    debug!("收到错误消息，被动断开连接: {:?}", e);
                                    // websocket 错误的话就会断开连接
                                    break;
                                }
                            }
                        }
                        *tx_r.lock().await = None;
                    });
                    trace!("准备跳出循环");
                    break 'outer;
                } else {
                    trace!("reconnection websocket");
                    sleep(Duration::from_secs(1)).await;
                    result = new_websocket().await;
                }
            }
            trace!("循环已跳出");
            MS_TTS_GET_NEW.store(false, Ordering::Release)
        } else {
            while MS_TTS_GET_NEW.load(Ordering::Relaxed) || !tx_socket.clone().lock().await.is_some() {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
        trace!("存在websocket连接，继续处理");

        debug!("发送请求: {} | {:?}",request.request_id, request);
        let msg1 = String::from("Path:speech.config\r\nContent-Type:application/json;charset=utf-8\r\n\r\n{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"},\"language\":{\"autoDetection\":false}}}}\r\n");

        let msg2 = format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n<speak xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"http://www.w3.org/2001/mstts\" xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" version=\"1.0\" xml:lang=\"zh-CN\"><voice name=\"{}\"><s /><mstts:express-as style=\"{}\"><prosody rate=\"{}%\" pitch=\"{}%\">{}</prosody></mstts:express-as><s /></voice></speak>", request.request_id, "zh-CN-XiaoxiaoNeural", "general", "0", "0", request.text);
        // 向 websocket 发送消息
        unsafe {
            Arc::get_mut_unchecked(&mut MS_TTS_DATA_CACHE.clone()).insert(request.request_id, Mutex::new(MsTtsCache {
                data: BytesMut::new(),
                reply: eb_msg.clone(),
            }))
        };
        // info!("consumer {} 7",id);
        {
            let jj = tx_socket.clone();
            let mut gg = jj.lock().await;
            let mut socket = gg.as_mut();
            // if socket.is_some() {
            //     let s = socket.unwrap();
            //     s.send(Message::Text(msg1)).await.unwrap();
            //     s.send(Message::Text(msg2)).await.unwrap();
            // }
            if let Some(s) = socket {
                s.send(Message::Text(msg1)).await.unwrap();
                s.send(Message::Text(msg2)).await.unwrap();
            }
        }
    }).await;

    let kk_s = get_ms_tts_config().await.unwrap();

    MS_TTS_CONFIG.get_or_init(move || kk_s);
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
    let args: crate::AppArgs = crate::AppArgs::parse();
    match args.server_area {
        ServerArea::Default => {
            info!("连接至官方服务器");
            new_websocket_by_select_server(None).await
            // new_websocket_by_select_server(Some("171.117.98.148")).await
        }
        ServerArea::China => {
            info!("连接至内陆服务器");
            let select = rand::thread_rng().gen_range(0..MS_TTS_SERVER_CHINA_LIST.len());
            new_websocket_by_select_server(Some(MS_TTS_SERVER_CHINA_LIST.get(select).unwrap())).await
        }
        ServerArea::ChinaHK => {
            info!("连接至香港服务器");
            let select = rand::thread_rng().gen_range(0..MS_TTS_SERVER_CHINA_HK_LIST.len());
            new_websocket_by_select_server(Some(MS_TTS_SERVER_CHINA_HK_LIST.get(select).unwrap())).await
        }
        ServerArea::ChinaTW => {
            info!("连接至台湾服务器");
            let select = rand::thread_rng().gen_range(0..MS_TTS_SERVER_CHINA_TW_LIST.len());
            new_websocket_by_select_server(Some(MS_TTS_SERVER_CHINA_TW_LIST.get(select).unwrap())).await
        }
    }
}

pub(crate) async fn new_websocket_by_select_server(
    server: Option<&str>,
) -> Result<WebSocketStream<TlsStream<TcpStream>>, String> {
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


    let sock = match server {
        Some(s) => {
            info!("连接至 {:?}", s);

            let kk: Vec<_> = s.split(".").map(|x| x.parse::<u8>().unwrap()).collect();
            TcpStream::connect((
                Ipv4Addr::new(
                    kk.get(0).unwrap().clone(),
                    kk.get(1).unwrap().clone(),
                    kk.get(2).unwrap().clone(),
                    kk.get(3).unwrap().clone(),
                ),
                443,
            ))
                .await
        }
        None => {
            info!("连接至官方服务器");
            TcpStream::connect("speech.platform.bing.com:443").await
        }
    };

    if let Err(e) = sock {
        return Err(format!(
            "连接到微软服务器发生异常，tcp握手失败! 请检查网络! {:?}",
            e
        ));
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
            trace!("websocket 握手成功");
            Ok(_websocket.0)
        }
        Err(e2) => Err(format!("websocket 握手失败! {:?}", e2)),
    };
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VoicesItem {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "LocalName")]
    pub local_name: String,
    #[serde(rename = "ShortName")]
    pub short_name: String,
    #[serde(rename = "Gender")]
    pub gender: String,
    #[serde(rename = "Locale")]
    pub locale: String,
    #[serde(rename = "LocaleName")]
    pub locale_name: String,
    #[serde(rename = "StyleList")]
    pub style_list: Option<Vec<String>>,
    #[serde(rename = "SampleRateHertz")]
    pub sample_rate_hertz: String,
    #[serde(rename = "VoiceType")]
    pub voice_type: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "RolePlayList")]
    pub role_play_list: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct VoicesList {
    pub voices_name_list: HashSet<String>,
    pub raw_data: Vec<Arc<VoicesItem>>,
    pub by_voices_name_map: HashMap<String, Arc<VoicesItem>>,
    pub by_locale_map: HashMap<String, Vec<Arc<VoicesItem>>>,
}

#[derive(Debug)]
pub struct MsTtsConfig {
    pub voices_list: VoicesList,
    pub quality_list: Vec<String>,
}

// 空白音频
const BLANK_MUSIC_FILE: &'static [u8] = include_bytes!("resource/blank.mp3");

// 发音人配置
const SPEAKERS_LIST_FILE: &'static [u8] = include_bytes!("resource/voices_list.json");

///
/// 获取微软文本转语音支持的发音人配置
///
///
pub(crate) async fn get_ms_tts_config() -> Option<MsTtsConfig> {
    let args: crate::AppArgs = crate::AppArgs::parse();

    let config_json_text = if args.do_not_update_speakers_list {
        String::from_utf8(SPEAKERS_LIST_FILE.to_vec()).unwrap()
    } else {
        let client = reqwest::Client::new();
        trace!("开始请求token");
        let resp = client
            .get("https://azure.microsoft.com/zh-cn/services/cognitive-services/text-to-speech/")
            .send()
            .await
            .expect("get token error");
        let html = resp.text().await.unwrap();
        //debug!("html内容：{}",html);
        let token = Regex::new(r#"token: "([a-zA-Z0-9\._-]+)""#)
            .unwrap()
            .captures(&html)
            .unwrap();
        let token_str = match token {
            Some(t) => {
                let df = t.get(1).unwrap().as_str();
                trace!("token获取成功：{}", df);
                Some(df.to_owned())
            }
            None => None,
        };
        if token_str.is_none() {
            return None;
        }
        let region = Regex::new(r#"region: "([a-z0-9]+)""#)
            .unwrap()
            .captures(&html)
            .unwrap();
        let region_str = match region {
            Some(r) => {
                let df = r.get(1).unwrap().as_str();
                trace!("region获取成功：{}", df);
                Some(df.to_owned())
            }
            None => None,
        };
        if region_str.is_none() {
            return None;
        }
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token_str.unwrap()).parse().unwrap(),
        );
        headers.insert("Accept", "application/json".parse().unwrap());

        let config_response = client
            .get(format!(
                "https://{}.tts.speech.microsoft.com/cognitiveservices/voices/list",
                region_str.unwrap()
            ))
            .headers(headers)
            .send()
            .await
            .unwrap();
        let config_respone_tmp = config_response.text().await;
        if let Ok(json_text) = config_respone_tmp {
            // tokio::fs::File::create("voices_list.json").await
            //     .unwrap().write_all(json_text.as_bytes()).await.unwrap();
            json_text
        } else {
            warn!("从微软服务器更新发音人列表失败！改为使用本地缓存");
            String::from_utf8(SPEAKERS_LIST_FILE.to_vec()).unwrap()
        }
    };


    let tmp_list_1: Vec<VoicesItem> = serde_json::from_str(&config_json_text).unwrap();

    trace!("长度:{}", tmp_list_1.len());

    let mut raw_data: Vec<Arc<VoicesItem>> = Vec::new();
    let mut voices_name_list: HashSet<String> = HashSet::new();
    let mut by_voices_name_map: HashMap<String, Arc<VoicesItem>> = HashMap::new();

    tmp_list_1.iter().for_each(|item| {
        let new = Arc::new(item.clone());
        raw_data.push(new.clone());
        voices_name_list.insert(item.short_name.to_string());
        by_voices_name_map.insert(item.short_name.to_string(), new);
    });

    let mut by_locale_map: HashMap<String, Vec<Arc<VoicesItem>>> = HashMap::new();

    let new_iter = raw_data.iter();
    for (key, group) in &new_iter.group_by(|i| i.locale.as_str()) {
        let mut locale_vec_list: Vec<Arc<VoicesItem>> = Vec::new();

        group.for_each(|j| {
            locale_vec_list.push(j.clone());
        });
        by_locale_map.insert(key.to_owned(), locale_vec_list);
    }

    let v_list = VoicesList {
        voices_name_list,
        raw_data,
        by_voices_name_map,
        by_locale_map,
    };


    let quality_list_tmp: Vec<String> = MS_TTS_QUALITY_LIST.iter().map(|i| i.to_string()).collect_vec();

    return Some(MsTtsConfig {
        voices_list: v_list,
        quality_list: quality_list_tmp,
    });

    None
}
