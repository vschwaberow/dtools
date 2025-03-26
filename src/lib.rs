// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/lib.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! # D64 Disk Image Library
//! 
//! This crate provides functionality for working with Commodore 64 D64 disk images.
//! It allows for reading, writing, and manipulating disk images including operations
//! such as formatting disks, managing files, and accessing sectors directly.
//! 
//! ## Features
//! 
//! * Read and write D64 disk images
//! * Format disk images with custom names and IDs
//! * List, extract, insert, and delete files
//! * Access and manipulate the Block Availability Map (BAM)
//! * Direct sector-level access
//! 
//! ## Example
//! 
//! ```rust
//! use d64lib::{D64, D64Error};
//! 
//! fn example() -> Result<(), D64Error> {
//!     // Create a new 35-track D64 disk image
//!     let mut disk = D64::new(35)?;
//!     
//!     // Format the disk with a name and ID
//!     disk.format("MY DISK", "01")?;
//!     
//!     // Save the disk image to a file
//!     disk.save_to_file("my_disk.d64")?;
//!     
//!     Ok(())
//! }
//! ```

mod constants;
mod error;
mod petscii;
mod bam;
mod file_info;
mod d64;

#[cfg(test)]
mod tests;

// Re-export all public items
pub use constants::*;
pub use error::D64Error;
pub use bam::BAM;
pub use file_info::FileInfo;
pub use d64::D64;
pub use petscii::{petscii_to_ascii, ascii_to_petscii};
