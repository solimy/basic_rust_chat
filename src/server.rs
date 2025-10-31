use std::thread;
use std::{
    io::{BufReader, BufWriter, prelude::*},
    net::{TcpListener, TcpStream},
};

use std::sync::{Arc, Mutex};

use crate::model;

pub fn serve(host: String, port: u16) {
    let db = Arc::new(Mutex::new(model::InMemoryDB::new()));
    let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();

    println!("Server listening on {}:{}", host, port);
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let db_clone = Arc::clone(&db);
        thread::spawn(move || {
            handle_connection(stream, db_clone);
        });
    }
}

fn handle_connection(stream: TcpStream, db: Arc<Mutex<model::InMemoryDB>>) {
    let user_id = stream.peer_addr().unwrap().to_string();

    connection_opened(
        stream.try_clone().unwrap(),
        Arc::clone(&db),
        user_id.clone(),
    );

    let read_stream = stream.try_clone().unwrap();
    let mut buffered_reader = BufReader::new(&read_stream);
    let mut line = String::new();

    let command = match buffered_reader.read_line(&mut line) {
        Ok(_) => line.clone().trim_end().to_string(),
        Err(_) => {
            eprintln!("Failed to read command from client.");
            return;
        }
    };

    match command.as_str() {
        "list" => handle_list(stream.try_clone().unwrap(), Arc::clone(&db), &user_id),
        "join" => {
            if let Some(conversation_id) = handle_join(
                stream.try_clone().unwrap(),
                &mut buffered_reader,
                Arc::clone(&db),
                &user_id,
            ) {
                handle_leave(Arc::clone(&db), &user_id, &conversation_id);
            }
        }
        _ => {
            eprintln!("Unknown command received: {}", command);
        }
    }

    connection_closed(Arc::clone(&db), &user_id);
}

fn connection_opened(stream: TcpStream, db: Arc<Mutex<model::InMemoryDB>>, user_id: String) {
    if let Ok(mut locked_db) = db.lock() {
        locked_db
            .connections
            .insert(user_id.clone(), stream.try_clone().unwrap());
    }
    println!("New connection from {}", user_id);
}

fn connection_closed(db: Arc<Mutex<model::InMemoryDB>>, user_id: &String) {
    if let Ok(mut locked_db) = db.lock() {
        locked_db.connections.remove(user_id);
    }
    println!("Connection from {} closed.", user_id);
}

fn handle_list(mut stream: TcpStream, db: Arc<Mutex<model::InMemoryDB>>, user_id: &String) {
    if let Ok(locked_db) = db.lock() {
        println!("Listing conversations for user {}", user_id);
        let count = locked_db.conversations.len();
        stream
            .write_all(format!("Total conversations: {}\n", count).as_bytes())
            .unwrap();
        for (conv_id, conv) in locked_db.conversations.iter() {
            stream
                .write_all(
                    format!(
                        "Conversation ID: {}, Users: {}, Messages: {}\n",
                        conv_id,
                        conv.users.len(),
                        conv.messages.len()
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
        return;
    } else {
        eprintln!("Failed to lock the database for listing conversations.");
        return;
    }
}

fn handle_join(
    mut stream: TcpStream,
    buffered_reader: &mut BufReader<&TcpStream>,
    db: Arc<Mutex<model::InMemoryDB>>,
    user_id: &String,
) -> Option<String> {
    let mut line = String::new();
    let conversation_id = match buffered_reader.read_line(&mut line) {
        Ok(_) => line.clone().trim_end().to_string(),
        Err(_) => {
            eprintln!("Failed to read conversation ID from client.");
            return None;
        }
    };

    println!(
        "User {} is joining conversation {}",
        user_id, conversation_id
    );

    if let Ok(mut locked_db) = db.lock() {
        if !locked_db.conversations.contains_key(&conversation_id) {
            println!(
                "Conversation {} does not exist. Creating a new one.",
                conversation_id
            );

            locked_db.conversations.insert(
                conversation_id.clone(),
                model::Conversation::new(user_id.clone()),
            );

            println!(
                "Conversation {} created by user {}",
                conversation_id, user_id
            );
        } else {
            let conversation = locked_db.conversations.get_mut(&conversation_id).unwrap();
            if !conversation.users.contains(&user_id) {
                conversation.users.push(user_id.clone());
                println!("User {} added to conversation {}", user_id, conversation_id);
            }
            for msg in &conversation.messages {
                println!(
                    "Sending history to {}: [{}] {}: {}",
                    user_id, conversation_id, msg.sender, msg.content
                );
                stream
                    .write_all(
                        format!("[{}] {}: {}\n", conversation_id, msg.sender, msg.content)
                            .as_bytes(),
                    )
                    .unwrap();
            }
            for (uid, mut conn) in &locked_db.connections {
                if uid != user_id {
                    conn.write_all(
                        format!(
                            "User {} has joined the conversation {}\n",
                            user_id, conversation_id
                        )
                        .as_bytes(),
                    )
                    .unwrap();
                }
            }
        }
    }

    line.clear();
    while let Ok(len) = buffered_reader.read_line(&mut line) {
        if len == 0 {
            break;
        }
        let message = model::Message::new(user_id.clone(), line.clone().trim_end().to_string());

        if let Ok(mut locked_db) = db.lock() {
            let conversation = locked_db.conversations.get_mut(&conversation_id).unwrap();
            conversation.add_message(message);
        }

        if let Ok(locked_db) = db.lock() {
            for uid in &locked_db.conversations.get(&conversation_id).unwrap().users {
                if let Some(conn) = locked_db.connections.get(uid) {
                    let mut writer = BufWriter::new(conn);
                    writer
                        .write_all(
                            format!("[{}] {}: {}\n", conversation_id, user_id, line.trim_end())
                                .as_bytes(),
                        )
                        .unwrap();
                }
            }
        }

        println!("[{}] {}: {}", conversation_id, user_id, line.trim_end());
        line.clear();
    }
    Some(conversation_id)
}

fn handle_leave(db: Arc<Mutex<model::InMemoryDB>>, user_id: &String, conversation_id: &String) {
    if let Ok(locked_db) = db.lock() {
        let conversation = locked_db.conversations.get(conversation_id).unwrap();
        for uid in &conversation.users {
            if let Some(conn) = locked_db.connections.get(uid) {
                let mut writer = BufWriter::new(conn);
                writer
                    .write_all(
                        format!(
                            "User {} has left the conversation {}\n",
                            user_id, conversation_id
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            }
        }
    }
    if let Ok(mut locked_db) = db.lock() {
        let conversation = locked_db.conversations.get_mut(conversation_id).unwrap();

        conversation.users.retain(|uid| uid != user_id);

        if conversation.users.is_empty() {
            println!(
                "No more users in conversation {}. Deleting conversation.",
                conversation_id
            );
            locked_db.conversations.remove(conversation_id);
        }
        locked_db.connections.remove(user_id);
    }
}
