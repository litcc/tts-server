use actix_web::{web, HttpRequest, HttpResponse};
use mime_guess::from_path;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::TTSServerError;
use crate::utils::azure_api::VoicesItem;
use crate::web::controller::ApiBaseResponse;
use rust_embed::RustEmbed;

///
/// 注册 web访问界面
pub(crate) fn register_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        // 获取 tts 支持列表
        web::resource("/api/list")
            .route(web::get().to(get_api_list))
            .route(web::post().to(get_api_list)),
    )
    .service(web::resource("/ms-tts/style/{informant}").route(web::get().to(get_ms_tts_style)))
    .route("/ms-tts/informant", web::get().to(get_ms_tts_informant))
    .route("/ms-tts/quality", web::get().to(get_ms_tts_quality))
    .service(web::resource("/").route(web::get().to(html_index)))
    .service(web::resource("/{_:.*}").route(web::get().to(dist)));
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
/// /api/list
/// 获取可用语音合成接口列表
///
pub(crate) async fn get_api_list(_req: HttpRequest) -> HttpResponse {
    let data = include_bytes!("../resource/api-list.json");
    let api_list: Vec<ApiListItem> = serde_json::from_slice(data).unwrap();
    // let tmp = ApiListResponse { list: api_list };
    HttpResponse::Ok()
        .content_type("application/json")
        .body(ApiBaseResponse::success(api_list).to_string())
}

async fn favicon_ico() -> HttpResponse {
    handle_embedded_file("favicon.ico")
}

pub(crate) async fn html_index() -> HttpResponse {
    handle_embedded_file("index.html")
}

pub(crate) async fn dist(path: web::Path<String>) -> HttpResponse {
    let patd = path.into_inner();
    handle_embedded_file(&patd)
}

///
/// /ms-tts/informant
/// 获取微软文本转语音接口发音人列表
///

pub(crate) async fn get_ms_tts_informant(_req: HttpRequest) -> HttpResponse {
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

pub(crate) async fn get_ms_tts_quality(_req: HttpRequest) -> HttpResponse {
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

pub(crate) async fn get_ms_tts_style(path_parme: web::Path<String>) -> HttpResponse {
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
