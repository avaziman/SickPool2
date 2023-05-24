use log::{info, trace, warn};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::io::{BufRead, BufReader, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use slab;
type Slab<T> = slab::Slab<T>;

const SERVER_TOKEN: Token = Token(usize::MAX);
const TIMEOUT_SEC: u64 = 5;
const BUFF_CAPACITY: usize = 16 * 1024;

pub struct Server {
    listener: TcpListener,
    token: Token,
    poll: Poll,
    connections: Slab<Connection>,
}

#[derive(Debug)]
struct Connection {
    addr: SocketAddr,
    reader: BufReader<TcpStream>,
}

// all pools are edge triggered
impl Server {
    pub fn new(saddr: SocketAddr) -> Server {
        let mut listener = TcpListener::bind(saddr).unwrap();
        let poll = Poll::new().unwrap();

        poll.registry()
            .register(&mut listener, SERVER_TOKEN, Interest::READABLE)
            .unwrap();

        Server {
            listener,
            token: SERVER_TOKEN,
            poll: poll,
            connections: Slab::<Connection>::with_capacity(1024),
        }
    }

    fn accept_connection(&mut self) {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                let conn_index = self.connections.insert(Connection {
                    addr,
                    reader: BufReader::with_capacity(BUFF_CAPACITY, stream),
                });
                let con = &mut self.connections[conn_index];

                let token = Token(conn_index);
                if let Err(e) =
                    self.poll
                        .registry()
                        .register(con.reader.get_mut(), token, Interest::READABLE)
                {
                    warn!(
                        "Error registering connection: {}, dropping connection {:?}",
                        e, con
                    );
                    self.connections.remove(conn_index);
                } else {
                    info!("Accepted connection: {:?}", con);
                }
            }
            Err(e) => {
                warn!("Error accepting connection: {}", e)
            }
        }
    }

    pub fn get_requests(&mut self) -> Vec<String> {
        let mut events = Events::with_capacity(128);
        let mut lines = Vec::<String>::with_capacity(128);

        if let Err(e) = self
            .poll
            .poll(&mut events, Some(Duration::from_secs(TIMEOUT_SEC)))
        {
            warn!("Error polling: {}", e);
            return lines;
        }

        for event in events.iter() {
            let token = event.token();
            if token == self.token {
                self.accept_connection();
                continue;
            }

            let conn = &mut self.connections[token.0];

            if event.is_readable() {
                loop {
                    match self.read_ready_line(token).take() {
                        Some(line) => lines.push(line),
                        None => break,
                    }
                }
            }
        }

        lines
    }

    fn read_ready_line(&mut self, token: Token) -> Option<String> {
        let conn = &mut self.connections[token.0];
        let mut line = String::new();
        match conn.reader.read_line(&mut line) {
            // disconnect. EOF
            Ok(0) => None,
            Ok(_) => {
                // remove delimiter
                line.pop();
                Some(line)
            },
            Err(e) => {
                match e.kind() {
                    // FINISHED READING
                    ErrorKind::WouldBlock => None,
                    ErrorKind::Interrupted => None,
                    // Other errors we'll consider fatal.
                    _ => {
                        warn!("Error reading: {}", e);
                        None
                    }
                }
            }
        }
    }
}
