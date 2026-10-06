use std::fmt;

macro_rules! impl_as_str
{
    ( $( $name:ident = $str:expr ),* $(,)? ) =>
    {
        $(
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
            pub struct $name;

            impl $name
            {
                pub const fn as_str(&self) -> &'static str { $str }
            }

            impl fmt::Display for $name
            {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
                {
                    f.write_str(self.as_str())
                }
            }
        )*
    };
}

impl_as_str!
{
    Success = "\x1B[m32->\x1B[0m",
    Warn = "\x1B[33m/!\\\x1B[0m",
    Error = "\x1B[31m{{!}}\x1B[0m",
    ANSIReset = "\x1B[0m",
    ANSIYellow = "\x1B[33m",
    ANSICyan = "\x1B[36m",
    ANSIPurple = "\x1B[35m",
}