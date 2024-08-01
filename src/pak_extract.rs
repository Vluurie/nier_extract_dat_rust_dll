use flate2::read::ZlibDecoder;
use serde_json::json;
use std::ffi::{CStr, CString};
use std::fs::{create_dir_all, File};
use std::io::{self, Read, Write};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

use crate::yax_to_xml_convert::convert_yax_file_to_xml;

/// Represents an entry in the PAK file header for NieR: Automata.
/// This struct is used to hold metadata about each file contained in the PAK file.
#[derive(Debug)]
struct HeaderEntry {
    r#type: u32,             // Type of the file
    uncompressed_size: u32,  // Uncompressed size of the file
    offset: u32,             // Offset to the file data
}

impl HeaderEntry {
    /// Creates a new `HeaderEntry` from the given `ByteDataWrapper`.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A mutable reference to a `ByteDataWrapper` containing the PAK file data.
    ///
    /// # Returns
    ///
    /// A new `HeaderEntry` instance.
    fn new(bytes: &mut ByteDataWrapper) -> Self {
        let r#type = bytes.read_u32();            // Read the type of the file
        let uncompressed_size = bytes.read_u32(); // Read the uncompressed size of the file
        let offset = bytes.read_u32();            // Read the offset to the file data
        HeaderEntry {
            r#type,
            uncompressed_size,
            offset,
        }
    }
}

/// Wrapper struct for handling byte data.
/// This struct provides utilities for reading various data types from a byte array, which is useful
/// for parsing binary file formats such as PAK files.
struct ByteDataWrapper {
    data: Vec<u8>,    // The raw byte data
    position: usize,  // The current position in the byte data
}

impl ByteDataWrapper {
    /// Creates a `ByteDataWrapper` from a file path.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the file to read.
    ///
    /// # Returns
    ///
    /// A new `ByteDataWrapper` instance containing the file's data.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Result` error if the file cannot be opened or read.
    fn from_file(file_path: &str) -> io::Result<Self> {
        let mut file = File::open(file_path)?;   // Open the file
        let mut data = Vec::new();               // Create a buffer for the data
        file.read_to_end(&mut data)?;            // Read the file's data into the buffer
        Ok(ByteDataWrapper { data, position: 0 }) // Return a new ByteDataWrapper with the data and position set to 0
    }

    /// Reads a `u32` from the current position.
    ///
    /// # Returns
    ///
    /// The read `u32` value.
    fn read_u32(&mut self) -> u32 {
        let result = u32::from_le_bytes(self.data[self.position..self.position + 4].try_into().unwrap()); // Read 4 bytes as a u32 in little-endian format
        self.position += 4;  // Move the position forward by 4 bytes
        result  // Return the read value
    }

    /// Reads a list of `u8` values of the given size from the current position.
    ///
    /// # Arguments
    ///
    /// * `size` - The number of `u8` values to read.
    ///
    /// # Returns
    ///
    /// A vector of read `u8` values.
    fn read_u8_list(&mut self, size: usize) -> Vec<u8> {
        let result = self.data[self.position..self.position + size].to_vec();  // Get the bytes for the list
        self.position += size;  // Move the position forward by the size of the list
        result  // Return the list
    }
}

/// Extracts a YAX file from the given PAK file metadata and saves it to the specified directory.
///
/// # Arguments
///
/// * `meta` - Metadata for the YAX file to be extracted.
/// * `size` - Size of the file to be extracted.
/// * `bytes` - A mutable reference to a `ByteDataWrapper` containing the PAK file data.
/// * `extract_dir` - Directory where the extracted YAX file will be saved.
/// * `index` - Index of the file in the PAK file.
///
/// # Returns
///
/// An `io::Result` indicating success or failure.
pub async fn extract_pak_yax(
    meta: &HeaderEntry,
    size: usize,
    bytes: &mut ByteDataWrapper,
    extract_dir: &Path,
    index: usize,
) -> io::Result<()> {
    bytes.position = meta.offset as usize;  // Set the position to the file's offset
    let is_compressed = meta.uncompressed_size > size as u32;  // Check if the file is compressed
    let read_size = if is_compressed {
        bytes.read_u32() as usize  // If compressed, read the compressed size
    } else {
        size - ((4 - (meta.uncompressed_size % 4)) % 4) as usize  // If not compressed, calculate the read size
    };

    let mut extracted_file = File::create(extract_dir.join(format!("{}.yax", index)))?;  // Create the output file
    let mut file_bytes = bytes.read_u8_list(read_size);  // Read the file's data
    if is_compressed {
        let mut decoder = ZlibDecoder::new(&file_bytes[..]);  // Create a Zlib decoder
        let mut decompressed_bytes = Vec::new();  // Create a buffer for the decompressed data
        decoder.read_to_end(&mut decompressed_bytes)?;  // Decompress the data
        file_bytes = decompressed_bytes;  // Use the decompressed data
    }
    extracted_file.write_all(&file_bytes)?;  // Write the file's data
    Ok(())
}

/// Extracts files from a PAK file and saves them to the specified directory.
/// This function also optionally converts extracted YAX files to XML format.
///
/// # Arguments
///
/// * `pak_path` - The path to the PAK file.
/// * `extract_dir` - The directory to extract the files to.
/// * `yax_to_xml` - Flag indicating whether to convert YAX files to XML.
///
/// # Returns
///
/// A result containing a vector of the extracted file paths or an error.
///
/// # Errors
///
/// This function returns an `io::Result` error if any file operation fails.
pub async fn extract_pak_files(
    pak_path: &str,
    extract_dir: &str,
    yax_to_xml: bool,
) -> io::Result<Vec<String>> {
    let mut bytes = ByteDataWrapper::from_file(pak_path)?;  // Read the PAK file into a ByteDataWrapper

    bytes.position = 8;  // Set the position to the first offset
    let first_offset = bytes.read_u32();  // Read the first offset
    let file_count = (first_offset - 4) / 12;  // Calculate the number of files

    bytes.position = 0;  // Reset the position
    let mut header_entries = Vec::with_capacity(file_count as usize);  // Create a buffer for the header entries
    for _ in 0..file_count {  // Read the header entries
        header_entries.push(HeaderEntry::new(&mut bytes));
    }

    let mut file_sizes = Vec::with_capacity(file_count as usize);  // Create a buffer for the file sizes
    for i in 0..file_count as usize {  // Calculate the file sizes
        let size = if i == file_count as usize - 1 {
            bytes.data.len() as u32 - header_entries[i].offset  // If the last file, calculate the size from the end of the data
        } else {
            header_entries[i + 1].offset - header_entries[i].offset  // Otherwise, calculate the size from the next file's offset
        };
        file_sizes.push(size);
    }

    create_dir_all(extract_dir)?;  // Create the extraction directory

    let extract_dir_path = Path::new(extract_dir);  // Create a path to the extraction directory
    for (i, meta) in header_entries.iter().enumerate() {  // Extract each file
        extract_pak_yax(meta, file_sizes[i] as usize, &mut bytes, extract_dir_path, i).await?;
    }

    let meta = json!({  // Create JSON metadata for the extracted files
        "files": header_entries.iter().enumerate().map(|(i, meta)| json!({
            "name": format!("{}.yax", i),
            "type": meta.r#type,
        })).collect::<Vec<_>>()
    });

    let pak_info_path = Path::new(extract_dir).join("pakInfo.json");  // Create the path for the metadata file
    let mut pak_info_file = File::create(pak_info_path)?;  // Create the metadata file
    pak_info_file.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?;  // Write the metadata to the file

    if yax_to_xml {  // Check if YAX files should be converted to XML
        let tasks: Vec<_> = (0..file_count as usize).map(|i| {  // Create a task for each file
            let extract_dir_path = extract_dir_path.to_path_buf();
            tokio::task::spawn(async move {  // Spawn an async task
                let yax_path = extract_dir_path.join(format!("{}.yax", i));  // Create the path to the YAX file
                let xml_path = yax_path.with_extension("xml");  // Create the path to the XML file
                convert_yax_file_to_xml(yax_path.to_str().unwrap(), xml_path.to_str().unwrap());  // Convert the YAX file to XML
            })
        }).collect();
        for task in tasks {  // Wait for all tasks to complete
            task.await.unwrap();
        }
    }

    Ok((0..file_count as usize).map(|i| extract_dir_path.join(format!("{}.yax", i)).to_str().unwrap().to_string()).collect())  // Return the list of extracted files
}

/// FFI function to extract PAK files from NieR: Automata and return the extracted file paths as a JSON string.
/// This function is intended to be called from Dart using Dart FFI.
///
/// # Arguments
///
/// * `pak_path` - The path to the PAK file.
/// * `extract_dir` - The directory to extract the files to.
/// * `yax_to_xml` - Flag indicating whether to convert YAX files to XML.
///
/// # Returns
///
/// A pointer to a C string containing the JSON representation of the extracted file paths, or null on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers from Dart.
#[no_mangle]
pub extern "C" fn extract_pak_files_ffi(
    pak_path: *const c_char,
    extract_dir: *const c_char,
    yax_to_xml: bool,
) -> *mut c_char {
    let pak_path = unsafe { CStr::from_ptr(pak_path) }.to_str().unwrap();  // Convert the PAK file path from a C string to a Rust string
    let extract_dir = unsafe { CStr::from_ptr(extract_dir) }.to_str().unwrap();  // Convert the extraction directory path from a C string to a Rust string

    let rt = tokio::runtime::Runtime::new().unwrap();  // Create a new Tokio runtime
    let result = rt.block_on(internal_extract_pak_files(pak_path, extract_dir, yax_to_xml));  // Extract the PAK files

    match result {
        Ok(files) => {
            let files_json = json!(files).to_string();  // Convert the list of extracted files to a JSON string
            let c_str = CString::new(files_json).unwrap();  // Convert the JSON string to a C string
            c_str.into_raw()  // Return the C string
        }
        Err(_) => ptr::null_mut(),  // Return null if an error occurs
    }
}

/// Internal function to extract files from a PAK file and save them to the specified directory.
/// This function is called by `extract_pak_files_ffi`.
///
/// # Arguments
///
/// * `pak_path` - The path to the PAK file.
/// * `extract_dir` - The directory to extract the files to.
/// * `yax_to_xml` - Flag indicating whether to convert YAX files to XML.
///
/// # Returns
///
/// A result containing a vector of the extracted file paths or an error.
///
/// # Errors
///
/// This function returns an `io::Result` error if any file operation fails.
async fn internal_extract_pak_files(
    pak_path: &str,
    extract_dir: &str,
    yax_to_xml: bool,
) -> io::Result<Vec<String>> {
    let mut bytes = ByteDataWrapper::from_file(pak_path)?;  // Read the PAK file into a ByteDataWrapper

    bytes.position = 8;  // Set the position to the first offset
    let first_offset = bytes.read_u32();  // Read the first offset
    let file_count = (first_offset - 4) / 12;  // Calculate the number of files

    bytes.position = 0;  // Reset the position
    let mut header_entries = Vec::with_capacity(file_count as usize);  // Create a buffer for the header entries
    for _ in 0..file_count {  // Read the header entries
        header_entries.push(HeaderEntry::new(&mut bytes));
    }

    let mut file_sizes = Vec::with_capacity(file_count as usize);  // Create a buffer for the file sizes
    for i in 0..file_count as usize {  // Calculate the file sizes
        let size = if i == file_count as usize - 1 {
            bytes.data.len() as u32 - header_entries[i].offset  // If the last file, calculate the size from the end of the data
        } else {
            header_entries[i + 1].offset - header_entries[i].offset  // Otherwise, calculate the size from the next file's offset
        };
        file_sizes.push(size);
    }

    create_dir_all(extract_dir)?;  // Create the extraction directory

    let extract_dir_path = Path::new(extract_dir);  // Create a path to the extraction directory
    for (i, meta) in header_entries.iter().enumerate() {  // Extract each file
        extract_pak_yax(meta, file_sizes[i] as usize, &mut bytes, extract_dir_path, i).await?;
    }

    let meta = json!({  // Create JSON metadata for the extracted files
        "files": header_entries.iter().enumerate().map(|(i, meta)| json!({
            "name": format!("{}.yax", i),
            "type": meta.r#type,
        })).collect::<Vec<_>>()
    });

    let pak_info_path = Path::new(extract_dir).join("pakInfo.json");  // Create the path for the metadata file
    let mut pak_info_file = File::create(pak_info_path)?;  // Create the metadata file
    pak_info_file.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?;  // Write the metadata to the file

    if yax_to_xml {  // Check if YAX files should be converted to XML
        let tasks: Vec<_> = (0..file_count as usize).map(|i| {  // Create a task for each file
            let extract_dir_path = extract_dir_path.to_path_buf();
            tokio::task::spawn(async move {  // Spawn an async task
                let yax_path = extract_dir_path.join(format!("{}.yax", i));  // Create the path to the YAX file
                let xml_path = yax_path.with_extension("xml");  // Create the path to the XML file
                convert_yax_file_to_xml(yax_path.to_str().unwrap(), xml_path.to_str().unwrap());  // Convert the YAX file to XML
            })
        }).collect();
        for task in tasks {  // Wait for all tasks to complete
            task.await.unwrap();
        }
    }

    Ok((0..file_count as usize).map(|i| extract_dir_path.join(format!("{}.yax", i)).to_str().unwrap().to_string()).collect())  // Return the list of extracted files
}
