
extern crate clap;
extern crate byteorder;
extern crate hex;


use std::net::TcpStream;
use std::thread;
use std::net::SocketAddr;
use std::io::{Read}; //Write
// use std::fmt;
use std::time;
use std::io::BufReader;
use clap::App;

mod mc_channel;
mod code_header_play;
mod game_stuff;

use mc_channel::{
    Packet, HandshakeState,
    ReadPlusPlus,
    WritePlusPlus,
    ChatMode,
    MainHand,
};

use game_stuff::{
    Position,
    PlayerState,
};


fn main() {
    let matches = App::new("MC_Dummy")
            .version("1.0")
            .author("C. Esterhuyse <christopher.esterhuyse@gmail.com>")
            .about("Connects a Dummy Client to an offline-mode Minecraft server which will wander around.")
            .args_from_usage("
                <ip> 'sets the server addr. Eg 127.0.0.1'
                <port> 'sets the server addr. Eg 25565'
                <playername> 'selects the name of the client for the game'
                ").get_matches();
    let ip = matches.value_of("ip").unwrap();
    let port = matches.value_of("port").unwrap();
    let playername = matches.value_of("playername").unwrap();
    if let Ok(addr) = (&format!("{}:{}", ip, port)).parse::<SocketAddr>() {
        let mut p = Player::new(addr, playername.to_owned(), ip.to_owned(), port.parse().unwrap());
        p.go();

    } else {
        println!(">> Couldn't parse ip string!");
    }
}

struct Player {
    stream: TcpStream,
    name: String,
    ip: String,
    port: u16,
    compression_thresh: Option<i32>,
    player_state: Option<PlayerState>,
}

type PayloadReader<'a> = BufReader<&'a [u8]>;


impl Player {
    fn new(addr: SocketAddr, name: String, ip: String, port: u16)  -> Self {
        println!(
            "Welcome, `{}`. Connecting to ip={}, port={}...",
            &name, &ip, &port,
        );
        let stream = {
            match TcpStream::connect(&addr) {
                Ok(stream) => stream,
                Err(e) => {
                    println!("Failed to connect. Error {:#?}", e);
                    panic!("AHH");
                },
            }
        };
        println!("Successfully connected!");
        Player {
            stream,
            ip,
            port,
            name,
            compression_thresh: None,
            player_state: None,
        }
    }

    fn handshake(&mut self) {
        Packet::new_handshake(340, &self.ip, self.port, HandshakeState::Login)
        .write_to(&mut self.stream, self.compression_thresh);

        Packet::new_loginstart(&self.name)
        .write_to(&mut self.stream, self.compression_thresh)
    }

    fn move_rel(&mut self, x: f64, y: f64, z: f64) {
        if let Some(ref mut state) = self.player_state {
            state.x += x;
            state.y += y;
            state.z += z;
        }
    }

    fn go(&mut self) {
        self.handshake();
        let mut buf = vec![]; 
        loop {
            let len = self.stream.read_varint() as usize;
            while len > buf.len() { &buf.push(0u8); }
            self.stream.read_exact(&mut buf[0..len]).unwrap();
            let mut payload = BufReader::new(&buf[0..len]);

            let code: u8 = match self.compression_thresh {
                None => {
                    let x = payload.read_varint();
                    assert!(0 <= x && x <= 256);
                    x as u8
                },
                Some(_k) => {
                    let uncompressed_len = payload.read_varint();
                    println!("uncompressed_len {}", uncompressed_len);
                    if uncompressed_len != 0 {
                        println!("zlib decompression not in yet!");
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
                    self.compression_thresh = Some(thresh);
                    
                },
                code::JOIN_GAME => {
                    Packet::new_client_settings("en_us", 12, ChatMode::Enabled, true, 127u8, MainHand::Right)
                    .write_to(&mut self.stream, self.compression_thresh);
                    Packet::new_plugin_message("MC|Brand", &vec![7, 118, 97, 110, 105, 108, 108, 97])
                    .write_to(&mut self.stream, self.compression_thresh);
                },
                code::PLAYER_POSITION_AND_LOOK => {
                    let x = payload.read_doubz();
                    let y = payload.read_doubz();
                    let z = payload.read_doubz();
                    let yaw = payload.read_floatz();
                    let pitch = payload.read_floatz();
                    let flags = payload.read_bytez();
                    let teleport_id = payload.read_varint();
                    println!("got pos_look {:#?}", (x,y,z,yaw,pitch,flags,teleport_id));
                    Packet::new_teleport_confirm(teleport_id)
                    .write_to(&mut self.stream, self.compression_thresh);

                    self.player_state = Some(PlayerState{
                        x, y, z, yaw, pitch,
                        on_ground: false,
                    });

                    let longsleep = time::Duration::from_millis(2000);
                    let shortsleep = time::Duration::from_millis(90);

                    let mut t = self.player_state.clone().unwrap();

                    loop {
                        thread::sleep(shortsleep);
                        t.y -= 0.18;
                        if t.y < 4.0 {
                            t.y = 4.0;
                            Packet::new_player_position(t.x, t.y, t.z, true)
                            .write_to(&mut self.stream, self.compression_thresh);
                            break;
                        } else {
                            Packet::new_player_position(t.x, t.y, t.z, false)
                            .write_to(&mut self.stream, self.compression_thresh);
                        }
                    }
                    thread::sleep(longsleep);
                    for _ in 0..20 {
                        thread::sleep(shortsleep);
                        t.x -= 0.18;
                        Packet::new_player_position_look(t.x, t.y, t.z, t.yaw, t.pitch, true)
                        .write_to(&mut self.stream, self.compression_thresh);
                    }
                },
                x => {
                    if code_is_known(x) {
                        println!("ignored code {} (hex {})", x, hex::encode(&[x;1]));
                    } else {
                        println!("unknown code {} (hex {})", x, hex::encode(&[x;1]));
                    }
                    
                },
            };
            let mut remainder = vec![];
            let p = payload.read_to_end(&mut remainder).unwrap();
            println!("remainder {}", hex::encode(&remainder[0..p]));
        }
    }

}


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