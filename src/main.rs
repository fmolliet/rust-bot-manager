mod commands;

use std::collections::HashSet;
use std::env;
use std::sync::Arc;

use dotenv;
use serenity::async_trait;
use serenity::framework::standard::macros::{ group, hook};
use serenity::framework::standard::{ StandardFramework};
use serenity::client::bridge::gateway::ShardManager;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{debug, error, info, instrument};


use crate::commands::ping::*;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}


struct Handler;

#[async_trait]
impl EventHandler for Handler {
    
    #[instrument(skip(self, _ctx))]
    async fn resume(&self, _ctx: Context, resume: ResumedEvent) {
        debug!("Resumed; trace: {:?}", resume.trace);
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        info!("Connected in {} servers", ready.guilds.len() );
    }
}

#[hook]
#[instrument]
async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!("Got command '{}' by user '{}'", command_name, msg.author.name);

    true
}


#[group]
#[commands(ping)]
struct General;

#[tokio::main]
#[instrument]
async fn main() {
    // Adicionando dot env para carregar arquivo .env local
    dotenv::dotenv().expect("Failed to load .env file");
    tracing_subscriber::fmt::init();
    
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    
    let http = Http::new(&token);
    
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };
    
    
    let framework =
        StandardFramework::new().configure(|c| c.owners(owners).prefix("!")).before(before).group(&GENERAL_GROUP);
    
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client =
        Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Err creating client");
    
    
        {
            let mut data = client.data.write().await;
            data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        }
    
        let shard_manager = client.shard_manager.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.expect("Could not register ctrl+c handler");
            shard_manager.lock().await.shutdown_all().await;
        });
    
    
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

