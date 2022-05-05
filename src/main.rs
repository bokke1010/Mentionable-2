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

mod structures;
use structures::*;

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

    async fn handle_ping(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.expect("No guild data found").0;
        // let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        let list_names: Vec<ApplicationCommandInteractionDataOption> = command.data.options.clone();

        let mut members: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
        let mut invalid_lists: Vec<String> = vec![];

        if let Ok(mut x) = self.db.clone().lock() {
            for list_name in list_names {
                let list_name = list_name.value.unwrap();
                let list_name = list_name.as_str().unwrap();

                if let Ok(list_id) = x.get_list_id_by_name(list_name, guild_id) {
                    members.extend(x.get_members_in_list(guild_id, list_id).unwrap());
                } else {
                    invalid_lists.push(list_name.to_string());
                }
            }
        }

        let mut content = String::new();
        if members.len() > 0 {
            content = format!("Mentioning {} members:\n", members.len());
            for member in members {
                content += format!("<@{}>, ", member).as_str();
            }
        }
        for falselist in invalid_lists {
            content += format!("\nThe list {} does not exist.", falselist).as_str();
        }

        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send ping response.");
    }

    fn add_member(&self, guild_id: u64, list_name: &str, member_id: u64) {
        if let Ok(mut x) = self.db.clone().lock() {
            let list_id = x.get_list_id_by_name(list_name, guild_id).unwrap();
            x.add_member(member_id, list_id).expect("Failed to add member to list");
        }
    }

    fn remove_member(&self, guild_id: u64, list_name: &str, member_id: u64) -> Result<bool, &str> {
        if let Ok(mut x) = self.db.clone().lock() {
            // Check membership...
            let get_list_id = x.get_list_id_by_name(list_name, guild_id);
            if let Ok(list_id) = get_list_id {
                if !x.has_member(member_id, list_id) {
                    return Ok(false);
                }
                x.remove_member(member_id, list_id).expect("Failed to remove membership.");
                return Ok(true);
            }
            return Ok(false);
        }
        return Err("Failed to obtain database mutex.");
    }

    async fn handle_join(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.expect("No guild data found").0;
        let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        let list_names: Vec<ApplicationCommandInteractionDataOption> = command.data.options.clone();

        let mut content =  format!("Attempting to add user with id {} to {} lists:", member_id, list_names.len());

        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();
            
            self.add_member(guild_id, list_name_str, member_id);
            content += format!("\nAdded to list {}", list_name_str).as_str();
        }
        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send join response.");
    }

    async fn handle_leave(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.expect("No guild data found").0;
        let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        let list_names: Vec<ApplicationCommandInteractionDataOption> = command.data.options.clone();
        
        let mut content = format!("Attempting to remove user with id {} from {} lists:", member_id, list_names.len());

        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();
            
            if self.remove_member(guild_id, list_name_str, member_id).expect("Failed to remove member") {
                content += format!("\nRemoved from list {}", list_name_str).as_str();
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

        let mut content =  format!("Creating list {}.", list_name);
        
        if let Ok(mut x) = self.db.clone().lock() {
            if !x.list_exists(guild_id, list_name) {
                x.add_list(guild_id, list_name.to_string(), "".to_string()).expect("list creation failed");
            } else {
                content = "This list already exists.".to_string();
            }
        }

        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send leave response.");
    }

    async fn handle_get(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.unwrap().0;
        let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        
        let mut content = format!("You are in the following lists:");
        if let Ok(mut x) = self.db.clone().lock() {
            let list_ids = x.get_lists_with_member(guild_id, member_id).unwrap();
            for list_id in list_ids {
                let list_name = x.get_list_name_by_id(list_id, guild_id).unwrap();
                content += format!("\n{}", list_name).as_str();
            }
        }

        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send get response.");
    }

    async fn handle_list(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: u64 = command.guild_id.unwrap().0;
        let member_id: u64 = command.member.as_ref().expect("Interaction not triggered by a member").user.id.0;
        
        let mut page: usize = 0;
        let mut filter: String = "".to_string();
        for option in command.data.options.clone() {
            if option.name == "page" {
                page = (option.value.unwrap().as_i64().unwrap() - 1) as usize;
            } else if option.name == "filter" {
                let value = option.value.unwrap();
                filter = value.as_str().unwrap().to_string();
            }
        }

        let mut content = String::new();
        if let Ok(mut x) = self.db.clone().lock() {
            let lists = x.get_lists_by_search(guild_id, page * 20, 20, filter.as_str()).unwrap();
            if lists.len() == 0 {
                content = "No lists found in this range.".to_string();
            } else {
                content = format!("Showing lists {}-{}:", page*20 + 1, page*20 + lists.len());
                for list in lists {
                    content += format!("\n{}", list.name).as_str();
                }
            }
        }

        command.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await.expect("Failed to send get response.");
    }

    async fn handle_invalid(&self, _command: &ApplicationCommandInteraction) {

    }

}

#[async_trait]
impl EventHandler for Handler {


    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "ping" => self.handle_ping(&command, &ctx).await,
                "join" => self.handle_join(&command, &ctx).await,
                "leave" => self.handle_leave(&command, &ctx).await,
                "create" => self.handle_create(&command, &ctx).await,
                "get" => self.handle_get(&command, &ctx).await,
                "list" => self.handle_list(&command, &ctx).await,
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


        println!("{:?}", ready.guilds.clone().into_iter().map(|x| x.id().0).collect::<Vec<u64>>());
        if let Ok(mut x) = self.db.clone().lock() {
            for guild in ready.guilds {
                x.add_guild(guild.id().0).ok();
            }
        }
        // ApplicationCommand::set_global_application_commands(&ctx.http, |command| command).await.unwrap();

        add_all_application_commands(&mut GuildId(466163515103641611), ctx).await;
    }
}

#[tokio::main]
async fn main() {
    // Load database
    let database: Database = Database::new("database.db".to_string()).expect("Database could not be loaded");
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
