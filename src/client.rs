use std::{
    io::{self, BufRead, BufReader},
    net::TcpStream,
    thread,
};

use crate::protocol::{chat, helpers::client::*, helpers::common::*};

use flatbuffers::FlatBufferBuilder;

pub fn list(host: String, port: u16) {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).expect("connect failed");

    let mut fbb = FlatBufferBuilder::new();
    let env = env_with_list_request(&mut fbb);
    fbb.finish(env, None);
    let bytes = fbb.finished_data();
    write_frame(&mut stream, bytes).expect("write failed");

    let mut reader = BufReader::new(stream);
        match read_frame(&mut reader) {
            Ok(buf) => {
                if let Err(e) = parse_and_print(&buf) {
                    eprintln!("parse error: {e}");
                }
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {},
            Err(e) => {
                eprintln!("read error: {e}");
            }
        }
}

pub fn join(host: String, port: u16, conversation_id: String) {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).expect("connect failed");

    println!("Connected to server at {}:{}", host, port);
    println!("Joining conversation: {}", &conversation_id);

    {
        let mut fbb = FlatBufferBuilder::new();
        let env = env_with_join_request(&mut fbb, &conversation_id);
        fbb.finish(env, None);
        let bytes = fbb.finished_data();
        write_frame(&mut stream, bytes).expect("write failed");
    }

    handle_connection(stream);
}

fn handle_connection(mut stream: TcpStream) {
    let read_stream = stream.try_clone().expect("clone failed");
    let mut buffered_reader = BufReader::new(read_stream);

    let reader_thread = thread::spawn(move || {
        loop {
            match read_frame(&mut buffered_reader) {
                Ok(buf) => {
                    if let Err(e) = parse_and_print(&buf) {
                        eprintln!("Failed to parse server frame: {}", e);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    eprintln!("Failed to read from server: {}", e);
                    break;
                }
            }
        }
    });

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
            let mut fbb = FlatBufferBuilder::new();
            let env = env_with_client_text(&mut fbb, &input_line);
            fbb.finish(env, None);
            if let Err(e) = write_frame(&mut stream, fbb.finished_data()) {
                eprintln!("write error: {e}");
                break;
            }
        }
    });

    let _ = writer_thread.join();
    let _ = reader_thread.join();
}

fn parse_and_print(buf: &[u8]) -> Result<(), String> {
    let env = chat::root_as_envelope(buf).map_err(|e| e.to_string())?;

    let frame_type = env.frame_type();
    match frame_type {
        chat::Message::ListResponse => {
            let lr = env.frame_as_list_response().ok_or("bad ListResponse")?;
            println!("Conversations:");
            if let Some(list) = lr.conversations() {
                for c in list {
                    println!("- Conversation ID: {}, Users: {}, Messages: {}",
                        c.id().unwrap_or_default(),
                        c.user_count(),
                        c.message_count()
                    );
                }
            }
        }
        chat::Message::ChatMessage => {
            let m = env.frame_as_chat_message().ok_or("bad ChatMessage")?;
            println!("{}", m.text().unwrap_or_default());
        }
        chat::Message::Error => {
            let e = env.frame_as_error().ok_or("bad Error")?;
            eprintln!("Server error: {}", e.message().unwrap_or_default());
        }
        _ => println!("Unsupported message type from server: {:?}", frame_type),
    }
    Ok(())
}
