use actix_web::http::{header, Method, StatusCode};
use std::future::Future;
use actix_web::{
    error, guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
    Result,
};

use crate::APIActors;

//, data: web::Data<APIActors>
#[get("/test")]
pub async fn index(req:HttpRequest, data: web::Data<APIActors>) ->  Result<HttpResponse, Error>   {
    let a  = data.storage.get_timewarp_state();
    if(a.is_some()){
        let mut a_ = a.unwrap();
        a_.latest_private_key = vec!();
        Ok(HttpResponse::Ok()
 .content_type("application/json")
 .body( serde_json::to_string_pretty(&a_).unwrap()))
    }else{
        Ok(HttpResponse::Ok().body("{}"))
    }
    
}

 