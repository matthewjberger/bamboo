+++
title = "Distributed Consensus is Hard"
tags = ["distributed-systems", "consensus", "pipernet"]
excerpt = "Building PiperNet taught me why distributed consensus is one of the hardest problems in computer science."
+++

Building PiperNet has given me a deep appreciation for why distributed consensus is considered one of the hardest problems in computer science.

## The Problem

Imagine you have 1000 computers that need to agree on something—say, the order of transactions. Sounds simple until you consider:

- Any computer might fail at any time
- Network messages can be delayed, duplicated, or lost
- Some computers might be malicious
- You can't trust any single computer

## The CAP Theorem

You've probably heard of CAP: you can only have two of Consistency, Availability, and Partition tolerance.

```
        Consistency
           /\
          /  \
         /    \
        /      \
       /________\
Availability  Partition
              Tolerance
```

For a decentralized network, we *must* have partition tolerance. So we're choosing between consistency and availability.

## What We Chose

PiperNet prioritizes availability with eventual consistency. Here's why:

1. **User experience** - Users hate seeing "service unavailable"
2. **Decentralization** - Requiring strong consistency often means central coordinators
3. **Our use case** - For file storage, slightly stale data is acceptable

## The Implementation

We use a variant of CRDTs (Conflict-free Replicated Data Types) combined with vector clocks:

```go
type FileMetadata struct {
    Hash        [32]byte
    VectorClock map[NodeID]uint64
    Chunks      []ChunkLocation
}

func (f *FileMetadata) Merge(other *FileMetadata) *FileMetadata {
    // CRDTs guarantee this merge is commutative,
    // associative, and idempotent
    merged := &FileMetadata{
        Hash:        f.Hash,
        VectorClock: mergeClocks(f.VectorClock, other.VectorClock),
        Chunks:      mergeChunks(f.Chunks, other.Chunks),
    }
    return merged
}
```

## Lessons Learned

1. **Start simple** - We tried Raft first, it was overkill
2. **Test failures** - Use chaos engineering from day one
3. **Monitor everything** - You can't debug what you can't see

Consensus is hard, but it's a solved problem. Don't reinvent unless you have to.
