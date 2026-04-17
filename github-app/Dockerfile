FROM python:3.12-slim

# Install system deps for building loopsleuth (Rust + CMake)
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential cmake pkg-config && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"

# Install loopsleuth
RUN pip install --no-cache-dir loopsleuth

# Install app dependencies
COPY requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r /app/requirements.txt

COPY app/ /app/app/

WORKDIR /app
EXPOSE 8000

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
