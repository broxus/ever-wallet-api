#!/bin/bash

sqlx migrate run && /app/application $1
