use clap::{ArgEnum, ArgMatches, Command, Parser, PossibleValue, ValueEnum};
use log::LevelFilter;
use once_cell::sync::OnceCell;

#[derive(Parser, Debug)]
#[clap(
name = env!("CARGO_PKG_NAME"),
version,
about = env!("CARGO_PKG_DESCRIPTION"),
author = env!("CARGO_PKG_AUTHORS"),
)]
pub struct AppArgs {
    /// 指定连接渠道， 可加速 Edge 接口请求速度
    #[clap(long, arg_enum, value_name = "area", default_value_t = ServerArea::Default)]
    pub server_area: ServerArea,

    /// 监听地址
    #[clap(long, value_name = "address", default_value_t = String::from("0.0.0.0"))]
    pub listen_address: String,

    /// 监听端口
    #[clap(long, value_name = "port", default_value_t = String::from("8080"))]
    pub listen_port: String,

    /// 显示可用发音人列表
    #[clap(long, parse(from_flag))]
    pub show_informant_list: bool,

    /// 显示音频质量参数列表
    #[clap(long, parse(from_flag))]
    pub show_quality_list: bool,

    /// 禁用 edge 免费预览接口
    #[clap(long, parse(from_flag))]
    pub close_edge_free_api: bool,

    /// 禁用 官方网页免费预览接口
    #[clap(long, parse(from_flag))]
    pub close_official_preview_api: bool,

    /// 禁用 官方网页收费（有免费额度）版本接口
    #[clap(long, parse(from_flag))]
    pub close_official_subscribe_api: bool,

    /// 对订阅API添加独立认证token
    #[clap(long, value_name = "token")]
    pub subscribe_api_auth_token: Option<String>,

    /// 指定不从官方更新最新发音人 (可以快速使用本地缓存启动程序)
    #[clap(long, parse(from_flag))]
    pub do_not_update_speakers_list: bool,

    /// 指定订阅API的官方订阅密钥以及地域， 可添加多个，遍历使用，格式：{subscribe_key},{region}   例： --subscribe-key 956d0b8cb34e4kb1b9cb8c614d313ae3,southeastasia
    #[clap(long)]
    pub subscribe_key: Vec<String>,

    /// 是否启用 webUI
    #[clap(long, parse(from_flag))]
    pub web_ui: bool,

    /// 是否开启 debug 日志  可用参数有: Off, Error, Warn, Info, Debug, Trace
    #[clap(long, default_value_t = LevelFilter::Info)]
    pub log_level: LevelFilter,

    /// 将日志记录至文件
    #[clap(long, parse(from_flag))]
    pub log_to_file: bool,

    /// 日志文件路径
    #[clap(long, default_value_t = format ! ("{}/local_ocr/ocr.log", std::env::temp_dir().to_str().unwrap()))]
    pub log_path: String,
}

impl AppArgs {
    #[allow(dead_code)]
    pub fn parse_macro() -> &'static Self {
        static GLOBAL_ARGS: OnceCell<AppArgs> = OnceCell::new();
        GLOBAL_ARGS.get_or_init(|| {
            let app_args = AppArgs::parse();
            app_args
        })
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn test_parse_macro(param: &[&str]) -> &'static Self {
        static TEST_GLOBAL_ARGS: OnceCell<AppArgs> = OnceCell::new();
        TEST_GLOBAL_ARGS.get_or_init(|| {
            let app_args = AppArgs::try_parse_from(param);
            app_args.unwrap()
        })
    }

    /*#[allow(dead_code)]
    pub fn parse_config() -> Self {
        /*let arg_parse = Command::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .disable_help_subcommand(true)
            .arg(
                Arg::new("version")
                    .long("version")
                    .short('v')
                    .action(ArgAction::Version)
                    .help("显示版本信息")
                    .display_order(0),
            )
            .arg(
                Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(ArgAction::Help)
                    .help("显示帮助")
                    .display_order(1),
            )
            .arg(
                Arg::new("listen_address")
                    .value_name("address")
                    .required(false)
                    .default_value("0.0.0.0")
                    .long("listen-address")
                    .short('a')
                    .takes_value(true)
                    .help("监听地址")
                    .display_order(2),
            )
            .arg(
                Arg::new("listen_port")
                    .value_name("port")
                    .required(false)
                    .default_value("8080")
                    .long("listen-port")
                    .short('p')
                    .takes_value(true)
                    .help("监听端口")
                    .display_order(3),
            )
            .arg(
                Arg::new("server_area")
                    .value_name("area")
                    .long("server-area")
                    .required(false)
                    .takes_value(true)
                    .help("微软 edge 接口指定连接渠道")
                    .display_order(4),
            )
            .arg(
                Arg::new("log_level")
                    .value_name("level")
                    .required(false)
                    .long("log-level")
                    .value_parser(EnumValueParser::<LevelFilterPack>::new())
                    .default_value(LevelFilter::Info.as_str())
                    .ignore_case(true)
                    .takes_value(true)
                    .help("日志等级")
                    .display_order(5),
            )
            .arg(
                Arg::new("log_to_file")
                    .required(false)
                    .long("log-to-file")
                    .takes_value(true)
                    .action(ArgAction::SetTrue)
                    .help("日志写入文件")
                    .display_order(6),
            )
            .arg(
                Arg::new("log_path")
                    .value_name("path")
                    .required(false)
                    .long("log-path")
                    // .default_value()
                    .takes_value(true)
                    .help("日志路径")

                    .display_order(7),
            )
            .subcommands(vec![
                Command::new("config")
                    .about("Controls configuration functionality")
                    .arg(Arg::new("config_file")),
                Command::new("debug").about("Controls debug functionality"),
            ]);
        let matchs = arg_parse.get_matches();
        info!("{:#?}", matchs);*/

    }*/
}

/// Wrapping LevelFilter
// 包装 LevelFilter 用以绕过孤儿规则
/*#[derive(Clone, Debug)]
struct LevelFilterPack(LevelFilter);

impl ToString for LevelFilterPack {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl PartialEq for LevelFilterPack {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl ValueEnum for LevelFilterPack {
    fn value_variants<'a>() -> &'a [Self] {
        let er = LevelFilter::iter()
            .map(|i| LevelFilterPack(i))
            .collect::<Vec<LevelFilterPack>>();
        let list = er.leak();
        list
    }

    fn from_str(input: &str, _ignore_case: bool) -> Result<Self, String> {
        Ok(LevelFilterPack(
            LevelFilter::from_str(input).unwrap_or(LevelFilter::Info),
        ))
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue<'a>> {
        Some(PossibleValue::new(self.0.as_str()))
    }
}*/
// impl From<ArgMatches> for AppArgs {
//     fn from(m: ArgMatches) -> Self {
//         AppArgs {
//             // verbose: *m.get_one::<bool>("verbose").expect("defaulted_by_clap"),
//             // name: m.get_one::<String>("name").cloned(),
//             server_area: m
//                 .get_one::<ServerArea>("server_area")
//                 .cloned()
//                 .unwrap_or(ServerArea::Default),
//             listen_address: m
//                 .get_one::<String>("listen_address")
//                 .cloned()
//                 .unwrap_or("0.0.0.0".to_owned()),
//             listen_port: m
//                 .get_one::<String>("listen_port")
//                 .cloned()
//                 .unwrap_or("8080".to_owned()),
//             show_informant_list: false,
//             show_quality_list: false,
//             do_not_update_speakers_list: false,
//             log_level: m
//                 .get_one::<LevelFilterPack>("log_level")
//                 .cloned()
//                 .map(|i| i.0),
//             log_to_file: m
//                 .get_one::<bool>("log_to_file").copied(),
//             log_path: None,
//         }
//     }
// }

/// Edge 免费接口连接地域
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum ServerArea {
    Default,
    China,
    ChinaHK,
    ChinaTW,
}
