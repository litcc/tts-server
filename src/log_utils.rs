use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
//, Logger
use crate::AppArgs;
use clap::Parser;
use log4rs::encode::pattern::PatternEncoder;

pub(crate) fn init_log() {
    let args: AppArgs = AppArgs::parse();
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%f)} [{t}] {T} {I} {h({l})} - {m}{n}",
        )))
        .build();

    let mut config = Config::builder();
    config = config.appender(Appender::builder().build("stdout", Box::new(stdout)));
    if args.log_to_file {
        let log_to_file = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S.%f)} [{t}] {T} {I} {l} - {m}{n}",
            )))
            .build(args.log_path.clone())
            .unwrap();
        config = config.appender(Appender::builder().build("file", Box::new(log_to_file)));
    }
    config = config
        .logger(Logger::builder().build("reqwest", LevelFilter::Warn))
        .logger(Logger::builder().build("rustls", LevelFilter::Warn))
        .logger(Logger::builder().build("tungstenite", LevelFilter::Warn))
        .logger(Logger::builder().build("actix_server::builder", LevelFilter::Warn))
        .logger(Logger::builder().build("hyper", LevelFilter::Warn));

    let mut root = Root::builder().appender("stdout");
    if args.log_to_file {
        root = root.appender("file");
    }

    let config_tmp = config
        .build(if args.debug {
            root.build(LevelFilter::Debug)
        } else {
            root.build(LevelFilter::Info)
        })
        .unwrap();

    log4rs::init_config(config_tmp).unwrap();
}
