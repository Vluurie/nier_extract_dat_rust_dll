# nier_extract_dat_rust_dll

This Rust DLL provides functionality to extract the .dat files of nier automata, also child files (only pak and yax files from the dat file!!). The implementation is by RaiderB's Dart `datExtractor.dart` witch was also rewritten from python mainly written by `xxk-i`, but rewritten now in Rust for faster computation time (total 30 seconds faster on converting ~11500 .yax files to xml on an i9 10900k CPU with SSD then with Dart).

RaiderB's dart file: https://github.com/ArthurHeitmann/nier_cli/blob/master/lib/fileTypeUtils/yax/yaxToXml.dart
xxk-i creator of Dat:  https://github.com/xxk-i/DATrepacker

This module is intended to be called primarily from Dart using FFI (Foreign Function Interface) but can also be integrated into other Rust projects.

Usage:
1. `yax_file_to_xml_file` function can be called from Dart code via FFI to convert a YAX file to an XML file.

Simply add the Input YAX and the Output for the XML file as parameter.

2. `extract_dat_files_ffi` function can be called from Dart code via FFI to extract the pak files out of the dat file or directly extract the pak content and convert it to xml.

Simply add the Input dat and the Output for the extracted folder as parameter, add `should_extract_pak_files` bool if you want to do it.

3. `extract_pak_files_ffi` function can be called from Dart code via FFI to extract the pak files out of the dat file or directly extract the pak content and convert it to xml.

Simply add the Input pak and the Output for the extracted folder as parameter, add `yax_to_xml` bool if you want to do it.

Dart usage example:

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
