#!/bin/bash

# Script para ejecutar todos los tests de integración
# Autor: AI Assistant
# Fecha: $(date)

echo "🚀 Ejecutando Tests de Integración para Redis"
echo "=============================================="

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Función para imprimir con colores
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Verificar que estamos en el directorio correcto
if [ ! -f "Cargo.toml" ]; then
    print_error "No se encontró Cargo.toml. Asegúrate de estar en el directorio raíz del proyecto."
    exit 1
fi

# Verificar que Rust está instalado
if ! command -v cargo &> /dev/null; then
    print_error "Cargo no está instalado. Por favor instala Rust primero."
    exit 1
fi

print_status "Verificando dependencias..."
cargo check --quiet
if [ $? -ne 0 ]; then
    print_error "Error en la verificación de dependencias."
    exit 1
fi
print_success "Dependencias verificadas correctamente."

echo ""
print_status "Ejecutando tests de integración..."

# Ejecutar tests de protocolo
echo ""
print_status "1. Tests de Protocolo RESP..."
cargo test --test integration_tests protocol_tests -- --nocapture
PROTOCOL_RESULT=$?

# Ejecutar tests de comandos
echo ""
print_status "2. Tests de Comandos Redis..."
cargo test --test integration_tests command_tests -- --nocapture
COMMAND_RESULT=$?

# Ejecutar tests de persistencia
echo ""
print_status "3. Tests de Persistencia..."
cargo test --test integration_tests persistence_tests -- --nocapture
PERSISTENCE_RESULT=$?

# Ejecutar tests de Pub/Sub
echo ""
print_status "4. Tests de Pub/Sub..."
cargo test --test integration_tests pubsub_tests -- --nocapture
PUBSUB_RESULT=$?

# Ejecutar todos los tests juntos
echo ""
print_status "5. Ejecutando todos los tests juntos..."
cargo test --test integration_tests -- --nocapture
ALL_RESULT=$?

echo ""
echo "=============================================="
print_status "Resumen de Resultados:"
echo "=============================================="

if [ $PROTOCOL_RESULT -eq 0 ]; then
    print_success "✅ Tests de Protocolo: PASARON"
else
    print_error "❌ Tests de Protocolo: FALLARON"
fi

if [ $COMMAND_RESULT -eq 0 ]; then
    print_success "✅ Tests de Comandos: PASARON"
else
    print_error "❌ Tests de Comandos: FALLARON"
fi

if [ $PERSISTENCE_RESULT -eq 0 ]; then
    print_success "✅ Tests de Persistencia: PASARON"
else
    print_error "❌ Tests de Persistencia: FALLARON"
fi

if [ $PUBSUB_RESULT -eq 0 ]; then
    print_success "✅ Tests de Pub/Sub: PASARON"
else
    print_error "❌ Tests de Pub/Sub: FALLARON"
fi

if [ $ALL_RESULT -eq 0 ]; then
    print_success "✅ Todos los Tests: PASARON"
else
    print_error "❌ Algunos Tests: FALLARON"
fi

echo ""
echo "=============================================="

# Determinar el resultado final
if [ $PROTOCOL_RESULT -eq 0 ] && [ $COMMAND_RESULT -eq 0 ] && [ $PERSISTENCE_RESULT -eq 0 ] && [ $PUBSUB_RESULT -eq 0 ]; then
    print_success "🎉 ¡Todos los tests de integración pasaron exitosamente!"
    exit 0
else
    print_error "💥 Algunos tests fallaron. Revisa los errores arriba."
    exit 1
fi 