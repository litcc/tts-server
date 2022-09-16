use std::time::Duration;
use log::{info, LevelFilter};
use crate::AppArgs;
use crate::utils::azure_api::{AzureApiEdgeFree, AzureApiNewWebsocket, AzureApiPreviewFreeToken, AzureApiRegionIdentifier, AzureApiSubscribeToken, AzureAuthKey};
use crate::utils::log::init_test_log;


///
/// 订阅 Key 获取测试
#[tokio::test]
async fn test_azure_api_subscribe_create_oauth_token() {
    init_test_log(LevelFilter::Debug);
    let region = std::env::var("REGION").unwrap();
    let subscription_key = std::env::var("SUBSCRIPTION").unwrap();
    let mut kk = AzureApiSubscribeToken::new(AzureApiRegionIdentifier::from(&region).unwrap(), &subscription_key);
    let oauth = kk.get_oauth_token().await;
    info!("oauth: \n{:?}",oauth);
    assert!(oauth.is_ok())
}


/// 订阅 Api Websocket 连接测试
#[tokio::test]
async fn test_azure_api_subscribe() {
    init_test_log(LevelFilter::Debug);
    let region = std::env::var("REGION").unwrap();
    let subscription_key = std::env::var("SUBSCRIPTION").unwrap();
    let mut kk = AzureApiSubscribeToken::new(AzureApiRegionIdentifier::from(&region).unwrap(), &subscription_key);
    let oauth = kk.get_oauth_token().await.unwrap();
    info!("{:?}",oauth);
    let jj = kk.get_connection().await;


    assert!(jj.is_ok())
}

/// Edge 免费预览接口 Websocket 连接测试
#[tokio::test]
async fn test_azure_api_edge_free() {
    init_test_log(LevelFilter::Debug);
    let args = AppArgs::test_parse_macro(&["test", "--listen-port", "8989"]);
    info!("{:?}",args);
    let mut kk2 = AzureApiEdgeFree::new();

    let mut kk3 = kk2.lock().await;

    let jj = kk3.get_connection().await;

    assert!(jj.is_ok())
}

/// 官网预览界面 免费接口 Websocket 连接测试
#[tokio::test]
async fn test_azure_api_preview_free() {
    init_test_log(LevelFilter::Debug);

    let mut kk2 = AzureApiPreviewFreeToken::new();
    let mut kk3 = kk2.lock().await;

    let jj = kk3.get_connection().await;

    // info!("{:?}",jj);
    // info!("{:?}",oauth_base);
    drop(kk3);
    assert!(jj.is_ok())
}

