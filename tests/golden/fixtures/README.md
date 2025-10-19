# Golden Test Fixtures

This directory contains captured weather packet JSON from Python WeeWX for golden testing.

## Format

Each fixture file should be a JSON array of weather packets:

```json
[
  {
    "dateTime": 1234567890,
    "interval": 300,
    "outTemp": 25.5,
    "outHumidity": 65.0,
    "barometer": 1013.25,
    "windSpeed": 5.2,
    "windDir": 180,
    "rain": 0.0
  }
]
```

## Capturing Fixtures from Python WeeWX

To create fixtures from your Python WeeWX installation:

1. **Enable packet logging** in weewx.conf:
   ```ini
   [Engine]
       [[Services]]
           data_services = user.packetlogger.PacketLogger
   ```

2. **Create a packet logger service** (user/packetlogger.py):
   ```python
   import json
   import weewx
   from weewx.engine import StdService

   class PacketLogger(StdService):
       def __init__(self, engine, config_dict):
           super(PacketLogger, self).__init__(engine, config_dict)
           self.bind(weewx.NEW_LOOP_PACKET, self.new_loop_packet)
           self.packets = []

       def new_loop_packet(self, event):
           self.packets.append(dict(event.packet))
           if len(self.packets) >= 100:
               with open('/tmp/weewx_packets.json', 'w') as f:
                   json.dump(self.packets, f, indent=2)
               self.packets = []
   ```

3. **Copy the logged packets** to this directory

## Creating Test Scenarios

### Simple Packet Test
Single interval with basic observations:
- File: `simple_packet.json`
- Duration: 1 interval (5 minutes)
- Fields: temperature, humidity, pressure, wind

### Multi-Interval Test
Multiple intervals to test aggregation boundaries:
- File: `multi_interval.json`
- Duration: 3+ intervals (15+ minutes)
- Tests: interval transitions, data accumulation

### Edge Cases
- Missing data (nulls)
- Extreme values
- Rapid changes
- Sensor failures

## Running Tests

```bash
# Run all golden tests
cargo test --test golden_tests -- --ignored

# Run specific test
cargo test --test golden_tests test_simple_packet_processing -- --ignored

# Update baselines after verifying changes are correct
UPDATE_BASELINES=1 cargo test --test golden_tests -- --ignored
```
