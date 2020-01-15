//use actix_web::http::{header, Method, StatusCode};
use crate::timewarping::{Protocol, WebRequestType};
//use std::future::Future;
use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
//use crate::indexstorage::{get_time_key, TIMEWARP_ID_PREFIX, TimewarpData};
use serde::{Serialize, Deserialize};
use crate::APIActors;
pub mod webask;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReturnData<T> {
    data:T
}


//, data: web::Data<APIActors>
#[get("/timewarpstate")]
pub async fn timewarpstateFn(req:HttpRequest, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let a  = data.storage.get_timewarp_state();
    if a.is_some(){
        let mut a_ = a.unwrap();
        a_.latest_private_key = vec!();
        Ok(HttpResponse::Ok()
 .content_type("application/json")
 .body( serde_json::to_string_pretty(&a_).unwrap()))
    }else{
        Ok(HttpResponse::Ok().body("{}"))
    }
    
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
    let r = data.storage.get_lifeline_tx(info.to_string());
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}


#[get("/timewarp")]
pub async fn timewarpsFn(req:HttpRequest, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    
    //let pong = webask::ask(data.actor_system.clone(), &data.tw_selecting.as_ref().unwrap().clone(), Protocol::Ping).await;
    //info!("{:?}", pong);
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&r).unwrap()))   
}


#[get("/timewarp/picked")]
pub async fn timewarpPickedFn(req:HttpRequest, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    
    let reply = webask::ask(data.actor_system.clone(), &data.tw_selecting.clone(), Protocol::WebRequest(WebRequestType::PickedTimewarp)).await;
    info!("{:?}", reply);
    match reply {
        Protocol::WebReply(__msg) => {
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body( __msg)) 
        },
        
        _ => {
            Ok(HttpResponse::NotFound()
        .content_type("application/json")
        .body("{}"))
        }
    }
     
}


#[get("/timewarp/{id}")]
pub async fn timewarpIdFn(info: web::Path<String>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = crate::indexstorage::get_n_timewarp_transactions(info.to_string(), 15, data.storage.clone());
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}

#[get("/timewarp/{id}/{max}")]
pub async fn timewarpIdMaxFn(info: web::Path<(String, i32)>, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let r = crate::indexstorage::get_n_timewarp_transactions(info.0.to_string(), std::cmp::min(100, info.1), data.storage.clone());
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}














 