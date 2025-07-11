#!/bin/bash

# Build and (re-)start the container
docker compose build && docker compose down && docker compose up -d

