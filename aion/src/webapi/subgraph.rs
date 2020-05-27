use actix_web::{
    web, Error, HttpRequest, HttpResponse,// HttpServer, Responder,
    Result,
};
use crate::APIActors;
use crate::SETTINGS;
use crate::webapi::ReturnData;
use serde::{Serialize, Deserialize};




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

// #[get("/subgraph/force_entry")]
// pub async fn forceEntryFn(data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
//     //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
//     let r = data.storage.get_last_lifeline();
//     //let r = data.storage.clone_state();
//     Ok(HttpResponse::Ok()
//     .content_type("application/json")
//     .body( serde_json::to_string_pretty(&ReturnData {
//         data: timewarp_path
//     }).unwrap()))
// }


#[get("/subgraph/connect/{start}/{end}")]
pub async fn subgraphConnectFn(info: web::Path<(String, String)>,data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let start = match data.storage.get_lifeline_tx(&info.0) {
        Some(v) => v,
        None => return Ok(HttpResponse::BadRequest().body("{\"error\" : \"Start is not a lifeline transaction\"}"))
    };
    let end = match data.storage.get_lifeline_tx(&info.1) {
        Some(v) => v,
        None => return Ok(HttpResponse::BadRequest().body("{\"error\" : \"End is not a lifeline transaction\"}"))
    };
    //let subgraph = data.storage.LIFELINE_SUBGRAPH.lock();
  

    info!("Calling subgraph connect");

    let timewarp_path = data.storage.get_subgraph_path(info.0.clone(), info.1.clone()).expect("To return a result");
    



    //let r = data.storage.clone_state();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: timewarp_path
    }).unwrap()))
}