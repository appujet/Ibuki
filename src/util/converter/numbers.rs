use super::super::errors::ConverterError;
use std::num::NonZeroU64;

pub struct IbukiGuildId(pub u64);

impl TryFrom<IbukiGuildId> for NonZeroU64 {
    type Error = ConverterError;

    fn try_from(value: IbukiGuildId) -> Result<Self, Self::Error> {
        NonZeroU64::new(value.0).ok_or(ConverterError::NonZeroU64(value.0))
    }
}

pub struct IbukiUserId(pub u64);

impl TryFrom<IbukiUserId> for NonZeroU64 {
    type Error = ConverterError;

    fn try_from(value: IbukiUserId) -> Result<Self, Self::Error> {
        NonZeroU64::new(value.0).ok_or(ConverterError::NonZeroU64(value.0))
    }
}
