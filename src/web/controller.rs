use std::fmt::{Debug};

use actix_web::{body::BoxBody, http::StatusCode, web, HttpRequest, HttpResponse};
use fancy_regex::Regex;
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use urlencoding::decode as url_decode;

use crate::{
    error::TTSServerError,
    info,
    ms_tts::MsTtsMsgResponse,
    random_string,
    utils::azure_api::{
        AzureApiEdgeFree, AzureApiSpeakerList, AzureApiSubscribeToken, MsApiOrigin,
        MsTtsMsgRequest, MS_TTS_QUALITY_LIST,
    },
    web::{
        entity::ApiBaseResponse, error::ControllerError, middleware::token_auth::AuthTokenValue,
    },
    AppArgs,
};

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
    /// 认证 Token
    pub token: Option<String>,
    // text_replace_list:Vec<String>,
    // phoneme_list:Vec<String>
}

impl MsTtsMsgRequestJson {
    pub async fn to_ms_request(
        &self,
        api_name: MsApiOrigin,
        request_id_value: String,
    ) -> Result<MsTtsMsgRequest, ControllerError> {
        let args = AppArgs::parse_macro();
        let text_value: String = {
            let mut text_tmp1 = self.text.as_str().to_string();
            // url 解码
            let text_tmp2: String = 'break_1: loop {
                let decoded = url_decode(&text_tmp1);
                if let Ok(s) = decoded {
                    if text_tmp1 == s {
                        break 'break_1 text_tmp1;
                    }
                    text_tmp1 = s.to_string();
                } else {
                    break 'break_1 text_tmp1;
                }
            };
            if text_tmp2.is_empty() {
                // 如果文字为空则返回1秒空音乐
                return Err(ControllerError::new("文本为空"));
            }

            // 转义符号
            let result = Regex::new(r"<")
                .unwrap()
                .replace_all(&text_tmp2, "&lt;")
                .to_string();
            let result = Regex::new(r">")
                .unwrap()
                .replace_all(&result, "&gt;")
                .to_string();

            let result = Regex::new(r"？")
                .unwrap()
                .replace_all(&result, "? ")
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

        let ms_informant_list = match api_name {
            MsApiOrigin::EdgeFree => {
                if !args.close_edge_free_api {
                    AzureApiEdgeFree::new().get_vices_list().await
                } else {
                    Err(TTSServerError::ProgramError(
                        "未开启 ms-tts-edge 接口，请勿调用".to_owned(),
                    ))
                }
            }
            MsApiOrigin::Subscription => {
                if !args.close_official_subscribe_api {
                    AzureApiSubscribeToken::get_vices_mixed_list().await
                } else {
                    Err(TTSServerError::ProgramError(
                        "未开启 ms-tts-subscribe 接口，请勿调用".to_owned(),
                    ))
                }
            }
        }
        .map_err(|e| {
            let err = ControllerError::new(format!("获取发音人数据错误 {:?}", e));
            error!("{:?}", err);
            err
        })?;

        // let ms_tts_config = &MS_TTS_CONFIG.get().unwrap();

        let informant_value: String = {
            let default = "zh-CN-XiaoxiaoNeural".to_owned();

            match &self.informant {
                Some(inf) => {
                    if ms_informant_list.voices_name_list.contains(inf) {
                        inf.to_string()
                    } else {
                        default
                    }
                }
                None => default,
            }
        }
        .trim()
        .to_owned();

        let informant_item = ms_informant_list
            .by_voices_name_map
            .get(&informant_value)
            .unwrap();

        let style_value: String = {
            let default = "general".to_owned();
            if let Some(style) = &self.style {
                match &informant_item.get_style() {
                    Some(e) => {
                        let s_t = style.to_lowercase();
                        if e.contains(&s_t) {
                            s_t
                        } else {
                            default
                        }
                    }
                    None => default,
                }
            } else {
                default
            }
        }
        .trim()
        .to_owned();

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
        }
        .trim()
        .to_owned();

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
        }
        .trim()
        .to_owned();

        let quality_value: String = {
            let default = "audio-24khz-48kbitrate-mono-mp3".to_owned();
            if let Some(quality) = &self.quality {
                if MS_TTS_QUALITY_LIST.contains(&quality.as_str()) {
                    quality.to_owned()
                } else {
                    default
                }
            } else {
                default
            }
        }
        .trim()
        .to_owned();

        Ok(MsTtsMsgRequest {
            text: text_value,
            request_id: request_id_value,
            informant: informant_value,
            style: style_value,
            rate: rate_value,
            pitch: pitch_value,
            quality: quality_value,
            subscribe_key: None,
            region: None,
        })
    }
}

impl AuthTokenValue for MsTtsMsgRequestJson {
    fn get_token(&self) -> Option<&str> {
        if self.token.is_some() {
            Some(self.token.as_ref().unwrap())
        } else {
            None
        }
    }
}

/// 监听
pub(crate) async fn tts_ms_post_controller(
    _req: HttpRequest,
    body: web::Json<MsTtsMsgRequestJson>,
) -> Result<HttpResponse, ControllerError> {
    let id = random_string(32);
    debug!("收到 post 请求{:?}", body);
    let request_tmp = body.to_ms_request(MsApiOrigin::EdgeFree, id.clone()).await;
    info!("解析 post 请求 {:?}", request_tmp);
    let re = request_ms_tts("tts_ms_edge_free", request_tmp).await;
    debug!("响应 post 请求 {}", &id);
    re
}

pub(crate) async fn tts_ms_get_controller(
    _req: HttpRequest,
    request: web::Query<MsTtsMsgRequestJson>,
) -> Result<HttpResponse, ControllerError> {
    let id = random_string(32);
    debug!("收到 get 请求{:?}", request);
    let request_tmp = request
        .to_ms_request(MsApiOrigin::EdgeFree, id.clone())
        .await;
    info!("解析 get 请求 {:?}", request_tmp);

    let re = request_ms_tts("tts_ms_edge_free", request_tmp).await;
    debug!("响应 get 请求 {}", &id);

    re
}

pub(crate) async fn tts_ms_subscribe_api_get_controller(
    _req: HttpRequest,
    request: web::Query<MsTtsMsgRequestJson>,
) -> Result<HttpResponse, ControllerError> {
    let id = random_string(32);
    debug!("收到 get 请求 /api/tts-ms-subscribe {:?}", request);
    let request_tmp = request
        .to_ms_request(MsApiOrigin::Subscription, id.clone())
        .await;
    info!("解析 get 请求 {:?}", request_tmp);
    let re = request_ms_tts("tts_ms_subscribe_api", request_tmp).await;
    debug!("响应 get 请求 {}", &id);
    re
}

pub(crate) async fn tts_ms_subscribe_api_post_controller(
    _req: HttpRequest,
    body: web::Json<MsTtsMsgRequestJson>,
) -> Result<HttpResponse, ControllerError> {
    let id = random_string(32);
    debug!("收到 post 请求 /api/tts-ms-subscribe {:?}", body);
    let request_tmp = body
        .to_ms_request(MsApiOrigin::Subscription, id.clone())
        .await;
    info!("解析 post 请求 /api/tts-ms-subscribe {:?}", request_tmp);
    let re = request_ms_tts("tts_ms_subscribe_api", request_tmp).await;
    debug!("响应 post 请求 {}", &id);
    re
}

async fn request_ms_tts(
    api_name: &str,
    data: Result<MsTtsMsgRequest, ControllerError>,
) -> Result<HttpResponse, ControllerError> {
    match data {
        Ok(rd) => {
            let id = rd.request_id.clone();
            // debug!("请求微软语音服务器");
            let kk = crate::GLOBAL_EB.request(api_name, rd.into()).await;
            // debug!("请求微软语音完成");
            match kk {
                Some(data) => {
                    let data = MsTtsMsgResponse::from_vec(data.as_bytes().unwrap().to_vec());

                    let mut respone = HttpResponse::build(StatusCode::OK).body(data.data);
                    respone.headers_mut().insert(
                        actix_web::http::header::CONTENT_TYPE,
                        data.file_type.parse().unwrap(),
                    );
                    Ok(respone)
                }
                None => {
                    warn!("生成语音失败 {}", id);

                    let ll: HttpResponse<BoxBody> = ApiBaseResponse::<()>::error("未知错误").into();
                    Ok(ll)
                }
            }
        }
        Err(e) => {
            if e.msg == "文本为空" {
                warn!("请求文本为空");
                let mut respone = HttpResponse::build(StatusCode::OK)
                    .body(crate::ms_tts::BLANK_MUSIC_FILE.to_vec());
                respone.headers_mut().insert(
                    actix_web::http::header::CONTENT_TYPE,
                    "audio/mpeg".parse().unwrap(),
                );
                Ok(respone)
            } else {
                error!("调用错误：{:?}", e);
                Err(e)
            }
        }
    }
}
