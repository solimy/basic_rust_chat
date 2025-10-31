use std::thread;
use std::{
    io::{BufReader, prelude::*},
    net::TcpStream,
};

pub fn list(host: String, port: u16) {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
    stream.write_all(b"list\n").unwrap();

    let mut buffered_reader = BufReader::new(&stream);
    let mut line = String::new();

    loop {
        line.clear();
        match buffered_reader.read_line(&mut line) {
            Ok(0) => break, // Connection closed
            Ok(_) => {
                print!("{}", line);
            }
            Err(e) => {
                eprintln!("Failed to read from server: {}", e);
                break;
            }
        }
    }
}

pub fn join(host: String, port: u16, conversation_id: String) {
    let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();

    println!("Connected to server at {}:{}", host, port);
    println!("Joining conversation: {}", conversation_id);

    handle_connection(stream, conversation_id);
}

fn handle_connection(mut stream: TcpStream, conversation_id: String) {
    let read_stream = stream.try_clone().unwrap();
    let mut buffered_reader = BufReader::new(read_stream);

    let reader_thread = thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            print!("> ");
            match buffered_reader.read_line(&mut line) {
                Ok(0) => break, // Connection closed
                Ok(_) => {
                    print!("{}", line);
                }
                Err(e) => {
                    eprintln!("Failed to read from server: {}", e);
                    break;
                }
            }
        }
    });

    let writer_thread = thread::spawn(move || {
        let stdin = std::io::stdin();

        stream
            .write_all(format!("join\n{}\n", conversation_id).as_bytes())
            .unwrap();

        for input_line in stdin.lock().lines() {
            let input_line = input_line.unwrap();
            stream
                .write_all(format!("{}\n", input_line).as_bytes())
                .unwrap();
            stream.flush().unwrap();
        }
    });

    writer_thread.join().unwrap();
    reader_thread.join().unwrap();
}
