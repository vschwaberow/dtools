// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/file_info.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! File information structures and related operations

/// File information from the directory
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File type (see file_types constants)
    pub file_type: u8,
    /// File name (in ASCII)
    pub name: String,
    /// Track of first sector
    pub track: u8,
    /// Sector of first sector
    pub sector: u8,
    /// File size in blocks (sectors)
    pub size_blocks: u16,
}

impl FileInfo {
    /// Creates a new FileInfo instance
    ///
    /// # Arguments
    ///
    /// * `file_type` - Type of the file (PRG, SEQ, etc.)
    /// * `name` - Name of the file
    /// * `track` - Track of the first sector
    /// * `sector` - Sector of the first sector
    /// * `size_blocks` - Size of the file in blocks
    ///
    /// # Returns
    ///
    /// A new FileInfo instance
    pub fn new(file_type: u8, name: &str, track: u8, sector: u8, size_blocks: u16) -> Self {
        Self {
            file_type,
            name: name.to_string(),
            track,
            sector,
            size_blocks,
        }
    }

    /// Checks if the file is a PRG file
    ///
    /// # Returns
    ///
    /// true if the file is a PRG file, false otherwise
    pub fn is_prg(&self) -> bool {
        (self.file_type & 0x07) == crate::file_types::PRG
    }

    /// Checks if the file is a SEQ file
    ///
    /// # Returns
    ///
    /// true if the file is a SEQ file, false otherwise
    pub fn is_seq(&self) -> bool {
        (self.file_type & 0x07) == crate::file_types::SEQ
    }

    /// Checks if the file is closed
    ///
    /// # Returns
    ///
    /// true if the file is closed, false otherwise
    pub fn is_closed(&self) -> bool {
        (self.file_type & crate::file_types::CLOSED) != 0
    }

    /// Checks if the file is locked
    ///
    /// # Returns
    ///
    /// true if the file is locked, false otherwise
    pub fn is_locked(&self) -> bool {
        (self.file_type & crate::file_types::LOCKED) != 0
    }
}