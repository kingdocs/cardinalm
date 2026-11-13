#!/usr/bin/env bash
# Sign Searchflux.app with a local self-signed code-signing identity.
#
# Why not ad-hoc (`codesign -s -`):
#   An ad-hoc signed app has this designated requirement:
#       designated => cdhash H"<hash of the binary>"
#   macOS TCC (Full Disk Access) binds the grant to that requirement, so every
#   rebuild changes the cdhash and silently invalidates the grant.
#
#   With a self-signed identity the requirement becomes:
#       designated => identifier "io.github.micheas.cardinalm"
#                      and certificate root = H"<certificate fingerprint>"
#   It no longer depends on the binary, so rebuilding does not require
#   re-authorizing. The certificate is not an Apple Developer ID, so Gatekeeper
#   still requires right-click > Open on first launch; that is expected.
#
# SECURITY: the certificate and private key live only in the login keychain
# and in a local 0700 directory. Never commit them, never put them in the app.
#
# Usage:
#   scripts/sign-self-signed.sh --create-identity   # once per machine
#   scripts/sign-self-signed.sh [path/to/Searchflux.app]
set -euo pipefail

IDENTITY="CardinalM Self-Signed"
SECDIR="$HOME/Library/Application Support/CardinalM-Signing"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

create_identity() {
  mkdir -p "$SECDIR"
  chmod 700 "$SECDIR"

  cat > "$SECDIR/openssl.cnf" << 'EOF'
[ req ]
distinguished_name = dn
x509_extensions    = v3_code_sign
prompt             = no
utf8               = yes

[ dn ]
CN = CardinalM Self-Signed
O  = Personal Build
C  = CN

[ v3_code_sign ]
basicConstraints     = critical,CA:FALSE
keyUsage             = critical,digitalSignature
extendedKeyUsage     = critical,codeSigning
subjectKeyIdentifier = hash
EOF
  chmod 600 "$SECDIR/openssl.cnf"

  # OpenSSL 3+ prefers -noenc; LibreSSL/older builds may need -nodes instead.
  openssl req -x509 -newkey rsa:3072 -sha256 -days 3650 -noenc \
    -keyout "$SECDIR/key.pem" -out "$SECDIR/cert.pem" -config "$SECDIR/openssl.cnf"
  chmod 600 "$SECDIR/key.pem"
  chmod 644 "$SECDIR/cert.pem"

  local pass
  pass=$(openssl rand -hex 16)
  # -legacy is required: macOS security(1) cannot verify the MAC of OpenSSL
  # 3+/4 default PKCS#12 bundles and fails with "MAC verification failed".
  openssl pkcs12 -export -legacy -out "$SECDIR/identity.p12" \
    -inkey "$SECDIR/key.pem" -in "$SECDIR/cert.pem" -passout pass:"$pass"
  chmod 600 "$SECDIR/identity.p12"

  security import "$SECDIR/identity.p12" -k "$KEYCHAIN" -P "$pass" \
    -A -T /usr/bin/codesign

  echo "Identity created: $IDENTITY"
  echo "Private material is in $SECDIR (0700). Do not commit it."
}

if [[ "${1:-}" == "--create-identity" ]]; then
  create_identity
  exit 0
fi

if ! security find-identity -p codesigning | grep -qF "$IDENTITY"; then
  echo "No identity named '$IDENTITY' in the login keychain." >&2
  echo "Run once on this machine: $0 --create-identity" >&2
  exit 1
fi

APP="${1:-/Applications/Searchflux.app}"
if [[ ! -d "$APP" ]]; then
  echo "App bundle not found: $APP" >&2
  exit 1
fi

# An interrupted codesign can leave a *.cstemp behind; the stray file breaks
# the sealed-resource check on the next run. Clean up before signing.
find "$APP" -name '*.cstemp' -delete 2>/dev/null || true

codesign --force --deep --sign "$IDENTITY" --timestamp=none "$APP"
codesign --verify --verbose=2 "$APP"
echo "--- designated requirement (the Full Disk Access grant is bound to this) ---"
codesign -d -r- "$APP" 2>&1 | grep designated
