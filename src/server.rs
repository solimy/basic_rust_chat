use std::{
    io::{self, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

use crate::model;
use crate::protocol;

pub fn serve(host: String, port: u16) {
    let db = Arc::new(Mutex::new(model::InMemoryDB::new()));
    let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();

    println!("Server listening on {}:{}", host, port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let db_clone = Arc::clone(&db);
                thread::spawn(move || {
                    handle_connection(&mut stream.try_clone().unwrap(), db_clone)
                });
            }
            Err(e) => eprintln!("Accept error: {e}"),
        }
    }
}

fn handle_connection(stream: &mut TcpStream, db: Arc<Mutex<model::InMemoryDB>>) -> io::Result<()> {
    let user_id = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".into());

    connection_opened(stream, Arc::clone(&db), user_id.clone());

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut current_conversation: Box<Option<String>> = Box::new(None);

    loop {
        match protocol::Message::read_message(&mut reader) {
            Ok(protocol::Message::ListRequest(_)) => handle_list_request(&db, stream),

            Ok(protocol::Message::JoinRequest(jr)) => {
                handle_join_request(stream, &db, &user_id, &jr, &mut current_conversation)
            }

            Ok(protocol::Message::ClientText(ct)) => {
                handler_client_text(stream, &db, &user_id, &ct, &mut current_conversation)
            }

            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                break;
            }

            _ => {
                let message: protocol::Message = protocol::Error {
                    message: "Unknown message type.".to_string(),
                }
                .into();
                message.write_message(stream, &message)?;
                Ok(())
            }
        }?;
    }

    if let Some(conv_id) = current_conversation.as_ref() {
        handle_leave(Arc::clone(&db), &user_id, &conv_id)?;
    }
    connection_closed(Arc::clone(&db), &user_id);
    Ok(())
}

fn connection_opened(stream: &mut TcpStream, db: Arc<Mutex<model::InMemoryDB>>, user_id: String) {
    {
        let mut dbm = db.lock().unwrap();
        dbm.connections
            .insert(user_id.clone(), stream.try_clone().unwrap());
    }
    println!("New connection from {}", user_id);
}

fn connection_closed(db: Arc<Mutex<model::InMemoryDB>>, user_id: &String) {
    {
        let mut dbm = db.lock().unwrap();
        dbm.connections.remove(user_id);
    }
    println!("Connection from {} closed.", user_id);
}

fn broadcast_chat(
    db: &Arc<Mutex<model::InMemoryDB>>,
    conv_id: &str,
    text: &str,
    skip_user: Option<&str>,
) -> io::Result<()> {
    let sinks: Vec<TcpStream> = {
        let dbm = db.lock().unwrap();
        let Some(conv) = dbm.conversations.get(conv_id) else {
            return Ok(());
        };
        conv.users
            .iter()
            .filter(|uid| skip_user.map_or(true, |s| *uid != s))
            .filter_map(|uid| dbm.connections.get(uid))
            .filter_map(|s| s.try_clone().ok())
            .collect()
    };

    let chat_message: protocol::Message = protocol::ChatMessage {
        text: text.to_string(),
    }
    .into();
    for mut sink in sinks {
        chat_message.write_message(&mut sink, &chat_message)?;
    }
    Ok(())
}

fn handle_leave(
    db: Arc<Mutex<model::InMemoryDB>>,
    user_id: &String,
    conversation_id: &String,
) -> io::Result<()> {
    broadcast_chat(
        &db,
        conversation_id,
        &format!(
            "User {} has left the conversation {}",
            user_id, conversation_id
        ),
        Some(user_id),
    )?;

    {
        let mut dbm = db.lock().unwrap();
        if let Some(conv) = dbm.conversations.get_mut(conversation_id) {
            conv.users.retain(|uid| uid != user_id);
            if conv.users.is_empty() {
                println!(
                    "No more users in conversation {}. Deleting conversation.",
                    conversation_id
                );
                dbm.conversations.remove(conversation_id);
            }
        }
        dbm.connections.remove(user_id);
    }
    Ok(())
}

fn handle_list_request(
    db: &Arc<Mutex<model::InMemoryDB>>,
    stream: &mut TcpStream,
) -> io::Result<()> {
    let summaries: Vec<protocol::ConversationSummary> = {
        let db = db.lock().unwrap();
        db.conversations
            .iter()
            .map(|(id, conv)| protocol::ConversationSummary {
                id: id.clone(),
                user_count: conv.users.len() as u32,
                message_count: conv.messages.len() as u32,
            })
            .collect()
    };
    let message: protocol::Message = protocol::ListResponse {
        conversations: summaries,
    }
    .into();

    message.write_message(stream, &message)?;
    Ok(())
}

fn handle_join_request(
    stream: &mut TcpStream,
    db: &Arc<Mutex<model::InMemoryDB>>,
    user_id: &String,
    jr: &protocol::JoinRequest,
    current_conversation: &mut Box<Option<String>>,
) -> io::Result<()> {
    let conv_id = jr.conversation_id.clone();
    println!("User {user_id} joining {conv_id}");

    {
        let mut dbm = db.lock().unwrap();
        let conv = dbm
            .conversations
            .entry(conv_id.clone())
            .or_insert_with(|| model::Conversation::new(user_id.clone()));
        if !conv.users.contains(&user_id) {
            conv.users.push(user_id.clone());
        }
    }

    let history: Vec<String> = {
        let dbm = db.lock().unwrap();
        dbm.conversations
            .get(&conv_id)
            .map(|c| {
                c.messages
                    .iter()
                    .map(|m| format!("[{}] {}: {}", conv_id, m.sender, m.content))
                    .collect()
            })
            .unwrap_or_default()
    };
    for line in history {
        let message: protocol::Message = protocol::ChatMessage { text: line }.into();
        message.write_message(stream, &message)?;
    }

    broadcast_chat(
        &db,
        &conv_id,
        &format!("User {} has joined the conversation {}", user_id, conv_id),
        Some(&user_id),
    )?;

    current_conversation.replace(conv_id);
    Ok(())
}

fn handler_client_text(
    stream: &mut TcpStream,
    db: &Arc<Mutex<model::InMemoryDB>>,
    user_id: &String,
    ct: &protocol::ClientText,
    current_conversation: &Box<Option<String>>,
) -> io::Result<()> {
    let Some(conv_id) = current_conversation.as_ref() else {
        let message: protocol::Message = protocol::Error {
            message: "You must join a conversation before sending messages.".to_string(),
        }
        .into();
        message.write_message(stream, &message)?;
        return Ok(());
    };

    let text = ct.text.clone();

    {
        let mut dbm = db.lock().unwrap();
        let conv = dbm.conversations.get_mut(conv_id).unwrap();
        conv.add_message(model::Message::new(user_id.clone(), text.to_string()));
    }

    broadcast_chat(
        &db,
        &conv_id,
        &format!("[{}] {}: {}", conv_id, user_id, text),
        None,
    )?;

    println!("[{}] {}: {}", conv_id, user_id, text);
    Ok(())
}
