# Docker Deployment - RustiDocs Cluster

Este documento describe el deployment con Docker del cluster Redis y los microservicios.

## Artefactos Incluidos

### ✅ Implementado:

1. **Dockerfile** - Multi-stage build para optimizar tamaño
2. **docker-compose.yml** - Configuración básica del cluster
3. **docker-compose.fixed.yml** - Configuración mejorada con volúmenes y networking
4. **docker-cluster.sh** - Script de gestión completo
5. **.env** - Variables de entorno (incluye GEMINI_API_KEY)

## Comandos de Gestión

### Construcción e Inicio
```bash
# Construir la imagen
./docker-cluster.sh build

# Iniciar el cluster completo
./docker-cluster.sh start

# Construir e iniciar en un solo comando
./docker-cluster.sh build && ./docker-cluster.sh start
```

### Gestión del Cluster
```bash
# Ver estado de todos los servicios
./docker-cluster.sh status

# Ver logs de todos los servicios
./docker-cluster.sh logs

# Ver logs de un servicio específico
./docker-cluster.sh logs node_1
./docker-cluster.sh logs llm_service

# Reiniciar un servicio
./docker-cluster.sh restart node_1
```

### Detención y Limpieza
```bash
# Detener el cluster (mantiene volúmenes)
./docker-cluster.sh stop

# Destruir completamente (elimina volúmenes)
./docker-cluster.sh destroy

# Limpiar archivos temporales
./docker-cluster.sh clean
```

## Arquitectura del Deployment

### Servicios Desplegados

1. **9 Nodos Redis** (node_1 a node_9)
   - Puertos: 7001-7009
   - Volúmenes persistentes para datos
   - Red compartida para comunicación

2. **LLM Service** 
   - Puerto: 8080
   - Conecta con node_1:7001
   - Requiere GEMINI_API_KEY

3. **Microservice**
   - Puerto: 8081
   - Para gestión de documentos

4. **Interfaz**
   - Puerto: 3000
   - Frontend web

### Networking

- Red personalizada `redis-cluster` para aislamiento
- Comunicación interna por nombres de contenedor
- Puertos expuestos solo para acceso externo necesario

### Persistencia

- Volúmenes Docker nombrados para cada nodo
- Datos persistentes entre reinicios
- Logs y dumps preservados

## Configuración Requerida

### Variables de Entorno

Editar `.env`:
```bash
GEMINI_API_KEY=tu_api_key_aqui
```

### Puertos en Uso

- **7001-7009**: Nodos Redis
- **8080**: LLM Service API
- **8081**: Microservice API  
- **3000**: Interfaz Web

## Resolución de Problemas

### Si el build falla:
```bash
# Limpiar imágenes y volver a construir
docker system prune -f
./docker-cluster.sh build
```

### Si los contenedores no se comunican:
```bash
# Verificar la red
docker network ls
docker network inspect rustidocs-llm_redis-cluster
```

### Si faltan datos:
```bash
# Verificar volúmenes
docker volume ls
docker volume inspect rustidocs-llm_node_1_data
```

### Ver logs detallados:
```bash
# Logs en tiempo real
./docker-cluster.sh logs

# Logs de un servicio específico
./docker-cluster.sh logs node_1
```

## Mejoras Implementadas

1. **Volúmenes persistentes** para datos de cada nodo
2. **Red dedicada** para mejor aislamiento  
3. **Dependencias explícitas** entre servicios
4. **Scripts de gestión** automatizados
5. **Política de reinicio** automático
6. **Puertos organizados** y documentados

## Uso en Producción

Para producción, considerar:

1. **Secrets management** para API keys
2. **Health checks** para cada servicio
3. **Resource limits** (CPU/memoria)
4. **Backup automático** de volúmenes
5. **Monitoring** y alertas
6. **Load balancing** si es necesario
