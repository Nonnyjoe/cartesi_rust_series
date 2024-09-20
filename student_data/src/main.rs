use hex;
use json::{object, parse, JsonValue};
use std::env;

#[derive(Clone)]
pub struct Student {
    name: String,
    age: u8,
    wallet_address: String,
    attendance_count: u128,
}

impl Student {
    fn new(name: String, age: u8, wallet_address: String) -> Self {
        Student {
            name,
            age,
            wallet_address,
            attendance_count: 0,
        }
    }

    fn take_attendance(&mut self) {
        self.attendance_count += 1;
    }
}

pub async fn handle_advance(
    _client: &hyper::Client<hyper::client::HttpConnector>,
    _server_addr: &str,
    request: JsonValue,
    all_students: &mut Vec<Student>,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    println!("Received advance request data {}", &request);
    let _payload = request["data"]["payload"]
        .as_str()
        .ok_or("Missing payload")?;

    // TODO: add application logic here
    println!("payload is: {}", _payload);

    let msg_sender: &str = request["data"]["metadata"]["msg_sender"]
        .as_str()
        .ok_or("Missing caller")?;

    // convert hex to string
    let bytes = hex::decode(&_payload[2..]).expect("decoding failed");
    let string_data = String::from_utf8(bytes).expect("Invalid UTF-8 sequence");
    println!("Decoded string: {}", string_data);

    // convert string to json
    let json_data = parse(&string_data).expect("Parse failed");

    if let JsonValue::Object(object) = json_data.clone() {
        let method = object
            .get("method")
            .expect("method not found")
            .as_str()
            .expect("conversion to str failed");
        match method {
            "create" => create_student(&json_data, all_students),
            "delete" => delete_student(&json_data, all_students),
            "sign_attendance" => {
                sign_attendance(msg_sender.to_string().to_lowercase(), all_students);
            }
            _ => {
                println!("Unknown method");
            }
        }
    }

    Ok("accept")
}

fn create_student(payload: &JsonValue, all_students: &mut Vec<Student>) {
    let wallet_address = payload["wallet_address"]
        .as_str()
        .unwrap()
        .to_string()
        .to_lowercase();
    let name = payload["name"].as_str().unwrap().to_string();
    let age = payload["age"].as_u64().unwrap() as u8;

    let student = Student::new(name, age, wallet_address);
    all_students.push(student);
}

fn delete_student(payload: &JsonValue, all_students: &mut Vec<Student>) {
    let wallet_address = payload["wallet_address"].as_str().unwrap().to_string();
    let index = all_students
        .iter()
        .position(|s| s.wallet_address == wallet_address.to_lowercase());
    if let Some(index) = index {
        all_students.remove(index);
    } else {
        println!("Wallet Address not found");
    }
}

fn sign_attendance(wallet_address: String, all_students: &mut Vec<Student>) {
    let index = all_students
        .iter_mut()
        .position(|s| s.wallet_address == wallet_address.to_lowercase());
    if let Some(index) = index {
        all_students[index].take_attendance();
    } else {
        println!("Wallet Address not found");
    }
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

    let mut all_students: Vec<Student> = vec![];

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
                "advance_state" => {
                    handle_advance(&client, &server_addr[..], req, &mut all_students).await?
                }
                "inspect_state" => handle_inspect(&client, &server_addr[..], req).await?,
                &_ => {
                    eprintln!("Unknown request type");
                    "reject"
                }
            };
        }
    }
}
