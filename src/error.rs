// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/error.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! Error types for D64 operations

use thiserror::Error;

/// Errors that can occur when working with D64 disk images
#[derive(Error, Debug)]
pub enum D64Error {
    /// I/O error occurred during file operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Invalid D64 file size (not matching 35 or 40 tracks)
    #[error("Invalid D64 file size: expected {expected} bytes, got {actual} bytes")]
    InvalidFileSize {
        expected: String, 
        actual: usize
    },
    
    /// Invalid track or sector reference
    #[error("Invalid track or sector: track {track}, sector {sector}")]
    InvalidTrackSector {
        track: u8,
        sector: u8
    },
    
    /// File not found on disk
    #[error("File not found: {filename}")]
    FileNotFound {
        filename: String
    },
    
    /// Disk is full (no free sectors available)
    #[error("Disk full: no free sectors available")]
    DiskFull,
    
    /// Circular reference detected in sector chain
    #[error("Circular reference detected in sector chain")]
    CircularReference,
    
    /// Corrupt directory structure
    #[error("Corrupt directory structure: {reason}")]
    CorruptDirectory {
        reason: String
    },
    
    /// Invalid data provided
    #[error("Invalid data: {reason}")]
    InvalidData {
        reason: String
    },
}