use log::{info, warn};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::SocketAddr;
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
struct Connection<T: Default + std::fmt::Debug> {
    addr: SocketAddr,
    reader: BufReader<TcpStream>,
    protocol_context: T,
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
        let conn = self.get_connnection(token);
        info!("Disconnecting connection: {:?}", conn);
        self.connections.remove(token.0);
    }

    fn get_connnection(&self, token: Token) -> &Connection<T> {
        &self.connections[token.0]
    }

    fn get_connnection_mut(&mut self, token: Token) -> &mut Connection<T> {
        &mut self.connections[token.0]
    }

    pub fn respond(&mut self, token: Token, msg: &str) {
        if let Err(e) = self
            .get_connnection_mut(token)
            .reader
            .get_ref()
            .write(msg.as_bytes())
        {
            eprintln!("error responding: {}", e);
        }
    }

    fn accept_connection(&mut self) -> Option<Token> {
        match self.listener.accept() {
            Ok((mut stream, addr)) => {
                let token = Token(self.connections.vacant_key());

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

                self.connections.insert(Connection {
                    addr,
                    reader: BufReader::with_capacity(BUFF_CAPACITY, stream),
                    protocol_context: T::default(),
                });

                info!("Accepted connection: {:?}", self.get_connnection(token));

                Some(token)
            }
            Err(e) => {
                warn!("Error accepting connection: {}", e);
                None
            }
        }
    }

    // (requests, new connections, removed_connections)
    pub fn get_requests(&mut self) -> (Vec<(String, Token, T)>, Vec<Token>, Vec<Token>) {
        let mut events = Events::with_capacity(128);
        let mut lines = Vec::<(String, Token, T)>::with_capacity(128);
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
                        Some(line) => lines.push((
                            line,
                            token,
                            self.connections.remove(token.0).protocol_context,
                        )),
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
        let conn = self.get_connnection_mut(token);

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
