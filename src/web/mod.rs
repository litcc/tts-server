pub(crate) mod controller;
// #[cfg(feature = "web-entrance")]
mod entity;
pub(crate) mod error;
pub(crate) mod utils;
pub(crate) mod web_entrance;

pub(crate) mod middleware;
mod vo;

use actix_web::{
    middleware::{Compress, Condition},
    web, App, HttpServer,
};
use log::{error, info};

// #[cfg(feature = "web-entrance")]
use crate::web::web_entrance::register_router;
use crate::{
    web::{
        controller::*,
        middleware::{error_handle::ErrorHandle, token_auth::TokenAuthentication},
    },
    AppArgs,
};

///
/// 注册 web 服务
///
///
pub(crate) async fn register_service() {
    let args = AppArgs::parse_macro();
    let web_server = HttpServer::new(|| {
        let app = App::new();
        let mut app = app.wrap(ErrorHandle).wrap(Compress::default());

        // 微软 TTS 文本转语音 相关接口

        // if !args.close_official_subscribe_api {
        app = app.service(
            // 新版本网页接口地址 （使用api收费访问）
            web::resource("/api/tts-ms-subscribe")
                .wrap(Condition::new(
                    args.subscribe_api_auth_token.is_some(),
                    TokenAuthentication::<MsTtsMsgRequestJson>::default(),
                ))
                .route(web::get().to(tts_ms_subscribe_api_get_controller))
                .route(web::post().to(tts_ms_subscribe_api_post_controller)),
        );
        // }

        // if !args.close_edge_free_api {
        app = app.service(
            // 旧版本 edge 预览接口
            web::resource("/api/tts-ms-edge")
                .route(web::get().to(tts_ms_get_controller))
                .route(web::post().to(tts_ms_post_controller)),
        );
        // }

        // 根据功能
        // #[cfg(feature = "web-entrance")]
        // {

        app = app.configure(register_router);

        // }
        app
    });
    let web_server = web_server.bind(format!("{}:{}", args.listen_address, args.listen_port));
    match web_server {
        Ok(server) => {
            let local_ip = local_ipaddress::get();
            info!(
                "启动 Api 服务成功 接口地址已监听至: {}:{}  自行修改 ip 以及 port",
                args.listen_address, args.listen_port
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
            error!(
                "启动 Api 服务失败，无法监听 {}:{}",
                args.listen_address, args.listen_port
            );
        }
    }
}
