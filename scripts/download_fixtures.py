#!/usr/bin/env python3
"""
Download real SEC XBRL filings from various companies to use as test fixtures.
These will be used for benchmarking and testing the parser.
"""

import os
import time
import urllib.request
from pathlib import Path

# Create fixtures directory
fixtures_dir = Path("fixtures")
fixtures_dir.mkdir(exist_ok=True)

# List of real SEC XBRL filings from various companies
# Format: (company_name, ticker, description, url)
filings = [
    # Apple filings
    ("apple", "AAPL", "10-K 2023 Instance", 
     "https://www.sec.gov/Archives/edgar/data/320193/000032019323000106/aapl-20230930_htm.xml"),
    ("apple", "AAPL", "10-K 2023 Labels", 
     "https://www.sec.gov/Archives/edgar/data/320193/000032019323000106/aapl-20230930_lab.xml"),
    ("apple", "AAPL", "10-K 2023 Calculation", 
     "https://www.sec.gov/Archives/edgar/data/320193/000032019323000106/aapl-20230930_cal.xml"),
    
    # Microsoft filings
    ("microsoft", "MSFT", "10-Q 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/789019/000095017023064280/msft-20230930_htm.xml"),
    ("microsoft", "MSFT", "10-Q 2023 Labels",
     "https://www.sec.gov/Archives/edgar/data/789019/000095017023064280/msft-20230930_lab.xml"),
    ("microsoft", "MSFT", "10-Q 2023 Presentation",
     "https://www.sec.gov/Archives/edgar/data/789019/000095017023064280/msft-20230930_pre.xml"),
    
    # Tesla filings
    ("tesla", "TSLA", "10-K 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/1318605/000162828024002390/tsla-20231231_htm.xml"),
    ("tesla", "TSLA", "10-K 2023 Definition",
     "https://www.sec.gov/Archives/edgar/data/1318605/000162828024002390/tsla-20231231_def.xml"),
    
    # Amazon filings
    ("amazon", "AMZN", "10-K 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/1018724/000101872424000006/amzn-20231231_htm.xml"),
    ("amazon", "AMZN", "10-K 2023 Labels",
     "https://www.sec.gov/Archives/edgar/data/1018724/000101872424000006/amzn-20231231_lab.xml"),
    
    # Google/Alphabet filings
    ("alphabet", "GOOGL", "10-K 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/1652044/000165204424000022/goog-20231231_htm.xml"),
    ("alphabet", "GOOGL", "10-K 2023 Calculation",
     "https://www.sec.gov/Archives/edgar/data/1652044/000165204424000022/goog-20231231_cal.xml"),
    
    # JPMorgan Chase filings
    ("jpmorgan", "JPM", "10-K 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/19617/000001961724000198/jpm-20231231_htm.xml"),
    ("jpmorgan", "JPM", "10-K 2023 Labels",
     "https://www.sec.gov/Archives/edgar/data/19617/000001961724000198/jpm-20231231_lab.xml"),
    
    # Walmart filings
    ("walmart", "WMT", "10-K 2024 Instance",
     "https://www.sec.gov/Archives/edgar/data/104169/000010416924000012/wmt-20240131_htm.xml"),
    ("walmart", "WMT", "10-K 2024 Presentation",
     "https://www.sec.gov/Archives/edgar/data/104169/000010416924000012/wmt-20240131_pre.xml"),
    
    # Johnson & Johnson filings
    ("jnj", "JNJ", "10-K 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/200406/000020040624000016/jnj-20231231_htm.xml"),
    
    # ExxonMobil filings
    ("exxon", "XOM", "10-K 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/34088/000003408824000013/xom-20231231_htm.xml"),
    
    # Berkshire Hathaway filings
    ("berkshire", "BRK", "10-K 2023 Instance",
     "https://www.sec.gov/Archives/edgar/data/1067983/000095017024021825/brka-20231231_htm.xml"),
]

def download_file(url, filepath):
    """Download a file from URL to filepath."""
    try:
        # Add headers to avoid being blocked
        request = urllib.request.Request(
            url,
            headers={
                'User-Agent': 'crabrl-test-fixtures/1.0 (testing@example.com)'
            }
        )
        
        with urllib.request.urlopen(request) as response:
            content = response.read()
            with open(filepath, 'wb') as f:
                f.write(content)
        return True
    except Exception as e:
        print(f"  Error: {e}")
        return False

def main():
    print("Downloading SEC XBRL fixtures from various companies...")
    print("=" * 60)
    
    downloaded = 0
    failed = 0
    
    for company, ticker, description, url in filings:
        # Create company directory
        company_dir = fixtures_dir / company
        company_dir.mkdir(exist_ok=True)
        
        # Generate filename from URL
        filename = url.split('/')[-1]
        filepath = company_dir / filename
        
        print(f"\n[{ticker}] {description}")
        print(f"  URL: {url}")
        print(f"  Saving to: {filepath}")
        
        if filepath.exists():
            print("  ✓ Already exists, skipping")
            continue
        
        if download_file(url, filepath):
            file_size = os.path.getsize(filepath)
            print(f"  ✓ Downloaded ({file_size:,} bytes)")
            downloaded += 1
        else:
            print(f"  ✗ Failed to download")
            failed += 1
        
        # Be polite to SEC servers
        time.sleep(0.5)
    
    print("\n" + "=" * 60)
    print(f"Download complete: {downloaded} downloaded, {failed} failed")
    print(f"Fixtures saved to: {fixtures_dir.absolute()}")
    
    # Show directory structure
    print("\nFixture structure:")
    for company_dir in sorted(fixtures_dir.iterdir()):
        if company_dir.is_dir():
            files = list(company_dir.glob("*.xml"))
            if files:
                print(f"  {company_dir.name}/")
                for f in sorted(files)[:3]:  # Show first 3 files
                    size = os.path.getsize(f)
                    print(f"    - {f.name} ({size:,} bytes)")
                if len(files) > 3:
                    print(f"    ... and {len(files)-3} more files")

if __name__ == "__main__":
    main()