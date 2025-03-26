// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/tests.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! Tests for the D64 library

use crate::{D64, BAM, constants::*, petscii::{petscii_to_ascii, ascii_to_petscii}};
use log::info;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn initialize_logging() {
    INIT.call_once(|| {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off"))
            .format_timestamp(None)
            .init();
    });
}

#[test]
fn test_create_new_disk() {
    initialize_logging();
    let d64 = D64::new(35).unwrap();
    assert_eq!(d64.tracks, 35);
    assert_eq!(d64.data.len(), D64_35_TRACKS_SIZE);
}

#[test]
fn test_read_write_sector() {
    initialize_logging();
    let mut d64 = D64::new(35).unwrap();
    
    // Create test data
    let mut test_data = [0u8; SECTOR_SIZE];
    for i in 0..SECTOR_SIZE {
        test_data[i] = (i % 256) as u8;
    }
    
    // Write test data to sector
    d64.write_sector(1, 0, &test_data).unwrap();
    
    // Read back and verify
    let read_data = d64.read_sector(1, 0).unwrap();
    assert_eq!(read_data, &test_data);
}

#[test]
fn test_format_disk() {
    initialize_logging();
    let mut d64 = D64::new(35).unwrap();
    d64.format("TEST DISK", "T1").unwrap();
    
    // Read BAM and check disk name and ID
    let bam = d64.read_bam().unwrap();
    assert_eq!(bam.get_disk_name(), "TEST DISK");
    assert_eq!(bam.get_disk_id(), "T1");
}

#[test]
fn test_bam_allocate_free_sector() {
    initialize_logging();
    let mut bam = BAM {
        tracks: 35,
        free_sectors: [0; MAX_TRACKS as usize],
        bitmap: [[0; 3]; MAX_TRACKS as usize],
        disk_name: [0; 16],
        disk_id: [0; 2],
        dos_type: 0x41,
    };
    
    // Initialize all sectors as free
    for track in 1..=35 {
        let idx = (track - 1) as usize;
        let sectors = SECTORS_PER_TRACK[idx];
        bam.free_sectors[idx] = sectors;
        
        // Set bitmap bytes for free sectors
        bam.bitmap[idx][0] = 0xFF;
        bam.bitmap[idx][1] = 0xFF;
        bam.bitmap[idx][2] = if sectors > 16 { 0xFF } else { (1 << sectors) - 1 };
    }
    
    // Test allocating a sector
    info!("Bitmap before allocation: {:?}", bam.bitmap[0]);
    bam.allocate_sector(1, 0).unwrap();
    info!("Bitmap after allocation: {:?}", bam.bitmap[0]);
    assert_eq!(bam.free_sectors[0], SECTORS_PER_TRACK[0] - 1);
    assert_eq!(bam.bitmap[0][0] & 0x01, 0); // First bit should be cleared (0 = allocated)
    
    // Test freeing a sector
    bam.free_sector(1, 0).unwrap();
    assert_eq!(bam.free_sectors[0], SECTORS_PER_TRACK[0]);
    assert_eq!(bam.bitmap[0][0] & 0x01, 1); // First bit should be set (1 = free)
}

#[test]
fn test_petscii_conversion() {
    initialize_logging();
    let original = "HELLO WORLD";
    let petscii = ascii_to_petscii(original);
    let ascii = petscii_to_ascii(&petscii);
    
    assert_eq!(ascii, original);
}

#[test]
fn test_insert_and_extract_file() {
    initialize_logging();
    let mut d64 = D64::new(35).unwrap();
    // Format the disk to initialize the BAM
    d64.format("TEST DISK", "01").unwrap();
    
    let content = b"This is a test file.";
    let filename = "TESTFILE";

    // Insert the file
    info!("Inserting file: {}", filename);
    d64.insert_file(filename, content).unwrap();

    // Extract the file and verify content
    info!("Extracting file: {}", filename);
    let extracted_content = d64.extract_file(filename).unwrap();
    assert_eq!(extracted_content, content);
}

#[test]
fn test_delete_file() {
    initialize_logging();
    let mut d64 = D64::new(35).unwrap();
    // Format the disk to initialize the BAM
    d64.format("TEST DISK", "01").unwrap();
    
    let content = b"Delete test content.";
    let filename = "DELETE";

    // Insert the file
    info!("Inserting file: {}", filename);
    d64.insert_file(filename, content).unwrap();

    // Delete the file
    info!("Deleting file: {}", filename);
    d64.delete_file(filename).unwrap();

    // Verify the file is deleted
    info!("Listing files after deletion");
    let files = d64.list_files().unwrap();
    assert!(!files.contains(&filename.to_string()));
}

#[test]
fn test_list_files() {
    initialize_logging();
    let mut d64 = D64::new(35).unwrap();
    // Format the disk to initialize the BAM
    d64.format("TEST DISK", "01").unwrap();
    
    let content1 = b"File 1 content.";
    let content2 = b"File 2 content.";

    info!("Inserting FILE1");
    d64.insert_file("FILE1", content1).unwrap();
    info!("Inserting FILE2");
    d64.insert_file("FILE2", content2).unwrap();

    info!("Listing files");
    let files = d64.list_files().unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.contains(&"FILE1".to_string()));
    assert!(files.contains(&"FILE2".to_string()));
}

#[test]
fn test_rename_file() {
    initialize_logging();
    let mut d64 = D64::new(35).unwrap();
    // Format the disk to initialize the BAM
    d64.format("TEST DISK", "01").unwrap();
    
    let content = b"Rename test content.";
    let old_name = "OLDNAME";
    let new_name = "NEWNAME";

    info!("Inserting file: {}", old_name);
    d64.insert_file(old_name, content).unwrap();

    info!("Renaming file from {} to {}", old_name, new_name);
    d64.rename_file(old_name, new_name).unwrap();

    info!("Listing files after renaming");
    let files = d64.list_files().unwrap();
    assert!(files.contains(&new_name.to_string()));
    assert!(!files.contains(&old_name.to_string()));
}