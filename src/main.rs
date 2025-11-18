use std::{collections::HashMap, io::stdin, sync::mpsc::Sender};
use futures_lite::StreamExt;
use iroh::EndpointId;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use iroh::{Endpoint, SecretKey, endpoint, protocol::Router};
use iroh_gossip::{TopicId, api::{Event, GossipReceiver}, net::Gossip};

#[tokio::main]
async fn main() -> Result<()> {


    // This generates an endpoint that can receive a specific private key
    let endpoint = Endpoint::builder().bind().await?;

    //Generates a gossip protocol to manage communication from the endpoint
    let gossip = Gossip::builder().spawn(endpoint.clone());

    //Router manage protocol and how to handle incoming messages
    let router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    let id = TopicId::from_bytes(rand::random());
    let endpoint_ids = vec![];

    let topic = gossip.subscribe(id, endpoint_ids).await?;

    let (sender , receiver) = topic.split();

    let message = Message::new(MessageBody::AboutMe{ from: endpoint.id(), name: String::from("emopou"), });
    //convert the message from a vec to bytes
    sender.broadcast(message.to_vec().into()).await?;

    tokio::spawn(subscribe_loop(receiver));

    //Creates a thread for input reading
    let (line_tx, mut line_rx) = tokio::sync::mpsc::channel(1);

    std::thread::spawn(move || input_loop(line_tx));

    println!("> type a message...");

    //broadcast each line
    while let Some(text) = line_rx.recv().await{
        let message = Message::new(MessageBody::Message { from: endpoint.id(), text: text.clone() });

        sender.broadcast(message.to_vec().into()).await?;
        println!(">sent: {text}")
    }

    router.shutdown().await?;

    Ok(())
}

//Create the message with a nonce to assure unique ids
#[derive(Debug, Serialize,Deserialize)]
struct Message{
    body: MessageBody,
    nonce: [u8; 16],
}

//Create message enum with message structure
#[derive(Debug, Serialize,Deserialize)]
enum MessageBody {
    AboutMe { from: EndpointId, name: String},
    Message { from: EndpointId, text: String},
}

//Create implementation to make constructor and pass message to vector
impl Message {
    fn from_bytes(bytes: &[u8]) -> Result<Self>{
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn new(body: MessageBody) -> Self{
        Self{
            body,
            nonce: rand::random(),
        }
    }

    pub fn to_vec(&self) -> Vec<u8>{
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

//input function
fn input_loop(line_tx: tokio::sync::mpsc::Sender<String>) -> Result<()> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();

    //loop through the entire buffer
    loop{
        stdin.read_line(&mut buffer)?;

        //Send it over the channel
        line_tx.blocking_send(buffer.clone())?;
        buffer.clear();
    }

}

async fn subscribe_loop(mut receiver: GossipReceiver) -> Result<()>{
    let mut names = HashMap::new();

    while let Some(event) = receiver.try_next().await?{
        if let Event::Received(msg) = event{
            match Message::from_bytes(&msg.content)?.body {
                MessageBody::AboutMe { from, name } => {
                    names.insert(from, name.clone());
                    print!("> {} is known as {}", from.fmt_short(), name);
                }
                MessageBody::Message { from, text} => {
                    let name = names
                        .get(&from)
                        .map_or_else(|| from.fmt_short().to_string(), String::to_string);
                    println!("{}: {}", name, text);
                }
            }
        }
    }
    Ok(())
}
//crafted by @DaWaildhorse
