# nier_yax_to_xml_rust

This Rust DLL provides functionality to convert YAX files to XML files. The implementation is by RaiderB's Dart `yaxToXml` script
but rewritten in Rust for faster computation time (total 25 seconds faster on converting ~11500 .yax files to xml on an i9 10900k CPU with SSD then with Dart).

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


Dart usage:

```dart
import 'dart:ffi';
import 'package:ffi/ffi.dart';

/// Loads the dynamic library for the Windows platform.
/// The DLL `yax_to_xml.dll` should be in the same directory as the Dart executable.
final DynamicLibrary dylib = DynamicLibrary.open('yax_to_xml.dll');

/// Type definition for the `yax_file_to_xml_file` function in the Rust library.
/// This represents the function signature in C: `void yax_file_to_xml_file(const char* yaxFilePath, const char* xmlFilePath)`.
typedef YaxFileToXmlFileFunc = Void Function(
    Pointer<Utf8> yaxFilePath, Pointer<Utf8> xmlFilePath);

/// Dart function type corresponding to the Rust `yax_file_to_xml_file` function.
typedef YaxFileToXmlFile = void Function(
    Pointer<Utf8> yaxFilePath, Pointer<Utf8> xmlFilePath);

/// Looks up the `yax_file_to_xml_file` function in the dynamic library and assigns it to a Dart variable.
/// The function can then be called from Dart.
final YaxFileToXmlFile yaxFileToXmlFile = dylib
    .lookup<NativeFunction<YaxFileToXmlFileFunc>>('yax_file_to_xml_file')
    .asFunction();

/// Converts a YAX file to an XML file by calling the Rust function.
/// - `yaxFilePath`: The path to the input YAX file.
/// - `xmlFilePath`: The path to the output XML file.
///
/// This function performs the following steps:
/// 1. Converts the Dart strings `yaxFilePath` and `xmlFilePath` to C-style UTF-8 strings.
/// 2. Calls the `yax_file_to_xml_file` function from the Rust library.
/// 3. Frees the allocated memory for the C-style strings.
Future<void> convertYaxFileToXmlFile(
    String yaxFilePath, String xmlFilePath) async {
  // Convert Dart strings to C-style UTF-8 strings.
  final Pointer<Utf8> yaxFilePathPtr = yaxFilePath.toNativeUtf8();
  final Pointer<Utf8> xmlFilePathPtr = xmlFilePath.toNativeUtf8();

  // Call the Rust function.
  yaxFileToXmlFile(yaxFilePathPtr, xmlFilePathPtr);

  // Free the allocated memory.
  malloc.free(yaxFilePathPtr);
  malloc.free(xmlFilePathPtr);
}
```
