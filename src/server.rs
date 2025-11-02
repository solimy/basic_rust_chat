use std::{
    io::{self, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

use crate::model;

use crate::protocol::{chat, helpers::common::*, helpers::server::*};

use flatbuffers::FlatBufferBuilder;

pub fn serve(host: String, port: u16) {
    let db = Arc::new(Mutex::new(model::InMemoryDB::new()));
    let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();

    println!("Server listening on {}:{}", host, port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let db_clone = Arc::clone(&db);
                thread::spawn(move || handle_connection(stream, db_clone));
            }
            Err(e) => eprintln!("Accept error: {e}"),
        }
    }
}

fn handle_connection(stream: TcpStream, db: Arc<Mutex<model::InMemoryDB>>) {
    let user_id = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".into());

    connection_opened(
        stream.try_clone().unwrap(),
        Arc::clone(&db),
        user_id.clone(),
    );

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut current_conversation: Box<Option<String>> = Box::new(None);

    loop {
        let frame_bytes = match read_frame(&mut reader) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                eprintln!("Read error from {user_id}: {e}");
                break;
            }
        };

        let env = match chat::root_as_envelope(&frame_bytes) {
            Ok(e) => e,
            Err(e) => {
                if let Ok(mut s) = stream.try_clone() {
                    let mut fbb = FlatBufferBuilder::new();
                    let env = env_with_error(&mut fbb, &format!("Bad envelope: {e}"));
                    fbb.finish(env, None);
                    let _ = write_frame(&mut s, fbb.finished_data());
                }
                continue;
            }
        };

        match env.frame_type() {
            chat::Message::ListRequest => handle_list_request(&db, &stream),

            chat::Message::JoinRequest => handle_join_request(
                &db,
                &stream,
                &user_id,
                &env.frame_as_join_request().expect("JoinRequest"),
                &mut current_conversation,
            ),

            chat::Message::ClientText => handler_client_text(
                &stream,
                &db,
                &user_id,
                &env.frame_as_client_text().expect("ClientText"),
                &current_conversation,
            ),

            _ => {
                if let Ok(mut s) = stream.try_clone() {
                    let mut fbb = FlatBufferBuilder::new();
                    let env = env_with_error(&mut fbb, "Unsupported message type to server");
                    fbb.finish(env, None);
                    let _ = write_frame(&mut s, fbb.finished_data());
                }
            }
        }
    }

    if let Some(conv_id) = current_conversation.as_ref() {
        handle_leave(Arc::clone(&db), &user_id, &conv_id);
    }
    connection_closed(Arc::clone(&db), &user_id);
}

fn connection_opened(stream: TcpStream, db: Arc<Mutex<model::InMemoryDB>>, user_id: String) {
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
) {
    let sinks: Vec<TcpStream> = {
        let dbm = db.lock().unwrap();
        let Some(conv) = dbm.conversations.get(conv_id) else {
            return;
        };
        conv.users
            .iter()
            .filter(|uid| skip_user.map_or(true, |s| *uid != s))
            .filter_map(|uid| dbm.connections.get(uid))
            .filter_map(|s| s.try_clone().ok())
            .collect()
    };

    let mut fbb = FlatBufferBuilder::new();
    let env = env_with_chat(&mut fbb, text);
    fbb.finish(env, None);
    let bytes = fbb.finished_data();

    for mut sink in sinks {
        let _ = write_frame(&mut sink, bytes);
    }
}

fn handle_leave(db: Arc<Mutex<model::InMemoryDB>>, user_id: &String, conversation_id: &String) {
    broadcast_chat(
        &db,
        conversation_id,
        &format!(
            "User {} has left the conversation {}",
            user_id, conversation_id
        ),
        Some(user_id),
    );

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
}

fn handle_list_request(db: &Arc<Mutex<model::InMemoryDB>>, stream: &TcpStream) {
    let summaries: Vec<(String, usize, usize)> = {
        let db = db.lock().unwrap();
        db.conversations
            .iter()
            .map(|(id, conv)| (id.clone(), conv.users.len(), conv.messages.len()))
            .collect()
    };
    let mut fbb = FlatBufferBuilder::new();
    let env = env_with_list_response(&mut fbb, &summaries);
    fbb.finish(env, None);
    let _ = write_frame(&mut stream.try_clone().unwrap(), fbb.finished_data());
}

fn handle_join_request(
    db: &Arc<Mutex<model::InMemoryDB>>,
    stream: &TcpStream,
    user_id: &String,
    jr: &chat::JoinRequest,
    current_conversation: &mut Box<Option<String>>,
) {
    let conv_id = jr.conversation_id().unwrap_or_default().to_string();
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
    if let Ok(mut s) = stream.try_clone() {
        for line in history {
            let mut fbb = FlatBufferBuilder::new();
            let env = env_with_chat(&mut fbb, &line);
            fbb.finish(env, None);
            let _ = write_frame(&mut s, fbb.finished_data());
        }
    }

    broadcast_chat(
        &db,
        &conv_id,
        &format!("User {} has joined the conversation {}", user_id, conv_id),
        Some(&user_id),
    );

    current_conversation.replace(conv_id);
}

fn handler_client_text(
    stream: &TcpStream,
    db: &Arc<Mutex<model::InMemoryDB>>,
    user_id: &String,
    ct: &chat::ClientText,
    current_conversation: &Box<Option<String>>,
) {
    let Some(conv_id) = current_conversation.as_ref() else {
        if let Ok(mut s) = stream.try_clone() {
            let mut fbb = FlatBufferBuilder::new();
            let env = env_with_error(&mut fbb, "Send JoinRequest before ClientText");
            fbb.finish(env, None);
            let _ = write_frame(&mut s, fbb.finished_data());
        }
        return;
    };

    let text = ct.text().unwrap_or_default();

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
    );

    println!("[{}] {}: {}", conv_id, user_id, text);
}
