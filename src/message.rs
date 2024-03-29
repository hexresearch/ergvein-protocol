use crate::util::*;
use consensus_encode::util::hex::ToHex;
pub use consensus_encode::util::stream_reader::StreamReader;
pub use consensus_encode::{
    deserialize, deserialize_partial, serialize, serialize_hex, Decodable, Encodable, Error,
    VarInt, MAX_VEC_SIZE,
};
use fix::aliases::si::Centi;
use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Write};
use std::{io, net};

macro_rules! impl_pure_encodable {
    ($ty:ident, $meth_dec:ident, $meth_enc:ident) => {
        impl Decodable for $ty {
            #[inline]
            fn consensus_decode<D: io::Read>(d: D) -> Result<Self, Error> {
                Ok($ty::$meth_dec(Decodable::consensus_decode(d)?))
            }
        }
        impl Encodable for $ty {
            #[inline]
            fn consensus_encode<S: io::Write>(&self, s: S) -> Result<usize, io::Error> {
                Encodable::consensus_encode(&self.$meth_enc(), s)
            }
        }
    };
}

/// Currencies that protocol aware of, there can be currencies that will never be implemented but
/// index in protocol is known.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Currency {
    Btc,
    TBtc,
    Ergo,
    TErgo,
    UsdtOmni,
    TUsdtOmni,
    Ltc,
    TLtc,
    Zec,
    TZec,
    Cpr,
    TCpr,
    Dash,
    TDash,
    Unknown(u32),
}

impl Default for Currency {
    fn default() -> Self {
        Currency::Btc
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::Btc => write!(f, "Bitcoin"),
            Currency::TBtc => write!(f, "Testnet Bitcoin"),
            Currency::Ergo => write!(f, "Ergo"),
            Currency::TErgo => write!(f, "Testnet Ergo"),
            Currency::UsdtOmni => write!(f, "USDT on Omni"),
            Currency::TUsdtOmni => write!(f, "Testnet USDT on Omni"),
            Currency::Ltc => write!(f, "Litecoin"),
            Currency::TLtc => write!(f, "Testnet Litecoin"),
            Currency::Zec => write!(f, "ZCash"),
            Currency::TZec => write!(f, "Testnet ZCash"),
            Currency::Cpr => write!(f, "Cypra"),
            Currency::TCpr => write!(f, "Testnet Cypra"),
            Currency::Dash => write!(f, "Dash"),
            Currency::TDash => write!(f, "Testnet Dash"),
            Currency::Unknown(i) => write!(f, "Unknown currency {}", i),
        }
    }
}

impl Currency {
    pub fn as_index(&self) -> u32 {
        match self {
            Currency::Btc => 0,
            Currency::TBtc => 1,
            Currency::Ergo => 2,
            Currency::TErgo => 3,
            Currency::UsdtOmni => 4,
            Currency::TUsdtOmni => 5,
            Currency::Ltc => 6,
            Currency::TLtc => 7,
            Currency::Zec => 8,
            Currency::TZec => 9,
            Currency::Cpr => 10,
            Currency::TCpr => 11,
            Currency::Dash => 12,
            Currency::TDash => 13,
            Currency::Unknown(i) => *i,
        }
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            0 => Currency::Btc,
            1 => Currency::TBtc,
            2 => Currency::Ergo,
            3 => Currency::TErgo,
            4 => Currency::UsdtOmni,
            5 => Currency::TUsdtOmni,
            6 => Currency::Ltc,
            7 => Currency::TLtc,
            8 => Currency::Zec,
            9 => Currency::TZec,
            10 => Currency::Cpr,
            11 => Currency::TCpr,
            12 => Currency::Dash,
            13 => Currency::TDash,
            i => Currency::Unknown(i),
        }
    }

    fn pack(&self) -> VarInt {
        VarInt(self.as_index() as u64)
    }

    fn unpack(i: VarInt) -> Self {
        Currency::from_index(i.0 as u32)
    }
}
impl_pure_encodable!(Currency, unpack, pack);

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Debug, Hash)]
pub enum Fiat {
    Usd,
    Eur,
    Rub,
    Unknown(u32),
}

impl Display for Fiat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Fiat::Usd => write!(f, "US Dollar"),
            Fiat::Eur => write!(f, "Euro"),
            Fiat::Rub => write!(f, "Ruble"),
            Fiat::Unknown(i) => write!(f, "Unknown fiat currency {}", i),
        }
    }
}

impl Fiat {
    pub fn as_index(&self) -> u32 {
        match self {
            Fiat::Usd => 0,
            Fiat::Eur => 1,
            Fiat::Rub => 2,
            Fiat::Unknown(i) => *i,
        }
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            0 => Fiat::Usd,
            1 => Fiat::Eur,
            2 => Fiat::Rub,
            i => Fiat::Unknown(i),
        }
    }

    fn pack(&self) -> VarInt {
        VarInt(self.as_index() as u64)
    }

    fn unpack(i: VarInt) -> Self {
        Fiat::from_index(i.0 as u32)
    }
}
impl_pure_encodable!(Fiat, unpack, pack);

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Debug, Hash)]
pub enum Address {
    Ipv4(net::SocketAddrV4),
    Ipv6(net::SocketAddrV6),
    OnionV3([u8; 56], u16),
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Ipv4(addr) => write!(f, "{}", addr),
            Address::Ipv6(addr) => write!(f, "{}", addr),
            Address::OnionV3(addr, p) => {
                let str_addr =
                    String::from_utf8(addr.to_vec()).unwrap_or_else(|_| format!("{:?}", addr));
                write!(f, "{}:{}", str_addr, p)
            }
        }
    }
}

fn ipv6_to_be(addr: [u16; 8]) -> [u16; 8] {
    [
        addr[0].to_be(),
        addr[1].to_be(),
        addr[2].to_be(),
        addr[3].to_be(),
        addr[4].to_be(),
        addr[5].to_be(),
        addr[6].to_be(),
        addr[7].to_be(),
    ]
}

impl Decodable for Address {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
        let t: u8 = Decodable::consensus_decode(&mut d)?;
        match t {
            0 => {
                let b: [u8; 4] = Decodable::consensus_decode(&mut d)?;
                let ip = net::Ipv4Addr::new(b[0], b[1], b[2], b[3]);
                let p: u16 = Decodable::consensus_decode(&mut d)?;
                let addr = net::SocketAddrV4::new(ip, p.to_be());
                Ok(Address::Ipv4(addr))
            }
            1 => {
                let b: [u16; 8] = ipv6_to_be(Decodable::consensus_decode(&mut d)?);
                let ip = net::Ipv6Addr::new(b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]);
                let p: u16 = Decodable::consensus_decode(&mut d)?;
                let addr = net::SocketAddrV6::new(ip, p.to_be(), 0, 0);
                Ok(Address::Ipv6(addr))
            }
            2 => {
                let mut b = [0; 56];
                d.read_exact(&mut b)?;
                let p: u16 = Decodable::consensus_decode(&mut d)?;
                Ok(Address::OnionV3(b, p.to_be()))
            }
            _ => Err(Error::ParseFailed("Unknown address type")),
        }
    }
}
impl Encodable for Address {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        match self {
            Address::Ipv4(sock) => {
                let addr_type: u8 = 0;
                let len = Encodable::consensus_encode(&addr_type, &mut s)?
                    + Encodable::consensus_encode(&sock.ip().octets(), &mut s)?
                    + Encodable::consensus_encode(&sock.port().to_be(), &mut s)?;
                Ok(len)
            }
            Address::Ipv6(sock) => {
                let addr_type: u8 = 1;
                let len = Encodable::consensus_encode(&addr_type, &mut s)?
                    + Encodable::consensus_encode(&sock.ip().octets(), &mut s)?
                    + Encodable::consensus_encode(&sock.port().to_be(), &mut s)?;
                Ok(len)
            }
            Address::OnionV3(b, p) => {
                let addr_type: u8 = 2;
                let len = Encodable::consensus_encode(&addr_type, &mut s)?
                    + s.write(b)?
                    + Encodable::consensus_encode(&p.to_be(), &mut s)?;
                Ok(len)
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Version {
    pub major: u16, // used only 10 bits
    pub minor: u16,
    pub patch: u16,
}

impl Default for Version {
    fn default() -> Self {
        Version::current()
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    /// Current implemented version
    pub fn current() -> Self {
        Version {
            major: 2,
            minor: 0,
            patch: 0,
        }
    }

    /// Check whether versions compatible
    pub fn compatible(&self, v: &Self) -> bool {
        self.major == v.major
    }

    /// Pack version as 32 bit word with 10 bits per component and 2 reserved bits.
    pub fn pack(&self) -> u32 {
        (((self.major & 0b000001111111111) as u32) << 2)
            + (((self.minor & 0b000001111111111) as u32) << 12)
            + (((self.patch & 0b000001111111111) as u32) << 22)
    }

    /// Unpack version from 32 bit word with 10 bits per component and 2 reserved bits.
    pub fn unpack(w: u32) -> Self {
        Version {
            major: ((w >> 2) & 0b000001111111111) as u16,
            minor: ((w >> 12) & 0b000001111111111) as u16,
            patch: ((w >> 22) & 0b000001111111111) as u16,
        }
    }

    fn pack_be(&self) -> u32 {
        self.pack().to_be()
    }
    fn unpack_be(w: u32) -> Self {
        Version::unpack(w.to_be())
    }
}
impl_pure_encodable!(Version, unpack_be, pack_be);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Message {
    Version(VersionMessage),
    VersionAck,
    GetFilters(FiltersReq),
    Filters(FiltersResp),
    Filter(FilterEvent),
    GetPeers,
    Peers(Vec<Address>),
    GetFee(Vec<Currency>),
    Fee(Vec<FeeResp>),
    PeerIntroduce(Vec<Address>),
    Reject(RejectMessage),
    Ping([u8; 8]),
    Pong([u8; 8]),
    GetRates(Vec<RateReq>),
    Rates(Vec<RateResp>),
    FullFilterInv,
    GetFullFilter,
    FullFilter(MemFilter),
    GetMemFilters,
    MemFilters(Vec<FilterPrefixPair>),
    GetMempool(Vec<TxPrefix>),
    MempoolChunk(MempoolChunkResp),
}

fn fmt_vec<T: Display>(v: &[T], f: &mut Formatter) -> std::fmt::Result {
    for (i, b) in v.iter().enumerate() {
        if i == v.len() - 1 {
            write!(f, "{}", b)?;
        } else {
            write!(f, "{}, ", b)?;
        }
    }
    Ok(())
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Version(msg) => msg.fmt(f),
            Message::VersionAck => write!(f, "verack"),
            Message::GetFilters(msg) => msg.fmt(f),
            Message::Filters(msg) => msg.fmt(f),
            Message::Filter(msg) => msg.fmt(f),
            Message::GetPeers => write!(f, "reqpeers"),
            Message::Peers(msg) => {
                write!(f, "peers: ")?;
                fmt_vec(msg, f)
            }
            Message::GetFee(msg) => {
                write!(f, "reqfee: ")?;
                fmt_vec(msg, f)
            }
            Message::Fee(msg) => {
                write!(f, "fee: ")?;
                fmt_vec(msg, f)
            }
            Message::PeerIntroduce(msg) => {
                write!(f, "peer announce: ")?;
                fmt_vec(msg, f)
            }
            Message::Reject(msg) => msg.fmt(f),
            Message::Ping(nonce) => write!(f, "ping {}", nonce.to_hex()),
            Message::Pong(nonce) => write!(f, "pong {}", nonce.to_hex()),
            Message::GetRates(msg) => {
                write!(f, "req rates: ")?;
                fmt_vec(msg, f)
            }
            Message::Rates(msg) => {
                write!(f, "rates: ")?;
                fmt_vec(msg, f)
            }
            Message::FullFilterInv => write!(f, "fullfilterinv"),
            Message::GetFullFilter => write!(f, "getfullfilter"),
            Message::FullFilter(msg) => {
                write!(f, "fullfilter: ")?;
                msg.fmt(f)
            }
            Message::GetMemFilters => write!(f, "getmemfilters"),
            Message::MemFilters(msg) => {
                write!(f, "memfilters: ")?;
                fmt_vec(msg, f)
            }
            Message::GetMempool(msg) => {
                write!(f, "getmempool: ")?;
                fmt_vec(msg, f)
            }
            Message::MempoolChunk(msg) => {
                write!(f, "mempoolchunk: ")?;
                msg.fmt(f)
            }
        }
    }
}

/// Maximum size of message in bytes
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

impl Message {
    pub fn id(&self) -> u32 {
        match self {
            Message::Version(_) => 0,
            Message::VersionAck => 1,
            Message::GetFilters(_) => 2,
            Message::Filters(_) => 3,
            Message::Filter(_) => 4,
            Message::GetPeers => 5,
            Message::Peers(_) => 6,
            Message::GetFee(_) => 7,
            Message::Fee(_) => 8,
            Message::PeerIntroduce(_) => 9,
            Message::Reject(_) => 10,
            Message::Ping(_) => 11,
            Message::Pong(_) => 12,
            Message::GetRates(_) => 13,
            Message::Rates(_) => 14,
            Message::FullFilterInv => 15,
            Message::GetFullFilter => 16,
            Message::FullFilter(_) => 17,
            Message::GetMemFilters => 18,
            Message::MemFilters(_) => 19,
            Message::GetMempool(_) => 20,
            Message::MempoolChunk(_) => 21,
        }
    }

    /// Get human readable name of message by its type id
    pub fn name_from_id(i: u32) -> Option<&'static str> {
        match i {
            0 => Some("version"),
            1 => Some("version ack"),
            2 => Some("req filters"),
            3 => Some("filters"),
            4 => Some("filter"),
            5 => Some("req peers"),
            6 => Some("peers"),
            7 => Some("req fee"),
            8 => Some("fee"),
            9 => Some("peer announce"),
            10 => Some("reject"),
            11 => Some("ping"),
            12 => Some("pong"),
            13 => Some("req rates"),
            14 => Some("rates"),
            15 => Some("full filter inv"),
            17 => Some("full fulter"),
            18 => Some("get mempool filters"),
            19 => Some("mempool filters"),
            20 => Some("get mempool"),
            21 => Some("mempool chunk"),
            _ => None,
        }
    }
}

impl Encodable for Message {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        fn write_payload<S, T>(mut s: &mut S, v: &T) -> Result<usize, io::Error>
        where
            S: io::Write,
            T: Encodable + ?Sized,
        {
            let payload = serialize(v);
            let mut len = 0;
            len += VarInt(payload.len() as u64).consensus_encode(&mut s)?;
            len += s.write(&payload)?;
            Ok(len)
        }

        let mut len = 0;
        len += VarInt(self.id() as u64).consensus_encode(&mut s)?;
        match self {
            Message::Version(msg) => len += write_payload(&mut s, msg)?,
            Message::VersionAck => (),
            Message::GetFilters(msg) => len += write_payload(&mut s, msg)?,
            Message::Filters(msg) => len += write_payload(&mut s, msg)?,
            Message::Filter(msg) => len += write_payload(&mut s, msg)?,
            Message::GetPeers => (),
            Message::Peers(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::GetFee(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::Fee(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::PeerIntroduce(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::Reject(msg) => len += write_payload(&mut s, msg)?,
            Message::Ping(msg) => len += write_payload(&mut s, msg)?,
            Message::Pong(msg) => len += write_payload(&mut s, msg)?,
            Message::GetRates(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::Rates(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::FullFilterInv => (),
            Message::GetFullFilter => (),
            Message::FullFilter(msg) => len += write_payload(&mut s, msg)?,
            Message::GetMemFilters => (),
            Message::MemFilters(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::GetMempool(msg) => len += write_payload(&mut s, &LengthVecRef(msg))?,
            Message::MempoolChunk(msg) => len += write_payload(&mut s, msg)?,
        }
        Ok(len)
    }
}

impl Decodable for Message {
    #[inline]
    fn consensus_decode<D: ::std::io::Read>(mut d: D) -> Result<Message, consensus_encode::Error> {
        fn read_payload<F, D>(mut d: &mut D, mut f: F) -> Result<Message, consensus_encode::Error>
        where
            F: FnMut(&mut [u8]) -> Result<Message, consensus_encode::Error>,
            D: io::Read,
        {
            let len = VarInt::consensus_decode(&mut d)?.0 as u32;
            if len as usize > MAX_MESSAGE_SIZE {
                Err(Error::ParseFailed("Message size is too large"))
            } else {
                let mut buf = vec![0; len as usize];
                d.read_exact(&mut buf)?;
                f(&mut buf)
            }
        }

        let id = VarInt::consensus_decode(&mut d)?.0 as u32;
        match id {
            0 => read_payload(&mut d, |buf| {
                Ok(Message::Version(deserialize::<VersionMessage>(&buf)?))
            }),
            1 => Ok(Message::VersionAck),
            2 => read_payload(&mut d, |buf| {
                Ok(Message::GetFilters(deserialize::<FiltersReq>(&buf)?))
            }),
            3 => read_payload(&mut d, |buf| {
                Ok(Message::Filters(deserialize::<FiltersResp>(&buf)?))
            }),
            4 => read_payload(&mut d, |buf| {
                Ok(Message::Filter(deserialize::<FilterEvent>(&buf)?))
            }),
            5 => Ok(Message::GetPeers),
            6 => read_payload(&mut d, |buf| {
                Ok(Message::Peers(deserialize::<LengthVec<Address>>(&buf)?.0))
            }),
            7 => read_payload(&mut d, |buf| {
                Ok(Message::GetFee(deserialize::<LengthVec<Currency>>(&buf)?.0))
            }),
            8 => read_payload(&mut d, |buf| {
                Ok(Message::Fee(deserialize::<LengthVec<FeeResp>>(&buf)?.0))
            }),
            9 => read_payload(&mut d, |buf| {
                Ok(Message::PeerIntroduce(
                    deserialize::<LengthVec<Address>>(&buf)?.0,
                ))
            }),
            10 => read_payload(&mut d, |buf| {
                Ok(Message::Reject(deserialize::<RejectMessage>(&buf)?))
            }),
            11 => read_payload(&mut d, |buf| {
                let mut nonce: [u8; 8] = Default::default();
                nonce.copy_from_slice(&buf[0..8]);
                Ok(Message::Ping(nonce))
            }),
            12 => read_payload(&mut d, |buf| {
                let mut nonce: [u8; 8] = Default::default();
                nonce.copy_from_slice(&buf[0..8]);
                Ok(Message::Pong(nonce))
            }),
            13 => read_payload(&mut d, |buf| {
                Ok(Message::GetRates(
                    deserialize::<LengthVec<RateReq>>(&buf)?.0,
                ))
            }),
            14 => read_payload(&mut d, |buf| {
                Ok(Message::Rates(deserialize::<LengthVec<RateResp>>(&buf)?.0))
            }),
            15 => Ok(Message::FullFilterInv),
            16 => Ok(Message::GetFullFilter),
            17 => read_payload(&mut d, |buf| {
                Ok(Message::FullFilter(deserialize::<MemFilter>(&buf)?))
            }),
            18 => Ok(Message::GetMemFilters),
            19 => read_payload(&mut d, |buf| {
                Ok(Message::MemFilters(
                    deserialize::<LengthVec<FilterPrefixPair>>(&buf)?.0,
                ))
            }),
            20 => read_payload(&mut d, |buf| {
                Ok(Message::GetMempool(
                    deserialize::<LengthVec<TxPrefix>>(&buf)?.0,
                ))
            }),
            21 => read_payload(&mut d, |buf| {
                Ok(Message::MempoolChunk(deserialize::<MempoolChunkResp>(
                    &buf,
                )?))
            }),
            _ => Err(Error::ParseFailed("Unknown message type")),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct ScanBlock {
    pub currency: Currency,
    pub version: Version,
    pub scan_height: u64,
    pub height: u64,
}

impl Display for ScanBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (v{}) {}/{}",
            self.currency, self.version, self.scan_height, self.height
        )
    }
}

impl Encodable for ScanBlock {
    #[inline]
    fn consensus_encode<S: ::std::io::Write>(&self, mut s: S) -> Result<usize, ::std::io::Error> {
        let mut len = 0;
        len += self.currency.consensus_encode(&mut s)?;
        len += self.version.consensus_encode(&mut s)?;
        len += VarInt(self.scan_height).consensus_encode(&mut s)?;
        len += VarInt(self.height).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for ScanBlock {
    #[inline]
    fn consensus_decode<D: ::std::io::Read>(
        mut d: D,
    ) -> Result<ScanBlock, consensus_encode::Error> {
        Ok(ScanBlock {
            currency: Decodable::consensus_decode(&mut d)?,
            version: Decodable::consensus_decode(&mut d)?,
            scan_height: VarInt::consensus_decode(&mut d)?.0,
            height: VarInt::consensus_decode(&mut d)?.0,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct VersionMessage {
    pub version: Version,
    pub time: u64,
    pub nonce: [u8; 8],
    pub scan_blocks: Vec<ScanBlock>,
}

impl Display for VersionMessage {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Version {} at {} with nonce {} and currencies:",
            self.version,
            self.time,
            self.nonce.to_hex()
        )?;
        fmt_vec(&self.scan_blocks, f)
    }
}

impl Encodable for VersionMessage {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.version.consensus_encode(&mut s)?;
        len += self.time.consensus_encode(&mut s)?;
        len += self.nonce.consensus_encode(&mut s)?;
        len += LengthVecRef(&self.scan_blocks).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for VersionMessage {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<VersionMessage, consensus_encode::Error> {
        Ok(VersionMessage {
            version: Decodable::consensus_decode(&mut d)?,
            time: Decodable::consensus_decode(&mut d)?,
            nonce: Decodable::consensus_decode(&mut d)?,
            scan_blocks: LengthVec::consensus_decode(&mut d)?.0,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct FiltersReq {
    pub currency: Currency,
    pub start: u64,
    pub amount: u32,
}

impl Display for FiltersReq {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "req {} {} filters from {}",
            self.amount, self.currency, self.start
        )
    }
}

impl Encodable for FiltersReq {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.currency.consensus_encode(&mut s)?;
        len += VarInt(self.start).consensus_encode(&mut s)?;
        len += VarInt(self.amount as u64).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for FiltersReq {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<FiltersReq, consensus_encode::Error> {
        Ok(FiltersReq {
            currency: Decodable::consensus_decode(&mut d)?,
            start: VarInt::consensus_decode(&mut d)?.0,
            amount: VarInt::consensus_decode(&mut d)?.0 as u32,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Filter {
    pub block_id: Vec<u8>,
    pub filter: Vec<u8>,
}

impl Display for Filter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Block {}, filter {}",
            self.block_id.to_hex(),
            self.filter.to_hex()
        )
    }
}

impl Encodable for Filter {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += s.write(&self.block_id)?;
        len += self.filter.consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for Filter {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<Filter, consensus_encode::Error> {
        let bid: [u8; 32] = Decodable::consensus_decode(&mut d)?;
        Ok(Filter {
            block_id: bid.to_vec(),
            filter: Decodable::consensus_decode(&mut d)?,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct FiltersResp {
    pub currency: Currency,
    pub filters: Vec<Filter>,
}

impl FiltersResp {
    pub fn compress<'a, I: Iterator<Item = &'a Filter>>(filters: I) -> Result<Vec<u8>, io::Error> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        for f in filters {
            e.write_all(&serialize(&f))?;
        }
        e.finish()
    }

    pub fn decompress(buf: &[u8]) -> Result<Vec<u8>, io::Error> {
        let mut gz = GzDecoder::new(Vec::new());
        gz.write_all(&buf)?;
        gz.finish()
    }
}

impl Display for FiltersResp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Filters for {}:", self.currency)?;
        fmt_vec(&self.filters, f)
    }
}

impl Encodable for FiltersResp {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.currency.consensus_encode(&mut s)?;
        len += VarInt(self.filters.len() as u64).consensus_encode(&mut s)?;

        let compressed_bytes = FiltersResp::compress(self.filters.iter())?;
        s.write_all(&compressed_bytes)?;
        len += compressed_bytes.len();

        Ok(len)
    }
}

impl Decodable for FiltersResp {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<FiltersResp, consensus_encode::Error> {
        let cur = Decodable::consensus_decode(&mut d)?;
        let amount = VarInt::consensus_decode(&mut d)?.0;

        let mut buf = vec![];
        d.read_to_end(&mut buf)?;
        let uncompressed = FiltersResp::decompress(&buf)?;

        let mut decoder = Cursor::new(uncompressed);
        let mut fs = vec![];
        for _ in 0..amount {
            fs.push(Filter::consensus_decode(&mut decoder)?);
        }

        Ok(FiltersResp {
            currency: cur,
            filters: fs,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct FilterEvent {
    pub currency: Currency,
    pub height: u64,
    pub block_id: Vec<u8>,
    pub filter: Vec<u8>,
}

impl Display for FilterEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "New {} filter at height {} and id {}, body: {}",
            self.currency,
            self.height,
            self.block_id.to_hex(),
            self.filter.to_hex()
        )
    }
}

impl Encodable for FilterEvent {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.currency.consensus_encode(&mut s)?;
        len += VarInt(self.height).consensus_encode(&mut s)?;
        s.write_all(&self.block_id)?;
        len += self.block_id.len();
        len += self.filter.consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for FilterEvent {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<FilterEvent, consensus_encode::Error> {
        let mut buf = [0; 32];
        Ok(FilterEvent {
            currency: Decodable::consensus_decode(&mut d)?,
            height: VarInt::consensus_decode(&mut d)?.0,
            block_id: {
                d.read_exact(&mut buf)?;
                buf.to_vec()
            },
            filter: Decodable::consensus_decode(&mut d)?,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct FeeBtc {
    pub fast_conserv: u64,
    pub fast_econom: u64,
    pub moderate_conserv: u64,
    pub moderate_econom: u64,
    pub cheap_conserv: u64,
    pub cheap_econom: u64,
}

impl Display for FeeBtc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Fast(conservative {} sat/byte, econom {} sat/byte)",
            self.fast_conserv, self.fast_econom
        )?;
        writeln!(
            f,
            "Moderate(conservative {} sat/byte, econom {} sat/byte)",
            self.moderate_conserv, self.moderate_econom
        )?;
        writeln!(
            f,
            "Cheap(conservative {} sat/byte, econom {} sat/byte)",
            self.cheap_conserv, self.cheap_econom
        )
    }
}

impl Encodable for FeeBtc {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += VarInt(self.fast_conserv).consensus_encode(&mut s)?;
        len += VarInt(self.fast_econom).consensus_encode(&mut s)?;
        len += VarInt(self.moderate_conserv).consensus_encode(&mut s)?;
        len += VarInt(self.moderate_econom).consensus_encode(&mut s)?;
        len += VarInt(self.cheap_conserv).consensus_encode(&mut s)?;
        len += VarInt(self.cheap_econom).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for FeeBtc {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<FeeBtc, consensus_encode::Error> {
        Ok(FeeBtc {
            fast_conserv: VarInt::consensus_decode(&mut d)?.0,
            fast_econom: VarInt::consensus_decode(&mut d)?.0,
            moderate_conserv: VarInt::consensus_decode(&mut d)?.0,
            moderate_econom: VarInt::consensus_decode(&mut d)?.0,
            cheap_conserv: VarInt::consensus_decode(&mut d)?.0,
            cheap_econom: VarInt::consensus_decode(&mut d)?.0,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct FeeOther {
    pub fast: u64,
    pub moderate: u64,
    pub cheap: u64,
}

impl Display for FeeOther {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Fast {} unit/byte", self.fast)?;
        writeln!(f, "Moderate {} unit/byte", self.moderate)?;
        writeln!(f, "Cheap {} unit/byte", self.cheap)
    }
}

impl Encodable for FeeOther {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += VarInt(self.fast).consensus_encode(&mut s)?;
        len += VarInt(self.moderate).consensus_encode(&mut s)?;
        len += VarInt(self.cheap).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for FeeOther {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<FeeOther, consensus_encode::Error> {
        Ok(FeeOther {
            fast: VarInt::consensus_decode(&mut d)?.0,
            moderate: VarInt::consensus_decode(&mut d)?.0,
            cheap: VarInt::consensus_decode(&mut d)?.0,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum FeeResp {
    Btc((Currency, FeeBtc)),
    Other((Currency, FeeOther)),
}

impl Display for FeeResp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FeeResp::Btc((cur, fee)) => {
                writeln!(f, "Fee for {}", cur)?;
                fee.fmt(f)
            }
            FeeResp::Other((cur, fee)) => {
                writeln!(f, "Fee for {}", cur)?;
                fee.fmt(f)
            }
        }
    }
}

impl Encodable for FeeResp {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        match self {
            FeeResp::Btc((currency, fee)) => {
                assert!(
                    *currency == Currency::Btc || *currency == Currency::TBtc,
                    "FeeBtc currency must be Btc or TBtc!"
                );
                len += currency.consensus_encode(&mut s)?;
                len += fee.consensus_encode(&mut s)?
            }
            FeeResp::Other((currency, fee)) => {
                len += currency.consensus_encode(&mut s)?;
                len += fee.consensus_encode(&mut s)?
            }
        }
        Ok(len)
    }
}

impl Decodable for FeeResp {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<FeeResp, consensus_encode::Error> {
        let cur = Currency::consensus_decode(&mut d)?;
        match cur {
            Currency::Btc | Currency::TBtc => {
                Ok(FeeResp::Btc((cur, Decodable::consensus_decode(&mut d)?)))
            }
            _ => Ok(FeeResp::Other((cur, Decodable::consensus_decode(&mut d)?))),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum RejectData {
    HeaderParsing,
    PayloadParsing,
    InternalError,
    ZeroBytesReceived,
    VersionNotSupported,
    Unknown(u32),
}

impl Display for RejectData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RejectData::HeaderParsing => write!(f, "header parsing"),
            RejectData::PayloadParsing => write!(f, "payload parsing"),
            RejectData::InternalError => write!(f, "internal error"),
            RejectData::ZeroBytesReceived => write!(f, "got zero bytes"),
            RejectData::VersionNotSupported => write!(f, "version is not supported"),
            RejectData::Unknown(i) => write!(f, "unknown error {}", i),
        }
    }
}

impl RejectData {
    pub fn to_code(&self) -> u32 {
        match self {
            RejectData::HeaderParsing => 0,
            RejectData::PayloadParsing => 1,
            RejectData::InternalError => 2,
            RejectData::ZeroBytesReceived => 3,
            RejectData::VersionNotSupported => 4,
            RejectData::Unknown(i) => *i,
        }
    }

    pub fn from_code(i: u32) -> Self {
        match i {
            0 => RejectData::HeaderParsing,
            1 => RejectData::PayloadParsing,
            2 => RejectData::InternalError,
            3 => RejectData::ZeroBytesReceived,
            4 => RejectData::VersionNotSupported,
            i => RejectData::Unknown(i),
        }
    }

    fn pack(&self) -> VarInt {
        VarInt(self.to_code() as u64)
    }

    fn unpack(i: VarInt) -> Self {
        RejectData::from_code(i.0 as u32)
    }
}
impl_pure_encodable!(RejectData, unpack, pack);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct RejectMessage {
    pub id: u32,
    pub data: RejectData,
    pub message: String,
}

impl Display for RejectMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "reject {} for {}, reason: {}",
            self.data,
            Message::name_from_id(self.id).unwrap_or("unknown"),
            self.message
        )
    }
}

impl Encodable for RejectMessage {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += VarInt(self.id as u64).consensus_encode(&mut s)?;
        len += self.data.consensus_encode(&mut s)?;
        len += self.message.consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for RejectMessage {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<RejectMessage, consensus_encode::Error> {
        Ok(RejectMessage {
            id: VarInt::consensus_decode(&mut d)?.0 as u32,
            data: Decodable::consensus_decode(&mut d)?,
            message: Decodable::consensus_decode(&mut d)?,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct RateReq {
    pub currency: Currency,
    pub fiats: Vec<Fiat>,
}

impl Display for RateReq {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} for ", self.currency)?;
        fmt_vec(&self.fiats, f)
    }
}

impl Encodable for RateReq {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.currency.consensus_encode(&mut s)?;
        len += LengthVecRef(&self.fiats).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for RateReq {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<RateReq, consensus_encode::Error> {
        Ok(RateReq {
            currency: Decodable::consensus_decode(&mut d)?,
            fiats: LengthVec::consensus_decode(&mut d)?.0,
        })
    }
}

/// Fiat value with 2 decimals after point
pub type Rate = Centi<u64>;

struct RateWord(Rate);

impl Encodable for RateWord {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let w: u64 = self.0.bits;
        let len = w.consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for RateWord {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<RateWord, consensus_encode::Error> {
        let w: u64 = Decodable::consensus_decode(&mut d)?;
        Ok(RateWord(Rate::new(w)))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct FiatRate {
    pub fiat: Fiat,
    pub rate: Rate,
}

impl Display for FiatRate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.rate.bits as f64 / 100.0, self.fiat)
    }
}

impl Encodable for FiatRate {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.fiat.consensus_encode(&mut s)?;
        len += RateWord(self.rate).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for FiatRate {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<FiatRate, consensus_encode::Error> {
        Ok(FiatRate {
            fiat: Decodable::consensus_decode(&mut d)?,
            rate: RateWord::consensus_decode(&mut d)?.0,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct RateResp {
    pub currency: Currency,
    pub rates: Vec<FiatRate>,
}

impl Display for RateResp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} rates: ", self.currency)?;
        fmt_vec(&self.rates, f)
    }
}

impl Encodable for RateResp {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.currency.consensus_encode(&mut s)?;
        len += LengthVecRef(&self.rates).consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for RateResp {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<RateResp, consensus_encode::Error> {
        Ok(RateResp {
            currency: Decodable::consensus_decode(&mut d)?,
            rates: LengthVec::consensus_decode(&mut d)?.0,
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct MemFilter(pub Vec<u8>);

impl MemFilter {
    pub fn compress(&self) -> Result<Vec<u8>, io::Error> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(&self.0)?;
        e.finish()
    }

    pub fn decompress(buf: &[u8]) -> Result<MemFilter, io::Error> {
        let mut gz = GzDecoder::new(Vec::new());
        gz.write_all(&buf)?;
        gz.finish().map(MemFilter)
    }
}

impl Display for MemFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mempool filter {}", self.0.to_hex())
    }
}

impl Encodable for MemFilter {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        self.0.consensus_encode(&mut s)
    }
}

impl Decodable for MemFilter {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<MemFilter, consensus_encode::Error> {
        Decodable::consensus_decode(&mut d).map(MemFilter)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct TxPrefix(pub [u8; 2]);

impl Display for TxPrefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Prefix: {}", self.0.to_hex())
    }
}

impl Encodable for TxPrefix {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        self.0.consensus_encode(&mut s)
    }
}

impl Decodable for TxPrefix {
    #[inline]
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<TxPrefix, consensus_encode::Error> {
        Decodable::consensus_decode(&mut d).map(TxPrefix)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct FilterPrefixPair {
    pub prefix: TxPrefix,
    pub filter: MemFilter,
}

impl Display for FilterPrefixPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Prefix: {:?}; ", self.prefix)?;
        self.filter.fmt(f)
    }
}

impl Encodable for FilterPrefixPair {
    #[inline]
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.prefix.consensus_encode(&mut s)?;
        len += self.filter.consensus_encode(&mut s)?;
        Ok(len)
    }
}

impl Decodable for FilterPrefixPair {
    #[inline]
    fn consensus_decode<D: io::Read>(
        mut d: D,
    ) -> Result<FilterPrefixPair, consensus_encode::Error> {
        let prefix = Decodable::consensus_decode(&mut d)?;
        let filter = Decodable::consensus_decode(&mut d)?;
        Ok(FilterPrefixPair { prefix, filter })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct MempoolChunkResp {
    pub prefix: TxPrefix,
    pub txs: Vec<Vec<u8>>, //byte repersentation of transactions
}

impl MempoolChunkResp {
    pub fn compress<'a, I: Iterator<Item = &'a Vec<u8>>>(filters: I) -> Result<Vec<u8>, io::Error> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        for f in filters {
            e.write_all(&serialize(&f))?;
        }
        e.finish()
    }

    pub fn decompress(buf: &[u8]) -> Result<Vec<u8>, io::Error> {
        let mut gz = GzDecoder::new(Vec::new());
        gz.write_all(&buf)?;
        gz.finish()
    }
}

impl Display for MempoolChunkResp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mempool chunk: {}:{}:", self.prefix, self.txs.len())?;
        let v : Vec<String> = self.txs.iter().map(|tx| tx.to_hex()).collect();
        fmt_vec(&v, f)
    }
}

impl Encodable for MempoolChunkResp {
    fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.prefix.consensus_encode(&mut s)?;
        len += VarInt(self.txs.len() as u64).consensus_encode(&mut s)?;
        let compressed_bytes = MempoolChunkResp::compress(self.txs.iter())?;
        s.write_all(&compressed_bytes)?;
        len += compressed_bytes.len();
        Ok(len)
    }
}

impl Decodable for MempoolChunkResp {
    fn consensus_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
        let prefix = Decodable::consensus_decode(&mut d)?;
        let amount = VarInt::consensus_decode(&mut d)?.0 as u32;
        let mut buf = vec![];
        d.read_to_end(&mut buf)?;
        let uncompressed = FiltersResp::decompress(&buf)?;

        let mut decoder = Cursor::new(uncompressed);
        let mut txs = vec![];
        for _ in 0..amount {
            txs.push(LengthVec::consensus_decode(&mut decoder)?.0);
        }

        Ok(MempoolChunkResp { prefix, txs })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use consensus_encode::util::hex::FromHex;
    use rand::Rng;

    #[test]
    fn version_test_1() {
        let ver = Version {
            major: 1,
            minor: 2,
            patch: 4,
        };
        assert_eq!(ver.pack(), 0b0000000100_0000000010_0000000001_00);
        assert_eq!(Version::unpack(0b0000000100_0000000010_0000000001_00), ver);
    }

    #[test]
    fn version_test_2() {
        let ver = Version {
            major: 2,
            minor: 0,
            patch: 0,
        };
        assert_eq!(ver.pack(), 0b0000000000_0000000000_0000000010_00);
        assert_eq!(Version::unpack(0b0000000000_0000000000_0000000010_00), ver);
    }

    #[test]
    fn version_compat_self() {
        assert!(
            Version::current().compatible(&Version::current()),
            "Version is compatible to self"
        );
    }

    #[test]
    fn address_test_v4() {
        let addr = Address::Ipv4(net::SocketAddrV4::new(
            net::Ipv4Addr::new(127, 0, 0, 1),
            4142,
        ));
        let bytes = vec![0, 127, 0, 0, 1, 0x10, 0x2E];
        assert_eq!(serialize(&addr), bytes);
        assert_eq!(deserialize::<Address>(&bytes).unwrap(), addr);
    }

    #[test]
    fn address_test_v6() {
        let addr = Address::Ipv6(net::SocketAddrV6::new(
            net::Ipv6Addr::new(
                0x2001, 0x0db8, 0x85a3, 0x0000, 0x0000, 0x8a2e, 0x0370, 0x7334,
            ),
            4142,
            0,
            0,
        ));
        let bytes = vec![
            1, 0x20, 0x01, 0x0d, 0xb8, 0x85, 0xa3, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03, 0x70,
            0x73, 0x34, 0x10, 0x2E,
        ];
        assert_eq!(serialize(&addr), bytes);
        assert_eq!(deserialize::<Address>(&bytes).unwrap(), addr);
    }

    #[test]
    fn address_test_onion() {
        let addr = Address::OnionV3(
            *b"jamie22ezawwi5r3o7lrgsno43jj7vq5en74czuw6wfmjzkhjjryxnid",
            9150,
        );
        let bytes = vec![
            2, 106, 97, 109, 105, 101, 50, 50, 101, 122, 97, 119, 119, 105, 53, 114, 51, 111, 55,
            108, 114, 103, 115, 110, 111, 52, 51, 106, 106, 55, 118, 113, 53, 101, 110, 55, 52, 99,
            122, 117, 119, 54, 119, 102, 109, 106, 122, 107, 104, 106, 106, 114, 121, 120, 110,
            105, 100, 35, 190,
        ];
        assert_eq!(serialize(&addr), bytes);
        assert_eq!(deserialize::<Address>(&bytes).unwrap(), addr);
    }

    #[test]
    fn version_msg_test() {
        let msg = Message::Version(VersionMessage {
            version: Version {
                major: 1,
                minor: 2,
                patch: 4,
            },
            time: 1615562102,
            nonce: [0, 1, 2, 3, 4, 5, 6, 7],
            scan_blocks: vec![
                ScanBlock {
                    currency: Currency::Btc,
                    version: Version {
                        major: 1,
                        minor: 2,
                        patch: 4,
                    },
                    scan_height: 674299,
                    height: 300000,
                },
                ScanBlock {
                    currency: Currency::Ergo,
                    version: Version {
                        major: 4,
                        minor: 1,
                        patch: 0,
                    },
                    scan_height: 374299,
                    height: 200000,
                },
            ],
        });
        let bytes = Vec::from_hex("00330100200476854b60000000000001020304050607020001002004fefb490a00fee09304000200001010fe1bb60500fe400d0300").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn version_msg_test_failed_version() {
        let msg = Message::Version(VersionMessage {
            version: Version {
                major: 2,
                minor: 0,
                patch: 0,
            },
            time: 1617118704,
            nonce: [25, 218, 220, 43, 225, 52, 200, 125],
            scan_blocks: vec![ScanBlock {
                currency: Currency::Btc,
                version: Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                scan_height: 677013,
                height: 677013,
            }],
        });
        let bytes = Vec::from_hex(
            "002400000008f04563600000000019dadc2be134c87d010000000004fe95540a00fe95540a00",
        )
        .unwrap();
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
        assert_eq!(serialize(&msg), bytes);
    }

    #[test]
    fn verack_msg_test() {
        let msg = Message::VersionAck;
        let bytes = Vec::from_hex("01").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn ping_msg_test() {
        let msg = Message::Ping([0xCF, 0x78, 0x06, 0, 0, 0, 0, 0]);
        let bytes = Vec::from_hex("0b08cf78060000000000").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn pong_msg_test() {
        let msg = Message::Pong([0xCF, 0x78, 0x06, 0, 0, 0, 0, 0]);
        let bytes = Vec::from_hex("0c08cf78060000000000").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn reject_msg_test() {
        let msg = Message::Reject(RejectMessage {
            id: 2,
            data: RejectData::InternalError,
            message: "Something went wrong".to_string(),
        });
        let bytes = Vec::from_hex("0a17020214536f6d657468696e672077656e742077726f6e67").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn filters_req_test() {
        let msg = Message::GetFilters(FiltersReq {
            currency: Currency::Btc,
            start: 445123,
            amount: 2000,
        });
        let bytes = Vec::from_hex("020900fec3ca0600fdd007").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn filters_req_test2() {
        let msg = Message::GetFilters(FiltersReq {
            currency: Currency::Btc,
            start: 400000,
            amount: 300,
        });
        let bytes = Vec::from_hex("020900fe801a0600fd2c01").unwrap();
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
        assert_eq!(serialize(&msg), bytes);
    }

    struct Header {
        message_id: u32,
        message_length: u64,
        message_payload: Vec<u8>,
    }

    impl Encodable for Header {
        #[inline]
        fn consensus_encode<S: io::Write>(&self, mut s: S) -> Result<usize, io::Error> {
            let mut len = 0;
            len += VarInt(self.message_id as u64).consensus_encode(&mut s)?;
            len += VarInt(self.message_length).consensus_encode(&mut s)?;
            len += self.message_payload.consensus_encode(&mut s)?;
            Ok(len)
        }
    }

    impl Decodable for Header {
        #[inline]
        fn consensus_decode<D: ::std::io::Read>(
            mut d: D,
        ) -> Result<Header, consensus_encode::Error> {
            let id = VarInt::consensus_decode(&mut d)?.0 as u32;
            let len = VarInt::consensus_decode(&mut d)?.0 as u64;

            let mut buf = vec![0; len as usize];
            d.read_exact(&mut buf)?;

            Ok(Header {
                message_id: id,
                message_length: len,
                message_payload: buf,
            })
        }
    }

    #[test]
    fn filters_resp_test() {
        let msg = Message::Filters(FiltersResp {
            currency: Currency::Btc,
            filters: vec![
                Filter {
                    block_id: b"12345678123456781234567812345678".to_vec(),
                    filter: b"abcd".to_vec(),
                },
                Filter {
                    block_id: b"22345678123456781234567812345678".to_vec(),
                    filter: b"ffff".to_vec(),
                },
            ],
        });
        let bytes = Vec::from_hex("032b00021f8b080000000000000333343236313533b730c441b3242625a71811529406040096e289844a000000").unwrap();
        let our_bytes = serialize(&msg);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
        assert_eq!(deserialize::<Message>(&our_bytes).unwrap(), msg);

        let header = deserialize::<Header>(&our_bytes).unwrap();
        assert_eq!(header.message_id, 3);
        assert_eq!(header.message_length, header.message_payload.len() as u64);
    }

    #[test]
    fn filter_event_test() {
        let msg = Message::Filter(FilterEvent {
            currency: Currency::Btc,
            height: 8083,
            block_id: b"12345678123456781234567812345678".to_vec(),
            filter: b"abcd".to_vec(),
        });
        let bytes = Vec::from_hex("042900fd931f31323334353637383132333435363738313233343536373831323334353637380461626364").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn fee_req_test() {
        let msg = Message::GetFee(vec![Currency::Btc, Currency::Dash]);
        let bytes = Vec::from_hex("070302000c").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn fee_resp_test() {
        let msg = Message::Fee(vec![
            FeeResp::Btc((
                Currency::Btc,
                FeeBtc {
                    fast_conserv: 4,
                    fast_econom: 8,
                    moderate_conserv: 15,
                    moderate_econom: 16,
                    cheap_conserv: 23,
                    cheap_econom: 42,
                },
            )),
            FeeResp::Other((
                Currency::Dash,
                FeeOther {
                    fast: 4,
                    moderate: 8,
                    cheap: 15,
                },
            )),
        ]);
        let bytes = Vec::from_hex("080c020004080f10172a0c04080f").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn peer_req_test() {
        let msg = Message::GetPeers;
        let bytes = Vec::from_hex("05").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn peer_resp_test() {
        let msg = Message::Peers(vec![
            Address::Ipv4(net::SocketAddrV4::new(
                net::Ipv4Addr::new(127, 0, 0, 1),
                8333,
            )),
            Address::Ipv6(net::SocketAddrV6::new(
                net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
                8333,
                0,
                0,
            )),
            Address::OnionV3(
                *b"jamie22ezawwi5r3o7lrgsno43jj7vq5en74czuw6wfmjzkhjjryxnid",
                9150,
            ),
        ]);
        let bytes = Vec::from_hex("065603007f000001208d0100000000000000000000000000000001208d026a616d69653232657a617777693572336f376c7267736e6f34336a6a37767135656e3734637a75773677666d6a7a6b686a6a7279786e696423be").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn rates_req_test() {
        let msg = Message::GetRates(vec![
            RateReq {
                currency: Currency::Btc,
                fiats: vec![Fiat::Usd, Fiat::Rub],
            },
            RateReq {
                currency: Currency::Dash,
                fiats: vec![Fiat::Eur, Fiat::Rub],
            },
        ]);
        let bytes = Vec::from_hex("0d0902000200020c020102").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn rates_resp_test() {
        let msg = Message::Rates(vec![
            RateResp {
                currency: Currency::Btc,
                rates: vec![
                    FiatRate {
                        fiat: Fiat::Usd,
                        rate: Rate::new(65003_23),
                    },
                    FiatRate {
                        fiat: Fiat::Rub,
                        rate: Rate::new(350000_42),
                    },
                ],
            },
            RateResp {
                currency: Currency::Dash,
                rates: vec![
                    FiatRate {
                        fiat: Fiat::Eur,
                        rate: Rate::new(0_12),
                    },
                    FiatRate {
                        fiat: Fiat::Rub,
                        rate: Rate::new(0_01),
                    },
                ],
            },
        ]);
        let bytes = Vec::from_hex("0e2902000200e32f63000000000002ea0e1602000000000c02010c00000000000000020100000000000000").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn peer_introduce_test() {
        let msg = Message::PeerIntroduce(vec![
            Address::Ipv4(net::SocketAddrV4::new(
                net::Ipv4Addr::new(127, 0, 0, 1),
                8333,
            )),
            Address::Ipv6(net::SocketAddrV6::new(
                net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
                8333,
                0,
                0,
            )),
            Address::OnionV3(
                *b"jamie22ezawwi5r3o7lrgsno43jj7vq5en74czuw6wfmjzkhjjryxnid",
                9150,
            ),
        ]);
        let bytes = Vec::from_hex("095603007f000001208d0100000000000000000000000000000001208d026a616d69653232657a617777693572336f376c7267736e6f34336a6a37767135656e3734637a75773677666d6a7a6b686a6a7279786e696423be").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn full_filter_inv_test() {
        let msg = Message::FullFilterInv;
        let bytes = Vec::from_hex("0f").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn get_full_filter_test() {
        let msg = Message::GetFullFilter;
        let bytes = Vec::from_hex("10").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn full_filter_test() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let mut f: Vec<u8> = Vec::new();
            let n = rng.gen_range(0..1024);
            (0..n).for_each(|_| {
                f.push(rng.gen());
            });
            let msg = Message::FullFilter(MemFilter(f));
            let bytes = serialize(&msg);
            assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
        }

        let msg = Message::FullFilter(MemFilter(Vec::from_hex("13ef4d423ac40d6e5e287611d82f69b1d75b22e30f3a78a9fe0985f45c45757a2efc71f20f491be782c84926941d40499d3238").unwrap()));
        let bytes = Vec::from_hex("11343313ef4d423ac40d6e5e287611d82f69b1d75b22e30f3a78a9fe0985f45c45757a2efc71f20f491be782c84926941d40499d3238").unwrap();

        assert_eq!(msg, deserialize(&bytes).unwrap());
    }

    #[test]
    fn get_mem_filters_test() {
        let msg = Message::GetMemFilters;
        let bytes = Vec::from_hex("12").unwrap();
        assert_eq!(serialize(&msg), bytes);
        assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
    }

    #[test]
    fn mem_filters_test() {
        let mut rng = rand::thread_rng();
        for _ in 0..10 {
            let mut fps: Vec<FilterPrefixPair> = Vec::new();
            let n = rng.gen_range(0..1024);
            (0..n).for_each(|_| {
                let prefix = TxPrefix(rng.gen());
                let m = rng.gen_range(0..1024);
                let mut f: Vec<u8> = Vec::with_capacity(m);
                (0..m).for_each(|_| {
                    f.push(rng.gen());
                });
                let filter = MemFilter(f);
                let fp = FilterPrefixPair { prefix, filter };
                fps.push(fp)
            });
            let msg = Message::MemFilters(fps);
            let bytes = serialize(&msg);
            assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
        }

        let mut fpairs = vec![
            FilterPrefixPair {
                prefix: TxPrefix([8, 192]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([18, 192]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([54, 192]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([195, 192]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([9, 128]),
                filter: MemFilter(Vec::from_hex("022d7f2b50e000").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([248, 0]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([72, 64]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([136, 64]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([10, 0]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([110, 64]),
                filter: MemFilter(Vec::from_hex("031b48714c5a3d93d0").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([209, 192]),
                filter: MemFilter(Vec::from_hex("02a49bfc5e31c0").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([121, 64]),
                filter: MemFilter(Vec::from_hex("0189ece0").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([231, 64]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([13, 0]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([39, 0]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([213, 192]),
                filter: MemFilter(Vec::from_hex("039776c95c592b27d0").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([98, 128]),
                filter: MemFilter(Vec::from_hex("01982900").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([171, 128]),
                filter: MemFilter(Vec::from_hex("014f0dd0").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([247, 0]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([39, 128]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([113, 64]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([150, 192]),
                filter: MemFilter(Vec::from_hex("030a2d1d7494236508").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([95, 0]),
                filter: MemFilter(Vec::from_hex("032704d4082c545300").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([181, 128]),
                filter: MemFilter(Vec::from_hex("031ddc812f62d786d8").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([202, 0]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([104, 192]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([201, 64]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([31, 128]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([29, 0]),
                filter: MemFilter(Vec::from_hex("00").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([114, 192]),
                filter: MemFilter(Vec::from_hex("0240f91b4f8b80").unwrap()),
            },
            FilterPrefixPair {
                prefix: TxPrefix([162, 64]),
                filter: MemFilter(Vec::from_hex("0253954696ad").unwrap()),
            },
        ];
        fpairs.sort(); // Input hex was sorted
        let msg = Message::MemFilters(fpairs);
        let bytes = Vec::from_hex("13c51f08c00100098007022d7f2b50e0000a0001000d00010012c001001d0001001f800100270001002780010036c00100484001005f0009032704d4082c5453006280040198290068c001006e4009031b48714c5a3d93d07140010072c0070240f91b4f8b807940040189ece08840010096c009030a2d1d7494236508a240060253954696adab8004014f0dd0b58009031ddc812f62d786d8c3c00100c9400100ca000100d1c00702a49bfc5e31c0d5c009039776c95c592b27d0e7400100f7000100f8000100").unwrap();
        if let Message::MemFilters(msg2) = deserialize::<Message>(&bytes).unwrap() {
            for FilterPrefixPair { prefix, filter } in msg2.iter() {
                println!("{}:{}", prefix, filter.0.to_hex());
            }
        }
        assert_eq!(msg, msg);
    }

    #[test]
    fn get_mempool_test() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let mut prefs: Vec<[u8; 2]> = Vec::new();
            let n = rng.gen_range(0..1024);
            (0..n).for_each(|_| {
                prefs.push(rng.gen());
            });
            let msg = Message::GetMempool(prefs.iter().map(|p| TxPrefix(p.clone())).collect());
            let bytes = serialize(&msg);
            assert_eq!(
                deserialize::<Message>(&bytes).unwrap(),
                msg,
                "Failed to deserialize random getmempool"
            );
        }

        let msg = Message::GetMempool(vec![TxPrefix([72, 12])]);
        let bytes = Vec::from_hex("140301480c").unwrap();
        assert_eq!(
            deserialize::<Message>(&bytes).unwrap(),
            msg,
            "Deserialized msg not equal to a specific one"
        );
    }

    #[test]
    fn mempool_chunk_test() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let prefix = TxPrefix(rng.gen());
            let amount: u32 = rng.gen_range(1..102);
            let mut txs: Vec<Vec<u8>> = Vec::new();
            (0..amount).for_each(|_| {
                let l = rng.gen_range(1..256);
                let mut tx = Vec::new();
                (0..l).for_each(|_| tx.push(rng.gen()));
                txs.push(tx);
            });
            let msg = Message::MempoolChunk(MempoolChunkResp { prefix, txs });
            let bytes = serialize(&msg);
            assert_eq!(deserialize::<Message>(&bytes).unwrap(), msg);
        }

        let tx = Vec::from_hex("02000000000101261560a27330e73b46351ac349ff35136f614d4dfdfb3a108fa85c140a1c61a901000000171600149fd77bca5b9369478c80dc5c5cc4101f7baf5a95feffffff0254c410000000000017a914ba906b3da20467de78552d0c089e3754f49f62688740420f000000000017a9140f912a6fc7ba91305934dba0ef566cbfc62fd2218702473044022045d75032c9f3806939ff10ffd79a040bdcbece2f90cb1dc95e3a3b7cf109da1e022012a37cc4fee1ff9ae19c6adf7d0bc84a122b5ce33d5c43bebff52a6796d512340121025609c093b93e3d4a003ebb0ec8e58700d12e6f05c0c1096f18ba3ef8ff931fca260d1b00").unwrap();
        let txs = vec![tx; 6];
        let prefix = TxPrefix([9, 128]);
        let msg = Message::MempoolChunk(MempoolChunkResp { prefix, txs });
        let bytes = Vec::from_hex("15fd1c010980061f8b0800000000000003fbcec400048c8c6aa2098b8a0d9e5bbb994a1df6fc6f2a9c9fe8ebfbf7b79540ff8a18112e99c4958c4055e2620c22f3af579f8a9e9ce9ded3702726e688807cf5faa8a9fffeffffcf14724400641283f84a915d13b26d17b1a4dfab08d5e5e198671ef2657e5246bb83133f4c9e7fa256fef15d130d224d6e2f781f96b3ff98fe25c576267703172605d7eb0146273f37645afe17f87f7d160bf79d7de7f4279c963d1967655df391f3961c9382d0e29a23ff1efe9ff5704ed6fd5aee135e42da318f6d639cf7edffaa953eedaa9009a3225318e781c93bed6cbd18ec76f39d78dace70512f9ff5c041ce7c895d763ffe4f963fa5c62bcdf07dd4dfa3fe1ef5f7a8bf8799bf015b13a09cd0050000").unwrap();
        assert_eq!(
            deserialize::<Message>(&bytes).unwrap(),
            msg,
            "Deserialized msg not equal to a specific one"
        );
    }
}
