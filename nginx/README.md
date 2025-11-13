# Nginx Configuration

Nginx reverse proxy configuration for Smally API.

## Directory Structure

```
nginx/
├── nginx.conf              # Main Nginx configuration
├── conf.d/
│   └── smally.conf     # Site-specific configuration
├── ssl/                    # SSL certificates (gitignored)
│   ├── fullchain.pem      # Full certificate chain
│   └── privkey.pem        # Private key
└── README.md              # This file
```

## SSL Certificate Setup

### Option 1: Let's Encrypt (Production)

**Prerequisites:**
- Domain pointed to your server
- Ports 80 and 443 accessible

**Steps:**

1. **Stop Nginx temporarily:**
   ```bash
   docker-compose -f docker-compose.prod.yml stop nginx
   ```

2. **Install Certbot:**
   ```bash
   sudo apt update
   sudo apt install certbot
   ```

3. **Get certificate:**
   ```bash
   sudo certbot certonly --standalone \
       -d api.yourdomain.com \
       --agree-tos \
       --email admin@yourdomain.com
   ```

4. **Copy certificates:**
   ```bash
   mkdir -p nginx/ssl
   sudo cp /etc/letsencrypt/live/api.yourdomain.com/fullchain.pem nginx/ssl/
   sudo cp /etc/letsencrypt/live/api.yourdomain.com/privkey.pem nginx/ssl/
   sudo chown -R $USER:$USER nginx/ssl
   ```

5. **Start Nginx:**
   ```bash
   docker-compose -f docker-compose.prod.yml start nginx
   ```

**Auto-renewal:**

Add to crontab (`crontab -e`):
```bash
0 3 * * * certbot renew --quiet && cp /etc/letsencrypt/live/api.yourdomain.com/*.pem /path/to/smally-api/nginx/ssl/ && cd /path/to/smally-api && docker-compose -f docker-compose.prod.yml restart nginx
```

### Option 2: Self-Signed (Development/Testing)

**Quick setup:**

```bash
mkdir -p nginx/ssl
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout nginx/ssl/privkey.pem \
    -out nginx/ssl/fullchain.pem \
    -subj "/CN=api.yourdomain.com/O=Smally/C=US"
```

**Note:** Browsers will show a security warning. For testing only!

### Option 3: Custom Certificate

If you have your own certificate from a CA:

1. Place certificate chain in `nginx/ssl/fullchain.pem`
2. Place private key in `nginx/ssl/privkey.pem`
3. Restart Nginx

## Configuration

### Update Domain

Edit `nginx/conf.d/smally.conf`:

```nginx
server_name api.yourdomain.com;  # Change this
```

### Rate Limiting

Current limits (can be adjusted in `nginx.conf`):

- API endpoints: 100 requests/second per IP
- Docs/static: 10 requests/second per IP
- Burst: 20 requests allowed

To adjust:

```nginx
# In nginx.conf
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=200r/s;
```

### Custom Headers

Add security headers in `nginx/conf.d/smally.conf`:

```nginx
add_header X-Custom-Header "value" always;
```

## Testing Configuration

**Test syntax:**
```bash
docker run --rm -v $(pwd)/nginx:/etc/nginx:ro nginx nginx -t
```

**Reload without downtime:**
```bash
docker-compose -f docker-compose.prod.yml exec nginx nginx -s reload
```

## Troubleshooting

### Certificate Not Found

**Error:**
```
nginx: [emerg] cannot load certificate "/etc/nginx/ssl/fullchain.pem"
```

**Solution:**
```bash
# Check if files exist
ls -la nginx/ssl/

# Generate self-signed cert if missing
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout nginx/ssl/privkey.pem \
    -out nginx/ssl/fullchain.pem \
    -subj "/CN=localhost"
```

### Permission Denied

**Error:**
```
Permission denied
```

**Solution:**
```bash
chmod 644 nginx/ssl/fullchain.pem
chmod 600 nginx/ssl/privkey.pem
chown -R $USER:$USER nginx/ssl
```

### Rate Limit Too Aggressive

**Symptom:** Legitimate requests getting 429 errors

**Solution:**
```nginx
# Increase rate limit in nginx.conf
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=200r/s;

# Or increase burst in smally.conf
limit_req zone=api_limit burst=50 nodelay;
```

### SSL Certificate Expiry

**Check expiry:**
```bash
openssl x509 -in nginx/ssl/fullchain.pem -noout -dates
```

**Renew Let's Encrypt:**
```bash
sudo certbot renew
# Then copy new certs to nginx/ssl/
```

## Security Best Practices

1. **Keep private key secure:**
   ```bash
   chmod 600 nginx/ssl/privkey.pem
   ```

2. **Use strong ciphers:**
   - Already configured in `smally.conf`
   - TLS 1.2+ only
   - Forward secrecy enabled

3. **HSTS enabled:**
   - Forces HTTPS for 1 year
   - Includes subdomains

4. **Regular updates:**
   ```bash
   docker pull nginx:alpine
   docker-compose -f docker-compose.prod.yml up -d nginx
   ```

## Monitoring

**Access logs:**
```bash
docker-compose -f docker-compose.prod.yml exec nginx tail -f /var/log/nginx/access.log
```

**Error logs:**
```bash
docker-compose -f docker-compose.prod.yml exec nginx tail -f /var/log/nginx/error.log
```

**Connection stats:**
```bash
docker-compose -f docker-compose.prod.yml exec nginx nginx -s reload
```

## Advanced Configuration

### WebSocket Support (if needed)

Add to location block in `smally.conf`:

```nginx
location /ws/ {
    proxy_pass http://smally_backend;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
}
```

### Custom Error Pages

```nginx
error_page 404 /404.html;
location = /404.html {
    root /usr/share/nginx/html;
    internal;
}
```

### IP Whitelisting

```nginx
# Allow only specific IPs
location /admin/ {
    allow 203.0.113.0/24;
    deny all;
    proxy_pass http://smally_backend;
}
```

## References

- [Nginx Documentation](https://nginx.org/en/docs/)
- [Let's Encrypt](https://letsencrypt.org/)
- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
