+++
title = "Tabs vs Spaces: A Mathematical Proof"
tags = ["programming", "humor", "opinions"]
excerpt = "I finally have mathematical proof that tabs are objectively superior. Fight me."
+++

I've been saying tabs are better than spaces for years. Now I have proof.

## The Argument for Spaces

Space advocates claim:

1. "Consistent rendering everywhere"
2. "What you see is what you get"
3. "It's the standard"

These are weak arguments from weak minds.

## The Mathematical Proof

Let's define some variables:

- `k` = keystrokes to indent one level
- `b` = bytes per indent level
- `c` = customizability (0 or 1)
- `t` = time wasted in pointless debates

For **spaces** (assuming 4-space indent):
```
k_spaces = 4  (or 1 with editor config)
b_spaces = 4
c_spaces = 0  (everyone sees same width)
```

For **tabs**:
```
k_tabs = 1
b_tabs = 1
c_tabs = 1  (everyone sets their preferred width)
```

The **Developer Efficiency Score (DES)**:

```
DES = (1/k) * (1/b) * (1 + c) - t

DES_spaces = (1/4) * (1/4) * (1 + 0) - t = 0.0625 - t
DES_tabs   = (1/1) * (1/1) * (1 + 1) - t = 2.0 - t
```

Tabs are **32x more efficient**.

Q.E.D.

## But Actually

The real answer is: use whatever your team decided, and *never bring it up again*.

The `t` (time wasted in debates) term dominates everything else. We spent three meetings on this at Pied Piper. THREE MEETINGS.

Configure your editor, set up a linter, and move on with your life.

```json
{
  "editor.insertSpaces": false,
  "editor.tabSize": 4
}
```

There. I've saved you hours of arguing. You're welcome.

## The Only Right Answer

The only truly correct indentation is:

```
\t\t\tif (condition) {
\t\t\t\tdoThing();
\t\t\t}
```

If you disagree, you're objectively wrong and I have the math to prove it.

*(This post is mostly a joke. Mostly.)*
