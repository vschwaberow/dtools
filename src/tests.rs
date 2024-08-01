// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/tests.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

use super::*;
use std::io::Cursor;

fn create_mock_d64() -> D64 {
    let mut d64 = D64::new(35).unwrap();
    
    // Create a simple file system structure
    let mut bam = d64.read_bam().unwrap();
    bam.set_disk_name("TEST DISK");
    bam.set_disk_id("2A");
    d64.write_bam(&bam).unwrap();

    // Add a file
    let content = b"Hello, World!";
    d64.insert_file("TEST FILE", content).unwrap();

    d64
}

#[test]
fn test_new_d64() {
    let d64 = D64::new(35).unwrap();
    assert_eq!(d64.tracks, 35);
    assert_eq!(d64.data.len(), D64_35_TRACKS_SIZE);
}

#[test]
fn test_read_write_sector() {
    let mut d64 = create_mock_d64();
    let test_data = [0x42; 256];
    d64.write_sector(1, 0, &test_data).unwrap();
    let read_data = d64.read_sector(1, 0).unwrap();
    assert_eq!(read_data, &test_data);
}

#[test]
fn test_bam_operations() {
    let mut d64 = create_mock_d64();
    let mut bam = d64.read_bam().unwrap();
    
    assert_eq!(bam.get_disk_name(), "TEST DISK");
    assert_eq!(bam.get_disk_id(), "2A");

    bam.allocate_sector(1, 0).unwrap();
    assert_eq!(bam.get_free_sectors_count(1).unwrap(), 20);

    bam.free_sector(1, 0).unwrap();
    assert_eq!(bam.get_free_sectors_count(1).unwrap(), 21);
}

#[test]
fn test_file_operations() {
    let mut d64 = create_mock_d64();
    
    let files = d64.list_files().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], "TEST FILE");

    let content = d64.extract_file("TEST FILE").unwrap();
    assert_eq!(content, b"Hello, World!");

    d64.insert_file("ANOTHER FILE", b"New content").unwrap();
    let files = d64.list_files().unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.contains(&"ANOTHER FILE".to_string()));
}

#[test]
fn test_find_free_sector() {
    let mut d64 = create_mock_d64();
    let (track, sector) = d64.find_free_sector().unwrap();
    assert!(track > 0 && track <= 35);
    assert!(sector < SECTORS_PER_TRACK[(track - 1) as usize]);
}

#[test]
fn test_trace_file() {
    let d64 = create_mock_d64();
    let sectors = d64.trace_file("TEST FILE").unwrap();
    assert!(!sectors.is_empty());
    assert_eq!(sectors[0].0, 18); // First sector should be on track 18 (directory track)
}

#[test]
#[should_panic(expected = "InvalidTrackSector")]
fn test_invalid_sector_access() {
    let d64 = create_mock_d64();
    d64.read_sector(0, 0).unwrap(); // Track 0 doesn't exist
}

#[test]
fn test_petscii_conversion() {
    let ascii = "HELLO, WORLD!";
    let petscii = ascii_to_petscii(ascii);
    let back_to_ascii = petscii_to_ascii(&petscii);
    assert_eq!(ascii, back_to_ascii);
}