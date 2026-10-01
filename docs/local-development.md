# Local development

1. `cp .env.example .env`
2. `docker compose up`
3. Check:
   - Web: http://localhost:3000
   - API health: http://localhost:8080/health
   - MinIO console: http://localhost:9001 (login with `MINIO_ROOT_USER` /
     `MINIO_ROOT_PASSWORD` from `.env`)
4. `docker compose down` when done.
