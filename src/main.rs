
pub mod ms_tts;

pub(crate) mod error;
pub(crate) mod utils;
pub(crate) mod web;
pub(crate) mod cmd;

#[cfg(test)]
pub(crate) mod tests;



use crate::utils::random_string;
use event_bus::core::EventBus;
use event_bus::message::VertxMessage;
pub use log::*;
use once_cell::sync::{Lazy, OnceCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use utils::log::init_log;
use crate::cmd::AppArgs;
use anyhow::Result;
use tokio::runtime::Runtime;

pub(crate) static GLOBAL_EB: Lazy<Arc<EventBus<VertxMessage>>> = Lazy::new(|| {
    let eb = EventBus::<VertxMessage>::new(Default::default());
    Arc::new(eb)
});

// #[tokio::main]
// async
async fn main_async() -> Result<()> {
    let args = AppArgs::parse_macro();
    debug!("程序参数: {:#?}",args);
    /*if args.show_quality_list {
        println!(
            "当前可使用的音频参数有: (注意：Edge免费接口可能个别音频参数无法使用，是正常情况，是因为微软不允许滥用！) \n{:?}",
            ms_tts::MS_TTS_QUALITY_LIST
        );
        std::process::exit(0);
    }
    if args.show_informant_list {
        ms_tts::MS_TTS_CONFIG
            .get_or_init(|| async { ms_tts::get_ms_tts_config().await.unwrap() })
            .await;
        println!(
            "当前可使用的发音人参数有: (注意：Edge免费接口可能个别发音人参数无法使用，是正常情况，是因为微软不允许滥用！) \n{:?}",
            ms_tts::MS_TTS_CONFIG
                .get()
                .unwrap()
                .voices_list
                .voices_name_list
        );
        std::process::exit(0);
    }*/
    //
    info!("准备启动，程序参数: {:?}", args);
    GLOBAL_EB.start().await;
    ms_tts::register_service().await;
    web::register_service(args.listen_address.clone(), args.listen_port.clone()).await;
    info!("谢谢使用，希望能收到您对软件的看法和建议！");

    Ok(())
}



static GLOBAL_RT: OnceCell<Runtime> = OnceCell::new();

fn main() -> Result<()> {
    init_log(LevelFilter::Debug,None,None);

    GLOBAL_RT
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name_fn(|| {
                    static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                    let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                    format!("aya-backend-global-{}", id)
                })
                .thread_stack_size(3 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
        })
        .block_on(main_async())
}