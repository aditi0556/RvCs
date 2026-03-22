use async_trait::async_trait;
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use futures::{AsyncReadExt, AsyncWriteExt};
use std::io;
#[derive(Default,Clone)]
pub struct RvcCodec;

#[derive(Debug, Clone)]
pub struct RvcRequest(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct RvcResponse(pub Vec<u8>);

#[async_trait]
impl Codec for RvcCodec {
    type Protocol = StreamProtocol;
    type Request = RvcRequest;
    type Response = RvcResponse;

    async fn read_request<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncReadExt + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        Ok(RvcRequest(buf))
    }

    async fn read_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncReadExt + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        Ok(RvcResponse(buf))
    }

    async fn write_request<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        RvcRequest(data): RvcRequest,
    ) -> io::Result<()>
    where
        T: AsyncWriteExt + Unpin + Send,
    {
        io.write_all(&data).await
    }

    async fn write_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        RvcResponse(data): RvcResponse,
    ) -> io::Result<()>
    where
        T: AsyncWriteExt + Unpin + Send,
    {
        io.write_all(&data).await
    }
}