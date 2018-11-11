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

pub struct DnsPacket {
    data: Bytes,
    tid: [u8;2],
}

impl DnsPacket {
    pub fn from(buffer: Bytes) -> DnsPacket {
        let mut tid: [u8;2] = [0;2];
        tid.copy_from_slice(&buffer[0..2]);
        DnsPacket{data: buffer, tid}
    }

    pub fn from_tid(buffer: Bytes, tid: [u8;2]) -> DnsPacket {
        let mut buffer = BytesMut::from(buffer);
        buffer[0] = tid[0];
        buffer[1] = tid[1];

        DnsPacket{data: buffer.freeze(), tid}
    }

//    fn parser(buffer: Bytes) -> DnsPacket {
//
//    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get_tid(&self) -> [u8;2] {
        self.tid.clone()
    }

    pub fn get_without_tid(&self) -> Bytes {
        let mut data: BytesMut = BytesMut::with_capacity(self.data.len());
        data.extend(self.data.iter());
        data[0] = b'\0';
        data[1] = b'\0';

        data.freeze()
    }

    pub fn get(&self) -> Bytes {
        self.data.clone()
    }
}

impl DnsCodec {
    pub fn new(listen_addr: SocketAddr) -> Result<(SplitSink<UdpFramed<DnsCodec>>, SplitStream<UdpFramed<DnsCodec>>), Error> {
        return match UdpSocket::bind(&listen_addr) {
            Ok(socket) => Ok(UdpFramed::new(socket, DnsCodec).split()),
            Err(e) => Err(e)
        };
    }
}

impl Decoder for DnsCodec {
    type Item = DnsPacket;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<DnsPacket>, io::Error> {
        if buf.len() > 0 {
            Ok(Some(DnsPacket::from(buf.clone().freeze())))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for DnsCodec {
    type Item = DnsPacket;
    type Error = io::Error;

    fn encode(&mut self, data: DnsPacket, buf: &mut BytesMut) -> Result<(), io::Error> {
        buf.clear();
        buf.reserve(data.len());
        let b: Bytes = data.get();
        buf.extend(b);
        Ok(())
    }
}