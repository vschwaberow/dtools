// SPDX-License-Identifier: MIT
// Project: dtools
// File: src/main.rs
// Author: Volker Schwaberow <volker@schwaberow.de>
// Copyright (c) 2024 Volker Schwaberow

use std::{fs::File, io::Write};

use clap::{Parser, Subcommand};
use d64lib::{D64Error, D64};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Read {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        track: u8,
        #[arg(short, long)]
        sector: u8,
    },
    Write {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        track: u8,
        #[arg(short, long)]
        sector: u8,
        #[arg(short, long)]
        data: String,
    },

    ShowBam {
        #[arg(short, long)]
        file: String,
    },

    FindFreeSector {
        #[arg(short, long)]
        file: String,
    },

    AllocateSector {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        track: u8,
        #[arg(short, long)]
        sector: u8,
    },
    FreeSector {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        track: u8,
        #[arg(short, long)]
        sector: u8,
    },

    SetDiskName {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        name: String,
    },
    SetDiskId {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        id: String,
    },

    List {
        #[arg(short, long)]
        file: String,
    },
    Extract {
        #[arg(short, long)]
        file: String,
        #[arg(short = 'n', long)]
        filename: String,
        #[arg(short, long)]
        output: String,
    },
    Create {
        #[arg(short, long)]
        file: String,
        #[arg(short, long, default_value = "35")]
        tracks: u8,
    },
    Format {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        id: String,
    },
    TraceFile {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        name: String,
    },
}

fn main() -> Result<(), D64Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Read {
            file,
            track,
            sector,
        } => {
            let d64 = D64::from_file(file)?;
            let data = d64.read_sector(*track, *sector)?;
            println!("Sector data: {:?}", data);
        }
        Commands::Write {
            file,
            track,
            sector,
            data,
        } => {
            let mut d64 = D64::from_file(file)?;
            let bytes = hex::decode(data).map_err(|_| D64Error::InvalidTrackSector)?;
            d64.write_sector(*track, *sector, &bytes)?;
            d64.save_to_file(file)?;
            println!("Sector written successfully");
        }

        Commands::FindFreeSector { file } => {
            let d64 = D64::from_file(file)?;
            match d64.find_free_sector() {
                Ok((track, sector)) => {
                    println!("Found free sector: track {}, sector {}", track, sector)
                }
                Err(D64Error::DiskFull) => println!("No free sectors available"),
                Err(e) => return Err(e),
            }
        }
        Commands::TraceFile { file, name } => {
            let d64 = D64::from_file(file)?;
            match d64.trace_file(name) {
                Ok(sectors) => {
                    println!("File '{}' is located in the following sectors:", name);
                    for (i, (track, sector)) in sectors.iter().enumerate() {
                        println!("  Block {}: Track {}, Sector {}", i + 1, track, sector);
                    }
                    println!("Total blocks: {}", sectors.len());
                }
                Err(D64Error::FileNotFound) => {
                    println!("File '{}' not found on the disk", name)
                }
                Err(e) => return Err(e),
            }
        }
        Commands::SetDiskName { file, name } => {
            let mut d64 = D64::from_file(file)?;
            let mut bam = d64.read_bam()?;
            bam.set_disk_name(name);
            d64.write_bam(&bam)?;
            d64.save_to_file(file)?;
            println!("Disk name set to: {}", name);
        }
        Commands::AllocateSector {
            file,
            track,
            sector,
        } => {
            let mut d64 = D64::from_file(file)?;
            d64.allocate_sector(*track, *sector)?;
            d64.save_to_file(file)?;
            println!("Allocated sector {} on track {}", sector, track);
        }
        Commands::FreeSector {
            file,
            track,
            sector,
        } => {
            let mut d64 = D64::from_file(file)?;
            d64.free_sector(*track, *sector)?;
            d64.save_to_file(file)?;
            println!("Freed sector {} on track {}", sector, track);
        }

        Commands::SetDiskId { file, id } => {
            let mut d64 = D64::from_file(file)?;
            let mut bam = d64.read_bam()?;
            bam.set_disk_id(id);
            d64.write_bam(&bam)?;
            d64.save_to_file(file)?;
            println!("Disk ID set to: {}", id);
        }

        Commands::ShowBam { file } => {
            let d64 = D64::from_file(file)?;
            let bam = d64.read_bam()?;
            println!("Disk Name: {}", bam.get_disk_name());
            println!("Disk ID: {}", bam.get_disk_id());
            println!("Free sectors per track:");
            for track in 1..=d64.tracks {
                println!(
                    "Track {}: {} free sectors",
                    track,
                    bam.get_free_sectors_count(track)?
                );
            }
        }

        Commands::Create { file, tracks } => {
            let d64 = D64::new(*tracks)?;
            d64.save_to_file(file)?;
            println!("Created new D64 file '{}' with {} tracks", file, tracks);
        }
        Commands::Format { file, name, id } => {
            let mut d64 = D64::from_file(file)?;
            d64.format(name, id)?;
            d64.save_to_file(file)?;
            println!(
                "Formatted D64 file '{}' with name '{}' and ID '{}'",
                file, name, id
            );
        }
        Commands::List { file } => {
            let d64 = D64::from_file(file)?;
            match d64.list_files() {
                Ok(files) => {
                    println!("Files in {}:", file);
                    for (i, file) in files.iter().enumerate() {
                        println!("{:2}. {}", i + 1, file);
                    }
                }
                Err(e) => println!("Error listing files: {}", e),
            }
        }
        Commands::Extract {
            file,
            filename,
            output,
        } => {
            let d64 = D64::from_file(file)?;
            let content = d64.extract_file(filename)?;
            let mut output_file = File::create(output)?;
            output_file.write_all(&content)?;
            println!("File '{}' extracted to '{}'", filename, output);
        }
    }

    Ok(())
}
