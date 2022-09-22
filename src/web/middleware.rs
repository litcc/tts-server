use std::future::{ready, Ready};
use std::marker::PhantomData;
use std::rc::Rc;
use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{HttpResponse};
use actix_web::http::{header, StatusCode};
use actix_web::http::header::HeaderValue;
use futures::future::LocalBoxFuture;
use log::info;
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
