+++
title = "Getting Started"
weight = 1
template = "docs.html"
+++

# Getting Started

Learn how to install Velocity and create your first application.

## Installation

Install Velocity using your preferred package manager:

```bash
npm install @velocity/core
# or
yarn add @velocity/core
# or
pnpm add @velocity/core
```

## Create a New Application

Create a new file called `app.ts`:

```typescript
import { Velocity, Router } from '@velocity/core';

const app = new Velocity();
const router = new Router();

router.get('/', (ctx) => {
  return ctx.json({ message: 'Hello, Velocity!' });
});

router.get('/users/:id', (ctx) => {
  const { id } = ctx.params;
  return ctx.json({ user: { id, name: 'John Doe' } });
});

app.use(router.routes());

app.listen(3000, () => {
  console.log('Server running at http://localhost:3000');
});
```

## Project Structure

A typical Velocity project looks like this:

```
my-app/
├── src/
│   ├── routes/
│   │   ├── users.ts
│   │   └── posts.ts
│   ├── middleware/
│   │   ├── auth.ts
│   │   └── logger.ts
│   ├── models/
│   │   └── user.ts
│   └── app.ts
├── tests/
├── package.json
└── velocity.config.ts
```

## Development Server

Start the development server with hot reload:

```bash
npx velocity dev
```

Open http://localhost:3000 to see your API. Edit any file and the server will automatically restart.

## Build for Production

Build your application for production:

```bash
npx velocity build
```

Then start the production server:

```bash
npx velocity start
```

## Next Steps

- Learn about [Configuration](../configuration/) options
- Explore [Routing](../content/) patterns
- Add [Middleware](../templating/) for authentication and logging
