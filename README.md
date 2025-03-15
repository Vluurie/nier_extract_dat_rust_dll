
This is a **faster** rewrite of `datExtractor.dart`, built to speed up extracting **script-related files** from **NieR: Automata** `.dat` archives. It’s optimized for the **NAER randomizer**, skipping Japanese-to-English translation to keep things quick and only extracting **`.pak` and `.yax` files**.

## Why This Exists
This is based on **RaiderB’s Dart version**, but now with a **ugly Rust rewrite** to make it faster.

### Speed Boost:
- **~30 seconds faster** when converting **~11,500 `.yax` files to XML**.  
- Tested on an **Intel i9-10900K (SSD)**.

## Disclaimer
No promises, no guarantees—just use it if you want.  



