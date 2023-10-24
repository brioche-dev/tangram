use super::xdr::{FromXdr, ToXdr};
use num::ToPrimitive;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct FileHandle {
	pub node: u64,
}

#[derive(Debug, Clone)]
pub struct FileAttr {
	pub attr_mask: Bitmap,
	pub attr_vals: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CallbackClient {
	pub program: u32,
	pub location: ClientAddr,
}

#[derive(Debug, Clone)]
pub struct ClientAddr {
	pub netid: String,
	pub addr: String,
}

#[derive(Clone)]
pub struct Bitmap(pub Vec<u32>);

pub struct FsId {
	pub major: u64,
	pub minor: u64,
}

pub type Cookie = u64;
pub type Count = u32;

#[derive(Debug, Clone)]
pub struct Entry {
	pub cookie: Cookie,
	pub name: String,
	pub attrs: FileAttr,
}

impl Bitmap {
	pub fn set(&mut self, bit: usize) {
		let word = bit / 32;
		if word >= self.0.len() {
			self.0.resize_with(word + 1, || 0);
		}
		self.0[word] |= 1 << (bit % 32);
	}

	pub fn get(&self, bit: usize) -> bool {
		let word = self.0.get(bit / 32).copied().unwrap_or(0);
		let flag = 1 & (word >> (bit % 32));
		flag != 0
	}

	pub fn intersection(&self, rhs: &Self) -> Self {
		let sz = self.0.len().max(rhs.0.len());
		let mut new = vec![0; sz];
		for (i, new) in new.iter_mut().enumerate() {
			let lhs = self.0.get(i).copied().unwrap_or(0);
			let rhs = rhs.0.get(i).copied().unwrap_or(0);
			*new = lhs & rhs;
		}
		Self(new)
	}
}

#[derive(Debug, Clone)]
pub struct Ace {
	pub type_: u32,
	pub flag: u32,
	pub mask: u32,
	pub who: String,
}

#[derive(Debug, Clone)]
pub struct FsLocations {
	pub fs_root: Vec<String>,
	pub locations: Vec<Location>,
}

#[derive(Debug, Clone)]
pub struct Location {
	pub server: Vec<String>,
	pub rootpath: Vec<String>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SpecData {
	pub specdata1: u32,
	pub specdata2: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Time {
	pub seconds: i64,
	pub nanos: u32,
}

impl Default for Time {
	fn default() -> Self {
		Self {
			seconds: 1,
			nanos: 0,
		}
	}
}

impl Time {
	pub fn now() -> Self {
		let now = std::time::SystemTime::now();
		let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap();
		Self {
			seconds: dur.as_secs().to_i64().unwrap(),
			nanos: dur.subsec_nanos(),
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct ChangeInfo {
	pub atomic: bool,
	pub before: u64,
	pub after: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StateId {
	pub seqid: u32,
	pub other: [u8; 12],
}

impl FromXdr for StateId {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let seqid = decoder.decode()?;
		let other = decoder.decode_n()?;
		Ok(Self { seqid, other })
	}
}

#[derive(Debug, Copy, Clone)]
pub enum OpenDelegation {
	None,
}

// https://datatracker.ietf.org/doc/html/rfc7530#section-16.16.2
// Note: there is an excessive amount of configuration data to support open with O_CREAT. We rely on the fact the server is allowed to reject this, however the rest of the Arg vec will be deserialized as garbage as a result.
#[derive(Debug, Clone)]
pub enum OpenFlags {
	Create,
	None,
}

#[derive(Debug, Clone)]
pub enum OpenClaim {
	// CLAIM_NULL: No special rights. Argument is the file name.
	Null(String),
	// CLAIM_PREVIOUS: Right to the file established by an open previous to server reboot. File identified by filehandle obtained previously rather than by name.
	Previous(OpenDelegationType),
	// CLAIM_DELEGATE_CUR: Right to file based on a delegation granted by the server. File is specified by name.
	DelegateCur {
		delegate_stateid: StateId,
		file: String,
		// todo: open claim delegate
	},
	// CLAIM_DELEGATE_PREV: Right to a file based on a delegation granted to a previous boot instance of the client. File is specified by name.
	DelegatePrev(String),
}

#[derive(Debug, Clone)]
pub enum OpenDelegationType {
	None,
	Read,
	Write,
}

#[derive(Debug, Clone)]
pub struct OpenOwner {
	pub clientid: u64,
	pub opaque: Vec<u8>,
}

impl ToXdr for FileHandle {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_opaque(&self.node.to_be_bytes())?;
		Ok(())
	}
}

impl FromXdr for FileHandle {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let decoded = decoder.decode_opaque()?;
		if decoded.len() != 8 {
			return Err(super::xdr::Error::Custom(
				"File handle size mismatch.".into(),
			));
		}
		let node = u64::from_be_bytes(decoded[0..8].try_into().unwrap());
		Ok(Self { node })
	}
}

impl ToXdr for ChangeInfo {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.atomic)?;
		encoder.encode(&self.before)?;
		encoder.encode(&self.after)?;
		Ok(())
	}
}

impl ToXdr for StateId {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.seqid)?;
		encoder.encode_n(self.other)?;
		Ok(())
	}
}

impl ToXdr for OpenDelegation {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		match self {
			Self::None => encoder.encode_int(0),
		}
	}
}

impl ToXdr for FileAttr {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.attr_mask)?;
		encoder.encode_opaque(&self.attr_vals)?;
		Ok(())
	}
}

impl FromXdr for FileAttr {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let attr_mask = decoder.decode()?;
		let attr_vals = decoder.decode_opaque()?.to_owned();
		Ok(Self {
			attr_mask,
			attr_vals,
		})
	}
}

impl ToXdr for CallbackClient {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_uint(self.program)?;
		encoder.encode(&self.location)?;
		Ok(())
	}
}

impl FromXdr for CallbackClient {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let program = decoder.decode_uint()?;
		let location = decoder.decode()?;
		Ok(Self { program, location })
	}
}

impl ToXdr for ClientAddr {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_str(&self.netid)?;
		encoder.encode_str(&self.addr)?;
		Ok(())
	}
}

impl FromXdr for ClientAddr {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let netid = decoder.decode_str()?.to_owned();
		let addr = decoder.decode_str()?.to_owned();
		Ok(Self { netid, addr })
	}
}

impl ToXdr for FsId {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.major)?;
		encoder.encode(&self.minor)?;
		Ok(())
	}
}

impl FromXdr for Bitmap {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		Ok(Self(decoder.decode()?))
	}
}

impl ToXdr for Bitmap {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.0)
	}
}

impl ToXdr for Ace {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.type_)?;
		encoder.encode(&self.flag)?;
		encoder.encode(&self.mask)?;
		encoder.encode(&self.who)?;
		Ok(())
	}
}

impl ToXdr for FsLocations {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.fs_root)?;
		encoder.encode(&self.locations)?;
		Ok(())
	}
}

impl ToXdr for Location {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.server)?;
		encoder.encode(&self.rootpath)?;
		Ok(())
	}
}

impl ToXdr for SpecData {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.specdata1)?;
		encoder.encode(&self.specdata2)?;
		Ok(())
	}
}

impl ToXdr for Time {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.seconds)?;
		encoder.encode(&self.nanos)?;
		Ok(())
	}
}

impl FromXdr for Time {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let seconds = decoder.decode()?;
		let nanos = decoder.decode()?;
		Ok(Self { seconds, nanos })
	}
}

impl FromXdr for OpenOwner {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let clientid = decoder.decode()?;
		let len = decoder.decode_uint()?;
		let bytes = decoder.decode_bytes(len.to_usize().unwrap())?;
		let opaque = bytes.to_owned();
		Ok(Self { clientid, opaque })
	}
}

impl FromXdr for OpenFlags {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		match decoder.decode_int()? {
			0 => Ok(Self::None),
			1 => Ok(Self::Create),
			_ => Err(super::xdr::Error::Custom(
				"Expected a flag openflags4 variant.".into(),
			)),
		}
	}
}

impl FromXdr for OpenClaim {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let tag = decoder.decode_int()?;
		match tag {
			0 => Ok(Self::Null(decoder.decode()?)),
			1 => Ok(Self::Previous(decoder.decode()?)),
			2 => Ok(Self::DelegateCur {
				delegate_stateid: decoder.decode()?,
				file: decoder.decode()?,
			}),
			3 => Ok(Self::DelegatePrev(decoder.decode()?)),
			_ => Err(super::xdr::Error::Custom(
				"Expected a claim delegation type.".into(),
			)),
		}
	}
}

impl FromXdr for OpenDelegationType {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let tag = decoder.decode_int()?;
		match tag {
			0 => Ok(Self::None),
			1 => Ok(Self::Read),
			2 => Ok(Self::Write),
			_ => Err(super::xdr::Error::Custom(
				"Expected an open delegation type.".into(),
			)),
		}
	}
}

impl ToXdr for Entry {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.cookie)?;
		encoder.encode(&self.name)?;
		encoder.encode(&self.attrs)?;
		Ok(())
	}
}

// Not explicitly defined in the RFC, but necessary.
pub const NFS4_VERIFIER_SIZE: usize = 8;
pub const NFS4_OTHER_SIZE: usize = 12;
pub const NFS4_FH_SIZE: usize = 128;

// RPC constants.
pub const RPC_VERS: u32 = 2;
pub const NFS_PROG: u32 = 100003;
pub const NFS_VERS: u32 = 4;

// Procedure codes.
pub const NFS_PROC_NULL: u32 = 0;
pub const NFS_PROC_COMPOUND: u32 = 1;

// Error codes.
pub const NFS4_OK: i32 = 0;
pub const NFS4ERR_PERM: i32 = 1;
pub const NFS4ERR_NOENT: i32 = 2;
pub const NFS4ERR_IO: i32 = 5;
pub const NFS4ERR_NXIO: i32 = 6;
pub const NFS4ERR_ACCESS: i32 = 13;
pub const NFS4ERR_EXIST: i32 = 17;
pub const NFS4ERR_XDEV: i32 = 18;
pub const NFS4ERR_NOTDIR: i32 = 20;
pub const NFS4ERR_ISDIR: i32 = 21;
pub const NFS4ERR_INVAL: i32 = 22;
pub const NFS4ERR_FBIG: i32 = 27;
pub const NFS4ERR_NOSPC: i32 = 28;
pub const NFS4ERR_ROFS: i32 = 30;
pub const NFS4ERR_MLINK: i32 = 31;
pub const NFS4ERR_NAMETOOLONG: i32 = 63;
pub const NFS4ERR_NOTEMPTY: i32 = 66;
pub const NFS4ERR_DQUOT: i32 = 69;
pub const NFS4ERR_STALE: i32 = 70;
pub const NFS4ERR_BADHANDLE: i32 = 10001;
pub const NFS4ERR_BAD_COOKIE: i32 = 10003;
pub const NFS4ERR_NOTSUPP: i32 = 10004;
pub const NFS4ERR_TOOSMALL: i32 = 10005;
pub const NFS4ERR_SERVERFAULT: i32 = 10006;
pub const NFS4ERR_BADTYPE: i32 = 10007;
pub const NFS4ERR_DELAY: i32 = 10008;
pub const NFS4ERR_SAME: i32 = 10009;
pub const NFS4ERR_DENIED: i32 = 10010;
pub const NFS4ERR_EXPIRED: i32 = 10011;
pub const NFS4ERR_LOCKED: i32 = 10012;
pub const NFS4ERR_GRACE: i32 = 10013;
pub const NFS4ERR_FHEXPIRED: i32 = 10014;
pub const NFS4ERR_SHARE_DENIED: i32 = 10015;
pub const NFS4ERR_WRONGSEC: i32 = 10016;
pub const NFS4ERR_CLID_INUSE: i32 = 10017;
pub const NFS4ERR_RESOURCE: i32 = 10018;
pub const NFS4ERR_MOVED: i32 = 10019;
pub const NFS4ERR_NOFILEHANDLE: i32 = 10020;
pub const NFS4ERR_MINOR_VERS_MISMATCH: i32 = 10021;
pub const NFS4ERR_STALE_CLIENTID: i32 = 10022;
pub const NFS4ERR_STALE_STATEID: i32 = 10023;
pub const NFS4ERR_OLD_STATEID: i32 = 10024;
pub const NFS4ERR_BAD_STATEID: i32 = 10025;
pub const NFS4ERR_BAD_SEQID: i32 = 10026;
pub const NFS4ERR_NOT_SAME: i32 = 10027;
pub const NFS4ERR_LOCK_RANGE: i32 = 10028;
pub const NFS4ERR_SYMLINK: i32 = 10029;
pub const NFS4ERR_RESTOREFH: i32 = 10030;
pub const NFS4ERR_LEASE_MOVED: i32 = 10031;
pub const NFS4ERR_ATTRNOTSUPP: i32 = 10032;
pub const NFS4ERR_NO_GRACE: i32 = 10033;
pub const NFS4ERR_RECLAIM_BAD: i32 = 10034;
pub const NFS4ERR_RECLAIM_CONFLICT: i32 = 10035;
pub const NFS4ERR_BADZDR: i32 = 10036;
pub const NFS4ERR_LOCKS_HELD: i32 = 10037;
pub const NFS4ERR_OPENMODE: i32 = 10038;
pub const NFS4ERR_BADOWNER: i32 = 10039;
pub const NFS4ERR_BADCHAR: i32 = 10040;
pub const NFS4ERR_BADNAME: i32 = 10041;
pub const NFS4ERR_BAD_RANGE: i32 = 10042;
pub const NFS4ERR_LOCK_NOTSUPP: i32 = 10043;
pub const NFS4ERR_OP_ILLEGAL: i32 = 10044;
pub const NFS4ERR_DEADLOCK: i32 = 10045;
pub const NFS4ERR_FILE_OPEN: i32 = 10046;
pub const NFS4ERR_ADMIN_REVOKED: i32 = 10047;
pub const NFS4ERR_CB_PATH_DOWN: i32 = 10048;
pub const NFS4ERR_BADIOMODE: i32 = 10049;
pub const NFS4ERR_BADLAYOUT: i32 = 10050;
pub const NFS4ERR_BAD_SESSION_DIGEST: i32 = 10051;
pub const NFS4ERR_BADSESSION: i32 = 10052;
pub const NFS4ERR_BADSLOT: i32 = 10053;
pub const NFS4ERR_COMPLETE_ALREADY: i32 = 10054;
pub const NFS4ERR_CONN_NOT_BOUND_TO_SESSION: i32 = 10055;
pub const NFS4ERR_DELEG_ALREADY_WANTED: i32 = 10056;
pub const NFS4ERR_BACK_CHAN_BUSY: i32 = 10057;
pub const NFS4ERR_LAYOUTTRYLATER: i32 = 10058;
pub const NFS4ERR_LAYOUTUNAVAILABLE: i32 = 10059;
pub const NFS4ERR_NOMATCHING_LAYOUT: i32 = 10060;
pub const NFS4ERR_RECALLCONFLICT: i32 = 10061;
pub const NFS4ERR_NOT_ONLY_OP: i32 = 10081;

// File types
pub const NF4REG: i32 = 1;
pub const NF4DIR: i32 = 2;
pub const NF4BLK: i32 = 3;
pub const NF4CHR: i32 = 4;
pub const NF4LNK: i32 = 5;
pub const NF4SOCK: i32 = 6;
pub const NF4FIFO: i32 = 7;
pub const NF4ATTRDIR: i32 = 8;
pub const NF4NAMEDATTR: i32 = 9;
