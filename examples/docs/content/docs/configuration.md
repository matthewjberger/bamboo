+++
title = "Configuration"
weight = 2
template = "docs.html"
+++

# Configuration

Configure your Velocity application using the `velocity.config.ts` file.

## Basic Configuration

```typescript
// velocity.config.ts
import { defineConfig } from '@velocity/core';

export default defineConfig({
  port: 3000,
  host: '0.0.0.0',
  env: 'development',
});
```

## Environment Variables

Velocity automatically loads `.env` files:

```bash
# .env
PORT=3000
DATABASE_URL=postgres://localhost:5432/myapp
JWT_SECRET=your-secret-key
```

Access them in your application:

```typescript
import { env } from '@velocity/core';

const dbUrl = env.DATABASE_URL;
const secret = env.JWT_SECRET;
```

## Configuration Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `port` | number | 3000 | Server port |
| `host` | string | localhost | Server host |
| `env` | string | development | Environment mode |
| `cors` | object | undefined | CORS configuration |
| `bodyLimit` | string | 1mb | Request body size limit |
| `trustProxy` | boolean | false | Trust reverse proxy headers |

## CORS Configuration

Enable and configure Cross-Origin Resource Sharing:

```typescript
export default defineConfig({
  cors: {
    origin: ['https://example.com', 'https://app.example.com'],
    methods: ['GET', 'POST', 'PUT', 'DELETE'],
    allowedHeaders: ['Content-Type', 'Authorization'],
    credentials: true,
  },
});
```

## Database Configuration

Configure your database connection:

```typescript
export default defineConfig({
  database: {
    client: 'postgres',
    connection: env.DATABASE_URL,
    pool: {
      min: 2,
      max: 10,
    },
    migrations: {
      directory: './migrations',
    },
  },
});
```

## Environment-Specific Configuration

Override settings based on environment:

```typescript
export default defineConfig({
  port: 3000,
  ...(process.env.NODE_ENV === 'production' && {
    port: 8080,
    trustProxy: true,
  }),
});
```
