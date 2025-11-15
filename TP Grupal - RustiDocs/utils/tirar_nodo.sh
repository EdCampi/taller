#!/usr/bin/env bash
# Script para detener (tirar abajo) un nodo específico del cluster Docker Compose


if [ -z "$1" ]; then
  echo "Uso: $0 <nro_nodo>"
  exit 1
fi

NODO="node_$1"

echo "Deteniendo el contenedor $NODO..."
docker compose stop $NODO

echo "Contenedor $NODO detenido."


#para ejecutar: ./tirar_nodo.sh 3 (aca tiro el nodo 3)