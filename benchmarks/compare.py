#!/usr/bin/env python3
"""
Compare crabrl performance with Arelle
"""

import subprocess
import time
import sys
from pathlib import Path

def run_crabrl(filepath):
    """Run crabrl and measure time"""
    cmd = ["../target/release/crabrl", "parse", filepath]
    start = time.perf_counter()
    result = subprocess.run(cmd, capture_output=True, text=True)
    elapsed = (time.perf_counter() - start) * 1000
    
    if result.returncode == 0:
        # Parse output for fact count
        facts = 0
        for line in result.stdout.split('\n'):
            if 'Facts:' in line:
                facts = int(line.split(':')[1].strip())
                break
        return elapsed, facts
    return None, 0

def run_arelle(filepath):
    """Run Arelle and measure time"""
    try:
        cmd = ["python3", "-m", "arelle.CntlrCmdLine", 
               "--file", filepath, "--skipDTS", "--logLevel", "ERROR"]
        start = time.perf_counter()
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        elapsed = (time.perf_counter() - start) * 1000
        
        if result.returncode == 0:
            return elapsed
        return None
    except:
        return None

def main():
    if len(sys.argv) < 2:
        print("Usage: compare.py <xbrl-file>")
        sys.exit(1)
    
    filepath = sys.argv[1]
    print(f"Comparing performance on: {filepath}\n")
    
    # Run crabrl
    crabrl_time, facts = run_crabrl(filepath)
    if crabrl_time:
        print(f"crabrl: {crabrl_time:.1f}ms ({facts} facts)")
    else:
        print("crabrl: Failed")
    
    # Run Arelle
    arelle_time = run_arelle(filepath)
    if arelle_time:
        print(f"Arelle: {arelle_time:.1f}ms")
    else:
        print("Arelle: Failed or not installed")
    
    # Calculate speedup
    if crabrl_time and arelle_time:
        speedup = arelle_time / crabrl_time
        print(f"\nSpeedup: {speedup:.1f}x faster")

if __name__ == "__main__":
    main()