use flate2::read::ZlibDecoder;
use serde_json::json;
use std::ffi::{CStr, CString};
use std::fs::{create_dir_all, File};
use std::io::{self, Read, Write};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

use crate::yax_to_xml_convert::convert_yax_to_xml;


#[derive(Debug)]
struct HeaderEntry {
    r#type: u32,           
    uncompressed_size: u32, 
    offset: u32,           
}

impl HeaderEntry {

    fn new(bytes: &mut ByteDataWrapper) -> Self {
        let r#type = bytes.read_u32();           
        let uncompressed_size = bytes.read_u32();
        let offset = bytes.read_u32();           
        HeaderEntry {
            r#type,
            uncompressed_size,
            offset,
        }
    }
}


struct ByteDataWrapper {
    data: Vec<u8>,  
    position: usize, 
}

impl ByteDataWrapper {

    fn from_file(file_path: &str) -> io::Result<Self> {
        let mut file = File::open(file_path)?; 
        let mut data = Vec::new();             
        file.read_to_end(&mut data)?;          
        Ok(ByteDataWrapper { data, position: 0 })
    }


    fn read_u32(&mut self) -> u32 {
        let result = u32::from_le_bytes(self.data[self.position..self.position + 4].try_into().unwrap());
        self.position += 4; 
        result
    }


    fn read_u8_list(&mut self, size: usize) -> Vec<u8> {
        let result = self.data[self.position..self.position + size].to_vec(); 
        self.position += size; 
        result
    }
}

pub async fn extract_pak_yax(
    meta: &HeaderEntry,
    size: usize,
    bytes: &mut ByteDataWrapper,
    extract_dir: &Path,
    index: usize,
) -> io::Result<()> {
    bytes.position = meta.offset as usize; 
    let is_compressed = meta.uncompressed_size > size as u32;  
    let read_size = if is_compressed {
        bytes.read_u32() as usize 
    } else {
        size - ((4 - (meta.uncompressed_size % 4)) % 4) as usize 
    };

    let mut extracted_file = File::create(extract_dir.join(format!("{}.yax", index)))?;  
    let mut file_bytes = bytes.read_u8_list(read_size); 
    if is_compressed {
        let mut decoder = ZlibDecoder::new(&file_bytes[..]); 
        let mut decompressed_bytes = Vec::new(); 
        decoder.read_to_end(&mut decompressed_bytes)?; 
        file_bytes = decompressed_bytes; 
    }
    extracted_file.write_all(&file_bytes)?; 
    Ok(())
}


pub async fn extract_pak_files(
    pak_path: &str,
    extract_dir: &str,
    yax_to_xml: bool,
) -> io::Result<Vec<String>> {
    let mut bytes = ByteDataWrapper::from_file(pak_path)?;  

    bytes.position = 8; 
    let first_offset = bytes.read_u32();
    let file_count = (first_offset - 4) / 12; 

    bytes.position = 0; 
    let mut header_entries = Vec::with_capacity(file_count as usize); 
    for _ in 0..file_count { 
        header_entries.push(HeaderEntry::new(&mut bytes));
    }

    let mut file_sizes = Vec::with_capacity(file_count as usize); 
    for i in 0..file_count as usize {
        let size = if i == file_count as usize - 1 {
            bytes.data.len() as u32 - header_entries[i].offset 
        } else {
            header_entries[i + 1].offset - header_entries[i].offset 
        };
        file_sizes.push(size);
    }

    create_dir_all(extract_dir)?;
    let extract_dir_path = Path::new(extract_dir);
    for (i, meta) in header_entries.iter().enumerate() {  
        extract_pak_yax(meta, file_sizes[i] as usize, &mut bytes, extract_dir_path, i).await?;
    }

    let meta = json!({ 
        "files": header_entries.iter().enumerate().map(|(i, meta)| json!({
            "name": format!("{}.yax", i),
            "type": meta.r#type,
        })).collect::<Vec<_>>()
    });

    let pak_info_path = Path::new(extract_dir).join("pakInfo.json");  
    let mut pak_info_file = File::create(pak_info_path)?; 
    pak_info_file.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?; 

    if yax_to_xml { 
        let tasks: Vec<_> = (0..file_count as usize).map(|i| { 
            let extract_dir_path = extract_dir_path.to_path_buf();
            tokio::task::spawn(async move { 
                let yax_path = extract_dir_path.join(format!("{}.yax", i)); 
                let xml_path = yax_path.with_extension("xml"); 
                convert_yax_to_xml(yax_path.to_str().unwrap(), xml_path.to_str().unwrap()); 
            })
        }).collect();
        for task in tasks { 
            task.await.unwrap();
        }
    }

    Ok((0..file_count as usize).map(|i| extract_dir_path.join(format!("{}.yax", i)).to_str().unwrap().to_string()).collect()) 
}


#[no_mangle]
pub extern "C" fn extract_pak_files_ffi(
    pak_path: *const c_char,
    extract_dir: *const c_char,
    yax_to_xml: bool,
) -> *mut c_char {
    let pak_path = unsafe { CStr::from_ptr(pak_path) }.to_str().unwrap(); 
    let extract_dir = unsafe { CStr::from_ptr(extract_dir) }.to_str().unwrap();  

    let rt = tokio::runtime::Runtime::new().unwrap(); 
    let result = rt.block_on(internal_extract_pak_files(pak_path, extract_dir, yax_to_xml)); 

    match result {
        Ok(files) => {
            let files_json = json!(files).to_string(); 
            let c_str = CString::new(files_json).unwrap(); 
            c_str.into_raw() 
        }
        Err(_) => ptr::null_mut(),  
    }
}


async fn internal_extract_pak_files(
    pak_path: &str,
    extract_dir: &str,
    yax_to_xml: bool,
) -> io::Result<Vec<String>> {
    let mut bytes = ByteDataWrapper::from_file(pak_path)?; 

    bytes.position = 8; 
    let first_offset = bytes.read_u32(); 
    let file_count = (first_offset - 4) / 12;  

    bytes.position = 0;
    let mut header_entries = Vec::with_capacity(file_count as usize);  
    for _ in 0..file_count { 
        header_entries.push(HeaderEntry::new(&mut bytes));
    }

    let mut file_sizes = Vec::with_capacity(file_count as usize); 
    for i in 0..file_count as usize { 
        let size = if i == file_count as usize - 1 {
            bytes.data.len() as u32 - header_entries[i].offset 
        } else {
            header_entries[i + 1].offset - header_entries[i].offset 
        };
        file_sizes.push(size);
    }

    create_dir_all(extract_dir)?; 

    let extract_dir_path = Path::new(extract_dir);
    for (i, meta) in header_entries.iter().enumerate() { 
        extract_pak_yax(meta, file_sizes[i] as usize, &mut bytes, extract_dir_path, i).await?;
    }

    let meta = json!({ 
        "files": header_entries.iter().enumerate().map(|(i, meta)| json!({
            "name": format!("{}.yax", i),
            "type": meta.r#type,
        })).collect::<Vec<_>>()
    });

    let pak_info_path = Path::new(extract_dir).join("pakInfo.json"); 
    let mut pak_info_file = File::create(pak_info_path)?;
    pak_info_file.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?;  

    if yax_to_xml { 
        let tasks: Vec<_> = (0..file_count as usize).map(|i| { 
            let extract_dir_path = extract_dir_path.to_path_buf();
            tokio::task::spawn(async move { 
                let yax_path = extract_dir_path.join(format!("{}.yax", i)); 
                let xml_path = yax_path.with_extension("xml"); 
                convert_yax_to_xml(yax_path.to_str().unwrap(), xml_path.to_str().unwrap());  
            })
        }).collect();
        for task in tasks { 
            task.await.unwrap();
        }
    }

    Ok((0..file_count as usize).map(|i| extract_dir_path.join(format!("{}.yax", i)).to_str().unwrap().to_string()).collect()) 
}
