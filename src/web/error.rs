use std::fmt::{Display, Formatter};
use actix_web::{error, HttpResponse};
use crate::web::entity::ApiBaseResponse;

#[derive(Debug)]
pub struct ControllerError {
    pub code: i32,
    pub msg: String,
}

impl ControllerError {
    pub fn new<T:Into<String>>(msg: T) -> Self {
        ControllerError {
            code: 500,
            msg: msg.into(),
        }
    }
    pub fn from_status_code<T:Into<String>>(code: i32, msg:T) -> Self {
        ControllerError {
            code,
            msg: msg.into(),
        }
    }
}

impl Display for ControllerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error Code: {}, Controller Error: {}",
            self.code, self.msg
        )
    }
}

// Use default implementation for `error_response()` method
impl error::ResponseError for ControllerError {
    fn error_response(&self) -> HttpResponse {
        ApiBaseResponse::<()>::error_by_status_code(self.code, &self.msg).into()
    }
}