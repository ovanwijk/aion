
extern crate actix_web;

extern crate serde_derive;
extern crate zmq;
extern crate serde;
extern crate json;
extern crate serde_json;

use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, Responder};


use json::JsonValue;
use serde_derive::{Deserialize, Serialize};
//fn index(info: web::Path<(u32, String)>) -> impl Responder {
//    format!("Hello {}! id:{}", info.1, info.0)
//}


#[derive(Serialize, Deserialize, Debug)]
struct MyObj {
    name: String,
    number: i32,
}

/// This handler uses json extractor
fn index(item: web::Json<MyObj>) -> HttpResponse {
    println!("model: {:?}", &item);
    HttpResponse::Ok().json(item.0) // <- send response
}

fn main() -> std::io::Result<()> {



    HttpServer::new(
        || App::new().service(
              web::resource("/{id}/{name}/index.html").to(index)))
        .bind("127.0.0.1:8080")?
        .run()


}