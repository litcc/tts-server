use std::future::{ready, Ready};
use std::marker::PhantomData;
use std::rc::Rc;
use actix_http::body::{BoxBody, to_bytes};
use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{HttpResponse};
use actix_web::http::{header, StatusCode};
use actix_web::http::header::HeaderValue;
use futures::future::LocalBoxFuture;
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::AppArgs;
use crate::web::entity::ApiBaseResponse;
use crate::web::utils::get_request_body_to_entity;

/// 请求体中获取 token
pub trait AuthTokenValue {
    fn get_token(&self) -> Option<&str>;
}


///
/// 权限认证中间件
///
#[derive(Debug, Clone)]
pub struct TokenAuthentication<T>
    where
        T: Serialize + DeserializeOwned + AuthTokenValue
{
    _marker: PhantomData<T>,
}


impl<T> Default for TokenAuthentication<T>
    where
        T: Serialize + DeserializeOwned + AuthTokenValue
{
    fn default() -> Self {
        TokenAuthentication::<T> {
            _marker: PhantomData,
        }
    }
}


impl<S, B, T> Transform<S, ServiceRequest> for TokenAuthentication<T>
    where
        S: 'static + Service<ServiceRequest, Response=ServiceResponse<B>, Error=actix_web::Error>,
        S::Future: 'static,
        B: 'static + MessageBody,
        T: Serialize + DeserializeOwned + AuthTokenValue,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Transform = TokenAuthenticationMiddleware<S, T>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TokenAuthenticationMiddleware {
            service: Rc::new(service),
            _marker: PhantomData,
        }))
    }
}

pub struct TokenAuthenticationMiddleware<S, T>
    where
        T: Serialize + DeserializeOwned + AuthTokenValue
{
    service: Rc<S>,
    _marker: PhantomData<T>,
}

impl<S, B, T> Service<ServiceRequest> for TokenAuthenticationMiddleware<S, T>
    where
        S: 'static + Service<ServiceRequest, Response=ServiceResponse<B>, Error=actix_web::Error>,
        S::Future: 'static,
        B: 'static + MessageBody,
        T: Serialize + DeserializeOwned + AuthTokenValue
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        info!("wrap TokenAuthenticationMiddleware");
        let svc = self.service.clone();
        let args = AppArgs::parse_macro();
        let system_token = args.subscribe_api_auth_token.as_ref().unwrap().as_str();

        // url query 中的 Token
        let req_path_token = {
            let query_list = req.request().query_string().split("&");
            let mut query_list_tmp = Vec::new();
            query_list.into_iter().for_each(|i| {
                let item: Vec<_> = i.split("=").collect();
                let key_name = item.get(0).unwrap();
                if key_name == &"token" {
                    query_list_tmp.push(item.get(1).as_ref().unwrap().to_string())
                }
            });
            let token_one = query_list_tmp.get(0);
            if let Some(d) = token_one {
                d.to_owned()
            } else {
                "".to_owned()
            }
        };
        if system_token == req_path_token {
            return Box::pin(async move {
                let fut = svc.call(req);
                let res = fut.await?;
                Ok(res.map_into_boxed_body().map_into_right_body())
            });
        }

        let header_token = req
            .headers()
            .get("token")
            .unwrap_or(&HeaderValue::from_str("").unwrap())
            .to_str()
            .unwrap()
            .to_string();

        if system_token == header_token {
            return Box::pin(async move {
                let fut = svc.call(req);
                let res = fut.await?;
                Ok(res.map_into_boxed_body().map_into_right_body())
            });
        }

        if req.request().method().as_str() == "POST" {
            Box::pin(async move {
                let body_entity: anyhow::Result<T> = get_request_body_to_entity(&mut req).await;
                return if let Ok(data) = body_entity {
                    if data.get_token().is_some() && data.get_token().unwrap() == system_token {
                        let fut = svc.call(req);
                        let res = fut.await?;
                        Ok(res.map_into_boxed_body().map_into_right_body())
                    } else {
                        let body = EitherBody::new(
                            ApiBaseResponse::<()>::error_by_status_code(
                                403,
                                "Token 错误，无法进行调用",
                            )
                                .to_string(),
                        );

                        let mut res = HttpResponse::with_body(StatusCode::FORBIDDEN, body);
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/json; charset=utf-8"),
                        );
                        let new_resp_s = req.into_parts().0;
                        let new_resp_s = ServiceResponse::new(new_resp_s, res);
                        Ok(new_resp_s.map_into_boxed_body().map_into_right_body())
                    }
                } else {
                    let body = EitherBody::new(
                        ApiBaseResponse::<()>::error_by_status_code(
                            500,
                            "解析请求中的 Token 错误",
                        )
                            .to_string(),
                    );

                    let mut res = HttpResponse::with_body(StatusCode::FORBIDDEN, body);
                    res.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json; charset=utf-8"),
                    );
                    let new_resp_s = req.into_parts().0;
                    let new_resp_s = ServiceResponse::new(new_resp_s, res);
                    Ok(new_resp_s.map_into_boxed_body().map_into_right_body())
                };
            })
        } else {
            Box::pin(async move {
                let body = EitherBody::new(
                    ApiBaseResponse::<()>::error_by_status_code(
                        403,
                        "Token 错误，无法进行调用",
                    )
                        .to_string(),
                );

                let mut res = HttpResponse::with_body(StatusCode::FORBIDDEN, body);
                res.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json; charset=utf-8"),
                );
                let new_resp_s = req.into_parts().0;
                let new_resp_s = ServiceResponse::new(new_resp_s, res);
                Ok(new_resp_s.map_into_boxed_body().map_into_right_body())
            })
        }
    }
}



///////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////   错误处理中间件  //////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone, Default)]
pub struct ErrorHandle;

impl<S, B> Transform<S, ServiceRequest> for ErrorHandle
    where
        S: 'static + Service<ServiceRequest, Response=ServiceResponse<B>, Error=actix_web::Error>,
        S::Future: 'static,
        B: 'static + MessageBody,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Transform = ErrorHandleMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ErrorHandleMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct ErrorHandleMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for ErrorHandleMiddleware<S>
    where
        S: 'static + Service<ServiceRequest, Response=ServiceResponse<B>, Error=actix_web::Error>,
        S::Future: 'static,
        B: 'static + MessageBody,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        info!("wrap ErrorHandleMiddleware");

        let res_call = self.service.call(req);
        Box::pin(async move {
            let res: ServiceResponse<BoxBody> = res_call.await?.map_into_boxed_body();

            if res.status().is_client_error() || res.status().is_server_error() {
                // 处理错误信息（如，序列化json数据出问题）
                let (req, resp) = res.into_parts(); // 将 ServiceResponse 拆分，获取所有权

                let head = resp.head().clone();

                debug!("ErrorHandleMiddleware:: ErrorResponseHead {:?}",&head);

                let status_code = head.status.as_u16() as i32;
                let status_str = head.status.canonical_reason().unwrap().to_string();

                let error_msg = if head.headers.get(header::CONTENT_TYPE).map(AsRef::as_ref)
                    == Some(b"text/plain; charset=utf-8")
                {

                    let body_error_msg = resp.into_body();
                    let body_error_msg_bytes = to_bytes(body_error_msg).await;
                    // let try_bytes = body_error_msg.try_into_bytes();
                    debug!("ErrorHandleMiddleware:: ErrorResponseBody {:?}",&body_error_msg_bytes);
                    match body_error_msg_bytes {
                        Ok(bytes) => Some(String::from_utf8(bytes.to_vec()).unwrap_or("from_utf8 error".to_owned())),
                        Err(err) => {
                            error!("match body_error_msg_bytes {:?}",err);
                            None
                        },
                    }
                } else {
                    None
                };

                // 构造一致的错误响应体
                let new_http_resp = if error_msg.is_some() {
                    let body_err = error_msg.unwrap();
                    warn!("ErrorHandleMiddleware :: HandleError :: {:?} {}",status_code, &body_err);
                    let body = EitherBody::new(
                        ApiBaseResponse::<()>::error_by_status_code(
                            status_code,
                            format!("{}: {}", status_str, body_err).as_str(),
                        )
                            .to_string(),
                    );

                    let mut res = HttpResponse::with_body(StatusCode::OK, body);
                    res.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json; charset=utf-8"),
                    );
                    res
                } else {
                    warn!("ErrorHandleMiddleware :: HandleError :: {:?} {}",status_code, &status_str);
                    let body = EitherBody::new(
                        ApiBaseResponse::<()>::error_by_status_code(
                            status_code,
                            &status_str,
                        )
                            .to_string(),
                    );
                    let mut res = HttpResponse::with_body(StatusCode::OK, body);
                    res.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json; charset=utf-8"),
                    );

                    res
                };
                let new_resp_s = ServiceResponse::new(req, new_http_resp);

                Ok(new_resp_s.map_into_boxed_body().map_into_right_body())
            } else {
                debug!("ErrorHandleMiddleware :: Ignore");
                Ok(res.map_into_right_body())
            }
        })
    }
}