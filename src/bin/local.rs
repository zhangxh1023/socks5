use std::net::{ TcpListener, TcpStream, Shutdown };
use std::thread;
use std::io::{ copy };

fn main() {
  let listener = TcpListener::bind("127.0.0.1:1186").unwrap();
  println!("listening 127.0.0.1:1186");
  for stream in listener.incoming() {
    let stream = stream.unwrap();
    thread::spawn(move || {
      socks5_handle(stream);
    });
  }
}

fn socks5_handle(stream: TcpStream) {
  let socks5_service = TcpStream::connect("127.0.0.1:8080").unwrap();
  let mut client_reader = stream.try_clone().unwrap();
  let mut socket_writer = socks5_service.try_clone().unwrap();

  thread::spawn(move || {
    copy(&mut client_reader, &mut socket_writer).unwrap();
    client_reader.shutdown(Shutdown::Read).unwrap();
    socket_writer.shutdown(Shutdown::Write).unwrap();
  });

  let mut client_writer = stream.try_clone().unwrap();
  let mut socket_reader = socks5_service.try_clone().unwrap();
  
  copy(&mut socket_reader, &mut client_writer).unwrap();
  socket_reader.shutdown(Shutdown::Read).unwrap();
  client_writer.shutdown(Shutdown::Write).unwrap();
}
