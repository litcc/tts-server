use futures::future::join_all;
use log::{error, info, LevelFilter};
use crate::AppArgs;
use crate::utils::azure_api::{AzureApiEdgeFree, AzureApiNewWebsocket, AzureApiPreviewFreeToken, AzureApiRegionIdentifier, AzureApiSubscribeToken, AzureAuthKey, VoicesItem};
use crate::utils::log::init_test_log;


///
/// 订阅 Key 获取测试
#[tokio::test]
async fn test_azure_api_subscribe_create_oauth_token() {
    init_test_log(LevelFilter::Debug, None);
    let region = std::env::var("REGION");
    let subscription_key = std::env::var("SUBSCRIPTION");

    if region.is_err() || subscription_key.is_err() {
        error!("未找到订阅key，跳过测试");
        std::process::exit(1);
    }
    let region = region.unwrap();
    let subscription_key = subscription_key.unwrap();
    let mut kk = AzureApiSubscribeToken::new(AzureApiRegionIdentifier::from(&region).unwrap(), &subscription_key);
    let oauth = kk.get_oauth_token().await;
    info!("oauth: \n{:?}",oauth);
    assert!(oauth.is_ok())
}


/// 订阅 Api Websocket 连接测试
#[tokio::test]
async fn test_azure_api_subscribe() {
    init_test_log(LevelFilter::Debug, None);
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
    init_test_log(LevelFilter::Debug, None);
    let args = AppArgs::test_parse_macro(&["test", "--listen-port", "8989"]);
    info!("{:?}",args);
    let mut kk2 = AzureApiEdgeFree::new();


    let jj = kk2.get_connection().await;

    assert!(jj.is_ok())
}

/// 官网预览界面 免费接口 Websocket 连接测试
#[tokio::test]
async fn test_azure_api_preview_free() {
    init_test_log(LevelFilter::Debug, None);

    let mut kk2 = AzureApiPreviewFreeToken::new();
    // let mut kk3 = kk2.lock().await;

    let jj = kk2.get_connection().await;

    // info!("{:?}",jj);
    // info!("{:?}",oauth_base);
    // drop(kk3);
    assert!(jj.is_ok())
}

///
#[tokio::test]
async fn test_azure_api_edge_free_connectivity_cn_hk() -> anyhow::Result<()> {
    init_test_log(LevelFilter::Debug, None);

    let mut list = Vec::new();
    for i in AzureApiEdgeFree::MS_TTS_SERVER_CHINA_HK_LIST {
        info!("[{}]", i);
        let kk = AzureApiEdgeFree::new_websocket_by_select_server(Some(i));
        list.push(kk);
    }

    let list = join_all(list).await;

    for (index,x) in list.iter().enumerate() {
        info!("ip: {}",AzureApiEdgeFree::MS_TTS_SERVER_CHINA_HK_LIST.get(index).unwrap());
        info!("ok: {:?}",x.is_ok())
    }

    Ok(())
}

///
#[tokio::test]
async fn test_azure_api_edge_free_connectivity_cn() -> anyhow::Result<()> {
    init_test_log(LevelFilter::Debug, None);

    let mut list = Vec::new();
    for i in AzureApiEdgeFree::MS_TTS_SERVER_CHINA_LIST {
        info!("[{}]", i);
        let kk = AzureApiEdgeFree::new_websocket_by_select_server(Some(i));
        list.push(kk);
    }

    let list = join_all(list).await;

    for (index,x) in list.iter().enumerate() {
        info!("ip: {}",AzureApiEdgeFree::MS_TTS_SERVER_CHINA_LIST.get(index).unwrap());
        info!("ok: {:?}",x.is_ok())
    }

    Ok(())
}

///
#[tokio::test]
async fn test_azure_api_edge_free_connectivity_cn_tw() -> anyhow::Result<()> {
    init_test_log(LevelFilter::Debug, None);

    let mut list = Vec::new();
    for i in AzureApiEdgeFree::MS_TTS_SERVER_CHINA_TW_LIST {
        info!("[{}]", i);
        let kk = AzureApiEdgeFree::new_websocket_by_select_server(Some(i));
        list.push(kk);
    }

    let list = join_all(list).await;

    for (index,x) in list.iter().enumerate() {
        info!("ip: {}",AzureApiEdgeFree::MS_TTS_SERVER_CHINA_TW_LIST.get(index).unwrap());
        info!("ok: {:?}",x.is_ok())
    }

    Ok(())
}


/// 官网预览界面 免费接口 Websocket 连接测试
#[tokio::test]
async fn test_azure_api_serde_voices_list_data() -> anyhow::Result<()> {
    init_test_log(LevelFilter::Debug, None);

    let body = include_bytes!("../resource/azure_voices_list.json");
    let voice_list: Vec<VoicesItem> = serde_json::from_slice(&body.to_vec()).map_err(|e| {
        error!("{:?}", e);
        e
    })?;

    println!("{:?}", voice_list);

    let body2 = include_bytes!("../resource/edge_voices_list.json");
    let voice_list2: Vec<VoicesItem> = serde_json::from_slice(&body2.to_vec()).map_err(|e| {
        error!("{:?}", e);
        e
    })?;

    println!("{:?}", voice_list2);

    Ok(())
}






