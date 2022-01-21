use crate::info;
use actix_web::{get, web, App, HttpServer, Responder};

/// 监听
#[actix_web::main]
pub(crate) async fn handle() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index))
        .bind("127.0.0.1:8080")
        .expect("监听http地址错误")
        .workers(1)
        .run()
        .await
        .expect("启动后端服务错误");

    Ok(())
}

#[get("/")]
async fn index() -> impl Responder {
    info!("收到http请求");
    // let event_bus = Arc::clone(&VERTX).event_bus();
    // event_bus.request("tts_service::use", Body::String(String::from("111")),move |m, _| {
    //     info!("tts_service::use 回调完成");
    //
    // });

    return format!("Hello  id:");
}

#[get("/{id}/{name}/index.html")]
async fn index1(web::Path((id, name)): web::Path<(u32, String)>) -> impl Responder {
    return format!("Hello {}! id:{}", name, id);
}
