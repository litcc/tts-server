pub mod ms_tts;

pub(crate) mod cmd;
pub(crate) mod error;
pub(crate) mod utils;
pub(crate) mod web;

#[cfg(test)]
pub(crate) mod tests;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use anyhow::Result;
use event_bus::{core::EventBus, message::VertxMessage};
pub use log::*;
use once_cell::sync::{Lazy, OnceCell};
use tokio::runtime::Runtime;
use utils::log::init_log;

use crate::{
    cmd::AppArgs,
    utils::{azure_api::MS_TTS_QUALITY_LIST, random_string},
};

pub(crate) static GLOBAL_EB: Lazy<Arc<EventBus<VertxMessage>>> = Lazy::new(|| {
    let eb = EventBus::<VertxMessage>::new(Default::default());
    Arc::new(eb)
});

// #[tokio::main]
// async
async fn main_async() -> Result<()> {
    let args = AppArgs::parse_macro();
    debug!("程序参数: {:#?}", args);
    if args.show_quality_list {
        println!(
            "当前可使用的音频参数有: (注意：Edge免费接口可能个别音频参数无法使用，是正常情况，是因为微软不允许滥用！) \n{:?}",
            MS_TTS_QUALITY_LIST
        );
        std::process::exit(0);
    }
    if args.show_informant_list {
        info!(
            "由于提供多种接口，且多种接口发音人可能会有不相同的支持，所以建议通过官方手段获取发音人列表！这里就不做展示了!",
        );
        std::process::exit(0);
    }
    //
    info!("准备启动，程序参数: {:?}", args);
    GLOBAL_EB.start().await;
    ms_tts::register_service().await;
    web::register_service().await;
    info!("谢谢使用，希望能收到您对软件的看法和建议！");
    Ok(())
}

static GLOBAL_RT: OnceCell<Runtime> = OnceCell::new();

fn main() -> Result<()> {
    let args = AppArgs::parse_macro();
    init_log(
        args.log_level,
        Some(args.log_to_file),
        Some(&args.log_path),
        None,
    );

    GLOBAL_RT
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name_fn(|| {
                    static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                    let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                    format!("tts-server-global-{}", id)
                })
                .thread_stack_size(3 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
        })
        .block_on(main_async())
}
