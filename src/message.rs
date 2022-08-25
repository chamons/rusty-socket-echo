use anyhow::Result;
use serde::{de, Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum EchoCommand {
    Hello(String),
    Message(String),
    Goodbye(),
}

impl EchoCommand {
    pub async fn send<W>(&self, stream: &mut W) -> Result<()>
    where
        W: tokio::io::AsyncWrite + std::marker::Unpin,
    {
        send(&self, stream).await
    }

    pub async fn read<R>(stream: &mut R) -> Result<EchoCommand>
    where
        R: tokio::io::AsyncRead + std::marker::Unpin,
    {
        read(stream).await
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum EchoResponse {
    EchoResponse(String),
    Goodbye(),
}

impl EchoResponse {
    pub async fn send<W>(&self, stream: &mut W) -> Result<()>
    where
        W: tokio::io::AsyncWrite + std::marker::Unpin,
    {
        send(&self, stream).await
    }

    pub async fn read<R>(stream: &mut R) -> Result<EchoResponse>
    where
        R: tokio::io::AsyncRead + std::marker::Unpin,
    {
        read(stream).await
    }
}

async fn send<T: ?Sized, W>(command: &T, stream: &mut W) -> Result<()>
where
    T: serde::Serialize,
    W: tokio::io::AsyncWrite + std::marker::Unpin,
{
    let data: Vec<u8> = bincode::serialize(command)?;
    stream.write_all(&data.len().to_be_bytes()).await?;
    stream.write_all(&data).await?;
    Ok(())
}

async fn read<T: de::DeserializeOwned + std::fmt::Debug, R>(stream: &mut R) -> Result<T>
where
    R: tokio::io::AsyncRead + std::marker::Unpin,
{
    let mut length: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
    stream.read_exact(&mut length).await?;
    let length = usize::from_be_bytes(length);
    let mut message = vec![0; length];
    stream.read_exact(&mut message).await?;
    let data: T = bincode::deserialize(&message)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek};

    use super::EchoCommand;

    #[tokio::test]
    async fn round_trip_message() {
        for message in [
            EchoCommand::Hello("foo.socket".to_owned()),
            EchoCommand::Message("Hello World".to_owned()),
            EchoCommand::Goodbye(),
        ] {
            let buff: Cursor<Vec<u8>> = Cursor::new(vec![]);
            tokio::pin!(buff);
            message.send(&mut buff).await.unwrap();
            buff.rewind().unwrap();
            assert_eq!(message, EchoCommand::read(&mut buff).await.unwrap());
        }
    }
}
