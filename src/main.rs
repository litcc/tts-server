#![feature(get_mut_unchecked)]

extern crate core;

use crate::ms_tts::MsTtsMsgRequest;
use crate::utils::random_string;
use clap::{ArgEnum, Parser};
use event_bus::core::{EventBus};
use event_bus::message::{VertxMessage};
pub use log::*;
use once_cell::sync::Lazy;
use std::sync::Arc;

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
#[clap(about = r##"TTS Api Server 软件仅供学习交流，严禁用于商业用途，请于24小时内删除！
    目前已实现接口有：[微软文本转语音] 后续看情况可能会再加入其他接口。

    微软文本转语音接口： /tts-ms
        接口支持 get,post 请求, get请求时参数拼接在url上,使用post时,参数请使用json body传递。
        目前支持参数有:
            text - 待转换内容 必填参数
            informant - 发音人 可选参数,大小写严格, 默认为 zh-CN-XiaoxiaoNeural  可通过命令行参数查看所有支持的列表
            style - 发音风格 可选参数，默认为 general
            rate - 语速 可选参数 值范围 0-3 可保留两位小数, 默认为 1
            pitch - 音调 可选参数 值范围 0-2 可保留两位小数, 默认为 1
            quality - 音频格式 可选参数,默认为 audio-24khz-48kbitrate-mono-mp3  可通过命令行参数查看所有支持的列表

        基本使用教程:
            举例： 在开源软件[阅读]App中可以使用如下配置来使用该接口
                http://ip:port/tts-ms,{
                    "method": "POST",
                    "body": {
                        "informant": "zh-CN-XiaoxiaoNeural",
                        "style": "general",
                        "rate": {{ speakSpeed / 15 }},
                        "text": "{{java.encodeURI(speakText).replace('+','%20')}}"
                    }
                }
"##)]
#[clap(long_about = r##"TTS Api Server 软件仅供学习交流，严禁用于商业用途，请于24小时内删除！
    目前已实现接口有：[微软文本转语音] 后续看情况可能会再加入其他接口。

    微软文本转语音接口： /tts-ms
        接口支持 get,post 请求, get请求时参数拼接在url上,使用post时,参数请使用json body传递。
        目前支持参数有:
            text - 待转换内容 必填参数
            informant - 发音人 可选参数,大小写严格, 默认为 zh-CN-XiaoxiaoNeural  可通过命令行参数查看所有支持的列表
            style - 发音风格 可选参数，默认为 general
            rate - 语速 可选参数 值范围 0-3 可保留两位小数, 默认为 1
            pitch - 音调 可选参数 值范围 0-2 可保留两位小数, 默认为 1
            quality - 音频格式 可选参数,默认为 audio-24khz-48kbitrate-mono-mp3  可通过命令行参数查看所有支持的列表

        基本使用教程:
            举例： 在开源软件[阅读]App中可以使用如下配置来使用该接口
                http://ip:port/tts-ms,{
                    "method": "POST",
                    "body": {
                        "informant": "zh-CN-XiaoxiaoNeural",
                        "style": "general",
                        "rate": {{ speakSpeed / 15 }},
                        "text": "{{java.encodeURI(speakText)}}"
                    }
                }
"##)]
pub(crate) struct AppArgs {
    /// 指定连接渠道
    #[clap(long, arg_enum, value_name = "area", default_value_t = ServerArea::Default)]
    server_area: ServerArea,

    /// 监听地址
    #[clap(long, value_name = "address", default_value_t = String::from("0.0.0.0"))]
    listen_address: String,

    /// 监听端口
    #[clap(long, value_name = "prot", default_value_t = String::from("8080"))]
    listen_port: String,

    /// 显示可用发音人列表
    #[clap(long)]
    show_informant_list: bool,

    /// 显示音频质量参数列表
    #[clap(long)]
    show_quality_list: bool,

    /// 指定不从官方更新最新发音人 (可以快速使用本地缓存启动程序)
    #[clap(long)]
    do_not_update_speakers_list: bool,

    /// 是否开启 debug 日志
    #[clap(long)]
    debug: bool,

    /// 将日志记录至文件
    #[clap(long)]
    log_to_file: bool,

    /// 日志文件路径
    #[clap(long,default_value_t = format!("{}/local_ocr/ocr.log", std::env::temp_dir().to_str().unwrap()))]
    log_path: String,

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
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
// async
async fn main() {
    // let runtime = tokio::runtime::Runtime::new().unwrap();

    log_utils::init_log();
    let args: AppArgs = AppArgs::parse();

    if args.show_quality_list {
        println!("当前可使用的音频参数有: \n{:?}", ms_tts::MS_TTS_QUALITY_LIST);
        std::process::exit(0);
    }

    if args.show_informant_list {
        ms_tts::MS_TTS_CONFIG.get_or_init(|| async  {
            ms_tts::get_ms_tts_config().await.unwrap()
        }).await;
        println!("当前可使用的发音人参数有: \n{:?}", ms_tts::MS_TTS_CONFIG.get().unwrap().voices_list.voices_name_list);
        std::process::exit(0);
    }

    info!("准备启动，程序参数: {:?}", args);
    GLOBAL_EB.start().await;
    ms_tts::register_service().await;
    controller::register_service(args.listen_address.clone(), args.listen_port.clone()).await;
    info!("谢谢使用，希望能收到您对软件的看法和建议！");
    std::process::exit(0);

    // runtime.block_on(async move {
    //
    // });

}
