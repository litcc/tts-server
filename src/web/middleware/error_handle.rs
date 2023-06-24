use std::{
    future::{ready, Ready},
    rc::Rc,
};

use actix_http::body::{to_bytes, BoxBody};
use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{header, header::HeaderValue},
    HttpResponse,
};
use futures::future::LocalBoxFuture;
use log::{debug, error, info, warn};


use crate::web::vo::{BaseResponse, EmptyVO};

/// Actix 错误处理中间件
#[derive(Debug, Clone, Default)]
pub(crate) struct ErrorHandle;

impl<S, B> Transform<S, ServiceRequest> for ErrorHandle
where
    S: 'static + Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
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

pub(crate) struct ErrorHandleMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for ErrorHandleMiddleware<S>
where
    S: 'static + Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
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

                debug!("ErrorHandleMiddleware:: ErrorResponseHead {:?}", &head);

                let content_type = head.headers.get(header::CONTENT_TYPE).map(AsRef::as_ref);

                let error_body = if content_type.is_none()
                    || content_type == Some(b"text/plain; charset=utf-8")
                {
                    let status_code = head.status.as_u16() as i32;
                    let status_str = head.status.canonical_reason().unwrap().to_string();
                    let body_error_msg_bytes = to_bytes(resp.into_body()).await;
                    debug!(
                        "ErrorHandleMiddleware:: ErrorResponseBody {:?}",
                        &body_error_msg_bytes
                    );
                    match body_error_msg_bytes {
                        Ok(bytes) => {
                            let body_err = String::from_utf8(bytes.to_vec())
                                .unwrap_or("from_utf8 error".to_owned());
                            warn!(
                                "ErrorHandleMiddleware :: HandleError :: {:?} {} {}",
                                status_code,
                                &body_err,
                                &req.path()
                            );
                            let body = EitherBody::new(
                                BaseResponse::<EmptyVO>::from_error_info::<String>(
                                    head.status.into(),
                                    if body_err.is_empty() {
                                        status_str
                                    } else {
                                        body_err
                                    },
                                )
                                .to_string(),
                            );

                            let mut res = HttpResponse::with_body(head.status, body);
                            res.headers_mut().insert(
                                header::CONTENT_TYPE,
                                HeaderValue::from_static("application/json; charset=utf-8"),
                            );

                            ServiceResponse::new(req, res)
                        }
                        Err(err) => {
                            error!("match body_error_msg_bytes {:?}", err);
                            let r = BaseResponse::<EmptyVO>::from_error_info::<String>(
                                head.status.into(),
                                status_str,
                            )
                            .http_resp();
                            ServiceResponse::new(req, r.map_into_boxed_body().map_into_right_body())
                        }
                    }
                } else {
                    ServiceResponse::new(req, resp.map_into_boxed_body().map_into_right_body())
                };

                Ok(error_body.map_into_boxed_body().map_into_right_body())
            } else {
                debug!("ErrorHandleMiddleware :: Ignore");
                Ok(res.map_into_right_body())
            }
        })
    }
}
