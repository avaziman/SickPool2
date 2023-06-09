use bitcoincore_rpc::bitcoin::secp256k1::Context;
use io_arc::IoArc;
use log::{info, warn};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use slab;
type Slab<T> = slab::Slab<T>;

const SERVER_TOKEN: Token = Token(usize::MAX);
const TIMEOUT_SEC: u64 = 5;
const BUFF_CAPACITY: usize = 16 * 1024;
const INITIAL_CLIENTS_CAPACITY: usize = 1024;

pub struct Server<T: Default + std::fmt::Debug> {
    listener: TcpListener,
    token: Token,
    poll: Poll,
    connections: Slab<Connection<T>>,
}

#[derive(Debug)]
/* pub */
pub struct Connection<T: Default + std::fmt::Debug> {
    addr: SocketAddr,
    stream: IoArc<TcpStream>,
    reader: BufReader<IoArc<TcpStream>>,
    protocol_context: Arc<Mutex<T>>,
}

pub fn respond(mut stream: IoArc<TcpStream>, msg: &str) {
    // self.add_client_context(token, t);
    if let Err(e) = stream.write_all(msg.as_bytes()) {
        warn!("error responding: {}", e);
    }
}


// all pools are edge triggered
impl<T: Default + std::fmt::Debug> Server<T> {
    pub fn new(saddr: SocketAddr) -> Server<T> {
        let mut listener = TcpListener::bind(saddr).unwrap();
        let poll = Poll::new().unwrap();

        poll.registry()
            .register(&mut listener, SERVER_TOKEN, Interest::READABLE)
            .unwrap();

        info!("Started server on {}", saddr);

        Server {
            listener,
            token: SERVER_TOKEN,
            poll: poll,
            connections: Slab::with_capacity(INITIAL_CLIENTS_CAPACITY),
        }
    }

    // also closes the underlying stream
    pub fn disconnect(&mut self, token: Token) {
        let cn = self.connections.remove(token.0);
        info!("Disconnecting connection: {:?}", cn);
    }

    fn accept_connection(&mut self) -> Option<Token> {
        match self.listener.accept() {
            Ok((mut stream, addr)) => {
                let cns = &mut self.connections;
                let vacant_entry = cns.vacant_entry();
                let token = Token(vacant_entry.key());
                let key = vacant_entry.key();

                if let Err(e) =
                    self.poll
                        .registry()
                        .register(&mut stream, token, Interest::READABLE)
                {
                    warn!(
                        "Error registering stream: {:?} -> {}, dropping...",
                        stream, e
                    );

                    // stream is dropped
                    return None;
                }

                let stream = IoArc::new(stream);

                let con = vacant_entry.insert(Connection {
                    addr,
                    reader: BufReader::with_capacity(BUFF_CAPACITY, stream.clone()),
                    protocol_context: Arc::new(Mutex::new(T::default())),
                    stream,
                });

                info!("Accepted connection (token: {}): {:?}", key, con);

                Some(token)
            }
            Err(e) => {
                warn!("Error accepting connection: {}", e);
                None
            }
        }
    }

    // (requests, new connections, removed_connections)
    pub fn read_requests(
        &mut self,
    ) -> (
        Vec<(String, IoArc<TcpStream>, Arc<Mutex<T>>)>,
        Vec<Token>,
        Vec<Token>,
    ) {
        let mut events = Events::with_capacity(128);
        let mut lines = Vec::<_>::with_capacity(128);
        let mut new_cons = Vec::new();
        let mut to_remove_cons: Vec<Token> = Vec::new();

        if let Err(e) = self
            .poll
            .poll(&mut events, Some(Duration::from_secs(TIMEOUT_SEC)))
        {
            warn!("Error polling: {}", e);
            return (lines, new_cons, to_remove_cons);
        }

        for event in events.iter() {
            let token = event.token();

            if token == self.token {
                if let Some(token_conn) = self.accept_connection().take() {
                    new_cons.push(token_conn)
                }
                continue;
            }
            if event.is_readable() {
                loop {
                    match self.read_ready_line(token, &mut to_remove_cons).take() {
                        Some(line) => {
                            let connection = &self.connections[token.0];
                            lines.push((line, connection.stream.clone(), connection.protocol_context.clone()))
                        }
                        None => break,
                    }
                }
            }
        }

        for to_remove in &to_remove_cons {
            self.disconnect(*to_remove);
        }

        (lines, new_cons, to_remove_cons)
    }

    fn read_ready_line(&mut self, token: Token, to_remove: &mut Vec<Token>) -> Option<String> {
        let conn = &mut self.connections[token.0];

        let mut line = String::new();
        match conn.reader.read_line(&mut line) {
            // disconnect. EOF
            Ok(0) => {
                warn!("Client EOF: {:?}", &conn);
                to_remove.push(token);
                None
            }
            Ok(_) => {
                // remove delimiter
                line.pop();
                Some(line)
            }
            Err(e) => {
                match e.kind() {
                    // FINISHED READING or Interrupted
                    ErrorKind::WouldBlock | ErrorKind::Interrupted => None,
                    // Other errors we'll consider fatal.
                    _ => {
                        warn!("Error reading: {}", e);
                        to_remove.push(token);
                        None
                    }
                }
            }
        }
    }
}
