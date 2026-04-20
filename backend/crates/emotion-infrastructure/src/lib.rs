//! Layer: infrastructure (Emotion bounded context).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![allow(clippy::missing_errors_doc)]

pub mod in_memory;
pub mod postgres_reading_repository;

pub use in_memory::InMemoryReadingRepository;
pub use postgres_reading_repository::PostgresReadingRepository;
