use std::collections::HashMap;
use std::net::TcpStream;

// Type alias for UserId -- user's unique identifier as a String (ip address of the user)
pub type UserId = String;

// Message struct representing a message in a conversation
#[derive(Debug)]
pub struct Message {
    // UserId of the sender
    pub sender: UserId,
    // Content of the message
    pub content: String,
}

// Type alias for ConversationId -- conversation's unique identifier as a String (name of the conversation)
pub type ConversationId = String;

// Conversation struct representing a chat conversation
#[derive(Debug)]
pub struct Conversation {
    // List of UserIds participating in the conversation
    pub users: Vec<UserId>,
    // List of messages in the conversation
    pub messages: Vec<Message>,
}

// In-memory database to store conversations
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
