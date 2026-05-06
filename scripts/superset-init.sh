#!/bin/sh
set -e

echo "[superset] running db upgrade..."
superset db upgrade

echo "[superset] creating admin user..."
superset fab create-admin \
  --username admin \
  --password admin \
  --firstname Admin \
  --lastname User \
  --email admin@example.com \
  2>/dev/null || true

echo "[superset] initializing..."
superset init 2>/dev/null || true

echo "[superset] ready"
