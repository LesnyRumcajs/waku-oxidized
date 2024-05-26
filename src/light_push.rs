use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::{request_response, StreamProtocol};
use prost::Message;
use std::io;

// TODO check what these should be
const MAX_LIGHTPUSH_RPC_SIZE: u64 = 1024 * 1024 * 1024;

pub mod messages {
    include!(concat!(env!("OUT_DIR"), "/waku.lightpush.rs"));
}

pub mod message {
    include!(concat!(env!("OUT_DIR"), "/waku.message.rs"));
}

pub const PROTOCOL_NAME: &str = "/vac/waku/lightpush/2.0.0-beta1";

#[derive(Clone, Default)]
pub struct Codec {}

#[async_trait]
impl request_response::Codec for Codec {
    type Protocol = StreamProtocol;
    type Request = messages::PushRpc;
    type Response = messages::PushRpc;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::with_capacity(MAX_LIGHTPUSH_RPC_SIZE as usize);
        io.read_to_end(&mut vec).await?;
        let request = Self::Request::decode_length_delimited(&vec[..])?;
        Ok(request)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::with_capacity(MAX_LIGHTPUSH_RPC_SIZE as usize);

        io.read_to_end(&mut vec).await?;
        let response = Self::Response::decode_length_delimited(&vec[..])?;
        Ok(response)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buf = req.encode_length_delimited_to_vec();
        io.write_all(buf.as_ref()).await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buf = resp.encode_length_delimited_to_vec();
        io.write_all(buf.as_ref()).await?;

        Ok(())
    }
}
