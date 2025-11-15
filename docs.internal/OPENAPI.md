# OpenAPI Documentation

The Smally API now includes interactive OpenAPI/Swagger documentation using [utoipa](https://github.com/juhaku/utoipa).

## Accessing the Documentation

### Swagger UI (Interactive)

Visit the Swagger UI at:

```
http://localhost:8000/swagger-ui
```

This provides:

- Interactive API testing
- Request/response examples
- Schema definitions
- Authentication testing

### OpenAPI JSON Spec

Get the raw OpenAPI specification at:

```
http://localhost:8000/openapi.json
```

This can be imported into tools like:

- Postman
- Insomnia
- API clients
- Code generators

## Using Swagger UI

### 1. Open Swagger UI

Navigate to `http://localhost:8000/swagger-ui` in your browser.

### 2. Authenticate

Click the **Authorize** button at the top right:

1. Enter your API key
2. Click **Authorize**
3. Click **Close**

### 3. Try Endpoints

#### Test Embedding Endpoint

1. Expand the **POST /v1/embed** endpoint
2. Click **Try it out**
3. Modify the request body:

   ```json
   {
     "text": "Hello world",
     "normalize": false
   }
   ```

4. Click **Execute**
5. View the response below

#### Health Check

1. Expand **GET /health**
2. Click **Try it out**
3. Click **Execute**
4. See service status and build information

## API Documentation Structure

### Tags

Endpoints are organized by tags:

- **embeddings**: Text embedding endpoints
- **health**: Health check and status endpoints

### Security

All endpoints requiring authentication use Bearer token authentication:

```
Authorization: Bearer YOUR_API_KEY
```

### Schemas

All request and response types are fully documented:

- `EmbedRequest`: Embedding request with text and options
- `EmbedResponse`: Embedding result with metadata
- `ErrorResponse`: Standard error format
- `HealthResponse`: Health check response

## Example Requests

### Using curl (from Swagger UI)

The Swagger UI generates curl commands for each request:

```bash
curl -X POST "http://localhost:8000/v1/embed" \
  -H "accept: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"text":"Hello world","normalize":false}'
```

### Using JavaScript

```javascript
const response = await fetch('http://localhost:8000/v1/embed', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer YOUR_API_KEY'
  },
  body: JSON.stringify({
    text: 'Hello world',
    normalize: false
  })
});

const data = await response.json();
console.log(data.embedding); // 384-dimensional vector
console.log(data.tokens);    // Token count
console.log(data.cached);    // Whether cached
```

### Using Python

```python
import requests

response = requests.post(
    'http://localhost:8000/v1/embed',
    headers={
        'Authorization': 'Bearer YOUR_API_KEY',
        'Content-Type': 'application/json'
    },
    json={
        'text': 'Hello world',
        'normalize': False
    }
)

data = response.json()
print(data['embedding'])  # 384-dimensional vector
print(data['tokens'])     # Token count
print(data['cached'])     # Whether cached
```

## Rate Limiting

The API includes rate limiting based on your subscription tier. Rate limit information is returned in response headers:

- `X-RateLimit-Limit`: Monthly request limit
- `X-RateLimit-Remaining`: Remaining requests this month
- `X-RateLimit-Reset`: Reset timestamp

Example response headers:

```
X-RateLimit-Limit: 20000
X-RateLimit-Remaining: 19950
X-RateLimit-Reset: 2025-02-01T00:00:00Z
```

## Error Handling

All errors follow a standard format:

```json
{
  "error": "invalid_request",
  "message": "Text cannot be empty"
}
```

Error types:

- `invalid_request`: Bad request (400)
- `invalid_api_key`: Authentication failed (401)
- `rate_limit_exceeded`: Quota exhausted (429)
- `text_too_long`: Input exceeds token limit (400)
- `internal_error`: Server error (500)

## Code Generation

Use the OpenAPI spec to generate client libraries:

### Using openapi-generator

```bash
# Download the spec
curl http://localhost:8000/openapi.json > openapi.json

# Generate TypeScript client
openapi-generator-cli generate \
  -i openapi.json \
  -g typescript-axios \
  -o ./client

# Generate Python client
openapi-generator-cli generate \
  -i openapi.json \
  -g python \
  -o ./client
```

### Using OpenAPI Tools

Import the spec URL into:

- **Postman**: File → Import → Link → `http://localhost:8000/openapi.json`
- **Insomnia**: Create → Import → URL → `http://localhost:8000/openapi.json`

## Adding New Endpoints

To add OpenAPI documentation for new endpoints:

### 1. Annotate Request/Response Types

```rust
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct MyResponse {
    /// Field description
    #[schema(example = "example value")]
    pub field: String,
}
```

### 2. Annotate Handler Function

```rust
#[utoipa::path(
    post,
    path = "/v1/my-endpoint",
    tag = "my-tag",
    request_body = MyRequest,
    responses(
        (status = 200, description = "Success", body = MyResponse),
        (status = 400, description = "Bad request", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn my_handler(Json(req): Json<MyRequest>) -> Json<MyResponse> {
    // ...
}
```

### 3. Add to OpenAPI Struct

```rust
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        my_handler,  // Add your handler here
        // ... other handlers
    ),
    components(
        schemas(
            MyRequest,   // Add your schemas here
            MyResponse,
            // ... other schemas
        )
    )
)]
pub struct ApiDoc;
```

## Production Deployment

### Update Server URLs

Edit `src/api/mod.rs`:

```rust
#[openapi(
    // ...
    servers(
        (url = "http://localhost:8000", description = "Local development"),
        (url = "https://api.example.com", description = "Production")
    )
)]
```

### Disable Swagger UI in Production (Optional)

If you want to disable Swagger UI in production but keep the OpenAPI spec:

```rust
// In main.rs
let app = Router::new()
    // ... routes ...

// Only add Swagger UI in development
#[cfg(debug_assertions)]
let app = app.merge(SwaggerUi::new("/swagger-ui")
    .url("/openapi.json", api::ApiDoc::openapi()));

app.layer(/* ... */)
```

## Benefits

✅ **Interactive Testing**: Test endpoints without writing code
✅ **Auto-Generated Docs**: Documentation stays in sync with code
✅ **Type Safety**: Compile-time validation of API schemas
✅ **Client Generation**: Generate clients in any language
✅ **Standards Compliant**: OpenAPI 3.0 specification
✅ **No Runtime Overhead**: Documentation generated at compile time

## Resources

- [utoipa Documentation](https://docs.rs/utoipa/)
- [OpenAPI Specification](https://swagger.io/specification/)
- [Swagger UI](https://swagger.io/tools/swagger-ui/)
