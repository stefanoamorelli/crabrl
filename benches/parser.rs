use crabrl::Parser;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::Path;

fn parse_sample_sec_file(c: &mut Criterion) {
    let parser = Parser::new();
    let sample_file = Path::new("fixtures/sample-sec.xml");
    
    if sample_file.exists() {
        c.bench_function("parse_sample_sec", |b| {
            b.iter(|| parser.parse_file(black_box(&sample_file)));
        });
    } else {
        // If no fixtures exist, use a minimal inline XBRL for benchmarking
        let minimal_xbrl = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbrl xmlns="http://www.xbrl.org/2003/instance">
  <context id="ctx1">
    <entity>
      <identifier scheme="http://www.sec.gov/CIK">0000000000</identifier>
    </entity>
    <period>
      <instant>2023-12-31</instant>
    </period>
  </context>
  <unit id="usd">
    <measure>iso4217:USD</measure>
  </unit>
</xbrl>"#;
        
        c.bench_function("parse_minimal", |b| {
            b.iter(|| parser.parse_str(black_box(minimal_xbrl)));
        });
    }
}

criterion_group!(benches, parse_sample_sec_file);
criterion_main!(benches);
