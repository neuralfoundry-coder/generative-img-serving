FROM python:3.11-slim

WORKDIR /app

RUN pip install --no-cache-dir fastapi uvicorn httpx pillow

COPY server.py .

# Port is configured via MOCK_PORT environment variable (default: 8001)
EXPOSE 8001 8002 8003 8004

CMD ["python", "server.py"]

