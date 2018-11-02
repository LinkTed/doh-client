use std::io;
use std::io::Error;
use std::net::SocketAddr;

use futures::stream::{SplitSink, SplitStream};
use futures::Stream;

use tokio::codec::{Decoder, Encoder};
use tokio::net::{UdpSocket, UdpFramed};

use bytes::{Bytes, BytesMut};


#[derive(Debug)]
pub struct DnsCodec;

impl DnsCodec {
    pub fn new(listen_addr: SocketAddr) -> Result<(SplitSink<UdpFramed<DnsCodec>>, SplitStream<UdpFramed<DnsCodec>>), Error> {
        return match UdpSocket::bind(&listen_addr) {
            Ok(socket) => Ok(UdpFramed::new(socket, DnsCodec).split()),
            Err(e) => Err(e)
        };
    }
}

impl Decoder for DnsCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<BytesMut>, io::Error> {
        if buf.len() > 0 {
            let len = buf.len();
            Ok(Some(buf.split_to(len)))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for DnsCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn encode(&mut self, data: Bytes, buf: &mut BytesMut) -> Result<(), io::Error> {
        buf.reserve(data.len());
        buf.extend(data);
        Ok(())
    }
}