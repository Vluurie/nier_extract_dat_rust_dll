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

struct DatHeader {
    id: String,
    file_number: u32,
    file_offsets_offset: u32,
    file_extensions_offset: u32,
    file_names_offset: u32,
    file_sizes_offset: u32,
    hash_map_offset: u32,
}

impl DatHeader {
    fn new(bytes: &mut ByteDataWrapper) -> io::Result<Self> {
        Ok(Self {
            id: bytes.read_string(4)?,
            file_number: bytes.read_u32()?,
            file_offsets_offset: bytes.read_u32()?,
            file_extensions_offset: bytes.read_u32()?,
            file_names_offset: bytes.read_u32()?,
            file_sizes_offset: bytes.read_u32()?,
            hash_map_offset: bytes.read_u32()?,
        })
    }
}

struct ByteDataWrapper {
    data: Vec<u8>,
    position: usize,
}

impl ByteDataWrapper {
    async fn from_file(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(Self { data, position: 0 })
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let value = (&self.data[self.position..]).read_u32::<LittleEndian>()?;
        self.position += 4;
        Ok(value)
    }

    fn read_string(&mut self, length: usize) -> io::Result<String> {
        let bytes = &self.data[self.position..self.position + length];
        self.position += length;
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    fn read_u8_list(&mut self, length: usize) -> io::Result<Vec<u8>> {
        let mut list = Vec::with_capacity(length);
        for _ in 0..length {
            list.push(self.read_u8()?);
        }
        Ok(list)
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        let value = self.data[self.position];
        self.position += 1;
        Ok(value)
    }

    fn set_position(&mut self, position: usize) {
        self.position = position;
    }
}

pub async fn extract_dat_files(
    dat_path: &str,
    extract_dir: &str,
    should_extract_pak_files: bool,
) -> io::Result<Vec<String>> {
    let mut bytes = ByteDataWrapper::from_file(dat_path).await?;
    if bytes.data.is_empty() {
        println!("Warning: Empty DAT file");
        return Ok(vec![]);
    }

    let header = DatHeader::new(&mut bytes)?;
    bytes.set_position(header.file_offsets_offset as usize);
    let file_offsets = (0..header.file_number)
        .map(|_| bytes.read_u32())
        .collect::<io::Result<Vec<_>>>()?;

    bytes.set_position(header.file_sizes_offset as usize);
    let file_sizes = (0..header.file_number)
        .map(|_| bytes.read_u32())
        .collect::<io::Result<Vec<_>>>()?;

    bytes.set_position(header.file_names_offset as usize);
    let name_length = bytes.read_u32()? as usize;
    let file_names = (0..header.file_number)
        .map(|_| {
            let name = bytes.read_string(name_length)?;
            Ok(name.split('\u{0000}').next().unwrap().to_string())
        })
        .collect::<io::Result<Vec<_>>>()?;

    fs::create_dir_all(extract_dir).await?;

    for i in 0..header.file_number as usize {
        bytes.set_position(file_offsets[i] as usize);
        let mut extracted_file = fs::File::create(Path::new(extract_dir).join(&file_names[i])).await?;
        extracted_file.write_all(&bytes.read_u8_list(file_sizes[i] as usize)?).await?;
    }

    let mut file_names_sorted = file_names.clone();
    file_names_sorted.sort_by(|a, b| {
        let a_parts: Vec<&str> = a.split('.').collect();
        let b_parts: Vec<&str> = b.split('.').collect();
        match a_parts[0].to_lowercase().cmp(&b_parts[0].to_lowercase()) {
            std::cmp::Ordering::Equal => a_parts[1].to_lowercase().cmp(&b_parts[1].to_lowercase()),
            other => other,
        }
    });

    let json_metadata = json!({
        "version": 1,
        "files": file_names_sorted,
        "basename": Path::new(dat_path).file_stem().unwrap().to_str().unwrap(),
        "ext": Path::new(dat_path).extension().unwrap().to_str().unwrap(),
    });

    let json_path = Path::new(extract_dir).join("dat_info.json");
    let mut json_file = fs::File::create(json_path).await?;
    json_file.write_all(serde_json::to_string_pretty(&json_metadata)?.as_bytes()).await?;

    if should_extract_pak_files {
        let pak_files: Vec<&String> = file_names_sorted.iter().filter(|file| file.ends_with(".pak")).collect();
        for pak_file in pak_files {
            let pak_path = Path::new(extract_dir).join(pak_file);
            let pak_extract_dir = Path::new(extract_dir).join(PAK_EXTRACT_SUBDIR).join(pak_file);
            extract_pak_files(pak_path.to_str().unwrap(), pak_extract_dir.to_str().unwrap(), true).await?;
        }
    }

    let extracted_files = file_names_sorted
        .iter()
        .map(|file| Path::new(extract_dir).join(file).to_str().unwrap().to_string())
        .collect();

    Ok(extracted_files)
}


#[no_mangle]
pub extern "C" fn extract_dat_files_ffi(dat_path: *const c_char, extract_dir: *const c_char, should_extract_pak_files: c_uint) -> *mut c_char {
    let dat_path = unsafe { CStr::from_ptr(dat_path).to_str().unwrap() };
    let extract_dir = unsafe { CStr::from_ptr(extract_dir).to_str().unwrap() };
    let should_extract_pak_files = should_extract_pak_files != 0;

    let rt = Runtime::new().unwrap();
    match rt.block_on(extract_dat_files(dat_path, extract_dir, should_extract_pak_files)) {
        Ok(files) => {
            let json_files = json!(files).to_string();
            CString::new(json_files).unwrap().into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

