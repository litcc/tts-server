//#![feature(get_mut_unchecked)]
// #![windows_subsystem = "windows"]
#![feature(async_closure)]

use bytes::Bytes;


pub use log::*;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::{Arc};
use std::thread;
use std::time::Duration;

use crate::ms_tts::MsTtsMsgRequest;
use crate::utils::random_string;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{Receiver, Sender};

mod controller;
mod log_utils;
pub mod ms_tts;
#[cfg(test)]
mod tests;
pub mod utils;
//mod event_bus;

pub(crate) static CHANNEL: OnceCell<HashMap<String, MpscChannel>> =
    OnceCell::new();


struct MpscChannel {
    sender: Sender<Bytes>,
    receiver: Arc<tokio::sync::Mutex<Receiver<Bytes>>>,
}

pub(crate) static RUNTIME: OnceCell<Runtime> = OnceCell::new();


fn init() {
    CHANNEL.get_or_init(|| {
        let mut tts_map = HashMap::new();
        //crossbeam_channel::bounded(2000)
        tts_map.insert("value".to_string(), {
            let (tx, mut rx) = tokio::sync::mpsc::channel(2000);
            let receiver = Arc::new(tokio::sync::Mutex::new(rx));
            MpscChannel {
                sender: tx,
                receiver,
            }
        });
        tts_map.insert("control".to_string(), {
            let (tx, mut rx) = tokio::sync::mpsc::channel(2000);
            let receiver = Arc::new(tokio::sync::Mutex::new(rx));
            MpscChannel {
                sender: tx,
                receiver,
            }
        });
        tts_map
    });

    RUNTIME.get_or_init(|| {
        let cpus = num_cpus::get() / 2;
        let cpus = if cpus < 1 { 1 } else { cpus };
        Builder::new_multi_thread()
            .thread_name("event-bus-thread")
            .worker_threads(cpus)
            .enable_all()
            .build()
            .unwrap()
    });
}

#[tokio::main]
async fn main() {
    log_utils::init_log();
    // env_logger::init();
    info!("Hello, world!");
    init();

    ms_tts::register_service();
    controller::register_service();

    //crossbeam_channel::unbounded() // 无限制队列大小
    //crossbeam_channel::bounded(2000) // 指定队列大小
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
    //
    // CHANNEL.get().unwrap().get("value").unwrap().sender
    //     .clone().send(kkk.to_bytes()).await.unwrap();
    //
    // let request_id2 = random_string(32);
    // let kkk2 = MsTtsMsgRequest {
    //     text: "您请进".to_string(),
    //     request_id: request_id2,
    //     informant: "".to_string(),
    //     style: "".to_string(),
    //     rate: "".to_string(),
    //     pitch: "".to_string(),
    //     quality: "".to_string(),
    // };
    // // thread::sleep(Duration::from_secs(5));
    // CHANNEL.get().unwrap().get("value").unwrap().sender
    //     .clone().send(kkk2.to_bytes())
    //     .await.unwrap();
    //
    // let request_id3 = random_string(32);
    // // thread::sleep(Duration::from_secs(5));
    //
    // let kkk3 = MsTtsMsgRequest {
    //     text: "您请进2".to_string(),
    //     request_id: request_id3,
    //     informant: "".to_string(),
    //     style: "".to_string(),
    //     rate: "".to_string(),
    //     pitch: "".to_string(),
    //     quality: "".to_string(),
    // };
    //
    // CHANNEL.get().unwrap().get("value").unwrap().sender
    //     .clone().send(kkk3.to_bytes())
    //     .await.unwrap();
    //
    // thread::sleep(Duration::from_secs(20));
}
