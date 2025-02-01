// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/lib.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use thiserror::Error;

#[cfg(test)]
mod tests;

const D64_35_TRACKS_SIZE: usize = 174848;
const D64_40_TRACKS_SIZE: usize = 196608;
const MAX_TRACKS: u8 = 40;
const SECTORS_PER_TRACK: [u8; 40] = [
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 19, 19, 19,
    19, 19, 19, 19, 18, 18, 18, 18, 18, 18, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
];

#[derive(Error, Debug)]
pub enum D64Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid D64 file size")]
    InvalidFileSize,
    #[error("Invalid track or sector")]
    InvalidTrackSector,
    #[error("File not found")]
    FileNotFound,
    #[error("Disk full")]
    DiskFull,
}

pub struct D64 {
    pub data: Vec<u8>,
    pub tracks: u8,
    track_offsets: Vec<usize>,
}

pub struct BAM {
    pub tracks: u8,
    pub free_sectors: [u8; 40],
    pub bitmap: [[u8; 3]; 40],
    pub disk_name: [u8; 16],
    pub disk_id: [u8; 2],
    pub dos_type: u8,
}

pub fn petscii_to_ascii(petscii: &[u8]) -> String {
    let mut s = String::with_capacity(petscii.len());
    for &c in petscii {
        let ch = if (0x20..=0x5F).contains(&c) {
            c as char
        } else if (0xC1..=0xDA).contains(&c) {
            (c - 0x80) as char
        } else {
            '?'
        };
        s.push(ch);
    }
    s
}

pub fn ascii_to_petscii(ascii: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(ascii.len());
    for c in ascii.chars() {
        let byte = if (' ' ..= '_').contains(&c) {
            c as u8
        } else if ('a' ..= 'z').contains(&c) {
            (c as u8) - 32
        } else {
            0x3F
        };
        v.push(byte);
    }
    v
}

impl D64 {
    fn compute_track_offsets(tracks: u8) -> Vec<usize> {
        let mut offsets = Vec::with_capacity(tracks as usize);
        let mut offset = 0;
        for t in 1..=tracks {
            offsets.push(offset);
            offset += SECTORS_PER_TRACK[(t - 1) as usize] as usize * 256;
        }
        offsets
    }

    pub fn new(tracks: u8) -> Result<Self, D64Error> {
        if tracks != 35 && tracks != 40 {
            return Err(D64Error::InvalidFileSize);
        }
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
    }

    pub fn from_file(path: &str) -> Result<Self, D64Error> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let tracks = match data.len() {
            D64_35_TRACKS_SIZE => 35,
            D64_40_TRACKS_SIZE => 40,
            _ => return Err(D64Error::InvalidFileSize),
        };
        Ok(Self {
            data,
            tracks,
            track_offsets: Self::compute_track_offsets(tracks),
        })
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), D64Error> {
        let mut file = File::create(path)?;
        file.write_all(&self.data)?;
        Ok(())
    }

    fn sector_offset(&self, track: u8, sector: u8) -> Result<usize, D64Error> {
        if track == 0 || track > self.tracks || sector >= SECTORS_PER_TRACK[(track - 1) as usize] {
            return Err(D64Error::InvalidTrackSector);
        }
        Ok(self.track_offsets[(track - 1) as usize] + sector as usize * 256)
    }

    pub fn get_sector_mut(&mut self, track: u8, sector: u8) -> Result<&mut [u8], D64Error> {
        let offset = self.sector_offset(track, sector)?;
        Ok(&mut self.data[offset..offset + 256])
    }

    pub fn read_sector(&self, track: u8, sector: u8) -> Result<&[u8], D64Error> {
        let offset = self.sector_offset(track, sector)?;
        Ok(&self.data[offset..offset + 256])
    }

    pub fn write_sector(&mut self, track: u8, sector: u8, data: &[u8]) -> Result<(), D64Error> {
        let offset = self.sector_offset(track, sector)?;
        self.data[offset..offset + 256].copy_from_slice(data);
        Ok(())
    }

    pub fn format(&mut self, disk_name: &str, disk_id: &str) -> Result<(), D64Error> {
        self.data.fill(0);
        let mut bam = [0u8; 256];
        bam[0] = 18;
        bam[1] = 1;
        bam[2] = 0x41;
        for track in 1..=self.tracks {
            let idx = (track - 1) as usize;
            let sectors = SECTORS_PER_TRACK[idx];
            bam[4 + idx * 4] = sectors;
            bam[5 + idx * 4] = 0xFF;
            bam[6 + idx * 4] = 0xFF;
            bam[7 + idx * 4] = if sectors > 16 { 0xFF } else { (1 << sectors) - 1 };
        }
        for track in 18..=19 {
            let idx = (track - 1) as usize;
            bam[4 + idx * 4] = 0;
            bam[5 + idx * 4] = 0;
            bam[6 + idx * 4] = 0;
            bam[7 + idx * 4] = 0;
        }
        let disk_name_bytes = ascii_to_petscii(disk_name);
        let disk_id_bytes = ascii_to_petscii(disk_id);
        bam[144..144 + disk_name_bytes.len()].copy_from_slice(&disk_name_bytes);
        bam[162..164].copy_from_slice(&disk_id_bytes);
        self.write_sector(18, 0, &bam)?;
        let mut dir = [0u8; 256];
        dir[1] = 0xFF;
        self.write_sector(18, 1, &dir)?;
        Ok(())
    }

    pub fn trace_file(&self, filename: &str) -> Result<Vec<(u8, u8)>, D64Error> {
        let (mut track, mut sector) = self.find_file(filename)?;
        let mut sectors = Vec::new();
        loop {
            sectors.push((track, sector));
            let data = self.read_sector(track, sector)?;
            let next_track = data[0];
            let next_sector = data[1];
            if next_track == 0 {
                break;
            }
            track = next_track;
            sector = next_sector;
        }
        Ok(sectors)
    }

    pub fn list_files(&self) -> Result<Vec<String>, D64Error> {
        let mut files = Vec::new();
        let dir_track = 18;
        let mut sector = 1;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert((dir_track, sector)) {
                return Err(D64Error::InvalidTrackSector);
            }
            let data = self.read_sector(dir_track, sector)?;
            for i in (0..256).step_by(32) {
                if data[i + 2] == 0 {
                    continue;
                }
                if data[i + 2] != 0 && data[i + 2] & 0x07 != 0 {
                    let name_end = data[i + 5..i + 21]
                        .iter()
                        .position(|&x| x == 0xA0)
                        .unwrap_or(16);
                    let name = petscii_to_ascii(&data[i + 5..i + 5 + name_end]);
                    files.push(name);
                }
            }
            let next_track = data[0];
            let next_sector = data[1];
            if next_track == 0 || (next_track == 18 && next_sector == 1) {
                break;
            }
            if next_track != 18 || next_sector >= SECTORS_PER_TRACK[17] {
                return Err(D64Error::InvalidTrackSector);
            }
            sector = next_sector;
        }
        Ok(files)
    }

    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>, D64Error> {
        let (mut track, mut sector) = self.find_file(filename)?;
        let mut content = Vec::new();
        loop {
            let data = self.read_sector(track, sector)?;
            let next_track = data[0];
            let next_sector = data[1];
            let bytes_to_read = if next_track == 0 { next_sector } else { 254 };
            content.extend_from_slice(&data[2..2 + bytes_to_read as usize]);
            if next_track == 0 {
                break;
            }
            track = next_track;
            sector = next_sector;
        }
        Ok(content)
    }

    pub fn insert_file(&mut self, filename: &str, content: &[u8]) -> Result<(), D64Error> {
        let mut remaining = content;
        let mut chain = Vec::new();
        while !remaining.is_empty() {
            let (track, sector) = self.find_free_sector()?;
            self.allocate_sector(track, sector)?;
            let mut buffer = [0u8; 256];
            let to_write = if remaining.len() > 254 { 254 } else { remaining.len() };
            buffer[2..2 + to_write].copy_from_slice(&remaining[..to_write]);
            chain.push((track, sector, buffer, to_write));
            remaining = &remaining[to_write..];
        }
        let next_values: Vec<(u8, u8)> = (0..chain.len())
            .map(|i| {
                if i < chain.len() - 1 {
                    let (t, s, _, _) = chain[i + 1];
                    (t, s)
                } else {
                    (0, chain[i].3 as u8)
                }
            })
            .collect();
        for i in 0..chain.len() {
            let (track, sector, ref mut buffer, _data_len) = chain[i];
            let (next_track, next_sector) = next_values[i];
            buffer[0] = next_track;
            buffer[1] = next_sector;
            self.write_sector(track, sector, buffer)?;
        }
        let (first_track, first_sector, _, _) = chain
            .first()
            .ok_or(D64Error::DiskFull)?;
        let dir_entry = self.create_dir_entry(filename, *first_track, *first_sector)?;
        self.write_dir_entry(dir_entry)?;
        Ok(())
    }
    

    pub fn delete_file(&mut self, filename: &str) -> Result<(), D64Error> {
        let _ = self.find_file(filename)?;
        let chain = self.trace_file(filename)?;
        for (track, sector) in chain {
            self.free_sector(track, sector)?;
        }
        let dir_track = 18;
        let mut sector = 1;
        loop {
            let data = self.get_sector_mut(dir_track, sector)?;
            for i in (0..256).step_by(32) {
                if data[i + 2] != 0 {
                    let name_end = data[i + 5..i + 21]
                        .iter()
                        .position(|&x| x == 0xA0)
                        .unwrap_or(16);
                    let name = petscii_to_ascii(&data[i + 5..i + 5 + name_end]);
                    if name.trim() == filename {
                        data[i + 2] = 0;
                        return Ok(());
                    }
                }
            }
            let next_sector = data[1];
            if next_sector == 0 {
                break;
            }
            sector = next_sector;
        }
        Err(D64Error::FileNotFound)
    }

    pub fn file_size(&self, filename: &str) -> Result<usize, D64Error> {
        let (mut track, mut sector) = self.find_file(filename)?;
        let mut size = 0;
        loop {
            let data = self.read_sector(track, sector)?;
            let next_track = data[0];
            let next_sector = data[1];
            if next_track == 0 {
                size += next_sector as usize;
                break;
            }
            size += 254;
            track = next_track;
            sector = next_sector;
        }
        Ok(size)
    }

    pub fn rename_file(&mut self, old_name: &str, new_name: &str) -> Result<(), D64Error> {
        let dir_track = 18;
        let mut sector = 1;
        loop {
            let data = self.get_sector_mut(dir_track, sector)?;
            for i in (0..256).step_by(32) {
                if data[i + 2] != 0 {
                    let name_end = data[i + 5..i + 21]
                        .iter()
                        .position(|&x| x == 0xA0)
                        .unwrap_or(16);
                    let name = petscii_to_ascii(&data[i + 5..i + 5 + name_end]);
                    if name.trim() == old_name {
                        let new_name_bytes = ascii_to_petscii(new_name);
                        data[i + 5..i + 21].fill(0xA0);
                        let copy_len = new_name_bytes.len().min(16);
                        data[i + 5..i + 5 + copy_len].copy_from_slice(&new_name_bytes[..copy_len]);
                        return Ok(());
                    }
                }
            }
            let next_sector = data[1];
            if next_sector == 0 {
                break;
            }
            sector = next_sector;
        }
        Err(D64Error::FileNotFound)
    }

    fn find_file(&self, filename: &str) -> Result<(u8, u8), D64Error> {
        let dir_track = 18;
        let mut sector = 1;
        loop {
            let data = self.read_sector(dir_track, sector)?;
            for i in (0..256).step_by(32) {
                if data[i + 2] != 0 && data[i + 2] & 0x07 != 0 {
                    let name = petscii_to_ascii(&data[i + 5..i + 21]);
                    if name.trim() == filename {
                        return Ok((data[i + 3], data[i + 4]));
                    }
                }
            }
            sector = data[1];
            if sector == 0 {
                break;
            }
        }
        Err(D64Error::FileNotFound)
    }

    pub fn read_bam(&self) -> Result<BAM, D64Error> {
        let bam_data = self.read_sector(18, 0)?;
        BAM::from_sector_data(bam_data, self.tracks)
    }

    pub fn write_bam(&mut self, bam: &BAM) -> Result<(), D64Error> {
        let bam_data = bam.to_sector_data();
        self.write_sector(18, 0, &bam_data)
    }

    pub fn allocate_sector(&mut self, track: u8, sector: u8) -> Result<(), D64Error> {
        let mut bam = self.read_bam()?;
        bam.allocate_sector(track, sector)?;
        self.write_bam(&bam)
    }

    pub fn free_sector(&mut self, track: u8, sector: u8) -> Result<(), D64Error> {
        let mut bam = self.read_bam()?;
        bam.free_sector(track, sector)?;
        self.write_bam(&bam)
    }

    pub fn find_free_sector(&self) -> Result<(u8, u8), D64Error> {
        let bam = self.read_bam()?;
        for track in 1..=self.tracks {
            if let Some(sector) = bam.find_free_sector(track) {
                return Ok((track, sector));
            }
        }
        Err(D64Error::DiskFull)
    }

    fn create_dir_entry(&self, filename: &str, track: u8, sector: u8) -> Result<[u8; 32], D64Error> {
        let mut entry = [0u8; 32];
        entry[2] = 0x82;
        entry[3] = track;
        entry[4] = sector;
        let name_bytes = ascii_to_petscii(filename);
        entry[5..5 + name_bytes.len()].copy_from_slice(&name_bytes);
        Ok(entry)
    }

    fn write_dir_entry(&mut self, entry: [u8; 32]) -> Result<(), D64Error> {
        let dir_track = 18;
        let mut sector = 1;
        loop {
            let mut data = self.read_sector(dir_track, sector)?.to_vec();
            for i in (0..256).step_by(32) {
                if data[i + 2] == 0 {
                    data[i..i + 32].copy_from_slice(&entry);
                    self.write_sector(dir_track, sector, &data)?;
                    return Ok(());
                }
            }
            sector = data[1];
            if sector == 0 {
                return Err(D64Error::DiskFull);
            }
        }
    }
}

impl BAM {
    fn from_sector_data(data: &[u8], tracks: u8) -> Result<Self, D64Error> {
        let mut bam = BAM {
            tracks,
            free_sectors: [0; 40],
            bitmap: [[0; 3]; 40],
            disk_name: [0; 16],
            disk_id: [0; 2],
            dos_type: data[2],
        };
        for track in 0..tracks as usize {
            bam.free_sectors[track] = data[4 + track * 4];
            bam.bitmap[track][0] = data[5 + track * 4];
            bam.bitmap[track][1] = data[6 + track * 4];
            bam.bitmap[track][2] = data[7 + track * 4];
        }
        bam.disk_name.copy_from_slice(&data[144..160]);
        bam.disk_id.copy_from_slice(&data[162..164]);
        Ok(bam)
    }

    fn to_sector_data(&self) -> Vec<u8> {
        let mut data = vec![0; 256];
        data[0] = 18;
        data[1] = 1;
        data[2] = self.dos_type;
        for track in 0..self.tracks as usize {
            data[4 + track * 4] = self.free_sectors[track];
            data[5 + track * 4] = self.bitmap[track][0];
            data[6 + track * 4] = self.bitmap[track][1];
            data[7 + track * 4] = self.bitmap[track][2];
        }
        data[144..160].copy_from_slice(&self.disk_name);
        data[162..164].copy_from_slice(&self.disk_id);
        data
    }

    pub fn allocate_sector(&mut self, track: u8, sector: u8) -> Result<(), D64Error> {
        if track == 0 || track > self.tracks || sector >= SECTORS_PER_TRACK[(track - 1) as usize] {
            return Err(D64Error::InvalidTrackSector);
        }
        let idx = (track - 1) as usize;
        let byte_idx = (sector / 8) as usize;
        let bit_idx = sector % 8;
        if self.bitmap[idx][byte_idx] & (1 << bit_idx) == 0 {
            return Ok(());
        }
        self.bitmap[idx][byte_idx] &= !(1 << bit_idx);
        self.free_sectors[idx] -= 1;
        Ok(())
    }

    pub fn free_sector(&mut self, track: u8, sector: u8) -> Result<(), D64Error> {
        if track == 0 || track > self.tracks || sector >= SECTORS_PER_TRACK[(track - 1) as usize] {
            return Err(D64Error::InvalidTrackSector);
        }
        let idx = (track - 1) as usize;
        let byte_idx = (sector / 8) as usize;
        let bit_idx = sector % 8;
        if self.bitmap[idx][byte_idx] & (1 << bit_idx) != 0 {
            return Ok(());
        }
        self.bitmap[idx][byte_idx] |= 1 << bit_idx;
        self.free_sectors[idx] += 1;
        Ok(())
    }

    pub fn find_free_sector(&self, track: u8) -> Option<u8> {
        if track == 0 || track > self.tracks {
            return None;
        }
        let idx = (track - 1) as usize;
        for (byte_idx, &byte) in self.bitmap[idx].iter().enumerate() {
            if byte != 0 {
                for bit_idx in 0..8 {
                    if byte & (1 << bit_idx) != 0 {
                        let sector = (byte_idx as u8) * 8 + bit_idx;
                        if sector < SECTORS_PER_TRACK[idx] {
                            return Some(sector);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_free_sectors_count(&self, track: u8) -> Result<u8, D64Error> {
        if track == 0 || track > self.tracks {
            return Err(D64Error::InvalidTrackSector);
        }
        Ok(self.free_sectors[(track - 1) as usize])
    }

    pub fn get_disk_name(&self) -> String {
        petscii_to_ascii(&self.disk_name)
    }

    pub fn get_disk_id(&self) -> String {
        petscii_to_ascii(&self.disk_id)
    }

    pub fn set_disk_name(&mut self, name: &str) {
        let name_bytes = ascii_to_petscii(name);
        self.disk_name[..name_bytes.len()].copy_from_slice(&name_bytes);
        self.disk_name[name_bytes.len()..].fill(0xA0);
    }

    pub fn set_disk_id(&mut self, id: &str) {
        let id_bytes = ascii_to_petscii(id);
        self.disk_id.copy_from_slice(&id_bytes[..2]);
    }
}
