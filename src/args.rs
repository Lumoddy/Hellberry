use std::{fmt, str};

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum Command {
    #[default]
    Help,
    Run {
        /// `"-listen"` or `"-listen-port"`
        listen_port: Option<u16>,
        /// `"-broadcast"` or `"-broadcast-port"`
        broadcast_port: Option<u16>,
        /// `"-name"` or `"-berry-name"`
        berry_name: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CommandParseError<A> {
    UnknownCommand { arg: A },
    RunUnknownArg { arg: A },
    RunArgMissingValue { arg: A },
    RunArgInvalidValue { arg: A, value: A },
    HelpHasArg { arg: A },
}

impl<A: AsRef<[u8]>> fmt::Display for CommandParseError<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandParseError::UnknownCommand { arg } =>
                write!(f, "unknown command \"{}\"", String::from_utf8_lossy(arg.as_ref())),
            CommandParseError::RunUnknownArg { arg } =>
                write!(f, "unknown argument \"{}\" for \"run\" command", String::from_utf8_lossy(arg.as_ref())),
            CommandParseError::RunArgMissingValue { arg } =>
                write!(f, "missing value after \"{}\" for \"run\" command", String::from_utf8_lossy(arg.as_ref())),
            CommandParseError::RunArgInvalidValue { arg, value } =>
                write!(f, "invalid value \"{}\" after \"{}\" for \"run\" command", String::from_utf8_lossy(arg.as_ref()), String::from_utf8_lossy(value.as_ref())),
            CommandParseError::HelpHasArg { arg } =>
                write!(f, "unexpected argument \"{}\" for \"help\" command", String::from_utf8_lossy(arg.as_ref())),
        }
    }
}

pub fn parse_args<I>(source: I) -> Result<Option<Command>, CommandParseError<I::Item>>
where
    I: IntoIterator<Item: AsRef<[u8]>> {
    let mut iter = source.into_iter();

    match iter.next() {
        Some(arg) if matches!(arg.as_ref(), b"run") => {
            let mut listen_port = None;
            let mut broadcast_port = None;
            let mut berry_name = None;

            while let Some(arg) = iter.next() {
                match arg.as_ref() {
                    b"-listen" | b"-listen-port" => {
                        if let Some(value) = iter.next() {
                            if let Ok(Ok(x)) = str::from_utf8(value.as_ref()).map(str::parse) {
                                listen_port = Some(x);
                            } else {
                                return Err(CommandParseError::RunArgInvalidValue { arg, value });
                            }
                        } else {
                            return Err(CommandParseError::RunArgMissingValue { arg });
                        }
                    },
                    b"-broadcast" | b"-broadcast-port" => {
                        if let Some(value) = iter.next() {
                            if let Ok(Ok(x)) = str::from_utf8(value.as_ref()).map(str::parse) {
                                broadcast_port = Some(x);
                            } else {
                                return Err(CommandParseError::RunArgInvalidValue { arg, value });
                            }
                        } else {
                            return Err(CommandParseError::RunArgMissingValue { arg });
                        }
                    },
                    b"-name" | b"-berry-name" => {
                        if let Some(value) = iter.next() {
                            if let Ok(x) = str::from_utf8(value.as_ref()) {
                                berry_name = Some(x.to_string());
                            } else {
                                return Err(CommandParseError::RunArgInvalidValue { arg, value });
                            }
                        } else {
                            return Err(CommandParseError::RunArgMissingValue { arg });
                        }
                    },
                    _ => {
                        return Err(CommandParseError::RunUnknownArg { arg });
                    },
                }
            }

            Ok(Some(Command::Run { listen_port, broadcast_port, berry_name }))
        },
        Some(x) if matches!(x.as_ref(), b"help") => {
            if let Some(arg) = iter.next() {
                return Err(CommandParseError::HelpHasArg { arg });
            }

            Ok(Some(Command::Help))
        },
        Some(x) => {
            Err(CommandParseError::UnknownCommand { arg: x })
        },
        None => {
            Ok(None)
        },
    }
}