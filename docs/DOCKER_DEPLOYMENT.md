# WeeWix Docker Deployment Guide

Complete guide for deploying WeeWix weather station software using Docker and Docker Compose.

## Quick Start

```bash
# 1. Copy environment template
cp .env.example .env

# 2. Edit .env with your configuration
nano .env

# 3. Create data directory for 600GB database
mkdir -p ./data/mysql

# 4. Start services
docker-compose -f docker-compose.enhanced.yml up -d

# 5. View logs
docker-compose -f docker-compose.enhanced.yml logs -f weewix
```

## Architecture Overview

```
┌─────────────────┐
│  Ecowitt GW1100 │
│ Weather Station │
└────────┬────────┘
         │ HTTP POST /data
         │ (Port 8080)
         ↓
┌─────────────────────────┐
│   WeeWix Application    │
│  (Rust, Distroless)     │
│  - HTTP Server :8080    │
│  - UDP Listener :9999   │
└────────┬────────────────┘
         │ SQL INSERT
         ↓
┌─────────────────────────┐
│   MariaDB Database      │
│  - 600GB Volume         │
│  - Compressed Storage   │
│  - Auto Stats           │
└─────────────────────────┘
```

## Services

### 1. WeeWix Application
- **Image**: Built from `Dockerfile.optimized`
- **Port 8080**: HTTP endpoint for weather station POST requests
- **Port 9999**: UDP interceptor for broadcast devices
- **Features**:
  - Multi-stage build for minimal image size
  - Distroless base for security
  - Non-root user execution
  - Health checks enabled

### 2. MariaDB Database
- **Image**: `mariadb:11`
- **Storage**: 600GB volume with compression
- **Optimizations**:
  - 4GB InnoDB buffer pool
  - 1GB log file size
  - Row compression enabled
  - 500 max connections
- **Initialization**: Automatic schema creation on first run

### 3. GW1100 Mock Device (Optional)
- **Image**: `curlimages/curl:8.10.1`
- **Purpose**: Testing and development
- **Activation**: Use profile `--profile testing`
- **Interval**: Posts data every 5 seconds (configurable)

## Configuration

### Environment Variables

Create `.env` from `.env.example` and customize:

```bash
# Database
DB_ROOT_PASS=your-secure-password
DB_PASS=your-weewx-password

# Station
STATION_ID=home
STATION_TIMEZONE=America/Chicago

# Storage
DB_VOLUME_PATH=./data/mysql
```

### Volume Configuration

The database volume is configured for 600GB storage:

```yaml
volumes:
  db_data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: ./data/mysql
```

**Requirements**:
- Directory must exist before starting
- Must have 600GB+ free space
- Use fast SSD/NVMe for best performance

## Usage

### Production Deployment

```bash
# Start all services
docker-compose -f docker-compose.enhanced.yml up -d

# Check status
docker-compose -f docker-compose.enhanced.yml ps

# View logs
docker-compose -f docker-compose.enhanced.yml logs -f

# Stop services
docker-compose -f docker-compose.enhanced.yml down
```

### Development with Mock Device

```bash
# Start with mock weather station
docker-compose -f docker-compose.enhanced.yml --profile testing up -d

# Watch mock device posts
docker logs -f weewix-mock-device

# Stop all including mock
docker-compose -f docker-compose.enhanced.yml --profile testing down
```

### Database Access

```bash
# Connect to MariaDB
docker exec -it weewix-mariadb mysql -u weewx -p weewx

# Show recent observations
SELECT * FROM weather_observations ORDER BY timestamp_utc DESC LIMIT 10;

# Check daily statistics
SELECT * FROM daily_statistics ORDER BY date_local DESC LIMIT 7;
```

### Backup and Restore

```bash
# Backup database
docker exec weewix-mariadb mysqldump -u root -p weewx > backup.sql

# Restore database
docker exec -i weewix-mariadb mysql -u root -p weewx < backup.sql
```

## Monitoring

### Health Checks

All services include health checks:

```bash
# Check WeeWix health
docker exec weewix-app /app/weewx --version

# Check MariaDB health
docker exec weewix-mariadb healthcheck.sh --connect
```

### Metrics

View application metrics:

```bash
# Database size
docker exec weewix-mariadb mysql -u root -p -e "
  SELECT
    table_schema AS 'Database',
    ROUND(SUM(data_length + index_length) / 1024 / 1024 / 1024, 2) AS 'Size (GB)'
  FROM information_schema.tables
  WHERE table_schema = 'weewx'
  GROUP BY table_schema;
"

# Record counts
docker exec weewix-mariadb mysql -u weewx -p weewx -e "
  SELECT COUNT(*) as total_observations FROM weather_observations;
"
```

## Troubleshooting

### Connection Issues

```bash
# Check network connectivity
docker network inspect weewix-net

# Test database connection from WeeWix
docker exec weewix-app nc -zv mariadb 3306
```

### Log Analysis

```bash
# WeeWix application logs
docker logs weewix-app --tail 100 -f

# MariaDB logs
docker logs weewix-mariadb --tail 100 -f

# System logs from database
docker exec weewix-mariadb mysql -u weewx -p weewx -e "
  SELECT * FROM system_log ORDER BY timestamp DESC LIMIT 20;
"
```

### Performance Tuning

For systems with different resources, adjust MariaDB settings in `docker-compose.enhanced.yml`:

```yaml
command:
  # For 8GB RAM systems
  - --innodb-buffer-pool-size=6G
  - --innodb-log-file-size=2G

  # For 16GB RAM systems
  - --innodb-buffer-pool-size=12G
  - --innodb-log-file-size=4G
```

## Security

### Production Checklist

- [ ] Change default passwords in `.env`
- [ ] Restrict database port externally (comment out port mapping)
- [ ] Use secrets management for sensitive data
- [ ] Enable firewall rules
- [ ] Regular security updates
- [ ] Monitor logs for suspicious activity

### Network Isolation

For production, remove external database port:

```yaml
# Comment out in docker-compose.enhanced.yml
# ports:
#   - "3306:3306"
```

## Maintenance

### Automatic Tasks

The database includes scheduled events:

- **Daily**: Calculate statistics at 1 AM
- **Weekly**: Optimize tables on Sunday at 2 AM

### Manual Maintenance

```bash
# Optimize tables manually
docker exec weewix-mariadb mysql -u root -p weewx -e "
  OPTIMIZE TABLE weather_observations;
  OPTIMIZE TABLE daily_statistics;
"

# Calculate daily stats manually
docker exec weewix-mariadb mysql -u root -p weewx -e "
  CALL calculate_daily_stats('home', CURDATE());
"
```

## Upgrading

```bash
# Pull latest images
docker-compose -f docker-compose.enhanced.yml pull

# Rebuild WeeWix
docker-compose -f docker-compose.enhanced.yml build --no-cache weewix

# Restart with new images
docker-compose -f docker-compose.enhanced.yml up -d
```

## Resources

- **Documentation**: See `docs/` directory
- **Configuration**: `config.example.toml`
- **Database Schema**: `scripts/db-init/01-create-schema.sql`
- **Environment Template**: `.env.example`

## Support

For issues and questions:
1. Check logs first
2. Review this documentation
3. Verify configuration in `.env`
4. Check GitHub issues
