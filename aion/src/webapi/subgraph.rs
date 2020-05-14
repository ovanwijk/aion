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


#[get("/subgraph/connect/{start}/{end}")]
pub async fn subgraphConnectFn(info: web::Path<(String, String)>,data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    //let r = crate::indexstorage::get_lastest_known_timewarps(data.storage.clone());
    let start = data.storage.get_lifeline_tx(&info.0);
    let end = data.storage.get_lifeline_tx(&info.1);
    if start.is_none() || end.is_none() {
        return Ok(HttpResponse::BadRequest().body("{\"error\" : \"Start or end not a  lifeline transaction\"}"));
    }
    let start_un = start.unwrap();
    let end_un = end.unwrap();

    



    let r = data.storage.clone_state();
    Ok(HttpResponse::Ok()
    .content_type("application/json")
    .body( serde_json::to_string_pretty(&ReturnData {
        data: r
    }).unwrap()))
}