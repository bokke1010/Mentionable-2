use std::cmp::{max, min};
use std::{
    env,
    sync::{Arc, Mutex},
};

use serenity::model::prelude::message_component::MessageComponentInteraction;
use serenity::{
    async_trait,
    builder::{
        CreateActionRow, CreateButton, CreateEmbed, CreateSelectMenu, CreateSelectMenuOption,
    },
    model::{
        gateway::Ready,
        id::{GuildId, RoleId, UserId},
        interactions::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
            },
            autocomplete::AutocompleteInteraction,
            message_component::ButtonStyle,
            Interaction, InteractionResponseType,
        },
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
    fn can_manage_messages(command: &ApplicationCommandInteraction) -> bool {
        let member = command.member.as_ref().unwrap();
        return member
            .permissions
            .unwrap()
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);
    }

    async fn send_text(text: String, command: &ApplicationCommandInteraction, ctx: &Context) {
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content(text))
            })
            .await
            .expect("Failed to send text response.");
    }

    async fn handle_ping(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        let member = command.member.as_ref().unwrap();
        let member_admin = Handler::can_manage_messages(command);
        let role_ids: &Vec<RoleId> = &member.roles;

        let list_names: Vec<ApplicationCommandInteractionDataOption> = command.data.options.clone();
        let mut members: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
        let mut invalid_lists: Vec<String> = vec![];
        if let Ok(mut x) = self.db.clone().lock() {
            let mut override_canping = 0;
            let mut override_cooldown = -1;
            for role_id in role_ids {
                let (_, role_canping, role_cooldown) = x.get_role_permissions(guild_id, *role_id);
                override_canping = max(override_canping, role_canping);
                if override_cooldown * role_cooldown > 0 {
                    override_cooldown = min(override_cooldown, role_cooldown);
                } else {
                    override_cooldown = max(override_cooldown, role_cooldown)
                }
            }

            let (general_cooldown, general_canping, pingcooldown) = x.get_guild_ping_data(guild_id);

            if override_canping == 1 || (!general_canping && override_canping == 0) {
                //TODO: Send return message
                return;
            }

            for list_name in list_names {
                let list_name = list_name.value.unwrap();
                let list_name = list_name.as_str().unwrap();

                if let Ok(list_id) = x.get_list_id_by_name(list_name, guild_id) {
                    let (list_cooldown, _, list_restrict_ping) = x.get_list_permissions(list_id);
                    if list_restrict_ping && !member_admin {
                        //TODO: add message for restricted list.
                        continue;
                    }
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
        } else if invalid_lists.len() == 0 {
            content += "These lists are empty.";
        }
        for falselist in invalid_lists {
            content += format!("\nThe list {} does not exist.", falselist).as_str();
        }

        Handler::send_text(content, command, ctx).await;
    }

    async fn autocomplete_ping(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;
        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.focused {
                filter = field.value.as_ref().unwrap().as_str().unwrap();
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        if let Ok(mut x) = self.db.clone().lock() {
            aliases = x
                .get_list_aliases_by_search(guild_id, 0, SUGGESTIONS, filter)
                .unwrap();
        }
        autocomplete
            .create_autocomplete_response(&ctx.http, |response| {
                for list in aliases {
                    response.add_string_choice(&list, &list);
                }
                response
            })
            .await
            .unwrap();
    }

    fn add_member(&self, guild_id: GuildId, list_name: &str, member_id: UserId) -> bool {
        if let Ok(mut x) = self.db.clone().lock() {
            let res_list_id = x.get_list_id_by_name(list_name, guild_id);
            if let Ok(list_id) = res_list_id {
                x.add_member(member_id, list_id)
                    .expect("Failed to add member to list");
                return true;
            }
        }
        return false;
    }

    fn remove_member(
        &self,
        guild_id: GuildId,
        list_name: &str,
        member_id: UserId,
    ) -> Result<bool, &str> {
        if let Ok(mut x) = self.db.clone().lock() {
            // Check membership...
            let get_list_id = x.get_list_id_by_name(list_name, guild_id);
            if let Ok(list_id) = get_list_id {
                if !x.has_member(member_id, list_id) {
                    return Ok(false);
                }
                x.remove_member(member_id, list_id)
                    .expect("Failed to remove membership.");
                return Ok(true);
            }
            return Ok(false);
        }
        return Err("Failed to obtain database mutex.");
    }

    async fn handle_join(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        let member_id: UserId = command
            .member
            .as_ref()
            .expect("Interaction not triggered by a member")
            .user
            .id;
        let list_names: Vec<ApplicationCommandInteractionDataOption> = command.data.options.clone();

        let mut content = format!(
            "Attempting to add user with id {} to {} lists:",
            member_id,
            list_names.len()
        );

        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();
            if self.add_member(guild_id, list_name_str, member_id) {
                content += format!("\nAdded to list {}", list_name_str).as_str();
            } else {
                content += format!("\nFailed to add user to list {}", list_name_str).as_str();
            }
        }
        Handler::send_text(content, command, ctx).await;
    }

    async fn handle_leave(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        let member_id: UserId = command
            .member
            .as_ref()
            .expect("Interaction not triggered by a member")
            .user
            .id;
        let list_names: Vec<ApplicationCommandInteractionDataOption> = command.data.options.clone();

        let mut content = format!(
            "Attempting to remove user with id {} from {} lists:",
            member_id,
            list_names.len()
        );
        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();

            if self
                .remove_member(guild_id, list_name_str, member_id)
                .expect("Failed to remove member")
            {
                content += format!("\nRemoved from list {}", list_name_str).as_str();
            }
        }
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content(content))
            })
            .await
            .expect("Failed to send leave response.");
    }

    async fn handle_create(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
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

        let mut content = format!("Creating list {}.", list_name);

        if let Ok(mut x) = self.db.clone().lock() {
            match x.get_list_id_by_name(list_name, guild_id) {
                Ok(_) => content = "This list already exists.".to_string(),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    x.add_list(guild_id, &list_name.to_string(), "".to_string())
                        .expect("list creation failed");
                    ()
                }
                a => {
                    a.unwrap();
                }
            };
        }

        Handler::send_text(content, command, ctx).await;
    }

    async fn handle_alias(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
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
        let list_alias: &str = &command
            .data
            .options
            .get(1)
            .expect("No list alias given")
            .value
            .as_ref()
            .expect("List alias argument has no value")
            .as_str()
            .expect("list alias is not a valid str.");

        let mut content = String::new();
        if let Ok(mut x) = self.db.clone().lock() {
            let res_id = x.get_list_id_by_name(list_name, guild_id);

            if let Ok(id) = res_id {
                x.add_alias(id, list_alias).unwrap();
                content = format!("Added alias {} to list {}.", list_alias, list_name);
            } else if Err(rusqlite::Error::QueryReturnedNoRows) == res_id {
                content = format!("There is no list named {} to alias to.", list_alias);
            } else {
                res_id.unwrap();
            }
        }

        Handler::send_text(content, command, ctx).await;
    }

    async fn handle_get(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.unwrap();
        let member_id: UserId = command
            .member
            .as_ref()
            .expect("Interaction not triggered by a member")
            .user
            .id;
        let mut content = format!("You are in the following lists:");
        if let Ok(mut x) = self.db.clone().lock() {
            let list_ids = x.get_lists_with_member(guild_id, member_id).unwrap();
            for list_id in list_ids {
                let list_names = x.get_list_names_by_id(list_id, guild_id).unwrap();
                content += format!("\n{}", list_names.join(", ")).as_str();
            }
        }

        Handler::send_text(content, command, ctx).await;
    }

    fn compose_list(
        &self,
        guild_id: GuildId,
        page: i64,
        filter: String,
    ) -> (CreateEmbed, Option<CreateActionRow>) {
        let mut embed = CreateEmbed::default();
        const PAGESIZE: usize = 20;
        let succes: bool;
        let mut maxlists: usize = 0;
        let lists: Vec<structures::structures::PingList>;
        let mut labels: Vec<String> = Vec::new();
        let mut page_selection: (usize, usize) = (0, 0);
        let page_count: usize;
        let mut visible_lists: Vec<(String, String)> = Vec::new();

        if let Ok(mut x) = self.db.clone().lock() {
            maxlists = x.count_lists_by_search(guild_id, filter.as_str(), false);
            if maxlists > 0 {
                lists = x
                    .get_lists_by_search(guild_id, filter.as_str(), false)
                    .unwrap();
                if page >= 0 {
                    let page = page as usize;
                    page_count = 1 + (maxlists - 1) / PAGESIZE;
                    for page_index in 0..page_count {
                        let page_start = page_index * PAGESIZE;
                        let page_end = min(maxlists, (page_index + 1) * PAGESIZE - 1);
                        labels.push(format!("{}:{}", page_start + 1, page_end + 1));
                        if page_index == page {
                            for list_index in page_start..page_end {
                                page_selection = (page_start, page_end);
                                visible_lists.push((
                                    x.get_list_names_by_id(lists[list_index].id, guild_id)
                                        .unwrap()
                                        .join(", "),
                                    lists[list_index].description.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        if maxlists == 0 {
            embed.color((255, 0, 0));
            embed.title("No lists found.");
            succes = false;
        } else if page < 0 || PAGESIZE * (page as usize) >= maxlists {
            embed.color((255, 127, 28));
            embed.title("List page out of range.");
            succes = true;
        } else {
            embed.color((127, 255, 160));

            embed.title(format!(
                "Showing lists {}-{} out of {}:",
                page_selection.0 + 1,
                page_selection.1 + 1,
                maxlists
            ));
            for list in visible_lists {
                embed.field(
                    list.0,
                    if list.1.is_empty() {
                        "-".to_string()
                    } else {
                        list.1
                    },
                    false,
                );
            }
            succes = true;
        }

        if !succes {
            return (embed, None);
        }

        let mut select_menu_options: Vec<CreateSelectMenuOption> = Vec::new();
        for (i, label) in labels.iter().enumerate() {
            select_menu_options.push(CreateSelectMenuOption::new(label, i.to_string()));
        }
        let mut select_menu = CreateSelectMenu::default();
        select_menu
            .custom_id(if filter != "" {
                filter
            } else {
                "|".to_string()
            })
            .placeholder("Navigate between pages")
            .options(|options| options.set_options(select_menu_options));
        let mut action_row = CreateActionRow::default();
        action_row.add_select_menu(select_menu);
        return (embed, Some(action_row));
    }

    async fn handle_list(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.unwrap();
        let mut page: i64 = 0;
        let mut filter: String = "".to_string();
        for option in command.data.options.iter() {
            if option.name == "page" {
                page = option.value.as_ref().unwrap().as_i64().unwrap() - 1;
            } else if option.name == "filter" {
                let value = option.value.as_ref().unwrap();
                filter = value.as_str().unwrap().to_string();
            }
        }

        let (embed, action_row) = self.compose_list(guild_id, page, filter);

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.ephemeral(true).add_embed(embed);
                        if let Some(action_row) = action_row {
                            message.components(|c| c.add_action_row(action_row));
                        }
                        message
                    })
            })
            .await
            .unwrap();
    }

    async fn list_page_from_component(
        &self,
        component: &MessageComponentInteraction,
        ctx: &Context,
    ) {
        let page = component.data.values[0].parse::<i64>().unwrap_or(0);
        let guild_id = component.guild_id.unwrap();
        let filter = if component.data.custom_id == "|" {
            "".to_string()
        } else {
            component.data.custom_id.clone()
        };

        let (embed, _action_row) = self.compose_list(guild_id, page, filter);

        component.defer(&ctx).await.unwrap();
        component
            .edit_original_interaction_response(&ctx.http, |response| response.set_embed(embed))
            .await
            .unwrap();
    }

    async fn handle_invalid(&self, _command: &ApplicationCommandInteraction) {}

    async fn handle_propose(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.unwrap();
        let name = command.data.options[0].value.as_ref().unwrap().to_string();
        let proposal_id: u64;

        if let Ok(mut x) = self.db.clone().lock() {
            let timestamp = serenity::model::Timestamp::now().unix_timestamp();
            proposal_id = x
                .start_proposal(guild_id, &name, "".to_string(), timestamp)
                .unwrap();
        } else {
            return;
        }

        let mut embed = CreateEmbed::default();
        embed
            .title(format!("A new list has been proposed: {}", name))
            .author(|author| {
                author
                    .icon_url(command.user.avatar_url().unwrap())
                    .name(command.user.name.clone())
            })
            .color((31, 127, 255));

        let mut button = CreateButton::default();
        button
            .custom_id(proposal_id.to_string())
            .label("Vote")
            .style(ButtonStyle::Secondary);
        let mut action_row = CreateActionRow::default();
        action_row.add_button(button);

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.add_embed(embed);
                        message.components(|c| c.add_action_row(action_row));
                        message
                    })
            })
            .await
            .unwrap();
    }

    async fn handle_cancel_proposal(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        if !Handler::can_manage_messages(command) {
            return;
        }
        let guild_id = command.guild_id.unwrap();
        let proposal_name = command.data.options[0].value.as_ref().unwrap().to_string();
        if let Ok(mut x) = self.db.clone().lock() {
            let list_id = x.get_list_id_by_name(&proposal_name, guild_id).unwrap();
            x.remove_proposal(list_id).unwrap();
        }

        Handler::send_text("canceled proposal".to_string(), command, ctx).await;
    }

    fn check_proposal(&self, list_id: u64) {
        if let Ok(mut x) = self.db.clone().lock() {
            let votes = x.get_proposal_votes(list_id);
            let guild_id = x.get_list_guild(list_id).unwrap();
            let vote_threshold = x.get_vote_threshold(guild_id).unwrap();
            if votes > vote_threshold {
                println!("vote succesful");
            }
        }
    }

    async fn autocomplete_proposal(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;
        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.focused {
                filter = field.value.as_ref().unwrap().as_str().unwrap();
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        if let Ok(mut x) = self.db.clone().lock() {
            aliases = x
                .get_proposals_by_search(guild_id, 0, SUGGESTIONS, filter)
                .unwrap();
        }
        autocomplete
            .create_autocomplete_response(&ctx.http, |response| {
                for list in aliases {
                    response.add_string_choice(&list, &list);
                }
                response
            })
            .await
            .unwrap();
    }

    async fn propose_vote_from_component(
        &self,
        component: &MessageComponentInteraction,
        ctx: &Context,
    ) {
        let list_id = component.data.custom_id.parse::<u64>().unwrap();
        if let Ok(mut x) = self.db.clone().lock() {
            x.vote_proposal(list_id, component.user.id).unwrap();
        }
        self.check_proposal(list_id);
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
                "alias" => self.handle_alias(&command, &ctx).await,
                "propose" => self.handle_propose(&command, &ctx).await,
                // "list_proposals" => "Nope".to_string(),
                // // admin commandsw
                // "add" => "Nope".to_string(),
                // "kick" => "Nope".to_string(),
                // "rename" => "Nope".to_string(),
                // "configure" => "Nope".to_string(),
                "cancel_proposal" => self.handle_cancel_proposal(&command, &ctx).await,
                _ => self.handle_invalid(&command).await,
            };
        } else if let Interaction::Autocomplete(completable) = interaction {
            match completable.data.name.as_str() {
                "ping" => self.autocomplete_ping(&completable, &ctx).await,
                "cancel_proposal" => self.autocomplete_proposal(&completable, &ctx).await,
                _ => (),
            }
        } else if let Interaction::MessageComponent(component) = interaction {
            match component
                .message
                .interaction
                .as_ref()
                .unwrap()
                .name
                .as_str()
            {
                "list" => self.list_page_from_component(&component, &ctx).await,
                "propose" => self.propose_vote_from_component(&component, &ctx).await,
                _ => (),
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        println!(
            "{:?}",
            ready
                .guilds
                .clone()
                .into_iter()
                .map(|x| x.id.0)
                .collect::<Vec<u64>>()
        );
        if let Ok(mut x) = self.db.clone().lock() {
            for guild in ready.guilds {
                x.add_guild(guild.id).ok();
            }
        }
        // ApplicationCommand::set_global_application_commands(&ctx.http, |command| command).await.unwrap();

        add_all_application_commands(&mut GuildId(466163515103641611), ctx).await;
    }
}

#[tokio::main]
async fn main() {
    // Load database
    let database: Database =
        Database::new("database.db".to_string()).expect("Database could not be loaded");
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

    let handler = Handler { db: database };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;

    // Build our client.
    let mut client = Client::builder(token, intents)
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
