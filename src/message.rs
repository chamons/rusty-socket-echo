use std::io;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum EchoCommand {
    Hello,
    Message(String),
    Goodbye,
}

impl EchoCommand {
    pub fn send(&self, stream: &mut dyn io::Write) -> Result<()> {
        let data: Vec<u8> = bincode::serialize(self).unwrap();
        stream.write(&data.len().to_be_bytes())?;
        stream.write_all(&data)?;
        Ok(())
    }

    pub fn read(stream: &mut dyn io::Read) -> Result<EchoCommand> {
        let mut length: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
        stream.read_exact(&mut length)?;
        let length = usize::from_be_bytes(length);
        let mut message = vec![0; length];
        stream.read_exact(&mut message)?;
        Ok(bincode::deserialize(&message)?)
    }
}

#[cfg(test)]
mod tests {
    use super::EchoCommand;

    #[test]
    fn round_trip_message() {
        for message in [EchoCommand::Hello, EchoCommand::Message("Hello World".to_owned()), EchoCommand::Goodbye] {
            let mut buff = vec![];
            message.send(&mut buff).unwrap();
            assert_eq!(message, EchoCommand::read(&mut buff.as_slice()).unwrap());
        }
    }
}
