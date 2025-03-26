// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/bam.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! Block Availability Map (BAM) functionality

use crate::constants::*;
use crate::error::D64Error;
use crate::petscii::{petscii_to_ascii, ascii_to_petscii};
use log::{info, warn};

/// Represents the Block Availability Map (BAM) of a D64 disk
#[derive(Clone, Debug)]
pub struct BAM {
    /// Number of tracks in the BAM
    pub tracks: u8,
    /// Number of free sectors per track
    pub free_sectors: [u8; MAX_TRACKS as usize],
    /// Bitmap of free sectors (3 bytes per track)
    pub bitmap: [[u8; 3]; MAX_TRACKS as usize],
    /// Disk name (PETSCII encoding, padded with 0xA0)
    pub disk_name: [u8; 16],
    /// Disk ID (PETSCII encoding)
    pub disk_id: [u8; 2],
    /// DOS type byte
    pub dos_type: u8,
}

impl BAM {
    /// Creates a BAM instance from sector data
    ///
    /// # Arguments
    ///
    /// * `data` - 256-byte sector data
    /// * `tracks` - Number of tracks in the disk
    ///
    /// # Returns
    ///
    /// A Result containing either a BAM instance or an error
    pub fn from_sector_data(data: &[u8], tracks: u8) -> Result<Self, D64Error> {
        if data.len() != SECTOR_SIZE {
            return Err(D64Error::InvalidData { 
                reason: "BAM sector data must be 256 bytes".to_string() 
            });
        }
        
        let mut bam = BAM {
            tracks,
            free_sectors: [0; MAX_TRACKS as usize],
            bitmap: [[0; 3]; MAX_TRACKS as usize],
            disk_name: [0; 16],
            disk_id: [0; 2],
            dos_type: data[2],
        };
        
        // Copy free sector counts and bitmaps
        for track in 0..tracks as usize {
            bam.free_sectors[track] = data[4 + track * 4];
            bam.bitmap[track][0] = data[5 + track * 4];
            bam.bitmap[track][1] = data[6 + track * 4];
            bam.bitmap[track][2] = data[7 + track * 4];
        }
        
        // Copy disk name and ID
        bam.disk_name.copy_from_slice(&data[144..160]);
        bam.disk_id.copy_from_slice(&data[162..164]);
        
        Ok(bam)
    }

    /// Converts the BAM to sector data
    ///
    /// # Returns
    ///
    /// A 256-byte vector containing the sector data
    pub fn to_sector_data(&self) -> Vec<u8> {
        let mut data = vec![0; SECTOR_SIZE];
        
        // Set directory location
        data[0] = DIRECTORY_TRACK;
        data[1] = DIRECTORY_FIRST_SECTOR;
        
        // Set DOS type
        data[2] = self.dos_type;
        
        // Copy free sector counts and bitmaps
        for track in 0..self.tracks as usize {
            data[4 + track * 4] = self.free_sectors[track];
            data[5 + track * 4] = self.bitmap[track][0];
            data[6 + track * 4] = self.bitmap[track][1];
            data[7 + track * 4] = self.bitmap[track][2];
        }
        
        // Copy disk name and ID
        data[144..160].copy_from_slice(&self.disk_name);
        data[162..164].copy_from_slice(&self.disk_id);
        
        // Set DOS version at offset 165-166
        data[165] = 0x32;  // ASCII '2'
        data[166] = 0x41;  // ASCII 'A'
        
        data
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
        if track == 0 || track > self.tracks || sector >= SECTORS_PER_TRACK[(track - 1) as usize] {
            return Err(D64Error::InvalidTrackSector { track, sector });
        }
        
        let idx = (track - 1) as usize;
        let byte_idx = (sector / 8) as usize;
        let bit_idx = sector % 8;
        
        // If the bit is already cleared, the sector is already allocated
        if self.bitmap[idx][byte_idx] & (1 << bit_idx) == 0 {
            warn!("Sector already allocated: track {}, sector {}", track, sector);
            return Ok(());
        }
        
        // Clear the bit to mark the sector as allocated
        info!("Bitmap byte before clearing: {:08b}", self.bitmap[idx][byte_idx]);
        self.bitmap[idx][byte_idx] &= !(1 << bit_idx);
        info!("Bitmap byte after clearing: {:08b}", self.bitmap[idx][byte_idx]);
        
        // Decrement free sector count
        if self.free_sectors[idx] > 0 {
            self.free_sectors[idx] -= 1;
        }
        
        info!("Allocated sector: track {}, sector {}", track, sector);
        info!("Free sectors remaining on track {}: {}", track, self.free_sectors[idx]);
        
        Ok(())
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
        if track == 0 || track > self.tracks || sector >= SECTORS_PER_TRACK[(track - 1) as usize] {
            return Err(D64Error::InvalidTrackSector { track, sector });
        }
        
        let idx = (track - 1) as usize;
        let byte_idx = (sector / 8) as usize;
        let bit_idx = sector % 8;
        
        // If the bit is already set, the sector is already free
        if self.bitmap[idx][byte_idx] & (1 << bit_idx) != 0 {
            warn!("Sector already free: track {}, sector {}", track, sector);
            return Ok(());
        }
        
        // Set the bit to mark the sector as free
        info!("Bitmap byte before setting: {:08b}", self.bitmap[idx][byte_idx]);
        self.bitmap[idx][byte_idx] |= 1 << bit_idx;
        info!("Bitmap byte after setting: {:08b}", self.bitmap[idx][byte_idx]);
        
        // Increment free sector count
        self.free_sectors[idx] += 1;
        
        // Ensure free count doesn't exceed sectors per track
        let max_sectors = SECTORS_PER_TRACK[idx];
        if self.free_sectors[idx] > max_sectors {
            info!("Correcting free sector count for track {}: {} -> {}", 
                  track, self.free_sectors[idx], max_sectors);
            self.free_sectors[idx] = max_sectors;
        }
        
        info!("Freed sector: track {}, sector {}", track, sector);
        info!("Free sectors now on track {}: {}", track, self.free_sectors[idx]);
        
        Ok(())
    }

    /// Finds a free sector on a specific track
    ///
    /// # Arguments
    ///
    /// * `track` - Track number to search
    ///
    /// # Returns
    ///
    /// An Option containing either the sector number or None if no free sectors
    pub fn find_free_sector(&self, track: u8) -> Option<u8> {
        if track == 0 || track > self.tracks {
            info!("Invalid track number: {}", track);
            return None;
        }
        
        let idx = (track - 1) as usize;
        
        // If track has no free sectors, return None immediately
        if self.free_sectors[idx] == 0 {
            info!("No free sectors on track {}", track);
            return None;
        }
        
        // Examine the bitmap to find a free sector
        for (byte_idx, &byte) in self.bitmap[idx].iter().enumerate() {
            if byte != 0 {
                // Find the first set bit in this byte
                for bit_idx in 0..8 {
                    if byte & (1 << bit_idx) != 0 {
                        let sector = (byte_idx as u8) * 8 + bit_idx;
                        if sector < SECTORS_PER_TRACK[idx] {
                            info!("Found free sector on track {}: sector {}", track, sector);
                            return Some(sector);
                        }
                    }
                }
            }
        }
        
        // If we get here, there's an inconsistency between the free count and bitmap
        info!("Inconsistency detected: free sector count does not match bitmap on track {}", track);
        None
    }

    /// Gets the number of free sectors on a track
    ///
    /// # Arguments
    ///
    /// * `track` - Track number
    ///
    /// # Returns
    ///
    /// A Result containing either the free sector count or an error
    pub fn get_free_sectors_count(&self, track: u8) -> Result<u8, D64Error> {
        if track == 0 || track > self.tracks {
            return Err(D64Error::InvalidTrackSector { 
                track, 
                sector: 0 
            });
        }
        
        Ok(self.free_sectors[(track - 1) as usize])
    }

    /// Gets the disk name as an ASCII string
    ///
    /// # Returns
    ///
    /// The disk name as a trimmed string
    pub fn get_disk_name(&self) -> String {
        let end = self.disk_name
            .iter()
            .position(|&c| c == PETSCII_SPACE)
            .unwrap_or(self.disk_name.len());
            
        let ascii = petscii_to_ascii(&self.disk_name[..end]);
        ascii.trim().to_string()
    }

    /// Gets the disk ID as an ASCII string
    ///
    /// # Returns
    ///
    /// The disk ID as a string
    pub fn get_disk_id(&self) -> String {
        petscii_to_ascii(&self.disk_id)
    }

    /// Sets the disk name
    ///
    /// # Arguments
    ///
    /// * `name` - New disk name
    pub fn set_disk_name(&mut self, name: &str) {
        // Convert name to PETSCII
        let name_bytes = ascii_to_petscii(name);
        
        // Fill with PETSCII spaces
        self.disk_name.fill(PETSCII_SPACE);
        
        // Copy name (up to 16 chars)
        let len = name_bytes.len().min(16);
        self.disk_name[..len].copy_from_slice(&name_bytes[..len]);
    }
    
    /// Sets the disk ID
    ///
    /// # Arguments
    ///
    /// * `id` - New disk ID
    pub fn set_disk_id(&mut self, id: &str) {
        // Convert ID to PETSCII
        let id_bytes = ascii_to_petscii(id);
        
        // Copy ID (up to 2 chars)
        let len = id_bytes.len().min(2);
        self.disk_id[..len].copy_from_slice(&id_bytes[..len]);
    }
}