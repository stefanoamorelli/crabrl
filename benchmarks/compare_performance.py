#!/usr/bin/env python3
"""Compare performance between crabrl and Arelle."""

import os
import sys
import time
import subprocess
import json
import statistics
from pathlib import Path
from tabulate import tabulate
import matplotlib.pyplot as plt

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

def benchmark_arelle(file_path, runs=3):
    """Benchmark Arelle parsing performance."""
    times = []
    
    for _ in range(runs):
        start = time.perf_counter()
        
        # Run Arelle in subprocess to isolate memory
        result = subprocess.run([
            sys.executable, "-c",
            f"""
import sys
sys.path.insert(0, 'venv/lib/python{sys.version_info.major}.{sys.version_info.minor}/site-packages')
from arelle import Cntlr
from arelle import ModelManager

# Suppress Arelle output
import logging
logging.getLogger("arelle").setLevel(logging.ERROR)

controller = Cntlr.Cntlr(logFileName=None)
controller.webCache.workOffline = True
modelManager = ModelManager.initialize(controller)

# Load and parse the XBRL file
modelXbrl = modelManager.load('{file_path}')
if modelXbrl:
    facts = len(modelXbrl.facts)
    contexts = len(modelXbrl.contexts)
    units = len(modelXbrl.units)
    print(f"{{facts}},{{contexts}},{{units}}")
    modelXbrl.close()
"""
        ], capture_output=True, text=True, cwd=Path(__file__).parent)
        
        end = time.perf_counter()
        
        if result.returncode == 0 and result.stdout:
            times.append(end - start)
            if len(times) == 1:  # Print counts on first run
                parts = result.stdout.strip().split(',')
                if len(parts) == 3:
                    print(f"    Arelle found: {parts[0]} facts, {parts[1]} contexts, {parts[2]} units")
        else:
            print(f"    Arelle error: {result.stderr}")
    
    if times:
        return {
            'mean': statistics.mean(times),
            'median': statistics.median(times),
            'stdev': statistics.stdev(times) if len(times) > 1 else 0,
            'min': min(times),
            'max': max(times),
            'runs': len(times)
        }
    return None

def benchmark_crabrl(file_path, runs=3):
    """Benchmark crabrl parsing performance."""
    times = []
    
    # Build the benchmark binary if needed
    subprocess.run(["cargo", "build", "--release", "--example", "benchmark_single"], 
                  capture_output=True, cwd=Path(__file__).parent.parent)
    
    for _ in range(runs):
        start = time.perf_counter()
        
        result = subprocess.run([
            "../target/release/examples/benchmark_single",
            file_path
        ], capture_output=True, text=True, cwd=Path(__file__).parent)
        
        end = time.perf_counter()
        
        if result.returncode == 0:
            times.append(end - start)
            if len(times) == 1 and result.stdout:  # Print counts on first run
                print(f"    crabrl output: {result.stdout.strip()}")
        else:
            print(f"    crabrl error: {result.stderr}")
    
    if times:
        return {
            'mean': statistics.mean(times),
            'median': statistics.median(times),
            'stdev': statistics.stdev(times) if len(times) > 1 else 0,
            'min': min(times),
            'max': max(times),
            'runs': len(times)
        }
    return None

def main():
    """Run comparative benchmarks."""
    print("=" * 80)
    print("XBRL Parser Performance Comparison: crabrl vs Arelle")
    print("=" * 80)
    
    test_files = [
        ("Tiny (10 facts)", "../test_data/test_tiny.xbrl"),
        ("Small (100 facts)", "../test_data/test_small.xbrl"),
        ("Medium (1K facts)", "../test_data/test_medium.xbrl"),
        ("Large (10K facts)", "../test_data/test_large.xbrl"),
        ("Huge (100K facts)", "../test_data/test_huge.xbrl"),
    ]
    
    results = []
    
    for name, file_path in test_files:
        if not Path(file_path).exists():
            print(f"Skipping {name}: file not found")
            continue
        
        file_size_mb = Path(file_path).stat().st_size / (1024 * 1024)
        print(f"\nBenchmarking {name} ({file_size_mb:.2f} MB)...")
        
        # Benchmark Arelle
        print("  Running Arelle...")
        arelle_stats = benchmark_arelle(file_path, runs=5)
        
        # Benchmark crabrl
        print("  Running crabrl...")
        crabrl_stats = benchmark_crabrl(file_path, runs=5)
        
        if arelle_stats and crabrl_stats:
            speedup = arelle_stats['median'] / crabrl_stats['median']
            results.append({
                'File': name,
                'Size (MB)': f"{file_size_mb:.2f}",
                'Arelle (ms)': f"{arelle_stats['median']*1000:.1f}",
                'crabrl (ms)': f"{crabrl_stats['median']*1000:.1f}",
                'Speedup': f"{speedup:.1f}x",
                'arelle_raw': arelle_stats['median'],
                'crabrl_raw': crabrl_stats['median'],
            })
    
    # Print results table
    print("\n" + "=" * 80)
    print("RESULTS SUMMARY")
    print("=" * 80)
    
    if results:
        table_data = [{k: v for k, v in r.items() if not k.endswith('_raw')} for r in results]
        print(tabulate(table_data, headers="keys", tablefmt="grid"))
        
        # Calculate average speedup
        speedups = [r['arelle_raw'] / r['crabrl_raw'] for r in results]
        avg_speedup = statistics.mean(speedups)
        print(f"\nAverage speedup: {avg_speedup:.1f}x faster than Arelle")
        
        # Create performance chart
        create_performance_chart(results)
    else:
        print("No results to display")

def create_performance_chart(results):
    """Create a performance comparison chart."""
    labels = [r['File'].split('(')[0].strip() for r in results]
    arelle_times = [r['arelle_raw'] * 1000 for r in results]
    crabrl_times = [r['crabrl_raw'] * 1000 for r in results]
    
    x = range(len(labels))
    width = 0.35
    
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))
    
    # Bar chart
    ax1.bar([i - width/2 for i in x], arelle_times, width, label='Arelle', color='#FF6B6B')
    ax1.bar([i + width/2 for i in x], crabrl_times, width, label='crabrl', color='#4ECDC4')
    ax1.set_xlabel('File Size')
    ax1.set_ylabel('Time (ms)')
    ax1.set_title('Parsing Time Comparison')
    ax1.set_xticks(x)
    ax1.set_xticklabels(labels, rotation=45)
    ax1.legend()
    ax1.grid(True, alpha=0.3)
    
    # Speedup chart
    speedups = [a/c for a, c in zip(arelle_times, crabrl_times)]
    ax2.bar(x, speedups, color='#95E77E')
    ax2.set_xlabel('File Size')
    ax2.set_ylabel('Speedup Factor')
    ax2.set_title('crabrl Speedup over Arelle')
    ax2.set_xticks(x)
    ax2.set_xticklabels(labels, rotation=45)
    ax2.grid(True, alpha=0.3)
    
    # Add value labels on bars
    for i, v in enumerate(speedups):
        ax2.text(i, v + 0.5, f'{v:.1f}x', ha='center', va='bottom')
    
    plt.tight_layout()
    plt.savefig('benchmark_results.png', dpi=150)
    print(f"\nPerformance chart saved to: benchmarks/benchmark_results.png")

if __name__ == "__main__":
    main()