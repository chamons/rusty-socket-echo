server:
    cargo run -- --server -v -v -v

client:
    cargo run -- -v -v -v

client-netcat:
    nc -U loopback-socket
