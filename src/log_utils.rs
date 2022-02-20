use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
//, Logger
use log4rs::encode::pattern::PatternEncoder;
use std::env;

pub(crate) fn init_log() {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%f)} [{t}] {T} {I} {h({l})} - {m}{n}",
        )))
        .build();

    let requests = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%f)} [{t}] {T} {I} {l} - {m}{n}",
        )))
        .build(format!(
            "{}/local_ocr/ocr.log",
            env::temp_dir().to_str().unwrap()
        ))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(requests)))
        .logger(Logger::builder().build("reqwest", LevelFilter::Warn))
        .logger(Logger::builder().build("rustls", LevelFilter::Warn))
        .logger(Logger::builder().build("tungstenite", LevelFilter::Warn))
        .logger(Logger::builder().build("actix_server::builder", LevelFilter::Warn))
        .logger(Logger::builder().build("hyper", LevelFilter::Warn))
        // .logger(Logger::builder()
        //     .appender("file")
        //     .additive(true)
        //     .build("app", LevelFilter::Info))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(LevelFilter::Info),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}
