use futures::{SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec};

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Number {
    Int(u64),
    Float(f64),
}

#[derive(Deserialize)]
struct Request {
    method: String,
    number: Number,
}

#[derive(Serialize)]
struct Response {
    method: String,
    prime: bool,
}

pub async fn handle_is_prime(socket: &mut TcpStream) -> anyhow::Result<()> {
    let mut framed = Framed::new(socket, LinesCodec::new());
    while let Some(line) = framed.next().await.transpose()? {
        let req: Request = serde_json::from_str(&line)?;
        if req.method != "isPrime" {
            return Err(anyhow::anyhow!("method is not isPrime"));
        }
        let prime = match req.number {
            Number::Int(i) => is_prime(i),
            Number::Float(f) => {
                if f.fract() == 0.0 {
                    is_prime(f as u64)
                } else {
                    false
                }
            }
        };
        let response = Response {
            method: "isPrime".into(),
            prime,
        };
        let response_json = serde_json::to_string(&response)?;
        framed.send(response_json).await?;
    }
    Ok(())
}

fn is_prime(i: u64) -> bool {
    if i < 2 {
        return false;
    }
    let max: u64 = ((i as f64).sqrt() + 1f64) as u64;
    for n in 2..max {
        if i % n == 0 {
            return false;
        }
    }
    return true;
}

#[cfg(test)]
mod tests {

    use futures::{SinkExt, StreamExt};
    use serde_json::{Value, json};
    use tokio::net::{TcpListener, TcpStream};
    use tokio_util::codec::{Framed, LinesCodec};

    use crate::handle_is_prime::handle_is_prime;

    // Import the function from your main crate or define it here

    async fn run_test(input_number: serde_json::Value, expected: bool) -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            handle_is_prime(&mut socket).await.unwrap();
        });

        let mut client = Framed::new(TcpStream::connect(addr).await?, LinesCodec::new());

        let input = json!({
            "method": "isPrime",
            "number": input_number
        });

        client.send(input.to_string()).await?;

        let Some(line) = client.next().await else {
            panic!("No response received");
        };

        let line = line?;
        let response: Value = serde_json::from_str(&line)?;
        assert_eq!(response["method"], "isPrime");
        assert_eq!(response["prime"], expected);

        Ok(())
    }

    #[tokio::test]
    async fn test_known_primes() -> anyhow::Result<()> {
        run_test(json!(2), true).await?;
        run_test(json!(3), true).await?;
        run_test(json!(13), true).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_known_non_primes() -> anyhow::Result<()> {
        run_test(json!(1), false).await?;
        run_test(json!(8), false).await?;
        run_test(json!(12), false).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_float_input() -> anyhow::Result<()> {
        run_test(json!(4.5), false).await?;
        run_test(json!(5.0), true).await?;
        Ok(())
    }
}
