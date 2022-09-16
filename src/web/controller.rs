use crate::{info, random_string};
use actix_web::http::{StatusCode};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use fancy_regex::Regex;
use log::{debug, warn};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use crate::ms_tts::{MsTtsMsgResponse, MS_TTS_CONFIG};
use urlencoding::decode as url_decode;
use crate::utils::azure_api::MsTtsMsgRequest;


// ##### Error Struct ############################################################################

#[derive(Debug)]
pub enum ControllerError {
    TextNone(String),
}

impl Display for ControllerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Error: {:?})", self)
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
            let result = Regex::new(r"！")
                .unwrap()
                .replace_all(&result, "! ")
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
        }.trim().to_owned();

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
        }.trim().to_owned();

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
        }.trim().to_owned();

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
        }.trim().to_owned();
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
        }.trim().to_owned();

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


pub(crate) async fn tts_ms_post_controller(
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

pub(crate) async fn tts_ms_get_controller(
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
            let kk = crate::GLOBAL_EB.request("tts_ms_edge_free", r.into()).await;
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





