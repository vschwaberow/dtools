// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/d64.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! Core D64 disk image functionality

use log::info;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::constants::*;
use crate::error::D64Error;
use crate::file_info::FileInfo;
use crate::bam::BAM;
use crate::petscii::{petscii_to_ascii, ascii_to_petscii};

/// Represents a D64 disk image with methods for manipulation
#[derive(Clone, Debug)]
pub struct D64 {
    /// Raw data of the disk image
    pub data: Vec<u8>,
    /// Number of tracks in the disk image (35 or 40)
    pub tracks: u8,
    /// Offset of each track in the data array
    track_offsets: Vec<usize>,
}

impl D64 {
    /// Computes the offset of each track in the disk image
    ///
    /// # Arguments
    ///
    /// * `tracks` - Number of tracks in the disk
    ///
    /// # Returns
    ///
    /// A vector containing the offset of each track in bytes
    fn compute_track_offsets(tracks: u8) -> Vec<usize> {
        let mut offsets = Vec::with_capacity(tracks as usize);
        let mut offset = 0;
        for t in 1..=tracks {
            offsets.push(offset);
            offset += SECTORS_PER_TRACK[(t - 1) as usize] as usize * SECTOR_SIZE;
        }
        offsets
    }

    /// Creates a new empty D64 disk image
    ///
    /// # Arguments
    ///
    /// * `tracks` - Number of tracks (must be 35 or 40)
    ///
    /// # Returns
    ///
    /// A Result containing either a new D64 instance or an error
    pub fn new(tracks: u8) -> Result<Self, D64Error> {
        match tracks {
            35 | 40 => {
                let size = if tracks == 35 {
                    D64_35_TRACKS_SIZE
                } else {
                    D64_40_TRACKS_SIZE
                };
                
                Ok(Self {
                    data: vec![0; size],
                    tracks,
                    track_offsets: Self::compute_track_offsets(tracks),
                })
            },
            _ => Err(D64Error::InvalidFileSize { 
                expected: "174848 (35 tracks) or 196608 (40 tracks)".to_string(), 
                actual: 0 
            })
        }
    }

    /// Loads a D64 disk image from a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the D64 file
    ///
    /// # Returns
    ///
    /// A Result containing either a D64 instance or an error
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, D64Error> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        let tracks = match data.len() {
            D64_35_TRACKS_SIZE => 35,
            D64_40_TRACKS_SIZE => 40,
            actual => return Err(D64Error::InvalidFileSize { 
                expected: format!("{} or {}", D64_35_TRACKS_SIZE, D64_40_TRACKS_SIZE), 
                actual 
            }),
        };
        
        Ok(Self {
            data,
            tracks,
            track_offsets: Self::compute_track_offsets(tracks),
        })
    }

    /// Saves the D64 disk image to a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save the D64 file
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), D64Error> {
        let mut file = File::create(path)?;
        file.write_all(&self.data)?;
        Ok(())
    }

    /// Calculates the offset of a sector in the disk image
    ///
    /// # Arguments
    ///
    /// * `track` - Track number (1-35/40)
    /// * `sector` - Sector number (0-20, depending on track)
    ///
    /// # Returns
    ///
    /// A Result containing either the offset in bytes or an error
    fn sector_offset(&self, track: u8, sector: u8) -> Result<usize, D64Error> {
        if track == 0 || track > self.tracks || sector >= SECTORS_PER_TRACK[(track - 1) as usize] {
            return Err(D64Error::InvalidTrackSector { track, sector });
        }
        Ok(self.track_offsets[(track - 1) as usize] + sector as usize * SECTOR_SIZE)
    }

    /// Gets a mutable reference to a sector
    ///
    /// # Arguments
    ///
    /// * `track` - Track number (1-35/40)
    /// * `sector` - Sector number (0-20, depending on track)
    ///
    /// # Returns
    ///
    /// A Result containing either a mutable slice of the sector data or an error
    pub fn get_sector_mut(&mut self, track: u8, sector: u8) -> Result<&mut [u8], D64Error> {
        let offset = self.sector_offset(track, sector)?;
        Ok(&mut self.data[offset..offset + SECTOR_SIZE])
    }

    /// Reads a sector from the disk image
    ///
    /// # Arguments
    ///
    /// * `track` - Track number (1-35/40)
    /// * `sector` - Sector number (0-20, depending on track)
    ///
    /// # Returns
    ///
    /// A Result containing either a slice of the sector data or an error
    pub fn read_sector(&self, track: u8, sector: u8) -> Result<&[u8], D64Error> {
        let offset = self.sector_offset(track, sector)?;
        Ok(&self.data[offset..offset + SECTOR_SIZE])
    }

    /// Writes data to a sector
    ///
    /// # Arguments
    ///
    /// * `track` - Track number (1-35/40)
    /// * `sector` - Sector number (0-20, depending on track)
    /// * `data` - Data to write (must be 256 bytes)
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn write_sector(&mut self, track: u8, sector: u8, data: &[u8]) -> Result<(), D64Error> {
        if data.len() != SECTOR_SIZE {
            return Err(D64Error::InvalidData { 
                reason: format!("Sector data must be exactly {} bytes", SECTOR_SIZE) 
            });
        }
        
        let offset = self.sector_offset(track, sector)?;
        self.data[offset..offset + SECTOR_SIZE].copy_from_slice(data);
        Ok(())
    }

    /// Formats the disk with specified disk name and ID
    ///
    /// This simulates the format command on a Commodore 64, creating an empty BAM
    /// and directory structure.
    ///
    /// # Arguments
    ///
    /// * `disk_name` - Name for the disk (up to 16 characters)
    /// * `disk_id` - ID for the disk (usually 2 characters)
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn format(&mut self, disk_name: &str, disk_id: &str) -> Result<(), D64Error> {
        // Clear the entire disk
        self.data.fill(0);
        
        // Create BAM sector
        let mut bam = [0u8; SECTOR_SIZE];
        
        // Set BAM header: first directory sector location and DOS type
        bam[0] = DIRECTORY_TRACK;
        bam[1] = DIRECTORY_FIRST_SECTOR;
        bam[2] = 0x41; // DOS version ('A')
        
        // Initialize free sector maps
        for track in 1..=self.tracks {
            let idx = (track - 1) as usize;
            let sectors = SECTORS_PER_TRACK[idx];
            bam[4 + idx * 4] = sectors; // Free sector count
            
            // Set bitmap bytes for free sectors
            // Each bit represents whether a sector is free (1) or allocated (0)
            bam[5 + idx * 4] = 0xFF;
            bam[6 + idx * 4] = 0xFF;
            bam[7 + idx * 4] = if sectors > 16 { 0xFF } else { (1 << sectors) - 1 };
        }
        
        // Mark directory track (18) and track 19 as allocated for system use
        for track in [DIRECTORY_TRACK, 19] {
            let idx = (track - 1) as usize;
            bam[4 + idx * 4] = 0; // No free sectors
            bam[5 + idx * 4] = 0; // All sectors allocated
            bam[6 + idx * 4] = 0;
            bam[7 + idx * 4] = 0;
        }
        
        // Set disk name (padded with 0xA0)
        bam[144..160].fill(PETSCII_SPACE);
        
        let disk_name_bytes = ascii_to_petscii(disk_name);
        let name_len = disk_name_bytes.len().min(16);
        bam[144..144 + name_len].copy_from_slice(&disk_name_bytes[..name_len]);
        
        // Set disk ID
        let disk_id_bytes = ascii_to_petscii(disk_id);
        let id_len = disk_id_bytes.len().min(2);
        bam[162..162 + id_len].copy_from_slice(&disk_id_bytes[..id_len]);
        
        // Set DOS version at offset 165
        bam[165] = 0x32;  // ASCII '2' - representing DOS 2.6
        bam[166] = 0x41;  // ASCII 'A'
        
        // Write BAM to disk
        self.write_sector(DIRECTORY_TRACK, BAM_SECTOR, &bam)?;
        
        // Initialize the first directory sector
        let mut dir = [0u8; SECTOR_SIZE];
        dir[1] = 0xFF; // No more directory sectors
        self.write_sector(DIRECTORY_TRACK, DIRECTORY_FIRST_SECTOR, &dir)?;
        
        Ok(())
    }

    /// Traces a file through the disk, returning a list of all sectors it occupies
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to trace
    ///
    /// # Returns
    ///
    /// A Result containing either a vector of (track, sector) pairs or an error
    pub fn trace_file(&self, filename: &str) -> Result<Vec<(u8, u8)>, D64Error> {
        let (mut track, mut sector) = self.find_file(filename)?;
        let mut sectors = Vec::new();
        let mut visited = HashSet::new();
        
        info!("Tracing file: {}", filename);

        loop {
            // Check for circular references
            if !visited.insert((track, sector)) {
                info!("Circular reference detected while tracing file: {}", filename);
                return Err(D64Error::CircularReference);
            }
            
            sectors.push((track, sector));
            info!("File block: track {}, sector {}", track, sector);

            let data = self.read_sector(track, sector)?;
            let next_track = data[0];
            let next_sector = data[1];
            
            if next_track == 0 {
                break;
            }
            
            // Validate next track/sector
            if next_track > self.tracks || 
               next_sector >= SECTORS_PER_TRACK[(next_track - 1) as usize] {
                return Err(D64Error::InvalidTrackSector { 
                    track: next_track, 
                    sector: next_sector 
                });
            }
            
            track = next_track;
            sector = next_sector;
        }
        
        info!("Completed tracing file: {}. Total blocks: {}", filename, sectors.len());
        Ok(sectors)
    }

    /// Lists all files on the disk
    ///
    /// # Returns
    ///
    /// A Result containing either a vector of filenames or an error
    pub fn list_files(&self) -> Result<Vec<String>, D64Error> {
        self.get_file_details().map(|details| {
            details.into_iter().map(|info| info.name).collect()
        })
    }

    /// Gets detailed information about all files on the disk
    ///
    /// # Returns
    ///
    /// A Result containing either a vector of FileInfo structs or an error
    pub fn get_file_details(&self) -> Result<Vec<FileInfo>, D64Error> {
        let mut files = Vec::new();
        let dir_track = DIRECTORY_TRACK;
        let mut sector = DIRECTORY_FIRST_SECTOR;
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops from corrupt directory structures
            if !visited.insert((dir_track, sector)) {
                return Err(D64Error::CorruptDirectory { 
                    reason: "Circular reference in directory sectors".to_string() 
                });
            }
            
            let data = self.read_sector(dir_track, sector)?;
            
            // Process the 8 directory entries in this sector
            for i in (0..SECTOR_SIZE).step_by(32) {
                let file_type = data[i + 2];
                
                // Skip unused entries (file_type == 0)
                if file_type == 0 {
                    continue;
                }
                
                // Only process valid file types (bits 0-3 indicate file type)
                if file_type & 0x07 != 0 {
                    // Find the end of the filename (padded with 0xA0)
                    let name_end = data[i + 5..i + 21]
                        .iter()
                        .position(|&x| x == PETSCII_SPACE)
                        .unwrap_or(16);
                    
                    let name = petscii_to_ascii(&data[i + 5..i + 5 + name_end]);
                    let track = data[i + 3];
                    let sector = data[i + 4];
                    let size_blocks = u16::from(data[i + 30]) | (u16::from(data[i + 31]) << 8);
                    
                    files.push(FileInfo {
                        file_type,
                        name,
                        track,
                        sector,
                        size_blocks,
                    });
                }
            }
            
            // Follow chain to next directory sector
            let next_track = data[0];
            let next_sector = data[1];
            
            // End of directory chain
            if next_track == 0 || (next_track == dir_track && next_sector == DIRECTORY_FIRST_SECTOR) {
                break;
            }
            
            // Validate next directory sector
            if next_track != dir_track || next_sector >= SECTORS_PER_TRACK[(dir_track - 1) as usize] {
                return Err(D64Error::CorruptDirectory { 
                    reason: format!("Invalid directory chain: track {}, sector {}", next_track, next_sector) 
                });
            }
            
            sector = next_sector;
        }
        
        Ok(files)
    }

    /// Extracts a file from the disk image
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to extract
    ///
    /// # Returns
    ///
    /// A Result containing either the file content as a vector of bytes or an error
    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>, D64Error> {
        let (mut track, mut sector) = self.find_file(filename)?;
        let mut content = Vec::new();
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops from corrupt files
            if !visited.insert((track, sector)) {
                return Err(D64Error::CircularReference);
            }
            
            let data = self.read_sector(track, sector)?;
            let next_track = data[0];
            let next_sector = data[1];
            
            // Determine how many bytes to read from this sector
            // If this is the last sector (next_track == 0), then next_sector
            // indicates how many bytes are used in this sector
            let bytes_to_read = if next_track == 0 { 
                next_sector as usize 
            } else { 
                MAX_SECTOR_DATA 
            };
            
            // Add sector data to content
            content.extend_from_slice(&data[2..2 + bytes_to_read]);
            
            // If this was the last sector, we're done
            if next_track == 0 {
                break;
            }
            
            // Validate next track/sector
            if next_track > self.tracks || 
               next_sector >= SECTORS_PER_TRACK[(next_track - 1) as usize] {
                return Err(D64Error::InvalidTrackSector { 
                    track: next_track, 
                    sector: next_sector 
                });
            }
            
            track = next_track;
            sector = next_sector;
        }
        
        Ok(content)
    }

    /// Inserts a file into the disk image
    ///
    /// # Arguments
    ///
    /// * `filename` - Name for the file
    /// * `content` - Content of the file as bytes
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn insert_file(&mut self, filename: &str, content: &[u8]) -> Result<(), D64Error> {
        // Check if the file already exists
        if self.find_file(filename).is_ok() {
            // Delete existing file with the same name
            self.delete_file(filename)?;
        }
        
        let mut remaining = content;
        let mut chain = Vec::new();
        
        // Split the content into sectors
        while !remaining.is_empty() {
            // Find a free sector
            let (track, sector) = self.find_free_sector()?;
            // Mark the sector as allocated
            self.allocate_sector(track, sector)?;
            
            let mut buffer = [0u8; SECTOR_SIZE];
            let to_write = remaining.len().min(MAX_SECTOR_DATA);
            
            // Copy file data into the sector buffer
            buffer[2..2 + to_write].copy_from_slice(&remaining[..to_write]);
            
            // Add this sector to our chain
            chain.push((track, sector, buffer, to_write));
            
            // Move to the remaining data
            remaining = &remaining[to_write..];
        }
        
        // If we have no sectors (empty file), use a single sector
        if chain.is_empty() {
            let (track, sector) = self.find_free_sector()?;
            self.allocate_sector(track, sector)?;
            let buffer = [0u8; SECTOR_SIZE];
            chain.push((track, sector, buffer, 0));
        }
        
        // Set up the track/sector links between sectors
        for i in 0..chain.len() {
            let mut buffer = chain[i].2;
            let track = chain[i].0;
            let sector = chain[i].1;
            let data_len = chain[i].3;
            
            if i < chain.len() - 1 {
                // Link to the next sector
                let next_track = chain[i + 1].0;
                let next_sector = chain[i + 1].1;
                buffer[0] = next_track;
                buffer[1] = next_sector;
            } else {
                // Last sector: set track to 0 and sector to the number of bytes used
                buffer[0] = 0;
                buffer[1] = data_len as u8;
            }
            
            // Write the modified buffer back to the chain
            chain[i].2 = buffer;
            
            // Write the sector to disk
            self.write_sector(track, sector, &buffer)?;
        }
        
        // Get the first sector in the chain
        if let Some((first_track, first_sector, _, _)) = chain.first() {
            // Create a directory entry for the file
            let dir_entry = self.create_dir_entry(filename, *first_track, *first_sector)?;
            
            // Write the directory entry
            self.write_dir_entry(dir_entry)?;
            
            Ok(())
        } else {
            Err(D64Error::DiskFull)
        }
    }

    /// Deletes a file from the disk image
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to delete
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn delete_file(&mut self, filename: &str) -> Result<(), D64Error> {
        // Make sure the file exists and get its sectors
        let chain = self.trace_file(filename)?;
        
        // Free all sectors used by the file
        for (track, sector) in chain {
            self.free_sector(track, sector)?;
        }
        
        // Find and mark the directory entry as deleted
        let dir_track = DIRECTORY_TRACK;
        let mut sector = DIRECTORY_FIRST_SECTOR;
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops
            if !visited.insert((dir_track, sector)) {
                return Err(D64Error::CorruptDirectory { 
                    reason: "Circular reference in directory".to_string() 
                });
            }
            
            let data = self.get_sector_mut(dir_track, sector)?;
            
            // Search this directory sector for the file
            for i in (0..SECTOR_SIZE).step_by(32) {
                if data[i + 2] != 0 {
                    // Extract the filename
                    let name_end = data[i + 5..i + 21]
                        .iter()
                        .position(|&x| x == PETSCII_SPACE)
                        .unwrap_or(16);
                    
                    let name = petscii_to_ascii(&data[i + 5..i + 5 + name_end]);
                    
                    // If this is our file, mark it as deleted (file_type = 0)
                    if name.trim() == filename.trim() {
                        data[i + 2] = 0;
                        return Ok(());
                    }
                }
            }
            
            // Follow chain to next directory sector
            let next_sector = data[1];
            if next_sector == 0 {
                break;
            }
            
            sector = next_sector;
        }
        
        // If we get here, the file was not found in the directory
        Err(D64Error::FileNotFound { filename: filename.to_string() })
    }

    /// Gets the size of a file in bytes
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file
    ///
    /// # Returns
    ///
    /// A Result containing either the file size in bytes or an error
    pub fn file_size(&self, filename: &str) -> Result<usize, D64Error> {
        let (mut track, mut sector) = self.find_file(filename)?;
        let mut size = 0;
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops
            if !visited.insert((track, sector)) {
                return Err(D64Error::CircularReference);
            }
            
            let data = self.read_sector(track, sector)?;
            let next_track = data[0];
            let next_sector = data[1];
            
            if next_track == 0 {
                // Last sector - next_sector contains the number of bytes in use
                size += next_sector as usize;
                break;
            }
            
            // Full sector minus the two bytes for track/sector link
            size += MAX_SECTOR_DATA;
            
            // Validate next track/sector
            if next_track > self.tracks || 
               next_sector >= SECTORS_PER_TRACK[(next_track - 1) as usize] {
                return Err(D64Error::InvalidTrackSector { 
                    track: next_track, 
                    sector: next_sector 
                });
            }
            
            track = next_track;
            sector = next_sector;
        }
        
        Ok(size)
    }

    /// Renames a file on the disk
    ///
    /// # Arguments
    ///
    /// * `old_name` - Current name of the file
    /// * `new_name` - New name for the file
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn rename_file(&mut self, old_name: &str, new_name: &str) -> Result<(), D64Error> {
        let dir_track = DIRECTORY_TRACK;
        let mut sector = DIRECTORY_FIRST_SECTOR;
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops
            if !visited.insert((dir_track, sector)) {
                return Err(D64Error::CorruptDirectory { 
                    reason: "Circular reference in directory".to_string() 
                });
            }
            
            let data = self.get_sector_mut(dir_track, sector)?;
            
            // Search this directory sector for the file
            for i in (0..SECTOR_SIZE).step_by(32) {
                if data[i + 2] != 0 {
                    // Extract the filename
                    let name_end = data[i + 5..i + 21]
                        .iter()
                        .position(|&x| x == PETSCII_SPACE)
                        .unwrap_or(16);
                    
                    let name = petscii_to_ascii(&data[i + 5..i + 5 + name_end]);
                    
                    // If this is our file, rename it
                    if name.trim() == old_name.trim() {
                        // Convert new name to PETSCII
                        let new_name_bytes = ascii_to_petscii(new_name);
                        
                        // Fill the filename field with PETSCII spaces
                        data[i + 5..i + 21].fill(PETSCII_SPACE);
                        
                        // Copy the new name (up to 16 chars)
                        let copy_len = new_name_bytes.len().min(16);
                        data[i + 5..i + 5 + copy_len].copy_from_slice(&new_name_bytes[..copy_len]);
                        
                        return Ok(());
                    }
                }
            }
            
            // Follow chain to next directory sector
            let next_sector = data[1];
            if next_sector == 0 {
                break;
            }
            
            sector = next_sector;
        }
        
        // If we get here, the file was not found
        Err(D64Error::FileNotFound { filename: old_name.to_string() })
    }

    /// Finds a file in the directory and returns its first track/sector
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to find
    ///
    /// # Returns
    ///
    /// A Result containing either a (track, sector) tuple or an error
    fn find_file(&self, filename: &str) -> Result<(u8, u8), D64Error> {
        let dir_track = DIRECTORY_TRACK;
        let mut sector = DIRECTORY_FIRST_SECTOR;
        let trimmed_filename = filename.trim();
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops
            if !visited.insert((dir_track, sector)) {
                return Err(D64Error::CorruptDirectory { 
                    reason: "Circular reference in directory".to_string() 
                });
            }
            
            let data = self.read_sector(dir_track, sector)?;
            
            // Search this directory sector for the file
            for i in (0..SECTOR_SIZE).step_by(32) {
                // Skip deleted files
                if data[i + 2] == 0 {
                    continue;
                }
                
                // Check if this is a valid file entry
                if data[i + 2] & 0x07 != 0 {
                    // Extract the filename
                    let name_end = data[i + 5..i + 21]
                        .iter()
                        .position(|&x| x == PETSCII_SPACE)
                        .unwrap_or(16);
                    
                    let name = petscii_to_ascii(&data[i + 5..i + 5 + name_end]);
                    
                    // If this is our file, return its track/sector
                    if name.trim() == trimmed_filename {
                        return Ok((data[i + 3], data[i + 4]));
                    }
                }
            }
            
            // Follow chain to next directory sector
            let next_sector = data[1];
            if next_sector == 0 {
                break;
            }
            
            sector = next_sector;
        }
        
        // If we get here, the file was not found
        Err(D64Error::FileNotFound { filename: filename.to_string() })
    }

    /// Reads the Block Availability Map (BAM)
    ///
    /// # Returns
    ///
    /// A Result containing either a BAM instance or an error
    pub fn read_bam(&self) -> Result<BAM, D64Error> {
        let bam_data = self.read_sector(DIRECTORY_TRACK, BAM_SECTOR)?;
        BAM::from_sector_data(bam_data, self.tracks)
    }

    /// Writes the Block Availability Map (BAM) to the disk
    ///
    /// # Arguments
    ///
    /// * `bam` - The BAM instance to write
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn write_bam(&mut self, bam: &BAM) -> Result<(), D64Error> {
        let bam_data = bam.to_sector_data();
        self.write_sector(DIRECTORY_TRACK, BAM_SECTOR, &bam_data)
    }

    /// Marks a sector as allocated in the BAM
    ///
    /// # Arguments
    ///
    /// * `track` - Track number
    /// * `sector` - Sector number
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn allocate_sector(&mut self, track: u8, sector: u8) -> Result<(), D64Error> {
        let mut bam = self.read_bam()?;
        info!("Allocating sector: track {}, sector {}", track, sector);
        bam.allocate_sector(track, sector)?;
        self.write_bam(&bam)
    }

    /// Marks a sector as free in the BAM
    ///
    /// # Arguments
    ///
    /// * `track` - Track number
    /// * `sector` - Sector number
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    pub fn free_sector(&mut self, track: u8, sector: u8) -> Result<(), D64Error> {
        let mut bam = self.read_bam()?;
        bam.free_sector(track, sector)?;
        self.write_bam(&bam)
    }

    /// Finds a free sector on the disk
    ///
    /// # Returns
    ///
    /// A Result containing either a (track, sector) tuple or an error
    pub fn find_free_sector(&self) -> Result<(u8, u8), D64Error> {
        let bam = self.read_bam()?;
        
        // Optimized sector allocation strategy that minimizes seek time
        let start_track = DIRECTORY_TRACK;
        let mut distance = 1;
        
        info!("Searching for a free sector starting from track {}", start_track);
        while distance < self.tracks {
            let track_in = start_track.saturating_sub(distance);
            if track_in > 0 {
                if let Some(sector) = bam.find_free_sector(track_in) {
                    info!("Found free sector: track {}, sector {}", track_in, sector);
                    return Ok((track_in, sector));
                }
            }
            
            let track_out = start_track + distance;
            if track_out <= self.tracks {
                if let Some(sector) = bam.find_free_sector(track_out) {
                    info!("Found free sector: track {}, sector {}", track_out, sector);
                    return Ok((track_out, sector));
                }
            }
            
            distance += 1;
        }
        
        info!("No free sectors available");
        Err(D64Error::DiskFull)
    }

    /// Creates a directory entry for a file
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file
    /// * `track` - Track of the first sector
    /// * `sector` - Sector of the first sector
    ///
    /// # Returns
    ///
    /// A Result containing either a 32-byte directory entry or an error
    fn create_dir_entry(&self, filename: &str, track: u8, sector: u8) -> Result<[u8; 32], D64Error> {
        let mut entry = [0u8; 32];
        
        // Set file type: PRG + Closed (0x82)
        entry[2] = file_types::PRG | file_types::CLOSED;
        
        // Set track/sector of first block
        entry[3] = track;
        entry[4] = sector;
        
        // Fill filename area with PETSCII spaces
        entry[5..21].fill(PETSCII_SPACE);
        
        // Convert filename to PETSCII
        let name_bytes = ascii_to_petscii(filename);
        let copy_len = name_bytes.len().min(16);
        entry[5..5 + copy_len].copy_from_slice(&name_bytes[..copy_len]);
        
        // Set size to 0 blocks initially (will be updated by DOS)
        entry[28] = 0;
        entry[29] = 0;
        
        Ok(entry)
    }

    /// Writes a directory entry to the disk
    ///
    /// # Arguments
    ///
    /// * `entry` - 32-byte directory entry to write
    ///
    /// # Returns
    ///
    /// A Result containing either () or an error
    fn write_dir_entry(&mut self, entry: [u8; 32]) -> Result<(), D64Error> {
        let dir_track = DIRECTORY_TRACK;
        let mut sector = DIRECTORY_FIRST_SECTOR;
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops
            if !visited.insert((dir_track, sector)) {
                return Err(D64Error::CorruptDirectory { 
                    reason: "Circular reference in directory".to_string() 
                });
            }
            
            let mut data = self.read_sector(dir_track, sector)?.to_vec();
            
            // Find an empty directory slot
            for i in (0..SECTOR_SIZE).step_by(32) {
                if data[i + 2] == 0 {
                    // Found an empty slot, write the entry
                    data[i..i + 32].copy_from_slice(&entry);
                    self.write_sector(dir_track, sector, &data)?;
                    return Ok(());
                }
            }
            
            // No empty slots in this sector, follow chain
            let next_sector = data[1];
            
            if next_sector == 0 {
                // End of directory chain, need to allocate a new sector
                let (new_track, new_sector) = self.find_free_sector()?;
                
                // Make sure the new sector is on the directory track
                if new_track != dir_track {
                    return Err(D64Error::DiskFull);
                }
                
                // Allocate the new sector
                self.allocate_sector(new_track, new_sector)?;
                
                // Update the link in the current sector
                data[1] = new_sector;
                self.write_sector(dir_track, sector, &data)?;
                
                // Initialize the new directory sector
                let mut new_data = [0u8; SECTOR_SIZE];
                new_data[0] = 0; // No next track
                new_data[1] = 0; // No next sector
                
                // Write the entry to the first slot
                new_data[0..32].copy_from_slice(&entry);
                self.write_sector(dir_track, new_sector, &new_data)?;
                
                return Ok(());
            }
            
            sector = next_sector;
        }
    }
}