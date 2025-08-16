# crabrl 🦀

[![Crates.io](https://img.shields.io/crates/v/crabrl.svg)](https://crates.io/crates/crabrl)
[![CI Status](https://github.com/stefanoamorelli/crabrl/workflows/CI/badge.svg)](https://github.com/stefanoamorelli/crabrl/actions)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/crabrl.svg)](https://crates.io/crates/crabrl)
[![docs.rs](https://docs.rs/crabrl/badge.svg)](https://docs.rs/crabrl)

Lightning-fast XBRL parser that's **50-150x faster** than traditional parsers, built for speed and accuracy when processing [SEC EDGAR](https://www.sec.gov/edgar) filings.

## Technical Architecture

crabrl is built on Rust's zero-cost abstractions and modern parsing techniques. While established parsers like [Arelle](https://arelle.org/) provide comprehensive XBRL specification support and extensive validation capabilities, crabrl focuses on high-performance parsing for scenarios where speed is critical.

### Implementation Details

| Optimization | Impact | Technology |
|-------------|---------|------------|
| **Zero-copy parsing** | -90% memory allocs | [`quick-xml`](https://github.com/tafia/quick-xml) with string slicing |
| **No garbage collection** | Predictable latency | Rust's ownership model |
| **Faster hashmaps** | 2x lookup speed | [`ahash`](https://github.com/tkaitchuck/aHash) instead of default hasher |
| **Compact strings** | -50% memory for small strings | [`compact_str`](https://github.com/ParkMyCar/compact_str) |
| **Parallelization** | 4-8x on multicore | [`rayon`](https://github.com/rayon-rs/rayon) work-stealing |
| **Memory mapping** | Zero-copy file I/O | [`memmap2`](https://github.com/RazrFalcon/memmap2-rs) |
| **Better allocator** | -25% allocation time | [`mimalloc`](https://github.com/microsoft/mimalloc) |

**Benchmark results:** 100,000 XBRL facts parsed in 56ms (crabrl) vs 2,672ms (Arelle) on identical hardware.

## XBRL Support Status

| Feature | Description | Status |
|---------|-------------|---------|
| **XBRL 2.1 Instance** | Parse facts, contexts, units from `.xml` files | ✅ Stable |
| **SEC Validation** | EDGAR-specific rules and checks | ✅ Stable |
| **Calculation Linkbase** | Validate arithmetic relationships | ✅ Stable |
| **Presentation Linkbase** | Extract display hierarchy | 🚧 Beta |
| **Label Linkbase** | Human-readable concept names | 🚧 Beta |
| **Definition Linkbase** | Dimensional relationships | 📋 Planned |
| **Formula Linkbase** | Business rules validation | 📋 Planned |
| **Inline XBRL (iXBRL)** | HTML-embedded XBRL | 📋 Planned |

## Installation

### From crates.io
```bash
cargo install crabrl
```

### From Source
```bash
git clone https://github.com/stefanoamorelli/crabrl
cd crabrl
cargo build --release --features cli
```

### As Library Dependency
```toml
[dependencies]
crabrl = "0.1.0"
```

## Usage

### CLI

```bash
# Parse and display summary
crabrl parse filing.xml

# Parse with statistics (timing and throughput)
crabrl parse filing.xml --stats

# Validate with generic rules
crabrl validate filing.xml

# Validate with SEC EDGAR rules
crabrl validate filing.xml --profile sec-edgar

# Validate with strict mode (warnings as errors)
crabrl validate filing.xml --strict

# Benchmark performance
crabrl bench filing.xml --iterations 100
```

### Library

#### Basic Usage

```rust
use crabrl::Parser;

// Parse XBRL document
let parser = Parser::new();
let doc = parser.parse_file("filing.xml")?;

// Access parsed data
println!("Facts: {}", doc.facts.len());
println!("Contexts: {}", doc.contexts.len());
println!("Units: {}", doc.units.len());
```

#### Parse from Different Sources

```rust
// From file path
let doc = parser.parse_file("filing.xml")?;

// From bytes
let xml_bytes = std::fs::read("filing.xml")?;
let doc = parser.parse_bytes(&xml_bytes)?;
```

#### Validation

```rust
use crabrl::{Parser, Validator};

let parser = Parser::new();
let doc = parser.parse_file("filing.xml")?;

// Generic validation
let validator = Validator::new();
let result = validator.validate(&doc)?;

if result.is_valid {
    println!("Document is valid!");
} else {
    for error in &result.errors {
        eprintln!("Error: {}", error);
    }
}

// SEC EDGAR validation (stricter rules)
let sec_validator = Validator::sec_edgar();
let sec_result = sec_validator.validate(&doc)?;
```

## Performance Measurements

Performance comparison with [Arelle](https://arelle.org/) v2.17.4 (Python-based XBRL processor with full specification support):

### Synthetic Dataset Benchmarks

| File Size | Facts | crabrl | Arelle | Ratio |
|-----------|------:|-------:|-------:|------:|
| Tiny      | 10    | 1.1 ms | 164 ms | 150x |
| Small     | 100   | 1.4 ms | 168 ms | 119x |
| Medium    | 1K    | 1.7 ms | 184 ms | 108x |
| Large     | 10K   | 6.1 ms | 351 ms | 58x  |
| Huge      | 100K  | 57 ms  | 2,672 ms | 47x |

### SEC Filing Parse Times

| Company | Filing Type | File Size | Facts | Parse Time | Throughput |
|---------|-------------|-----------|-------|------------|------------|
| Apple | [10-K 2023](https://www.sec.gov/Archives/edgar/data/320193/000032019323000106/aapl-20230930_htm.xml) | 1.4 MB | 1,075 | 2.1 ms | 516K facts/sec |
| Microsoft | [10-Q 2023](https://www.sec.gov/Archives/edgar/data/789019/000095017023064280/msft-20230930_htm.xml) | 2.8 MB | 2,341 | 4.3 ms | 544K facts/sec |
| Tesla | [10-K 2023](https://www.sec.gov/Archives/edgar/data/1318605/000162828024002390/tsla-20231231_htm.xml) | 3.1 MB | 3,122 | 5.8 ms | 538K facts/sec |

### Run Your Own Benchmarks

```bash
# Quick benchmark with Criterion
cargo bench

# Compare against Arelle
cd benchmarks && python compare_performance.py

# Test on real SEC filings
python scripts/download_fixtures.py  # Download Apple, MSFT, Tesla, etc.
cargo run --release --bin crabrl -- bench fixtures/apple/aapl-20230930_htm.xml
```

## Resources & Links

### XBRL Standards
- [XBRL International](https://www.xbrl.org/) - Official XBRL specifications
- [XBRL 2.1 Specification](https://www.xbrl.org/Specification/XBRL-2.1/REC-2003-12-31/XBRL-2.1-REC-2003-12-31+corrected-errata-2013-02-20.html) - Core standard we implement
- [SEC EDGAR](https://www.sec.gov/edgar/searchedgar/companysearch) - Search real company filings
- [EDGAR Filer Manual](https://www.sec.gov/info/edgar/forms/edgform.pdf) - SEC filing requirements

### Dependencies We Use

| Crate | Purpose | Why We Chose It |
|-------|---------|-----------------|
| [`quick-xml`](https://github.com/tafia/quick-xml) | XML parsing | Zero-copy, fastest XML parser in Rust |
| [`ahash`](https://github.com/tkaitchuck/aHash) | HashMap hashing | 2x faster than default hasher |
| [`compact_str`](https://github.com/ParkMyCar/compact_str) | String storage | Small string optimization |
| [`rayon`](https://github.com/rayon-rs/rayon) | Parallelization | Work-stealing for automatic load balancing |
| [`mimalloc`](https://github.com/microsoft/mimalloc) | Memory allocator | Microsoft's high-performance allocator |
| [`criterion`](https://github.com/bheisler/criterion.rs) | Benchmarking | Statistical benchmarking with graphs |

### Alternative XBRL Parsers
- [Arelle](https://arelle.org/) - Complete XBRL processor with validation, formulas, and rendering (Python)
- [python-xbrl](https://github.com/manusimidt/py-xbrl) - Lightweight Python parser
- [xbrl-parser](https://www.npmjs.com/package/xbrl-parser) - JavaScript/Node.js
- [XBRL4j](https://github.com/br-data/xbrl-parser) - Java implementation

## License ⚖️

This open-source project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0). This means:

- You can use, modify, and distribute this software
- If you modify and distribute it, you must release your changes under AGPL-3.0
- If you run a modified version on a server, you must provide the source code to users
- See the [LICENSE](LICENSE) file for full details

For commercial licensing options or other licensing inquiries, please contact stefano@amorelli.tech.

© 2025 Stefano Amorelli – Released under the GNU Affero General Public License v3.0. Enjoy! 🎉