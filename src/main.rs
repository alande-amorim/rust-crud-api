use postgres::Error as PostgresError;
use postgres::{Client, NoTls};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[macro_use]
extern crate serde_derive;

// model user struct with id, name, email
#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
}

// DATABASE URL
const DB_URL: &str = "postgres://postgres:postgres@localhost:5432/crud-api";

const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND_RESPONSE: &str = "HTTP/1.1 404 OK\r\n\r\n";
const INTERNAL_SERVER_ERROR_RESPONSE: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

fn main() {
    if let Err(e) = set_database() {
        println!("Error: {}", e);
        return;
    }

    let listener = TcpListener::bind("0.0.0.0:8000".to_string()).unwrap();
    println!("Server listening on port 8000");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if request.starts_with("POST /users") => handle_post_request(r),
                r if request.starts_with("GET /users/") => handle_get_request(r),
                r if request.starts_with("GET /users") => handle_get_all_request(r),
                r if request.starts_with("PUT /users") => handle_put_request(r),
                r if request.starts_with("DELETE /users") => handle_delete_request(r),
                _ => (NOT_FOUND_RESPONSE.to_string(), "404 Not Found".to_string()),
            };

            stream
                .write_all(format!("{}{}", status_line, content).as_bytes())
                .unwrap();
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

fn handle_post_request(request: &str) -> (String, String) {
    match (
        get_user_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(user), Ok(mut client)) => {
            client
                .execute(
                    "INSERT INTO users (name, email) VALUES ($1, $2)",
                    &[&user.name, &user.email],
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "User created".to_string())
        }
        _ => (
            INTERNAL_SERVER_ERROR_RESPONSE.to_string(),
            "Error".to_string(),
        ),
    }
}

fn handle_get_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            match client.query_one("SELECT * FROM users where id = $1", &[&id]) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };

                    (
                        OK_RESPONSE.to_string(),
                        serde_json::to_string(&user).unwrap(),
                    )
                }
                _ => (NOT_FOUND_RESPONSE.to_string(), "User not found".to_string()),
            }
        }
        _ => (
            INTERNAL_SERVER_ERROR_RESPONSE.to_string(),
            "User not found".to_string(),
        ),
    }
}

fn handle_get_all_request(_request: &str) -> (String, String) {
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut users = Vec::new();
            for row in client.query("SELECT * FROM users", &[]).unwrap() {
                users.push(User {
                    id: row.get(0),
                    name: row.get(1),
                    email: row.get(2),
                });
            }

            (
                OK_RESPONSE.to_string(),
                serde_json::to_string(&users).unwrap(),
            )
        }
        _ => (
            INTERNAL_SERVER_ERROR_RESPONSE.to_string(),
            "Error".to_string(),
        ),
    }
}

fn handle_put_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        get_user_request_body(&request),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(user), Ok(mut client)) => {
            println!("{:#?}", user);

            client
                .execute(
                    "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                    &[&user.name, &user.email, &id],
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "User updated".to_string())
        }
        _ => (
            INTERNAL_SERVER_ERROR_RESPONSE.to_string(),
            "Error".to_string(),
        ),
    }
}

fn handle_delete_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(DB_URL, NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            let rows_affected = client
                .execute("DELETE FROM users WHERE id = $1", &[&id])
                .unwrap();

            if rows_affected == 0 {
                return (NOT_FOUND_RESPONSE.to_string(), "User not found".to_string());
            }

            (OK_RESPONSE.to_string(), "User has been removed".to_string())
        }
        _ => (
            INTERNAL_SERVER_ERROR_RESPONSE.to_string(),
            "Error".to_string(),
        ),
    }
}

fn set_database() -> Result<(), PostgresError> {
    let mut client = Client::connect(DB_URL, NoTls)?;
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL
        )",
    )?;
    Ok(())
}

fn get_id(request: &str) -> &str {
    request
        .split("/")
        .nth(2)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

// deserialize user from request body with the id
fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
