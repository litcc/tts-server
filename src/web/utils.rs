use std::hash::Hash;
use actix_web::dev::{ServiceRequest};
use actix_web::http::header;
use actix_web::HttpMessage;
use anyhow::{Error, Result};
use bytes::BytesMut;
use futures::StreamExt;
use log::error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use actix_http::h1::Payload;


pub async fn get_request_body_to_entity<T>(sr: &mut ServiceRequest) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
{
    let content_type = sr
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.clone().to_str().unwrap_or("").to_string())
        .unwrap_or_else(|| "".to_string())
        .to_lowercase();

    // Content type that is not application json won't be logged since it can cause
    // harm in some setups, this might be a feature to implement sometimes in the future,
    // once we get a proper chance to test it and figure out all the bugs that keep happening,
    // but for now we will simply set it as Null value in the log.
    //
    // Issue that we got was that some multipart forms weren't recognized properly after
    // the things we did here below to them, the issue couldn't be reproduced in a local
    // setting, but it was happening within the cluster.
    //
    // Payload would apear okay in treblle.com, but later methods that were supposed
    // to handle that payload reported invalid multipart data, or form data.
    if content_type != "application/json" {
        return Err(Error::msg("Content type is not application/json"));
    }
    let mut body_bytes = BytesMut::new();
    let mut stream = sr.take_payload();
    while let Some(chunk) = stream.next().await {
        body_bytes.extend_from_slice(&chunk?);
    }
    let bytes = body_bytes.freeze();

    let (_sender, mut orig_payload) = Payload::create(true);
    orig_payload.unread_data(bytes.clone());
    sr.set_payload(actix_http::Payload::from(orig_payload));

    if bytes.is_empty() {
        return Err(Error::msg("Empty body received"));
    }
    let serde_obj = serde_json::from_slice::<T>(&bytes.to_vec())
        .map(|v| v)
        .map_err(|e| {
            let err_msg = format!("Error deserializing request body: {}", e);
            error!("{}", err_msg);
            Error::msg(err_msg)
        })?;
    Ok(serde_obj)
}