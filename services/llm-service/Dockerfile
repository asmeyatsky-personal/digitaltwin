# LLM Service with Multi-Provider LLM Support
# Multi-stage build for optimized image size

FROM python:3.12-slim AS builder

ARG INCLUDE_LOCAL_EMBEDDINGS=false

RUN pip install --no-cache-dir --upgrade pip

RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Conditionally install sentence-transformers (adds ~2GB with torch)
RUN if [ "$INCLUDE_LOCAL_EMBEDDINGS" = "true" ]; then \
      pip install --no-cache-dir sentence-transformers>=3.0.0; \
    fi


FROM python:3.12-slim

COPY --from=builder /opt/venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

WORKDIR /app

COPY main.py .
COPY providers/ ./providers/
COPY embeddings/ ./embeddings/

RUN useradd --create-home appuser && \
    chown -R appuser:appuser /app
USER appuser

ENV PYTHONUNBUFFERED=1

EXPOSE 8004

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8004/health')" || exit 1

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8004", "--workers", "1"]
