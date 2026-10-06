use std::io::Write;

use tokio::io;

use crate::packet::{ESCAPE, START};

use super::Reset;

pub trait Serialize
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>;

    fn write_size(&self) -> (usize, Option<usize>) { (0, None) }
}

impl<T: Serialize> Serialize for &T
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        (*self).write_to(writer)
    }
}

impl<T: Serialize> Serialize for &mut T
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        (**self).write_to(writer)
    }
}

impl Serialize for Reset
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        writer.write_all(&[ESCAPE, START])
    }
}

impl Serialize for u8
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        match self.to_be_bytes()
        {
            [ESCAPE] => writer.write_all(&[ESCAPE, ESCAPE]),
            [byte] => writer.write_all(&[byte]),
        }
    }
}

impl Serialize for u16
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        match self.to_be_bytes()
        {
            [ESCAPE, ESCAPE] => writer.write_all(&[ESCAPE, ESCAPE, ESCAPE, ESCAPE]),
            [byte0, ESCAPE] => writer.write_all(&[byte0, ESCAPE, ESCAPE]),
            [ESCAPE, byte1] => writer.write_all(&[ESCAPE, ESCAPE, byte1]),
            [byte0, byte1] => writer.write_all(&[byte0, byte1]),
        }
    }
}

impl<T: Serialize> Serialize for [T]
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        u16::try_from(self.len()).expect("slice length exceeds u16::MAX").write_to(writer)?;
        for item in self { item.write_to(writer)? }
        Ok(())
    }
}

impl Serialize for str
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        self.as_bytes().write_to(writer)
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
                fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
                {
                    $(self. $field .write_to(writer)?;)*
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