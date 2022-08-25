# rusty-socket-echo

This is a sample which uses [tokio](https://tokio.rs/) to create a simple client/server that speak a common protocol for echo. 

Instead of a simple line protocol (repeat each line and quit when the socket closes), this projcet uses enums serialized with [bincode](https://docs.rs/bincode/latest/bincode/) to connect/disconnect sessions. Admittingly this is overkill for this example, but it was for learning. 

