# Multi-stage build for ans-nerves
FROM python:3.12-slim AS builder
WORKDIR /app
COPY pyproject.toml .
RUN pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir \
      grpcio>=1.60 \
      grpcio-tools>=1.60 \
      pyarrow>=15 \
      pydantic>=2 \
      httpx>=0.26 \
      tenacity>=8 \
      structlog>=24 \
      numpy>=1.26 \
      sentence-transformers>=2.7 \
      openai>=1.60 \
      lancedb>=0.17

FROM python:3.12-slim AS runtime
WORKDIR /app
COPY --from=builder /usr/local/lib/python3.12/site-packages /usr/local/lib/python3.12/site-packages
COPY ans_nerves/ ans_nerves/
COPY pyproject.toml .

ENV PYTHONUNBUFFERED=1
ENV ANS_GRPC_HOST=host.docker.internal

EXPOSE 8080
CMD ["python", "-m", "ans_nerves", "serve"]
