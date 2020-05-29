//use actix_web::http::{header, Method, StatusCode};
use crate::timewarping::{Protocol, WebRequestType};
//use std::future::Future;
use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
//use crate::indexstorage::{get_time_key, TIMEWARP_ID_PREFIX, TimewarpData};
use serde::{Serialize, Deserialize};
//use iota_client;
use crate::APIActors;
use crate::SETTINGS;
use iota_lib_rs::prelude::*;
use iota_model::Transaction;
pub mod webask;
pub mod lifeline;
pub mod storage;
pub mod subgraph;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReturnData<T> {
    data:T
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AIONStatus {
    code: i64,
    text: String,
    problems: Vec<String>,    
    latest_milestone_timestamp: i64,
    latest_milestone_index: i64,
    transactions_pinned: i64,
    transactions_stored: i64,
    last_lifeline: i64,
    pin_jobs: i64,
    faulty_pin_jobs: i64,
    active_timewarps: i64,
    inactive_timewarps: i64

}


#[get("/status")]
pub async fn aionStatusFn(req:HttpRequest, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let a  = data.storage.get_last_lifeline();
    let all_started = &data.storage.get_generic_cache(crate::indexstorage::ALL_STARTED).unwrap_or_default();
    let (code, text) = if std::str::from_utf8(all_started).unwrap() == "true" { (0, "Ok")} else { (1, "Starting")};
    let lmi = data.storage.get_generic_cache(crate::indexstorage::LMI_CONST).unwrap_or_default();
    let lm_time = data.storage.get_generic_cache(crate::indexstorage::LMI_TIME).unwrap_or_default();
    let pin_jobs:Vec<String> = match data.storage.get_generic_cache(crate::indexstorage::P_CACHE_PULLJOB) {
        Some(value) => serde_json::from_slice(&*value).unwrap(),
        None => vec!()
       };
    let faulty_pin_jobs:Vec<String> = data.storage.list_faulty_pull_jobs();

    let timewarps = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let mut active_timewarps = 0;
    let mut inactive_timewarps = 0;
    for tw in timewarps.iter() {
        if tw.timestamp > crate::now() - (tw.avg_distance * 5) {
            active_timewarps += 1;
        }else {
            inactive_timewarps += 1;
        }
    }
    if a.is_some(){
       
        Ok(HttpResponse::Ok()
 .content_type("application/json")
 .body( serde_json::to_string_pretty(&AIONStatus {
    code: code,
    text: text.to_string(),
    problems: vec!(),
    latest_milestone_timestamp: crate::read_be_i64(&mut lm_time.as_slice()),
    latest_milestone_index: crate::read_be_i64(&mut lmi.as_slice()),
    transactions_pinned: 0,
    transactions_stored: 0,
    last_lifeline: a.unwrap().timestamp,
    pin_jobs: pin_jobs.len() as i64,
    faulty_pin_jobs: faulty_pin_jobs.len() as i64,
    active_timewarps: active_timewarps,
    inactive_timewarps: inactive_timewarps


 }).unwrap()))
    }else{
        Ok(HttpResponse::Ok().body("{}"))
    }
    
}


//, data: web::Data<APIActors>
#[get("/timewarpstate")]
pub async fn timewarpstateFn(req:HttpRequest, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let a  = data.storage.get_timewarp_state();
    if a.is_some(){
        let mut a_ = a.unwrap();
        a_.latest_private_key = vec!();
        a_.seed = String::default();
        Ok(HttpResponse::Ok()
 .content_type("application/json")
 .body( serde_json::to_string_pretty(&a_).unwrap()))
    }else{
        Ok(HttpResponse::Ok().body("{}"))
    }
    
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
    let r = data.storage.tw_detection_get_decision_data(info.to_string());
    //let r = crate::indexstorage::get_n_timewarp_transactions(info.to_string(), 15, data.storage.clone());
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














 