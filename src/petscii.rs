// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/petscii.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

//! Utilities for converting between PETSCII and ASCII character sets

/// Converts PETSCII encoded bytes to ASCII string
///
/// PETSCII is the character encoding used by Commodore computers.
/// This function performs a simple mapping from PETSCII to ASCII.
///
/// # Arguments
///
/// * `petscii` - A slice of bytes in PETSCII encoding
///
/// # Returns
///
/// An ASCII string with the converted text
pub fn petscii_to_ascii(petscii: &[u8]) -> String {
    let mut s = String::with_capacity(petscii.len());
    for &c in petscii {
        // Handle PETSCII to ASCII conversion with more precise mapping
        let ch = match c {
            // Special cases and control codes
            0xA0 => ' ', // PETSCII shifted space
            
            // Letters (uppercase in PETSCII, 0xC1-0xDA maps to ASCII A-Z)
            0xC1..=0xDA => (c - 0x80) as char,
            
            // Unshifted letters and symbols (0x20-0x5F)
            0x20..=0x5F => c as char,
            
            // Fallback for any other character
            _ => '?'
        };
        s.push(ch);
    }
    s
}

/// Converts ASCII string to PETSCII encoded bytes
///
/// # Arguments
///
/// * `ascii` - An ASCII string to convert
///
/// # Returns
///
/// A vector of bytes in PETSCII encoding
pub fn ascii_to_petscii(ascii: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(ascii.len());
    for c in ascii.chars() {
        // More comprehensive ASCII to PETSCII conversion
        let byte = match c {
            // Space and symbols
            ' ' => 0x20,
            '!' ..= '/' => c as u8,
            
            // Numbers
            '0' ..= '9' => c as u8,
            
            // Symbols
            ':' ..= '@' => c as u8,
            
            // Uppercase letters (ASCII A-Z maps to PETSCII 0x41-0x5A)
            'A' ..= 'Z' => c as u8,
            
            // Lowercase letters (convert to uppercase in PETSCII)
            'a' ..= 'z' => (c as u8) - 32,
            
            // Fallback
            _ => 0x3F // Question mark
        };
        v.push(byte);
    }
    v
}