//#![feature(get_mut_unchecked)]
// #![windows_subsystem = "windows"]
#![feature(async_closure)]
#![feature(get_mut_unchecked)]

extern crate core;

use crate::ms_tts::MsTtsMsgRequest;
use crate::utils::random_string;
use clap::{ArgEnum, Parser};
use event_bus::core::{EventBus, EventBusOptions};
use event_bus::message::{Body, VertxMessage};
pub use log::*;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

mod controller;
mod log_utils;
pub mod ms_tts;
#[cfg(test)]
mod tests;
pub mod utils;

#[derive(Parser, Debug)]
#[clap(author, version)]
#[clap(name = "tts-server")]
#[clap(author = "litcc")]
#[clap(about = None, long_about = "TTS Api Server 软件仅供学习交流，严禁用于商业用途，请于24小时内删除！")]
// #[clap(help = "TTS api server")]
pub(crate) struct AppArgs {
    /// 指定连接渠道
    // #[clap(long, value_name = "area", default_value_t = String::from("us"))]
    #[clap(long, arg_enum, value_name = "area", default_value_t = ServerArea::Default)]
    server_area: ServerArea,

    /// 监听地址
    #[clap(long, value_name = "address", default_value_t = String::from("0.0.0.0"))]
    listen_address: String,

    /// 监听端口
    #[clap(long, value_name = "prot", default_value_t = String::from("8080"))]
    listen_port: String,
}

#[derive(Debug,Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum ServerArea {
    Default,
    China,
    ChinaHK,
    ChinaTW,
}


pub(crate) static GLOBAL_EB: Lazy<Arc<EventBus<VertxMessage>>> = Lazy::new(|| {
    let eb = EventBus::<VertxMessage>::new(Default::default());
    Arc::new(eb)
});

#[tokio::main]
async fn main() {

    log_utils::init_log();
    // info!("软件仅供学习交流，严禁用于商业用途，请于24小时内删除！");
    let args: AppArgs = AppArgs::parse();
    println!("Hello {:?}!", args);
    GLOBAL_EB.start().await;
    ms_tts::register_service().await;
    controller::register_service(args.listen_address, args.listen_port);
    //
    //
    // let request_id = random_string(32);
    // let kkk = MsTtsMsgRequest {
    //     text: "你好啊".to_string(),
    //     request_id: request_id,
    //     informant: "".to_string(),
    //     style: "".to_string(),
    //     rate: "".to_string(),
    //     pitch: "".to_string(),
    //     quality: "".to_string(),
    // };
    // debug!("发送请求");
    // //GLOBAL_EB.send("ms_tts", Body::ByteArray(kkk.to_bytes().to_vec())).await;
    // GLOBAL_EB.send("ms_tts", kkk.into());
    //
    //
    // tokio::time::sleep(Duration::from_secs(120)).await;
}
