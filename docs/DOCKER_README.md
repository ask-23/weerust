# WeeWix Docker Setup

Complete Docker infrastructure for WeeWix weather station data ingestion system.

## Files Overview

### Core Docker Files
- **`Dockerfile.optimized`** - Multi-stage production Dockerfile
  - Build stage: Compiles Rust binary with caching
  - Runtime stage: Minimal distroless image (~50MB)
  - Security: Non-root user, stripped binary
  - Performance: Layer caching, compressed build

- **`docker-compose.enhanced.yml`** - Production orchestration
  - WeeWix application service
  - MariaDB with 600GB volume support
  - Optional GW1100 mock device
  - Health checks and monitoring
  - Optimized networking

- **`.dockerignore`** - Build optimization
  - Excludes unnecessary files from Docker context
  - Reduces build time and image size

### Configuration
- **`.env.example`** - Environment template
  - Copy to `.env` before deployment
  - Contains all configurable options
  - Database, networking, logging settings

### Database Initialization
- **`scripts/db-init/01-create-schema.sql`**
  - Weather observations table (compressed, indexed)
  - Daily statistics aggregation table
  - System logging table
  - Optimized for 600GB dataset

- **`scripts/db-init/02-create-procedures.sql`**
  - Stored procedures for daily stats calculation
  - Functions for temperature conversion
  - Heat index calculation
  - Automated maintenance events

### Backup Scripts
- **`scripts/db-backup/backup.sh`**
  - Automated database backup with compression
  - Configurable retention policy (default 30 days)
  - Logs backup metadata to database

- **`scripts/db-backup/restore.sh`**
  - Safe database restoration
  - Confirmation prompts
  - Before/after record counts

### Documentation
- **`docs/DOCKER_DEPLOYMENT.md`** - Complete deployment guide
- **`docs/DOCKER_README.md`** - This file

## Quick Start

```bash
# 1. Setup
cp .env.example .env
mkdir -p data/mysql

# 2. Configure (edit .env)
nano .env

# 3. Deploy
docker-compose -f docker-compose.enhanced.yml up -d

# 4. Monitor
docker-compose -f docker-compose.enhanced.yml logs -f weewix
```

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Docker Network (weewix-net)               │
│                                                              │
│  ┌─────────────────┐                                        │
│  │  GW1100 Device  │                                        │
│  │  (External)     │                                        │
│  └────────┬────────┘                                        │
│           │ HTTP POST                                        │
│           ↓                                                  │
│  ┌─────────────────────────────────────────────┐           │
│  │  WeeWix Container (172.25.0.20)             │           │
│  │  ┌────────────────────────────────────┐     │           │
│  │  │  Rust Application                  │     │           │
│  │  │  - HTTP Server :8080               │     │           │
│  │  │  - UDP Listener :9999              │     │           │
│  │  │  - Ecowitt Parser                  │     │           │
│  │  └────────────────────────────────────┘     │           │
│  └────────┬────────────────────────────────────┘           │
│           │ MySQL Client                                    │
│           ↓                                                  │
│  ┌─────────────────────────────────────────────┐           │
│  │  MariaDB Container (172.25.0.10)            │           │
│  │  ┌────────────────────────────────────┐     │           │
│  │  │  Database Engine                   │     │           │
│  │  │  - InnoDB Compression              │     │           │
│  │  │  - 600GB Volume                    │     │           │
│  │  │  - Auto Stats & Maintenance        │     │           │
│  │  └────────────────────────────────────┘     │           │
│  └─────────────────────────────────────────────┘           │
│                                                              │
│  ┌─────────────────────────────────────────────┐           │
│  │  GW1100 Mock (Optional, Testing Profile)    │           │
│  │  - Simulates weather station                │           │
│  │  - Posts data every 5 seconds               │           │
│  └─────────────────────────────────────────────┘           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Key Features

### WeeWix Container
- **Minimal Image**: Distroless base, ~50MB total
- **Security**: Non-root user (uid 65532)
- **Performance**: Multi-stage build with layer caching
- **Reliability**: Health checks every 30s
- **Logging**: JSON structured logs, 50MB max per file

### MariaDB Container
- **Storage**: 600GB volume with compression
- **Performance**: 4GB buffer pool, optimized for time-series data
- **Maintenance**: Automatic daily stats and weekly optimization
- **Backup**: Scripts included for compressed backups
- **Monitoring**: Health checks and metrics

### Networking
- **Bridge Network**: Isolated network (172.25.0.0/16)
- **Static IPs**: Predictable container addresses
- **Service Discovery**: DNS-based (weewix, mariadb)
- **External Access**: Configurable port mappings

## Environment Variables

### Critical Settings
```bash
# Database credentials (CHANGE IN PRODUCTION!)
DB_ROOT_PASS=rootpass
DB_PASS=weewxpass

# Storage location (must have 600GB+ free)
DB_VOLUME_PATH=./data/mysql

# Station configuration
STATION_ID=home
STATION_TIMEZONE=America/Chicago
```

### Optional Settings
```bash
# HTTP/UDP ports
WEEWIX_HTTP_PORT=8080
WEEWIX_UDP_PORT=9999

# Logging
RUST_LOG=info
INSERT_LOGGING=true

# Mock device (testing only)
MOCK_POST_INTERVAL=5
```

## Database Schema

### weather_observations
- **Purpose**: Primary data table for all weather readings
- **Storage**: InnoDB compressed, ~60% space savings
- **Indexes**: timestamp_utc, station_id, temp/humidity, wind
- **Retention**: Configurable via archival procedure

### daily_statistics
- **Purpose**: Pre-aggregated daily stats for fast queries
- **Updates**: Automatic at 1 AM daily
- **Data**: Min/max/avg for temp, humidity, wind, rain

### system_log
- **Purpose**: Application and database events
- **Levels**: DEBUG, INFO, WARN, ERROR, CRITICAL
- **Uses**: Troubleshooting, audit trail

## Maintenance

### Automated Tasks
- **Daily 1 AM**: Calculate yesterday's statistics
- **Sunday 2 AM**: Optimize tables
- **On-insert**: Temperature conversions, heat index

### Manual Tasks
```bash
# Backup database
docker exec weewix-mariadb /backup/backup.sh

# Check database size
docker exec weewix-mariadb mysql -u weewx -p -e "
  SELECT table_name, ROUND(data_length/1024/1024/1024,2) AS 'Size (GB)'
  FROM information_schema.tables
  WHERE table_schema = 'weewx'
  ORDER BY data_length DESC;
"

# View recent observations
docker exec weewix-mariadb mysql -u weewx -p weewx -e "
  SELECT timestamp_utc, temp_f, humidity, wind_speed_mph, rain_daily_in
  FROM weather_observations
  ORDER BY timestamp_utc DESC
  LIMIT 20;
"
```

## Troubleshooting

### Container won't start
```bash
# Check logs
docker logs weewix-app
docker logs weewix-mariadb

# Verify configuration
cat .env

# Check disk space
df -h
```

### No data in database
```bash
# Check if WeeWix is receiving data
docker logs weewix-app | grep "POST /data"

# Test with mock device
docker-compose -f docker-compose.enhanced.yml --profile testing up -d gw1100-mock
docker logs -f weewix-mock-device

# Verify database connection
docker exec weewix-app nc -zv mariadb 3306
```

### Performance issues
```bash
# Check MariaDB performance
docker exec weewix-mariadb mysql -u root -p -e "SHOW ENGINE INNODB STATUS\G"

# Monitor resource usage
docker stats

# Optimize tables
docker exec weewix-mariadb mysql -u root -p weewx -e "
  OPTIMIZE TABLE weather_observations;
  OPTIMIZE TABLE daily_statistics;
"
```

## Security

### Production Checklist
- [ ] Change all passwords in `.env`
- [ ] Use strong passwords (20+ characters)
- [ ] Restrict database port (remove external mapping)
- [ ] Enable firewall rules
- [ ] Use Docker secrets for sensitive data
- [ ] Regular security updates
- [ ] Monitor logs for anomalies
- [ ] Implement backup strategy
- [ ] Test disaster recovery

### Hardening
```yaml
# Remove external database access
# In docker-compose.enhanced.yml, comment out:
# ports:
#   - "3306:3306"

# Use Docker secrets (recommended for production)
secrets:
  db_password:
    file: ./secrets/db_password.txt
```

## Upgrading

```bash
# Pull latest base images
docker-compose -f docker-compose.enhanced.yml pull

# Rebuild WeeWix
docker-compose -f docker-compose.enhanced.yml build --no-cache weewix

# Backup before upgrading
./scripts/db-backup/backup.sh

# Deploy new version
docker-compose -f docker-compose.enhanced.yml up -d

# Verify
docker-compose -f docker-compose.enhanced.yml ps
docker logs weewix-app --tail 50
```

## Resource Requirements

### Minimum
- **CPU**: 2 cores
- **RAM**: 6GB (4GB for MariaDB, 2GB for system)
- **Disk**: 650GB (600GB database + 50GB overhead)

### Recommended
- **CPU**: 4+ cores
- **RAM**: 16GB (12GB for MariaDB, 4GB for system)
- **Disk**: 1TB+ SSD/NVMe
- **Network**: 100 Mbps+

## Support

For detailed information:
- **Deployment Guide**: `docs/DOCKER_DEPLOYMENT.md`
- **Configuration**: `.env.example`
- **Database Schema**: `scripts/db-init/01-create-schema.sql`
- **Main README**: `../README.md`
