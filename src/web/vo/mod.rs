

use actix_http::body::BoxBody;
use actix_web::{http::StatusCode, HttpRequest, HttpResponse, HttpResponseBuilder, Responder};

use log::{error};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{Error, Result};

/// Http 响应码
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpStatus {
    pub code: u16,
    pub reason: Option<String>,
}

impl From<StatusCode> for HttpStatus {
    fn from(value: StatusCode) -> Self {
        let reason = value.canonical_reason().map(|i| i.to_string());
        HttpStatus {
            code: value.as_u16(),
            reason,
        }
    }
}

impl From<HttpStatus> for StatusCode {
    fn from(value: HttpStatus) -> Self {
        StatusCode::from_u16(value.code).expect("错误的状态码")
    }
}

/// 基础响应体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseResponse<T> {
    /// http 响应码
    pub code: u16,
    /// 具体响应内容 (可选)
    pub data: Option<T>,
    /// 响应信息或错误信息 (可选)
    pub msg: Option<String>,
}

impl<T> BaseResponse<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn from_result<E: Into<Error>>(arg: Result<T, E>) -> Self {
        match arg {
            Ok(data) => {
                let status: HttpStatus = StatusCode::OK.into();
                Self {
                    code: status.code,
                    msg: status.reason,
                    data: Some(data),
                }
            }
            Err(e) => {
                let err = e.into();
                error!("BaseResponse InternalError: {:?}", err);
                let status: HttpStatus = StatusCode::INTERNAL_SERVER_ERROR.into();
                Self {
                    code: status.code,
                    msg: Some(format!("错误信息: {}", err)),
                    data: None,
                }
            }
        }
    }

    pub fn from(arg: T) -> Self {
        let status: HttpStatus = StatusCode::OK.into();
        Self {
            code: status.code,
            msg: status.reason,
            data: Some(arg),
        }
    }

    pub fn from_error(code: HttpStatus, arg: &Error) -> Self {
        error!("BaseResponse from_error: {:?} {:?}", code, arg);
        Self {
            code: code.code,
            msg: Some(format!("请求失败, {}", arg)),
            data: None,
        }
    }

    pub fn from_error_info<V: ToString>(code: HttpStatus, info: V) -> Self {
        let err_str = info.to_string();
        error!("BaseResponse from_error_info: {:?} {:?}", code, err_str);
        Self {
            code: code.code,
            msg: Some(err_str),
            data: None,
        }
    }

    pub fn http_resp(&self) -> HttpResponse {
        return HttpResponseBuilder::new(StatusCode::from_u16(self.code).unwrap())
            .insert_header(("Access-Control-Allow-Origin", "*"))
            .insert_header(("Cache-Control", "no-cache"))
            .insert_header(("Content-Type", "text/json;charset=utf-8"))
            .body(self.to_string());
    }
}

impl<T> ToString for BaseResponse<T>
where
    T: Serialize + DeserializeOwned,
{
    fn to_string(&self) -> String {
        serde_json::to_string(self)
            .map_err(|e| error!("BaseResponse<T> ToString 序列化错误 {:?}", e))
            .unwrap()
    }
}

impl<T> Responder for BaseResponse<T>
where
    T: Serialize + DeserializeOwned,
{
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        self.http_resp()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmptyVO {}

pub const EMPTY_VO: EmptyVO = EmptyVO {};
