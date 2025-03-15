# nier_extract_dat_rust_dll

/*
#################################################################
# Project Overview
#
# This implementation is a performance-optimized rewrite, 
# focusing on extracting script-based child files more efficiently 
# for use with the NAER randomizer. Unlike the original, it omits 
# Japanese-to-English translation to improve speed and focuses 
# solely on extracting script-based child files.
#
# The functionality mirrors standard extraction of `.dat` files 
# from NieR: Automata, specifically handling child files 
# (`.pak` and `.yax`) within the `.dat` archive.
#
# Originally developed by RaiderB, this Dart implementation of 
# `datExtractor.dart` rewritten in Rust for 
# improved performance. The Rust version reduces 
# conversion time by approximately ~30 seconds when processing 
# ~11,500 `.yax` files into XML on an Intel i9-10900K with an SSD, 
# compared to the Dart implementation.
#
# Disclaimer: This software is provided "as is" without any 
# warranties, express or implied. Use at your own risk.
#################################################################
*/

