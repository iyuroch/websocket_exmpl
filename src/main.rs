//! A chat server that broadcasts a message to all connections.
//!
//! This is a simple line-based server which accepts WebSocket connections,
//! reads lines from those connections, and broadcasts the lines to all other
//! connected clients.
//!
//! You can test this out by running:
//!
//!     cargo run --example server 127.0.0.1:12345
//!
//! And then in another window run:
//!
//!     cargo run --example client ws://127.0.0.1:12345/
//!
//! You can run the second command in multiple windows and then chat between the
//! two, seeing the messages from the other client as they're received. For all
//! connected clients they'll all join the same room and see everyone else's
//! messages.

extern crate futures;
extern crate tokio;
extern crate tokio_threadpool;
extern crate tokio_tungstenite;
extern crate tungstenite;

use std::time::Duration;
use std::env;
use std::io::{Error, ErrorKind};

use tokio_threadpool::blocking;
use futures::stream::Stream;
use futures::Future;
use futures::future::{lazy, poll_fn};
use futures::Sink;
use tokio::timer::Interval;
use tokio::net::TcpListener;
// use tokio::
use tungstenite::protocol::Message;

use tokio_tungstenite::accept_async;

fn fibonacci(n: u32) -> u32 {
	let (mut x_0, mut x_1) = (0, 1);
	for _ in 0..n {
		let y = x_0;
		x_0 = x_1;
		x_1 = y + x_1;
	}
	x_1
}

fn main() {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse().unwrap();

    let socket = TcpListener::bind(&addr).unwrap();
    println!("Listening on: {}", addr);

	// let mut rt = tokio::runtime::Runtime::new().unwrap();

    let srv = socket.incoming().for_each(move |stream| {

        let addr = stream.peer_addr().expect("connected streams should have a peer address");
        println!("Peer address: {}", addr);

        accept_async(stream).and_then(move |ws_stream| {
            println!("New WebSocket connection: {}", addr);

            // Let's split the WebSocket stream, so we can work with the
            // reading and writing halves separately.
            let (mut sink, stream) = ws_stream.split();

            // Whenever we receive a message from the client, we print it and
            // send to other clients, excluding the sender.
            let ws_reader = stream.for_each(move |message: Message| {
                println!("Received a message from {}: {}", addr, message);
                Ok(())
            });
			
			let mut counter = 0;

			let ws_writer = Interval::new_interval(Duration::from_millis(3000))
							.for_each(move |_instant| {
								counter += 1;
								println!("fire; instant={:?}", counter);
								tokio::spawn(lazy(move || {
										poll_fn(move || {
											blocking(|| {
												let msg = fibonacci(counter);
												println!("fibonacci; msg={:?}", msg);
												// sink.start_send(Message::from(msg.to_string())).unwrap();
											}).map_err(|_| panic!("the threadpool shut down"))
										})
									})
								);
								Ok(())
							})
							.map_err(|e| panic!("interval errored; err={:?}", e));

            let connection = ws_reader.map(|_| ()).map_err(|_| ())
                                      .select(ws_writer.map(|_| ()).map_err(|_| ()));

            tokio::spawn(connection.then(move |_| {
                println!("Connection {} closed.", addr);
                Ok(())
            }));

            Ok(())
        }).map_err(|e| {
            println!("Error during the websocket handshake occurred: {}", e);
            Error::new(ErrorKind::Other, e)
        })
    });

    // Execute server.
    tokio::run(srv.map_err(|_e| ()));
}
