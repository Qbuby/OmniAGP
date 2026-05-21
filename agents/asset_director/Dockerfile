FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . ./agents/asset_director/

EXPOSE 8092

CMD ["uvicorn", "agents.asset_director.server:app", "--host", "0.0.0.0", "--port", "8092"]
