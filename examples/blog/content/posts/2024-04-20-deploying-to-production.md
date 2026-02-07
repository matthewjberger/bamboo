+++
title = "Startup Lessons: Technical Debt"
tags = ["startups", "engineering", "lessons"]
categories = ["startups"]
excerpt = "What I've learned about managing technical debt while trying to ship fast."
+++

Three years into Pied Piper, I've learned a lot about technical debt. Here's what I wish someone had told me earlier.

## Technical Debt is a Loan

The metaphor is apt. Like financial debt:

- **It lets you move faster now** by borrowing from the future
- **Interest accumulates** - The longer you wait, the worse it gets
- **Some debt is good** - A mortgage lets you buy a house you couldn't afford with cash

The problem is when you take on debt without realizing it.

## The Three Types

### 1. Deliberate Debt

"We know this is hacky, but we need to ship by Friday."

This is fine *if* you actually pay it back. We keep a `DEBT.md` file:

```markdown
## Technical Debt Register

### High Priority
- [ ] Auth service still uses MD5 (security risk)
- [ ] No rate limiting on public API

### Medium Priority
- [ ] Duplicate code in compression modules
- [ ] Test coverage below 60% in networking layer

### Low Priority
- [ ] Variable naming inconsistent
- [ ] Some functions exceed 100 lines
```

### 2. Accidental Debt

You didn't know it was debt when you wrote it. Later, requirements changed and your design doesn't fit.

This is unavoidable. The key is recognizing it early.

### 3. Bit Rot

Code that was fine but degraded over time. Dependencies got updated. The ecosystem moved on. Your patterns became antipatterns.

## How We Manage It

1. **20% rule** - One day per week for debt repayment
2. **Boy Scout rule** - Leave code cleaner than you found it
3. **Debt ceiling** - If the register gets too long, we stop features

## The Hardest Lesson

Sometimes the best move is to declare bankruptcy and rewrite. We did this with our storage layer. It hurt, but the alternative was death by a thousand cuts.

Technical debt isn't bad. *Unmanaged* technical debt is bad.
