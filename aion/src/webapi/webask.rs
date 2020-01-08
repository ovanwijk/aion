use crate::timewarping::Protocol;


use std::sync::{Arc, Mutex};

use futures::channel::oneshot::{channel, Sender as ChannelSender};

use futures::FutureExt;

use riker::actors::*;

/// Convenience fuction to send and receive a message from an actor
///
/// This function sends a message `msg` to the provided actor `receiver`
/// and returns a `Future` which will be completed when `receiver` replies
/// by sending a message to the `sender`. The sender is a temporary actor
/// that fulfills the `Future` upon receiving the reply.
///
/// `futures::future::RemoteHandle` is the future returned and the task
/// is executed on the provided executor `ctx`.
///
/// This pattern is especially useful for interacting with actors from outside
/// of the actor system, such as sending data from HTTP request to an actor
/// and returning a future to the HTTP response, or using await.
///


pub async fn ask<T>(ctx: Arc<ActorSystem>, receiver: &T, msg: Protocol) -> Protocol
where
 
    T: Tell<Protocol>,
{
    let (tx, rx) = channel::<Protocol>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let props = Props::new_args(Box::new(AskActor::new), tx);
    let actor = ctx.tmp_actor_of(props).unwrap();
    receiver.tell(msg, Some(actor.into()));
   
    ctx.run(rx.map(|r| r.unwrap())).unwrap().await
    
}

struct AskActor<Msg> {
    tx: Arc<Mutex<Option<ChannelSender<Msg>>>>,
}

impl<Msg: Message> AskActor<Msg> {
    fn new(tx: Arc<Mutex<Option<ChannelSender<Msg>>>>) -> BoxActor<Msg> {
        let ask = AskActor { tx };
        Box::new(ask)
    }
}

impl<Msg: Message> Actor for AskActor<Msg> {
    type Msg = Msg;

    fn recv(&mut self, ctx: &Context<Msg>, msg: Msg, _: Sender) {
        if let Ok(mut tx) = self.tx.lock() {
            tx.take().unwrap().send(msg).unwrap();
        }
        ctx.stop(&ctx.myself);
    }
}