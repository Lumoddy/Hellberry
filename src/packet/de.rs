use std::convert::Infallible;
use std::string;

use tokio::io::{self, AsyncRead, AsyncReadExt};

use super::{ESCAPE, Reset, START};

#[derive(Debug)]
pub enum Error<O = Infallible>
{
    Reset(Reset),
    InvalidEscape { byte: u8 },
    IO(io::Error),
    Other(O),
}

impl Error<Infallible>
{
    pub fn upcast<O>(self) -> Error<O>
    {
        match self
        {
            Self::Reset(reset) => Error::Reset(reset),
            Self::InvalidEscape { byte } => Error::InvalidEscape { byte },
            Self::IO(x) => Error::IO(x),
        }
    }
}

impl<O> Error<O>
{
    pub fn coerce<U>(self) -> Error<U>
    where
        O: Into<U>,
    {
        match self
        {
            Self::Reset(reset) => Error::Reset(reset),
            Self::InvalidEscape { byte } => Error::InvalidEscape { byte },
            Self::IO(x) => Error::IO(x),
            Self::Other(x) => Error::Other(x.into()),
        }
    }
}

impl<O> From<O> for Error<O>
{
    fn from(value: O) -> Self { Self::Other(value) }
}

pub trait Deserialize: Sized
{
    type Error;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, Error<Self::Error>>;
}

impl Deserialize for u8
{
    type Error = Infallible;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, Error<Self::Error>>
    {
        match reader.read_u8().await.map_err(Error::IO)?
        {
            ESCAPE => match reader.read_u8().await.map_err(Error::IO)?
            {
                ESCAPE => Ok(ESCAPE),
                START => Err(Error::Reset(Reset)),
                byte => Err(Error::InvalidEscape { byte }),
            },
            byte => Ok(byte),
        }
    }
}

impl Deserialize for u16
{
    type Error = Infallible;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, Error<Self::Error>>
    {
        Ok(u16::from_be_bytes(
        [
            u8::read_from(reader).await?,
            u8::read_from(reader).await?,
        ]))
    }
}

impl<T: Deserialize> Deserialize for Vec<T>
{
    type Error = T::Error;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, Error<Self::Error>>
    {
        let len = u16::read_from(reader).await.map_err(Error::upcast)?;
        let mut result = Vec::with_capacity(len as usize);
        for _ in 0..len { result.push(T::read_from(reader).await?) }
        Ok(result)
    }
}

impl Deserialize for String
{
    type Error = string::FromUtf8Error;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, Error<Self::Error>>
    {
        Ok(String::from_utf8(Vec::read_from(reader).await.map_err(Error::upcast)?)?)
    }
}

macro_rules! impl_tuple
{
    ( $( ( $first_field:tt : $first_ty:ident $( , $field:tt : $ty:ident )* $(,)? ) $(,)? )* ) =>
    {
        $(
            impl< $first_ty : Deserialize, $( $ty : Deserialize,)* > Deserialize for ( $first_ty, $( $ty ,)* )
            where
                $( $ty :: Error: Into< $first_ty ::Error>,)*
            {
                type Error = $first_ty ::Error;

                async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, Error<Self::Error>>
                {
                    Ok((
                        $first_ty ::read_from(reader).await?,
                        $( $ty ::read_from(reader).await.map_err(Error::coerce)?,)*
                    ))
                }
            }
        )*
    };
}

impl Deserialize for ()
{
    type Error = Infallible;

    async fn read_from(_: &mut (impl AsyncRead + Unpin)) -> Result<Self, Error<Self::Error>> { Ok(()) }
}

impl_tuple!
{
    (0: T0),
    (0: T0, 1: T1),
    (0: T0, 1: T1, 2: T2),
    (0: T0, 1: T1, 2: T2, 3: T3),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7),
}