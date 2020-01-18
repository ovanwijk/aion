use crate::timewarping::{Protocol, WebRequestType};
//use std::future::Future;
use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
use crate::APIActors;
use crate::SETTINGS;
use crate::webapi::ReturnData;

#[get("/lifeline")]
pub async fn lifelineLatestF(data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = data.storage.get_last_lifeline();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}

#[get("/lifeline/unpinned")]
pub async fn lifelineUnpinnedF(data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = data.storage.get_unpinned_lifeline();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}



#[get("/lifeline/{id}")]
pub async fn lifelineIdFn(info: web::Path<String>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = data.storage.get_lifeline_tx(&info);
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}