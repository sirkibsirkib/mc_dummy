
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
            .args_from_usage("
                <ip> 'sets the server addr. Eg 127.0.0.1'
                <port> 'sets the server addr. Eg 25565'
                ").get_matches();
    let ip = matches.value_of("ip").unwrap();
    let port = matches.value_of("port").unwrap();
    if let Ok(addr) = (&format!("{}:{}", ip, port)).parse::<SocketAddr>() {
        go(addr, ip, port.parse().expect("bad port num"));
    } else {
        println!(">> Couldn't parse ip string!");
    }
}


fn go(addr: SocketAddr, ip: &str, port: u16) {
    println!("Connecting to ip={}, port={}", ip, port);
    let mut stream = {
        match TcpStream::connect(&format!("{}:{}", ip, port)) {
            Ok(stream) => stream,
            Err(e) => {
                println!("Failed to connect. Error {:#?}", e);
                return;
            },
        }
    };
    println!("Did the thing");

    let playername = "BobbyG";

    Packet::new_handshake(340, ip, 25565, HandshakeState::Login)
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
        use code_header_play as code;
        match code {
            code::SET_COMPRESSION => {
                let thresh = payload.read_varint();
                println!("THRESH = {}", thresh);
                compression_thresh = Some(thresh as u32);
            },
            x => {
                if code_is_known(x) {
                    println!("ignored code {} (hex {})", x, hex::encode(&[x;1]));
                } else {
                    println!("unknown code {} (hex {})", x, hex::encode(&[x;1]));
                }
                
            }
        };
        let mut remainder = vec![];
        let p = payload.read_to_end(&mut remainder).unwrap();
        println!("remainder {}", hex::encode(&remainder[0..p]));
    }
}



impl<T: Read> ReadPlusPlus for T {}


fn code_is_known(x: u8) -> bool {
    use code_header_play as code;
    match x {
        code::PLAYER_POSITION_AND_LOOK    |
        code::CHANGE_GAME_STATE           |
        code::DISPLAY_SCOREBOARD          |
        code::ENTITY_PROPERTIES           |
        code::JOIN_GAME       |
        code::LOGIN_SUCCESS   |
        code::PLUGIN_MSG      | 
        code::SET_SLOT        |
        code::CLOSE_WINDOW    |
        code::WINDOW_ITEMS    |
        code::ENTITY_VELOCITY |
        code::SET_EXPERIENCE  |
        code::UPDATE_HEALTH   |
        code::WORLD_BORDER    |
        code::TIME_UPDATE     |
        code::SPAWN_POSITION  |
        code::UNLOCK_RECIPES  |
        code::PLAYER_LOOK     |
        code::PLAYER_LIST_ITEM|
        code::ADVANCEMENTS    |
        code::ENTITY_METADATA |
        code::PLAYER_ABILTIES => true,
        _ => false,
    }
}