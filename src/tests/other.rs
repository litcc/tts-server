// use std::process::exit;
// use crate::ms_tts::{MsTtsMsgRequest, TAG_BODY_SPLIT, TAG_NONE_DATA_START};
// use bytes::{BufMut, BytesMut};
// use fancy_regex::Regex;
// use std::thread;
// use std::time::Duration;
// use chrono::Utc;
// use futures::{SinkExt, StreamExt};
// use log::{debug, error, info, LevelFilter, trace};
// use tokio::fs::File;
// use tokio::io::AsyncWriteExt;
// use tokio_tungstenite::tungstenite::Message;
//
// // use json::JsonValue;
//
// use super::*;
//
// // 测试日志
// fn init_log() {
//     utils::log::init_test_log(LevelFilter::Debug);
// }
//
// #[test]
// fn test_get_ms_tts_token() {
//     println!("ms tts websocket token: ");
//     let token: [u8; 32] = [
//         54, 65, 53, 65, 65, 49, 68, 52, 69, 65, 70, 70, 52, 69, 57, 70, 66, 51, 55, 69, 50, 51, 68,
//         54, 56, 52, 57, 49, 68, 54, 70, 52,
//     ];
//
//     let _kk = "\r\nX-StreamId:";
//     let _kk = "\r\nX-StreamId:";
//     println!("{}", String::from_utf8(token.to_vec()).unwrap())
// }
//
// #[test]
// fn test4() {
//     thread::spawn(|| {
//         for i in 1..10 {
//             println!("hi number {} from the spawned thread!", i);
//             thread::sleep(Duration::from_millis(1));
//         }
//     });
//
//     for i in 1..5 {
//         println!("hi number {} from the main thread!", i);
//         thread::sleep(Duration::from_millis(1));
//     }
// }
//
// ///
// /// 测试微软服务器连通性
// #[tokio::test]
// async fn test_ms_server_connectivity_cn() {
//     init_log();
//     info!("开始测试服务器连通性");
//     for i in AzureApiEdgeFree::MS_TTS_SERVER_CHINA_LIST {
//         info!("[{}]", i);
//         let kk = new_websocket_by_select_server(Some(i)).await;
//         error!("{:?}", kk);
//         assert!(kk.is_ok());
//     }
// }
//
// /// 测试微软服务器连通性
// #[tokio::test]
// async fn test_ms_server_connectivity_cn_tw() {
//     init_log();
//     info!("开始测试服务器连通性");
//
//     for i in AzureApiEdgeFree::MS_TTS_SERVER_CHINA_TW_LIST {
//         info!("[{}]", i);
//         let kk = new_websocket_by_select_server(Some(i)).await;
//         assert!(kk.is_ok());
//     }
// }
//
// /// 测试微软服务器连通性
// #[tokio::test]
// async fn test_ms_server_connectivity_cn_hk() {
//     init_log();
//     info!("开始测试服务器连通性");
//     for i in AzureApiEdgeFree::MS_TTS_SERVER_CHINA_HK_LIST {
//         info!("[{}]", i);
//         let kk = new_websocket_by_select_server(Some(i)).await;
//         assert!(kk.is_ok());
//     }
// }
//
//
// /// 测试微软服务器连通性
// #[tokio::test]
// async fn test_ms_server_connectivity() {
//     init_log();
//     info!("开始测试服务器连通性");
//
//     let kk = new_websocket_by_select_server(None).await;
//     assert!(kk.is_ok());
// }
//
//
// #[tokio::test]
// async fn test_ms_tts_websocket_call() {
//     init_log();
//     debug!("test_float_calculate");
//     //Some(ms_tts::MS_TTS_SERVER_CHINA_LIST.last().unwrap())
//     let ip = AzureApiEdgeFree::MS_TTS_SERVER_CHINA_LIST.first().unwrap();
//     let ip = "182.61.148.24";
//     let ip = "182.61.148.24";
//     debug!("ip:{}",ip);
//     // 182.61.148.24 不行
//     let kk = new_websocket_by_select_server(None).await.unwrap();
//     debug!("构造 ssml 消息");
//     let id = random_string(32);
//     let create_time = Utc::now();
//     let time = format!("{:?}", create_time);
//
//
//     let mut msg1 = format!("Path: speech.config\r\nX-Timestamp: {}\r\nX-RequestId: {}\r\nContent-Type: application/json\r\n\r\n", &time, &id);
//     msg1.push_str("{\"context\":{\"system\":{\"name\":\"SpeechSDK\",\"version\":\"1.19.0\",\"build\":\"JavaScript\",\"lang\":\"JavaScript\"},\"os\":{\"platform\":\"Browser/Win32\",\"name\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36 Edg/102.0.1245.39\",\"version\":\"5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36 Edg/102.0.1245.39\"}}}");
//
//     let mut msg2 = String::new();
//     msg2.push_str(format!("Path: synthesis.context\r\nX-RequestId: {}\r\nX-Timestamp: {}\r\nContent-Type: application/json", &id, &time).as_str());
//     msg2.push_str("\r\n\r\n{\"synthesis\":{\"audio\":{\"metadataOptions\":{\"bookmarkEnabled\":false,\"sentenceBoundaryEnabled\":false,\"visemeEnabled\":false,\"wordBoundaryEnabled\":false},\"outputFormat\":\"ogg-16khz-16bit-mono-opus\"},\"language\":{\"autoDetection\":false}}}");
//
//
//     //
//     // let msg3 = format!(r"Path: ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nPath:ssml\r\n\r\n<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xmlns:mstts='https://www.w3.org/2001/mstts' xml:lang='en-US'><voice name='Microsoft Server Speech Text to Speech Voice (zh-CN, XiaoxiaoNeural)'><prosody pitch='+0Hz' rate ='+0%' volume='+0%'>新华社北京6月15日电（记者魏玉坤、王悦阳）国家统计局15日发布数据显示，5月份，我国经济逐步克服疫情不利影响，生产需求逐步恢复，就业物价总体稳定，主要指标边际改善，国民经济呈现恢复势头。</prosody></voice></speak>",id);
//
//
//     //
//     // let mut msg1 = String::new();
//     // msg1.push_str("Content-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n");
//     // msg1.push_str("{\"context\":{\"synthesis\":{\"audio\":{\"metadataoptions\":{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"},\"outputFormat\":\"webm-24khz-16bit-mono-opus\"}}}}\r\n");
//
//     //Path: ssml
//     // X-RequestId: 3508B5D2EDF1460292AE6F03EB6F4F32
//     // X-Timestamp: 2022-06-16T15:42:07.494Z
//     // Content-Type: application/ssml+xml
//
//     let mut msg3 = String::new();
//
//     msg3.push_str(format!("Path:ssml\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\n\r\n", &id).as_str());
//     //
//     // Microsoft Server Speech Text to Speech Voice (zh-CN, XiaoxiaoNeural)
//     msg3.push_str("<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xmlns:mstts='https://www.w3.org/2001/mstts' xmlns:emo=\"http://www.w3.org/2009/10/emotionml\" xml:lang='en-US'><voice name='zh-CN-XiaoxiaoNeural'><prosody pitch='+0Hz' rate ='+0%' volume='+0%'>新华社北京6月15日电（记者魏玉坤、王悦阳）国家统计局15日发布数据显示，5月份，我国经济逐步克服疫情不利影响，生产需求逐步恢复，就业物价总体稳定，主要指标边际改善，国民经济呈现恢复势头。</prosody></voice></speak>");
//
//
//     // msg3.push_str(r#"<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xmlns:mstts="http://www.w3.org/2001/mstts" xmlns:emo="http://www.w3.org/2009/10/emotionml" xml:lang="zh-CN"><voice name="zh-CN-XiaoxiaoNeural"><mstts:express-as style="chat">也可以合成多角色多情感的有声<prosody contour="(49%, -40%)">书</prosody>，例如：</mstts:express-as></voice><voice name="zh-CN-YunyeNeural">黛玉冷笑道：</voice><voice name="zh-CN-XiaoxiaoNeural"><s /><mstts:express-as style="disgruntled">“我说呢，亏了绊住，不然，早就飞了来了。”</mstts:express-as><s /> </voice><voice name="zh-CN-YunyeNeural">宝玉道：</voice><voice name="zh-CN-YunxiNeural">“只许和你玩，替你解闷。不过偶然到他那里，就说这些闲话。”</voice><voice name="zh-CN-XiaoxiaoNeural"><mstts:express-as style="angry">”好没意思的话！去不去，关我什么事儿？又没叫你替我解闷儿，还许你<mstts:ttsbreak strength="none" />从此<prosody contour="(24%, +49%) (59%, -2%)">不</prosody><prosody rate="-15.00%" contour="(24%, +49%) (59%, -2%)">理</prosody><prosody contour="(24%, +49%) (59%, -2%)">我呢</prosody>！”</mstts:express-as></voice><voice name="zh-CN-YunyeNeural"><s />说着，便赌气回房去了。</voice></speak>"#);
//
//     debug!("{:#?}",msg1);
//     debug!("{:#?}",msg2);
//     debug!("{:#?}",msg3);
//
//     let (mut a, mut b) = kk.split();
//     // a.send(Message::Text(msg1)).await.unwrap();
//     debug!("发送 ssml 消息");
//     a.send(Message::Text(msg2)).await.unwrap();
//
//     a.send(Message::Text(msg3)).await.unwrap();
//     let mut df = b.next().await;
//     let mut resp = BytesMut::new();
//     while df.is_some() {
//         debug!("收到消息");
//         let kk = df.unwrap();
//         if let Ok(ss) = kk {
//             info!("{:?}",ss);
//             get_file_body(&mut resp, ss).await;
//         } else {
//             error!("{:?}",kk)
//         }
//         df = b.next().await;
//     }
//
//     // 向 websocket 发送消息
//     //
//     // thread::sleep(Duration::from_secs(5));
// }
//
// async fn get_file_body(resp: &mut BytesMut, msg: Message) {
//     match msg {
//         Message::Ping(s) => {
//             trace!("收到ping消息: {:?}", s);
//         }
//         Message::Pong(s) => {
//             trace!("收到pong消息: {:?}", s);
//         }
//         Message::Close(s) => {
//             debug!("被动断开连接: {:?}", s);
//         }
//         Message::Text(s) => {
//             let id = s[12..44].to_string();
//             // info!("到消息: {}", id);
//             if let Some(_i) = s.find("Path:turn.start") {
//                 // MS_TTS_DATA_CACHE.lock().await.insert(id, BytesMut::new());
//             } else if let Some(_i) = s.find("Path:turn.end") {
//                 trace!("响应 {}， 结束", id);
//                 File::create(format!("tmp/test-{}.mp3", &id)).await
//                     .unwrap().write_all(&resp.to_vec()).await.unwrap();
//                 ;
//                 exit(0)
//             }
//         }
//         Message::Binary(s) => {
//             if s.starts_with(&crate::ms_tts::TAG_NONE_DATA_START) {
//                 // }
//                 // drop(cache_map);
//                 // drop(cache);
//                 // unsafe { Arc::get_mut_unchecked(&mut MS_TTS_DATA_CACHE.clone()).get_mut(&id).unwrap().lock().await.data.put(body) };
//                 let id = String::from_utf8(s[14..46].to_vec()).unwrap();
//                 trace!("二进制响应体结束 TAG_NONE_DATA_START, {}",id);
//                 // trace!("二进制响应体 ,{}",id);
//             }/* else if s.starts_with(&TAG_NONE_DATA_START) {
//                 let id = String::from_utf8(s[14..46].to_vec()).unwrap();
//                 trace!("二进制响应体结束 TAG_NONE_DATA_START, {}",id);
//             } */else {
//                 let id = String::from_utf8(s[14..46].to_vec()).unwrap();
//                 let mut body = BytesMut::from(s.as_slice());
//                 let index = binary_search(&s, &TAG_BODY_SPLIT).unwrap();
//                 let head = body.split_to(index + TAG_BODY_SPLIT.len());
//                 resp.put(body);
//                 // if cache_map.file_type.is_none() {
//                 let head = String::from_utf8(head.to_vec()[2..head.len()].to_vec()).unwrap();
//                 let head_list = head.split("\r\n").collect::<Vec<&str>>();
//                 let content_type = head_list[1].to_string().split(":").collect::<Vec<&str>>()[1].to_string();
//                 trace!("content_type: {}", content_type);
//
//                 trace!("其他二进制类型: {} ", unsafe { String::from_utf8_unchecked(s.to_vec()) });
//             }
//         }
//         _ => {}
//     }
// }
//
//
// #[tokio::test]
// async fn test_bytes() {
//     init_log();
//     info!("test_ms_tts_websocket");
//
//     let _tag_some_data_start = [0, 128];
//     let _tag_none_data_start = [0, 103];
//
//     let tag1 = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
//     let _tag1_2: [u8; 12] = [80, 97, 116, 104, 58, 97, 117, 100, 105, 111, 13, 10];
//     let tag2: [u8; 5] = [0, 128, 88, 45, 82];
//
//     info!("tag1: {:?}", tag1.as_bytes());
//     info!("tag2: {}", unsafe {
//         String::from_utf8_unchecked(tag2.to_vec())
//     });
//
//     let _b = BytesMut::new();
//
//     // b.put(&b"123"[..]);
//     // b.reserve(2);
//     // b.put_slice(b"xy");
//     // info!("{:?}",b);
//     // info!("{:?}",b.capacity());
// }
//
// #[tokio::test]
// async fn test_serialize() {
//     init_log();
//     info!("test_serialize");
//     let test = MsTtsMsgRequest {
//         text: "123".to_string(),
//         request_id: "123".to_string(),
//         informant: "123".to_string(),
//         style: "123".to_string(),
//         rate: "123".to_string(),
//         pitch: "123".to_string(),
//         quality: "123".to_string(),
//     };
//     let encoded: Vec<u8> = bincode::serialize(&test).unwrap();
//
//     info!("test: {:?}", encoded);
//     //let decoded: MsTtsRequest = bincode::deserialize(&encoded[..]).unwrap();
//     //let adsf:Vec<u8> = postcard::to_allocvec(&test).unwrap();
// }
//
// // #[tokio::test]
// #[test]
// fn test_regex() {
//     init_log();
//     info!("test_regex");
//
//     let re = Regex::new(r"^\s*$").unwrap();
//     let result = re
//         .is_match(
//             r#"
// asdfasdf"#,
//         )
//         .unwrap();
//     info!("{}", result);
//     thread::sleep(Duration::from_secs(5));
// }
//
// #[tokio::test]
// // #[test]
// async fn test_get_ms_tts_config() {
//     init_log();
//     debug!("test_get_ms_tts_config");
//
//     let kk = crate::ms_tts::get_ms_tts_config().await;
//     if let Some(s) = kk {
//         debug!("请求成功");
//         info!("{:?}", s);
//     } else {
//         debug!("请求失败");
//     }
//
//     //info!("{}",result);
//     thread::sleep(Duration::from_secs(5));
// }
//
// #[tokio::test]
// // #[test]
// async fn test_ms_tts_config() {
//     init_log();
//     debug!("test_get_ms_tts_config");
//
//     let kk_s = crate::ms_tts::get_ms_tts_config().await.unwrap();
//     info!("{:?}", kk_s);
//     info!(
//         "en-US: {}",
//         kk_s.voices_list.by_locale_map.get("en-US").unwrap().len()
//     );
//
//     // 411
//
//     // info!("{}",kk);
//     thread::sleep(Duration::from_secs(5));
// }
//
// #[tokio::test]
// // #[test]
// async fn test_float_calculate() {
//     init_log();
//     debug!("test_float_calculate");
//     let x: f32 = 3.14159;
//
//     let _kk = num::BigInt::parse_bytes(b"2", 10);
//     let sin_x = x.sin();
//     println!("sin({}) = {}", x, sin_x);
//     let style = 1.6666666666666666666666666666666;
//     let kk_s = 100.00 * style - 100.00;
//
//     info!("{:.0}", kk_s);
//
//     //
//     thread::sleep(Duration::from_secs(5));
// }
//
// use urlencoding::decode as url_decode;
// use crate::{ms_tts, random_string, utils};
// use crate::utils::azure_api::{AzureApiEdgeFree};
// use crate::utils::binary_search;
//
// #[tokio::test]
// async fn test_url_decode() {
//     init_log();
//     debug!("test_float_calculate");
//     let mut sss = "你好".to_string();
//
//     let text_tmp2: String = 'break_1: loop {
//         let decoded = url_decode(&sss);
//         if let Ok(s) = decoded {
//             if sss == s.to_string() {
//                 break 'break_1 sss;
//             }
//             sss = s.to_string();
//         } else {
//             break 'break_1 sss;
//         }
//     };
//
//     info!("{}", text_tmp2);
//     //
//     thread::sleep(Duration::from_secs(5));
// }
