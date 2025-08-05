use tower::Layer;

// A simple marker struct representing the custom layer.
struct MyLayer;

// The middleware struct that wraps another service.
// It adds additional behavior before or after calling the inner service.
struct MyMiddleware<S> {
    inner: S,
}

// Implement the Layer trait to turn a service into a wrapped (middleware) service.
impl<S> Layer<S> for MyLayer {
    // The type of the service after applying the middleware.
    type Service = MyMiddleware<S>;

    // The method that wraps the given service with the middleware.
    fn layer(&self, inner: S) -> Self::Service {
        MyMiddleware { inner }
    }
}