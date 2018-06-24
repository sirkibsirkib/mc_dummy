
use std::net::TcpStream;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};
use std::io::{Read, Write};
use std::io;
use hex;


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
	Handshake, LoginStart
}

pub struct Packet {
	bytes: Vec<u8>,
}

//// PUBLIC
impl Packet {
	pub fn new_handshake(version: i32, addr: &str,
						 port: u16, state: HandshakeState) -> Self
	{
		let mut packet = Self::new_raw();
		packet.write_preamble(Preamble::Handshake);
		packet.write_varint(version);
		packet.write_string(addr);
		packet.write_unsigned_short(port);
		packet.write_handshake_state(state);
		packet
	}

	pub fn new_loginstart(name: &str) -> Self {
		let mut packet = Self::new_raw();
		packet.write_preamble(Preamble::LoginStart);
		packet.write_string(name);
		packet
	}

	pub fn write_to<W: Write>(&self, w: &mut W) {
		let len = self.bytes.len();
		w.write_varint(len as i32);
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
		};
		self.write(& [byte; 1]);
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
}