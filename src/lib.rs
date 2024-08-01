/// Modules used in this file
pub mod hash_map;
pub mod jap_to_eng;
pub mod yax_to_xml_convert;
pub mod pak_extract;

use pak_extract::extract_pak_files;
use tokio::runtime::Runtime;

use std::path::Path;
use std::fs::File;
use std::io::{self, Read};
use byteorder::{LittleEndian, ReadBytesExt};
use serde_json::json;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};

const PAK_EXTRACT_SUBDIR: &str = "pakExtracted";

/// Represents the header of a DAT file in NieR: Automata.
/// This struct is used to parse and hold metadata about the contents of a DAT file.
struct DatHeader {
    id: String,                      // Identifier for the DAT file
    file_number: u32,                // Number of files contained in the DAT file
    file_offsets_offset: u32,        // Offset to the file offsets table
    file_extensions_offset: u32,     // Offset to the file extensions table
    file_names_offset: u32,          // Offset to the file names table
    file_sizes_offset: u32,          // Offset to the file sizes table
    hash_map_offset: u32,            // Offset to the hash map table
}

impl DatHeader {
    /// Creates a new `DatHeader` from the given `ByteDataWrapper`.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A mutable reference to a `ByteDataWrapper` containing the DAT file data.
    ///
    /// # Returns
    ///
    /// A new `DatHeader` instance.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Result` error if reading any of the fields fails.
    fn new(bytes: &mut ByteDataWrapper) -> io::Result<Self> {
        Ok(Self {
            id: bytes.read_string(4)?,                      // Read the first 4 bytes as the ID
            file_number: bytes.read_u32()?,                 // Read the next 4 bytes as the number of files
            file_offsets_offset: bytes.read_u32()?,         // Read the offset to the file offsets table
            file_extensions_offset: bytes.read_u32()?,      // Read the offset to the file extensions table
            file_names_offset: bytes.read_u32()?,           // Read the offset to the file names table
            file_sizes_offset: bytes.read_u32()?,           // Read the offset to the file sizes table
            hash_map_offset: bytes.read_u32()?,             // Read the offset to the hash map table
        })
    }
}

/// Wrapper struct for handling byte data.
/// This struct provides utilities for reading various data types from a byte array, which is useful
/// for parsing binary file formats such as DAT files.
struct ByteDataWrapper {
    data: Vec<u8>,    // The raw byte data
    position: usize,  // The current position in the byte data
}

impl ByteDataWrapper {
    /// Creates a `ByteDataWrapper` from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to read.
    ///
    /// # Returns
    ///
    /// A new `ByteDataWrapper` instance containing the file's data.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Result` error if the file cannot be opened or read.
    async fn from_file(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;   // Open the file
        let mut data = Vec::new();          // Create a buffer for the data
        file.read_to_end(&mut data)?;       // Read the file's data into the buffer
        Ok(Self { data, position: 0 })      // Return a new ByteDataWrapper with the data and position set to 0
    }

    /// Reads a `u32` from the current position.
    ///
    /// # Returns
    ///
    /// The read `u32` value.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Result` error if reading the value fails.
    fn read_u32(&mut self) -> io::Result<u32> {
        let value = (&self.data[self.position..]).read_u32::<LittleEndian>()?;  // Read 4 bytes as a u32 in little-endian format
        self.position += 4;  // Move the position forward by 4 bytes
        Ok(value)  // Return the read value
    }

    /// Reads a string of the given length from the current position.
    ///
    /// # Arguments
    ///
    /// * `length` - The length of the string to read.
    ///
    /// # Returns
    ///
    /// The read string.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Result` error if reading the string fails.
    fn read_string(&mut self, length: usize) -> io::Result<String> {
        let bytes = &self.data[self.position..self.position + length];  // Get the bytes for the string
        self.position += length;  // Move the position forward by the length of the string
        Ok(String::from_utf8_lossy(bytes).to_string())  // Convert the bytes to a string and return it
    }

    /// Reads a list of `u8` values of the given length from the current position.
    ///
    /// # Arguments
    ///
    /// * `length` - The number of `u8` values to read.
    ///
    /// # Returns
    ///
    /// A vector of read `u8` values.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Result` error if reading the values fails.
    fn read_u8_list(&mut self, length: usize) -> io::Result<Vec<u8>> {
        let mut list = Vec::with_capacity(length);  // Create a buffer for the list
        for _ in 0..length {  // Read the specified number of bytes
            list.push(self.read_u8()?);  // Read a byte and add it to the list
        }
        Ok(list)
    }

    /// Reads a `u8` from the current position.
    ///
    /// # Returns
    ///
    /// The read `u8` value.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Result` error if reading the value fails.
    fn read_u8(&mut self) -> io::Result<u8> {
        let value = self.data[self.position];  // Get the byte at the current position
        self.position += 1;  // Move the position forward by 1 byte
        Ok(value)  // Return the read value
    }

    /// Sets the current position to the given value.
    ///
    /// # Arguments
    ///
    /// * `position` - The new position value.
    fn set_position(&mut self, position: usize) {
        self.position = position;
    }
}

/// Extracts DAT files from the specified path in the NieR: Automata game and saves them to the given directory.
/// This function also handles the extraction of PAK files if the flag is set.
///
/// # Arguments
///
/// * `dat_path` - The path to the DAT file.
/// * `extract_dir` - The directory to extract the files to.
/// * `should_extract_pak_files` - Flag indicating whether to extract PAK files.
///
/// # Returns
///
/// A result containing a vector of the extracted file paths or an error.
///
/// # Errors
///
/// This function returns an `io::Result` error if any file operation fails.
pub async fn extract_dat_files(
    dat_path: &str,
    extract_dir: &str,
    should_extract_pak_files: bool,
) -> io::Result<Vec<String>> {
    let mut bytes = ByteDataWrapper::from_file(dat_path).await?;  // Read the DAT file into a ByteDataWrapper
    if bytes.data.is_empty() {  // Check if the DAT file is empty
        println!("Warning: Empty DAT file");  // Print a warning if the file is empty
        return Ok(vec![]); 
    }

    let header = DatHeader::new(&mut bytes)?;  // Parse the DAT header
    bytes.set_position(header.file_offsets_offset as usize);  // Set the position to the file offsets table
    let file_offsets = (0..header.file_number)  // Read the file offsets
        .map(|_| bytes.read_u32())
        .collect::<io::Result<Vec<_>>>()?;

    bytes.set_position(header.file_sizes_offset as usize);  // Set the position to the file sizes table
    let file_sizes = (0..header.file_number)  // Read the file sizes
        .map(|_| bytes.read_u32())
        .collect::<io::Result<Vec<_>>>()?;

    bytes.set_position(header.file_names_offset as usize);  // Set the position to the file names table
    let name_length = bytes.read_u32()? as usize;  // Read the length of the file names
    let file_names = (0..header.file_number)  // Read the file names
        .map(|_| {
            let name = bytes.read_string(name_length)?;  // Read a file name
            Ok(name.split('\u{0000}').next().unwrap().to_string())  // Split the name at the null terminator and return the first part
        })
        .collect::<io::Result<Vec<_>>>()?;

    fs::create_dir_all(extract_dir).await?;  // Create the extraction directory

    for i in 0..header.file_number as usize {  // Extract each file
        bytes.set_position(file_offsets[i] as usize);  // Set the position to the file's offset
        let mut extracted_file = fs::File::create(Path::new(extract_dir).join(&file_names[i])).await?;  // Create the output file
        extracted_file.write_all(&bytes.read_u8_list(file_sizes[i] as usize)?).await?;  // Write the file's data
    }

    let mut file_names_sorted = file_names.clone();  // Create a sorted copy of the file names
    file_names_sorted.sort_by(|a, b| {  // Sort the file names
        let a_parts: Vec<&str> = a.split('.').collect();  // Split the first name into parts
        let b_parts: Vec<&str> = b.split('.').collect();  // Split the second name into parts
        match a_parts[0].to_lowercase().cmp(&b_parts[0].to_lowercase()) {  // Compare the first parts (ignoring case)
            std::cmp::Ordering::Equal => a_parts[1].to_lowercase().cmp(&b_parts[1].to_lowercase()),  // If equal, compare the second parts
            other => other,
        }
    });

    let json_metadata = json!({  // Create JSON metadata for the extracted files
        "version": 1,
        "files": file_names_sorted,
        "basename": Path::new(dat_path).file_stem().unwrap().to_str().unwrap(),
        "ext": Path::new(dat_path).extension().unwrap().to_str().unwrap(),
    });

    let json_path = Path::new(extract_dir).join("dat_info.json");  // Create the path for the metadata file
    let mut json_file = fs::File::create(json_path).await?;  // Create the metadata file
    json_file.write_all(serde_json::to_string_pretty(&json_metadata)?.as_bytes()).await?;  // Write the metadata to the file

    if should_extract_pak_files {  // Check if PAK files should be extracted
        let pak_files: Vec<&String> = file_names_sorted.iter().filter(|file| file.ends_with(".pak")).collect();  // Get the PAK files
        for pak_file in pak_files {  // Extract each PAK file
            let pak_path = Path::new(extract_dir).join(pak_file);  // Create the path for the PAK file
            let pak_extract_dir = Path::new(extract_dir).join(PAK_EXTRACT_SUBDIR).join(pak_file);  // Create the extraction directory for the PAK file
            extract_pak_files(pak_path.to_str().unwrap(), pak_extract_dir.to_str().unwrap(), true).await?;  // Extract the PAK file
        }
    }

    let extracted_files = file_names_sorted  // Create a list of the extracted file paths
        .iter()
        .map(|file| Path::new(extract_dir).join(file).to_str().unwrap().to_string())
        .collect();

    Ok(extracted_files)
}

/// FFI function to extract DAT files from NieR: Automata and return the extracted file paths as a JSON string.
/// This function is intended to be called from Dart using Dart FFI.
///
/// # Arguments
///
/// * `dat_path` - The path to the DAT file.
/// * `extract_dir` - The directory to extract the files to.
/// * `should_extract_pak_files` - Flag indicating whether to extract PAK files (0 or 1).
///
/// # Returns
///
/// A pointer to a C string containing the JSON representation of the extracted file paths, or null on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers from Dart.
#[no_mangle]
pub extern "C" fn extract_dat_files_ffi(dat_path: *const c_char, extract_dir: *const c_char, should_extract_pak_files: c_uint) -> *mut c_char {
    let dat_path = unsafe { CStr::from_ptr(dat_path).to_str().unwrap() };  // Convert the DAT file path from a C string to a Rust string
    let extract_dir = unsafe { CStr::from_ptr(extract_dir).to_str().unwrap() };  // Convert the extraction directory path from a C string to a Rust string
    let should_extract_pak_files = should_extract_pak_files != 0;  // Convert the flag to a boolean

    let rt = Runtime::new().unwrap();
    match rt.block_on(extract_dat_files(dat_path, extract_dir, should_extract_pak_files)) {  // Extract the DAT files
        Ok(files) => {
            let json_files = json!(files).to_string();  // Convert the list of extracted files to a JSON string
            CString::new(json_files).unwrap().into_raw()  // Convert the JSON string to a C string and return it
        }
        Err(_) => std::ptr::null_mut(), 
    }
}
