# nier_extract_dat_rust_dll

This Rust DLL provides functionality to extract the .dat files of nier automata, also child files (only pak and yax files from the dat file!!). The implementation is by RaiderB's Dart `datExtractor.dart` witch was also rewritten from python mainly written by `xxk-i`, but rewritten now in Rust for faster computation time (total 30 seconds faster on converting ~11500 .yax files to xml on an i9 10900k CPU with SSD then with Dart).

RaiderB's dart file: https://github.com/ArthurHeitmann/nier_cli/blob/master/lib/fileTypeUtils/yax/yaxToXml.dart

xxk-i god creator of dat repacker and other crazy stuff:  https://github.com/xxk-i/DATrepacker

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
import 'dart:convert';
import 'package:NAER/naer_utils/exception_handler.dart'; // <--- from NAER
import 'package:ffi/ffi.dart';

/// ╔════════════════════════════════════════════════════╗
/// ║          Dynamic Library Loader                   ║
/// ╚════════════════════════════════════════════════════╝
/// Loads the dynamic library `extract_dat_files.dll` for handling file extractions.
/// 
/// 📄 **Library**: `extract_dat_files.dll`
/// ⚠️ **Note**: Ensure this file is located in the same directory as your Dart executable.
final DynamicLibrary dylib = DynamicLibrary.open('extract_dat_files.dll');

/// ╔════════════════════════════════════════════════════╗
/// ║     Rust Function Signature (FFI Typedef)         ║
/// ╚════════════════════════════════════════════════════╝
/// Represents the function signature used to interact with the Rust library.
/// 
/// ### Parameters:
/// - `datPath` (*Pointer<Utf8>*): Path to the `.dat` file (UTF-8 encoded).
/// - `extractDir` (*Pointer<Utf8>*): Path to the output directory (UTF-8 encoded).
/// - `shouldExtractPakFiles` (*Uint8*): Flag indicating whether to extract `.pak` files too.
typedef ExtractDatFilesFFIFunc = Pointer<Utf8> Function(
    Pointer<Utf8> datPath, Pointer<Utf8> extractDir, Uint8 shouldExtractPakFiles);

typedef ExtractDatFilesFFI = Pointer<Utf8> Function(
    Pointer<Utf8> datPath, Pointer<Utf8> extractDir, int shouldExtractPakFiles);

final ExtractDatFilesFFI extractDatFilesFFI = dylib
    .lookup<NativeFunction<ExtractDatFilesFFIFunc>>('extract_dat_files_ffi')
    .asFunction();

/// ╔════════════════════════════════════════════════════╗
/// ║            File Extraction Utility                ║
/// ╚════════════════════════════════════════════════════╝
/// Extracts .dat files from NieR (Note: No translation is added to the output extracted
/// as mostly used for automatic extract - modify - repack
///
/// ### Parameters:
/// - `datFilePath` (*String*): Path to the input `.dat` file.
/// - `extractDirPath` (*String*): Path to the directory for the extracted files.
/// - `shouldExtractPakFiles` (*bool*): Whether to extract `.pak` files.

/// ### Returns:
/// - A `Future<List<String>>` containing paths of the extracted files.
Future<List<String>> extractDatFiles(
    final String datFilePath, final String extractDirPath,
    {required final bool shouldExtractPakFiles}) async {

// alloc
  final Pointer<Utf8> datFilePathPtr = datFilePath.toNativeUtf8();
  final Pointer<Utf8> extractDirPathPtr = extractDirPath.toNativeUtf8();

  try {
    final Pointer<Utf8> resultPtr = extractDatFilesFFI(
        datFilePathPtr, extractDirPathPtr, shouldExtractPakFiles ? 1 : 0);

    if (resultPtr == nullptr) {
      throw Exception('Error extracting DAT files.');
    }

    final String resultStr = resultPtr.toDartString();

    final List<dynamic> files = jsonDecode(resultStr);
    return files.cast<String>();
  } catch (error, stackTrace) {
// use any handler, here is used from NAER
    ExceptionHandler().handle(
      error,
      stackTrace,
      extraMessage: '''
        Error occurred while extracting DAT files.
        DAT File Path: $datFilePath
        Extract Directory Path: $extractDirPath
        Should Extract PAK Files: $shouldExtractPakFiles
      ''',
    );
    rethrow;
  } finally {
// memFree
    malloc.free(datFilePathPtr);
    malloc.free(extractDirPathPtr);
  }
}


```
