-- WeeWix Database Schema Initialization
-- MariaDB optimized for 600GB weather data archive
-- Supports Ecowitt GW1100 and compatible weather stations

USE weewx;

-- ============================================================================
-- Weather Observations Table - Primary data storage
-- ============================================================================
CREATE TABLE IF NOT EXISTS weather_observations (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- Station identification
    station_id VARCHAR(50) NOT NULL DEFAULT 'home',
    station_type VARCHAR(50),

    -- Timestamp (UTC and local)
    timestamp_utc DATETIME(3) NOT NULL,
    timestamp_local DATETIME(3),

    -- Temperature (Fahrenheit and Celsius)
    temp_f DECIMAL(5,2),
    temp_c DECIMAL(5,2),
    feels_like_f DECIMAL(5,2),

    -- Humidity
    humidity TINYINT UNSIGNED,
    dewpoint_f DECIMAL(5,2),

    -- Barometric Pressure (inches Hg)
    baro_abs_in DECIMAL(5,3),
    baro_rel_in DECIMAL(5,3),

    -- Wind
    wind_dir SMALLINT UNSIGNED,
    wind_speed_mph DECIMAL(5,2),
    wind_gust_mph DECIMAL(5,2),

    -- Solar & UV
    solar_radiation DECIMAL(7,2),
    uv_index TINYINT UNSIGNED,

    -- Precipitation (inches)
    rain_rate_in DECIMAL(6,3),
    rain_event_in DECIMAL(6,3),
    rain_hourly_in DECIMAL(6,3),
    rain_daily_in DECIMAL(6,3),
    rain_weekly_in DECIMAL(6,3),
    rain_monthly_in DECIMAL(6,3),
    rain_yearly_in DECIMAL(6,3),

    -- Additional sensors (if available)
    indoor_temp_f DECIMAL(5,2),
    indoor_humidity TINYINT UNSIGNED,

    -- Device metadata
    runtime_seconds INT UNSIGNED,
    heap_bytes INT UNSIGNED,
    software_type VARCHAR(100),
    model VARCHAR(50),
    frequency VARCHAR(10),

    -- Raw JSON for extensibility
    raw_json JSON,

    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- Indexes for performance
    INDEX idx_timestamp_utc (timestamp_utc),
    INDEX idx_station_timestamp (station_id, timestamp_utc),
    INDEX idx_created_at (created_at),
    INDEX idx_temp_humidity (temp_f, humidity),
    INDEX idx_wind (wind_speed_mph, wind_dir)

) ENGINE=InnoDB
  DEFAULT CHARSET=utf8mb4
  COLLATE=utf8mb4_unicode_ci
  ROW_FORMAT=COMPRESSED
  COMMENT='Primary weather observations from Ecowitt stations';

-- ============================================================================
-- Daily Statistics Table - Aggregated daily data for fast queries
-- ============================================================================
CREATE TABLE IF NOT EXISTS daily_statistics (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    station_id VARCHAR(50) NOT NULL,
    date_local DATE NOT NULL,

    -- Temperature stats
    temp_max_f DECIMAL(5,2),
    temp_min_f DECIMAL(5,2),
    temp_avg_f DECIMAL(5,2),

    -- Humidity stats
    humidity_max TINYINT UNSIGNED,
    humidity_min TINYINT UNSIGNED,
    humidity_avg DECIMAL(4,1),

    -- Wind stats
    wind_max_mph DECIMAL(5,2),
    wind_avg_mph DECIMAL(5,2),
    wind_dominant_dir SMALLINT UNSIGNED,

    -- Precipitation
    rain_total_in DECIMAL(6,3),

    -- Solar
    solar_max DECIMAL(7,2),
    uv_max TINYINT UNSIGNED,

    -- Record counts
    observation_count INT UNSIGNED,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY uk_station_date (station_id, date_local),
    INDEX idx_date_local (date_local)

) ENGINE=InnoDB
  DEFAULT CHARSET=utf8mb4
  COLLATE=utf8mb4_unicode_ci
  COMMENT='Daily aggregated weather statistics';

-- ============================================================================
-- System Log Table - Track application events and errors
-- ============================================================================
CREATE TABLE IF NOT EXISTS system_log (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    timestamp DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    level ENUM('DEBUG', 'INFO', 'WARN', 'ERROR', 'CRITICAL') NOT NULL,
    component VARCHAR(50),
    message TEXT,
    metadata JSON,

    INDEX idx_timestamp (timestamp),
    INDEX idx_level (level)

) ENGINE=InnoDB
  DEFAULT CHARSET=utf8mb4
  COLLATE=utf8mb4_unicode_ci
  COMMENT='Application logging and event tracking';

-- ============================================================================
-- Database Maintenance and Optimization Settings
-- ============================================================================

-- Enable innodb_file_per_table for better space management
SET GLOBAL innodb_file_per_table = 1;

-- Optimize for large datasets
SET GLOBAL innodb_stats_on_metadata = 0;

-- Grant privileges to weewx user
GRANT SELECT, INSERT, UPDATE, DELETE ON weewx.* TO 'weewx'@'%';
FLUSH PRIVILEGES;
