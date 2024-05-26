use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::{request_response, StreamProtocol};
use prost::Message;
use std::io;

// TODO check what these should be
/// Max request size in bytes
const REQUEST_SIZE_MAXIMUM: u64 = 1024 * 1024;
/// Max response size in bytes
const RESPONSE_SIZE_MAXIMUM: u64 = 10 * 1024 * 1024;

pub mod messages {
    include!(concat!(env!("OUT_DIR"), "/waku.metadata.rs"));
}

pub const PROTOCOL_NAME: &str = "/vac/waku/metadata/1.0.0";

pub use messages::*;

#[derive(Clone, Default)]
pub struct Codec {}

#[async_trait]
impl request_response::Codec for Codec {
    type Protocol = StreamProtocol;
    type Request = messages::WakuMetadataRequest;
    type Response = messages::WakuMetadataResponse;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<messages::WakuMetadataRequest>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();

        io.take(REQUEST_SIZE_MAXIMUM).read_to_end(&mut vec).await?;

        let request = messages::WakuMetadataRequest::decode(&vec[..])?;
        Ok(request)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<messages::WakuMetadataResponse>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();

        io.take(RESPONSE_SIZE_MAXIMUM).read_to_end(&mut vec).await?;
        let response = messages::WakuMetadataResponse::decode(&vec[..])?;
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
        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf)?;

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
        let mut buf = Vec::with_capacity(resp.encoded_len());
        buf.reserve(resp.encoded_len());
        resp.encode(&mut buf)?;

        io.write_all(buf.as_ref()).await?;

        Ok(())
    }
}
