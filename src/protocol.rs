use std::io::{self, Read, Write};

use bincode::{Decode, Encode, decode_from_std_read, encode_into_std_write};
use derive_more::derive::From;

#[derive(Encode, Decode)]
pub struct ListRequest {}

#[derive(Encode, Decode)]
pub struct ConversationSummary {
    pub id: String,
    pub user_count: u32,
    pub message_count: u32,
}
#[derive(Encode, Decode)]
pub struct ListResponse {
    pub conversations: Vec<ConversationSummary>,
}

#[derive(Encode, Decode)]
pub struct JoinRequest {
    pub conversation_id: String,
}

#[derive(Encode, Decode)]
pub struct ClientText {
    pub text: String,
}

#[derive(Encode, Decode)]
pub struct ChatMessage {
    pub text: String,
}

#[derive(Encode, Decode)]
pub struct Error {
    pub message: String,
}

#[derive(Encode, Decode, From)]
pub enum Message {
    ListRequest(ListRequest),
    ListResponse(ListResponse),
    JoinRequest(JoinRequest),
    ClientText(ClientText),
    ChatMessage(ChatMessage),
    Error(Error),
}

impl Message {
    pub fn write_message<W: Write>(&self, stream: &mut W, payload: &Message) -> io::Result<()> {
        encode_into_std_write(payload, stream, bincode::config::standard()).map_err(
            |e| match e {
                bincode::error::EncodeError::Io { inner, .. }
                    if inner.kind() == io::ErrorKind::UnexpectedEof =>
                {
                    inner
                }
                e => io::Error::new(io::ErrorKind::Other, format!("Encoding error: {}", e)),
            },
        )?;
        Ok(())
    }

    pub fn read_message<R: Read>(reader: &mut R) -> io::Result<Message> {
        let message: Message =
            decode_from_std_read(reader, bincode::config::standard()).map_err(|e| match e {
                bincode::error::DecodeError::Io { inner, .. }
                    if inner.kind() == io::ErrorKind::UnexpectedEof =>
                {
                    inner
                }
                e => io::Error::new(io::ErrorKind::Other, format!("Decoding error: {}", e)),
            })?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_serialization_ok() {
        let original: Message = ChatMessage {
            text: String::from("Hello, world!"),
        }
        .into();
        let mut buffer: Vec<u8> = Vec::new();
        original.write_message(&mut buffer, &original).unwrap();
        let mut reader = BufReader::new(&buffer[..]);
        let deserialized = Message::read_message(&mut reader).unwrap();
        match deserialized {
            Message::ChatMessage(chat_msg) => {
                assert_eq!(chat_msg.text, "Hello, world!");
            }
            _ => panic!("Expected ChatMessage variant"),
        }
    }
}
