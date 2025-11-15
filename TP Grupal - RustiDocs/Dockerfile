# Etapa 1: Build
FROM rust:latest AS builder
WORKDIR /app

COPY Cargo.toml ./ 
COPY Cargo.lock* ./

RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/node.rs && \
    echo "fn main() {}" > src/bin/interfaz.rs && \
    echo "fn main() {}" > src/bin/llm_service.rs && \
    echo "fn main() {}" > src/bin/microservice.rs && \
    echo "fn main() {}" > src/bin/microservice_docker.rs && \
    cargo build --release

COPY src ./src
RUN cargo build --release

# Etapa 2: Runtime
FROM debian:bookworm-slim
WORKDIR /app

# Instalar dependencias necesarias
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

# Copiar binarios
COPY --from=builder /app/target/release/llm_service /app/
COPY --from=builder /app/target/release/node /app/
COPY --from=builder /app/target/release/microservice /app/
COPY --from=builder /app/target/release/microservice_docker /app/
COPY --from=builder /app/target/release/interfaz /app/

# Copiar configs estáticos
COPY src/config ./config

COPY user.acl ./user.acl
COPY utils/nodes ./utils/nodes

CMD ["./llm_service"]
