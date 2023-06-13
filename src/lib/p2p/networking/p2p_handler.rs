// P2P "server" differes from protocol server:
// it needs to connect to peers, not only accept connecting

struct HandlerP2P{
    server: ProtocolServer
}

