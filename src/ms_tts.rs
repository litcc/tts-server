use bytes::{BufMut, Bytes, BytesMut};
use event_bus::message::IMessage;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::{debug, error, trace};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, OnceCell};
use tokio::time::sleep;
use tokio_native_tls::TlsStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::utils::azure_api::{
    AzureApiEdgeFree, AzureApiGenerateXMML, AzureApiNewWebsocket, AzureApiPreviewFreeToken,
    AzureApiRegionIdentifier, AzureApiSpeakerList, AzureApiSubscribeToken, AzureSubscribeKey,
    MsTtsMsgRequest, VoicesList, MS_TTS_QUALITY_LIST,
};
use crate::utils::binary_search;
use crate::AppArgs;

// "Path:audio\r\n"
pub(crate) static TAG_BODY_SPLIT: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10];
// gX-R
pub(crate) static TAG_NONE_DATA_START: [u8; 2] = [0, 103];

impl MsTtsMsgRequest {
    #[inline]
    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(bincode::serialize(self).unwrap())
    }

    #[inline]
    pub fn from_bytes(bytes: Bytes) -> Self {
        let data: Self = bincode::deserialize(&bytes[..]).unwrap();
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct MsTtsMsgResponse {
    pub request_id: String,
    pub data: Vec<u8>,
    pub file_type: String,
}

impl MsTtsMsgResponse {
    #[inline]
    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(bincode::serialize(self).unwrap())
    }

    #[inline]
    pub fn from_bytes(bytes: Bytes) -> Self {
        let data: Self = bincode::deserialize(&bytes[..]).unwrap();
        data
    }

    #[inline]
    pub fn to_vec(&self) -> Vec<u8> {
        Bytes::from(bincode::serialize(self).unwrap()).to_vec()
    }

    #[inline]
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let data: Self = bincode::deserialize(&bytes[..]).unwrap();
        data
    }
}

type WebsocketRt = SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>;

pub struct MsTtsCache {
    pub data: BytesMut,
    pub reply: IMessage,
    pub file_type: Option<String>,
}

#[derive(Debug)]
pub struct MsSocketInfo<T>
    where
        T: AzureApiSpeakerList + AzureApiNewWebsocket + AzureApiGenerateXMML,
{
    azure_api: Arc<T>,
    tx: Arc<Mutex<Option<WebsocketRt>>>,
    new: AtomicBool,
}

///
/// 微软 文本转语音接口注册服务
#[allow(dead_code)]
pub(crate) async fn register_service() {
    debug!("register_service");

    let args = AppArgs::parse_macro();
    if args.close_edge_free_api
        && args.close_official_preview_api
        && args.close_official_subscribe_api
    {
        error!("请最起码启用一个api");
        std::process::exit(1);
    }

    // 注册 edge 免费接口的服务
    if !args.close_edge_free_api {
        /// edge 免费接口 socket 连接
        static SOCKET_TX_EDGE_FREE: OnceCell<Arc<Mutex<Option<WebsocketRt>>>> =
            OnceCell::const_new();

        /// edge 免费接口 数据缓存
        static MS_TTS_DATA_CACHE_EDGE_FREE: Lazy<
            Arc<Mutex<HashMap<String, Arc<Mutex<MsTtsCache>>>>>,
        > = Lazy::new(|| {
            let kk = HashMap::new();
            Arc::new(Mutex::new(kk))
        });
        /// edge 免费接口 新请求限制措施
        static MS_TTS_GET_NEW_EDGE_FREE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

        AzureApiEdgeFree::new().get_vices_list().await.unwrap();

        crate::GLOBAL_EB
            .consumer("tts_ms_edge_free", |fn_msg| async move {
                let eb_msg = fn_msg.msg.clone();
                let eb = Arc::clone(&fn_msg.eb);
                let ll = Bytes::from(
                    eb_msg
                        .body()
                        .await
                        .as_bytes()
                        .expect("event_bus[ms-tts]: body is not bytes")
                        .to_vec(),
                );
                let request = MsTtsMsgRequest::from_bytes(ll);
                let tx_socket = Arc::clone(
                    SOCKET_TX_EDGE_FREE
                        .get_or_init(|| async { Arc::new(Mutex::new(None)) })
                        .await,
                );

                if !MS_TTS_GET_NEW_EDGE_FREE.load(Ordering::Relaxed)
                    && !tx_socket.clone().lock().await.is_some()
                {
                    MS_TTS_GET_NEW_EDGE_FREE.store(true, Ordering::Release);
                    debug!("websocket is not connected");
                    let mut result = AzureApiEdgeFree::new().get_connection().await;
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
                                process_response_body(
                                    rx_tmp,
                                    tx_tmp1,
                                    MS_TTS_DATA_CACHE_EDGE_FREE.clone(),
                                )
                                    .await;
                            });
                            trace!("准备跳出循环");
                            break 'outer;
                        } else {
                            trace!("reconnection websocket");
                            sleep(Duration::from_secs(1)).await;
                            result = AzureApiEdgeFree::new().get_connection().await;
                        }
                    }
                    trace!("循环已跳出");
                    MS_TTS_GET_NEW_EDGE_FREE.store(false, Ordering::Release)
                } else {
                    while MS_TTS_GET_NEW_EDGE_FREE.load(Ordering::Relaxed)
                        || !tx_socket.clone().lock().await.is_some()
                    {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
                trace!("存在websocket连接，继续处理");
                let request_id = request.request_id.clone();
                debug!("发送请求: {} | {:?}", request.request_id, request);

                let xmml = AzureApiEdgeFree::new()
                    .generate_xmml(request)
                    .await
                    .expect("generate_xmml 错误");

                // 向 websocket 发送消息
                MS_TTS_DATA_CACHE_EDGE_FREE.clone().lock().await.insert(
                    request_id,
                    Arc::new(Mutex::new(MsTtsCache {
                        data: BytesMut::new(),
                        reply: eb_msg.clone(),
                        file_type: None,
                    })),
                );

                let mut gg = tx_socket.lock().await;
                let socket = gg.as_mut();
                if let Some(s) = socket {
                    for i in xmml {
                        debug!("\n >>>>>>>>>>  xmml data\n{}\n <<<<<<<<<<\n", &i);
                        s.send(Message::Text(i)).await.unwrap();
                    }
                    drop(gg)
                }
            })
            .await;
    }

    // 注册 官网预览api 服务
    if !args.close_official_preview_api {
        /// 官网 免费预览接口 socket 连接
        static SOCKET_TX_OFFICIAL_PREVIEW: OnceCell<Arc<Mutex<Option<WebsocketRt>>>> =
            OnceCell::const_new();

        /// 官网 免费预览接口 数据缓存
        static MS_TTS_DATA_CACHE_OFFICIAL_PREVIEW: Lazy<
            Arc<Mutex<HashMap<String, Arc<Mutex<MsTtsCache>>>>>,
        > = Lazy::new(|| {
            let kk = HashMap::new();
            Arc::new(Mutex::new(kk))
        });

        /// 官网 免费预览接口 新请求限制措施
        static MS_TTS_GET_NEW_OFFICIAL_PREVIEW: Lazy<AtomicBool> =
            Lazy::new(|| AtomicBool::new(false));

        AzureApiPreviewFreeToken::new().get_vices_list().await.unwrap();

        crate::GLOBAL_EB
            .consumer("tts_ms_official_preview", |fn_msg| async move {
                let eb_msg = fn_msg.msg.clone();
                let eb = Arc::clone(&fn_msg.eb);
                let ll = Bytes::from(
                    eb_msg
                        .body()
                        .await
                        .as_bytes()
                        .expect("event_bus[tts_ms_official_preview]: body is not bytes")
                        .to_vec(),
                );
                let request = MsTtsMsgRequest::from_bytes(ll);
                let tx_socket = Arc::clone(
                    SOCKET_TX_OFFICIAL_PREVIEW
                        .get_or_init(|| async { Arc::new(Mutex::new(None)) })
                        .await,
                );

                if !MS_TTS_GET_NEW_OFFICIAL_PREVIEW.load(Ordering::Relaxed)
                    && !tx_socket.clone().lock().await.is_some()
                {
                    MS_TTS_GET_NEW_OFFICIAL_PREVIEW.store(true, Ordering::Release);
                    debug!("websocket is not connected");
                    // let mut info_mut = ;
                    let mut result = AzureApiPreviewFreeToken::new().get_connection().await;
                    // drop(info_mut);
                    // let mut result = new_websocket_edge_free().await;
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
                                process_response_body(
                                    rx_tmp,
                                    tx_tmp1,
                                    MS_TTS_DATA_CACHE_OFFICIAL_PREVIEW.clone(),
                                )
                                    .await;
                            });
                            trace!("准备跳出循环");
                            break 'outer;
                        } else {
                            trace!("reconnection websocket");
                            sleep(Duration::from_secs(1)).await;
                            result = AzureApiPreviewFreeToken::new().get_connection().await;
                        }
                    }
                    trace!("循环已跳出");
                    MS_TTS_GET_NEW_OFFICIAL_PREVIEW.store(false, Ordering::Release)
                } else {
                    while MS_TTS_GET_NEW_OFFICIAL_PREVIEW.load(Ordering::Relaxed)
                        || !tx_socket.clone().lock().await.is_some()
                    {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
                trace!("存在websocket连接，继续处理");

                let request_id = request.request_id.clone();
                debug!("发送请求: {} | {:?}", request.request_id, request);

                let xmml = AzureApiPreviewFreeToken::new()
                    .generate_xmml(request)
                    .await
                    .expect("generate_xmml 错误");

                // 向 websocket 发送消息
                MS_TTS_DATA_CACHE_OFFICIAL_PREVIEW
                    .clone()
                    .lock()
                    .await
                    .insert(
                        request_id,
                        Arc::new(Mutex::new(MsTtsCache {
                            data: BytesMut::new(),
                            reply: eb_msg.clone(),
                            file_type: None,
                        })),
                    );

                let mut gg = tx_socket.lock().await;
                let socket = gg.as_mut();
                if let Some(s) = socket {
                    for i in xmml {
                        debug!("\n >>>>>>>>>>  xmml data\n{}\n <<<<<<<<<<\n", &i);
                        s.send(Message::Text(i)).await.unwrap();
                    }
                    drop(gg)
                }
            })
            .await;
    }

    // 注册 官网ApiKey 调用服务
    if !args.close_official_subscribe_api {
        static OFFICIAL_SUBSCRIBE_API_LIST: OnceCell<Vec<AzureSubscribeKey>> =
            OnceCell::const_new();
        static OFFICIAL_SUBSCRIBE_API_USE_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

        let vec_list = &args.subscribe_key;

        OFFICIAL_SUBSCRIBE_API_LIST
            .get_or_init(|| async move {
                let azure_subscribe_keys = AzureSubscribeKey::from(vec_list);
                azure_subscribe_keys
            })
            .await;

        if OFFICIAL_SUBSCRIBE_API_LIST.get().unwrap().len() < 1 {
            error!("为了启用 subscribe api 最起码得添加一个有用的数据吧");
            std::process::exit(1);
        }

        /// 官网 免费预览接口 socket 连接
        static SOCKET_TX_MAP_OFFICIAL_SUBSCRIBE: OnceCell<
            Mutex<HashMap<String, Arc<MsSocketInfo<AzureApiSubscribeToken>>>>,
        > = OnceCell::const_new();

        // 设定程序配置的订阅key
        SOCKET_TX_MAP_OFFICIAL_SUBSCRIBE
            .get_or_init(|| async move {
                let mut h = HashMap::new();
                for subscribe_key in OFFICIAL_SUBSCRIBE_API_LIST.get().unwrap().iter() {
                    let info = MsSocketInfo {
                        azure_api: AzureApiSubscribeToken::new_from_subscribe_key(subscribe_key),
                        tx: Arc::new(Mutex::new(None)),
                        new: AtomicBool::new(false),
                    };
                    h.insert(subscribe_key.hash_str(), Arc::new(info));
                }
                Mutex::new(h)
            })
            .await;

        // 根据程序内订阅key获取发音人等数据
        if let Err(e) = AzureApiSubscribeToken::get_vices_mixed_list().await {
            error!("获取订阅key 的音频列表失败, {:?}", e);
            std::process::exit(1);
        }

        /// 官网 订阅API 响应数据缓存
        static MS_TTS_DATA_CACHE_OFFICIAL_SUBSCRIBE: Lazy<
            Arc<Mutex<HashMap<String, Arc<Mutex<MsTtsCache>>>>>,
        > = Lazy::new(|| {
            let kk = HashMap::new();
            Arc::new(Mutex::new(kk))
        });

        #[macro_export]
        macro_rules! get_subscribe_api_tx_for_map {
            () => {
                SOCKET_TX_MAP_OFFICIAL_SUBSCRIBE.get().unwrap().lock().await
            };
        }
        //

        crate::GLOBAL_EB
            .consumer("tts_ms_subscribe_api", |fn_msg| async move {
                let eb_msg = fn_msg.msg.clone();
                let eb = Arc::clone(&fn_msg.eb);
                let ll = Bytes::from(
                    eb_msg
                        .body()
                        .await
                        .as_bytes()
                        .expect("event_bus[tts_ms_subscribe_api]: body is not bytes")
                        .to_vec(),
                );
                let request = MsTtsMsgRequest::from_bytes(ll);

                let key_info = if (&request.region).is_some() && (&request.subscribe_key).is_some()
                {
                    let key_tmp = String::from(request.subscribe_key.as_ref().unwrap());
                    let region_tmp = String::from(request.region.as_ref().unwrap());

                    let api_key = AzureSubscribeKey(
                        key_tmp,
                        AzureApiRegionIdentifier::from(&region_tmp).unwrap(),
                    );
                    let hash = api_key.hash_str();
                    let if_contains = get_subscribe_api_tx_for_map!().contains_key(&hash);
                    let key_info = if if_contains {
                        get_subscribe_api_tx_for_map!().get(&hash).unwrap().clone()
                    } else {
                        let key_info = Arc::new(MsSocketInfo {
                            azure_api: AzureApiSubscribeToken::new_from_subscribe_key(&api_key),
                            tx: Arc::new(Mutex::new(None)),
                            new: AtomicBool::new(false),
                        });
                        get_subscribe_api_tx_for_map!().insert(hash, key_info.clone());
                        key_info
                    };
                    key_info
                } else {
                    let index = *OFFICIAL_SUBSCRIBE_API_USE_INDEX.lock().await;
                    let key_info = OFFICIAL_SUBSCRIBE_API_LIST
                        .get()
                        .unwrap()
                        .get(index)
                        .unwrap();
                    get_subscribe_api_tx_for_map!()
                        .get(&key_info.hash_str())
                        .unwrap()
                        .clone()
                };
                let azure_api = key_info.azure_api.clone();

                let tx_socket = key_info.tx.clone();

                if !key_info.new.load(Ordering::Relaxed)
                    && !tx_socket.clone().lock().await.is_some()
                {
                    key_info.new.store(true, Ordering::Release);

                    debug!("websocket is not connected");
                    // let mut info_mut = ;
                    // let token_info = key_info.lock().await.azure_api.clone();
                    let mut result = azure_api.get_connection().await;
                    // drop(info_mut);
                    // let mut result = new_websocket_edge_free().await;
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
                                process_response_body(
                                    rx_tmp,
                                    tx_tmp1,
                                    MS_TTS_DATA_CACHE_OFFICIAL_SUBSCRIBE.clone(),
                                )
                                    .await;
                                // 更新下一次进行连接的 Api 下标
                                let num = *OFFICIAL_SUBSCRIBE_API_USE_INDEX.lock().await + 1;
                                if num > OFFICIAL_SUBSCRIBE_API_LIST.get().unwrap().len() {
                                    *OFFICIAL_SUBSCRIBE_API_USE_INDEX.lock().await = 0;
                                } else {
                                    *OFFICIAL_SUBSCRIBE_API_USE_INDEX.lock().await = num;
                                }
                            });
                            trace!("准备跳出循环");
                            break 'outer;
                        } else {
                            trace!("reconnection websocket");
                            sleep(Duration::from_secs(1)).await;
                            result = AzureApiPreviewFreeToken::new().get_connection().await;
                        }
                    }
                    trace!("循环已跳出");
                    key_info.new.store(false, Ordering::Release)
                } else {
                    while key_info.new.load(Ordering::Relaxed)
                        || !tx_socket.clone().lock().await.is_some()
                    {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
                trace!("存在websocket连接，继续处理");

                let request_id = request.request_id.clone();
                debug!("发送请求: {} | {:?}", request_id, request);

                let xmml = azure_api
                    .generate_xmml(request)
                    .await
                    .expect("generate_xmml 错误");

                // 向 websocket 发送消息
                MS_TTS_DATA_CACHE_OFFICIAL_SUBSCRIBE
                    .clone()
                    .lock()
                    .await
                    .insert(
                        request_id,
                        Arc::new(Mutex::new(MsTtsCache {
                            data: BytesMut::new(),
                            reply: eb_msg.clone(),
                            file_type: None,
                        })),
                    );

                let mut gg = tx_socket.lock().await;
                let socket = gg.as_mut();
                if let Some(s) = socket {
                    for i in xmml {
                        debug!("\n >>>>>>>>>>  xmml data\n{}\n <<<<<<<<<<\n", &i);
                        s.send(Message::Text(i)).await.unwrap();
                    }
                    drop(gg)
                }
            })
            .await;
    }
}

/// 处理微软api 响应
#[allow(dead_code)]
async fn process_response_body(
    rx_r: SplitStream<WebSocketStream<TlsStream<TcpStream>>>,
    tx_r: Arc<Mutex<Option<WebsocketRt>>>,
    cache_db: Arc<Mutex<HashMap<String, Arc<Mutex<MsTtsCache>>>>>,
) {
    let mut rx_r = rx_r;
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
                        if let Some(_i) = s.find("Path:turn.start") {} else if let Some(_i) = s.find("Path:turn.end") {
                            trace!("响应 {}， 结束", id);
                            let data = { cache_db.lock().await.remove(&id) };
                            if let Some(data) = data {
                                debug!("结束请求: {}", id);
                                let data = data.lock().await;

                                let body = MsTtsMsgResponse {
                                    request_id: id,
                                    data: data.data.to_vec().clone(),
                                    file_type: data.file_type.as_ref().unwrap().to_string(),
                                };
                                data.reply.reply(body.to_vec().into()).await;
                                // eb_msg.reply(data.to_vec().into()).await;
                            } else {
                                trace!("响应 不存在回复");
                            }
                            // 测试代码
                            // File::create(format!("tmp/{}.mp3", id)).await
                            //     .unwrap().write_all(&cache.get(&id).unwrap().to_vec()).await.unwrap();
                            // ;
                        }
                    }
                    Message::Binary(s) => {
                        if s.starts_with(&TAG_NONE_DATA_START) {
                            let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                            trace!("二进制响应体结束 TAG_NONE_DATA_START, {}", id);
                        } else {
                            let id = String::from_utf8(s[14..46].to_vec()).unwrap();
                            let mut body = BytesMut::from(s.as_slice());
                            let index = binary_search(&s, &TAG_BODY_SPLIT).unwrap();
                            let head = body.split_to(index + TAG_BODY_SPLIT.len());
                            let cache = { cache_db.lock().await.get(&id).unwrap().clone() };
                            let mut cache_map = cache.lock().await;
                            cache_map.data.put(body);
                            if cache_map.file_type.is_none() {
                                let head = String::from_utf8(head.to_vec()[2..head.len()].to_vec())
                                    .unwrap();
                                let head_list = head.split("\r\n").collect::<Vec<&str>>();
                                let content_type =
                                    head_list[1].to_string().split(":").collect::<Vec<&str>>()[1]
                                        .to_string();
                                trace!("content_type: {}", content_type);
                                cache_map.file_type = Some(content_type);
                            }
                            drop(cache_map);
                            drop(cache);
                            // unsafe { Arc::get_mut_unchecked(&mut MS_TTS_DATA_CACHE.clone()).get_mut(&id).unwrap().lock().await.data.put(body) };
                            trace!("二进制响应体 ,{}", id);
                        } /* else {
                              trace!("其他二进制类型: {} ", unsafe { String::from_utf8_unchecked(s.to_vec()) });
                          }*/
                    }
                    _ => {}
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
}

#[derive(Debug)]
pub struct MsTtsConfig {
    pub voices_list: VoicesList,
    pub quality_list: Vec<String>,
}

// 空白音频
#[allow(dead_code)]
pub(crate) const BLANK_MUSIC_FILE: &'static [u8] = include_bytes!("resource/blank.mp3");
