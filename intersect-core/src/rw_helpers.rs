use binrw::{binrw, BinRead, BinResult, BinWrite};
use std::io::Cursor;

// extension trait to make it easier to write to a newly allocated buffer with binrw
pub trait BinWriteAlloc: BinWrite {
    fn serialise(&self) -> Vec<u8>
    where
        Self: binrw::meta::WriteEndian,
        for<'a> <Self as BinWrite>::Args<'a>: Default,
    {
        let mut writer = Cursor::new(Vec::new());
        self.write(&mut writer).unwrap();
        writer.into_inner()
    }
}

impl<T: BinWrite> BinWriteAlloc for T {}

pub trait BinReadAlloc: BinRead {
    fn deserialise(bytes: &[u8]) -> BinResult<Self>
    where
        Self: binrw::meta::ReadEndian,
        for<'a> <Self as BinRead>::Args<'a>: Default,
    {
        Self::read(&mut Cursor::new(bytes))
    }
}

impl<T: BinRead> BinReadAlloc for T {}

// Alternative to Option<T> that serialises to a fixed-size output
// instead of optionally outputting data

#[binrw]
#[br(import_raw(r: <T as BinRead>::Args<'_>))]
#[bw(import_raw(r: <T as BinWrite>::Args<'_>))]
pub enum RWOption<T>
where
    T: BinWrite + BinRead,
{
    #[brw(magic = 0xffu8)]
    Some(
        #[br(args_raw = r)]
        #[bw(args_raw = r)]
        T,
    ),
    #[brw(magic = 0x00u8)]
    None,
}

impl<T> RWOption<T>
where
    T: binrw::BinWrite + binrw::BinRead,
    for<'a> <T as binrw::BinRead>::Args<'a>: Default,
    for<'a> <T as binrw::BinWrite>::Args<'a>: Default,
{
    pub fn from_clone(value: &Option<T>) -> Self
    where
        T: Clone,
    {
        match value {
            Some(v) => RWOption::Some(v.clone()),
            None => RWOption::None,
        }
    }

    pub fn from_into<R>(value: &Option<R>) -> Self
    where
        R: Into<T> + Clone,
    {
        match value {
            Some(v) => RWOption::Some(Into::<T>::into(v.clone())),
            None => RWOption::None,
        }
    }

    pub fn into_from<R>(value: Self) -> Option<R>
    where
        R: From<T>,
    {
        match value {
            RWOption::Some(v) => Some(v.into()),
            RWOption::None => None,
        }
    }
}

impl<T> From<Option<T>> for RWOption<T>
where
    T: binrw::BinWrite + binrw::BinRead,
    for<'a> <T as binrw::BinRead>::Args<'a>: Default,
    for<'a> <T as binrw::BinWrite>::Args<'a>: Default,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => RWOption::Some(v),
            None => RWOption::None,
        }
    }
}

impl<T> From<&Option<T>> for RWOption<T>
where
    T: binrw::BinWrite + binrw::BinRead + Clone,
    for<'a> <T as binrw::BinRead>::Args<'a>: Default,
    for<'a> <T as binrw::BinWrite>::Args<'a>: Default,
{
    fn from(value: &Option<T>) -> Self {
        match value {
            Some(v) => RWOption::Some(v.clone()),
            None => RWOption::None,
        }
    }
}

impl<T> From<RWOption<T>> for Option<T>
where
    T: binrw::BinWrite + binrw::BinRead,
    for<'a> <T as binrw::BinRead>::Args<'a>: Default,
    for<'a> <T as binrw::BinWrite>::Args<'a>: Default,
{
    fn from(value: RWOption<T>) -> Self {
        match value {
            RWOption::Some(v) => Some(v),
            RWOption::None => None,
        }
    }
}
