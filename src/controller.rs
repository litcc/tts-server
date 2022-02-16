use crate::{info, random_string, MsTtsMsgRequest};
use actix_web::http::StatusCode;
use actix_web::web::{get, post};
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use fancy_regex::Regex;
use log::debug;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::ms_tts::{MsTtsConfig, MS_TTS_CONFIG};
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
    pub fn to_ms_request(&self, request_id_value: String) -> Result<MsTtsMsgRequest, ControllerError> {
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
                .replace_all(&text_tmp2, "?")
                .to_string();
            let result = Regex::new(r"，")
                .unwrap()
                .replace_all(&result, ",")
                .to_string();
            let result = Regex::new(r"。")
                .unwrap()
                .replace_all(&result, ".")
                .to_string();
            let result = Regex::new(r"：")
                .unwrap()
                .replace_all(&result, ":")
                .to_string();
            let result = Regex::new(r"；")
                .unwrap()
                .replace_all(&result, ";")
                .to_string();
            // let result = Regex::new(r"，").unwrap().replace_all(&result,",").to_string();
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
                        if e.contains(style) {
                            style.to_owned()
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
#[actix_web::main]
pub(crate) async fn register_service() {
    HttpServer::new(|| {
        App::new()
            .service(
                web::resource("/tts-ms")
                    // .name("user_detail")
                    // .guard(guard::Header("content-type", "application/json"))
                    .route(web::get().to(tts_ms_get_controller))
                    .route(web::post().to(tts_ms_post_controller)),
            )
        // .route("/tts-ms", post().to(tts_ms_post_controller))
        // .route("/tts-ms", get().to(tts_ms_get_controller))
    })
        .bind("0.0.0.0:8080")
        .expect("监听http地址错误")
        .workers(1)
        .run()
        .await
        .unwrap();
}

async fn tts_ms_post_controller(
    _req: HttpRequest,
    body: web::Json<MsTtsMsgRequestJson>,
) -> HttpResponse {
    let id = random_string(32);
    // let (tx, mut rx) = tokio::sync::oneshot::channel();
    let request_tmp = body.to_ms_request(id.clone());
    info!("收到post请求 {:?}", request_tmp);
    let re = match request_tmp {
        Ok(r) => {
            debug!("请求订阅项");
            let kk = crate::GLOBAL_EB.request("tts_ms", r.into()).await;
            debug!("响应订阅项");
            match kk {
                Some(data) => {
                    let mut respone =
                        HttpResponse::build(StatusCode::OK).body(data.as_bytes().unwrap().to_vec());
                    respone.headers_mut().insert(
                        actix_web::http::header::CONTENT_TYPE,
                        "audio/*".parse().unwrap(),
                    );

                    respone
                }
                None => {
                    let mut respone = HttpResponse::build(StatusCode::OK).body("未知错误");
                    respone.headers_mut().insert(
                        actix_web::http::header::CONTENT_TYPE,
                        "text".parse().unwrap(),
                    );
                    respone
                }
            }
        }
        Err(e) => {
            let mut respone = HttpResponse::build(StatusCode::OK).body("未知错误");
            respone.headers_mut().insert(
                actix_web::http::header::CONTENT_TYPE,
                "text".parse().unwrap(),
            );
            respone
        }
    };
    debug!("响应 post 请求 {}", &id);
    return re;
}

async fn tts_ms_get_controller(
    _req: HttpRequest,
    request: web::Query<MsTtsMsgRequestJson>,
) -> HttpResponse {

    let test_id = random_string(5);
    info!("controller {} -1",test_id);
    let id = random_string(32);
    // let (tx, mut rx) = tokio::sync::oneshot::channel();
    let request_tmp = request.to_ms_request(id.clone());
    info!("controller {} -2",test_id);
    info!("收到 get 请求 {:?}", request_tmp);
    let re = match request_tmp {
        Ok(r) => {
            debug!("请求微软语音服务器");
            info!("controller {} -3",test_id);
            let kk = crate::GLOBAL_EB.request("tts_ms", r.into()).await;
            info!("controller {} -4",test_id);
            debug!("请求微软语音完成");
            match kk {
                Some(data) => {
                    info!("controller {} -5",test_id);
                    let mut respone =
                        HttpResponse::build(StatusCode::OK).body(data.as_bytes().unwrap().to_vec());
                    respone.headers_mut().insert(
                        actix_web::http::header::CONTENT_TYPE,
                        "audio/*".parse().unwrap(),
                    );
                    info!("controller {} -6",test_id);
                    respone
                }
                None => {
                    let mut respone = HttpResponse::build(StatusCode::OK).body("未知错误");
                    respone.headers_mut().insert(
                        actix_web::http::header::CONTENT_TYPE,
                        "text".parse().unwrap(),
                    );
                    respone
                }
            }
        }
        Err(e) => {
            let mut respone = HttpResponse::build(StatusCode::OK).body("未知错误");
            respone.headers_mut().insert(
                actix_web::http::header::CONTENT_TYPE,
                "text".parse().unwrap(),
            );
            respone
        }
    };
    info!("controller {} -7",test_id);
    debug!("响应 get 请求 {}", &id);
    return re;
}

#[get("/{id}/{name}/index.html")]
async fn index1(web::Path((id, name)): web::Path<(u32, String)>) -> impl Responder {
    return format!("Hello {}! id:{}", name, id);
}
