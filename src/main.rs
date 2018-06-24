
extern crate clap;
extern crate byteorder;
extern crate hex;


use std::net::TcpStream;
use std::thread;
use std::net::SocketAddr;
use std::io::{Read, Write};
use std::fmt;
use std::time;
use std::io::BufReader;
use clap::App;

mod mc_channel;
mod code_header_play;

use mc_channel::{
    Packet, HandshakeState,
    ReadPlusPlus,
    WritePlusPlus,
};


fn main() {
    let matches = App::new("MC_Dummy")
            .version("1.0")
            .author("C. Esterhuyse <christopher.esterhuyse@gmail.com>")
            .about("Connects a Dummy Client to an offline-mode Minecraft server which will wander around.")
            .args_from_usage("<ip> 'sets the server addr. Eg 127.0.0.1:25565'")
            .get_matches();
    let ip = matches.value_of("ip").unwrap();
    if let Ok(addr) = ip.parse::<SocketAddr>() {
        go(addr);
    } else {
        println!(">> Couldn't parse ip string `{}`. Good example: `127.0.0.1:8000`", ip);
    }
}


fn go(addr: SocketAddr) {
    let mut stream = TcpStream::connect("127.0.0.1:25565").unwrap();
    println!("Did the thing");

    let playername = "BobbyG";

    Packet::new_handshake(340, "127.0.0.1", 25565, HandshakeState::Login)
    .write_to(&mut stream);

    Packet::new_loginstart(playername)
    .write_to(&mut stream);

    let sleeptime = time::Duration::from_millis(300);

    let mut buf: Vec<u8> = vec![];
    while buf.len() < 64 {
        buf.push(0);
    }

    let mut compression_thresh = None;

    loop {
        let len = stream.read_varint() as usize;
        while len > buf.len() {
            buf.push(0u8);
        }
        stream.read_exact(&mut buf[0..len]);

        /*
        We wrap the payload buffer into a reader.
        its now impossible to read out of bounds and calls to payload
        will advance the state of the reader (`payload`) as expected
        */
        let mut payload = BufReader::new(&buf[0..len]);

        let code: u8 = match compression_thresh {
            None => {
                let x = payload.read_varint();
                assert!(0 <= x && x <= 256);
                x as u8
            },
            Some(k) => {
                let compress_code = payload.read_varint();
                println!("compress_code {}", compress_code);
                if compress_code != 0 {
                    continue;
                }
                let x = payload.read_varint();
                assert!(0 <= x && x <= 256);
                x as u8
            }   
        };

        println!("len {}. code {}", len, code);
        use code_header_play::*;
        match code {
            SET_COMPRESSION => {
                let thresh = payload.read_varint();
                println!("THRESH = {}", thresh);
                compression_thresh = Some(thresh as u32);
            },
            JOIN_GAME       |
            LOGIN_SUCCESS   |
            PLUGIN_MSG      | 
            SET_SLOT        |
            CLOSE_WINDOW    |
            WINDOW_ITEMS    |
            PLAYER_ABILTIES => {
                println!("idgaf.");
            },
            x => {
                println!("unknown code {} (hex {})", x, hex::encode(&[x;1]));
            }
        };
        let mut remainder = vec![];
        let p = payload.read_to_end(&mut remainder).unwrap();
        println!("remainder {}", hex::encode(&remainder[0..p]));
    }
}



impl<T: Read> ReadPlusPlus for T {}