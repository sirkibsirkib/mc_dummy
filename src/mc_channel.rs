
use std::net::TcpStream;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};
use std::io::{Read, Write};
use std::io;
use hex;
use game_stuff::Position;
use std::mem;


pub enum Clientward {
	SetCompressionMessage,
	LoginSuccessMessage,
	JoinGameMessage,
	PlayerAbilitiesMessage,
	PluginMessage,
	ChunkDataMessage,
	StateChangeMessage,
	PositionRotationMessage,
	WorldBorderMessage,
	TimeMessage,
	SpawnPositionMessage,
	ScoreboardDisplayMessage,
	SetWindowContentsMessage,
	UnlockRecipesMessage,
	ChatMessage,
	UserListItemMessage,
	AdvancementsMessage,
	EntityMetadataMessage,
	EntityPropertyMessage,
}

pub enum HandshakeState {
	Login, Status,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Preamble {
	Handshake, LoginStart, PlayerPosition, ClientSettings, TeleportConfirm,
	PluginMessage,
}

pub struct Packet {
	bytes: Vec<u8>,
}

pub enum ChatMode {
	Enabled, CommandsOnly, Hidden
}
pub enum MainHand {
	Left, Right,
}

//// PUBLIC
impl Packet {
	pub fn new_handshake(version: i32, addr: &str,
						 port: u16, state: HandshakeState) -> Self
	{
		//WORKS
		let mut packet = Self::new_raw();
		packet.write_preamble(Preamble::Handshake);
		packet.write_varint(version);
		packet.write_string(addr);
		packet.write_unsigned_short(port);
		packet.write_handshake_state(state);
		packet
	}

	pub fn new_loginstart(name: &str) -> Self {
		//WORKS
		let mut packet = Self::new_raw();
		packet.write_preamble(Preamble::LoginStart);
		packet.write_string(name);
		packet
	}

	pub fn new_player_position(x:f64, feet_y:f64, z:f64, on_ground:bool) -> Self {
		let mut packet = Self::new_raw();
		println!("YEEEE");
		packet.write_preamble(Preamble::PlayerPosition);
		packet.write_doubz(x);
		packet.write_doubz(feet_y);
		packet.write_doubz(z);
		packet.write_boolz(on_ground);
		packet
	}

	pub fn new_client_settings(locale:&str, view_distance:i8, chat_mode:ChatMode, chat_colors:bool, skin_flags:u8, main_hand:MainHand) -> Self {
		//WORKS
		let mut packet = Self::new_raw();
		packet.write_preamble(Preamble::ClientSettings);
		packet.write_string(locale);
		packet.write_i8(view_distance);
		packet.write_chat_mode(chat_mode);
		packet.write_boolz(chat_colors);
		packet.write_u8(skin_flags);
		packet.write_main_hand(main_hand);
		packet
	}

	pub fn new_teleport_confirm(id: i32) -> Self {
		//WORKS
		let mut packet = Self::new_raw();
		packet.write_preamble(Preamble::TeleportConfirm);
		packet.write_varint(id);
		packet
	}

	pub fn new_plugin_message(plugin_str:&str, data:&[u8]) -> Self {
		//works
		let mut packet = Self::new_raw();
		packet.write_preamble(Preamble::PluginMessage);
		packet.write_string(plugin_str);
		packet.write(data);
		packet
	}

	pub fn write_to<W: Write>(&self, w: &mut W, compression_header: Option<i32>) {
		match compression_header {
			Some(_) => {
				let len = self.bytes.len() + 1;
				w.write_varint(len as i32);
				w.write_varint(0); // compression_header
			},
			None => {
				let len = self.bytes.len();
				w.write_varint(len as i32);
			}
		}
		w.write(&self.bytes[..]);
	}

	fn new_raw() -> Self {
		Packet { bytes: vec![] }
	}
}

impl Write for Packet {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		let r = self.bytes.write(buf);
		println!("hex {:?}", hex::encode(& self.bytes[..]));
		r
	}

	fn flush(&mut self) -> io::Result<()> {
		self.bytes.flush()
	}
}


pub trait WritePlusPlus: Write {
	fn write_varint(&mut self, mut x: i32) {
		let mut buf = [0u8, 10];
        let mut i = 0;
        if x < 0 {
            x = x + (1 << 32);
        }
        while x >= 0x80 {
            buf[i] = (x & 0x7F) as u8 | 0x80;
            x = x >> 7;
            i = i + 1;
        }
        buf[i] = x as u8;
        self.write(&buf[..i+1]);
	}

	fn write_string(&mut self, s: &str) {
        self.write_varint(s.len() as i32);
        self.write(s.as_bytes());
    }

    fn write_unsigned_short(&mut self, x: u16) {
		self.write_u16::<BigEndian>(x).unwrap();
    }

    fn write_handshake_state(&mut self, state: HandshakeState) {
    	self.write_varint(
    		match state {
	    		HandshakeState::Status => 1,
	    		HandshakeState::Login => 2,
	    	}
	    );
    }

    fn write_preamble(&mut self, preamble: Preamble) {
    	let byte: u8 = match preamble {
			Preamble::Handshake => 0x00,
			Preamble::LoginStart => 0x00,
			Preamble::PlayerPosition => 0x0D,
			Preamble::ClientSettings => 0x04,
			Preamble::TeleportConfirm => 0x00,
			Preamble::PluginMessage => 0x09,
		};
		self.write(& [byte; 1]);
    }

    fn write_position(&mut self, position: &Position) {
    	let val = 
    		((position.x & 0x3FFFFFF) << 38)
    		| ((position.y & 0xFFF) << 26)
    		| (position.z & 0x3FFFFFF);
    	self.write_u64::<LittleEndian>(unsafe {  mem::transmute(val)  });
    }

    fn write_doubz(&mut self, x:f64) {
    	self.write_f64::<BigEndian>(x);
    }

    fn write_boolz(&mut self, x:bool) {
    	if x {
    		self.write_u8(0x01);
    	} else {
    		self.write_u8(0x00);
    	}
    }

    fn write_chat_mode(&mut self, mode: ChatMode) {
    	let val = match mode {
    		ChatMode::Enabled => 0,
    		ChatMode::CommandsOnly => 1,
    		ChatMode::Hidden => 2,
    	};
    	self.write_varint(val);
    }

    fn write_main_hand(&mut self, x:MainHand) {
    	let val = match x {
    		MainHand::Left => 0,
    		MainHand::Right => 1,
    	};
    	self.write_varint(val);
    }
}


impl<T: Write> WritePlusPlus for T {}

pub trait ReadPlusPlus: Read {
	fn read_varint(&mut self) -> i32 {
        let (mut total, mut shift, mut val) = (0, 0, 0x80);

        while (val & 0x80) != 0 {
            val = self.read_u8().unwrap() as i32;
            total = total | ((val & 0x7F) << shift);
            shift = shift + 7;
        }

        if (total & (1 << 31)) != 0 {
            total = total - (1 << 32);
        }
        total
    }

    fn read_position(&mut self) -> Position {
    	let val = self.read_u64::<BigEndian>().unwrap();
		let x = val >> 38;
		let y = (val >> 26) & 0xFFF;
		let z = val << 38 >> 38;
		Position {
			x: (if x >= 2^25 { x as i64 - 2^26 } else {x as i64}),
			y: (if y >= 2^25 { y as i64 - 2^12 } else {y as i64}),
			z: (if z >= 2^25 { z as i64 - 2^26 } else {z as i64}),
		}
    }
}
impl<T: Read> ReadPlusPlus for T {}
