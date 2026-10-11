use tokio::io::{self, AsyncWrite, AsyncWriteExt};

use crate::packet::{ESCAPE, START};

use super::Reset;

pub trait Serialize
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>;
}

impl<T: Serialize> Serialize for &T
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        (*self).write_to(writer).await
    }
}

impl<T: Serialize> Serialize for &mut T
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        (**self).write_to(writer).await
    }
}

impl Serialize for Reset
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        writer.write_all(&[ESCAPE, START]).await
    }
}

impl Serialize for u8
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        match self.to_be_bytes()
        {
            [ESCAPE] => writer.write_all(&[ESCAPE, ESCAPE]).await,
            [byte] => writer.write_all(&[byte]).await,
        }
    }
}

impl Serialize for u16
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        match self.to_be_bytes()
        {
            [ESCAPE, ESCAPE] => writer.write_all(&[ESCAPE, ESCAPE, ESCAPE, ESCAPE]).await,
            [byte0, ESCAPE] => writer.write_all(&[byte0, ESCAPE, ESCAPE]).await,
            [ESCAPE, byte1] => writer.write_all(&[ESCAPE, ESCAPE, byte1]).await,
            [byte0, byte1] => writer.write_all(&[byte0, byte1]).await,
        }
    }
}

impl<T: Serialize> Serialize for [T]
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        u16::try_from(self.len()).expect("slice length exceeds u16::MAX").write_to(writer).await?;
        for item in self { item.write_to(writer).await? }
        Ok(())
    }
}

impl<T: Serialize> Serialize for Vec<T>
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        self.as_slice().write_to(writer).await
    }
}

impl Serialize for str
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        self.as_bytes().write_to(writer).await
    }
}

impl Serialize for String
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        self.as_str().write_to(writer).await
    }
}

macro_rules! impl_tuple
{
    ( $( ( $( $field:tt : $ty:ident ),* $(,)? ) $(,)? )* ) =>
    {
        $(
            impl< $( $ty : Serialize,)* > Serialize for ( $( $ty ,)* )
            {
                #[allow(unused_variables)]
                async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
                {
                    $(self. $field .write_to(writer).await?;)*
                    Ok(())
                }
            }
        )*
    };
}

impl_tuple!
{
    (),
    (0: T0),
    (0: T0, 1: T1),
    (0: T0, 1: T1, 2: T2),
    (0: T0, 1: T1, 2: T2, 3: T3),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6),
    (0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7),
}