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


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifelineSyncRequest {
    start: String,
    end: String,
}

#[post("/lifeline/create_pull_request")]
pub async fn lifelineConnectF(info: web::Json<LifelineSyncRequest>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let st = data.storage.clone();
    let start_tx = st.get_lifeline_tx(&info.start);
    let end_tx = st.get_lifeline_tx(&info.end);
    if start_tx.is_none() || end_tx.is_none() {
        return Ok(HttpResponse::NotFound()
        .content_type("application/json")
        .body("{\"status\":\"Start or end transaction is not a known lifeline transaction.\"}"));
    }
    let un_start = start_tx.unwrap();
    let un_end = end_tx.unwrap();
    if un_start.timestamp >= un_end.timestamp {
        return Ok(HttpResponse::NotFound()
        .content_type("application/json")
        .body("{\"status\":\"End must be older that start.\"}"));
    }
    let r = data.storage.get_last_lifeline();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}


#[get("/subgraph")]
pub async fn subgraphFn(data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = data.storage.clone_state();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}

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


#[get("/lifeline/ts/{timewarmp}")]
pub async fn lifelineTsFn(info: web::Path<i64>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = data.storage.get_lifeline_ts(&info);
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}



#[get("/lifeline/ts_index/{timewarmp}")]
pub async fn lifelineIndexFn(info: web::Path<i64>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = data.storage.get_lifeline(&crate::indexstorage::get_time_key(&info));
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