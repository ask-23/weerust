-- WeeWix Stored Procedures and Functions
-- Database utilities for maintenance and analytics

USE weewx;

DELIMITER $$

-- ============================================================================
-- Procedure: Calculate daily statistics from observations
-- ============================================================================
CREATE PROCEDURE IF NOT EXISTS calculate_daily_stats(
    IN p_station_id VARCHAR(50),
    IN p_date DATE
)
BEGIN
    INSERT INTO daily_statistics (
        station_id,
        date_local,
        temp_max_f,
        temp_min_f,
        temp_avg_f,
        humidity_max,
        humidity_min,
        humidity_avg,
        wind_max_mph,
        wind_avg_mph,
        rain_total_in,
        solar_max,
        uv_max,
        observation_count
    )
    SELECT
        p_station_id,
        p_date,
        MAX(temp_f),
        MIN(temp_f),
        AVG(temp_f),
        MAX(humidity),
        MIN(humidity),
        AVG(humidity),
        MAX(wind_speed_mph),
        AVG(wind_speed_mph),
        MAX(rain_daily_in),
        MAX(solar_radiation),
        MAX(uv_index),
        COUNT(*)
    FROM weather_observations
    WHERE station_id = p_station_id
      AND DATE(timestamp_local) = p_date
    ON DUPLICATE KEY UPDATE
        temp_max_f = VALUES(temp_max_f),
        temp_min_f = VALUES(temp_min_f),
        temp_avg_f = VALUES(temp_avg_f),
        humidity_max = VALUES(humidity_max),
        humidity_min = VALUES(humidity_min),
        humidity_avg = VALUES(humidity_avg),
        wind_max_mph = VALUES(wind_max_mph),
        wind_avg_mph = VALUES(wind_avg_mph),
        rain_total_in = VALUES(rain_total_in),
        solar_max = VALUES(solar_max),
        uv_max = VALUES(uv_max),
        observation_count = VALUES(observation_count),
        updated_at = CURRENT_TIMESTAMP;
END$$

-- ============================================================================
-- Procedure: Archive old observations (for data retention management)
-- ============================================================================
CREATE PROCEDURE IF NOT EXISTS archive_old_observations(
    IN p_days_to_keep INT
)
BEGIN
    DECLARE v_cutoff_date DATETIME;
    DECLARE v_deleted_count INT;

    SET v_cutoff_date = DATE_SUB(NOW(), INTERVAL p_days_to_keep DAY);

    -- First ensure daily stats are calculated
    INSERT INTO system_log (level, component, message, metadata)
    VALUES ('INFO', 'archive', 'Starting archival process', JSON_OBJECT('cutoff_date', v_cutoff_date));

    -- Delete old observations
    DELETE FROM weather_observations
    WHERE timestamp_utc < v_cutoff_date;

    SET v_deleted_count = ROW_COUNT();

    -- Log completion
    INSERT INTO system_log (level, component, message, metadata)
    VALUES ('INFO', 'archive', 'Archival completed',
            JSON_OBJECT('deleted_count', v_deleted_count, 'cutoff_date', v_cutoff_date));

    -- Optimize tables
    OPTIMIZE TABLE weather_observations;
    OPTIMIZE TABLE daily_statistics;
END$$

-- ============================================================================
-- Function: Convert Fahrenheit to Celsius
-- ============================================================================
CREATE FUNCTION IF NOT EXISTS fahrenheit_to_celsius(temp_f DECIMAL(5,2))
RETURNS DECIMAL(5,2)
DETERMINISTIC
BEGIN
    RETURN (temp_f - 32) * 5 / 9;
END$$

-- ============================================================================
-- Function: Calculate heat index (feels like temperature)
-- ============================================================================
CREATE FUNCTION IF NOT EXISTS calculate_heat_index(temp_f DECIMAL(5,2), humidity TINYINT)
RETURNS DECIMAL(5,2)
DETERMINISTIC
BEGIN
    DECLARE hi DECIMAL(10,6);

    -- Simplified heat index formula (Rothfusz regression)
    IF temp_f >= 80 AND humidity >= 40 THEN
        SET hi = -42.379
            + 2.04901523 * temp_f
            + 10.14333127 * humidity
            - 0.22475541 * temp_f * humidity
            - 0.00683783 * temp_f * temp_f
            - 0.05481717 * humidity * humidity
            + 0.00122874 * temp_f * temp_f * humidity
            + 0.00085282 * temp_f * humidity * humidity
            - 0.00000199 * temp_f * temp_f * humidity * humidity;
        RETURN ROUND(hi, 2);
    ELSE
        RETURN temp_f;
    END IF;
END$$

DELIMITER ;

-- ============================================================================
-- Create scheduled events for automatic maintenance
-- ============================================================================

-- Enable event scheduler
SET GLOBAL event_scheduler = ON;

-- Daily stats calculation (runs at 1 AM)
CREATE EVENT IF NOT EXISTS daily_stats_calculation
ON SCHEDULE EVERY 1 DAY
STARTS (TIMESTAMP(CURRENT_DATE) + INTERVAL 1 DAY + INTERVAL 1 HOUR)
DO
    CALL calculate_daily_stats('home', DATE_SUB(CURDATE(), INTERVAL 1 DAY));

-- Weekly table optimization (runs Sunday at 2 AM)
CREATE EVENT IF NOT EXISTS weekly_optimization
ON SCHEDULE EVERY 1 WEEK
STARTS (TIMESTAMP(CURRENT_DATE) + INTERVAL (7 - WEEKDAY(CURRENT_DATE)) DAY + INTERVAL 2 HOUR)
DO
BEGIN
    OPTIMIZE TABLE weather_observations;
    OPTIMIZE TABLE daily_statistics;
    INSERT INTO system_log (level, component, message)
    VALUES ('INFO', 'maintenance', 'Weekly table optimization completed');
END;
