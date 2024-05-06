use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::result;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

type Result<T> = result::Result<T, ()>;

pub struct Config {
    pub port: i32,
}

enum Command {
    ClientConnected(Arc<TcpStream>),
    ClientDisconnected(Arc<TcpStream>),
    NewMessage {
        author: Arc<TcpStream>,
        bytes: Vec<u8>,
    },
}

struct Client {
    conn: Arc<TcpStream>,
}

fn server(messages: Receiver<Command>) -> Result<()> {
    let mut clients = HashMap::new();
    loop {
        let msg = messages.recv().expect("The server receiver is not hung up");
        match msg {
            Command::ClientConnected(author) => {
                let addr = author.peer_addr().unwrap();
                clients.insert(
                    addr.clone(),
                    Client {
                        conn: author.clone(),
                    },
                );
                println!("[INFO]: Client connected: {addr}");
            }
            Command::ClientDisconnected(author) => {
                let addr = author.peer_addr().unwrap();
                clients.remove(&addr);
                println!("[INFO]: Client disconnected: {addr}");
            }
            Command::NewMessage { author, bytes } => {
                let author_addr = author.peer_addr().unwrap();
                for (addr, client) in clients.iter() {
                    if *addr != author_addr {
                        let _ = client.conn.as_ref().write(&bytes);
                    }
                }
            }
        }
    }
}

fn client(stream: Arc<TcpStream>, messages: Sender<Command>) -> Result<()> {
    messages
        .send(Command::ClientConnected(stream.clone()))
        .unwrap();

    let mut buffer = Vec::new();
    buffer.resize(64, 0);
    loop {
        let read = stream.as_ref().read(&mut buffer).map_err(|err| {
            eprintln!("ERROR: could not read message from client: {err}");
            let _ = messages
                .send(Command::ClientDisconnected(stream.clone()))
                .map_err(|err| {
                    eprintln!(
                        "[ERROR]: could not send disconnect message to the server thread: {err}"
                    )
                });
        })?;

        let _ = messages
            .send(Command::NewMessage {
                author: stream.clone(),
                bytes: buffer[0..read].to_vec(),
            })
            .map_err(|err| {
                eprintln!("[ERROR]: could not send message to the server thread: {err}")
            });
    }
}

pub fn start_server(config: Config) {
    let listener = match TcpListener::bind(format!("127.0.0.1:{}", config.port)) {
        Ok(l) => {
            println!("[INFO]: Server started on 127.0.0.1:{}", config.port);
            l
        }
        Err(_) => {
            eprintln!("[ERROR]: Failed to bind server TCP listener");
            return;
        }
    };

    let (message_sender, message_receiver): (Sender<Command>, Receiver<Command>) = channel();
    thread::spawn(|| server(message_receiver));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let sender = message_sender.clone();
                thread::spawn(|| client(Arc::new(stream), sender));
            }
            Err(e) => {
                eprintln!("[ERROR]: Failed to accept connection: {}", e);
            }
        }
    }
}
