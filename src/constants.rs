// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/constants.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! Constants and definitions for the D64 format

/// Size of a standard 35-track D64 disk image in bytes
pub const D64_35_TRACKS_SIZE: usize = 174848;
/// Size of an extended 40-track D64 disk image in bytes
pub const D64_40_TRACKS_SIZE: usize = 196608;
/// Maximum number of tracks in a D64 disk image
pub const MAX_TRACKS: u8 = 40;
/// Number of sectors per track for each of the 40 tracks
pub const SECTORS_PER_TRACK: [u8; MAX_TRACKS as usize] = [
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 19, 19, 19,
    19, 19, 19, 19, 18, 18, 18, 18, 18, 18, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
];
/// Directory track on a D64 disk
pub const DIRECTORY_TRACK: u8 = 18;
/// First sector of the directory
pub const DIRECTORY_FIRST_SECTOR: u8 = 1;
/// BAM sector on track 18
pub const BAM_SECTOR: u8 = 0;
/// Size of a sector in bytes
pub const SECTOR_SIZE: usize = 256;
/// Maximum data bytes in a standard sector (excluding the track/sector link)
pub const MAX_SECTOR_DATA: usize = 254;
/// PETSCII space character
pub const PETSCII_SPACE: u8 = 0xA0;

/// File type flags
pub mod file_types {
    /// DEL file type (0x00)
    pub const DEL: u8 = 0x00;
    /// SEQ file type (0x01)
    pub const SEQ: u8 = 0x01;
    /// PRG file type (0x02)
    pub const PRG: u8 = 0x02;
    /// USR file type (0x03)
    pub const USR: u8 = 0x03;
    /// REL file type (0x04)
    pub const REL: u8 = 0x04;
    /// File closed flag (0x80)
    pub const CLOSED: u8 = 0x80;
    /// File locked flag (0x40)
    pub const LOCKED: u8 = 0x40;
}