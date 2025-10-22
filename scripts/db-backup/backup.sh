#!/bin/bash
# WeeWix Database Backup Script
# Performs compressed backup of MariaDB weather database

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/backup}"
DB_NAME="${DB_NAME:-weewx}"
DB_USER="${DB_USER:-root}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/weewix_backup_${TIMESTAMP}.sql.gz"
RETENTION_DAYS="${RETENTION_DAYS:-30}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== WeeWix Database Backup ===${NC}"
echo "Timestamp: ${TIMESTAMP}"
echo "Database: ${DB_NAME}"
echo "Backup file: ${BACKUP_FILE}"

# Create backup directory if it doesn't exist
mkdir -p "${BACKUP_DIR}"

# Perform backup with compression
echo -e "${YELLOW}Creating backup...${NC}"
if mysqldump \
    --single-transaction \
    --quick \
    --lock-tables=false \
    --routines \
    --events \
    -u "${DB_USER}" \
    -p"${MARIADB_ROOT_PASSWORD}" \
    "${DB_NAME}" | gzip > "${BACKUP_FILE}"; then

    echo -e "${GREEN}✓ Backup created successfully${NC}"

    # Calculate backup size
    BACKUP_SIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
    echo "Backup size: ${BACKUP_SIZE}"

    # Calculate record counts
    RECORD_COUNT=$(mysql -u "${DB_USER}" -p"${MARIADB_ROOT_PASSWORD}" "${DB_NAME}" -N -e \
        "SELECT COUNT(*) FROM weather_observations;")
    echo "Records backed up: ${RECORD_COUNT}"
else
    echo -e "${RED}✗ Backup failed${NC}"
    exit 1
fi

# Clean up old backups
echo -e "${YELLOW}Cleaning up old backups (older than ${RETENTION_DAYS} days)...${NC}"
find "${BACKUP_DIR}" -name "weewix_backup_*.sql.gz" -mtime +${RETENTION_DAYS} -delete
REMAINING=$(find "${BACKUP_DIR}" -name "weewix_backup_*.sql.gz" | wc -l)
echo "Backups remaining: ${REMAINING}"

# Log to database
mysql -u "${DB_USER}" -p"${MARIADB_ROOT_PASSWORD}" "${DB_NAME}" -e "
    INSERT INTO system_log (level, component, message, metadata)
    VALUES ('INFO', 'backup', 'Database backup completed',
            JSON_OBJECT(
                'backup_file', '${BACKUP_FILE}',
                'size', '${BACKUP_SIZE}',
                'record_count', ${RECORD_COUNT}
            ));
"

echo -e "${GREEN}=== Backup Complete ===${NC}"
