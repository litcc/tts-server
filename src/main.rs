//#![feature(get_mut_unchecked)]
// #![windows_subsystem = "windows"]
#![feature(async_closure)]
#![feature(get_mut_unchecked)]

extern crate core;

use std::sync::Arc;
pub use log::*;
use once_cell::sync::{Lazy};

use std::time::Duration;
use event_bus::core::{EventBus, EventBusOptions};
use event_bus::message::{Body, VertxMessage};




use crate::ms_tts::MsTtsMsgRequest;
use crate::utils::random_string;

mod controller;
mod log_utils;
pub mod ms_tts;
#[cfg(test)]
mod tests;
pub mod utils;
//mod event_bus;


pub(crate) static GLOBAL_EB: Lazy<Arc<EventBus<VertxMessage>>> = Lazy::new(|| {
    let eb = EventBus::<VertxMessage>::new(Default::default());
    Arc::new(eb)
});


#[tokio::main]
async fn main() {
    log_utils::init_log();
    // env_logger::init();
    info!("Hello, world!");
    GLOBAL_EB.start().await;
    ms_tts::register_service().await;
    controller::register_service();


    let request_id = random_string(32);
    let kkk = MsTtsMsgRequest {
        text: "你好啊".to_string(),
        request_id: request_id,
        informant: "".to_string(),
        style: "".to_string(),
        rate: "".to_string(),
        pitch: "".to_string(),
        quality: "".to_string(),
    };
    debug!("发送请求");
    //GLOBAL_EB.send("ms_tts", Body::ByteArray(kkk.to_bytes().to_vec())).await;
    GLOBAL_EB.send("ms_tts", kkk.into());




    tokio::time::sleep(Duration::from_secs(120)).await;
}
