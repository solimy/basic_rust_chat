use std::{
    io::{self, BufReader, Read, Write},
    net::TcpStream,
};

use crate::protocol::chat;

const LEN_PREFIX: usize = chat::LenPrefix::Value.0 as usize;

pub fn write_frame(stream: &mut TcpStream, buf: &[u8]) -> io::Result<()> {
    stream.write_all(&(buf.len() as u32).to_be_bytes())?;
    stream.write_all(buf)?;
    stream.flush()
}

fn read_exact_all(r: &mut dyn Read, mut buf: &mut [u8]) -> io::Result<()> {
    while !buf.is_empty() {
        let n = r.read(buf)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "eof"));
        }
        let tmp = buf;
        buf = &mut tmp[n..];
    }
    Ok(())
}

pub fn read_frame(reader: &mut BufReader<TcpStream>) -> io::Result<Vec<u8>> {
    let mut lb = [0u8; LEN_PREFIX];
    reader.read_exact(&mut lb)?;
    let len = u32::from_be_bytes(lb) as usize;
    let mut payload = vec![0u8; len];
    read_exact_all(reader, &mut payload)?;
    Ok(payload)
}
