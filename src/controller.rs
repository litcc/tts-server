use crate::{info, random_string, MsTtsMsgRequest};
use actix_web::http::StatusCode;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use fancy_regex::Regex;
use log::{debug, error, warn};
use mime_guess::from_path;
use rust_embed::RustEmbed;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use crate::ms_tts::{MsTtsMsgResponse, VoicesItem, MS_TTS_CONFIG};
use urlencoding::decode as url_decode;

// ##### Error Struct ############################################################################

#[derive(Debug)]
pub enum ControllerError {
    TextNone(String),
}

impl Display for ControllerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Error: {})", self)
    }
}

impl Error for ControllerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiBaseResponse<T> {
    pub code: i32,
    pub data: Option<T>,
    pub msg: String,
}

impl<T> ToString for ApiBaseResponse<T>
where
    T: Serialize + DeserializeOwned + Debug,
{
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl<T> ApiBaseResponse<T> {
    pub fn success(data: T) -> ApiBaseResponse<T> {
        ApiBaseResponse {
            code: 0,
            data: Some(data),
            msg: "success".to_string(),
        }
    }

    pub fn error(msg: &str) -> ApiBaseResponse<T> {
        ApiBaseResponse {
            code: -1,
            data: None,
            msg: msg.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct MsTtsMsgRequestJson {
    // 待生成文本
    pub text: String,
    // 发音人
    pub informant: Option<String>,
    // 音频风格
    pub style: Option<String>,
    // 语速
    pub rate: Option<f32>,
    // 音调
    pub pitch: Option<f32>,
    // 音频格式
    pub quality: Option<String>,
    // text_replace_list:Vec<String>,
    // phoneme_list:Vec<String>
}

impl MsTtsMsgRequestJson {
    pub fn to_ms_request(
        &self,
        request_id_value: String,
    ) -> Result<MsTtsMsgRequest, ControllerError> {
        let text_value: String = {
            let mut text_tmp1 = self.text.as_str().to_string();
            // url 解码
            let text_tmp2: String = 'break_1: loop {
                let decoded = url_decode(&text_tmp1);
                if let Ok(s) = decoded {
                    if text_tmp1 == s.to_string() {
                        break 'break_1 text_tmp1;
                    }
                    text_tmp1 = s.to_string();
                } else {
                    break 'break_1 text_tmp1;
                }
            };
            if text_tmp2.is_empty() {
                // 如果文字为空则返回1秒空音乐
                return Err(ControllerError::TextNone("".to_string()));
            }

            let result = Regex::new(r"？")
                .unwrap()
                .replace_all(&text_tmp2, "? ")
                .to_string();
            let result = Regex::new(r"，")
                .unwrap()
                .replace_all(&result, ", ")
                .to_string();
            let result = Regex::new(r"。")
                .unwrap()
                .replace_all(&result, ". ")
                .to_string();
            let result = Regex::new(r"：")
                .unwrap()
                .replace_all(&result, ": ")
                .to_string();
            let result = Regex::new(r"；")
                .unwrap()
                .replace_all(&result, "; ")
                .to_string();

            result
        };
        let ms_tts_config = &MS_TTS_CONFIG.get().unwrap();

        let informant_value: String = {
            let default = "zh-CN-XiaoxiaoNeural".to_owned();

            match &self.informant {
                Some(inf) => {
                    if ms_tts_config.voices_list.voices_name_list.contains(inf) {
                        inf.to_string()
                    } else {
                        default
                    }
                }
                None => default,
            }
        };

        let informant_item = ms_tts_config
            .voices_list
            .by_voices_name_map
            .get(&informant_value)
            .unwrap();

        let style_value: String = {
            let default = "general".to_owned();
            if let Some(style) = &self.style {
                match &informant_item.style_list {
                    Some(e) => {
                        let s_t = style.to_lowercase();
                        if e.contains(&s_t) {
                            s_t.to_owned()
                        } else {
                            default
                        }
                    }
                    None => default,
                }
            } else {
                default
            }
        };

        let rate_value: String = {
            let default = "0".to_owned();

            if let Some(style) = &self.rate {
                // num::Num
                if style <= &0.0 {
                    "-100".to_owned()
                } else if style >= &3.0 {
                    "200".to_owned()
                } else {
                    let tmp = 100.00 * style - 100.00;
                    format!("{:.0}", tmp)
                }
            } else {
                default
            }
        };

        let pitch_value: String = {
            let default = "0".to_owned();
            if let Some(pitch) = &self.pitch {
                if pitch <= &0.0 {
                    "-50".to_owned()
                } else if pitch >= &2.0 {
                    "50".to_owned()
                } else {
                    let tmp = 50.00 * pitch - 50.00;
                    format!("{:.0}", tmp)
                }
            } else {
                default
            }
        };
        let quality_list = &ms_tts_config.quality_list;

        let quality_value: String = {
            let default = "audio-24khz-48kbitrate-mono-mp3".to_owned();
            if let Some(quality) = &self.quality {
                if quality_list.contains(quality) {
                    quality.to_owned()
                } else {
                    default
                }
            } else {
                default
            }
        };

        Ok(MsTtsMsgRequest {
            text: text_value,
            request_id: request_id_value,
            informant: informant_value,
            style: style_value,
            rate: rate_value,
            pitch: pitch_value,
            quality: quality_value,
        })
    }
}

/// 监听

pub(crate) async fn register_service(address: String, port: String) {
    let web_server = HttpServer::new(|| {
        let mut app = App::new();

        // api
        app = app.service(
            web::resource("/api-list")
                .route(web::get().to(get_api_list))
                .route(web::post().to(get_api_list)),
        );

        // 微软 TTS 文本转语音 相关接口
        app = app
            .service(
                web::resource("/tts-ms")
                    .route(web::get().to(tts_ms_get_controller))
                    .route(web::post().to(tts_ms_post_controller)),
            )
            .service(
                web::resource("/ms-tts/style/{informant}")
                    // .name("user_detail")
                    .route(web::get().to(get_ms_tts_style)),
            );
        app = app
            .route("/ms-tts/informant", web::get().to(get_ms_tts_informant))
            .route("/ms-tts/quality", web::get().to(get_ms_tts_quality));

        // web
        app = app
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/{_:.*}").route(web::get().to(dist)));

        app
    });
    let web_server = web_server.bind(format!("{}:{}", address, port));
    match web_server {
        Ok(server) => {
            let local_ip = local_ipaddress::get();
            info!(
                "启动 Api 服务成功 接口地址已监听至: http://{}:{}/tts-ms  自行修改 ip 以及 port",
                address, port
            );
            if local_ip.is_some() {
                info!(
                    "您当前局域网ip可能为: {} 请自行替换上面的监听地址",
                    local_ip.unwrap()
                );
            }
            server
                .workers(1)
                .max_connections(1000)
                .backlog(1000)
                .run()
                .await
                .unwrap();
        }
        Err(_e) => {
            error!("启动 Api 服务失败，无法监听 {}:{}", address, port);
        }
    }
}

async fn tts_ms_post_controller(
    _req: HttpRequest,
    body: web::Json<MsTtsMsgRequestJson>,
) -> HttpResponse {
    let id = random_string(32);
    debug!("收到 post 请求{:?}", body);
    let request_tmp = body.to_ms_request(id.clone());
    info!("解析 post 请求 {:?}", request_tmp);
    let re = request_ms_tts(request_tmp).await;
    debug!("响应 post 请求 {}", &id);
    return re;
}

async fn tts_ms_get_controller(
    _req: HttpRequest,
    request: web::Query<MsTtsMsgRequestJson>,
) -> HttpResponse {
    let id = random_string(32);
    debug!("收到 get 请求{:?}", request);
    let request_tmp = request.to_ms_request(id.clone());
    info!("解析 get 请求 {:?}", request_tmp);
    let re = request_ms_tts(request_tmp).await;
    debug!("响应 get 请求 {}", &id);
    return re;
}

async fn request_ms_tts(data: Result<MsTtsMsgRequest, ControllerError>) -> HttpResponse {
    match data {
        Ok(r) => {
            let id = r.request_id.clone();
            // debug!("请求微软语音服务器");
            let kk = crate::GLOBAL_EB.request("tts_ms", r.into()).await;
            // debug!("请求微软语音完成");
            match kk {
                Some(data) => {
                    let data = MsTtsMsgResponse::from_vec(data.as_bytes().unwrap().to_vec());

                    let mut respone = HttpResponse::build(StatusCode::OK).body(data.data);
                    respone.headers_mut().insert(
                        actix_web::http::header::CONTENT_TYPE,
                        data.file_type.parse().unwrap(),
                    );
                    respone
                }
                None => {
                    let mut respone = HttpResponse::build(StatusCode::OK).body("未知错误");
                    respone.headers_mut().insert(
                        actix_web::http::header::CONTENT_TYPE,
                        "text".parse().unwrap(),
                    );
                    warn!("生成语音失败 {}", id);
                    respone
                }
            }
        }
        Err(e) => match e {
            ControllerError::TextNone(_t) => {
                let mut respone = HttpResponse::build(StatusCode::OK)
                    .body(crate::ms_tts::BLANK_MUSIC_FILE.to_vec());
                respone.headers_mut().insert(
                    actix_web::http::header::CONTENT_TYPE,
                    "audio/mpeg".parse().unwrap(),
                );
                respone
            }
        },
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiListResponse {
    list: Vec<ApiListItem>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiListItem {
    name: String,
    desc: String,
    url: String,
    params: Vec<ApiParam>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ParamType {
    String,
    Float,
    List,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiParam {
    index: usize,
    param_type: ParamType,
    param_name: String,
    param_desc: String,
    // Float
    #[serde(skip_serializing_if = "Option::is_none")]
    float_min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    float_max: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_value: Option<f32>,
    // List
    #[serde(skip_serializing_if = "Option::is_none")]
    list_data_url: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ListDataItem {
    key: String,
    desc: String,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    data: serde_json::Value,
}

#[derive(RustEmbed)]
#[folder = "web/dist/"]
struct WebAsset;

fn handle_embedded_file(path: &str) -> HttpResponse {
    // let index_html = WebAsset::get("prefix/index.html").unwrap();
    //RustEmbed::get()  Asset::get
    match WebAsset::get(path) {
        Some(content) => {
            let kk = content.data;
            let hh = kk.into_owned();
            HttpResponse::Ok()
                .content_type(from_path(path).first_or_octet_stream().as_ref())
                .body(hh)
        }
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

///
/// /api-list
/// 获取可用语音合成接口列表
///
async fn get_api_list(_req: HttpRequest) -> HttpResponse {
    let data = include_bytes!("resource/api-list.json");
    let api_list: Vec<ApiListItem> = serde_json::from_slice(data).unwrap();
    // let tmp = ApiListResponse { list: api_list };
    HttpResponse::Ok()
        .content_type("application/json")
        .body(ApiBaseResponse::success(api_list).to_string())
}

async fn index() -> HttpResponse {
    handle_embedded_file("index.html")
}

async fn dist(path: web::Path<String>) -> HttpResponse {
    let patd = path.into_inner();
    handle_embedded_file(&patd)
}

async fn favicon_ico() -> HttpResponse {
    handle_embedded_file("favicon.ico")
}

///
/// /ms-tts/informant
/// 获取微软文本转语音接口发音人列表
///
async fn get_ms_tts_informant(_req: HttpRequest) -> HttpResponse {
    let ms_tts_data = crate::ms_tts::MS_TTS_CONFIG.get();

    match ms_tts_data {
        Some(data) => {
            let mut list: Vec<ListDataItem> = Vec::new();
            data.voices_list.voices_name_list.iter().for_each(|v| {
                let voice_item = data.voices_list.by_voices_name_map.get(v).unwrap();
                let desc = format!(
                    "{} - {} - {}",
                    &voice_item.display_name, &voice_item.local_name, &voice_item.locale_name
                );
                let voices_item = VoicesItem {
                    name: voice_item.name.clone(),
                    display_name: voice_item.display_name.clone(),
                    local_name: voice_item.local_name.clone(),
                    short_name: voice_item.short_name.clone(),
                    gender: voice_item.gender.clone(),
                    locale: voice_item.locale.clone(),
                    locale_name: voice_item.locale_name.clone(),
                    style_list: None,
                    sample_rate_hertz: voice_item.sample_rate_hertz.clone(),
                    voice_type: voice_item.voice_type.clone(),
                    status: voice_item.status.clone(),
                    role_play_list: None,
                };

                let tmp = serde_json::to_value(voices_item).unwrap();

                list.push(ListDataItem {
                    key: v.clone(),
                    desc,
                    data: tmp,
                });
            });

            HttpResponse::Ok()
                .content_type("application/json")
                .body(ApiBaseResponse::success(list).to_string())
        }
        None => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(ApiBaseResponse::<()>::error("配置数据不存在").to_string()),
    }
}

///
/// /ms-tts/quality
/// 获取微软文本转语音接口音质列表
///
async fn get_ms_tts_quality(_req: HttpRequest) -> HttpResponse {
    let list_tmp = crate::ms_tts::MS_TTS_QUALITY_LIST;

    let mut list: Vec<ListDataItem> = Vec::new();

    list_tmp.iter().for_each(|v| {
        list.push(ListDataItem {
            key: v.to_string(),
            desc: "".to_owned(),
            data: serde_json::Value::Null,
        });
    });

    HttpResponse::Ok()
        .content_type("application/json")
        .body(ApiBaseResponse::success(list).to_string())
}

//
// #[get("/ms-tts/style/{informant}")]
async fn get_ms_tts_style(path_parme: web::Path<String>) -> HttpResponse {
    let informant = path_parme.into_inner();
    let ms_tts_data = crate::ms_tts::MS_TTS_CONFIG.get();

    match ms_tts_data {
        Some(data) => {
            let mut list_data: Vec<ListDataItem> = Vec::new();

            let voice_item = data.voices_list.by_voices_name_map.get(&informant).unwrap();
            let vec_style = if voice_item.style_list.is_some() {
                let mut ff = Vec::new();
                ff.push("general".to_string());
                let mut kk = voice_item
                    .style_list
                    .as_ref()
                    .unwrap()
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                ff.append(&mut kk);
                ff
            } else {
                let mut ff = Vec::new();
                ff.push("general".to_string());
                ff
            };

            vec_style.iter().for_each(|v| {
                list_data.push(ListDataItem {
                    key: v.clone(),
                    desc: "".to_owned(),
                    data: serde_json::Value::Null,
                });
            });

            HttpResponse::Ok()
                .content_type("application/json")
                .body(ApiBaseResponse::success(list_data).to_string())
        }
        None => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(ApiBaseResponse::<()>::error("配置数据不存在").to_string()),
    }
}
