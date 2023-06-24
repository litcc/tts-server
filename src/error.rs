use anyhow::{Error as AnyError, Result as AnyResult};
use thiserror::Error;

pub type Result<T, E = AnyError> = AnyResult<T, E>;

pub type Error = AnyError;
///
/// 该程序所有错误
#[derive(Error, Debug)]
pub enum TTSServerError {
    ///
    /// 程序意外错误
    #[error("Program unexpected error! {0}")]
    ProgramError(String),
    ///
    /// 第三方api调用出错
    #[error("3rd party api call failed! {0}")]
    ThirdPartyApiCallFailed(String),
}
