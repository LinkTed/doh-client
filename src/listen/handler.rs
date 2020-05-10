use bytes::Bytes;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::stream::StreamExt;

use std::net::SocketAddr;

use tokio::net::udp::{RecvHalf, SendHalf};
use tokio::net::UdpSocket;
use tokio::spawn;

async fn send_handler(mut receiver: UnboundedReceiver<(Bytes, SocketAddr)>, mut send: SendHalf) {
    while let Some((msg, socket_addr)) = receiver.next().await {
        if let Err(e) = send.send_to(&msg, &socket_addr).await {
            error!("Could not send reponse to {}: {}", socket_addr, e);
            return;
        }
    }
}

pub(crate) fn handler(socket: UdpSocket) -> (RecvHalf, UnboundedSender<(Bytes, SocketAddr)>) {
    let (sender, receiver) = unbounded::<(Bytes, SocketAddr)>();
    let (recv, send) = socket.split();

    spawn(send_handler(receiver, send));

    (recv, sender)
}
