use std::collections::HashMap;
use std::net::TcpStream;

pub type UserId = String;

#[derive(Debug)]
pub struct Message {
    pub sender: UserId,
    pub content: String,
}

pub type ConversationId = String;

#[derive(Debug)]
pub struct Conversation {
    pub users: Vec<UserId>,
    pub messages: Vec<Message>,
}

#[derive(Debug)]
pub struct InMemoryDB {
    pub conversations: HashMap<ConversationId, Conversation>,
    pub connections: HashMap<UserId, TcpStream>,
}

impl Message {
    pub fn new(sender: String, content: String) -> Self {
        Message { sender, content }
    }
}

impl Conversation {
    pub fn new(first_user: UserId) -> Self {
        Conversation {
            users: vec![first_user],
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }
}

impl InMemoryDB {
    pub fn new() -> Self {
        InMemoryDB {
            conversations: HashMap::new(),
            connections: HashMap::new(),
        }
    }
}
