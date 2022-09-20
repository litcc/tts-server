pub(crate) mod controller;
// #[cfg(feature = "web-entrance")]
pub(crate) mod web_entrance;
pub(crate) mod error;
mod entity;

use crate::web::controller::*;
use actix_web::middleware::Compress;
use actix_web::{web, App, HttpServer};
use log::{error, info};

// #[cfg(feature = "web-entrance")]
use crate::web::web_entrance::register_router;

///
/// 注册 web 服务
///
///
pub(crate) async fn register_service(address: String, port: String) {
    let web_server = HttpServer::new(|| {
        let mut app = App::new();
        let mut app = app.wrap(Compress::default());

        // 微软 TTS 文本转语音 相关接口
        app = app.service(
            // 新版本网页接口地址 （使用api收费访问）
            web::resource("/api/tts-ms-subscribe-api")
                .route(web::get().to(tts_ms_subscribe_api_get_controller))
                .route(web::post().to(tts_ms_subscribe_api_post_controller)),

        ).service(
            // 新版本网页接口地址 （免费预览）
            web::resource("/api/tts-ms-official-preview")
                .route(web::get().to(tts_ms_official_preview_get_controller))
                .route(web::post().to(tts_ms_official_preview_post_controller)),
        ).service(
            // 旧版本 edge 预览接口
            web::resource("/api/tts-ms-edge")
                .route(web::get().to(tts_ms_get_controller))
                .route(web::post().to(tts_ms_post_controller)),
        );

        // 根据功能
        // #[cfg(feature = "web-entrance")]
        // {
            app = app.configure(register_router);
        // }

        app
    });
    let web_server = web_server.bind(format!("{}:{}", address, port));
    match web_server {
        Ok(server) => {
            let local_ip = local_ipaddress::get();
            info!(
                "启动 Api 服务成功 接口地址已监听至: {}:{}  自行修改 ip 以及 port",
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
