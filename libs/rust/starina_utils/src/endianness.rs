#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct LittleEndian<T>(T);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BigEndian<T>(T);

macro_rules! impl_little_endian {
    ($($t:ty),+) => {
        $(
            impl LittleEndian<$t> {
                pub const fn from_host(native: $t) -> Self {
                    LittleEndian(native.to_le())
                }

                pub const fn to_host(&self) -> $t {
                    if cfg!(target_endian = "big") {
                        self.0.to_be()
                    } else {
                        self.0
                    }
                }
            }

            impl From<$t> for LittleEndian<$t> {
                fn from(value: $t) -> Self {
                    Self::from_host(value)
                }
            }
        )+
    };
}

macro_rules! impl_big_endian {
    ($($t:ty),+) => {
        $(
            impl BigEndian<$t> {
                pub const fn from_host(native: $t) -> Self {
                    BigEndian(native.to_be())
                }

                pub const fn to_host(&self) -> $t {
                    if cfg!(target_endian = "little") {
                        self.0.to_le()
                    } else {
                        self.0
                    }
                }
            }

            impl From<$t> for BigEndian<$t> {
                fn from(value: $t) -> Self {
                    Self::from_host(value)
                }
            }
        )+
    };
}

impl_little_endian!(u16, u32, u64, i16, i32, i64);
impl_big_endian!(u16, u32, u64, i16, i32, i64);
