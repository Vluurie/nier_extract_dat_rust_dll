# yax_to_xml_rust

/*!
This Rust DLL provides functionality to convert YAX files to XML files. The implementation is inspired by RaiderB's Dart `yaxToXml` script
but rewritten in Rust for faster computation time (total 25 seconds faster on an i9 10900k then with dart).

RaiderB's dart file: https://github.com/ArthurHeitmann/nier_cli/blob/master/lib/fileTypeUtils/yax/yaxToXml.dart

This module is intended to be called primarily from Dart using FFI (Foreign Function Interface).

Modules:
- `hash_map`: Contains the `HASH_TO_STRING_MAP` for mapping hash values to strings.
- `jap_to_eng`: Contains the `JAP_TO_ENG` map for translating Japanese strings to English.

Dependencies:
- `quick_xml`: Used for creating and writing XML events.
- `encoding_rs`: Used for decoding SHIFT_JIS encoded strings.

The main components include:
- `YaxNode`: A struct representing a node in the YAX structure.
- Functions to read and convert YAX data to XML.
- The external C function `yax_file_to_xml_file` that get's called mainly from Dart FFI for converting YAX files to XML files.

Usage:
1. `yax_file_to_xml_file` function can be called from Dart code via FFI to convert a YAX file to an XML file.

Simply add the Input YAX and the Output for the XML file as parameter.

2. The core logic involves reading nodes from the YAX file, mapping their tags and text, and writing these nodes to an XML structure.

This structure then get's mapped like in F-SERVO for editing or for getting the structured elements for file manipulations of the element's values like NAER does.

*/