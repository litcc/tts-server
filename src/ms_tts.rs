use std::convert::TryFrom;
use std::net::Ipv4Addr;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::{client_async_with_config, WebSocketStream};
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::{Method, Uri, Version};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;


use crate::info;
use crate::utils::{get_system_ca_config, random_string};

//log::set_max_level(LevelFilter::Trace);
//"202.89.233.100","202.89.233.101" speech.platform.bing.com

pub(crate) fn register_service() {
    todo!()
}



///
/// 获取新的隧道连接
pub(crate) async fn ms_tts_websocket() -> Result<WebSocketStream<TlsStream<TcpStream>>, String> {
    let trusted_client_token = format!("6A5AA1D4EAFF4E9FB37E23D68491D6F4");
    let connect_id = random_string(32);
    let uri = Uri::builder()
        .scheme("wss")
        .authority("speech.platform.bing.com")
        .path_and_query(format!(
            "/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&ConnectionId={}",
            &trusted_client_token, &connect_id
        ))
        .build();
    if let Err(e) = uri {
        return Err(format!("uri 构建错误 {:?}", e));
    }
    let uri = uri.unwrap();

    let request_builder = Request::builder().uri(uri).method(Method::GET)
        .header("Sec-webSocket-Extension", "permessage-deflate")
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .header("Accept", "*/*")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.212 Safari/537.36 Edg/90.0.818.62")
        .header("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold")
        .version(Version::HTTP_11);
    let request = request_builder.body(());
    if let Err(e) = request {
        return Err(format!("request_builder 构建错误 {:?}", e));
    }
    let request = request.unwrap();

    let config = get_system_ca_config();
    let config = TlsConnector::from(Arc::new(config));
    let domain = tokio_rustls::rustls::ServerName::try_from("speech.platform.bing.com");

    if let Err(e) = domain {
        return Err(format!("dns解析错误 speech.platform.bing.com {:?}", e));
    }
    let domain = domain.unwrap();

    //let mut sock = TcpStream::connect("speech.platform.bing.com:443").await.unwrap();
    let sock = TcpStream::connect((Ipv4Addr::new(202, 89, 233, 100), 443)).await;

    if let Err(e) = sock {
        return Err(format!("tcp握手失败! 请检查网络! {:?}", e));
    }
    let sock = sock.unwrap();

    let tsl_stream = config.connect(domain, sock).await;

    if let Err(e) = tsl_stream {
        return Err(format!("tsl握手失败! {}", e));
    }
    let tsl_stream = tsl_stream.unwrap();

    let websocket = client_async_with_config(
        request,
        tsl_stream,
        Some(WebSocketConfig {
            max_send_queue: None,
            max_message_size: None,
            max_frame_size: None,
            accept_unmasked_frames: false,
        }),
    )
        .await;
    return match websocket {
        Ok(_websocket) => {
            info!("websocket 握手成功");
            Ok(_websocket.0)
        }
        Err(e2) => Err(format!("websocket 握手失败! {:?}", e2)),
    };
}

