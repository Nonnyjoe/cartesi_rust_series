use hex;
use hyper::Response;
use json::{object, parse, JsonValue};
use std::{env, fmt::format};

pub async fn handle_advance(
    _client: &hyper::Client<hyper::client::HttpConnector>,
    _server_addr: &str,
    request: JsonValue,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    println!("Received advance request data {}", &request);
    let _payload = request["data"]["payload"]
        .as_str()
        .ok_or("Missing payload")?;
    // TODO: add application logic here
    println!("payload is: {}", _payload);
    // convert hex to string
    let bytes = hex::decode(&_payload[2..]).expect("Decoding failed");
    let string_data = String::from_utf8(bytes).expect("Invalid UTF-8 sequence");
    println!("Decoded string: {}", string_data);

    // convert string to json
    let json_data = parse(&string_data).expect("Parse failed");
    let method = json_data["method"]
        .as_str()
        .expect("failed to unwrap method");
    println!("Method is: {}", method);

    let value_1 = json_data["value_1"]
        .as_f64()
        .expect("failed to unwrap value 1");

    let value_2 = json_data["value_2"]
        .as_f64()
        .expect("failed to unwrap value 2");
    println!(
        "method: {}, value_1: {}, value_2: {}",
        method, value_1, value_2
    );
    let mut result: Option<f64> = None;

    match method {
        "add" => result = Some(value_1 + value_2),
        "sub" => result = Some(value_1 - value_2),
        "div" => result = Some(value_1 / value_2),
        "mul" => result = Some(value_1 * value_2),
        _ => {
            println!("Unknown method");
        }
    }

    println!("result is {:?}", result.expect("Method not executed"));

    // convert float to hex representation
    let hex_value = format!("0x{}", hex::encode(result.unwrap().to_string()));
    println!("hex_value is {:?}", hex_value);

    let response = object! {
        "payload" => format!("{}", hex_value)
    };

    // Send out a notice with the result
    let request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .uri(format!("{}/notice", &_server_addr))
        .body(hyper::Body::from(response.dump()))?;
    let response = _client.request(request).await?;
    println!("Notice sending status {}", response.status());

    Ok("accept")
}

pub async fn handle_inspect(
    _client: &hyper::Client<hyper::client::HttpConnector>,
    _server_addr: &str,
    request: JsonValue,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    println!("Received inspect request data {}", &request);
    let _payload = request["data"]["payload"]
        .as_str()
        .ok_or("Missing payload")?;
    // TODO: add application logic here
    Ok("accept")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let mut status = "accept";
    loop {
        println!("Sending finish");
        let response = object! {"status" => status.clone()};
        let request = hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(format!("{}/finish", &server_addr))
            .body(hyper::Body::from(response.dump()))?;
        let response = client.request(request).await?;
        println!("Received finish status {}", response.status());

        if response.status() == hyper::StatusCode::ACCEPTED {
            println!("No pending rollup request, trying again");
        } else {
            let body = hyper::body::to_bytes(response).await?;
            let utf = std::str::from_utf8(&body)?;
            let req = json::parse(utf)?;

            let request_type = req["request_type"]
                .as_str()
                .ok_or("request_type is not a string")?;
            status = match request_type {
                "advance_state" => handle_advance(&client, &server_addr[..], req).await?,
                "inspect_state" => handle_inspect(&client, &server_addr[..], req).await?,
                &_ => {
                    eprintln!("Unknown request type");
                    "reject"
                }
            };
        }
    }
}
