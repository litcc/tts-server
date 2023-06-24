use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::error;
// use mime_guess::from_path;
// use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::{
    utils::azure_api::{
        AzureApiEdgeFree, AzureApiSpeakerList, AzureApiSubscribeToken, MsApiOrigin,
        MS_TTS_QUALITY_LIST,
    },
    web::{entity::ApiBaseResponse, error::ControllerError, vo::BaseResponse},
    AppArgs,
};

///
/// 注册 web访问界面
pub(crate) fn register_router(cfg: &mut web::ServiceConfig) {
    cfg.service(
        // 获取 tts 支持列表
        web::resource("/api/list")
            .route(web::get().to(get_api_list))
            .route(web::post().to(get_api_list)),
    )
    .service(
        web::resource("/api/ms-tts/style/{api_name}/{informant}")
            .route(web::get().to(get_ms_tts_style)),
    )
    .route(
        "/api/ms-tts/informant/{api_name}",
        web::get().to(get_ms_tts_informant),
    )
    .route("/api/ms-tts/quality", web::get().to(get_ms_tts_quality));
    // 等待web UI 适配
    // .service(web::resource("/").route(web::get().to(html_index)))
    // .service(web::resource("/{_:.*}").route(web::get().to(dist)));
}

// 等待WebUi适配
/*#[derive(RustEmbed)]
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

async fn favicon_ico() -> HttpResponse {
    handle_embedded_file("favicon.ico")
}

pub(crate) async fn html_index() -> HttpResponse {
    handle_embedded_file("index.html")
}

pub(crate) async fn dist(path: web::Path<String>) -> HttpResponse {
    let patd = path.into_inner();
    handle_embedded_file(&patd)
}*/

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiListResponse {
    list: Vec<ApiListItem>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ApiListItem {
    pub api_id: String,
    pub api_name: String,
    pub api_desc: String,
    pub api_url: String,
    pub params: Vec<ApiParam>,
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
    pub key: String,
    pub desc: String,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub data: serde_json::Value,
}

///
/// /api/list
/// 获取可用语音合成接口列表
///
pub(crate) async fn get_api_list(_req: HttpRequest) -> Result<impl Responder, ControllerError> {
    let mut api_list: Vec<ApiListItem> = Vec::new();
    let args = AppArgs::parse_macro();
    if !args.close_edge_free_api {
        let data = include_bytes!("../resource/api/ms-api-edge.json");
        api_list.push(serde_json::from_slice(data).unwrap())
    }
    if !args.close_official_subscribe_api {
        let data = include_bytes!("../resource/api/ms-api-subscribe.json");
        api_list.push(serde_json::from_slice(data).unwrap())
    }
    Ok(BaseResponse::from(api_list))
}

///
/// /api/ms-tts/informant/{api_name}
/// 获取微软文本转语音接口发音人列表
///
pub(crate) async fn get_ms_tts_informant(
    path_params: web::Path<String>,
) -> Result<HttpResponse, ControllerError> {
    let api_name = MsApiOrigin::try_from(path_params.into_inner()).map_err(|e| {
        let err = format!("接口配置数据不存在 {:?}", e);
        error!("{}", err);
        ControllerError::new(err)
    })?;
    let args = AppArgs::parse_macro();
    let mut list: Vec<ListDataItem> = Vec::new();

    let vices_list = match api_name {
        MsApiOrigin::EdgeFree => {
            if args.close_edge_free_api {
                None
            } else {
                Some(
                    AzureApiEdgeFree::new()
                        .get_vices_list()
                        .await
                        .map_err(|e| {
                            let err = ControllerError::new(format!(
                                "获取 ms-tts-edge 接口发音人列表失败 {:?}",
                                e
                            ));
                            error!("{:?}", err);
                            err
                        })?,
                )
            }
        }
        MsApiOrigin::Subscription => {
            if args.close_official_subscribe_api {
                None
            } else {
                Some(
                    AzureApiSubscribeToken::get_vices_mixed_list()
                        .await
                        .map_err(|e| {
                            let err = ControllerError::new(format!(
                                "获取 ms-tts-subscribe 接口发音人列表失败 {:?}",
                                e
                            ));
                            error!("{:?}", err);
                            err
                        })?,
                )
            }
        }
    };
    if vices_list.is_none() {
        let err = ControllerError::new("配置数据不存在");
        error!("{:?}", err);
        return Err(err);
    }
    let vices_list = vices_list.unwrap();
    vices_list.voices_name_list.iter().for_each(|v| {
        let voice_item = vices_list.by_voices_name_map.get(v).unwrap();

        let desc = voice_item.get_desc();
        let tmp = serde_json::to_value(voice_item.as_ref()).unwrap();

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
#[derive(Deserialize, Debug)]
pub(crate) struct PathParams {
    pub api_name: String,
    pub informant: String,
}

pub(crate) async fn get_ms_tts_style(
    path_params: web::Path<PathParams>,
) -> Result<HttpResponse, ControllerError> {
    let params = path_params.into_inner();
    let informant = params.informant;
    let api_name = MsApiOrigin::try_from(params.api_name).map_err(|e| {
        let err = format!("接口配置数据不存在 {:?}", e);
        error!("{}", err);
        ControllerError::new(err)
    })?;
    let args = AppArgs::parse_macro();

    let vices_list = match api_name {
        MsApiOrigin::EdgeFree => {
            if args.close_edge_free_api {
                None
            } else {
                Some(
                    AzureApiEdgeFree::new()
                        .get_vices_list()
                        .await
                        .map_err(|e| {
                            let err = ControllerError::new(format!(
                                "获取 ms-tts-edge 接口发音人列表失败 {:?}",
                                e
                            ));
                            error!("{:?}", err);
                            err
                        })?,
                )
            }
        }
        MsApiOrigin::Subscription => {
            if args.close_official_subscribe_api {
                None
            } else {
                Some(
                    AzureApiSubscribeToken::get_vices_mixed_list()
                        .await
                        .map_err(|e| {
                            let err = ControllerError::new(format!(
                                "获取 ms-tts-subscribe 接口发音人列表失败 {:?}",
                                e
                            ));
                            error!("{:?}", err);
                            err
                        })?,
                )
            }
        }
    };

    if vices_list.is_none() {
        let err = ControllerError::new("配置数据不存在，请查看是否参数正确，或是否开启该接口");
        error!("{:?}", err);
        return Err(err);
    }
    let vices_list = vices_list.unwrap();

    let mut list_data: Vec<ListDataItem> = Vec::new();
    let voice_item = vices_list.by_voices_name_map.get(&informant).unwrap();
    let vec_style = if voice_item.get_style().is_some() {
        let mut ff = vec!["general".to_owned()];
        let mut kk = voice_item
            .get_style()
            .as_ref()
            .unwrap().to_vec();
        ff.append(&mut kk);
        ff
    } else {
        let ff = vec!["general".to_owned()];
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
