//! INTERCEPTOR UDP driver: receives WeatherPacket JSON over UDP

use crate::{IngestError, IngestResult, StationDriver};
use std::net::SocketAddr;
use tokio::{
    net::UdpSocket,
    time::{timeout, Duration},
};
use weex_core::WeatherPacket;

pub struct InterceptorUdpDriver {
    bind: SocketAddr,
    socket: Option<UdpSocket>,
    active: bool,
    recv_timeout: Duration,
}

impl InterceptorUdpDriver {
    pub fn new(bind: SocketAddr) -> Self {
        Self {
            bind,
            socket: None,
            active: false,
            recv_timeout: Duration::from_secs(5),
        }
    }

    fn socket_ref(&self) -> Result<&UdpSocket, IngestError> {
        self.socket
            .as_ref()
            .ok_or_else(|| IngestError::DriverError("socket not active".into()))
    }
}

#[async_trait::async_trait]
impl StationDriver for InterceptorUdpDriver {
    fn name(&self) -> &str {
        "interceptor-udp"
    }

    async fn start(&mut self) -> IngestResult<()> {
        if self.active {
            return Err(IngestError::DriverError("already started".into()));
        }
        let sock = UdpSocket::bind(self.bind)
            .await
            .map_err(|e| IngestError::CommunicationError(e.to_string()))?;
        // For tests and local runs, allow reuse
        sock.set_broadcast(true)
            .map_err(|e| IngestError::CommunicationError(e.to_string()))?;
        self.socket = Some(sock);
        self.active = true;
        Ok(())
    }

    async fn stop(&mut self) -> IngestResult<()> {
        self.active = false;
        self.socket = None;
        Ok(())
    }

    async fn get_packet(&mut self) -> IngestResult<WeatherPacket> {
        if !self.active {
            return Err(IngestError::DriverError("not active".into()));
        }
        let sock = self.socket_ref()?;
        let mut buf = vec![0u8; 2048];
        let (n, _peer) = timeout(self.recv_timeout, sock.recv_from(&mut buf))
            .await
            .map_err(|_| IngestError::Timeout)??;
        let slice = &buf[..n];
        let packet: WeatherPacket =
            serde_json::from_slice(slice).map_err(|e| IngestError::InvalidPacket(e.to_string()))?;
        Ok(packet)
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_interceptor_udp_roundtrip() {
        let bind = SocketAddr::from_str("127.0.0.1:0").unwrap();
        let mut driver = InterceptorUdpDriver::new(bind);
        driver.start().await.unwrap();
        let local = driver.socket.as_ref().unwrap().local_addr().unwrap();

        // Send a minimal WeatherPacket JSON
        let sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let json = r#"{
            "dateTime": 1700000000,
            "station": "interceptor",
            "interval": 5,
            "outTemp": 21.5
        }"#;
        sock.send_to(json.as_bytes(), local).await.unwrap();

        let pkt = driver.get_packet().await.unwrap();
        assert_eq!(pkt.date_time, 1700000000);
        assert_eq!(pkt.station.as_deref(), Some("interceptor"));
        assert_eq!(pkt.interval, Some(5));
        assert!(pkt.observations.contains_key("outTemp"));

        driver.stop().await.unwrap();
    }
}
