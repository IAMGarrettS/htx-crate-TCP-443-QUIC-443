#!/usr/bin/env bash
set -e

# Directory for certificates
CERT_DIR="certs"
mkdir -p "$CERT_DIR"

echo "Generating a local CA..."
# Generate CA private key
openssl genrsa -out "$CERT_DIR/ca.key" 4096

# Generate CA certificate
openssl req -x509 -new -nodes -key "$CERT_DIR/ca.key" -sha256 -days 3650 \
  -subj "/C=US/ST=Guatemala/L=Guatemala/O=HTX/OU=Dev/CN=HTX-CA" \
  -out "$CERT_DIR/ca.crt"

echo "Generating server certificate..."
# Generate server private key
openssl genrsa -out "$CERT_DIR/server.key" 2048

# Generate server CSR (Certificate Signing Request)
openssl req -new -key "$CERT_DIR/server.key" \
  -subj "/C=US/ST=Guatemala/L=Guatemala/O=HTX/OU=Dev/CN=localhost" \
  -out "$CERT_DIR/server.csr"

# Create a config for SAN (Subject Alternative Name)
cat > "$CERT_DIR/server.ext" <<EOL
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
EOL

# Sign server certificate with CA
openssl x509 -req -in "$CERT_DIR/server.csr" -CA "$CERT_DIR/ca.crt" -CAkey "$CERT_DIR/ca.key" \
  -CAcreateserial -out "$CERT_DIR/server.crt" -days 365 -sha256 -extfile "$CERT_DIR/server.ext"

echo "Certificates generated in $CERT_DIR:"
ls -l "$CERT_DIR"
echo "Done! You can now run the echo server and client."
