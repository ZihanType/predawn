use std::{borrow::Cow, fmt};

use tokio_tungstenite::tungstenite::{
    self,
    protocol::{self, frame::coding},
};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CloseCode {
    /// Indicates a normal closure, meaning that the purpose for
    /// which the connection was established has been fulfilled.
    Normal,
    /// Indicates that an endpoint is "going away", such as a server
    /// going down or a browser having navigated away from a page.
    Away,
    /// Indicates that an endpoint is terminating the connection due
    /// to a protocol error.
    Protocol,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received a type of data it cannot accept (e.g., an
    /// endpoint that understands only text data MAY send this if it
    /// receives a binary message).
    Unsupported,
    /// Indicates that no status code was included in a closing frame. This
    /// close code makes it possible to use a single method, `on_close` to
    /// handle even cases where no close code was provided.
    Status,
    /// Indicates an abnormal closure. If the abnormal closure was due to an
    /// error, this close code will not be used. Instead, the `on_error` method
    /// of the handler will be called with the error. However, if the connection
    /// is simply dropped, without an error, this close code will be sent to the
    /// handler.
    Abnormal,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received data within a message that was not
    /// consistent with the type of the message (e.g., non-UTF-8 \[RFC3629\]
    /// data within a text message).
    Invalid,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received a message that violates its policy.  This
    /// is a generic status code that can be returned when there is no
    /// other more suitable status code (e.g., Unsupported or Size) or if there
    /// is a need to hide specific details about the policy.
    Policy,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received a message that is too big for it to
    /// process.
    Size,
    /// Indicates that an endpoint (client) is terminating the
    /// connection because it has expected the server to negotiate one or
    /// more extension, but the server didn't return them in the response
    /// message of the WebSocket handshake.  The list of extensions that
    /// are needed should be given as the reason for closing.
    /// Note that this status code is not used by the server, because it
    /// can fail the WebSocket handshake instead.
    Extension,
    /// Indicates that a server is terminating the connection because
    /// it encountered an unexpected condition that prevented it from
    /// fulfilling the request.
    Error,
    /// Indicates that the server is restarting. A client may choose to reconnect,
    /// and if it does, it should use a randomized delay of 5-30 seconds between attempts.
    Restart,
    /// Indicates that the server is overloaded and the client should either connect
    /// to a different IP (when multiple targets exist), or reconnect to the same IP
    /// when a user has performed an action.
    Again,

    #[doc(hidden)]
    Other(u16),
}

/// A struct representing the close command.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloseFrame<'a> {
    /// The reason as a code.
    pub code: CloseCode,
    /// The reason as text string.
    pub reason: Cow<'a, str>,
}

impl From<CloseCode> for u16 {
    fn from(code: CloseCode) -> Self {
        match code {
            CloseCode::Normal => 1000,
            CloseCode::Away => 1001,
            CloseCode::Protocol => 1002,
            CloseCode::Unsupported => 1003,
            CloseCode::Status => 1005,
            CloseCode::Abnormal => 1006,
            CloseCode::Invalid => 1007,
            CloseCode::Policy => 1008,
            CloseCode::Size => 1009,
            CloseCode::Extension => 1010,
            CloseCode::Error => 1011,
            CloseCode::Restart => 1012,
            CloseCode::Again => 1013,
            CloseCode::Other(code) => code,
        }
    }
}

impl<'a> From<&'a CloseCode> for u16 {
    fn from(code: &'a CloseCode) -> Self {
        (*code).into()
    }
}

impl From<u16> for CloseCode {
    fn from(code: u16) -> Self {
        match code {
            1000 => CloseCode::Normal,
            1001 => CloseCode::Away,
            1002 => CloseCode::Protocol,
            1003 => CloseCode::Unsupported,
            1005 => CloseCode::Status,
            1006 => CloseCode::Abnormal,
            1007 => CloseCode::Invalid,
            1008 => CloseCode::Policy,
            1009 => CloseCode::Size,
            1010 => CloseCode::Extension,
            1011 => CloseCode::Error,
            1012 => CloseCode::Restart,
            1013 => CloseCode::Again,
            _ => CloseCode::Other(code),
        }
    }
}

impl fmt::Display for CloseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = u16::from(*self);
        <u16 as fmt::Display>::fmt(&code, f)
    }
}

impl<'a> CloseFrame<'a> {
    pub fn into_owned(self) -> CloseFrame<'static> {
        CloseFrame {
            code: self.code,
            reason: Cow::Owned(self.reason.into_owned()),
        }
    }
}

impl<'a> fmt::Display for CloseFrame<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.reason, self.code)
    }
}

/// An enum representing the various forms of a WebSocket message.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Message {
    /// A text WebSocket message
    Text(String),
    /// A binary WebSocket message
    Binary(Vec<u8>),
    /// A ping message with the specified payload
    ///
    /// The payload here must have a length less than 125 bytes
    Ping(Vec<u8>),
    /// A pong message with the specified payload
    ///
    /// The payload here must have a length less than 125 bytes
    Pong(Vec<u8>),
    /// A close message with the optional close frame.
    Close(Option<CloseFrame<'static>>),
}

impl Message {
    pub fn into_tungstenite(self) -> tungstenite::Message {
        match self {
            Message::Text(text) => tungstenite::Message::Text(text),
            Message::Binary(binary) => tungstenite::Message::Binary(binary),
            Message::Ping(ping) => tungstenite::Message::Ping(ping),
            Message::Pong(pong) => tungstenite::Message::Pong(pong),
            Message::Close(close) => {
                tungstenite::Message::Close(close.map(|close| protocol::CloseFrame {
                    code: coding::CloseCode::from(u16::from(close.code)),
                    reason: close.reason,
                }))
            }
        }
    }

    pub fn from_tungstenite(message: tungstenite::Message) -> Option<Self> {
        match message {
            tungstenite::Message::Text(text) => Some(Self::Text(text)),
            tungstenite::Message::Binary(binary) => Some(Self::Binary(binary)),
            tungstenite::Message::Ping(ping) => Some(Self::Ping(ping)),
            tungstenite::Message::Pong(pong) => Some(Self::Pong(pong)),
            tungstenite::Message::Close(close) => {
                Some(Self::Close(close.map(|close| CloseFrame {
                    code: CloseCode::from(u16::from(close.code)),
                    reason: close.reason,
                })))
            }
            tungstenite::Message::Frame(_) => None,
        }
    }
}
