use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

pub struct StompClient {
    stream: BufReader<TcpStream>,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub command: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Frame {
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }
    pub fn body_str(&self) -> &str {
        std::str::from_utf8(&self.body).unwrap_or("")
    }
}

fn write_frame(buf: &mut Vec<u8>, command: &str, headers: &[(&str, &str)], body: &[u8]) {
    buf.extend_from_slice(command.as_bytes());
    buf.push(b'\n');
    for (k, v) in headers {
        buf.extend_from_slice(k.as_bytes());
        buf.push(b':');
        buf.extend_from_slice(v.as_bytes());
        buf.push(b'\n');
    }
    if !body.is_empty() {
        let len = body.len().to_string();
        buf.extend_from_slice(b"content-length:");
        buf.extend_from_slice(len.as_bytes());
        buf.push(b'\n');
    }
    buf.push(b'\n');
    buf.extend_from_slice(body);
    buf.push(0);
}

impl StompClient {
    pub async fn connect(host: &str, port: u16, login: &str, passcode: &str) -> Result<Self> {
        let tcp = TcpStream::connect((host, port))
            .await
            .with_context(|| format!("tcp connect {host}:{port}"))?;
        tcp.set_nodelay(true)?;
        let mut client = Self { stream: BufReader::new(tcp) };

        let mut buf = Vec::new();
        write_frame(
            &mut buf,
            "CONNECT",
            &[
                ("accept-version", "1.2"),
                ("host", host),
                ("login", login),
                ("passcode", passcode),
                ("heart-beat", "10000,10000"),
            ],
            b"",
        );
        client.stream.get_mut().write_all(&buf).await?;

        let frame = client.read_frame().await?;
        if frame.command != "CONNECTED" {
            bail!("expected CONNECTED, got {}", frame.command);
        }
        Ok(client)
    }

    pub async fn subscribe(&mut self, id: &str, destination: &str, ack: &str) -> Result<()> {
        let mut buf = Vec::new();
        write_frame(
            &mut buf,
            "SUBSCRIBE",
            &[
                ("id", id),
                ("destination", destination),
                ("ack", ack),
            ],
            b"",
        );
        self.stream.get_mut().write_all(&buf).await?;
        Ok(())
    }

    pub async fn ack(&mut self, message_id: &str) -> Result<()> {
        let mut buf = Vec::new();
        write_frame(&mut buf, "ACK", &[("id", message_id)], b"");
        self.stream.get_mut().write_all(&buf).await?;
        Ok(())
    }

    pub async fn send(
        &mut self,
        destination: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<()> {
        let mut buf = Vec::new();
        write_frame(
            &mut buf,
            "SEND",
            &[
                ("destination", destination),
                ("content-type", content_type),
                ("persistent", "true"),
            ],
            body,
        );
        self.stream.get_mut().write_all(&buf).await?;
        self.stream.get_mut().flush().await?;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        let mut buf = Vec::new();
        write_frame(&mut buf, "DISCONNECT", &[("receipt", "bye")], b"");
        self.stream.get_mut().write_all(&buf).await?;
        Ok(())
    }

    pub async fn read_message(&mut self, wait: Duration) -> Result<Option<Frame>> {
        loop {
            let frame = match timeout(wait, self.read_frame()).await {
                Err(_) => return Ok(None),
                Ok(r) => r?,
            };
            match frame.command.as_str() {
                "MESSAGE" => return Ok(Some(frame)),
                "ERROR" => bail!(
                    "STOMP ERROR: {} body={}",
                    frame.header("message").unwrap_or(""),
                    frame.body_str()
                ),
                "RECEIPT" => continue,
                "" => continue, // heartbeat
                _ => continue,
            }
        }
    }

    async fn read_frame(&mut self) -> Result<Frame> {
        let mut command = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            let n = self.stream.read(&mut byte).await?;
            if n == 0 {
                bail!("connection closed");
            }
            if byte[0] == b'\n' {
                if command.is_empty() {
                    // empty line = heartbeat or pre-frame whitespace, keep reading
                    continue;
                }
                break;
            }
            if byte[0] == b'\r' {
                continue;
            }
            command.push(byte[0]);
        }
        let cmd_str = String::from_utf8(command).map_err(|e| anyhow!("bad cmd: {e}"))?;
        let mut headers = HashMap::new();
        loop {
            let mut line = Vec::new();
            loop {
                let mut byte = [0u8; 1];
                let n = self.stream.read(&mut byte).await?;
                if n == 0 {
                    bail!("connection closed reading headers");
                }
                if byte[0] == b'\n' {
                    break;
                }
                if byte[0] == b'\r' {
                    continue;
                }
                line.push(byte[0]);
            }
            if line.is_empty() {
                break;
            }
            let s = String::from_utf8(line).map_err(|e| anyhow!("bad header: {e}"))?;
            if let Some((k, v)) = s.split_once(':') {
                headers.entry(k.to_string()).or_insert_with(|| v.to_string());
            }
        }

        let body = if let Some(len) = headers.get("content-length") {
            let len: usize = len.parse().map_err(|_| anyhow!("bad content-length"))?;
            let mut buf = vec![0u8; len];
            self.stream.read_exact(&mut buf).await?;
            // consume trailing NULL
            let mut nul = [0u8; 1];
            let _ = self.stream.read(&mut nul).await?;
            buf
        } else {
            let mut buf = Vec::new();
            loop {
                let mut byte = [0u8; 1];
                let n = self.stream.read(&mut byte).await?;
                if n == 0 {
                    bail!("connection closed reading body");
                }
                if byte[0] == 0 {
                    break;
                }
                buf.push(byte[0]);
            }
            buf
        };

        Ok(Frame {
            command: cmd_str,
            headers,
            body,
        })
    }
}
