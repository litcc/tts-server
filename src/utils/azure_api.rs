use crate::cmd::ServerArea;
use crate::error::TTSServerError;
use crate::{random_string, AppArgs};
use chrono::{Duration, TimeZone, Utc};
use event_bus::async_utils::BoxFutureSync;
use futures::future::join_all;
use futures::SinkExt;
use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use once_cell::sync::{Lazy, OnceCell};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::net::Ipv4Addr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio_native_tls::{native_tls, TlsStream};
use tokio_tungstenite::tungstenite::handshake::client::{generate_key, Request};
use tokio_tungstenite::tungstenite::http::{Method, Uri, Version};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{client_async_with_config, tungstenite, WebSocketStream};


// 发音人配置
#[allow(dead_code)]
pub(crate) const AZURE_SPEAKERS_LIST_FILE: &'static [u8] = include_bytes!("../resource/azure_voices_list.json");

// 发音人配置
#[allow(dead_code)]
pub(crate) const EDGE_SPEAKERS_LIST_FILE: &'static [u8] = include_bytes!("../resource/edge_voices_list.json");


/// 该程序实现的 Api 调用方式
#[derive(Debug)]
pub enum MsApiOrigin {
    /// 传统 edge 免费预览接口
    EdgeFree,
    /// 官方宣传页面 免费预览接口
    OfficialPreview,
    /// 官方 api-key 接口
    Subscription,
}

impl TryFrom<String> for MsApiOrigin {
    type Error = TTSServerError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "ms-tts-edge" => {
                Ok(MsApiOrigin::EdgeFree)
            }
            "ms-tts-preview" => {
                Ok(MsApiOrigin::OfficialPreview)
            }
            "ms-tts-subscribe" => {
                Ok(MsApiOrigin::Subscription)
            }
            _ => Err(TTSServerError::ProgramError("不存在的接口名称".to_owned())),
        }
    }
}


impl Into<String> for MsApiOrigin {
    fn into(self) -> String {
        match self {
            MsApiOrigin::EdgeFree => "ms-tts-edge".to_owned(),
            MsApiOrigin::OfficialPreview => "ms-tts-preview".to_owned(),
            MsApiOrigin::Subscription => "ms-tts-subscribe".to_owned(),
        }
    }
}


/// Azure Api 地域标识符
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum AzureApiRegionIdentifier {
    ///南非北部
    SouthAfricaNorth,
    ///东亚
    EastAsia,
    ///东南亚
    SoutheastAsia,
    ///澳大利亚东部
    AustraliaEast,
    ///印度中部
    CentralIndia,
    /// Jio 印度西部
    JioIndiaWest,
    ///日本东部
    JapanEast,
    ///日本西部
    JapanWest,
    ///韩国中部
    KoreaCentral,
    ///加拿大中部
    CanadaCentral,
    ///北欧
    NorthEurope,
    ///西欧
    WestEurope,
    ///法国中部
    FranceCentral,
    ///德国中西部
    GermanyWestCentral,
    ///挪威东部
    NorwayEast,
    ///瑞士北部
    SwitzerlandNorth,
    ///瑞士西部
    SwitzerlandWest,
    ///英国南部
    UkSouth,
    ///阿拉伯联合酋长国北部
    UaeNorth,
    ///巴西南部
    BrazilSouth,
    ///美国中部
    CentralUs,
    ///美国东部
    EastUs,
    ///美国东部2
    EastUs2,
    ///美国中北部
    NorthCentralUs,
    ///美国中南部
    SouthCentralUs,
    ///USGov亚利桑那州
    UsGovArizona,
    ///USGov弗吉尼亚州
    UsGovVirginia,
    ///美国中西部
    WestCentralUs,
    ///美国西部
    WestUs,
    ///美国西部2
    WestUs2,
    ///美国西部3
    WestUs3,
}

impl AzureApiRegionIdentifier {
    /// 获取地域标识
    #[allow(dead_code)]
    pub fn value(&self) -> String {
        match self {
            AzureApiRegionIdentifier::SouthAfricaNorth => "southafricanorth",
            AzureApiRegionIdentifier::EastAsia => "eastasia",
            AzureApiRegionIdentifier::SoutheastAsia => "southeastasia",
            AzureApiRegionIdentifier::AustraliaEast => "australiaeast",
            AzureApiRegionIdentifier::CentralIndia => "centralindia",
            AzureApiRegionIdentifier::JapanEast => "japaneast",
            AzureApiRegionIdentifier::JapanWest => "japanwest",
            AzureApiRegionIdentifier::KoreaCentral => "koreacentral",
            AzureApiRegionIdentifier::CanadaCentral => "canadacentral",
            AzureApiRegionIdentifier::NorthEurope => "northeurope",
            AzureApiRegionIdentifier::WestEurope => "westeurope",
            AzureApiRegionIdentifier::FranceCentral => "francecentral",
            AzureApiRegionIdentifier::GermanyWestCentral => "germanywestcentral",
            AzureApiRegionIdentifier::NorwayEast => "norwayeast",
            AzureApiRegionIdentifier::SwitzerlandNorth => "switzerlandnorth",
            AzureApiRegionIdentifier::SwitzerlandWest => "switzerlandwest",
            AzureApiRegionIdentifier::UkSouth => "uksouth",
            AzureApiRegionIdentifier::UaeNorth => "uaenorth",
            AzureApiRegionIdentifier::BrazilSouth => "brazilsouth",
            AzureApiRegionIdentifier::CentralUs => "centralus",
            AzureApiRegionIdentifier::EastUs => "eastus",
            AzureApiRegionIdentifier::EastUs2 => "eastus2",
            AzureApiRegionIdentifier::NorthCentralUs => "northcentralus",
            AzureApiRegionIdentifier::SouthCentralUs => "southcentralus",
            AzureApiRegionIdentifier::UsGovArizona => "usgovarizona",
            AzureApiRegionIdentifier::UsGovVirginia => "usgovvirginia",
            AzureApiRegionIdentifier::WestCentralUs => "westcentralus",
            AzureApiRegionIdentifier::WestUs => "westus",
            AzureApiRegionIdentifier::WestUs2 => "westus2",
            AzureApiRegionIdentifier::WestUs3 => "westus3",
            AzureApiRegionIdentifier::JioIndiaWest => "jioindiawest",
        }
            .to_owned()
    }

    #[allow(dead_code)]
    pub fn from(region_str: &str) -> Result<Self, TTSServerError> {
        let kk = match region_str {
            "southafricanorth" => AzureApiRegionIdentifier::SouthAfricaNorth,
            "eastasia" => AzureApiRegionIdentifier::EastAsia,
            "southeastasia" => AzureApiRegionIdentifier::SoutheastAsia,
            "australiaeast" => AzureApiRegionIdentifier::AustraliaEast,
            "centralindia" => AzureApiRegionIdentifier::CentralIndia,
            "japaneast" => AzureApiRegionIdentifier::JapanEast,
            "japanwest" => AzureApiRegionIdentifier::JapanWest,
            "koreacentral" => AzureApiRegionIdentifier::KoreaCentral,
            "canadacentral" => AzureApiRegionIdentifier::CanadaCentral,
            "northeurope" => AzureApiRegionIdentifier::NorthEurope,
            "westeurope" => AzureApiRegionIdentifier::WestEurope,
            "francecentral" => AzureApiRegionIdentifier::FranceCentral,
            "germanywestcentral" => AzureApiRegionIdentifier::GermanyWestCentral,
            "norwayeast" => AzureApiRegionIdentifier::NorwayEast,
            "switzerlandnorth" => AzureApiRegionIdentifier::SwitzerlandNorth,
            "switzerlandwest" => AzureApiRegionIdentifier::SwitzerlandWest,
            "uksouth" => AzureApiRegionIdentifier::UkSouth,
            "uaenorth" => AzureApiRegionIdentifier::UaeNorth,
            "brazilsouth" => AzureApiRegionIdentifier::BrazilSouth,
            "centralus" => AzureApiRegionIdentifier::CentralUs,
            "eastus" => AzureApiRegionIdentifier::EastUs,
            "eastus2" => AzureApiRegionIdentifier::EastUs2,
            "northcentralus" => AzureApiRegionIdentifier::NorthCentralUs,
            "southcentralus" => AzureApiRegionIdentifier::SouthCentralUs,
            "usgovarizona" => AzureApiRegionIdentifier::UsGovArizona,
            "usgovvirginia" => AzureApiRegionIdentifier::UsGovVirginia,
            "westcentralus" => AzureApiRegionIdentifier::WestCentralUs,
            "westus" => AzureApiRegionIdentifier::WestUs,
            "westus2" => AzureApiRegionIdentifier::WestUs2,
            "westus3" => AzureApiRegionIdentifier::WestUs3,
            "jioindiawest" => AzureApiRegionIdentifier::JioIndiaWest,
            _ => {
                return Err(TTSServerError::ProgramError("解析地域标签错误".to_owned()));
            }
        };

        Ok(kk)
    }
}

///
/// 可用音质参数
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
    "audio-24khz-96kbitrate-mono-mp3",
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

///
/// 微软认证
pub trait AzureAuthSubscription {
    /// 获取订阅key
    fn get_subscription_key(&self) -> BoxFutureSync<Result<String, TTSServerError>>;
}

///
/// 微软认证
pub trait AzureAuthKey {
    /// 获取地域标识
    fn get_region_identifier(
        &self,
    ) -> BoxFutureSync<Result<AzureApiRegionIdentifier, TTSServerError>>;
    /// 获取认证key
    fn get_oauth_token(&self) -> BoxFutureSync<Result<String, TTSServerError>>;
}

///
/// 微软获取 发音人列表
pub trait AzureApiSpeakerList {
    /// 获取发音人列表
    fn get_vices_list(&self) -> BoxFutureSync<Result<VoicesList, TTSServerError>>;
}

/// 创建新的请求
pub trait AzureApiNewWebsocket {
    fn get_connection(
        &self,
    ) -> BoxFutureSync<Result<WebSocketStream<TlsStream<TcpStream>>, TTSServerError>>;
}

/// 生成 xmml 请求
pub trait AzureApiGenerateXMML {
    fn generate_xmml(
        &self,
        data: MsTtsMsgRequest,
    ) -> BoxFutureSync<Result<Vec<String>, TTSServerError>>;
}

/// Azure 认证 Key
#[derive(PartialEq, Debug)]
pub struct AzureSubscribeKey(
    /// 订阅key
    pub String,
    /// 地域
    pub AzureApiRegionIdentifier,
);

impl AzureSubscribeKey {
    pub fn hash_str(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn from(list: &Vec<String>) -> Vec<Self> {
        let mut k_list = Vec::new();
        for i in list.iter() {
            let item = AzureSubscribeKey::try_from(i.as_str());
            if let Ok(d) = item {
                k_list.push(d)
            }
        }
        k_list
    }
}

impl TryFrom<&str> for AzureSubscribeKey {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let l: Vec<_> = value.split(',').collect();
        if l.len() != 2 {
            let err =
                anyhow::Error::msg("错误的订阅字符串, 请检查订阅key参数是否符合要求".to_owned());
            warn!("{:?}", err);
            return Err(err);
        }
        let key = l.get(0).unwrap().to_string();
        let region = l.get(1).unwrap().to_string();
        let region = AzureApiRegionIdentifier::from(&region)?;
        Ok(AzureSubscribeKey(key, region))
    }
}

impl Hash for AzureSubscribeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.value().hash(state);
    }
}

/// 生成 xmml 的数据
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

    // 使用订阅 API 时可用传递 自定义订阅 key，以及自定义订阅地区
    #[serde(default)]
    pub subscribe_key: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    // 以前java版本支持的功能，目前没时间支持
    // text_replace_list:Vec<String>,
    // phoneme_list:Vec<String>
}

///
/// 发音人列表
#[derive(Debug)]
pub struct VoicesList {
    pub voices_name_list: HashSet<String>,
    pub raw_data: Vec<Arc<VoicesItem>>,
    pub by_voices_name_map: HashMap<String, Arc<VoicesItem>>,
    pub by_locale_map: HashMap<String, Vec<Arc<VoicesItem>>>,
}

///
/// Azure 订阅版文本转语音官方相关接口实例
pub(crate) struct AzureApiSubscribeToken {
    region_identifier: AzureApiRegionIdentifier,
    subscription_key: String,
    oauth_token: Arc<Mutex<Option<String>>>,
    oauth_get_time: Arc<Mutex<i64>>,
    voices_list: RwLock<Option<Vec<Arc<VoicesItem>>>>,
}

impl Hash for AzureApiSubscribeToken {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.subscription_key.hash(state);
        self.region_identifier.value().hash(state);
    }
}

static MS_TTS_SUBSCRIBE_TOKEN_LIST: Lazy<Arc<Mutex<HashMap<String, Arc<AzureApiSubscribeToken>>>>> =
    Lazy::new(|| {
        let kk = HashMap::new();
        Arc::new(Mutex::new(kk))
    });

static MS_TTS_SUBSCRIBE_VICES_MIXED_LIST: tokio::sync::OnceCell<Vec<Arc<VoicesItem>>> =
    tokio::sync::OnceCell::const_new();

impl AzureApiSubscribeToken {
    /// 过期时间
    const EXPIRED_TIME: i64 = 8;
    /// 请求 user-agent
    const USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1379.1";

    #[inline]
    pub(crate) fn hash_str(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// 获取过期时间
    #[allow(dead_code)]
    #[inline]
    fn get_expired_time() -> Duration {
        Duration::minutes(AzureApiSubscribeToken::EXPIRED_TIME)
    }

    /// 实例化付费版 Api key
    #[allow(dead_code)]
    pub(crate) fn new(region: AzureApiRegionIdentifier, subscription_key: &str) -> Arc<Self> {
        let new_token = AzureApiSubscribeToken {
            region_identifier: region,
            subscription_key: subscription_key.to_owned(),
            oauth_token: Arc::new(Mutex::new(None)),
            oauth_get_time: Arc::new(Mutex::new(0)),
            voices_list: RwLock::new(None),
        };
        let hash = new_token.hash_str();
        let mut k_list = loop {
            let kk = MS_TTS_SUBSCRIBE_TOKEN_LIST.try_lock();
            if kk.is_ok() {
                break kk.unwrap();
            }
        };
        let arc_token = Arc::new(new_token);
        return if k_list.contains_key(&hash) {
            k_list.get(&hash).unwrap().clone()
        } else {
            k_list.insert(hash, arc_token.clone());
            arc_token
        };
    }

    #[allow(dead_code)]
    pub(crate) fn new_from_subscribe_key(data: &AzureSubscribeKey) -> Arc<Self> {
        Self::new(data.1.clone(), &data.0)
    }

    // 获取程序中配置的所有订阅key
    pub(crate) async fn get_subscribe_key_list() -> Vec<Arc<AzureApiSubscribeToken>> {
        let mut list = Vec::new();
        for x in MS_TTS_SUBSCRIBE_TOKEN_LIST.lock().await.values() {
            list.push(x.clone())
        }
        list
    }

    /// 获取程序中配置所有订阅key的发音人列表 注：列表只取交集，防止部分地区发音人在随机api中无法使用
    pub(crate) async fn get_vices_mixed_list() -> Result<VoicesList, TTSServerError> {
        let list = MS_TTS_SUBSCRIBE_VICES_MIXED_LIST
            .get_or_init(|| async move {
                let list = Self::get_subscribe_key_list().await;
                let mut resp = Vec::new();
                for x in list {
                    let kk = x.clone();
                    let call = || async move { kk.get_vices_list().await };
                    resp.push(call())
                }
                let resp: Vec<Result<VoicesList, TTSServerError>> = join_all(resp).await;

                let mut tmp: Option<Vec<Arc<VoicesItem>>> = None;

                for re in resp {
                    if re.is_ok() {
                        let d = re.unwrap().raw_data;
                        // resp_new.push(d.raw_data);

                        if let Some(t) = tmp {
                            let intersect = t
                                .iter()
                                .filter(|&u| d.contains(u))
                                .cloned()
                                .collect::<Vec<_>>();
                            tmp = Some(intersect);
                        } else {
                            tmp = Some(d.clone())
                        }
                    } else {
                        error!("{:?}", re.err().unwrap());
                        std::process::exit(1);
                    }
                }
                let intersect = tmp.unwrap();
                intersect
            })
            .await;
        let arc_list = collating_list_of_pronouncers_arc(list);
        Ok(arc_list)
    }

    /// 判断认证 Token 是否过期
    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn is_expired(&self) -> bool {
        let now = Utc::now();
        let p_time = self.oauth_get_time.lock().await;
        let old = Utc.timestamp(*p_time, 0);
        drop(p_time);
        return if now - old > Self::get_expired_time() {
            true
        } else {
            false
        };
    }

    /// 判断认证 oauth_token 是否为 None
    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn token_is_none(&self) -> bool {
        let token = self.oauth_token.lock().await;
        let is = token.is_none();
        drop(token);
        is
    }

    /// 判断认证 oauth_token 是否为 None
    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn set_new_token(&self, token: String) {
        self.oauth_token.lock().await.replace(token);
        *self.oauth_get_time.lock().await = Utc::now().timestamp();
    }

    /// 根据 subscription_key 获取新的 auth key
    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn get_auth_key_by_subscription_key(&self) -> Result<(), TTSServerError> {
        let oauth_key = get_oauth_token(self).await?;
        self.set_new_token(oauth_key).await;
        Ok(())
    }

    ///
    /// 使用 Azure 接口连接文本转语音服务
    pub(crate) async fn get_text_to_speech_connection<T>(
        api_info: &T,
    ) -> Result<WebSocketStream<TlsStream<TcpStream>>, TTSServerError>
        where
            T: AzureAuthKey + Sync + Send,
    {
        let region = api_info.get_region_identifier().await?.value();
        let oauth_token = api_info.get_oauth_token().await?;
        let connect_id = random_string(32);
        let connect_domain = format!("{}.tts.speech.microsoft.com", &region);
        let uri = Uri::builder()
            .scheme("wss")
            .authority(connect_domain.clone())
            .path_and_query(format!(
                "/cognitiveservices/websocket/v1?Authorization=bearer%20{}&X-ConnectionId={}",
                oauth_token, connect_id
            ))
            .build()
            .map_err(|e| {
                error!("{:?}", e);
                TTSServerError::ProgramError(format!("uri build error {:?}", e))
            })?;

        let request_builder = Request::builder()
            .uri(uri)
            .method(Method::GET)
            .header("Accept-Encoding", "gzip, deflate, br")
            .header(
                "Accept-Language",
                "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
            )
            .header("Cache-Control", "no-cache")
            .header("Connection", "Upgrade")
            .header("Host", &connect_domain)
            // .header("Origin", "http://127.0.0.1:8080")
            .header("Pragma", "no-cache")
            .header(
                "Sec-webSocket-Extension",
                "permessage-deflate; client_max_window_bits",
            )
            .header("Sec-WebSocket-Key", generate_key())
            .header("Sec-WebSocket-Version", "13")
            .header("Upgrade", "websocket")
            .header("User-Agent", Self::USER_AGENT)
            .version(Version::HTTP_11);

        let request = request_builder.body(()).map_err(|e| {
            error!("{:?}", e);
            TTSServerError::ProgramError(format!("request_builder build error {:?}", e))
        })?;

        // let jj = native_tls::TlsConnector::new().unwrap();
        let jj = native_tls::TlsConnector::builder()
            .use_sni(true)
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| {
                error!("{:?}", e);
                TTSServerError::ProgramError(format!("创建Tcp连接失败! {:?}", e))
            })?;
        let config = tokio_native_tls::TlsConnector::from(jj);

        let sock = TcpStream::connect(format!("{}:443", &connect_domain))
            .await
            .map_err(|e| {
                error!("连接到微软服务器发生异常，tcp握手失败! 请检查网络! {:?}", e);
                TTSServerError::ProgramError(format!(
                    "连接到微软服务器发生异常，tcp握手失败! 请检查网络! {:?}",
                    e
                ))
            })?;

        let tls_stream = config.connect(&connect_domain, sock).await.map_err(|e| {
            error!("tls 握手失败 {:?}", e);
            TTSServerError::ProgramError(format!("tls 握手失败! {:?}", e))
        })?;

        let websocket = client_async_with_config(
            request,
            tls_stream,
            Some(WebSocketConfig {
                max_send_queue: None,
                max_message_size: None,
                max_frame_size: None,
                accept_unmasked_frames: false,
            }),
        )
            .await
            .map_err(|e| {
                error!("websocket 握手失败 {:?}", e);
                TTSServerError::ProgramError(format!("websocket 握手失败! {:?}", e))
            })?;

        Ok(websocket.0)
    }

    // 以后的版本可能实现的功能，当拥有多个 api时，支持使用负载均衡处理，超过或接近免费额度则更换api
    // pub async fn get_free_quota(self){
    //     // https://management.azure.com/subscriptions/{subscription-id}/resourceGroups/{resource-group-name}/providers/{resource-provider-namespace}/{resource-type}/{resource-name}/providers/microsoft.insights/metrics?metricnames={metric}&timespan={starttime/endtime}&$filter={filter}&resultType=metadata&api-version={apiVersion}
    // }
}

///
/// 为付费订阅版实现获取订阅key
// #[async_trait]
impl AzureAuthSubscription for AzureApiSubscribeToken {
    #[inline]
    fn get_subscription_key(&self) -> BoxFutureSync<Result<String, TTSServerError>> {
        let su_k = self.subscription_key.clone();
        Box::pin(async move { Ok(su_k) })
    }
}

///
/// 为付费订阅版实现获取认证key以及地域
// #[async_trait]
impl AzureAuthKey for AzureApiSubscribeToken {
    /// 获取订阅地区
    #[inline]
    fn get_region_identifier(
        &self,
    ) -> BoxFutureSync<Result<AzureApiRegionIdentifier, TTSServerError>> {
        let ri = self.region_identifier.clone();
        // Ok(self.region_identifier.clone())
        Box::pin(async move { Ok(ri) })
    }
    /// 获取 Azure Oauth Token
    #[inline]
    fn get_oauth_token(&self) -> BoxFutureSync<Result<String, TTSServerError>> {
        Box::pin(async move {
            if self.is_expired().await || self.token_is_none().await {
                self.get_auth_key_by_subscription_key().await?;
            }
            let jj = self.oauth_token.lock().await.clone().unwrap();
            Ok(jj)
        })
    }
}

/// 实现微软官网Api订阅接口获取发音人列表
impl AzureApiSpeakerList for AzureApiSubscribeToken {
    #[inline]
    fn get_vices_list(&self) -> BoxFutureSync<Result<VoicesList, TTSServerError>> {
        Box::pin(async move {
            if self.voices_list.read().await.is_none() {
                let mut voice_arc_list = Vec::new();
                let kk = if let Ok(d) = get_voices_list_by_authkey(self).await {
                    d
                } else {
                    warn!("请求 AzureApiSubscribeToken 发音人列表出错，改用缓存数据！");
                    let data_str = String::from_utf8(AZURE_SPEAKERS_LIST_FILE.to_vec()).unwrap();
                    let tmp_list_1: Vec<VoicesItem> = serde_json::from_str(&data_str).unwrap();
                    tmp_list_1
                };
                for voice in kk {
                    voice_arc_list.push(Arc::new(voice))
                }
                self.voices_list.write().await.replace(voice_arc_list);
            };

            let data = self.voices_list.read().await;

            let list = collating_list_of_pronouncers_arc(data.as_ref().unwrap());
            Ok(list)
        })
    }
}

/// 实现微软官网Api订阅接口获取新连接
impl AzureApiNewWebsocket for AzureApiSubscribeToken {
    #[inline]
    fn get_connection(
        &self,
    ) -> BoxFutureSync<Result<WebSocketStream<TlsStream<TcpStream>>, TTSServerError>> {
        Box::pin(async move {
            let mut new_socket = Self::get_text_to_speech_connection(self).await?;
            let mut msg1 = String::new();
            let create_time = Utc::now();
            let time = format!("{:?}", create_time);
            msg1.push_str(format!("Path: speech.config\r\nX-Timestamp: {}\r\nContent-Type: application/json; charset=utf-8", &time).as_str());
            msg1.push_str("\r\n\r\n");

            let user_agent = Self::USER_AGENT;
            msg1.push_str(r#"{"context":{"system":{"name":"SpeechSDK","version":"1.19.0","build":"JavaScript","lang":"JavaScript"},"os":{"platform":"Browser/Win32","name":""#);
            msg1.push_str(user_agent);
            msg1.push_str(r#"","version":""#);
            msg1.push_str(&user_agent[8..user_agent.len()]);
            msg1.push_str(r#""}}}"#);
            // xmml_data.push(msg1);
            new_socket
                .send(tungstenite::Message::Text(msg1))
                .await
                .map_err(|e| {
                    error!("发送配置数据错误; {:?}", e);
                    TTSServerError::ProgramError("发送配置数据错误".to_owned())
                })?;
            Ok(new_socket)
        })
    }
}

impl AzureApiGenerateXMML for AzureApiSubscribeToken {
    fn generate_xmml(
        &self,
        data: MsTtsMsgRequest,
    ) -> BoxFutureSync<Result<Vec<String>, TTSServerError>> {
        Box::pin(async move {
            let mut xmml_data = Vec::new();
            let create_time = Utc::now();
            let time = format!("{:?}", create_time);

            let mut msg1 = String::new();

            msg1.push_str("Path: synthesis.context\r\n");
            msg1.push_str(format!("X-RequestId: {}\r\n", data.request_id).as_str());
            msg1.push_str(format!("X-Timestamp: {}\r\n", time).as_str());
            msg1.push_str("Content-Type: application/json\r\n\r\n");
            msg1.push_str(r#"{"synthesis":{"audio":{"metadataOptions":{"bookmarkEnabled":false,"sentenceBoundaryEnabled":false,"visemeEnabled":false,"wordBoundaryEnabled":false},"outputFormat":""#);
            msg1.push_str(data.quality.as_str());
            msg1.push_str(r#""},"language":{"autoDetection":false}}}"#);
            xmml_data.push(msg1);

            let mut msg2 = String::new();
            msg2.push_str(format!("Path: ssml\r\nX-RequestId: {}\r\nX-Timestamp:{}\r\nContent-Type:application/ssml+xml\r\n\r\n", &data.request_id, &time).as_str());
            msg2.push_str(format!("<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xmlns:mstts='https://www.w3.org/2001/mstts' xmlns:emo='http://www.w3.org/2009/10/emotionml' xml:lang='en-US'><voice name='{}'><mstts:express-as style='{}' ><prosody rate ='{}%' pitch='{}%'>{}</prosody></mstts:express-as></voice></speak>",
                                  data.informant, data.style, data.rate, data.pitch, data.text).as_str());
            xmml_data.push(msg2);
            Ok(xmml_data)
        })
    }
}

///
/// 微软官网预览页面api接口
#[derive(Debug)]
pub(crate) struct AzureApiPreviewFreeToken {
    /*region_identifier: Option<AzureApiRegionIdentifier>,
    oauth_token: Option<String>,
    oauth_token_base64: Option<String>,
    oauth_get_time: i64,*/
    voices_list: RwLock<Option<Vec<Arc<VoicesItem>>>>,
}

//
// unsafe impl Sync for AzureApiPreviewFreeToken {}
//
// unsafe impl Send for AzureApiPreviewFreeToken {}

impl AzureApiPreviewFreeToken {
    /// 过期时间
    #[allow(dead_code)]
    const EXPIRED_TIME: i64 = 4;

    const USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1379.1";

    /// 获取过期时间
    #[allow(dead_code)]
    fn get_expired_time() -> Duration {
        Duration::minutes(AzureApiPreviewFreeToken::EXPIRED_TIME)
    }

    /// 实例化预览版 Api key
    #[allow(dead_code)]
    pub fn new() -> Arc<Self> {
        static INSTANCE: OnceCell<Arc<AzureApiPreviewFreeToken>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                Arc::new(Self {
                    voices_list: RwLock::new(None),
                })
            })
            .clone()
    }

    async fn get_vices_list_request() -> Result<Vec<Arc<VoicesItem>>, TTSServerError> {
        let url = "https://eastus.api.speech.microsoft.com/cognitiveservices/voices/list";
        let resp = reqwest::Client::new()
            .get(url)
            .header("Host", "eastus.api.speech.microsoft.com")
            .header("Accept", "*/*")
            .header("accept-encoding", "gzip, deflate, br")
            .header(
                "accept-language",
                "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
            )
            .header("origin", "https://azure.microsoft.com")
            .header("referer", "https://azure.microsoft.com/")
            .header(
                "sec-ch-ua",
                r#""Microsoft Edge";v="105", " Not;A Brand";v="99", "Chromium";v="105""#,
            )
            .header("sec-ch-ua-mobile", r#"?0"#)
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", r#"empty"#)
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("user-agent", Self::USER_AGENT)
            .send()
            .await
            .map_err(|e| {
                let err = TTSServerError::ThirdPartyApiCallFailed(format!("{:?}", e));
                error!("request build err: {:#?}",e);
                err
            })?;

        let body = if resp.status() == 200 {
            Ok(resp.bytes().await.map_err(|e| {
                let err = TTSServerError::ProgramError(format!("Error parsing response body! {:?}", e));
                error!("{:?}",err);
                err
            })?)
        } else {
            error!("{:#?}", resp);
            Err(TTSServerError::ThirdPartyApiCallFailed(
                "Third-party interface corresponding error".to_owned(),
            ))
        }?;

        let voice_list: Vec<VoicesItem> = serde_json::from_slice(&body).map_err(|e| {
            error!("{:?}", e);
            TTSServerError::ProgramError(format!("Failed to deserialize voice list! {:?}", e))
        })?;

        let mut voice_arc_list = Vec::new();
        for voice in voice_list {
            voice_arc_list.push(Arc::new(voice))
        }
        Ok(voice_arc_list)
    }

    /*/// 判断认证 Token 是否过期
    #[inline]
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        // let now = Local::now().timestamp();
        let old = Utc.timestamp(self.oauth_get_time, 0);
        return if now - old > Self::get_expired_time() {
            true
        } else {
            false
        };
    }*/
    /*
    /// 从官方网站获取key
    #[inline]
    pub async fn get_auth_key_by_official_website(&mut self) -> Result<(), TTSServerError> {

        let resp = reqwest::Client::new()
            .get("https://azure.microsoft.com/zh-cn/services/cognitive-services/text-to-speech/")
            .send()
            .await
            .map_err(|e| {
                error!("{:?}", e);
                TTSServerError::ProgramError(format!("获取 认证key 失败 {:?}", e))
            })?;
        let html = resp.text().await.map_err(|e| {
            error!("{:?}", e);
            TTSServerError::ProgramError(format!("获取响应体错误 {:?}", e))
        })?;

        let token = Regex::new(r#"token: "([a-zA-Z0-9\._-]+)""#)
            .map_err(|e| {
                error!("{:?}", e);
                TTSServerError::ProgramError(format!("正则匹配错误 {:?}", e))
            })?
            .captures(&html)
            .map(|i| match i {
                Some(t) => {
                    let df = t.get(1).unwrap().as_str();
                    debug!("token获取成功：{}", df);
                    Ok(df.to_owned())
                }
                None => {
                    error!("获取 token 失败");
                    Err(TTSServerError::ProgramError(format!("获取 token 失败")))
                }
            })
            .map_err(|e| {
                error!("{:?}", e);
                TTSServerError::ProgramError(format!("获取 token 失败 {:?}", e))
            })??;

        let region = Regex::new(r#"region: "([a-z0-9]+)""#)
            .map_err(|e| {
                error!("{:?}", e);
                TTSServerError::ProgramError(format!("正则匹配错误 {:?}", e))
            })?
            .captures(&html)
            .map(|i| match i {
                Some(t) => {
                    let df = t.get(1).unwrap().as_str();
                    debug!("region获取成功：{}", df);
                    Ok(df.to_owned())
                }
                None => {
                    error!("获取 region 失败");
                    Err(TTSServerError::ProgramError(format!("获取 region 失败")))
                }
            })
            .map_err(|e| {
                error!("{:?}", e);
                TTSServerError::ProgramError(format!("获取 region 失败 {:?}", e))
            })??;

        self.oauth_token_base64.replace(base64::encode(&token));
        self.oauth_token.replace(token);
        self.region_identifier.replace(AzureApiRegionIdentifier::from(&region)?);
        self.oauth_get_time = Utc::now().timestamp();

        Ok(())
    }*/
}

/// 实现微软官网免费预览接口获取发音人列表
impl AzureApiSpeakerList for AzureApiPreviewFreeToken {
    #[inline]
    fn get_vices_list(&self) -> BoxFutureSync<Result<VoicesList, TTSServerError>> {
        Box::pin(async move {
            if self.voices_list.read().await.is_none() {
                let kk = if let Ok(d) = Self::get_vices_list_request().await {
                    d
                } else {
                    warn!("请求 AzureApiPreviewFreeToken 发音人列表出错，改用缓存数据！");
                    let data_str = String::from_utf8(AZURE_SPEAKERS_LIST_FILE.to_vec()).unwrap();
                    let tmp_list_1: Vec<VoicesItem> = serde_json::from_str(&data_str).unwrap();
                    let mut voice_arc_list = Vec::new();
                    for voice in tmp_list_1 {
                        voice_arc_list.push(Arc::new(voice))
                    }
                    voice_arc_list
                };
                self.voices_list.write().await.replace(kk);
            }
            let data = self.voices_list.read().await;
            let list = collating_list_of_pronouncers_arc(data.as_ref().unwrap());
            Ok(list)
        })
    }
}

/// 实现微软官网Api订阅接口获取新连接
impl AzureApiNewWebsocket for AzureApiPreviewFreeToken {
    #[inline]
    fn get_connection(
        &self,
    ) -> BoxFutureSync<Result<WebSocketStream<TlsStream<TcpStream>>, TTSServerError>> {
        Box::pin(async move {
            let connect_id = random_string(32);
            let connect_domain = "eastus.api.speech.microsoft.com";
            let user_agent = Self::USER_AGENT;
            let uri = Uri::builder()
                .scheme("wss")
                .authority(connect_domain)
                .path_and_query(format!(
                    "/cognitiveservices/websocket/v1?TrafficType=AzureDemo&Authorization=bearer%20undefined&X-ConnectionId={}",
                    &connect_id
                ))
                .build()
                .map_err(|e| {
                    let err = format!("AzureApiPreviewFreeToken 请求端点构建失败 {:?}", e);
                    error!("{}", err);
                    TTSServerError::ProgramError(err)
                })?;

            let request_builder = Request::builder()
                .uri(uri)
                .method(Method::GET)
                // .header("Authorization", format!("Bearer {}", &oauth_token_base64))
                .header("Cache-Control", "no-cache")
                .header("Pragma", "no-cache")
                .header("Accept", "*/*")
                .header("Accept-Encoding", "gzip, deflate, br")
                .header(
                    "Accept-Language",
                    "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
                )
                .header("User-Agent", user_agent)
                .header("Host", connect_domain)
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", generate_key())
                .header(
                    "Sec-webSocket-Extension",
                    "permessage-deflate; client_max_window_bits",
                )
                .version(Version::HTTP_11);
            let request = request_builder.body(()).map_err(|e| {
                let err = format!("AzureApiPreviewFreeToken 请求体构建失败 {:?}", e);
                error!("{}", err);
                TTSServerError::ProgramError(err)
            })?;

            // let jj = native_tls::TlsConnector::new().unwrap();
            let jj = native_tls::TlsConnector::builder()
                .use_sni(false)
                .build()
                .map_err(|e| {
                    let err = format!("创建Tcp连接失败! {:?}", e);
                    error!("{}", err);
                    TTSServerError::ProgramError(err)
                })?;
            let config = tokio_native_tls::TlsConnector::from(jj);

            let sock = TcpStream::connect(format!("{}:443", &connect_domain))
                .await
                .map_err(|e| {
                    let err = format!(
                        "连接到微软服务器发生异常，tcp握手失败! 请检查网络! {:?}",
                        e
                    );
                    error!("{}", err);
                    TTSServerError::ProgramError(err)
                })?;

            let tsl_stream = config.connect(&connect_domain, sock).await.map_err(|e| {
                let err = format!("tsl 握手失败! {:?}", e);
                error!("{}", err);
                TTSServerError::ProgramError(err)
            })?;

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
                .await
                .map_err(|e| {
                    let err = format!("websocket 握手失败! {:?}", e);
                    error!("{}", err);
                    TTSServerError::ProgramError(err)
                })?;

            let mut new_socket = websocket.0;

            let mut msg1 = String::new();
            let create_time = Utc::now();
            let time = format!("{:?}", create_time);
            msg1.push_str(format!("Path: speech.config\r\nX-Timestamp: {}\r\nContent-Type: application/json; charset=utf-8", &time).as_str());
            msg1.push_str("\r\n\r\n");
            msg1.push_str(r#"{"context":{"system":{"name":"SpeechSDK","version":"1.19.0","build":"JavaScript","lang":"JavaScript"},"os":{"platform":"Browser/Win32","name":""#);
            msg1.push_str(user_agent);
            msg1.push_str(r#"","version":""#);
            msg1.push_str(&user_agent[8..user_agent.len()]);
            msg1.push_str(r#""}}}"#);
            // xmml_data.push(msg1);
            new_socket
                .send(tungstenite::Message::Text(msg1))
                .await
                .map_err(|e| {
                    let err = "发送配置数据错误".to_owned();
                    error!("{}", err);
                    TTSServerError::ProgramError(err)
                })?;

            Ok(new_socket)
        })
    }
}

impl AzureApiGenerateXMML for AzureApiPreviewFreeToken {
    fn generate_xmml(
        &self,
        data: MsTtsMsgRequest,
    ) -> BoxFutureSync<Result<Vec<String>, TTSServerError>> {
        Box::pin(async move {
            let mut xmml_data = Vec::new();
            let create_time = Utc::now();
            let time = format!("{:?}", create_time);

            let mut msg1 = String::new();

            msg1.push_str("Path: synthesis.context\r\n");
            msg1.push_str(format!("X-RequestId: {}\r\n", data.request_id).as_str());
            msg1.push_str(format!("X-Timestamp: {}\r\n", time).as_str());
            msg1.push_str("Content-Type: application/json\r\n\r\n");
            msg1.push_str(r#"{"synthesis":{"audio":{"metadataOptions":{"bookmarkEnabled":false,"sentenceBoundaryEnabled":false,"visemeEnabled":false,"wordBoundaryEnabled":false},"outputFormat":"audio-24khz-96kbitrate-mono-mp3"},"language":{"autoDetection":false}}}"#);
            xmml_data.push(msg1);

            let mut msg2 = String::new();
            msg2.push_str(format!("Path: ssml\r\nX-RequestId: {}\r\nX-Timestamp:{}\r\nContent-Type:application/ssml+xml\r\n\r\n", &data.request_id, &time).as_str());
            msg2.push_str(format!("<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xmlns:mstts='https://www.w3.org/2001/mstts' xmlns:emo='http://www.w3.org/2009/10/emotionml' xml:lang='en-US'><voice name='{}'><mstts:express-as style='{}' ><prosody rate ='{}%' pitch='{}%'>{}</prosody></mstts:express-as></voice></speak>",
                                  data.informant, data.style, data.rate, data.pitch, data.text).as_str());
            xmml_data.push(msg2);
            Ok(xmml_data)
        })
    }
}

///
/// 微软 Edge浏览器 免费预览 api
#[derive(Debug)]
pub(crate) struct AzureApiEdgeFree {
    voices_list: RwLock<Option<Vec<Arc<VoicesItem>>>>,
}

impl AzureApiEdgeFree {
    pub(crate) const TOKEN: [u8; 32] = [
        54, 65, 53, 65, 65, 49, 68, 52, 69, 65, 70, 70, 52, 69, 57, 70, 66, 51, 55, 69, 50, 51, 68,
        54, 56, 52, 57, 49, 68, 54, 70, 52,
    ];

    #[allow(dead_code)]
    pub(crate) const MS_TTS_SERVER_CHINA_LIST: [&'static str; 6] = [
        // 北京节点
        "202.89.233.100",
        "202.89.233.101",
        "202.89.233.102",
        "202.89.233.103",
        "202.89.233.104",
        "182.61.148.24",
    ];

    #[allow(dead_code)]
    pub(crate) const MS_TTS_SERVER_CHINA_HK_LIST: [&'static str; 8] = [
        "149.129.121.248",
        // "103.200.112.245",
        // "47.90.51.125",
        // "61.239.177.5",
        "149.129.88.238",
        "103.68.61.91",
        "47.75.141.93",
        // "34.96.186.48",
        "47.240.87.168",
        "47.57.114.186",
        "150.109.51.247",
        // "20.205.113.91",
        "35.241.115.60",
    ];

    #[allow(dead_code)]
    pub(crate) const MS_TTS_SERVER_CHINA_TW_LIST: [&'static str; 2] = [
        "34.81.240.201",
        "34.80.106.199",
    ];

    const USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36 Edg/102.0.1245.39";

    #[allow(dead_code)]
    pub(crate) fn new() -> Arc<Self> {
        static INSTANCE: OnceCell<Arc<AzureApiEdgeFree>> = OnceCell::new();

        INSTANCE
            .get_or_init(|| {
                Arc::new(Self {
                    voices_list: RwLock::new(None),
                })
            })
            .clone()
    }

    ///
    /// edge 免费版本
    /// 根据命令行中地域配置进行连接
    ///
    pub(crate) async fn new_websocket_edge_free() -> Result<WebSocketStream<TlsStream<TcpStream>>, TTSServerError> {
        #[cfg(test)]
            let args = AppArgs::test_parse_macro(&[]);
        #[cfg(not(test))]
            let args = AppArgs::parse_macro();
        debug!("指定加速ip {:#?}", args.server_area);
        match args.server_area {
            ServerArea::Default => {
                info!("连接至官方服务器, 根据 dns 解析");
                Self::new_websocket_by_select_server(None).await
                // new_websocket_by_select_server(Some("171.117.98.148")).await
            }
            ServerArea::China => {
                info!("连接至内陆服务器");
                let select = rand::thread_rng()
                    .gen_range(0..AzureApiEdgeFree::MS_TTS_SERVER_CHINA_LIST.len());
                Self::new_websocket_by_select_server(Some(
                    AzureApiEdgeFree::MS_TTS_SERVER_CHINA_LIST
                        .get(select)
                        .unwrap(),
                ))
                    .await
            }
            ServerArea::ChinaHK => {
                info!("连接至香港服务器");
                let select = rand::thread_rng()
                    .gen_range(0..AzureApiEdgeFree::MS_TTS_SERVER_CHINA_HK_LIST.len());
                Self::new_websocket_by_select_server(Some(
                    AzureApiEdgeFree::MS_TTS_SERVER_CHINA_HK_LIST
                        .get(select)
                        .unwrap(),
                ))
                    .await
            }
            ServerArea::ChinaTW => {
                info!("连接至台湾服务器");
                let select = rand::thread_rng()
                    .gen_range(0..AzureApiEdgeFree::MS_TTS_SERVER_CHINA_TW_LIST.len());
                Self::new_websocket_by_select_server(Some(
                    AzureApiEdgeFree::MS_TTS_SERVER_CHINA_TW_LIST
                        .get(select)
                        .unwrap(),
                ))
                    .await
            }
        }
    }

    ///
    /// edge 免费版本
    /// 获取新的隧道连接
    pub(crate) async fn new_websocket_by_select_server(
        server: Option<&str>,
    ) -> Result<WebSocketStream<TlsStream<TcpStream>>, TTSServerError> {
        let connect_id = random_string(32);
        let uri = Uri::builder()
            .scheme("wss")
            .authority("speech.platform.bing.com")
            .path_and_query(format!(
                "/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&ConnectionId={}",
                String::from_utf8(AzureApiEdgeFree::TOKEN.to_vec()).unwrap(),
                &connect_id
            ))
            .build();
        if let Err(e) = uri {
            return Err(TTSServerError::ProgramError(format!(
                "uri 构建错误 {:?}",
                e
            )));
        }
        let uri = uri.unwrap();

        let request_builder = Request::builder()
            .uri(uri)
            .method(Method::GET)
            .header("Cache-Control", "no-cache")
            .header("Pragma", "no-cache")
            .header("Accept", "*/*")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header(
                "Accept-Language",
                "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
            )
            .header("User-Agent", Self::USER_AGENT)
            .header(
                "Origin",
                "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold",
            )
            .header("Host", "speech.platform.bing.com")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key())
            .header(
                "Sec-webSocket-Extension",
                "permessage-deflate; client_max_window_bits",
            )
            .version(Version::HTTP_11);
        let request = request_builder.body(());
        if let Err(e) = request {
            return Err(TTSServerError::ProgramError(format!(
                "request_builder 构建错误 {:?}",
                e
            )));
        }
        let request = request.unwrap();

        let domain = "speech.platform.bing.com";

        // let jj = native_tls::TlsConnector::new().unwrap();
        let jj = native_tls::TlsConnector::builder()
            .use_sni(false)
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();

        let config = tokio_native_tls::TlsConnector::from(jj);

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
                info!("连接至 speech.platform.bing.com");
                TcpStream::connect("speech.platform.bing.com:443").await
            }
        };

        if let Err(e) = sock {
            return Err(TTSServerError::ProgramError(format!(
                "连接到微软服务器发生异常，tcp握手失败! 请检查网络! {:?}",
                e
            )));
        }
        let sock = sock.unwrap();

        let tsl_stream = config.connect(domain, sock).await;

        if let Err(e) = tsl_stream {
            error!("{:?}", e);
            return Err(TTSServerError::ProgramError(format!("tsl握手失败! {}", e)));
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
            Err(e2) => Err(TTSServerError::ProgramError(format!(
                "websocket 握手失败! {:?}",
                e2
            ))),
        };
    }

    async fn get_vices_list_request() -> Result<Vec<Arc<VoicesItem>>, TTSServerError> {
        let url = format!(
            "https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/voices/list?trustedclienttoken={}",
            String::from_utf8(Self::TOKEN.to_vec()).unwrap()
        );
        let resp = reqwest::Client::new()
            .get(url)
            .header("Host", "speech.platform.bing.com")
            .header("Connection", "keep-alive")
            .header(
                "sec-ch-ua",
                r#""Microsoft Edge";v="107", "Chromium";v="107", "Not=A?Brand";v="24""#,
            )
            .header(
                "sec-ch-ua-mobile",
                r#"?0"#,
            )
            .header("User-Agent", Self::USER_AGENT)
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("Accept", "*/*")
            .header(
                "X-Edge-Shopping-Flag",
                "1",
            ).header(
            "Sec-Fetch-Site",
            "none",
        ).header(
            "Sec-Fetch-Mode",
            "cors",
        ).header(
            "Sec-Fetch-Dest",
            "empty",
        )
            .header("Accept-Encoding", "gzip, deflate, br")
            .header(
                "Accept-Language",
                "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
            )
            .send()
            .await
            .map_err(|e| {
                error!("request build err: {:#?}",e);
                TTSServerError::ThirdPartyApiCallFailed(format!("{:?}", e.to_string()))
            })?;

        let body = if resp.status() == 200 {
            Ok(resp.bytes().await.map_err(|e| {
                error!("{:#?}", e);
                TTSServerError::ProgramError(format!("Error parsing response body! {:?}", e))
            })?)
        } else {
            error!("{:#?}", resp);
            Err(TTSServerError::ThirdPartyApiCallFailed(
                "Third-party interface corresponding error".to_owned(),
            ))
        }?;
        let voice_list: Vec<VoicesItem> = serde_json::from_slice(&body).map_err(|e| {
            error!("{:?}", e);
            TTSServerError::ProgramError(format!("Failed to deserialize voice list! {:?}", e))
        })?;
        let mut voice_arc_list = Vec::new();
        for voice in voice_list {
            voice_arc_list.push(Arc::new(voice))
        }

        Ok(voice_arc_list)
    }
}

/// 实现微软官网免费预览接口获取发音人列表
impl AzureApiSpeakerList for AzureApiEdgeFree {
    #[inline]
    fn get_vices_list(&self) -> BoxFutureSync<Result<VoicesList, TTSServerError>> {
        Box::pin(async move {
            if self.voices_list.read().await.is_none() {
                let kk = if let Ok(d) = Self::get_vices_list_request().await {
                    d
                } else {
                    warn!("请求 AzureApiEdgeFree 发音人列表出错，改用缓存数据！");
                    let data_str = String::from_utf8(EDGE_SPEAKERS_LIST_FILE.to_vec()).unwrap();
                    let tmp_list_1: Vec<VoicesItem> = serde_json::from_str(&data_str).unwrap();
                    let mut voice_arc_list = Vec::new();
                    for voice in tmp_list_1 {
                        voice_arc_list.push(Arc::new(voice))
                    }
                    voice_arc_list
                };
                self.voices_list.write().await.replace(kk);
            };

            let data = self.voices_list.read().await;

            let list = collating_list_of_pronouncers_arc(data.as_ref().unwrap());
            Ok(list)
        })
    }
}

impl AzureApiNewWebsocket for AzureApiEdgeFree {
    fn get_connection(
        &self,
    ) -> BoxFutureSync<Result<WebSocketStream<TlsStream<TcpStream>>, TTSServerError>> {
        Box::pin(async move {
            let mut new_socket = Self::new_websocket_edge_free().await?;
            let mut msg1 = String::new();
            let create_time = Utc::now();
            let time = format!("{:?}", create_time);
            msg1.push_str(format!("X-Timestamp: {}\r\nContent-Type: application/json; charset=utf-8\r\nPath: speech.config", &time).as_str());
            msg1.push_str("\r\n\r\n");
            msg1.push_str(r#"{"context":{"synthesis":{"audio":{"metadataoptions":{"sentenceBoundaryEnabled":"false","wordBoundaryEnabled":"false"},"outputFormat":"webm-24khz-16bit-mono-opus"}}}}"#);
            // xmml_data.push(msg1);
            new_socket
                .send(tungstenite::Message::Text(msg1))
                .await
                .map_err(|e| {
                    let err = format!("发送配置数据错误; {:?}", e);
                    error!("{}",err);
                    TTSServerError::ProgramError(err)
                })?;
            Ok(new_socket)
        })
    }
}

impl AzureApiGenerateXMML for AzureApiEdgeFree {
    fn generate_xmml(
        &self,
        data: MsTtsMsgRequest,
    ) -> BoxFutureSync<Result<Vec<String>, TTSServerError>> {
        Box::pin(async move {
            let mut xmml_data = Vec::new();
            let create_time = Utc::now();
            let time = format!("{:?}", create_time);
            let mut msg2 = String::new();
            msg2.push_str(format!("X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{}\r\nPath:ssml\r\n\r\n", &data.request_id, &time).as_str());
            msg2.push_str(format!("<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xmlns:mstts='https://www.w3.org/2001/mstts' xml:lang='en-US'><voice name='{}'><prosody pitch='+0Hz' rate ='{}%' volume='+0%'>{}</prosody></voice></speak>",
                                  data.informant, data.rate, data.text).as_str());
            xmml_data.push(msg2);
            Ok(xmml_data)
        })
    }
}

///
/// 微软文本转语音认证 token 获取，使用官方 Api
pub(crate) async fn get_oauth_token<T>(api_info: &T) -> Result<String, TTSServerError>
    where
        T: AzureAuthSubscription + AzureAuthKey + Sync + Send,
{
    let region = api_info.get_region_identifier().await?.value();
    let key = api_info.get_subscription_key().await?;
    let resp = reqwest::Client::new()
        .post(format!(
            "https://{}.api.cognitive.microsoft.com/sts/v1.0/issueToken",
            &region
        ))
        .header("Ocp-Apim-Subscription-Key", key)
        .header("Host", format!("{}.api.cognitive.microsoft.com", &region))
        .header("Content-type", "application/x-www-form-urlencoded")
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(|e| {
            let err = TTSServerError::ThirdPartyApiCallFailed(format!("{:?}", e.to_string()));
            error!("{:?}", e);
            err
        })?;
    let body = if resp.status() == 200 {
        Ok(resp.bytes().await.map_err(|e| {
            error!("Error parsing response body! {:#?}", e);
            TTSServerError::ProgramError(format!("Error parsing response body! {}", e.to_string()))
        })?)
    } else if resp.status() == 401 {
        error!(
            "请检查您传入的 订阅KEY 以及 地域 是否正确, Azure Api 调用失败; {:?}",
            resp
        );
        Err(TTSServerError::ThirdPartyApiCallFailed(
            "Call without permission!".to_owned(),
        ))
    } else {
        error!("Third-party interface corresponding error: {:#?}", resp);
        Err(TTSServerError::ThirdPartyApiCallFailed(
            "Third-party interface corresponding error".to_owned(),
        ))
    }?;

    Ok(String::from_utf8(body.to_vec()).map_err(|e| {
        error!("{:#?}", e);
        TTSServerError::ProgramError(format!("Binary to string error! {}", e.to_string()))
    })?)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum VoicesItem {
    AzureApi {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "DisplayName")]
        display_name: String,
        #[serde(rename = "LocalName")]
        local_name: String,
        #[serde(rename = "ShortName")]
        short_name: String,
        #[serde(rename = "Gender")]
        gender: String,
        #[serde(rename = "Locale")]
        locale: String,
        #[serde(rename = "LocaleName")]
        locale_name: String,
        #[serde(rename = "StyleList", skip_serializing_if = "Option::is_none")]
        style_list: Option<Vec<String>>,
        #[serde(rename = "SampleRateHertz")]
        sample_rate_hertz: String,
        #[serde(rename = "VoiceType")]
        voice_type: String,
        #[serde(rename = "Status")]
        status: String,
        #[serde(rename = "RolePlayList", skip_serializing_if = "Option::is_none")]
        role_play_list: Option<Vec<String>>,
        #[serde(rename = "WordsPerMinute", skip_serializing_if = "Option::is_none")]
        words_per_minute: Option<String>,
    },
    EdgeApi {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "ShortName")]
        short_name: String,
        #[serde(rename = "Gender")]
        gender: String,
        #[serde(rename = "Locale")]
        locale: String,
        #[serde(rename = "SuggestedCodec")]
        suggested_codec: String,
        #[serde(rename = "FriendlyName")]
        friendly_name: String,
        #[serde(rename = "Status")]
        status: String,
        #[serde(rename = "VoiceTag")]
        voice_tag: HashMap<String, Vec<String>>,
    },
}


// ///
// /// Azure 官方文本转语音 发音人结构体
// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct VoicesItem {
//     #[serde(rename = "Name")]
//     pub name: String,
//     #[serde(rename = "DisplayName", skip_serializing_if = "Option::is_none")]
//     pub display_name: Option<String>,
//
//     #[serde(rename = "LocalName")]
//     pub local_name: String,
//     #[serde(rename = "ShortName")]
//     pub short_name: String,
//     #[serde(rename = "Gender")]
//     pub gender: String,
//     #[serde(rename = "Locale")]
//     pub locale: String,
//     #[serde(rename = "LocaleName")]
//     pub locale_name: String,
//     #[serde(rename = "StyleList", skip_serializing_if = "Option::is_none")]
//     pub style_list: Option<Vec<String>>,
//     #[serde(rename = "SampleRateHertz")]
//     pub sample_rate_hertz: String,
//     #[serde(rename = "VoiceType")]
//     pub voice_type: String,
//     #[serde(rename = "Status")]
//     pub status: String,
//     #[serde(rename = "RolePlayList", skip_serializing_if = "Option::is_none")]
//     pub role_play_list: Option<Vec<String>>,
//
//     // edge api 接口独立数据
//     #[serde(rename = "FriendlyName", skip_serializing_if = "Option::is_none")]
//     pub friendly_name: Option<String>,
//     #[serde(rename = "SuggestedCodec", skip_serializing_if = "Option::is_none")]
//     pub suggested_codec: Option<String>,
//     #[serde(rename = "WordsPerMinute", skip_serializing_if = "Option::is_none")]
//     pub words_per_minute: Option<Vec<String>>,
//     #[serde(rename = "VoiceTag", skip_serializing_if = "Option::is_none")]
//     pub voice_tag: Option<HashMap<String,Vec<String>>>,
// }

impl VoicesItem {
    #[inline]
    pub fn get_short_name(&self) -> String {
        return match self {
            VoicesItem::AzureApi { short_name, .. } => { short_name.clone() }
            VoicesItem::EdgeApi { short_name, .. } => { short_name.clone() }
        };
    }
    #[inline]
    pub fn get_local(&self) -> &str {
        return match self {
            VoicesItem::AzureApi { locale, .. } => { locale.as_str() }
            VoicesItem::EdgeApi { locale, .. } => { locale.as_str() }
        };
    }
    #[inline]
    pub fn get_style(&self) -> Option<Vec<String>> {
        return match self {
            VoicesItem::AzureApi { style_list, .. } => { style_list.clone() }
            VoicesItem::EdgeApi { voice_tag, .. } => {
                if let Some(e0) = voice_tag.get("ContentCategories") {
                    Some(e0.clone())
                } else {
                    None
                }
            }
        };
    }

    pub fn get_desc(&self) -> String {
        return match self {
            VoicesItem::AzureApi { voice_type, local_name, display_name, .. } => {
                if voice_type == "Neural" {
                    format!("Microsoft {} Online (Natural) - {}", display_name, local_name)
                } else {
                    format!("Microsoft {} Online - {}", display_name, local_name)
                }
            }
            VoicesItem::EdgeApi { friendly_name, .. } => {
                friendly_name.clone()
            }
        };
    }
}

impl PartialEq for VoicesItem {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match self {
            VoicesItem::AzureApi {
                short_name: short_name1,
                ..
            } => {
                if let VoicesItem::AzureApi { short_name: short_name2, .. } = other {
                    short_name1 == short_name2
                } else {
                    false
                }
            }
            VoicesItem::EdgeApi {
                short_name: short_name1,
                ..
            } => {
                if let VoicesItem::EdgeApi { short_name: short_name2, .. } = other {
                    short_name1 == short_name2
                } else {
                    false
                }
            }
        }
    }
}

///
/// 根据 Api key 获取文本转语音 发音人列表
async fn get_voices_list_by_authkey<T>(api_info: &T) -> Result<Vec<VoicesItem>, TTSServerError>
    where
        T: AzureAuthKey + Sync + Send,
{
    let region = api_info.get_region_identifier().await?.value();
    let oauth_token = api_info.get_oauth_token().await?;
    let resp = reqwest::Client::new()
        .get(format!(
            "https://{}.tts.speech.microsoft.com/cognitiveservices/voices/list",
            &region
        ))
        .header("Authorization", format!("Bearer {}", oauth_token))
        .header("Host", format!("{}.tts.speech.microsoft.com", &region))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| TTSServerError::ThirdPartyApiCallFailed(format!("{:?}", e.to_string())))?;

    let body = if resp.status() == 200 {
        Ok(resp.bytes().await.map_err(|e| {
            TTSServerError::ProgramError(format!("Error parsing response body! {:?}", e))
        })?)
    } else {
        error!("{:#?}", resp);
        Err(TTSServerError::ThirdPartyApiCallFailed(
            "Third-party interface corresponding error".to_owned(),
        ))
    }?;

    let voice_list: Vec<VoicesItem> = serde_json::from_slice(&body).map_err(|e| {
        error!("{:?}", e);
        TTSServerError::ProgramError(format!("Failed to deserialize voice list! {:?}", e))
    })?;

    Ok(voice_list)
}

pub(crate) fn collating_list_of_pronouncers(list: Vec<VoicesItem>) -> VoicesList {
    let mut raw_data: Vec<Arc<VoicesItem>> = Vec::new();
    let mut voices_name_list: HashSet<String> = HashSet::new();
    let mut by_voices_name_map: HashMap<String, Arc<VoicesItem>> = HashMap::new();

    list.iter().for_each(|item| {
        let new = Arc::new(item.clone());
        raw_data.push(new.clone());
        voices_name_list.insert(item.get_short_name());
        by_voices_name_map.insert(item.get_short_name(), new);
    });

    let mut by_locale_map: HashMap<String, Vec<Arc<VoicesItem>>> = HashMap::new();

    let new_iter = raw_data.iter();
    for (key, group) in &new_iter.group_by(|i| i.get_local()) {
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
    return v_list;
}

pub(crate) fn collating_list_of_pronouncers_arc(raw_data: &Vec<Arc<VoicesItem>>) -> VoicesList {
    let mut voices_name_list: HashSet<String> = HashSet::new();
    let mut by_voices_name_map: HashMap<String, Arc<VoicesItem>> = HashMap::new();

    raw_data.iter().for_each(|item| {
        let new = item.clone();
        voices_name_list.insert(item.get_short_name());
        by_voices_name_map.insert(item.get_short_name(), new);
    });

    let mut by_locale_map: HashMap<String, Vec<Arc<VoicesItem>>> = HashMap::new();

    let new_iter = raw_data.iter();
    for (key, group) in &new_iter.group_by(|&i| i.get_local()) {
        let mut locale_vec_list: Vec<Arc<VoicesItem>> = Vec::new();

        group.for_each(|j| {
            locale_vec_list.push(j.clone());
        });
        by_locale_map.insert(key.to_owned(), locale_vec_list);
    }

    let v_list = VoicesList {
        voices_name_list,
        raw_data: raw_data.clone(),
        by_voices_name_map,
        by_locale_map,
    };
    return v_list;
}
