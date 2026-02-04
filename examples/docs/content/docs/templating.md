+++
title = "Middleware"
weight = 4
template = "docs.html"
+++

# Middleware

Add cross-cutting functionality to your Velocity application with middleware.

## Using Middleware

Apply middleware to your application:

```typescript
import { Velocity, logger, cors } from '@velocity/core';

const app = new Velocity();

// Apply middleware globally
app.use(logger());
app.use(cors());

// Apply to specific routes
app.use('/api', authMiddleware);
```

## Built-in Middleware

Velocity includes several commonly-used middleware:

### Logger

Log all incoming requests:

```typescript
import { logger } from '@velocity/core';

app.use(logger({
  format: ':method :url :status :response-time ms',
  skip: (ctx) => ctx.path === '/health',
}));
```

### CORS

Enable Cross-Origin Resource Sharing:

```typescript
import { cors } from '@velocity/core';

app.use(cors({
  origin: 'https://example.com',
  credentials: true,
}));
```

### Body Parser

Parse request bodies (included by default):

```typescript
import { bodyParser } from '@velocity/core';

app.use(bodyParser({
  limit: '10mb',
  types: ['json', 'form', 'multipart'],
}));
```

### Rate Limiting

Protect against abuse:

```typescript
import { rateLimit } from '@velocity/core';

app.use(rateLimit({
  windowMs: 60 * 1000, // 1 minute
  max: 100, // 100 requests per window
}));
```

## Custom Middleware

Create your own middleware:

```typescript
import { Middleware } from '@velocity/core';

const timing: Middleware = async (ctx, next) => {
  const start = Date.now();

  await next();

  const duration = Date.now() - start;
  ctx.header('X-Response-Time', `${duration}ms`);
};

app.use(timing);
```

## Authentication Middleware

Example JWT authentication middleware:

```typescript
import { Middleware, HttpError } from '@velocity/core';
import { verify } from 'jsonwebtoken';

const auth: Middleware = async (ctx, next) => {
  const header = ctx.headers.authorization;

  if (!header?.startsWith('Bearer ')) {
    throw new HttpError(401, 'Missing authorization header');
  }

  const token = header.slice(7);

  try {
    const payload = verify(token, process.env.JWT_SECRET!);
    ctx.state.user = payload;
  } catch {
    throw new HttpError(401, 'Invalid token');
  }

  await next();
};

// Protect routes
router.get('/profile', auth, (ctx) => {
  return ctx.json({ user: ctx.state.user });
});
```

## Error Handling Middleware

Catch and format errors:

```typescript
const errorHandler: Middleware = async (ctx, next) => {
  try {
    await next();
  } catch (error) {
    if (error instanceof HttpError) {
      return ctx.status(error.status).json({
        error: error.message,
      });
    }

    console.error(error);
    return ctx.status(500).json({
      error: 'Internal server error',
    });
  }
};

app.use(errorHandler);
```

## Middleware Order

Middleware executes in the order it's added. Place error handlers first and authentication before protected routes:

```typescript
app.use(errorHandler);  // 1. Catch all errors
app.use(logger());      // 2. Log requests
app.use(cors());        // 3. CORS headers
app.use(bodyParser());  // 4. Parse bodies
app.use('/api', auth);  // 5. Auth for /api routes
app.use(router.routes()); // 6. Route handlers
```
