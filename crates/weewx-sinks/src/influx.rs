use anyhow::{anyhow, Result};
use reqwest::Client;
use weex_core::{ObservationValue, Sink, WeatherPacket};

pub struct InfluxSink {
    client: Client,
    base_url: String,
    org: String,
    bucket: String,
    token: String,
}

impl InfluxSink {
    pub fn new(base_url: String, org: String, bucket: String, token: String) -> Result<Self> {
        if base_url.is_empty() || org.is_empty() || bucket.is_empty() || token.is_empty() {
            return Err(anyhow!("invalid influx configuration"));
        }
        let client = Client::builder().build()?;
        Ok(Self {
            client,
            base_url,
            org,
            bucket,
            token,
        })
    }

    fn to_line_protocol(&self, packet: &WeatherPacket) -> String {
        // measurement name "weather"
        let mut tags: Vec<String> = Vec::new();
        if let Some(station) = &packet.station {
            tags.push(format!("station={}", station.replace(' ', "\\ "))); // basic escaping
        }
        let mut fields: Vec<String> = Vec::new();
        for (k, v) in &packet.observations {
            match v {
                ObservationValue::Float(f) => fields.push(format!("{}={}", k, f)),
                ObservationValue::Integer(i) => fields.push(format!("{}={}i", k, i)),
                _ => {}
            }
        }
        if let Some(iv) = packet.interval {
            fields.push(format!("interval={}i", iv));
        }
        let tags_str = if tags.is_empty() {
            String::new()
        } else {
            format!(",{}", tags.join(","))
        };
        let fields_str = fields.join(",");
        format!("weather{} {} {}", tags_str, fields_str, packet.date_time)
    }
}

#[async_trait::async_trait]
impl Sink for InfluxSink {
    async fn emit(&mut self, packet: &WeatherPacket) -> Result<()> {
        let line = self.to_line_protocol(packet);
        let url = format!(
            "{}/api/v2/write?org={}&bucket={}",
            self.base_url, self.org, self.bucket
        );
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(line)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("influx write failed: {} {}", status, text));
        }
        Ok(())
    }
}
