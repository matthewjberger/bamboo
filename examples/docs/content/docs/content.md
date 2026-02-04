+++
title = "Routing"
weight = 3
template = "docs.html"
+++

# Routing

Define routes and handle HTTP requests in Velocity.

## Basic Routes

Create routes using the Router class:

```typescript
import { Router } from '@velocity/core';

const router = new Router();

router.get('/users', (ctx) => {
  return ctx.json({ users: [] });
});

router.post('/users', async (ctx) => {
  const body = await ctx.body();
  return ctx.json({ created: body }, 201);
});

router.put('/users/:id', async (ctx) => {
  const { id } = ctx.params;
  const body = await ctx.body();
  return ctx.json({ updated: { id, ...body } });
});

router.delete('/users/:id', (ctx) => {
  return ctx.status(204);
});
```

## Route Parameters

Access URL parameters through `ctx.params`:

```typescript
// GET /users/123
router.get('/users/:id', (ctx) => {
  const { id } = ctx.params; // "123"
  return ctx.json({ id });
});

// GET /posts/2024/01/hello-world
router.get('/posts/:year/:month/:slug', (ctx) => {
  const { year, month, slug } = ctx.params;
  return ctx.json({ year, month, slug });
});
```

## Query Parameters

Access query string parameters through `ctx.query`:

```typescript
// GET /search?q=hello&page=1
router.get('/search', (ctx) => {
  const { q, page } = ctx.query;
  return ctx.json({ query: q, page: Number(page) || 1 });
});
```

## Request Body

Parse request body with automatic content-type detection:

```typescript
router.post('/api/data', async (ctx) => {
  // Automatically parses JSON, form data, etc.
  const body = await ctx.body();
  return ctx.json(body);
});
```

## Response Methods

Velocity provides fluent response helpers:

```typescript
// JSON response
ctx.json({ data: 'value' });

// Plain text
ctx.text('Hello, World!');

// HTML
ctx.html('<h1>Hello</h1>');

// Redirect
ctx.redirect('/new-location');

// Set status
ctx.status(201).json({ created: true });

// Set headers
ctx.header('X-Custom', 'value').json({ data: true });
```

## Route Groups

Organize routes with prefixes:

```typescript
const api = new Router({ prefix: '/api/v1' });

api.get('/users', listUsers);     // GET /api/v1/users
api.get('/users/:id', getUser);   // GET /api/v1/users/:id
api.post('/users', createUser);   // POST /api/v1/users

app.use(api.routes());
```

## Error Handling

Handle errors gracefully:

```typescript
router.get('/users/:id', async (ctx) => {
  const user = await db.users.find(ctx.params.id);

  if (!user) {
    return ctx.status(404).json({
      error: 'User not found'
    });
  }

  return ctx.json(user);
});
```
