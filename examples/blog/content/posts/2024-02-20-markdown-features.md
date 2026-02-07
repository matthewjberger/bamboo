+++
title = "Understanding Weissman Scores"
tags = ["compression", "algorithms", "metrics"]
categories = ["engineering"]
excerpt = "A deep dive into how compression algorithm performance is actually measured."
+++

{{% note type="info" title="Background" %}}
This post assumes basic familiarity with compression algorithms. If you're new to the topic, check out our [Middle-Out Compression Explained](/posts/building-static-sites/) post first.
{{% /note %}}

When we first demoed Pied Piper's compression, everyone asked about the Weissman Score. Here's what it actually means and why it matters.

## What is a Weissman Score?

The Weissman Score is a theoretical metric for comparing compression algorithms. It accounts for both:

1. **Compression ratio** - How small the output is compared to input
2. **Speed** - How fast the algorithm runs

The formula looks like this:

```
W = α * (r / r̄) * (log(T̄) / log(T))
```

Where:
- `r` = compression ratio achieved
- `r̄` = reference compression ratio
- `T` = time to compress
- `T̄` = reference time
- `α` = scaling constant

## Why Both Matter

A naive approach might achieve 99% compression but take hours. A fast algorithm might run in milliseconds but barely compress anything. The Weissman Score captures the tradeoff.

## Our Results

| Algorithm | Ratio | Time | Weissman |
|-----------|-------|------|----------|
| gzip | 2.8x | 1.2s | 2.1 |
| bzip2 | 3.1x | 4.8s | 2.3 |
| Pied Piper | 5.2x | 0.8s | 5.2 |

The middle-out approach gives us both better compression *and* faster execution.

{{% details summary="How we benchmarked these results" %}}
All benchmarks were run on identical hardware (32-core AMD EPYC, 128GB RAM) using the Canterbury Corpus dataset. Each algorithm was given the same input files and measured over 100 iterations to account for variance. The Weissman Score was calculated using the standard formula with gzip as the reference implementation.
{{% /details %}}

## The Takeaway

Metrics matter. When evaluating any algorithm, make sure you're measuring what actually counts for your use case.
