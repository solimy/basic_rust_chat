use std::{
    io::{self, BufRead, BufReader},
    net::TcpStream,
    thread,
};

use crate::protocol;

pub fn list(host: String, port: u16) {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).expect("connect failed");

    let message: protocol::Message = protocol::ListRequest {}.into();
    message
        .write_message(&mut stream, &message)
        .expect("write failed");
    let mut reader = BufReader::new(stream);
    match protocol::Message::read_message(&mut reader) {
        Ok(protocol::Message::ListResponse(lr)) => {
            println!("Conversations:");
            for c in lr.conversations {
                println!(
                    "- Conversation ID: {}, Users: {}, Messages: {}",
                    c.id, c.user_count, c.message_count
                );
            }
        }
        Ok(_) => {
            eprintln!("Unexpected message type from server.");
        }
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            eprintln!("Connection closed by server.");
        }
        Err(e) => {
            eprintln!("Read error: {}", e);
        }
    }
}

pub fn join(host: String, port: u16, conversation_id: String) {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).expect("connect failed");

    println!("Connected to server at {}:{}", host, port);
    println!("Joining conversation: {}", &conversation_id);

    handle_connection(&mut stream, &conversation_id);
}

fn handle_connection(stream: &mut TcpStream, conversation_id: &String) {
    let read_stream = stream.try_clone().expect("clone failed");
    let mut buffered_reader = BufReader::new(read_stream);

    handle_join(stream, &conversation_id);

    let reader_thread = thread::spawn(move || {
        loop {
            match protocol::Message::read_message(&mut buffered_reader) {
                Ok(msg) => match msg {
                    protocol::Message::ChatMessage(cm) => {
                        println!("{}", cm.text);
                    }
                    protocol::Message::Error(err) => {
                        eprintln!("Server error: {}", err.message);
                    }
                    _ => {
                        eprintln!("Unexpected message type from server.");
                    }
                },
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    eprintln!("Connection closed by server.");
                    break;
                }
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }
        }
    });

    let mut stream = stream.try_clone().expect("clone failed");
    let writer_thread = thread::spawn(move || {
        let stdin = std::io::stdin();
        for input_line in stdin.lock().lines() {
            let input_line = match input_line {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("stdin error: {e}");
                    break;
                }
            };
            let message: protocol::Message = protocol::ClientText { text: input_line }.into();
            if let Err(e) = message.write_message(&mut stream, &message) {
                eprintln!("Write error: {}", e);
                break;
            }
        }
    });

    let _ = writer_thread.join();
    let _ = reader_thread.join();
}

fn handle_join(stream: &mut TcpStream, conversation_id: &String) {
    let message: protocol::Message = protocol::JoinRequest {
        conversation_id: conversation_id.clone(),
    }
    .into();
    message
        .write_message(stream, &message)
        .expect("write failed");
}
