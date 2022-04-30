use std::env;

use serenity::{
    async_trait,
    model::{
        gateway::Ready,
        id::GuildId,
        interactions::{
            application_command::{
                ApplicationCommand,
                ApplicationCommandInteractionDataOptionValue,
                ApplicationCommandOptionType,
                ApplicationCommandInteraction,
            },
            Interaction,
            InteractionResponseType,
        },
        prelude::User,
    },
    prelude::*,
};


// use std::collections::HashMap;

mod guild_commands;
use crate::guild_commands::guild_commands::add_all_application_commands;
mod database;
// use crate::database::data_access;

static mut TEST_VALUE: i32 = 1;

struct Handler;

impl Handler {
    
    fn handle_join(&self, command: &ApplicationCommandInteraction) -> String {
        // let options = command
        //     .data
        //     .options
        //     .get(0)
        //     .expect("Expected user option")
        //     .resolved
        //     .as_ref()
        //     .expect("Expected user object");
    
        // let response = {
        //     if let ApplicationCommandInteractionDataOptionValue::User(user, _member) =
        //         options
        //     {
        //         format!("{}'s id is {}", user.tag(), user.id)
        //     } else {
        //         "Please provide a valid user".to_string()
        //     }        
        // };
        unsafe {
            TEST_VALUE+= 1;
            return TEST_VALUE.to_string();
        }
    }
}

#[async_trait]
impl EventHandler for Handler {

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            println!("{:#?}", command);
            let content = match command.data.name.as_str() {
                "ping" => "Hey, I'm alive!".to_string(),
                "join" => self.handle_join(&command),
                "leave" => "Nope".to_string(),
                "get" => "Nope".to_string(),
                "list" => "Nope".to_string(),
                "propose" => "Nope".to_string(),
                "list_proposals" => "Nope".to_string(),
                // admin commands
                "add" => "Nope".to_string(),
                "kick" => "Nope".to_string(),
                "rename" => "Nope".to_string(),
                "configure" => "Nope".to_string(),
                _ => "not implemented :(".to_string(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        // let commands = ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
        //     commands
        add_all_application_commands(&mut GuildId(466163515103641611), ctx).await;
        // let guild_command = add_all_application_commands(&mut GuildId(466163515103641611), ctx).await;
        // println!("I created the following guild command: {:#?}", guild_command);
    }
}

#[tokio::main]
async fn main() {
    // Load database
    let database: database::data_access::Database;
    let args = env::args();
    let mut wait_type = 0;
    for (i, arg) in args.enumerate() {
        match wait_type {
            0 => {
                wait_type = match arg {
                    "--import".to_string() => 1,
                    _ => 0,
                }
            },
            1 => {
                database::data_access::new("base");
                wait_type = 0;
            },
            _ => wait_type = 0,
        }
    }


    // Configure the client with your Discord bot token in the environment.
    let token = "ODQzMTgyMjI4MjM4NjMwOTIy.YKAIpA.0WFR4KQiyLlc0jLzzY52tfEz_Ps";
    // let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // The Application Id is usually the Bot User Id.
    // let application_id: u64 = env::var("APPLICATION_ID")
    //     .expect("Expected an application id in the environment")
    //     .parse()
    //     .expect("application id is not a valid id");
    let application_id: u64 = 843182228238630922;

    let handler = Handler;

    // Build our client.
    let mut client = Client::builder(token)
        .event_handler(handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
