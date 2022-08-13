use std::io;

use anyhow::Result;
use serde::{de, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum EchoCommand {
    Hello(String),
    Message(String, String),
    Goodbye(String),
}

impl EchoCommand {
    pub fn send(&self, stream: &mut dyn io::Write) -> Result<()> {
        send(&self, stream)
    }

    pub fn read(stream: &mut dyn io::Read) -> Result<EchoCommand> {
        read(stream)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum EchoResponse {
    IdAssigned(String),
    EchoResponse(String),
    Goodbye(),
}

impl EchoResponse {
    pub fn send(&self, stream: &mut dyn io::Write) -> Result<()> {
        send(&self, stream)
    }

    pub fn read(stream: &mut dyn io::Read) -> Result<EchoResponse> {
        read(stream)
    }
}

fn send<T: ?Sized>(command: &T, stream: &mut dyn io::Write) -> Result<()>
where
    T: serde::Serialize,
{
    let data: Vec<u8> = bincode::serialize(command)?;
    stream.write_all(&data.len().to_be_bytes())?;
    stream.write_all(&data)?;
    Ok(())
}

fn read<T: de::DeserializeOwned>(stream: &mut dyn io::Read) -> Result<T> {
    let mut length: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
    stream.read_exact(&mut length)?;
    let length = usize::from_be_bytes(length);
    let mut message = vec![0; length];
    stream.read_exact(&mut message)?;
    let data: T = bincode::deserialize(&message)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::EchoCommand;

    #[test]
    fn round_trip_message() {
        for message in [
            EchoCommand::Hello("foo.socket".to_owned()),
            EchoCommand::Message("Hello World".to_owned(), "1".to_string()),
            EchoCommand::Goodbye("1".to_string()),
        ] {
            let mut buff = vec![];
            message.send(&mut buff).unwrap();
            assert_eq!(message, EchoCommand::read(&mut buff.as_slice()).unwrap());
        }
    }
}
