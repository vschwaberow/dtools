# dtools

A Rust-based command-line utility for manipulating Commodore 64 D64 disk images, offering a Rust library to use in other projects.

## Features

- Create and format D64 images (35 or 40 tracks)
- List, extract, and insert files
- Read and write individual sectors
- Manage Block Availability Map (BAM)
- PETSCII/ASCII conversion

## Building

Requires Rust 1.54 or later.

```bash
cargo build --release
```

The binary will be in `target/release/dtools`.

## Usage

### Create a new D64 image

```bash
dtools create -f newdisk.d64 -t 35
```

### Format a D64 image

```bash
dtools format -f mydisk.d64 -n "MY DISK" -i "01"
```

### List files on a D64 image

```bash
dtools list -f mydisk.d64
```

### Insert a file

```bash
dtools insert -f mydisk.d64 -n "MYFILE" -i /path/to/input/file
```

### Extract a file

```bash
dtools extract -f mydisk.d64 -n "MYFILE" -o /path/to/output/file
```

### Read a sector

```bash
dtools read -f mydisk.d64 -t 18 -s 0
```

### Write to a sector

```bash
dtools write -f mydisk.d64 -t 18 -s 0 -d "0123456789ABCDEF"
```

### Show BAM

```bash
dtools show-bam -f mydisk.d64
```

### Find a free sector

```bash
dtools find-free-sector -f mydisk.d64
```

## Library Usage

`dtools` can also be used as a library in other Rust projects:

```rust
use d64lib::{D64, D64Error};

fn main() -> Result {
    let mut d64 = D64::from_file("mydisk.d64")?;
    let files = d64.list_files()?;
    println!("Files on disk: {:?}", files);
    Ok(())
}
```

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)