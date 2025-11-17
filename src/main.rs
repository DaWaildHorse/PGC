use std::io::{Read, Write, self, BufRead};
use std::net::{TcpListener, TcpStream};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};

fn send_encrypted(stream: &mut TcpStream, cipher: &Aes256Gcm, message: &str) -> std::io::Result<()> {
    let encrypted = encrypt_message(cipher, message)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    let length = encrypted.len() as u32;
    stream.write_all(&length.to_be_bytes())?;
    
    stream.write_all(&encrypted)?;
    stream.flush()?;
    
    Ok(())
}

fn encrypt_ses(cipher: &Aes256Gcm, message: &str) -> -> Result<Vec<u8>, String> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, message.as_bytes()).map_err(|e| format!("Encryption failed: {:?}", e))?;

    let mut result = Vec::new();
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

fn chat_set(stream: TcpStream) {
    let read_stream = stream.try_clone().expect("Failed to clone");
    let mut write_stream = stream;

    // Thread reads from TCP stream
    std::thread::spawn(move || {
        let mut buffer = [0; 1024];
        let mut read_stream = read_stream;

        loop {
            match read_stream.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    let message = String::from_utf8_lossy(&buffer[..n]);
                    println!("Them: {}", message.trim());
                }
                _ => {
                    println!("Connection closed");
                    break;
                }
            }
        }
    });
    
    // Main thread writes from keyboard
    let stdin = io::stdin();
    println!("Start chatting (Ctrl+C to exit):\n");
    
    for line in stdin.lock().lines() {
        match line {
            Ok(message) => {
                if let Err(e) = send_encrypted(&mut write_stream, &cipher, &message) {
                    println!("Failed to send message: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}

fn server_function(port: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    println!("Listening on port {}...", port);
    println!("Waiting for connection...\n");
    
    let (stream, addr) = listener.accept()?;
    println!("Connected to: {}\n", addr);
    
    chat_set(stream);
    Ok(())
}

fn client_function(address: &str) -> std::io::Result<()> {
    println!("Connecting to {}...", address);
    let stream = TcpStream::connect(address)?;
    println!("Connected!\n");
    
    chat_set(stream);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        println!("P2P Chat Application");
        println!("\nUsage:");
        println!("  Server mode: {} listen <port>", args[0]);
        println!("  Client mode: {} connect <ip:port>", args[0]);
        println!("\nExamples:");
        println!("  {} listen 8080", args[0]);
        println!("  {} connect 127.0.0.1:8080", args[0]);
        return;
    }
    
    match args[1].as_str() {
        "listen" => {
            let port = args.get(2).unwrap_or(&"8080".to_string()).clone();
            if let Err(e) = server_function(&port) {
                eprintln!("Server error: {}", e);
            }
        }
        "connect" => {
            if args.len() < 3 {
                println!("Need address!");
                println!("Example: {} connect 127.0.0.1:8080", args[0]);
                return;
            }
            if let Err(e) = client_function(&args[2]) {
                eprintln!("Connection error: {}", e);
            }
        }
        _ => {
            println!("Unknown command: '{}'", args[1]);
            println!("Use 'listen' or 'connect'");
        }
    }
}
