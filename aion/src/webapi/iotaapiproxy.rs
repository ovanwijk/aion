use crate::timewarping::{Protocol, WebRequestType};
//use std::future::Future;
use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
use crate::APIActors;
use crate::SETTINGS;
use crate::webapi::ReturnData;
use serde::{Serialize, Deserialize};




// #[post("/")]
// pub async fn store_storage_object_fn(postData: String, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
//     return Ok()
// }