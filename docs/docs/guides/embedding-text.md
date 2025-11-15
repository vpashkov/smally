---
sidebar_position: 1
---

# Embedding Text

Learn how to use the Smally API to create high-quality text embeddings.

## Basic Usage

The `/v1/embed` endpoint converts text into a 384-dimensional embedding vector:

```bash
curl -X POST "http://localhost:8000/v1/embed" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Machine learning is transforming technology",
    "normalize": false
  }'
```

## Request Parameters

### `text` (required)

The text to embed. Can be a word, sentence, or paragraph.

- **Type**: `string`
- **Max length**: 2000 characters
- **Max tokens**: 128

```json
{
  "text": "Your text here"
}
```

### `normalize` (optional)

Whether to L2 normalize the embedding vector.

- **Type**: `boolean`
- **Default**: `false`

```json
{
  "text": "Your text here",
  "normalize": true
}
```

**When to use normalization:**
- Cosine similarity calculations (vectors are already normalized)
- Consistent magnitude across all embeddings
- Some distance metrics work better with normalized vectors

## Response Format

```json
{
  "embedding": [0.0234, -0.1567, 0.0892, ...],
  "tokens": 8,
  "cached": false,
  "model": "all-MiniLM-L6-v2"
}
```

### Fields

- **`embedding`**: 384-dimensional float array
- **`tokens`**: Number of tokens in the input text
- **`cached`**: Whether the result was served from cache
- **`model`**: Model identifier used for embeddings

## Use Cases

### Semantic Search

Find similar documents using cosine similarity:

```python
import numpy as np

def cosine_similarity(a, b):
    return np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b))

# Embed query and documents
query_emb = embed("machine learning algorithms")
doc1_emb = embed("Neural networks and deep learning")
doc2_emb = embed("The weather forecast for tomorrow")

# Calculate similarity
sim1 = cosine_similarity(query_emb, doc1_emb)  # High similarity
sim2 = cosine_similarity(query_emb, doc2_emb)  # Low similarity
```

### Clustering

Group similar texts together:

```python
from sklearn.cluster import KMeans

# Embed multiple texts
texts = [
    "Machine learning basics",
    "Deep neural networks",
    "Cooking pasta recipes",
    "Italian cuisine guide"
]

embeddings = [embed(text) for text in texts]

# Cluster
kmeans = KMeans(n_clusters=2)
labels = kmeans.fit_predict(embeddings)
# [0, 0, 1, 1] - Groups ML and cooking topics
```

### Duplicate Detection

Find near-duplicate content:

```python
threshold = 0.95  # High similarity threshold

def is_duplicate(text1, text2):
    emb1 = embed(text1)
    emb2 = embed(text2)
    similarity = cosine_similarity(emb1, emb2)
    return similarity > threshold
```

### Question Answering

Match questions to answers:

```python
question = "How do I reset my password?"
answers = [
    "Visit the password reset page and enter your email",
    "Contact support for billing questions",
    "Check our API documentation for integration help"
]

# Find best matching answer
question_emb = embed(question)
answer_embs = [embed(ans) for ans in answers]

similarities = [cosine_similarity(question_emb, ans_emb)
                for ans_emb in answer_embs]

best_answer = answers[np.argmax(similarities)]
```

## Best Practices

### Input Text Quality

✅ **Good inputs:**
- Complete sentences or phrases
- Clean, well-formatted text
- Consistent language and style

❌ **Poor inputs:**
- Single words (except for specific use cases)
- Extremely long paragraphs (truncated at 128 tokens)
- Mixed languages in same text

### Batch Processing

Process multiple texts efficiently:

```python
import asyncio
import aiohttp

async def embed_batch(texts):
    async with aiohttp.ClientSession() as session:
        tasks = [
            embed_async(session, text)
            for text in texts
        ]
        return await asyncio.gather(*tasks)

async def embed_async(session, text):
    async with session.post(
        'http://localhost:8000/v1/embed',
        headers={'Authorization': f'Bearer {API_KEY}'},
        json={'text': text, 'normalize': False}
    ) as response:
        return await response.json()

# Embed 100 texts concurrently
texts = [...]  # Your texts
embeddings = asyncio.run(embed_batch(texts))
```

### Caching

Leverage automatic caching for frequently used texts:

```python
# First request: Fresh computation
result1 = embed("common query")  # cached: false

# Second request: Served from cache
result2 = embed("common query")  # cached: true, ~1ms
```

See [Caching Guide](/docs/guides/caching) for details.

## Error Handling

### Text Too Long

```json
{
  "error": "text_too_long",
  "message": "Text exceeds maximum token limit of 128"
}
```

**Solution**: Split long texts into chunks or summarize.

### Empty Text

```json
{
  "error": "invalid_request",
  "message": "Text cannot be empty"
}
```

**Solution**: Validate input before sending.

### Rate Limit

```json
{
  "error": "rate_limit_exceeded",
  "message": "Monthly quota exhausted"
}
```

**Solution**: Check `X-RateLimit-Reset` header and wait or upgrade tier.

## Performance Tips

1. **Use caching**: Identical inputs are cached automatically
2. **Batch requests**: Use async/concurrent requests for multiple texts
3. **Normalize wisely**: Only normalize when needed (e.g., cosine similarity)
4. **Monitor rate limits**: Check response headers to avoid quota exhaustion

## Next Steps

- [Caching](/docs/guides/caching) - Understand how caching works
- [Rate Limits](/docs/guides/rate-limits) - Monitor and optimize usage
- [API Reference](/api) - Full API documentation
