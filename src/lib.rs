#![doc = include_str!("../README.md")]

pub mod error;

#[cfg(feature = "sync")]
pub mod synchronous;

#[cfg(feature = "async")]
pub mod asynchronous;
