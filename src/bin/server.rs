use std::io::{Read, Write, copy};
use std::net::{Shutdown, TcpListener, TcpStream, SocketAddr, IpAddr, Ipv4Addr};
use std::thread;
use std::str;
use std::time::Duration;

fn main() {
  let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
  println!("listening 127.0.0.1:8080");
  for stream in listener.incoming() {
    let stream = stream.unwrap();
    thread::spawn(move || {
      handle_connection(stream);
    });
  }
}

fn handle_connection(mut stream: TcpStream) {
  let mut buf = [0; 3];
  stream.read(&mut buf).unwrap();
  if buf[0] != 0x05 {
    stream.shutdown(Shutdown::Both).unwrap();
    println!("shutdown");
    return;
  }

  stream.write_all(&[5, 0]).unwrap();

  let mut addr_buf = [0; 4];
  stream.read_exact(&mut addr_buf).unwrap();
  if addr_buf[0] != 5 || addr_buf[1] != 1 {
    stream.shutdown(Shutdown::Both).unwrap();
    println!("shutdown");
    return;
  }
  enum Destination {
    Ipv4(SocketAddr),
    DomainName(Vec<u8>, u16),
    Unknown,
  }
  let addr = match addr_buf[3] {
    0x01 => {
      let mut buf = vec![0u8; 6];
      stream.read_exact(&mut buf).unwrap();
      let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3])),
      buf[4] as u16 * 256 + buf[5] as u16);
      let temp: Vec<u8> = vec![0x05, 0x00, 0x00, 0x01];
      stream.write_all(&[temp, buf].concat()).unwrap();
      Destination::Ipv4(socket_addr)
    },
    0x03 => {
      let mut len = [0u8; 1];
      stream.read_exact(&mut len).unwrap();
      let len = len[0] as usize;
      let mut buf = vec![0u8; len];
      stream.read_exact(&mut buf).unwrap();
      let mut port = vec![0u8; 2];
      stream.read_exact(&mut port).unwrap();

      let temp: Vec<u8> = vec![0x05, 0x00, 0x00, 0x03];
      stream.write_all(&[temp, vec![len as u8], buf.clone(), port.clone()].concat()).unwrap();

      Destination::DomainName(buf, port[0] as u16 * 256 + port[1] as u16)
    },
    0x04 => Destination::Unknown,
    _ => Destination::Unknown,
  };

  match addr {
    Destination::Ipv4(socket_addr) => {
      let dest_stream = TcpStream::connect(socket_addr).unwrap();
      dest_stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
      let mut client_reader = stream.try_clone().unwrap();
      let mut socket_writer = dest_stream.try_clone().unwrap();

      thread::spawn(move || {
        copy(&mut client_reader, &mut socket_writer).unwrap();
        client_reader.shutdown(Shutdown::Read).unwrap();
        socket_writer.shutdown(Shutdown::Write).unwrap();
      });

      let mut socket_reader = dest_stream.try_clone().unwrap();
      let mut client_writer = stream.try_clone().unwrap();

      copy(&mut socket_reader, &mut client_writer).unwrap();
      socket_reader.shutdown(Shutdown::Read).unwrap();
      client_writer.shutdown(Shutdown::Write).unwrap();
    },
    Destination::DomainName(buf, port) => {
      let dest_stream = TcpStream::connect(format!("{}:{}", str::from_utf8(&buf).unwrap(), port)).expect("domain dest connect fail");
      dest_stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
      let mut client_reader = stream.try_clone().unwrap();
      let mut socket_writer = dest_stream.try_clone().unwrap();

      thread::spawn(move || {
        copy(&mut client_reader, &mut socket_writer).unwrap();
        client_reader.shutdown(Shutdown::Read).unwrap();
        socket_writer.shutdown(Shutdown::Write).unwrap();
      });

      let mut socket_reader = dest_stream.try_clone().unwrap();
      let mut client_writer = stream.try_clone().unwrap();

      copy(&mut socket_reader, &mut client_writer).unwrap();
      socket_reader.shutdown(Shutdown::Read).unwrap();
      client_writer.shutdown(Shutdown::Write).unwrap();

    },
    Destination::Unknown => {
      stream.shutdown(Shutdown::Both).unwrap();
    }
  }
}
