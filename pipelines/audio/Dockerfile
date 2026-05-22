FROM python:3.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg libsndfile1 && \
    rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . ./pipelines/audio/

EXPOSE 8090

CMD ["uvicorn", "pipelines.audio.server:app", "--host", "0.0.0.0", "--port", "8090"]
