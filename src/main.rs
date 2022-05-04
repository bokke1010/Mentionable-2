use std::{env, sync::{Arc, Mutex}};


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
                ApplicationCommandInteractionDataOption,
            },
            Interaction,
            InteractionResponseType,
        },
        prelude::User,
    },
    prelude::*,
};


mod guild_commands;
use crate::guild_commands::guild_commands::add_all_application_commands;
mod database;
use database::data_access::Database;

struct Handler {
    db: Arc<Mutex<Database>>,
}

impl Handler {
    
    // async fn handle_join(&self, command: &ApplicationCommandInteraction) {
    //     // let options = command
    //     //     .data
    //     //     .options
    //     //     .get(0)
    //     //     .expect("Expected user option")
    //     //     .resolved
    //     //     .as_ref()
    //     //     .expect("Expected user object");
    
    //     // let response = {
    //     //     if let ApplicationCommandInteractionDataOptionValue::User(user, _member) =
    //     //         options
    //     //     {
    //     //         format!("{}'s id is {}", user.tag(), user.id)
    //     //     } else {
    //     //         "Please provide a valid user".to_string()
    //     //     }
    //     // };
    //     unsafe {
    //         TEST_VALUE+= 1;
    //     }

    //     if let Err(why) = command
    //         .create_interaction_response(&ctx.http, |response| {
    //             response
    //                 .kind(InteractionResponseType::ChannelMessageWithSource)
    //                 .interaction_response_data(|message| message.content(content))
    //         })
    //         .await
    //     {
    //         println!("Cannot respond to slash command: {}", why);
    //     }
    // }

    async fn handle_ping(&self, command: &ApplicationCommandInteraction) {

    }

    fn add_member(&self, guild_id: u64, list_id: u64, member_id: u64) {
        if let Ok(mut x) = self.db.clone().lock() {
            x.add_member(member_id, list_id).expect("Failed to add member to list");
        }
    }

    fn remove_member(&self, guild_id: u64, list_id: u64, member_id: u64) -> Result<bool, &str> {
        if let Ok(mut x) = self.db.clone().lock() {
            // Check membership...
            // return notmember
            if !x.has_member(member_id, list_id) {
                return Ok(false);
            }
            x.remove_member(member_id, list_id).expect("Failed to remove membership.");
            return Ok(true);
        }
        return Err("Failed to obtain database mutex.");
    }

    async fn handle_join(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.expect("No guild data found").0;
        let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        let list_ids: &Vec<ApplicationCommandInteractionDataOption> = &command.data.options;

        let mut content =  format!("Attempting to add user with id {} to {} lists:", member_id, list_ids.len());

        for list_id in list_ids {
            let parsed_list_id: u64 = list_id.value.as_ref().expect("No value given").as_u64().expect("Value was not a valid id");
            self.add_member(guild_id, parsed_list_id, member_id);
            content += format!("\nAdded to list {}", parsed_list_id).as_str();
        }
        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send leave response.");
    }

    async fn handle_leave(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.expect("No guild data found").0;
        let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        let list_ids: &Vec<ApplicationCommandInteractionDataOption> = &command.data.options;
        
        let mut content =  format!("Attempting to remove user with id {} from {} lists:", member_id, list_ids.len());
        
        for list_id in list_ids {
            let parsed_list_id: u64 = list_id.value.as_ref().expect("No value given").as_u64().expect("Value was not a valid id");
            if self.remove_member(guild_id, parsed_list_id, member_id).expect("Failed to remove member") {
                content += format!("\nRemoved from list {}", parsed_list_id).as_str();
            }
        }
        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send leave response.");
    }

    async fn handle_create(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.expect("No guild data found").0;
        let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        let list_name: &str = &command
            .data
            .options
            .get(0)
            .expect("No list name given")
            .value
            .as_ref()
            .expect("List name argument has no value")
            .as_str()
            .expect("list name is not a valid str.");
        
        let content =  format!("Creating list {}.", list_name);
        
        if let Ok(mut x) = self.db.clone().lock() {
            x.add_list(guild_id, list_name.to_string(), "".to_string()).expect("list creation failed");
        }

        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send leave response.");
    }

    async fn handle_invalid(&self, command: &ApplicationCommandInteraction) {

    }
}

#[async_trait]
impl EventHandler for Handler {


    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "ping" => self.handle_ping(&command).await,
                "join" => self.handle_join(&command, &ctx).await,
                "leave" => self.handle_leave(&command, &ctx).await,
                "create" => self.handle_create(&command, &ctx).await,
                // "get" => "Nope".to_string(),
                // "list" => "Nope".to_string(),
                // "propose" => "Nope".to_string(),
                // "list_proposals" => "Nope".to_string(),
                // // admin commandsw
                // "add" => "Nope".to_string(),
                // "kick" => "Nope".to_string(),
                // "rename" => "Nope".to_string(),
                // "configure" => "Nope".to_string(),
                _ => self.handle_invalid(&command).await,
            };
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        add_all_application_commands(&mut GuildId(466163515103641611), ctx).await;
    }
}

#[tokio::main]
async fn main() {
    // Load database
    let database: Database = Database::new("base".to_string()).expect("Database could not be loaded");
    let database = Arc::new(Mutex::new(database));


    // let args = env::args();
    // let mut wait_type = 0;
    // for (i, arg) in args.enumerate() {
    //     match wait_type {
    //         0 => {
    //             wait_type = match arg.as_str() {
    //                 "--import" => 1,
    //                 _ => 0,
    //             }
    //         },
    //         1 => {
    //             // database::data_access::new("base");
    //             wait_type = 0;
    //         },
    //         _ => wait_type = 0,
    //     }
    // }


    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // The Application Id is usually the Bot User Id.
    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    let handler = Handler{db: database};

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
