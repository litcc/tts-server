use crate::utils::azure_api::{
    AzureApiEdgeFree, AzureApiPreviewFreeToken, AzureApiSpeakerList, AzureApiSubscribeToken,
    VoicesItem, MS_TTS_QUALITY_LIST,
};
use crate::web::entity::ApiBaseResponse;
use crate::web::error::ControllerError;
use crate::AppArgs;
use actix_web::{web, HttpRequest, HttpResponse};
use log::{error, warn};
use mime_guess::from_path;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

///
/// 注册 web访问界面
pub(crate) fn register_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        // 获取 tts 支持列表
        web::resource("/api/list")
            .route(web::get().to(get_api_list))
            .route(web::post().to(get_api_list)),
    )
        .service(web::resource("/api/ms-tts/style/{informant}").route(web::get().to(get_ms_tts_style)))
        .route(
            "/api/ms-tts/informant/{api_name}",
            web::get().to(get_ms_tts_informant),
        )
        .route("/api/ms-tts/quality", web::get().to(get_ms_tts_quality))
        .service(web::resource("/").route(web::get().to(html_index)));
        // .service(web::resource("/{_:.*}").route(web::get().to(dist)));
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiListResponse {
    list: Vec<ApiListItem>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiListItem {
    api_name: String,
    api_desc: String,
    api_url: String,
    params: Vec<ApiParam>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "param_type")]
pub enum ApiParam {
    Text {
        index: usize,
        param_name: String,
        param_desc: String,
        max_len: Option<u64>,
        min_len: Option<u64>,
    },
    Float {
        index: usize,
        param_name: String,
        param_desc: String,
        // #[serde(skip_serializing_if = "Option::is_none")]
        float_min: Option<f32>,
        // #[serde(skip_serializing_if = "Option::is_none")]
        float_max: Option<f32>,
        // #[serde(skip_serializing_if = "Option::is_none")]
        default_value: Option<f32>,
    },
    List {
        index: usize,
        param_name: String,
        param_desc: String,
        list_data_url: String,
    },
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
pub(crate) async fn get_api_list(_req: HttpRequest) -> Result<HttpResponse, ControllerError> {
    let mut api_list: Vec<ApiListItem> = Vec::new();
    let args = AppArgs::parse_macro();
    if !args.close_edge_free_api {
        let data = include_bytes!("../resource/api/ms-api-edge.json");
        api_list.push(serde_json::from_slice(data).unwrap())
    }
    if !args.close_official_preview_api {
        let data = include_bytes!("../resource/api/ms-api-preview.json");
        api_list.push(serde_json::from_slice(data).unwrap())
    }
    if !args.close_official_subscribe_api {
        let data = include_bytes!("../resource/api/ms-api-subscribe.json");
        api_list.push(serde_json::from_slice(data).unwrap())
    }
    Ok(ApiBaseResponse::success(Some(api_list)).into())
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
/// /api/ms-tts/informant/{api_name}
/// 获取微软文本转语音接口发音人列表
///

pub(crate) async fn get_ms_tts_informant(
    _req: HttpRequest,
    path_params: web::Path<String>,
) -> Result<HttpResponse, ControllerError> {
    let api_name = path_params.into_inner();
    let args = AppArgs::parse_macro();
    let ms_tts_data = crate::ms_tts::MS_TTS_CONFIG.get();

    let mut list: Vec<ListDataItem> = Vec::new();

    let vices_list = match api_name.as_str() {
        "ms-tts-edge" => {
            if args.close_edge_free_api {
                None
            } else {
                Some(AzureApiEdgeFree::new()
                    .get_vices_list()
                    .await
                    .map_err(|e| {
                        let err =
                            ControllerError::new(format!("获取 ms-tts-edge 接口发音人列表失败 {:?}", e));
                        error!("{:?}", err);
                        err
                    })?)
            }
        }
        "ms-tts-preview" => {
            if args.close_official_preview_api {
                None
            } else {
                Some(AzureApiPreviewFreeToken::new()
                    .get_vices_list()
                    .await
                    .map_err(|e| {
                        let err =
                            ControllerError::new(format!("获取 ms-tts-preview 接口发音人列表失败 {:?}", e));
                        error!("{:?}", err);
                        err
                    })?)
            }
        }
        "ms-tts-subscribe" => {
            if args.close_official_subscribe_api {
                None
            } else {
                Some(AzureApiSubscribeToken::get_vices_mixed_list()
                    .await
                    .map_err(|e| {
                        let err = ControllerError::new(format!(
                            "获取 ms-tts-subscribe 接口发音人列表失败 {:?}",
                            e
                        ));
                        error!("{:?}", err);
                        err
                    })?)
            }
        }
        _ => {
            None
        }
    };
    if let None = vices_list {
        let err = ControllerError::new("配置数据不存在");
        error!("{:?}", err);
        return Err(err);
    }
    let vices_list = vices_list.unwrap();
    vices_list.voices_name_list.iter().for_each(|v| {
        let voice_item = vices_list.by_voices_name_map.get(v).unwrap();
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

    Ok(ApiBaseResponse::success(Some(list)).into())
}

///
/// /ms-tts/quality
/// 获取微软文本转语音接口音质列表

pub(crate) async fn get_ms_tts_quality(_req: HttpRequest) -> Result<HttpResponse, ControllerError> {
    let list_tmp = MS_TTS_QUALITY_LIST;

    let mut list: Vec<ListDataItem> = Vec::new();

    list_tmp.iter().for_each(|v| {
        list.push(ListDataItem {
            key: v.to_string(),
            desc: "".to_owned(),
            data: serde_json::Value::Null,
        });
    });

    Ok(ApiBaseResponse::success(Some(list)).into())
}

//
// #[get("/ms-tts/style/{informant}")]

pub(crate) async fn get_ms_tts_style(
    path_params: web::Path<String>,
) -> Result<HttpResponse, ControllerError> {
    let informant = path_params.into_inner();
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
            Ok(ApiBaseResponse::success(Some(list_data)).into())
        }
        None => Err(ControllerError::new("配置数据不存在")),
    }
}
