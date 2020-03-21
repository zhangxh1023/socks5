use async_std::io;
use async_std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use async_std::prelude::*;
use async_std::task;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Client};
use std::convert::Infallible;
use std::net::SocketAddr;

const SOCKS5_VERSION: u8 = 0x05;
const SOCKS5_AUTH_METHOD_NONE: u8 = 0x00;
const SOCKS5_CMD_TCP_CONNECT: u8 = 0x01;
const SOCKS5_RSV: u8 = 0x00;
const SOCKS5_ADDR_TYPE_IPV4: u8 = 0x01;
const SOCKS5_ADDR_TYPE_DOMAIN: u8 = 0x03;
const SOCKS5_ADDR_TYPE_IPV6: u8 = 0x04;
const SOCKS5_REPLY_SUCCEEDED: u8 = 0x00;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
  run()
}

fn run() -> Result<()> {
  let socks5_fut = socks5_accept_loop("127.0.0.1:1186");
  println!("listening 127.0.0.1:1186");
  task::spawn(async {
    println!("listening 127.0.0.1:1187");
    http_accept_loop("127.0.0.1:1187");
  });
  task::block_on(socks5_fut)
}

async fn socks5_accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
  let listener = TcpListener::bind(addr).await?;
  let mut incoming = listener.incoming();
  while let Some(stream) = incoming.next().await {
    let stream = stream?;
    println!("Accept from {}", stream.peer_addr()?);
    spawn_and_log_error(socks5_handle(stream));
  }
  Ok(())
}

#[tokio::main]
async fn http_accept_loop(addr: impl ToSocketAddrs) {
  // We'll bind to 127.0.0.1:3000
  let addr = SocketAddr::from(([127, 0, 0, 1], 1187));

  // A `Service` is needed for every connection, so this
  // creates one from our `hello_world` function.
  let make_svc = make_service_fn(|_conn| async {
    // service_fn converts our function into a `Service`
    Ok::<_, Infallible>(service_fn(http_handle))
  });

  let server = Server::bind(&addr).serve(make_svc);

  // Run this server for... forever!
  if let Err(e) = server.await {
    eprintln!("server error: {}", e);
  }
}

async fn http_handle(req: Request<Body>) -> Result<Response<Body>> {
  let mut dest_stream = TcpStream::connect("127.0.0.1:8080").await?;
  let buf = [SOCKS5_VERSION, 1, SOCKS5_AUTH_METHOD_NONE];
  dest_stream.write_all(&buf).await?;

  let mut buf = [0u8; 2];
  dest_stream.read_exact(&mut buf).await?;

  let req_uri = format!("{}", req.uri());
  let req_uri_slice: Vec<&str> = req_uri.split(":").collect();
  let base_buf = vec![SOCKS5_VERSION, SOCKS5_CMD_TCP_CONNECT, SOCKS5_RSV, SOCKS5_ADDR_TYPE_DOMAIN];
  let domain_bytes = req_uri_slice[0].as_bytes().to_vec();
  let port_bytes = if let Some(s) = req_uri_slice.get(1) {
    s.as_bytes().to_vec()
  } else {
    "80".as_bytes().to_vec()
  };
  let domain_len = vec![domain_bytes.len() as u8];

  let buf = [
    base_buf,
    domain_len,
    domain_bytes,
    port_bytes,
  ].concat();

  dest_stream.write_all(&buf).await?;

  let mut buf = [0u8; 4];
  dest_stream.read_exact(&mut buf).await?;

  match buf[3] {
    SOCKS5_ADDR_TYPE_IPV4 => {
      let mut buf = [0u8; 4];
      dest_stream.read_exact(&mut buf).await?;
    },
    SOCKS5_ADDR_TYPE_DOMAIN => {
      let mut domain_size = [0u8; 1];
      dest_stream.read_exact(&mut domain_size).await?;
      let mut buf = vec![0u8; domain_size[0] as usize];
      dest_stream.read_exact(&mut buf).await?;
    },
    SOCKS5_ADDR_TYPE_IPV6 => {
      let mut buf = [0u8; 16];
      dest_stream.read_exact(&mut buf).await?;
    },
    _ => (),
  };

  let mut buf = [0u8; 2];
  dest_stream.read_exact(&mut buf).await?;

  let client = Client::new();

  println!("request remote uri: {}", req.uri());

  match client.request(req).await {
    Ok(response) => Ok(response),
    Err(e) => {
      println!("{}", e);
      let response = Response::new(Body::empty());
      Ok(response)
    }
  }
}

fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
  F: Future<Output = Result<()>> + Send + 'static,
{
  task::spawn(async move {
    if let Err(e) = fut.await {
      eprintln!("{}", e)
    }
  })
}

async fn socks5_handle(mut stream: TcpStream) -> Result<()> {
  let mut dest_stream = TcpStream::connect("127.0.0.1:8080").await?;

  let mut stream_reader = stream.clone();
  let mut dest_stream_writer = dest_stream.clone();
  task::spawn(async move {
    if let Err(e) = io::copy(&mut stream_reader, &mut dest_stream_writer).await {
      println!("{}", e);
    };
    if let Err(e) = stream_reader.shutdown(Shutdown::Read) {
      println!("{}", e);
    };
    if let Err(e) = dest_stream_writer.shutdown(Shutdown::Write) {
      println!("{}", e);
    };
  });

  io::copy(&mut dest_stream, &mut stream).await?;
  if let Err(e) = stream.shutdown(Shutdown::Write) {
    println!("{}", e);
  };
  if let Err(e) = dest_stream.shutdown(Shutdown::Read) {
    println!("{}", e);
  };

  Ok(())
}
