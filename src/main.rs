use serenity::{
    async_trait,
    builder::{
        CreateActionRow, CreateButton, CreateEmbed, CreateSelectMenu, CreateSelectMenuOption,
    },
    futures::StreamExt,
    model::{
        application::{
            command::CommandOptionType,
            component::{ActionRow, ActionRowComponent, ButtonStyle, InputTextStyle},
            interaction::{
                application_command::{
                    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
                },
                autocomplete::AutocompleteInteraction,
                message_component::MessageComponentInteraction,
                Interaction, InteractionResponseType,
            },
        },
        gateway::Ready,
        guild::Member,
        id::{ChannelId, GuildId, MessageId, RoleId, UserId},
        user::User,
    },
    prelude::*,
};

use std::{
    cmp::min,
    collections::BTreeSet,
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use dotenv::dotenv;

mod structures;
use structures::{JoinResult, ListId, ProposalStatus, LOGCONDITION, LOGTRIGGER, PERMISSION};

mod guild_commands;

mod database;
use database::Database;

mod pickle_import;

struct DB;
struct BotData {
    database: Arc<Mutex<Database>>,
    global: std::collections::HashMap<GuildId, u64>,
    local: std::collections::HashMap<ListId, u64>,
}

impl TypeMapKey for DB {
    type Value = BotData;
}

enum ListInvalidReasons {
    OnLocalCooldown,
    OnGlobalCooldown,
    GuildRestrictPing,
    ChannelRestrictPing,
    ListRestrictPing,
    DoesNotExist,
    RoleRestrictPing,
}

/*
enum CommandInvalidReasons {
    ChannelRestriction,
    GuildRestriction,
    RoleRestriction,
    DoesNotExist,
}
*/

struct Handler {
    is_loop_running: AtomicBool,
}

impl Handler {
    fn can_manage_messages(command: &ApplicationCommandInteraction) -> bool {
        let member = command
            .member
            .as_ref()
            .expect("Command not called from guild, permissions not applicable");
        return member
            .permissions
            .expect("This member reference has to be aquired from an interaction")
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);
    }

    async fn send_channel(
        text: &str,
        channel_id: ChannelId,
        ctx: &Context,
        ephemeral: bool,
        reference: Option<serenity::model::prelude::MessageReference>,
    ) {
        channel_id
            .send_message(&ctx.http, |message| {
                let flags = if ephemeral {
                    serenity::model::prelude::MessageFlags::EPHEMERAL
                } else {
                    serenity::model::prelude::MessageFlags::empty()
                };
                message.content(text).flags(flags);
                if let Some(reference) = reference {
                    message.reference_message(reference);
                }
                message
            })
            .await
            .expect("Failed to send text response, see error for details.");
    }

    async fn send_text(
        text: &str,
        command: &ApplicationCommandInteraction,
        ctx: &Context,
        ephemeral: bool,
    ) {
        // Can fail if message is too long.
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content(text).ephemeral(ephemeral))
            })
            .await
            .expect("Failed to send text response, see error for details.");
    }

    async fn send_not_allowed(command: &ApplicationCommandInteraction, ctx: &Context) {
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .content("You do not have permission to use this command")
                            .ephemeral(true)
                    })
            })
            .await
            .expect("Failed to send text response.");
    }

    async fn handle_ping(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        let channel_id = command.channel_id;
        let member = command
            .member
            .as_ref()
            .expect("/ping not called from guild");
        let member_admin = Handler::can_manage_messages(command);
        let role_ids: &Vec<RoleId> = &member.roles;

        let list_names: Vec<CommandDataOption> = command.data.options.clone();
        let mut list_ids: Vec<ListId> = vec![];
        let mut members: BTreeSet<UserId> = BTreeSet::new();
        let mut invalid_lists: Vec<(String, ListInvalidReasons)> = vec![];

        let mut data = ctx.data.write().await;
        let BotData {
            database: db,
            global,
            local,
        } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");
        let timestamp = serenity::model::Timestamp::now().unix_timestamp() as u64;
        if let Ok(mut x) = db.clone().lock() {
            let mut override_canping = if member_admin {
                PERMISSION::ALLOW
            } else {
                PERMISSION::NEUTRAL
            };
            let mut ignore_cooldown = member_admin;

            let (_, user_canping, user_ignore_cooldown) =
                x.get_user_permissions(guild_id, member.user.id);
            ignore_cooldown = ignore_cooldown || user_ignore_cooldown;
            override_canping = override_canping.combine(user_canping);

            for role_id in role_ids {
                let (_, role_canping, role_ignore_cooldown) =
                    x.get_role_permissions(guild_id, *role_id);
                override_canping = override_canping.combine(role_canping);
                ignore_cooldown = ignore_cooldown || role_ignore_cooldown;
            }

            let (general_cooldown, general_canping, pingcooldown) = x.get_guild_ping_data(guild_id);
            let (_, channel_ping_rule, _) = x.get_channel_permissions(guild_id, channel_id);

            if override_canping == PERMISSION::DENY {
                invalid_lists.push(("all".to_string(), ListInvalidReasons::RoleRestrictPing));
            } else if override_canping == PERMISSION::NEUTRAL {
                if !general_canping {
                    invalid_lists.push(("all".to_string(), ListInvalidReasons::GuildRestrictPing));
                } else if channel_ping_rule == PERMISSION::DENY {
                    invalid_lists
                        .push(("all".to_string(), ListInvalidReasons::ChannelRestrictPing));
                }
            }

            let last_global = global.entry(guild_id).or_insert(0);

            if !ignore_cooldown && *last_global + general_cooldown >= timestamp {
                invalid_lists.push(("all".to_string(), ListInvalidReasons::OnGlobalCooldown));
            }

            for list_name in &list_names {
                let list_name = list_name
                    .value
                    .as_ref()
                    .expect("Invalid /ping definition, should not be subcommand");
                let list_name = list_name
                    .as_str()
                    .expect("Invalid /ping definition, should be string type");

                if let Some(list_id) = x.get_list_id_by_name(list_name, guild_id) {
                    let last_time = local.entry(list_id).or_insert(0);
                    let (mut list_cooldown, _, list_ping_permission) =
                        x.get_list_permissions(list_id);
                    if list_cooldown == -1 {
                        list_cooldown = pingcooldown as i64;
                    }

                    if list_ping_permission == PERMISSION::DENY && !member_admin {
                        invalid_lists
                            .push((list_name.to_string(), ListInvalidReasons::ListRestrictPing));
                        continue;
                    }

                    if !member_admin && *last_time + (list_cooldown as u64) >= timestamp {
                        invalid_lists
                            .push((list_name.to_string(), ListInvalidReasons::OnLocalCooldown));
                        continue;
                    }

                    members.extend(x.get_members_in_list(list_id));
                    list_ids.push(list_id);
                } else {
                    invalid_lists.push((list_name.to_string(), ListInvalidReasons::DoesNotExist));
                }
            }
        }

        // I hate this, but it should work well enough...
        let present_ids: BTreeSet<UserId> = BTreeSet::from_iter(
            command
                .guild_id
                .unwrap()
                .members_iter(&ctx.http)
                .map(|member| member.expect("error fetching members").user.id)
                .collect::<Vec<UserId>>()
                .await,
        );

        let members: Vec<&UserId> = members.intersection(&present_ids).collect();

        let mut first_message = true;

        let mut ephemeral = false;
        let mut content = String::new();
        if invalid_lists.len() == 0 {
            global.insert(guild_id, timestamp);
            for list_id in list_ids {
                local.insert(list_id, timestamp);
            }

            if members.len() > 0 {
                content = format!("Mentioning ");
                let mut i = 0;
                for list_name in &list_names {
                    let list_name = list_name
                        .value
                        .as_ref()
                        .expect("Invalid /ping definition, should not be subcommand");
                    let list_name = list_name
                        .as_str()
                        .expect("Invalid /ping definition, should be string type");
                    content += list_name;
                    i += 1;
                    if i < list_names.len() {
                        content += ", ";
                    }
                }
                content += format!(" with {} members:\n", members.len()).as_str();
                for member in members {
                    content += format!("<@{}>, ", member).as_str();
                    if content.len() > 1940 {
                        if first_message {
                            Handler::send_text(&content, command, ctx, false).await;
                            first_message = false;
                        } else {
                            Handler::send_channel(&content, command.channel_id, ctx, false, None)
                                .await;
                        }
                        content.clear();
                    }
                }
            } else {
                content += "These lists are empty.";
            }
        } else {
            ephemeral = true;
            for falselist in invalid_lists {
                content += match falselist.1 {
                    ListInvalidReasons::ChannelRestrictPing => {
                        format!("\nPings are not allowed in this channel.")
                    }
                    ListInvalidReasons::DoesNotExist => {
                        format!("\nThe list {} does not exist.", falselist.0)
                    }
                    ListInvalidReasons::GuildRestrictPing => {
                        format!("\nYou do not have permission to ping in this server.")
                    }
                    ListInvalidReasons::ListRestrictPing => {
                        format!("\nThe list {} cannot be pinged.", falselist.0)
                    }
                    ListInvalidReasons::OnGlobalCooldown => {
                        format!("\nAnother ping has happed recently, please try again later.")
                    }
                    ListInvalidReasons::OnLocalCooldown => {
                        format!("\nThe list {} has been pinged recently, please try again later or exclude this list.", falselist.0)
                    }
                    ListInvalidReasons::RoleRestrictPing => {
                        format!("One of your roles prevents you from using the ping command.\n")
                    }
                }.as_str()
            }
        }
        if first_message {
            Handler::send_text(&content, command, ctx, ephemeral).await;
        } else {
            Handler::send_channel(&content, command.channel_id, ctx, false, None).await;
        }
    }

    async fn autocomplete_ping(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;
        let member = autocomplete
            .member
            .as_ref()
            .expect("/ping used outside guild");
        let member_admin = member
            .permissions
            .expect("member reference did not originate from interaction")
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);

        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.focused && field.kind == CommandOptionType::String {
                filter = field
                    .value
                    .as_ref()
                    .expect("/ping definition incorrect, should not contain subcommands")
                    .as_str()
                    .expect("This must be a string");
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        if let Ok(mut x) = db.clone().lock() {
            aliases = x.get_list_aliases_by_search(guild_id, 0, SUGGESTIONS, filter, member_admin)
        }
        autocomplete
            .create_autocomplete_response(&ctx.http, |response| {
                for list in aliases {
                    response.add_string_choice(&list, &list);
                }
                response
            })
            .await
            .expect("Failure communicating with discord api");
    }

    async fn autocomplete_join(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;
        let member = autocomplete
            .member
            .as_ref()
            .expect("/join was not triggered from within a guild");
        let member_admin = member
            .permissions
            .expect("member reference does not originate from interaction")
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);
        let mut userid = member.user.id;

        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.kind == CommandOptionType::User {
                let resolved_value = field
                    .resolved
                    .as_ref()
                    .expect("user field does not contain value?");
                userid = match resolved_value {
                    CommandDataOptionValue::User(ref target, _) => target.id,
                    _ => panic!("Invalid argument type"),
                };
            }
            if field.focused && field.kind == CommandOptionType::String {
                filter = field
                    .value
                    .as_ref()
                    .expect("Filter field does not contain value?")
                    .as_str()
                    .expect("string could not be interpreted as string");
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        if let Ok(mut x) = db.clone().lock() {
            aliases =
                x.get_list_joinable_by_search(guild_id, userid, SUGGESTIONS, filter, member_admin);
        }
        autocomplete
            .create_autocomplete_response(&ctx.http, |response| {
                for list in aliases {
                    response.add_string_choice(&list, &list);
                }
                response
            })
            .await
            .expect("Discord api error as follows: ");
    }

    async fn autocomplete_leave(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;
        let member = autocomplete
            .member
            .as_ref()
            .expect("/leave called outside guild");
        let member_admin = member
            .permissions
            .unwrap()
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);
        let mut userid = member.user.id;

        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.kind == CommandOptionType::User {
                let id_string = field.value.as_ref().expect("Field does not contain value");
                userid = UserId {
                    0: id_string
                        .as_str()
                        .expect("userid cannot be parsed as string")
                        .parse::<u64>()
                        .expect("id string cannot be parsed as u64"),
                };
            }
            if field.focused && field.kind == CommandOptionType::String {
                filter = field.value.as_ref().unwrap().as_str().unwrap(); // okay if it contains a string
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        if let Ok(mut x) = db.clone().lock() {
            aliases = x.get_list_membership_by_search(
                guild_id,
                userid,
                SUGGESTIONS,
                filter,
                member_admin,
            );
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

    async fn add_member(
        &self,
        guild_id: GuildId,
        list_name: &str,
        member_id: UserId,
        as_admin: bool,
        ctx: &Context,
    ) -> JoinResult {
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");
        // member_admin = Handler::can_manage_messages(command);

        if let Ok(mut x) = db.clone().lock() {
            let res_list_id = x.get_list_id_by_name(list_name, guild_id);
            if let Some(list_id) = res_list_id {
                let (_, list_join_permission, _) = x.get_list_permissions(list_id);
                if list_join_permission == PERMISSION::DENY && !as_admin {
                    return JoinResult::MissingPerms;
                }
                return x.add_member(member_id, list_id);
            } else {
                return JoinResult::ListDoesNotExist;
            }
        }
        return JoinResult::BotError;
    }

    async fn remove_member(
        &self,
        guild_id: GuildId,
        list_name: &str,
        member_id: UserId,
        as_admin: bool,
        ctx: &Context,
    ) -> JoinResult {
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        if let Ok(mut x) = db.clone().lock() {
            if let Some(list_id) = x.get_list_id_by_name(list_name, guild_id) {
                let (_, list_join_permission, _) = x.get_list_permissions(list_id);
                if list_join_permission == PERMISSION::DENY && !as_admin {
                    return JoinResult::MissingPerms;
                }
                if x.remove_member(member_id, list_id)
                    .expect("Failed to remove membership.")
                {
                    return JoinResult::Succes;
                } else {
                    return JoinResult::AlreadyMember;
                };
            } else {
                return JoinResult::ListDoesNotExist;
            }
        }
        return JoinResult::BotError;
    }

    async fn handle_join(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        let member_id: UserId = command
            .member
            .as_ref()
            .expect("Interaction not triggered by a member")
            .user
            .id;
        let member_admin = Handler::can_manage_messages(command);
        let list_names: Vec<CommandDataOption> = command.data.options.clone();

        let mut content = format!("Joining the following {} lists:", list_names.len());

        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();
            match self
                .add_member(guild_id, list_name_str, member_id, member_admin, ctx)
                .await
            {
                JoinResult::AlreadyMember => {
                    content += format!("\nYou already joined the list {}", list_name_str).as_str();
                }
                JoinResult::Succes => {
                    content += format!("\nAdded to list {}", list_name_str).as_str();
                }
                JoinResult::ListDoesNotExist => {
                    content += format!("\nThe list {} does not exist", list_name_str).as_str();
                }
                JoinResult::MissingPerms => {
                    content += format!(
                        "\nYou do not have permission to join the list {}.",
                        list_name_str
                    )
                    .as_str();
                }
                JoinResult::BotError => {
                    content += format!(
                        "\nSomething went wrong trying to join the \"{}\" list.",
                        list_name_str
                    )
                    .as_str();
                }
            }
        }
        Handler::send_text(&content, command, ctx, true).await;
    }

    async fn handle_leave(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        let member_id: UserId = command
            .member
            .as_ref()
            .expect("Interaction not triggered by a member")
            .user
            .id;
        let as_admin = Handler::can_manage_messages(command);
        let list_names: Vec<CommandDataOption> = command.data.options.clone();

        let mut content = format!("Leaving the following {} lists:", list_names.len());
        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();

            match self
                .remove_member(guild_id, list_name_str, member_id, as_admin, ctx)
                .await
            {
                JoinResult::Succes => {
                    content += format!("\nRemoved from list {}", list_name_str).as_str();
                }
                JoinResult::AlreadyMember => {
                    content += format!("\nYou were not in the list {}", list_name_str).as_str();
                }
                JoinResult::ListDoesNotExist => {
                    content += format!("\nThe list {} does not exist", list_name_str).as_str();
                }
                JoinResult::MissingPerms => {
                    content += format!(
                        "\nYou do not have permission to leave the list {}",
                        list_name_str
                    )
                    .as_str();
                }
                JoinResult::BotError => {
                    content +=
                        format!("\nFailed to remove member from list {}", list_name_str).as_str();
                }
            }
        }
        Handler::send_text(&content, command, ctx, true).await;
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

        let as_admin = Handler::can_manage_messages(command);
        if !as_admin {
            Handler::send_not_allowed(&command, &ctx).await;
            return;
        }

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        let mut content = "".to_string();

        if let Ok(mut x) = db.clone().lock() {
            if let Some(_) = x.add_list(guild_id, &list_name.to_string()) {
                content += format!("Creating list {}.", list_name).as_str();
            } else {
                content += "This list already exists.";
            };
        }

        Handler::send_text(&content, command, ctx, false).await;
    }

    async fn handle_remove(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
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

        let as_admin = Handler::can_manage_messages(command);
        if !as_admin {
            Handler::send_not_allowed(&command, &ctx).await;
            return;
        }

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        let mut content = format!("Removing list {}.", list_name);

        if let Ok(mut x) = db.clone().lock() {
            match x.get_list_id_by_name(list_name, guild_id) {
                Some(id) => {
                    x.remove_list(id).expect("list removal failed");
                    content = "Removed list.".to_string();
                }
                None => {
                    content += "List does not exist.";
                    ()
                }
            };
        }

        Handler::send_text(&content, command, ctx, false).await;
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

        let as_admin = Handler::can_manage_messages(command);
        if !as_admin {
            Handler::send_not_allowed(command, ctx).await;
            return;
        }
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        let mut content = String::new();
        if let Ok(mut x) = db.clone().lock() {
            let res_id = x.get_list_id_by_name(list_name, guild_id);

            if let Some(id) = res_id {
                x.add_alias_inline(id, list_alias);
                content = format!("Added alias {} to list {}.", list_alias, list_name);
            } else {
                content = format!("There is no list named {} to alias to.", list_alias);
            }
        }

        Handler::send_text(&content, command, ctx, false).await;
    }

    async fn handle_remove_alias(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
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

        let as_admin = Handler::can_manage_messages(command);
        if !as_admin {
            Handler::send_not_allowed(command, ctx).await;
            return;
        }
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        let mut content = String::new();
        if let Ok(mut x) = db.clone().lock() {
            if let Some(id) = x.get_list_id_by_name(list_name, guild_id) {
                if x.get_list_names(id).len() > 1 {
                    x.remove_alias(None, id, list_name).unwrap();
                    content = format!("Removed alias {}.", list_name);
                } else {
                    content = format!("You cannot remove the last alias of a list.")
                }
            } else {
                content = format!("There is no list named {} to alias to.", list_name);
            }
        }

        Handler::send_text(&content, command, ctx, false).await;
    }

    async fn autocomplete_alias(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let member = autocomplete.member.as_ref().unwrap();
        let member_admin = member
            .permissions
            .unwrap()
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);
        if !member_admin {
            autocomplete
                .create_autocomplete_response(&ctx.http, |response| response)
                .await
                .unwrap();
            return;
        }
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;

        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.focused && field.name == "name" {
                if let CommandDataOptionValue::String(ref s) = field.resolved.as_ref().unwrap() {
                    filter = s;
                }
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        if let Ok(mut x) = db.clone().lock() {
            aliases = x.get_list_aliases_by_search(guild_id, 0, SUGGESTIONS, filter, true)
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

    async fn handle_get(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.unwrap();
        let member_id: UserId = command
            .member
            .as_ref()
            .expect("Interaction not triggered by a member")
            .user
            .id;

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");
        let mut content = format!("You are in the following lists:");
        if let Ok(mut x) = db.clone().lock() {
            let list_ids = x.get_lists_with_member(guild_id, member_id).unwrap();
            for list_id in list_ids {
                let list_names = x.get_list_names(list_id);
                content += format!("\n{}", list_names.join(", ")).as_str();
            }
        }

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content(content).ephemeral(true))
            })
            .await
            .unwrap();
    }

    async fn handle_add(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        if !Handler::can_manage_messages(command) {
            Handler::send_not_allowed(&command, &ctx).await;
            return;
        }

        let target_value = command
            .data
            .options
            .iter()
            .filter(|x| x.name == "user")
            .next()
            .unwrap()
            .resolved
            .as_ref()
            .unwrap();
        let member_id = match target_value {
            CommandDataOptionValue::User(ref target, _) => target.id,
            _ => panic!("Invalid argument type"),
        };
        let list_names: Vec<CommandDataOption> = command.data.options.clone();

        let mut content = format!(
            "Attempting to add user with id {} to {} lists:",
            member_id,
            list_names.len() - 1
        );

        for list_name in list_names {
            if list_name.name == "user" {
                continue;
            };
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();
            match self
                .add_member(guild_id, list_name_str, member_id, true, ctx)
                .await
            {
                JoinResult::AlreadyMember => {
                    content += format!("\nUser was already in list {}", list_name_str).as_str();
                }
                JoinResult::Succes => {
                    content += format!("\nAdded to list {}", list_name_str).as_str();
                }
                JoinResult::ListDoesNotExist => {
                    content += format!("\nList {} does not exist", list_name_str).as_str();
                }
                JoinResult::MissingPerms => {
                    content += format!(
                        "\nYou do not have permission to add user to list {}.",
                        list_name_str
                    )
                    .as_str();
                }
                JoinResult::BotError => {
                    content += format!("\nAn error occured adding user to list {}", list_name_str)
                        .as_str();
                }
            }
        }
        Handler::send_text(&content, command, ctx, false).await;
    }

    async fn handle_kick(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.expect("No guild data found");
        if !Handler::can_manage_messages(command) {
            Handler::send_not_allowed(&command, &ctx).await;
            return;
        }

        let target_value = command
            .data
            .options
            .iter()
            .filter(|x| x.name == "user")
            .next()
            .unwrap()
            .resolved
            .as_ref()
            .unwrap();
        let member_id = match target_value {
            CommandDataOptionValue::User(ref target, _) => target.id,
            _ => panic!("Invalid argument type"),
        };
        let list_names: Vec<CommandDataOption> = command.data.options.clone();

        let mut content = format!(
            "Attempting to remove user with id {} from {} lists:",
            member_id,
            list_names.len() - 1
        );
        for list_name in list_names {
            if list_name.name == "user" {
                continue;
            };
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();

            match self
                .remove_member(guild_id, list_name_str, member_id, true, ctx)
                .await
            {
                JoinResult::Succes => {
                    content += format!("\nRemoved from list {}", list_name_str).as_str();
                }
                JoinResult::AlreadyMember => {
                    content +=
                        format!("\nThis person was not in the list {}", list_name_str).as_str();
                }
                JoinResult::ListDoesNotExist => {
                    content += format!("\nThe list {} does not exist", list_name_str).as_str();
                }
                JoinResult::MissingPerms => {
                    content += format!(
                        "\nYou do not have permission to remove users from the list {}",
                        list_name_str
                    )
                    .as_str();
                }
                JoinResult::BotError => {
                    content +=
                        format!("\nFailed to remove member from list {}", list_name_str).as_str();
                }
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

    async fn compose_list(
        &self,
        guild_id: GuildId,
        page: i64,
        filter: String,
        ctx: &Context,
    ) -> (CreateEmbed, Option<CreateActionRow>) {
        const PAGESIZE: usize = 20;
        let succes: bool;
        let mut maxlists: usize = 0;
        let lists: Vec<structures::PingList>;
        let mut labels: Vec<String> = Vec::new();
        let mut page_selection: (usize, usize) = (0, 0);
        let page_count: usize;
        let mut visible_lists: Vec<(String, String)> = Vec::new();

        let data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get::<DB>().unwrap();

        if let Ok(mut x) = db.clone().lock() {
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
                                    x.get_list_names(lists[list_index].id).join(", "),
                                    lists[list_index].description.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }
        let mut embed = CreateEmbed::default();

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
        let guild_id: GuildId = command.guild_id.expect("Command called outside guild");
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

        let (embed, action_row) = self.compose_list(guild_id, page, filter, ctx).await;

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

        let (embed, _action_row) = self.compose_list(guild_id, page, filter, ctx).await;

        component.defer(&ctx).await.unwrap();
        component
            .edit_original_interaction_response(&ctx.http, |response| response.set_embed(embed))
            .await
            .unwrap();
    }

    async fn handle_configure(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        if !Handler::can_manage_messages(command) {
            Handler::send_not_allowed(&command, &ctx).await;
            return;
        }
        let guild_id: GuildId = command.guild_id.expect("Command called outside guild");

        let mut embed = CreateEmbed::default();

        let subcom = &command.data.options[0];

        let data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get::<DB>().expect("Cannot find database");
        if let Ok(mut x) = db.clone().lock() {
            match subcom {
                CommandDataOption { ref name, .. } if name == "show" => {
                    let (a, b, c) = x.get_guild_ping_data(guild_id);
                    let (d, e, f) = x.get_propose_settings(guild_id);
                    embed
                        .color((0, 0, 0))
                        .description("test")
                        .field(
                            "Guild-wide",
                            format!(
                                "allow pings {}\nshared cooldown {}\nlist cooldown {}",
                                b, a, c
                            ),
                            false,
                        )
                        .field(
                            "proposal settings",
                            format!("enable {}\ntimeout {}\nthreshold {}", d, e, f),
                            false,
                        );
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "guild" => {
                    embed
                        .color((255, 0, 0))
                        .description("Configuring guild settings");
                    for setting in options {
                        match setting.name.as_str() {
                            "allow_ping" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_guild_canping(guild_id, *b).unwrap();
                                    embed.field(
                                        "Can ping",
                                        format!("public can ping set to {}", b),
                                        false,
                                    );
                                } else {
                                    panic!("The parameter disable_propose for configure role is incorrectly configured");
                                }
                            }
                            "set_guild_ping_cooldown" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Integer(ref b) = *temp {
                                    x.set_guild_general_cooldown(guild_id, *b as u64).unwrap();
                                    embed.field(
                                        "Guild ping cooldown",
                                        format!("Guild-wide cooldown set to {}", b),
                                        false,
                                    );
                                } else {
                                    panic!("The parameter disable_propose for configure role is incorrectly configured");
                                }
                            }
                            "set_list_ping_cooldown" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Integer(ref b) = *temp {
                                    x.set_guild_ping_cooldown(guild_id, *b as u64).unwrap();
                                    embed.field(
                                        "List ping cooldown",
                                        format!("List ping cooldown set to {}", b),
                                        false,
                                    );
                                } else {
                                    panic!("The parameter disable_propose for configure role is incorrectly configured");
                                }
                            }
                            _ => (),
                        }
                    }
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "role" => {
                    let role_value = options
                        .iter()
                        .filter(|x| x.name.as_str() == "role")
                        .next()
                        .expect("No role argument given")
                        .resolved
                        .as_ref()
                        .unwrap();
                    let role: RoleId = if let CommandDataOptionValue::Role(ref i) = *role_value {
                        i.id
                    } else {
                        panic!("List argument is not a valid integer")
                    };
                    for setting in options {
                        match setting.name.as_str() {
                            "propose" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::String(ref propose_perm) =
                                    resolved_value
                                {
                                    let perm = PERMISSION::from_str(&propose_perm).unwrap();
                                    x.set_role_canpropose(guild_id, role, perm).unwrap();
                                    embed.field(
                                        "disable propose",
                                        format!("Proposal permission: {}", perm),
                                        false,
                                    );
                                } else {
                                    panic!("The parameter propose for configure role is incorrectly configured");
                                }
                            }
                            "ping" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::String(ref mention_perm) =
                                    *resolved_value
                                {
                                    let perm = PERMISSION::from_str(&mention_perm).unwrap();
                                    x.set_role_canping(guild_id, role, perm).unwrap();
                                    embed.field("Ping permission: ", format!("{}", perm), false);
                                } else {
                                    panic!("The parameter ping for configure role is incorrectly configured");
                                }
                            }
                            "exclude_from_cooldown" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_role_ignore_cooldown(guild_id, role, *b).unwrap();
                                    embed.field("role cooldown", format!("{}", b), false);
                                } else {
                                    panic!("The parameter exclude_from_cooldown for configure role is incorrectly configured");
                                }
                            }
                            _ => (),
                        }
                    }
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "user" => {
                    let user_value = options
                        .iter()
                        .filter(|x| x.name.as_str() == "user")
                        .next()
                        .expect("No user argument given")
                        .resolved
                        .as_ref()
                        .unwrap();
                    let user: UserId =
                        if let CommandDataOptionValue::User(ref target, _) = *user_value {
                            target.id
                        } else {
                            panic!("List argument is not a valid integer")
                        };
                    for setting in options {
                        match setting.name.as_str() {
                            "propose" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::String(ref propose_perm) =
                                    *resolved_value
                                {
                                    let perm = PERMISSION::from_str(&propose_perm).unwrap();
                                    x.set_user_propose(guild_id, user, perm);
                                    embed.field(
                                        "disable propose",
                                        format!("Proposal permission: {}", perm),
                                        false,
                                    );
                                } else {
                                    panic!("The parameter propose for configure user is incorrectly configured");
                                }
                            }
                            "ping" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::String(ref mention_perm) =
                                    *resolved_value
                                {
                                    let perm = PERMISSION::from_str(&mention_perm).unwrap();
                                    x.set_user_canping(guild_id, user, perm);
                                    embed.field("Ping permission: ", format!("{}", perm), false);
                                } else {
                                    panic!("The parameter ping for configure user is incorrectly configured");
                                }
                            }
                            "exclude_from_cooldown" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_user_cooldown(guild_id, user, *b);
                                    embed.field("user cooldown", format!("{}", b), false);
                                } else {
                                    panic!("The parameter exclude_from_cooldown for configure role is incorrectly configured");
                                }
                            }
                            _ => (),
                        }
                    }
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "list" => {
                    let list_value = options
                        .iter()
                        .find(|x| x.name.as_str() == "list")
                        .expect("No list argument given")
                        .resolved
                        .as_ref()
                        .expect("Missing resolved value");
                    let list_str: &str =
                        if let CommandDataOptionValue::String(ref list_ref) = *list_value {
                            list_ref.as_str()
                        } else {
                            panic!("List argument is not valid")
                        };
                    if let Some(list) = x.get_list_id_by_name(list_str, guild_id) {
                        for setting in options {
                            match setting.name.as_str() {
                                "description" => {
                                    let resolved_value = setting.resolved.as_ref().unwrap();
                                    if let CommandDataOptionValue::String(ref description) =
                                        *resolved_value
                                    {
                                        x.set_description(list, description);
                                        embed.field(
                                            "set description",
                                            format!("{}", description),
                                            false,
                                        );
                                    } else {
                                        panic!("The parameter description for configure list is incorrectly configured");
                                    }
                                }
                                "cooldown" => {
                                    let resolved_value = setting.resolved.as_ref().unwrap();
                                    if let CommandDataOptionValue::Integer(ref cooldown) =
                                        *resolved_value
                                    {
                                        x.set_cooldown(list, *cooldown);
                                        embed.field("set cooldown", format!("{}", cooldown), false);
                                    } else {
                                        panic!("The parameter cooldown for configure list is incorrectly configured");
                                    }
                                }
                                "allow_join" => {
                                    let resolved_value = setting.resolved.as_ref().unwrap();

                                    if let CommandDataOptionValue::String(ref joinable) =
                                        *resolved_value
                                    {
                                        let perm = PERMISSION::from_str(&joinable).unwrap();
                                        x.set_joinable(list, perm);
                                        embed.field("set joinable", format!("{}", joinable), false);
                                    } else {
                                        panic!("The parameter allow_join for configure list is incorrectly configured");
                                    }
                                }
                                "allow_ping" => {
                                    let resolved_value = setting.resolved.as_ref().unwrap();
                                    if let CommandDataOptionValue::String(ref pingable) =
                                        *resolved_value
                                    {
                                        let perm = PERMISSION::from_str(&pingable).unwrap();
                                        x.set_pingable(list, perm);
                                        embed.field("allow ping", format!("{}", pingable), false);
                                    } else {
                                        panic!("The parameter allow_ping for configure list is incorrectly configured");
                                    }
                                }
                                "show" => {
                                    let temp = setting.resolved.as_ref().unwrap();
                                    if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                        x.set_visible(list, *b);
                                        embed.field("set visible", format!("{}", b), false);
                                    } else {
                                        panic!("The parameter show for configure list is incorrectly configured");
                                    }
                                }
                                _ => (),
                            }
                        }
                    } else {
                        embed.field(
                            "List not found",
                            format!("The list with name {} was not found.", list_str),
                            false,
                        );
                    }
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "channel" => {
                    let channel_value = options
                        .iter()
                        .filter(|x| x.name.as_str() == "channel")
                        .next()
                        .expect("No channel argument given")
                        .resolved
                        .as_ref()
                        .unwrap();
                    let channel: ChannelId =
                        if let CommandDataOptionValue::Channel(ref i) = *channel_value {
                            i.id
                        } else {
                            panic!("List argument is not a valid integer")
                        };
                    for setting in options {
                        match setting.name.as_str() {
                            "mentioning" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::String(ref mention_perm) =
                                    *resolved_value
                                {
                                    let perm = PERMISSION::from_str(&mention_perm).unwrap();
                                    embed.field("set mentioning", format!("{}", perm), false);
                                    x.set_channel_mentioning(channel, perm);
                                }
                            }
                            "proposing" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::String(ref propose_perm) =
                                    *resolved_value
                                {
                                    let perm = PERMISSION::from_str(&propose_perm).unwrap();
                                    embed.field("set proposing", format!("{}", perm), false);
                                    x.set_channel_proposing(channel, perm);
                                }
                            }
                            "visible_commands" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref visible_commands) =
                                    *resolved_value
                                {
                                    x.set_channel_public_visible(channel, *visible_commands);
                                    embed.field(
                                        "set public commands visible",
                                        format!("{}", visible_commands),
                                        false,
                                    );
                                }
                            }
                            _ => (),
                        }
                    }
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "proposals" => {
                    for setting in options {
                        match setting.name.as_str() {
                            "enabled" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref prop_enabled) =
                                    *resolved_value
                                {
                                    x.set_guild_canpropose(guild_id, *prop_enabled);
                                    embed.field(
                                        "enable proposals",
                                        format!("{}", prop_enabled),
                                        false,
                                    );
                                }
                            }
                            "timeout" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Integer(ref value) = *resolved_value
                                {
                                    x.set_propose_timeout(guild_id, *value as u64);
                                    embed.field("proposal timeout", format!("{}", value), false);
                                }
                            }
                            "threshold" => {
                                let resolved_value = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Integer(ref value) = *resolved_value
                                {
                                    x.set_propose_threshold(guild_id, *value as u64);
                                    embed.field("proposal threshold", format!("{}", value), false);
                                }
                            }
                            _ => (),
                        }
                    }
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "log" => {
                    embed
                        .color((255, 0, 0))
                        .description("Configuring logging settings");
                    for setting in options {
                        match setting.name.as_str() {
                            "set_channel" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Channel(ref b) = *temp {
                                    x.set_log_channel(guild_id, Some(b.id)).unwrap();
                                    embed.field(
                                        "log channel",
                                        format!("log channel set to {}", b.name.as_ref().unwrap()),
                                        false,
                                    );
                                } else {
                                    panic!("The parameter log_channel for configure log is incorrectly configured");
                                }
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.add_embed(embed))
            })
            .await
            .unwrap();
    }

    async fn autocomplete_configure(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;
        let member = autocomplete
            .member
            .as_ref()
            .expect("/configure being used outside guild");
        let member_admin = member
            .permissions
            .expect("Member reference not from interaction")
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);
        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.name == "list" {
                for subfield in &field.options {
                    if subfield.focused && subfield.name == "list" {
                        filter = subfield.value.as_ref().unwrap().as_str().unwrap();
                    }
                }
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        if let Ok(mut x) = db.clone().lock() {
            aliases = x.get_list_aliases_by_search(guild_id, 0, SUGGESTIONS, filter, member_admin)
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

    async fn handle_invalid(&self, _command: &ApplicationCommandInteraction) {}

    async fn handle_propose(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command
            .guild_id
            .expect("/propose called from outside guild");
        let channel_id = command.channel_id;
        let name = match command.data.options[0].resolved.as_ref().unwrap() {
            CommandDataOptionValue::String(a) => a,
            _ => panic!("invalid argument for propose"),
        };

        let mut proposal_id: Option<u64> = None;
        let as_admin = Handler::can_manage_messages(command);
        let member = command
            .member
            .as_ref()
            .expect("Member reference not from interaction");
        let role_ids: &Vec<RoleId> = &member.roles;

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        let mut override_canpropose: PERMISSION;

        if let Ok(mut x) = db.clone().lock() {
            let (general_propose, ..) = x.get_propose_settings(guild_id);

            override_canpropose = match (as_admin, general_propose) {
                (true, _) => PERMISSION::ALLOW,
                (false, true) => PERMISSION::NEUTRAL,
                (false, false) => PERMISSION::DENY,
            };
            let (_, _, propose_base) = x.get_channel_permissions(guild_id, channel_id);
            override_canpropose = override_canpropose.combine(propose_base);

            let (user_can_propose, ..) = x.get_user_permissions(guild_id, member.user.id);
            override_canpropose = override_canpropose.combine(user_can_propose);

            for role_id in role_ids {
                let (role_can_propose, ..) = x.get_role_permissions(guild_id, *role_id);
                override_canpropose = override_canpropose.combine(role_can_propose);
            }

            if override_canpropose != PERMISSION::DENY {
                let timestamp = serenity::model::Timestamp::now().unix_timestamp();
                proposal_id = x.start_proposal(guild_id, name, timestamp, channel_id);
                if proposal_id != None {
                    x.vote_proposal(proposal_id.unwrap(), member.user.id);
                }
            }
        } else {
            return;
        }

        let mut embed = CreateEmbed::default();

        if proposal_id != None && override_canpropose != PERMISSION::DENY {
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
                .custom_id(proposal_id.unwrap().to_string())
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
            let smes = command.get_interaction_response(&ctx.http).await.unwrap();
            let message_id = smes.id;
            if let Ok(mut x) = db.clone().lock() {
                x.complete_proposal(proposal_id.unwrap(), message_id); //FIXME: dunno what's wrong
            }
        } else {
            if override_canpropose == PERMISSION::DENY {
                embed
                    .title("You do not have permission to use /propose here.")
                    .color((255, 0, 0));
            } else {
                embed.title("This list already exists").color((0, 255, 0));
            }
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.add_embed(embed).ephemeral(true)
                        })
                })
                .await
                .unwrap();
        }
    }

    async fn handle_cancel_proposal(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        if !Handler::can_manage_messages(command) {
            command
                .create_interaction_response(&ctx.http, |res| {
                    res.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|mes| {
                            mes.ephemeral(true)
                                .content("You do not have permission to use this command")
                        })
                })
                .await
                .unwrap();
            return;
        }
        let mut data: Option<(ListId, MessageId, ChannelId)> = None;

        for (_, message) in &command.data.resolved.messages {
            if message.is_own(&ctx.cache) {
                match &message.components[..] {
                    [ActionRow { components: cs, .. }] => {
                        if let ActionRowComponent::Button(b) = &cs[0] {
                            if b.label.as_ref().unwrap() == "Vote" {
                                data = Some((
                                    b.custom_id.as_ref().unwrap().parse::<u64>().unwrap(),
                                    message.id,
                                    message.channel_id,
                                ));
                                break;
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        if let Some((list_id, message_id, channel_id)) = data {
            let mut data = ctx.data.write().await;
            let BotData { database: db, .. } = data
                .get_mut::<DB>()
                .expect("Could not find database in bot data");

            let mut embed = CreateEmbed::default();
            if let Ok(mut x) = db.clone().lock() {
                if x.remove_proposal(list_id).unwrap() {
                    x.remove_list(list_id).unwrap();
                    embed.title("Voting cancelled");
                } else {
                    if x.get_list_exists(list_id) {
                        embed.title("Proposal already accepted");
                    } else {
                        embed.title("Proposal already removed");
                    }
                }
            }

            channel_id
                .edit_message(&ctx.http, message_id, |prev| {
                    prev.components(|c| c).set_embed(embed)
                })
                .await
                .unwrap();
        }
        command
            .create_interaction_response(&ctx.http, |res| {
                res.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|mes| {
                        mes.ephemeral(true)
                            .content("Check the original proposal message for results.")
                    })
            })
            .await
            .unwrap();
    }

    async fn handle_accept_proposal(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        if !Handler::can_manage_messages(command) {
            command
                .create_interaction_response(&ctx.http, |res| {
                    res.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|mes| {
                            mes.ephemeral(true)
                                .content("You do not have permission to use this command")
                        })
                })
                .await
                .unwrap();
            return;
        }

        let mut data: Option<(ListId, MessageId, ChannelId)> = None;

        for (_, message) in &command.data.resolved.messages {
            if message.is_own(&ctx.cache) {
                match &message.components[..] {
                    [ActionRow { components: cs, .. }] => {
                        if let ActionRowComponent::Button(b) = &cs[0] {
                            if b.label.as_ref().unwrap() == "Vote" {
                                data = Some((
                                    b.custom_id.as_ref().unwrap().parse::<u64>().unwrap(),
                                    message.id,
                                    message.channel_id,
                                ));
                                break;
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        if let Some((list_id, message_id, channel_id)) = data {
            let mut data = ctx.data.write().await;
            let BotData { database: db, .. } = data
                .get_mut::<DB>()
                .expect("Could not find database in bot data");

            let mut embed = CreateEmbed::default();
            if let Ok(mut x) = db.clone().lock() {
                if x.accept_proposal(list_id) {
                    embed.title("Proposal acccepted");
                } else {
                    if x.get_list_exists(list_id) {
                        embed.title("Proposal already accepted");
                    } else {
                        embed.title("Proposal already removed");
                    }
                }
            }

            channel_id
                .edit_message(&ctx.http, message_id, |prev| {
                    prev.components(|c| c).set_embed(embed)
                })
                .await
                .unwrap();
        }
        command
            .create_interaction_response(&ctx.http, |res| {
                res.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|mes| {
                        mes.ephemeral(true)
                            .content("Check the original proposal message for results.")
                    })
            })
            .await
            .unwrap();
    }

    /// Gets called without guild context automatically
    async fn external_check_proposals(ctx: &Context) {
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");
        let now = serenity::model::Timestamp::now().unix_timestamp() as u64;

        let mut replies: Vec<(ChannelId, MessageId, bool)> = vec![];
        if let Ok(mut x) = db.clone().lock() {
            for (guild_id, proposal) in x.get_bot_proposals() {
                if let ProposalStatus::ACTIVE(list_id, votes, timestamp, channel_id, message_id) =
                    proposal
                {
                    let (_, vote_timeout, vote_threshold) = x.get_propose_settings(guild_id);
                    if votes >= vote_threshold {
                        x.accept_proposal(list_id);
                        if channel_id != 0 && message_id != 0 {
                            replies.push((channel_id, message_id, true));
                        }
                    } else if timestamp + vote_timeout <= now {
                        x.remove_proposal(list_id).unwrap();
                        x.remove_list(list_id).unwrap();
                        if channel_id != 0 && message_id != 0 {
                            replies.push((channel_id, message_id, false));
                        }
                    }
                }
            }
        }

        //TODO: this is lot of potential awaits, actually using async might be nice?
        for (channel_id, message_id, accepted) in replies {
            let reference =
                serenity::model::prelude::MessageReference::from((channel_id, message_id));
            let new_message = if accepted {
                "Proposal accepted"
            } else {
                "Proposal timed out"
            };
            Handler::send_channel(new_message, channel_id, ctx, false, Some(reference)).await;
            let mut embed = CreateEmbed::default();

            embed.description(new_message);
            channel_id
                .edit_message(&ctx.http, message_id, |data| {
                    data.set_embed(embed)
                        .set_components(serenity::builder::CreateComponents::default())
                })
                .await
                .unwrap();
        }
    }

    async fn handle_list_proposals(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id = command.guild_id.unwrap();
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        let mut embed = CreateEmbed::default();
        let now = serenity::model::Timestamp::now().unix_timestamp() as u64;

        if let Ok(mut x) = db.clone().lock() {
            let (_, timeout, threshold) = x.get_propose_settings(guild_id);
            let proposals = x.get_proposals(guild_id);
            if proposals.len() == 0 {
                embed.title("No proposals found");
            }
            for (name, proposal) in proposals {
                if let ProposalStatus::ACTIVE(_, votes, timestamp, channel_id, message_id) =
                    proposal
                {
                    let minutes = (timeout as i64 - (now - timestamp) as i64) / 60;
                    let (hours, minutes) = (minutes / 60, minutes % 60);

                    if message_id == 0 {
                        embed.field(
                            name,
                            format!(
                                "Has {} / {} votes, {} hours and {} minutes remaining.",
                                votes, threshold, hours, minutes,
                            ),
                            true,
                        );
                    } else {
                        embed.field(
                            name,
                            format!(
                                "Has {} / {} votes, {} hours and {} minutes remaining.\n{}",
                                votes,
                                threshold,
                                hours,
                                minutes,
                                message_id.link(channel_id, Some(guild_id)),
                            ),
                            true,
                        );
                    }
                }
            }
        }

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.add_embed(embed).ephemeral(true))
            })
            .await
            .unwrap();
    }

    async fn check_proposal(&self, list_id: ListId, ctx: &Context) -> ProposalStatus {
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");

        if let Ok(mut x) = db.clone().lock() {
            match x.get_proposal_data(list_id) {
                ProposalStatus::ACTIVE(_, votes, timestamp, t1, t2) => {
                    let guild_id = x.get_list_guild(list_id).unwrap();
                    let (_, _, vote_threshold) = x.get_propose_settings(guild_id);
                    if votes >= vote_threshold {
                        x.accept_proposal(list_id);
                        return ProposalStatus::ACCEPTED(list_id);
                    }
                    // Do not remove proposals when voting for social reasons?
                    return ProposalStatus::ACTIVE(list_id, votes, timestamp, t1, t2);
                }
                a => return a,
            };
        }
        panic!("could not get db access");
    }

    async fn propose_vote_from_component(
        &self,
        component: &MessageComponentInteraction,
        ctx: &Context,
    ) {
        let list_id = component.data.custom_id.parse::<u64>().unwrap();
        {
            let mut data = ctx.data.write().await;
            let BotData { database: db, .. } = data
                .get_mut::<DB>()
                .expect("Could not find database in bot data");
            if let Ok(mut x) = db.clone().lock() {
                x.vote_proposal(list_id, component.user.id);
            } else {
                panic!("database access error");
            }
        }

        let old_embed = &component.message.embeds[0];

        let mut embed = CreateEmbed::default();
        match self.check_proposal(list_id, ctx).await {
            ProposalStatus::ACCEPTED(..) => {
                embed.description("Proposal accepted");
            }
            ProposalStatus::DENIED => {
                // Doesn't happen
                embed.description("Proposal expired");
            }
            ProposalStatus::REMOVED => {
                // Won't happen because a list getting cancelled because you voted ain't fun
                embed.description("Proposal not found");
            }
            ProposalStatus::ACTIVE(_, votes, ..) => {
                embed.author(|ab| {
                    ab.icon_url(
                        old_embed
                            .author
                            .as_ref()
                            .expect("Old proposal embed malformed")
                            .icon_url
                            .as_ref()
                            .expect("Old proposal embed malformed"),
                    )
                    .name(
                        &old_embed
                            .author
                            .as_ref()
                            .expect("Old proposal embed malformed")
                            .name,
                    )
                });
                embed.title(
                    old_embed
                        .title
                        .as_ref()
                        .expect("Old proposal embed malformed"),
                );
                embed.description(format!("Votes: {}", votes));

                component
                    .create_interaction_response(&ctx.http, |res| {
                        res.kind(InteractionResponseType::UpdateMessage)
                            .interaction_response_data(|data| data.set_embed(embed))
                    })
                    .await
                    .unwrap();
                return;
            }
        }
        component
            .create_interaction_response(&ctx.http, |res| {
                res.kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|data| {
                        data.set_embed(embed)
                            .set_components(serenity::builder::CreateComponents::default())
                    })
            })
            .await
            .unwrap();
    }

    async fn check_triggers(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        member: &Member,
        triggers: Vec<LOGTRIGGER>,
    ) -> Vec<(ChannelId, String)> {
        let mut responses: Vec<(ChannelId, String)> = vec![];
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");
        if let Ok(mut x) = db.clone().lock() {
            'outer: for trigger in triggers {
                if let Some(id) = x.has_response(guild_id, trigger) {
                    for condition in x.get_response_conditions(id) {
                        if !match condition {
                            (LOGCONDITION::HasRole(role_id), invert, _) => {
                                member.roles.contains(&role_id) ^ invert
                            }
                        } {
                            continue 'outer;
                        }
                    }
                    let (channel_id, msg) = x.get_response(guild_id, id).unwrap();
                    responses.push((channel_id, msg));
                }
            }
        }
        responses
    }

    async fn _handle_context_ping(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let mut main_question = CreateActionRow::default();
        main_question.create_input_text(|text| {
            text.custom_id("top")
                .style(InputTextStyle::Paragraph)
                .label("Name something you like within Inverted Fate?")
                .value("Doesn't have to be your favorite thing, but try to be somewhat specific")
        });
        let mut cominter = CreateActionRow::default();
        cominter.create_input_text(|text| {
            text.custom_id("main")
                .style(InputTextStyle::Paragraph)
                .label("What do you expect from the community?")
                .value("Mostly anything is fine, don't worry!")
        });
        let mut friends = CreateActionRow::default();
        friends.create_input_text(|text| {
            text.custom_id("side")
                .style(InputTextStyle::Short)
                .label("If a friend referred you, please mention them")
        });
        let mut cc = CreateActionRow::default();
        cc.create_input_text(|text| {
            text.custom_id("bottom")
                .style(InputTextStyle::Paragraph)
                .label("Are you a content creator?")
                .value("Feel free to share links where applicable.")
        });

        let mut other_fandoms = CreateActionRow::default();
        other_fandoms.create_input_text(|text| {
            text.custom_id("bottomer")
                .style(InputTextStyle::Paragraph)
                .label("What other interests do you have?")
                .value("Fandoms, games, hobbies etc. outside of undertale.")
        });

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::Modal)
                    .interaction_response_data(|modal| {
                        modal
                            .title("Welcome to the Inverted Fate community.")
                            .custom_id("AAA")
                            .content("\
                                We would like you to answer a few questions to make sure you are here in good faith.\
                                There are no correct or incorrect answers to these questions.\
                                Once we have had a chance to consider your answers, you will be let into the greater community.\
                            ")

                            .components(|component| {
                                component
                                    .add_action_row(main_question)
                                    .add_action_row(cominter)
                                    .add_action_row(friends)
                                    .add_action_row(cc)
                                    .add_action_row(other_fandoms)
                            })
                    })
            })
            .await
            .unwrap();
    }

    async fn handle_log_purge(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        if !Handler::can_manage_messages(&command) {
            Handler::send_not_allowed(command, ctx).await;
            return;
        }

        if !command.app_permissions.unwrap().manage_messages() {
            Handler::send_text("Bot lacks permissions for purge", command, ctx, false).await;
            return;
        }

        let guild_id = command.guild_id.unwrap();
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");
        let mut nls = false;
        if let Ok(x) = db.clone().lock() {
            let so = x.get_log_channel(guild_id).unwrap();
            if so == None {
                nls = true;
            }
        }
        if nls {
            Handler::send_text("No log channel specified", command, ctx, false).await;
            return;
        }

        let mut ref_user: Option<&User> = None;
        for field in &command.data.options {
            if field.kind == CommandOptionType::User {
                let resolved_value = field.resolved.as_ref().unwrap();
                if let CommandDataOptionValue::User(ref target, _) = resolved_value {
                    ref_user = Some(target);
                };
            }
        }

        let mut messages = command
            .channel_id
            .messages(&ctx.http, |x| x.limit(25))
            .await
            .unwrap();
        messages.reverse();

        let mut select_menu_options: Vec<CreateSelectMenuOption> = Vec::new();
        for message in messages {
            if serenity::model::Timestamp::now().unix_timestamp()
                - message.timestamp.unix_timestamp()
                > 60 * 60 * (24 * 7 * 2 - 1)
            // Giving 1 hour to process the command in edge cases.
            {
                continue;
            }
            let label = match message.content.len() {
                0 => "Empty",
                1..=90 => message.content.as_str(),
                _ => &message.content[0..50],
            };
            select_menu_options.push(CreateSelectMenuOption::new(label, message.id));
        }

        if select_menu_options.len() == 0 {
            Handler::send_text("No recent messages to log or purge", command, ctx, true).await;
            return;
        }
        let mut select_menu = CreateSelectMenu::default();
        select_menu
            .placeholder("Select multiple messages")
            .max_values(min(18, select_menu_options.len() as u64))
            .options(|options| options.set_options(select_menu_options));

        if let Some(user) = ref_user {
            select_menu.custom_id(format!(
                "{}#{} - {}",
                user.name, user.discriminator, user.id
            ));
        } else {
            select_menu.custom_id("-");
        }

        let mut action_row = CreateActionRow::default();
        action_row.add_select_menu(select_menu);

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .ephemeral(true)
                            .components(|c| c.add_action_row(action_row))
                    })
            })
            .await
            .unwrap();
    }

    async fn process_log_purge(&self, component: &MessageComponentInteraction, ctx: &Context) {
        let member = component.member.as_ref().unwrap();
        if !member
            .permissions
            .unwrap()
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES)
        {
            component.defer(&ctx.http).await.unwrap();
            return;
        }

        let guild_id = component.guild_id.unwrap();
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data
            .get_mut::<DB>()
            .expect("Could not find database in bot data");
        let res_cid: ChannelId;
        if let Ok(x) = db.clone().lock() {
            let so = x.get_log_channel(guild_id).unwrap();
            res_cid = match so {
                Some(cid) => cid,
                None => return,
            }
        } else {
            panic!("Database access error");
        }

        let ids = &component
            .data
            .values
            .iter()
            .map(|a| MessageId::from(a.parse::<u64>().unwrap()))
            .collect::<Vec<MessageId>>();

        let mut messages = component
            .channel_id
            .messages(&ctx.http, |x| x.limit(30))
            .await
            .unwrap();
        messages.reverse();

        let mut embed = CreateEmbed::default();
        embed.title(format!("Delete log: {}", component.data.custom_id));
        for message in messages.into_iter().filter(|a| ids.contains(&a.id)) {
            const SUBMESSAGE_LENGTH: usize = 970;
            let mut citer = message.content.chars();
            let mut i = 0;
            loop {
                let submessage = citer.by_ref().take(SUBMESSAGE_LENGTH).collect::<String>();
                if submessage.len() == 0 && i > 0 {
                    break;
                }

                embed.field(
                    if i == 0 {
                        &message.author.name
                    } else {
                        "continued..."
                    },
                    if submessage.len() > 0 {
                        &submessage
                    } else {
                        "Embed / image"
                    },
                    false,
                );
                i += 1;
            }
        }

        component
            .channel_id
            .delete_messages(&ctx.http, ids)
            .await
            .unwrap(); //TODO: Might want to reverify no old messages are included
        res_cid
            .send_message(&ctx.http, |response| response.set_embed(embed))
            .await
            .unwrap();
        component.defer(&ctx.http).await.unwrap();
    }

    async fn handle_list_auto_response(
        &self,
        command: &ApplicationCommandInteraction,
        ctx: &Context,
    ) {
        if !Handler::can_manage_messages(&command) {
            Handler::send_not_allowed(command, ctx).await;
            return;
        }
        let guild_id: GuildId = command.guild_id.unwrap();
        let mut embed = CreateEmbed::default();

        let data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get::<DB>().unwrap();
        if let Ok(x) = db.clone().lock() {
            let responses = x.get_all_responses(guild_id).unwrap();
            for (channel_id, response_message, trigger) in responses {
                embed.field(
                    trigger,
                    format!(
                        "Message to channel with id {} as follows:\n{}",
                        channel_id, response_message
                    ),
                    false,
                );
            }
        } else {
            panic!("Could not get database access.");
        }
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.set_embed(embed))
            })
            .await
            .unwrap();
    }

    async fn handle_auto_response(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        if !Handler::can_manage_messages(&command) {
            Handler::send_not_allowed(command, ctx).await;
            return;
        }
        let guild_id: GuildId = command.guild_id.unwrap();

        let message: &str;

        let subcom = &command.data.options[0];
        let CommandDataOption {
            ref name, options, ..
        } = subcom;

        let data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get::<DB>().unwrap();
        if let Ok(mut x) = db.clone().lock() {
            let mut role_id = RoleId(0);
            let mut channel_id = ChannelId(0);
            let mut response_message = "";
            for setting in options {
                match &setting.resolved.as_ref().unwrap() {
                    CommandDataOptionValue::Role(role) => role_id = role.id,
                    CommandDataOptionValue::Channel(p_channel) => channel_id = p_channel.id,
                    CommandDataOptionValue::String(string) => response_message = string,
                    _ => (),
                }
            }
            let trigger = match name.as_str() {
                "role_add" => LOGTRIGGER::RoleAdd(role_id),
                "role_remove" => LOGTRIGGER::RoleRemove(role_id),
                "join_server" => LOGTRIGGER::JoinServer(),
                _ => panic!("invalid subcommand name"),
            };
            match command.data.name.as_str() {
                "add_auto_response" => {
                    x.add_response(guild_id, trigger, channel_id, response_message)
                        .unwrap();
                    message = "Added automatic response.";
                }
                "remove_auto_response" => {
                    if x.remove_response(guild_id, trigger).unwrap() {
                        message = "Removed automatic response.";
                    } else {
                        message = "Could not find automatic response.";
                    }
                }

                _ => panic!("Invalid auto response subcommand"),
            }
        } else {
            panic!("Could not get database access.");
        }

        Handler::send_text(message, command, ctx, false).await;
    }

    async fn handle_auto_response_condition(
        &self,
        command: &ApplicationCommandInteraction,
        ctx: &Context,
    ) {
        if !Handler::can_manage_messages(&command) {
            Handler::send_not_allowed(command, ctx).await;
            return;
        }
        let guild_id: GuildId = command.guild_id.unwrap();

        let message: &str;

        let subcom = &command.data.options[0];
        let CommandDataOption {
            ref name, options, ..
        } = subcom;

        let data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get::<DB>().unwrap();
        if let Ok(mut x) = db.clone().lock() {
            let mut role_id = RoleId(0);
            let mut target_role_id = RoleId(0);
            let mut invert = false;
            for setting in options {
                match (setting.name.as_str(), &setting.resolved.as_ref().unwrap()) {
                    ("role", CommandDataOptionValue::Role(role)) => role_id = role.id,
                    ("condition", _) => (),
                    ("required_role", CommandDataOptionValue::Role(role)) => {
                        target_role_id = role.id
                    }
                    ("invert", CommandDataOptionValue::Boolean(b)) => invert = *b,
                    _ => (),
                }
            }
            let trigger = match name.as_str() {
                "role_add" => LOGTRIGGER::RoleAdd(role_id),
                "role_remove" => LOGTRIGGER::RoleRemove(role_id),
                "join_server" => LOGTRIGGER::JoinServer(),
                _ => panic!("invalid subcommand name"),
            };

            let log_condition = LOGCONDITION::HasRole(target_role_id);
            if let Some(id) = x.has_response(guild_id, trigger) {
                match command.data.name.as_str() {
                    "add_auto_response_condition" => {
                        x.add_response_condition(id, log_condition, invert);
                        message = "Succesfully added condition."
                    }
                    "remove_auto_response_condition" => {
                        let conditions = x.get_response_conditions(id);
                        let ind_opt = conditions.iter().position(|(t_log_cond, t_inv, _)| {
                            *t_log_cond == log_condition && *t_inv == invert
                        });
                        if let Some(ind) = ind_opt {
                            x.remove_response_condition(conditions[ind].2);
                            message = "Succesfully removed condition";
                        } else {
                            message = "Condition not found"
                        }
                    }
                    _ => panic!("unrecognized auto response condition command"),
                }
            } else {
                message = "No such auto response exists.";
            }
        } else {
            panic!("Could not get database access.");
        }
        Handler::send_text(message, command, ctx, false).await;
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "ping" => self.handle_ping(&command, &ctx).await,
                // "ping with context" => self.handle_context_ping(&command, &ctx).await,
                "join" => self.handle_join(&command, &ctx).await,
                "leave" => self.handle_leave(&command, &ctx).await,
                "get" => self.handle_get(&command, &ctx).await,
                "list" => self.handle_list(&command, &ctx).await,
                "propose" => self.handle_propose(&command, &ctx).await,
                "list_proposals" => self.handle_list_proposals(&command, &ctx).await,
                // admin commands
                "alias" => self.handle_alias(&command, &ctx).await,
                "create" => self.handle_create(&command, &ctx).await,
                "remove" => self.handle_remove(&command, &ctx).await,
                "add" => self.handle_add(&command, &ctx).await,
                "kick" => self.handle_kick(&command, &ctx).await,
                "remove_alias" => self.handle_remove_alias(&command, &ctx).await,
                "configure" => self.handle_configure(&command, &ctx).await,
                "Cancel proposal" => self.handle_cancel_proposal(&command, &ctx).await,
                "Accept proposal" => self.handle_accept_proposal(&command, &ctx).await,
                "log_purge" => self.handle_log_purge(&command, &ctx).await,
                "list_auto_responses" => self.handle_list_auto_response(&command, &ctx).await,
                "add_auto_response" | "remove_auto_response" => {
                    self.handle_auto_response(&command, &ctx).await
                }
                "add_auto_response_condition" | "remove_auto_response_condition" => {
                    self.handle_auto_response_condition(&command, &ctx).await
                }
                _ => self.handle_invalid(&command).await,
            };
        } else if let Interaction::Autocomplete(completable) = interaction {
            match completable.data.name.as_str() {
                "ping" | "remove" => self.autocomplete_ping(&completable, &ctx).await,
                "configure" => self.autocomplete_configure(&completable, &ctx).await,
                "alias" => self.autocomplete_alias(&completable, &ctx).await,
                "remove_alias" => self.autocomplete_alias(&completable, &ctx).await,
                "add" | "join" => self.autocomplete_join(&completable, &ctx).await,
                "kick" | "leave" => self.autocomplete_leave(&completable, &ctx).await,
                _ => (),
            }
        } else if let Interaction::MessageComponent(component) = interaction {
            match component
                .message
                .interaction // we may get modals not spawned by interactions in the future, if so this may fail
                .as_ref()
                .unwrap()
                .name
                .as_str()
            {
                "list" => self.list_page_from_component(&component, &ctx).await,
                "propose" => self.propose_vote_from_component(&component, &ctx).await,
                "log_purge" => self.process_log_purge(&component, &ctx).await,
                _ => println!("Unknown interaction: {:?}", &component), // remove eventually?
            }
        } else if let Interaction::ModalSubmit(modal) = interaction {
            modal
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content("Succes"))
                })
                .await
                .unwrap();
        }
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old_if_available: Option<Member>,
        new: Member,
    ) {
        if let Some(old) = old_if_available {
            let oldset = BTreeSet::from_iter(old.roles);
            let newset = BTreeSet::from_iter(new.roles.iter().cloned());

            let mut responses: Vec<(ChannelId, String)> = vec![];
            responses.extend(
                self.check_triggers(
                    &ctx,
                    new.guild_id,
                    &new,
                    oldset
                        .difference(&newset)
                        .map(|id| LOGTRIGGER::RoleRemove(*id))
                        .collect(),
                )
                .await,
            );
            responses.extend(
                self.check_triggers(
                    &ctx,
                    new.guild_id,
                    &new,
                    newset
                        .difference(&oldset)
                        .map(|id| LOGTRIGGER::RoleAdd(*id))
                        .collect(),
                )
                .await,
            );
            for (channel, message_str) in responses {
                let message_str = message_str
                    .replace("{userID}", format!("{}", new.user.id).as_str())
                    .replace("{name}", new.user.name.as_str());
                channel
                    .send_message(&ctx.http, |message| message.content(message_str))
                    .await
                    .unwrap();
            }
        } else {
            println!(
                "could not resolve old roles of member with id {}",
                new.user.id
            );
        }
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        for (channel, message_str) in self
            .check_triggers(
                &ctx,
                new_member.guild_id,
                &new_member,
                vec![LOGTRIGGER::JoinServer()],
            )
            .await
        {
            let message_str = message_str
                .replace("{userID}", format!("{}", new_member.user.id).as_str())
                .replace("{name}", new_member.user.name.as_str());
            channel
                .send_message(&ctx.http, |message| message.content(message_str))
                .await
                .unwrap();
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

        {
            let mut data = ctx.data.write().await;
            let BotData { database: db, .. } = data
                .get_mut::<DB>()
                .expect("Could not find database in bot data");
            if let Ok(mut x) = db.clone().lock() {
                for guild in &ready.guilds {
                    x.add_guild(guild.id).ok();
                }
            }
        }

        for mut guild in ready.guilds {
            guild_commands::add_all_application_commands(&mut guild.id, &ctx).await;
        }
    }

    async fn cache_ready<'life0>(&'life0 self, ctx: Context, _guilds: Vec<GuildId>) {
        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx1 = Arc::clone(&ctx);
            tokio::spawn(async move {
                loop {
                    Handler::external_check_proposals(&ctx1).await;
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

#[tokio::main]
async fn main() {
    enum ParseType {
        SCANNING,
        IMPORT,
    }
    #[derive(std::cmp::PartialEq)]
    enum ProgramTarget {
        RUN,
        IMPORT,
    }

    dotenv().ok();

    let args = env::args();
    let mut parse_type = ParseType::SCANNING;
    let mut program_target = ProgramTarget::RUN;
    for arg in args {
        match parse_type {
            ParseType::SCANNING => {
                parse_type = match arg.as_str() {
                    "--import" => ParseType::IMPORT,
                    _ => ParseType::SCANNING,
                }
            }
            ParseType::IMPORT => {
                // There's a bunch of naive path traversal here, check this again later

                let mut db = Database::new("database.db");
                let filename = arg.split("/").collect::<Vec<&str>>();
                let filename = filename.last().unwrap();
                let filename = filename.split(".").next().unwrap();
                let gid = filename.parse::<u64>().unwrap();
                println!("guild id is {}", gid);
                pickle_import::import_pickled(&arg, GuildId::from(gid), &mut db);
                program_target = ProgramTarget::IMPORT;
                parse_type = ParseType::SCANNING;
            }
        }
    }

    if program_target != ProgramTarget::RUN {
        return;
    }

    // Load database
    let database: Database = Database::new("database.db");
    let database = Mutex::new(database);

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // The Application Id is usually the Bot User Id.
    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    let handler = Handler {
        is_loop_running: AtomicBool::default(),
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    // Build our client.
    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");
    {
        let mut data = client.data.write().await;
        let bot_data = BotData {
            database: Arc::new(database),
            global: std::collections::HashMap::new(),
            local: std::collections::HashMap::new(),
        };
        data.insert::<DB>(bot_data);
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
