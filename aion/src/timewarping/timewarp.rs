extern crate zmq as zmq;
use zmq::*;


mod timewarping {
    fn start() {
        println!("Hello, world!");

        let ctx = zmq::Context::new();

        let socket = ctx.socket(zmq::STREAM).unwrap();
        socket.bind("tcp://*:8888").unwrap();
        loop {
            let data = socket.recv_multipart(0).unwrap();
            println!(
                "Identity: {:?} Message : {}",
                data[0],
                str::from_utf8(&data[1]).unwrap()
            );
        }
    }
}