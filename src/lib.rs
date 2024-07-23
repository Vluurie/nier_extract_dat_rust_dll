mod hash_map;
mod jap_to_eng;

use hash_map::HASH_TO_STRING_MAP;
use jap_to_eng::JAP_TO_ENG;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use encoding_rs::SHIFT_JIS;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn hash_to_string_map(hash: u32) -> Option<&'static str> {
    HASH_TO_STRING_MAP.get(&hash).copied()
}

fn jap_to_eng(japanese: &str) -> Option<&'static str> {
    JAP_TO_ENG.get(japanese).copied()
}

#[derive(Debug)]
struct YaxNode {
    indentation: u8,
    tag_name_hash: u32,
    string_offset: u32,
    tag_name: String,
    text: Option<String>,
    children: Vec<YaxNode>,
}

impl YaxNode {
    fn from_bytes(bytes: &mut impl Read) -> Self {
        let mut buffer = [0; 1];
        bytes.read_exact(&mut buffer).unwrap();
        let indentation = buffer[0];

        let mut buffer = [0; 4];
        bytes.read_exact(&mut buffer).unwrap();
        let tag_name_hash = u32::from_le_bytes(buffer);

        let mut buffer = [0; 4];
        bytes.read_exact(&mut buffer).unwrap();
        let string_offset = u32::from_le_bytes(buffer);

        let tag_name = hash_to_string_map(tag_name_hash).unwrap_or("UNKNOWN").to_string();

        YaxNode {
            indentation,
            tag_name_hash,
            string_offset,
            tag_name,
            text: None,
            children: Vec::new(),
        }
    }

    fn to_xml(&self, include_annotations: bool) -> BytesStart {
        let mut element = BytesStart::borrowed(self.tag_name.as_bytes(), self.tag_name.len());

        if let Some(text) = &self.text {
            if include_annotations && text.starts_with("0x") && text.len() > 2 {
                if let Ok(hash) = u32::from_str_radix(&text[2..], 16) {
                    if hash != 0 {
                        if let Some(hash_lookup) = hash_to_string_map(hash) {
                            element.push_attribute(("str", hash_lookup));
                        }
                    }
                }
            } else if include_annotations && !text.is_ascii() {
                if let Some(translation) = jap_to_eng(text) {
                    element.push_attribute(("eng", translation));
                }
            }
        }

        if include_annotations && self.tag_name == "UNKNOWN" {
            element.push_attribute(("id", format!("0x{:x}", self.tag_name_hash).as_str()));
        }

        element
    }

    fn to_xml_events(&self, writer: &mut Writer<&mut Vec<u8>>, include_annotations: bool) {
        writer.write_event(Event::Start(self.to_xml(include_annotations))).unwrap();

        if let Some(text) = &self.text {
            let mut text = text.clone();
            if text.contains("&quot;") {
                text = text.replace("&quot;", "\"\"");
            }

            writer.write_event(Event::Text(BytesText::from_plain_str(&text))).unwrap();
        }

        for child in &self.children {
            child.to_xml_events(writer, include_annotations);
        }

        writer.write_event(Event::End(BytesEnd::borrowed(self.tag_name.as_bytes()))).unwrap();
    }
}

fn read_string_zero_terminated(bytes: &mut impl Read) -> Option<String> {
    let mut buffer = Vec::new();
    let mut byte = [0; 1];
    while let Ok(_) = bytes.read_exact(&mut byte) {
        if byte[0] == 0 {
            break;
        }
        buffer.push(byte[0]);
    }
    if buffer.is_empty() {
        None
    } else {
        let (decoded_str, _, _) = SHIFT_JIS.decode(&buffer);
        Some(decoded_str.into_owned())
    }
}

fn yax_to_xml<R: Read + Seek>(mut bytes: R, include_annotations: bool) -> Vec<u8> {
    let mut buffer = [0; 4];
    bytes.read_exact(&mut buffer).unwrap();
    let node_count = u32::from_le_bytes(buffer);

    let mut nodes = Vec::new();
    for _ in 0..node_count {
        nodes.push(YaxNode::from_bytes(&mut bytes));
    }

    let mut strings = HashMap::new();
    while let Ok(position) = bytes.stream_position() {
        if let Some(string) = read_string_zero_terminated(&mut bytes) {
            strings.insert(position as u32, string);
        } else {
            break;
        }
    }

    for node in &mut nodes {
        node.text = strings.get(&node.string_offset).cloned();
    }

    let mut root_nodes = Vec::new();
    for node in nodes {
        if node.indentation == 0 {
            root_nodes.push(node);
        } else {
            let parent_indent = node.indentation - 1;
            let mut parent = root_nodes.last_mut().unwrap();
            while parent.indentation != parent_indent {
                parent = parent.children.last_mut().unwrap();
            }
            parent.children.push(node);
        }
    }

    let mut buffer = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buffer, b'\t', 1);

    writer.write_event(Event::Start(BytesStart::borrowed(b"root", 4))).unwrap();
    for root_node in root_nodes {
        root_node.to_xml_events(&mut writer, include_annotations);
    }
    writer.write_event(Event::End(BytesEnd::borrowed(b"root"))).unwrap();

    buffer
}

#[no_mangle]
pub extern "C" fn yax_file_to_xml_file(yax_file_path: *const c_char, xml_file_path: *const c_char) {
    let yax_file_path = unsafe { CStr::from_ptr(yax_file_path).to_str().unwrap() };
    let xml_file_path = unsafe { CStr::from_ptr(xml_file_path).to_str().unwrap() };

    let yax_file = File::open(yax_file_path).expect("Failed to open YAX file");
    let xml_bytes = yax_to_xml(BufReader::new(yax_file), true);

    let mut xml_file = BufWriter::new(File::create(xml_file_path).expect("Failed to create XML file"));
    xml_file.write_all(b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n").unwrap();
    xml_file.write_all(&xml_bytes).unwrap();
}