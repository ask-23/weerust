#!/bin/bash
# WeeWix Database Restore Script
# Restores compressed backup of MariaDB weather database

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/backup}"
DB_NAME="${DB_NAME:-weewx}"
DB_USER="${DB_USER:-root}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== WeeWix Database Restore ===${NC}"

# Check if backup file is provided
if [ $# -eq 0 ]; then
    echo -e "${YELLOW}Available backups:${NC}"
    ls -lh "${BACKUP_DIR}"/weewix_backup_*.sql.gz
    echo ""
    echo "Usage: $0 <backup_file.sql.gz>"
    exit 1
fi

BACKUP_FILE="$1"

# Validate backup file
if [ ! -f "${BACKUP_FILE}" ]; then
    echo -e "${RED}✗ Backup file not found: ${BACKUP_FILE}${NC}"
    exit 1
fi

echo "Backup file: ${BACKUP_FILE}"
BACKUP_SIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
echo "Backup size: ${BACKUP_SIZE}"

# Confirmation prompt
echo -e "${RED}WARNING: This will overwrite the current database!${NC}"
read -p "Are you sure you want to restore? (yes/no): " CONFIRM

if [ "${CONFIRM}" != "yes" ]; then
    echo "Restore cancelled."
    exit 0
fi

# Get current record count before restore
RECORDS_BEFORE=$(mysql -u "${DB_USER}" -p"${MARIADB_ROOT_PASSWORD}" "${DB_NAME}" -N -e \
    "SELECT COUNT(*) FROM weather_observations;" 2>/dev/null || echo "0")
echo "Current records: ${RECORDS_BEFORE}"

# Perform restore
echo -e "${YELLOW}Restoring database...${NC}"
if gunzip < "${BACKUP_FILE}" | mysql -u "${DB_USER}" -p"${MARIADB_ROOT_PASSWORD}" "${DB_NAME}"; then
    echo -e "${GREEN}✓ Database restored successfully${NC}"

    # Get record count after restore
    RECORDS_AFTER=$(mysql -u "${DB_USER}" -p"${MARIADB_ROOT_PASSWORD}" "${DB_NAME}" -N -e \
        "SELECT COUNT(*) FROM weather_observations;")
    echo "Restored records: ${RECORDS_AFTER}"

    # Log to database
    mysql -u "${DB_USER}" -p"${MARIADB_ROOT_PASSWORD}" "${DB_NAME}" -e "
        INSERT INTO system_log (level, component, message, metadata)
        VALUES ('INFO', 'restore', 'Database restored from backup',
                JSON_OBJECT(
                    'backup_file', '${BACKUP_FILE}',
                    'records_before', ${RECORDS_BEFORE},
                    'records_after', ${RECORDS_AFTER}
                ));
    "
else
    echo -e "${RED}✗ Restore failed${NC}"
    exit 1
fi

echo -e "${GREEN}=== Restore Complete ===${NC}"
