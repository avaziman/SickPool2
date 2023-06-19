use io_arc::IoArc;
use log::{info, warn};
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use threadpool::ThreadPool;

use slab;

use crate::config::ServerConfig;
use crate::protocol::Protocol;
type Slab<T> = slab::Slab<T>;

const SERVER_TOKEN: Token = Token(usize::MAX);
// const TIMEOUT_SEC: u64 = 5;
const TIMEOUT_SEC: u64 = 1;
const BUFF_CAPACITY: usize = 16 * 1024;
const INITIAL_CLIENTS_CAPACITY: usize = 1024;

pub struct Server<P: Protocol> {
    pub conf: ServerConfig,
    listener: TcpListener,
    token: Token,
    poll: Poll,
    connections: Slab<Connection<P::ClientContext>>,
    protocol: Arc<P>,
    tpool: ThreadPool,
}

// #[derive(Debug)]
/* pub */
pub struct Connection<T> {
    // written: usize,
    // responded: usize,
    addr: SocketAddr,
    stream: IoArc<TcpStream>,
    reader: BufReader<IoArc<TcpStream>>,
    protocol_context: Arc<Mutex<T>>,
    connected: bool,
}

impl<T> std::fmt::Display for Connection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Connection address: {}", self.addr)
    }
}

pub fn respond(mut stream: IoArc<TcpStream>, msg: &[u8]) {
    // self.add_client_context(token, t);
    if let Err(e) = stream.write_all(msg) {
        warn!("error responding: {}", e);
    }
    stream.flush().unwrap()
}

// all pools are edge triggered
impl<P: Protocol<Request = Vec<u8>, Response = Vec<u8>> + Send + Sync + 'static> Server<P> {
    pub fn new(conf: ServerConfig, protocol: Arc<P>) -> Server<P> {
        let mut listener = TcpListener::bind(conf.address).unwrap();
        let poll = Poll::new().unwrap();

        poll.registry()
            .register(&mut listener, SERVER_TOKEN, Interest::READABLE)
            .unwrap();

        info!("Started server on {:?}", conf.address);

        Server {
            listener,
            protocol,
            token: SERVER_TOKEN,
            poll: poll,
            connections: Slab::with_capacity(INITIAL_CLIENTS_CAPACITY),
            tpool: threadpool::Builder::new()
                .num_threads(8)
                .thread_stack_size(8_000_000)
                .thread_name("Server protocol processing thread".into())
                .build(),
            conf,
        }
    }

    pub fn get_connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn is_connected(&self, addr: SocketAddr) -> bool {
        for (token, con) in &self.connections {
            if con.addr == addr {
                return true;
            }
        }
        false
    }

    pub fn process_requests(&mut self) {
        let requests = self.read_requests();
        for (req, writer, ctx) in requests.into_iter() {
            let protocol: Arc<P> = self.protocol.clone();

            self.tpool.execute(move || {
                let mut ptx = P::ProcessingContext::default();
                // info!("Received request: {:?}", req);
                let now = Instant::now();

                let stratum_resp = protocol.process_request(req, ctx, &mut ptx);

                let elapsed = now.elapsed().as_micros();
                respond(writer, stratum_resp.as_ref());

                // info!("Processed response: {:?}, in {}us", stratum_resp, elapsed);
            });
        }

        // if !rem_cons.is_empty() {
        //     for peer in self.protocol.peers_to_connect() {
        //         self.server.connect(peer);
        //     }
        // }
    }

    // also closes the underlying stream
    fn disconnect(&mut self, token: Token) {
        let cn = self.connections.remove(token.0);
        info!("Disconnecting connection: {}", cn);

        self.protocol.delete_client(cn.addr, cn.protocol_context, token);

        // all ars must be dropped here
    }

    fn accept_connection(&mut self) -> Option<Token> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                if let Err(e) = stream.set_nodelay(true) {
                    warn!("Failed to set socket nodelay: {}", addr);
                    return None;
                }
                self.add_connection(stream, addr)
            }
            Err(e) => {
                warn!("Error accepting connection: {}", e);
                None
            }
        }
    }

    fn add_connection(&mut self, mut stream: TcpStream, addr: SocketAddr) -> Option<Token> {
        let cns = &mut self.connections;
        let vacant_entry = cns.vacant_entry();
        let token = Token(vacant_entry.key());
        let key = vacant_entry.key();

        if let Err(e) = self.poll.registry().register(
            &mut stream,
            token,
            Interest::READABLE.add(Interest::WRITABLE),
        ) {
            warn!(
                "Error registering stream: {:?} -> {}, dropping...",
                stream, e
            );

            // stream is dropped
            return None;
        }

        let stream = IoArc::new(stream);

        let ctx = match self.protocol.create_client(addr, stream.clone(), token) {
            Some(k) => k,
            None => {
                return None;
            }
        };

        let con = vacant_entry.insert(Connection {
            addr,
            reader: BufReader::with_capacity(BUFF_CAPACITY, stream.clone()),
            protocol_context: Arc::new(Mutex::new(ctx)),
            stream,
            connected: true,
        });

        info!("Accepted connection (token: {}): {}", key, con);
        Some(token)
    }

    // (requests, new connections, removed_connections)
    pub fn read_requests(
        &mut self,
    ) -> Vec<(Vec<u8>, IoArc<TcpStream>, Arc<Mutex<P::ClientContext>>)> {
        let mut events = Events::with_capacity(128);
        let mut lines = Vec::<_>::with_capacity(128);
        let mut new_cons = Vec::new();
        let mut removed_cons: Vec<Token> = Vec::new();

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
                if let Some(token_conn) = self.accept_connection().take() {
                    new_cons.push(token_conn)
                }
                continue;
            }

            if event.is_readable() {
                loop {
                    match self.read_ready_line(token, &mut removed_cons).take() {
                        Some(line) => {
                            let connection = &self.connections[token.0];
                            lines.push((
                                line,
                                connection.stream.clone(),
                                connection.protocol_context.clone(),
                            ))
                        }
                        None => break,
                    }
                }
            }

            // TODO: buffer failed writes
            if event.is_writable() {
                let con = &mut self.connections[token.0];
                if !con.connected {
                    match con.stream.as_ref().peer_addr() {
                        Ok(k) => {
                            // connected
                            con.connected = true;
                            info!("Connection to {} has been established!", con.addr);
                            
                            self.protocol
                                .client_conncted(con.stream.clone(), con.protocol_context.clone());
                        }
                        Err(e) => {
                            // connection still pending...
                        }
                    }
                }
            }
        }

        for to_remove in &removed_cons {
            self.disconnect(*to_remove);
        }

        lines
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Option<IoArc<TcpStream>> {
        info!("Connecting to {}...", addr);

        match TcpStream::connect(addr) {
            Ok(stream) => {
                match self.add_connection(stream, addr) {
                    Some(t) => {
                        let con = &mut self.connections[t.0];
                        con.connected = false;
                        Some(con.stream.clone())

                    }
                    None => None,
                }

                // there was an idea to register write event to know when the connection established ...
            }
            Err(e) => {
                warn!("Failed to connect to: {} -> {}", addr, e);
                None
            }
        }
    }

    // fn broadcast(&self, tokens: &[Token]) {
    //     for token in tokens {
    //         respond(self.connections[token.0]);
    //     }
    // }

    fn read_ready_line(&mut self, token: Token, to_remove: &mut Vec<Token>) -> Option<Vec<u8>> {
        let conn = &mut self.connections[token.0];

        let mut line = Vec::new();
        match conn.reader.read_until('\n' as u8, &mut line) {
            // disconnect. EOF
            Ok(0) => {
                warn!("Client EOF: {}", &conn);
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

// WHEN A NEW JOB COMES, processing threads need to first update their context, then we can notify the clients, shares that are being processed whilst the new job was received are acceptable
