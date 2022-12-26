pub mod tcp_reader;
pub mod tcp_writer;

use std::future::Future;

use async_channel::{Receiver, Sender};
use bytes::BytesMut;

use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::packets::Packet;