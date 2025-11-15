#!/bin/bash
# Cleans all .rdb files and server.aof from every node_* subfolder inside nodes

echo "Searching for .rdb files and server.aof in all node_* folders under nodes..."
find nodes -type d -name "node_*" -exec sh -c '
    for dir in "$@"; do
        rm -fv "$dir"/*.rdb "$dir"/server.aof
    done
' sh {} +
echo "Cleanup complete! All .rdb files and server.aof files have been removed from node_* folders."