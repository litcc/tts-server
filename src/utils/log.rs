use std::collections::HashMap;

use log::{debug, LevelFilter};
use log4rs::{
    self,
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
};

///
/// 初始化日志
#[allow(dead_code)]
pub(crate) fn init_log(
    log_level: LevelFilter,
    log_to_file: Option<bool>,
    log_path: Option<&str>,
    custom_level: Option<HashMap<String, LevelFilter>>,
) {
    let log_to_file = log_to_file.unwrap_or(false);
    let log_path_default = format!(
        "{}/tts-server/server.log",
        std::env::temp_dir().to_str().unwrap()
    );
    let log_path = if let Some(p) = log_path {
        p.to_owned()
    } else {
        log_path_default
    };

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%f)} [{t}] {T} {I} {h({l})} - {m}{n}",
        )))
        .build();

    let mut config = Config::builder();
    config = config.appender(Appender::builder().build("stdout", Box::new(stdout)));
    if log_to_file {
        let log_to_file = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S.%f)} [{t}] {T} {I} {l} - {m}{n}",
            )))
            .build(log_path.clone())
            .unwrap();
        config = config.appender(Appender::builder().build("file", Box::new(log_to_file)));
    }
    if let Some(c_l) = custom_level {
        for x in c_l {
            config = config.logger(Logger::builder().build(x.0, x.1))
        }
    }

    let mut root = Root::builder().appender("stdout");
    if log_to_file {
        root = root.appender("file");
    }

    // let config_tmp = config
    //     .build(root.build(LevelFilter::from_str(args.log_level.to_uppercase().as_str()).unwrap()))
    //     .unwrap();

    let config_tmp = config.build(root.build(log_level)).unwrap();

    log4rs::init_config(config_tmp).unwrap();
    if log_to_file {
        debug!("日志文件路径: {}", log_path);
    }
}

///
/// 初始化测试日志
#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn init_test_log(
    log_level: LevelFilter,
    custom_level: Option<HashMap<String, LevelFilter>>,
) {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%f)} [{t}] {T} {I} {h({l})} - {m}{n}",
        )))
        .build();
    let mut config = Config::builder();
    config = config.appender(Appender::builder().build("stdout", Box::new(stdout)));
    // .logger(Logger::builder().build("reqwest", LevelFilter::Warn))
    // .logger(Logger::builder().build("rustls", LevelFilter::Warn))
    // .logger(Logger::builder().build("actix_server::builder", LevelFilter::Warn))
    // .logger(Logger::builder().build("hyper", LevelFilter::Warn));

    if let Some(c_l) = custom_level {
        for x in c_l {
            config = config.logger(Logger::builder().build(x.0, x.1))
        }
    }

    let root = Root::builder().appender("stdout");
    let config_tmp = config.build(root.build(log_level)).unwrap();

    log4rs::init_config(config_tmp).unwrap();
}
