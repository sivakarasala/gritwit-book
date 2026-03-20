// Chapter 16 DSA Exercise: Middleware as Function Composition
//
// Tower layers form an onion: each middleware wraps the next.
// f(g(h(handler))) — request flows inward, response flows outward.
// This is the decorator pattern applied to async services.

use std::collections::HashMap;
use std::fmt;

// ----------------------------------------------------------------
// Part 1: Middleware as function composition
// ----------------------------------------------------------------

/// A simplified HTTP request
#[derive(Debug, Clone)]
struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

/// A simplified HTTP response
#[derive(Debug, Clone)]
struct Response {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
}

impl Response {
    fn ok(body: &str) -> Self {
        Response {
            status: 200,
            headers: HashMap::new(),
            body: body.to_string(),
        }
    }

    fn error(status: u16, body: &str) -> Self {
        Response {
            status,
            headers: HashMap::new(),
            body: body.to_string(),
        }
    }
}

/// A service that processes a request and returns a response.
/// Each middleware wraps an inner service, forming an onion.
trait Service {
    fn call(&self, req: &Request) -> Response;
    fn name(&self) -> &str;
}

// ----------------------------------------------------------------
// The core handler (innermost layer)
// ----------------------------------------------------------------

struct AppHandler;

impl Service for AppHandler {
    fn call(&self, req: &Request) -> Response {
        println!("    [Handler] Processing {} {}", req.method, req.path);
        match req.path.as_str() {
            "/api/v1/health_check" => Response::ok("{\"status\": \"ok\"}"),
            "/api/v1/exercises" => Response::ok("{\"exercises\": [...]}"),
            "/login" => Response::ok("<html>Login page</html>"),
            _ => Response::error(404, "Not Found"),
        }
    }
    fn name(&self) -> &str {
        "Handler"
    }
}

// ----------------------------------------------------------------
// Middleware layers — each wraps an inner service
// ----------------------------------------------------------------

/// Request ID layer: assigns a unique ID to each request
struct RequestIdLayer<S: Service> {
    inner: S,
    counter: std::cell::Cell<u64>,
}

impl<S: Service> RequestIdLayer<S> {
    fn new(inner: S) -> Self {
        RequestIdLayer {
            inner,
            counter: std::cell::Cell::new(0),
        }
    }
}

impl<S: Service> Service for RequestIdLayer<S> {
    fn call(&self, req: &Request) -> Response {
        let id = self.counter.get() + 1;
        self.counter.set(id);

        let mut modified = req.clone();
        modified
            .headers
            .insert("X-Request-Id".to_string(), format!("req-{:04}", id));
        println!("    [RequestId] Assigned req-{:04}", id);

        let mut response = self.inner.call(&modified);
        response
            .headers
            .insert("X-Request-Id".to_string(), format!("req-{:04}", id));
        response
    }
    fn name(&self) -> &str {
        "RequestId"
    }
}

/// Logging/tracing layer
struct TraceLayer<S: Service> {
    inner: S,
}

impl<S: Service> TraceLayer<S> {
    fn new(inner: S) -> Self {
        TraceLayer { inner }
    }
}

impl<S: Service> Service for TraceLayer<S> {
    fn call(&self, req: &Request) -> Response {
        let req_id = req
            .headers
            .get("X-Request-Id")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "    [Trace] -> {} {} (id={})",
            req.method, req.path, req_id
        );

        let response = self.inner.call(req);

        println!(
            "    [Trace] <- {} {} (status={})",
            req.method, req.path, response.status
        );
        response
    }
    fn name(&self) -> &str {
        "Trace"
    }
}

/// Auth layer: checks for Authorization header
struct AuthLayer<S: Service> {
    inner: S,
    public_paths: Vec<String>,
}

impl<S: Service> AuthLayer<S> {
    fn new(inner: S, public_paths: Vec<String>) -> Self {
        AuthLayer {
            inner,
            public_paths,
        }
    }
}

impl<S: Service> Service for AuthLayer<S> {
    fn call(&self, req: &Request) -> Response {
        // Skip auth for public paths
        if self.public_paths.iter().any(|p| req.path == *p) {
            println!("    [Auth] Public route — skipping auth");
            return self.inner.call(req);
        }

        if let Some(token) = req.headers.get("Authorization") {
            if token.starts_with("Bearer ") {
                println!("    [Auth] Authenticated (token: {}...)", &token[..15.min(token.len())]);
                return self.inner.call(req);
            }
        }

        println!("    [Auth] UNAUTHORIZED — short-circuiting");
        Response::error(401, "Unauthorized")
    }
    fn name(&self) -> &str {
        "Auth"
    }
}

/// Rate limiting layer: blocks after threshold
struct RateLimitLayer<S: Service> {
    inner: S,
    max_requests: usize,
    request_count: std::cell::Cell<usize>,
}

impl<S: Service> RateLimitLayer<S> {
    fn new(inner: S, max_requests: usize) -> Self {
        RateLimitLayer {
            inner,
            max_requests,
            request_count: std::cell::Cell::new(0),
        }
    }
}

impl<S: Service> Service for RateLimitLayer<S> {
    fn call(&self, req: &Request) -> Response {
        let count = self.request_count.get() + 1;
        self.request_count.set(count);

        if count > self.max_requests {
            println!(
                "    [RateLimit] BLOCKED (request #{} > limit {})",
                count, self.max_requests
            );
            return Response::error(429, "Too Many Requests");
        }
        println!(
            "    [RateLimit] Allowed ({}/{})",
            count, self.max_requests
        );
        self.inner.call(req)
    }
    fn name(&self) -> &str {
        "RateLimit"
    }
}

// ----------------------------------------------------------------
// Part 2: Chain of Responsibility
// ----------------------------------------------------------------

trait RequestHandler: fmt::Display {
    fn handle(&self, req: &Request) -> Option<Response>;
}

struct HealthCheckHandler;
struct ExercisesHandler;
struct NotFoundHandler;

impl fmt::Display for HealthCheckHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HealthCheck")
    }
}
impl RequestHandler for HealthCheckHandler {
    fn handle(&self, req: &Request) -> Option<Response> {
        if req.path == "/api/v1/health_check" {
            println!("    [{}] Handling request", self);
            Some(Response::ok("{\"status\": \"ok\"}"))
        } else {
            None
        }
    }
}

impl fmt::Display for ExercisesHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Exercises")
    }
}
impl RequestHandler for ExercisesHandler {
    fn handle(&self, req: &Request) -> Option<Response> {
        if req.path.starts_with("/api/v1/exercises") {
            println!("    [{}] Handling request", self);
            Some(Response::ok("{\"exercises\": []}"))
        } else {
            None
        }
    }
}

impl fmt::Display for NotFoundHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NotFound")
    }
}
impl RequestHandler for NotFoundHandler {
    fn handle(&self, req: &Request) -> Option<Response> {
        println!("    [{}] No handler matched {}", self, req.path);
        Some(Response::error(404, "Not Found"))
    }
}

fn chain_of_responsibility(handlers: &[&dyn RequestHandler], req: &Request) -> Response {
    for handler in handlers {
        if let Some(response) = handler.handle(req) {
            return response;
        }
        println!("    [{}] Passing to next...", handler);
    }
    Response::error(500, "No handler in chain")
}

fn main() {
    println!("=== Middleware as Function Composition ===\n");

    // Part 1: Build the middleware stack
    // Layers are nested: RateLimit(Auth(RequestId(Trace(Handler))))
    // Execution order: RateLimit -> Auth -> RequestId -> Trace -> Handler
    println!("--- Part 1: Middleware Onion ---");
    println!("  Stack: RateLimit -> Auth -> RequestId -> Trace -> Handler");
    println!("  (outermost layer handles request first, response last)\n");

    let handler = AppHandler;
    let traced = TraceLayer::new(handler);
    let with_id = RequestIdLayer::new(traced);
    let authed = AuthLayer::new(
        with_id,
        vec![
            "/api/v1/health_check".to_string(),
            "/login".to_string(),
        ],
    );
    let stack = RateLimitLayer::new(authed, 3);

    // Test requests
    let requests = vec![
        ("Public (no auth needed)", Request {
            method: "GET".to_string(),
            path: "/api/v1/health_check".to_string(),
            headers: HashMap::new(),
        }),
        ("Authenticated", Request {
            method: "GET".to_string(),
            path: "/api/v1/exercises".to_string(),
            headers: vec![("Authorization".to_string(), "Bearer token123".to_string())]
                .into_iter()
                .collect(),
        }),
        ("Unauthenticated (blocked by Auth)", Request {
            method: "GET".to_string(),
            path: "/api/v1/exercises".to_string(),
            headers: HashMap::new(),
        }),
        ("Rate limited (4th request)", Request {
            method: "GET".to_string(),
            path: "/api/v1/health_check".to_string(),
            headers: HashMap::new(),
        }),
    ];

    for (label, req) in &requests {
        println!("  Request: {} — {} {}", label, req.method, req.path);
        let response = stack.call(req);
        println!(
            "  Response: {} {}\n",
            response.status, response.body
        );
    }

    // Part 2: Chain of Responsibility
    println!("--- Part 2: Chain of Responsibility ---");
    let handlers: Vec<&dyn RequestHandler> = vec![
        &HealthCheckHandler,
        &ExercisesHandler,
        &NotFoundHandler,
    ];

    let test_paths = [
        "/api/v1/health_check",
        "/api/v1/exercises",
        "/api/v1/nonexistent",
    ];

    for path in &test_paths {
        let req = Request {
            method: "GET".to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
        };
        println!("  GET {}", path);
        let response = chain_of_responsibility(&handlers, &req);
        println!("  => {} {}\n", response.status, response.body);
    }

    // Part 3: Mathematical view
    println!("--- Part 3: Composition as Math ---");
    println!("  Each middleware is a function: f(inner) -> wrapped_service");
    println!("  Composition: RateLimit(Auth(RequestId(Trace(Handler))))");
    println!();
    println!("  Request  -> RateLimit -> Auth -> RequestId -> Trace -> Handler");
    println!("  Response <- RateLimit <- Auth <- RequestId <- Trace <- Handler");
    println!();
    println!("  Tower's Layer trait (simplified):");
    println!("    trait Layer<S> {{");
    println!("        type Service;");
    println!("        fn layer(&self, inner: S) -> Self::Service;");
    println!("    }}");
    println!();
    println!("  Axum applies .layer() calls in REVERSE order:");
    println!("    .layer(session_layer)        // 4th added -> runs 4th");
    println!("    .layer(TraceLayer)           // 3rd added -> runs 3rd");
    println!("    .layer(SetRequestIdLayer)    // 2nd added -> runs 2nd");
    println!("    .layer(PropagateRequestId)   // 1st added -> runs 1st");
    println!();
    println!("  The last .layer() wraps the outermost shell.");

    println!("\n=== Key Insights ===");
    println!("1. Middleware = function composition: f(g(h(handler)))");
    println!("2. Each layer can: modify request, call inner, modify response, or short-circuit");
    println!("3. Tower layers are monomorphized — zero dynamic dispatch overhead");
    println!("4. Layer ordering matters: Trace needs the request ID from SetRequestIdLayer");
    println!("5. Rate limiting and auth can short-circuit (never reach the handler)");
    println!("6. Chain of responsibility: each handler decides to handle or delegate");
}
