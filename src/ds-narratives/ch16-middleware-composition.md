# Plates on a Barbell: Middleware as Function Composition

## The Problem

Your GrindIt REST API is growing up. It started as a few endpoints — log a score, fetch workouts, get the leaderboard. Now the real world comes knocking:

- **Authentication**: Is this athlete logged in?
- **Rate limiting**: Is someone hammering the leaderboard endpoint 1000 times a second?
- **Logging**: Which endpoints are slow? Who's hitting what?
- **CORS**: The PWA on `grindit.app` needs to call the API on `api.grindit.app`.

Four concerns. None of them have anything to do with the actual workout logic. But every single request needs all four checks *before* it reaches your handler.

## The Naive Way

Your first attempt: one massive function that does everything.

```rust,ignore
fn handle_request(req: Request) -> Response {
    // Auth check
    let token = match req.headers.get("Authorization") {
        Some(t) => t,
        None => return Response::new(401, "Unauthorized"),
    };
    if !verify_token(token) {
        return Response::new(401, "Invalid token");
    }

    // Rate limiting
    let ip = req.remote_addr;
    if get_request_count(ip) > 100 {
        return Response::new(429, "Too many requests");
    }
    increment_request_count(ip);

    // Logging
    let start = std::time::Instant::now();

    // CORS
    if req.method == "OPTIONS" {
        let mut resp = Response::new(200, "");
        resp.set_header("Access-Control-Allow-Origin", "*");
        return resp;
    }

    // FINALLY, the actual logic
    let response = match req.path.as_str() {
        "/scores" => handle_scores(req),
        "/workouts" => handle_workouts(req),
        _ => Response::new(404, "Not found"),
    };

    // More logging
    println!("Request took {:?}", start.elapsed());

    response
}
```

This is a 40-line function and we haven't even added compression yet. Every new concern means editing this monster. Miss a closing brace, accidentally reorder the auth check below rate limiting, and now unauthenticated users can burn through your rate limit pool.

What happens when your coach says "add request compression"? You wade into the swamp and pray.

## The Insight

Look at a barbell. Each plate is independent — you can add a 10, remove a 5, swap the order. The barbell doesn't care what plates are on it. The plates don't care what other plates exist.

What if each concern — auth, rate limiting, logging, CORS — was a separate plate? Stack them in any order. Add or remove without touching the others. Each one does exactly one job: inspect the request, maybe modify it, pass it along (or reject it).

This is **middleware composition** — and underneath, it's just **function composition**. Each middleware is a function that takes a "next handler" and returns a new handler that wraps it with extra behavior.

```
Request --> [CORS] --> [Logging] --> [RateLimit] --> [Auth] --> Handler
Response <-- [CORS] <-- [Logging] <-- [RateLimit] <-- [Auth] <-- Handler
```

This is called the **onion model**. The request peels inward through layers; the response wraps back out. Logging sees the request on the way in *and* can time the response on the way out.

## The Build

Let's build this from scratch. First, our basic types:

```rust
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
    remote_addr: String,
}

#[derive(Debug, Clone)]
struct Response {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
}

impl Response {
    fn new(status: u16, body: &str) -> Self {
        Response {
            status,
            headers: HashMap::new(),
            body: body.to_string(),
        }
    }
}
```

Now the core abstraction — a handler is anything that takes a request and returns a response:

```rust
trait Handler {
    fn handle(&self, req: Request) -> Response;
}

// Any function with the right signature is a handler
impl<F: Fn(Request) -> Response> Handler for F {
    fn handle(&self, req: Request) -> Response {
        (self)(req)
    }
}
```

And middleware is anything that wraps one handler to produce another:

```rust
trait Middleware {
    fn wrap(&self, next: Box<dyn Handler>) -> Box<dyn Handler>;
}
```

Let's build our four plates. **Logging** — wraps around the handler to time it:

```rust
struct LoggingMiddleware;

struct LoggingHandler {
    next: Box<dyn Handler>,
}

impl Handler for LoggingHandler {
    fn handle(&self, req: Request) -> Response {
        let method = req.method.clone();
        let path = req.path.clone();
        let start = Instant::now();

        let response = self.next.handle(req);

        println!("[{}] {} {} -> {} ({:?})",
            method, path, response.status,
            response.body.len(), start.elapsed());
        response
    }
}

impl Middleware for LoggingMiddleware {
    fn wrap(&self, next: Box<dyn Handler>) -> Box<dyn Handler> {
        Box::new(LoggingHandler { next })
    }
}
```

**Auth** — rejects requests without a valid token:

```rust
struct AuthMiddleware {
    valid_tokens: Vec<String>,
}

struct AuthHandler {
    valid_tokens: Vec<String>,
    next: Box<dyn Handler>,
}

impl Handler for AuthHandler {
    fn handle(&self, req: Request) -> Response {
        match req.headers.get("Authorization") {
            Some(token) if self.valid_tokens.contains(token) => {
                self.next.handle(req)
            }
            _ => Response::new(401, "Unauthorized: grab your membership card"),
        }
    }
}

impl Middleware for AuthMiddleware {
    fn wrap(&self, next: Box<dyn Handler>) -> Box<dyn Handler> {
        Box::new(AuthHandler {
            valid_tokens: self.valid_tokens.clone(),
            next,
        })
    }
}
```

**CORS** — adds the right headers so the PWA can talk to the API:

```rust
struct CorsMiddleware {
    allowed_origin: String,
}

struct CorsHandler {
    allowed_origin: String,
    next: Box<dyn Handler>,
}

impl Handler for CorsHandler {
    fn handle(&self, req: Request) -> Response {
        // Preflight requests get an immediate response
        if req.method == "OPTIONS" {
            let mut resp = Response::new(204, "");
            resp.headers.insert(
                "Access-Control-Allow-Origin".into(),
                self.allowed_origin.clone(),
            );
            resp.headers.insert(
                "Access-Control-Allow-Methods".into(),
                "GET, POST, PUT, DELETE".into(),
            );
            return resp;
        }

        let mut response = self.next.handle(req);
        response.headers.insert(
            "Access-Control-Allow-Origin".into(),
            self.allowed_origin.clone(),
        );
        response
    }
}

impl Middleware for CorsMiddleware {
    fn wrap(&self, next: Box<dyn Handler>) -> Box<dyn Handler> {
        Box::new(CorsHandler {
            allowed_origin: self.allowed_origin.clone(),
            next,
        })
    }
}
```

Now the **Pipeline** — our barbell that holds all the plates:

```rust
struct Pipeline {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl Pipeline {
    fn new() -> Self {
        Pipeline { middlewares: Vec::new() }
    }

    fn add(mut self, mw: impl Middleware + 'static) -> Self {
        self.middlewares.push(Box::new(mw));
        self
    }

    /// Build the final handler by wrapping from inside out.
    fn build(self, handler: impl Handler + 'static) -> Box<dyn Handler> {
        let mut current: Box<dyn Handler> = Box::new(handler);
        // Wrap in reverse so the first middleware added is the outermost layer
        for mw in self.middlewares.into_iter().rev() {
            current = mw.wrap(current);
        }
        current
    }
}
```

## The Payoff

```rust
fn main() {
    // The actual workout handler — clean, focused, no cross-cutting concerns
    let workout_handler = |req: Request| -> Response {
        match req.path.as_str() {
            "/scores" => Response::new(200, r#"{"score": "Fran 3:45"}"#),
            "/workouts" => Response::new(200, r#"{"wod": "21-15-9 Thrusters & Pull-ups"}"#),
            _ => Response::new(404, "Not found"),
        }
    };

    // Stack the plates
    let app = Pipeline::new()
        .add(LoggingMiddleware)
        .add(CorsMiddleware { allowed_origin: "https://grindit.app".into() })
        .add(AuthMiddleware { valid_tokens: vec!["athlete-token-123".into()] })
        .build(workout_handler);

    // Authenticated request
    let mut headers = HashMap::new();
    headers.insert("Authorization".into(), "athlete-token-123".into());
    let req = Request {
        method: "GET".into(),
        path: "/scores".into(),
        headers,
        body: String::new(),
        remote_addr: "192.168.1.1".into(),
    };

    let resp = app.handle(req);
    assert_eq!(resp.status, 200);
    assert!(resp.headers.contains_key("Access-Control-Allow-Origin"));

    // Unauthenticated request — auth middleware rejects it
    let bad_req = Request {
        method: "GET".into(),
        path: "/scores".into(),
        headers: HashMap::new(),
        body: String::new(),
        remote_addr: "192.168.1.1".into(),
    };

    let resp = app.handle(bad_req);
    assert_eq!(resp.status, 401);

    println!("Pipeline works. Each plate does its job.");
}
```

Need compression? Add one plate. Don't touch anything else:

```rust,ignore
let app = Pipeline::new()
    .add(LoggingMiddleware)
    .add(CorsMiddleware { allowed_origin: "https://grindit.app".into() })
    .add(AuthMiddleware { valid_tokens: vec!["token".into()] })
    // New plate, zero changes to existing code:
    // .add(CompressionMiddleware { min_size: 1024 })
    .build(workout_handler);
```

This is exactly how **Axum's layer system** and **Tower middleware** work under the hood. You've just built the core idea from scratch.

## Complexity Comparison

| Concern | Monolithic Handler | Composable Middleware |
|---------|-------------------|----------------------|
| Add new concern | Edit mega-function, risk breaking others | Add one struct, call `.add()` |
| Remove a concern | Carefully delete nested code | Remove one `.add()` call |
| Reorder concerns | Restructure the whole function | Swap two lines |
| Test one concern | Mock everything else | Test middleware in isolation |
| Code per concern | Tangled with all others | **Self-contained** |

This isn't about Big-O — it's about **human-O**. The time it takes *you* to add a feature without introducing a bug.

## Try It Yourself

1. **RateLimitMiddleware**: Build a rate limiter that tracks request counts per IP using a `HashMap<String, (u64, Instant)>` (count + window start). Return 429 if more than 100 requests in 60 seconds. Where in the pipeline should this go — before or after auth?

2. **Request ID middleware**: Generate a unique ID for each request (a simple counter works) and attach it to the response headers as `X-Request-Id`. This is invaluable for debugging — "which request failed?" becomes answerable.

3. **Conditional middleware**: Build a `ConditionalMiddleware` that only applies its inner middleware if a predicate matches (e.g., only rate-limit POST requests, not GETs). This is how real frameworks implement route-specific middleware.
