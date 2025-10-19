//! Packet buffering for interval management

use crate::{ArchiveError, ArchiveResult};
use weex_core::WeatherPacket;
use std::collections::VecDeque;

/// Buffer for collecting packets within an interval
pub struct PacketBuffer {
    interval: i32,
    packets: VecDeque<WeatherPacket>,
    current_interval_end: Option<i64>,
    max_packets: usize,
}

impl PacketBuffer {
    /// Create a new packet buffer with specified interval (seconds)
    pub fn new(interval: i32) -> Self {
        // Calculate max packets to prevent unbounded growth
        // Assume worst case: packet every second
        let max_packets = (interval as usize * 2).max(100);

        Self {
            interval,
            packets: VecDeque::with_capacity(max_packets),
            current_interval_end: None,
            max_packets,
        }
    }

    /// Add a packet to the buffer
    ///
    /// Returns Some(end_time) if the interval is complete and should be flushed
    pub fn add(&mut self, packet: WeatherPacket) -> ArchiveResult<Option<i64>> {
        if self.packets.len() >= self.max_packets {
            return Err(ArchiveError::BufferOverflow);
        }

        let packet_time = packet.date_time;

        // Determine interval end time
        let interval_end = match self.current_interval_end {
            Some(end) => end,
            None => {
                // First packet - calculate interval end
                let end = self.calculate_interval_end(packet_time);
                self.current_interval_end = Some(end);
                end
            }
        };

        // Check if packet belongs to current interval
        if packet_time <= interval_end {
            self.packets.push_back(packet);
            Ok(None)
        } else {
            // Packet is in next interval - current interval is complete
            self.packets.push_back(packet);
            let completed_interval = self.current_interval_end;
            self.current_interval_end = Some(self.calculate_interval_end(packet_time));
            Ok(completed_interval)
        }
    }

    /// Calculate interval end time for a given timestamp
    fn calculate_interval_end(&self, timestamp: i64) -> i64 {
        let interval = self.interval as i64;
        ((timestamp / interval) + 1) * interval
    }

    /// Drain all packets from buffer (for flushing)
    pub fn drain(&mut self) -> Vec<WeatherPacket> {
        let packets: Vec<_> = self.packets.drain(..).collect();
        self.current_interval_end = None;
        packets
    }

    /// Get current packet count
    pub fn len(&self) -> usize {
        self.packets.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    /// Get current interval end time
    pub fn interval_end(&self) -> Option<i64> {
        self.current_interval_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use weex_core::ObservationValue;

    fn make_packet(timestamp: i64) -> WeatherPacket {
        let mut observations = HashMap::new();
        observations.insert("outTemp".to_string(), ObservationValue::Float(25.0));

        WeatherPacket {
            date_time: timestamp,
            station: None,
            interval: None,
            observations,
        }
    }

    #[test]
    fn test_buffer_single_interval() {
        let mut buffer = PacketBuffer::new(300); // 5 minute intervals

        // Add packets within same interval
        let result = buffer.add(make_packet(100)).unwrap();
        assert!(result.is_none()); // No flush yet

        let result = buffer.add(make_packet(200)).unwrap();
        assert!(result.is_none()); // Still in same interval

        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_buffer_interval_boundary() {
        let mut buffer = PacketBuffer::new(300); // 5 minute intervals

        // Add packet at timestamp 100 (interval 0-300)
        let result = buffer.add(make_packet(100)).unwrap();
        assert!(result.is_none());

        // Add packet at timestamp 400 (crosses to next interval 300-600)
        let result = buffer.add(make_packet(400)).unwrap();
        assert_eq!(result, Some(300)); // First interval should flush at 300

        assert_eq!(buffer.len(), 2); // Both packets still in buffer until drain
    }

    #[test]
    fn test_buffer_drain() {
        let mut buffer = PacketBuffer::new(300);

        buffer.add(make_packet(100)).unwrap();
        buffer.add(make_packet(200)).unwrap();

        assert_eq!(buffer.len(), 2);

        let packets = buffer.drain();
        assert_eq!(packets.len(), 2);
        assert!(buffer.is_empty());
        assert_eq!(buffer.interval_end(), None);
    }

    #[test]
    fn test_calculate_interval_end() {
        let buffer = PacketBuffer::new(300);

        assert_eq!(buffer.calculate_interval_end(0), 300);
        assert_eq!(buffer.calculate_interval_end(100), 300);
        assert_eq!(buffer.calculate_interval_end(300), 600);
        assert_eq!(buffer.calculate_interval_end(301), 600);
        assert_eq!(buffer.calculate_interval_end(600), 900);
    }
}
