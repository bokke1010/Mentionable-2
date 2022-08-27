use std::{
    cmp::{max, min},
    env,
    sync::{Arc, Mutex},
};

use serenity::{
    async_trait,
    builder::{
        CreateActionRow, CreateButton, CreateEmbed, CreateSelectMenu, CreateSelectMenuOption,
    },
    model::{
        application::{
            component::ButtonStyle,
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
        id::{ChannelId, GuildId, RoleId, UserId},
    },
    prelude::*,
};

mod structures;
use structures::structures::{ListId, PERMISSION};

mod guild_commands;
use crate::guild_commands::guild_commands::add_all_application_commands;
mod database;
use database::data_access::Database;

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

enum CommandInvalidReasons {
    ChannelRestriction,
    GuildRestriction,
    RoleRestriction,
    DoesNotExist,
}

struct Handler;

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
        let channel_id = command.channel_id;
        let member = command.member.as_ref().unwrap();
        let member_admin = Handler::can_manage_messages(command);
        let role_ids: &Vec<RoleId> = &member.roles;

        let list_names: Vec<CommandDataOption> = command.data.options.clone();
        let mut list_ids: Vec<ListId> = vec![];
        let mut members: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
        let mut invalid_lists: Vec<(String, ListInvalidReasons)> = vec![];

        let mut data = ctx.data.write().await;
        let BotData {
            database: db,
            global,
            local,
        } = data.get_mut::<DB>().unwrap();
        let timestamp = serenity::model::Timestamp::now().unix_timestamp() as u64;
        if let Ok(mut x) = db.clone().lock() {
            let mut override_canping = if member_admin {
                PERMISSION::ALLOW
            } else {
                PERMISSION::NEUTRAL
            };
            let mut override_cooldown = -1;
            for role_id in role_ids {
                let (_, role_canping, role_cooldown) = x.get_role_permissions(guild_id, *role_id);
                override_canping = override_canping.combine(role_canping);
                if override_cooldown * role_cooldown > 0 {
                    override_cooldown = min(override_cooldown, role_cooldown);
                } else {
                    override_cooldown = max(override_cooldown, role_cooldown)
                }
            }

            let (general_cooldown, general_canping, pingcooldown) = x.get_guild_ping_data(guild_id);
            let (_, channel_ping_rule, _) = x.get_channel_permissions(guild_id, channel_id);
            let channel_restrict_ping = channel_ping_rule == PERMISSION::DENY;
            if override_canping == PERMISSION::DENY {
                invalid_lists.push(("all".to_string(), ListInvalidReasons::RoleRestrictPing));
            } else if override_canping == PERMISSION::NEUTRAL {
                if !general_canping {
                    invalid_lists.push(("all".to_string(), ListInvalidReasons::GuildRestrictPing));
                } else if channel_restrict_ping {
                    invalid_lists.push(("".to_string(), ListInvalidReasons::ChannelRestrictPing));
                }
            }

            let last_global = global.entry(guild_id).or_insert(0);

            if general_cooldown && *last_global + (pingcooldown as u64) >= timestamp {
                invalid_lists.push(("all".to_string(), ListInvalidReasons::OnGlobalCooldown));
            }

            for list_name in list_names {
                let list_name = list_name.value.unwrap();
                let list_name = list_name.as_str().unwrap();

                if let Ok(list_id) = x.get_list_id_by_name(list_name, guild_id) {
                    let last_time = local.entry(list_id).or_insert(0);
                    let (list_cooldown, _, list_restrict_ping) = x.get_list_permissions(list_id);

                    if list_restrict_ping && !member_admin {
                        invalid_lists
                            .push((list_name.to_string(), ListInvalidReasons::ListRestrictPing));
                        continue;
                    }

                    if *last_time + (list_cooldown as u64) <= timestamp {
                        invalid_lists
                            .push((list_name.to_string(), ListInvalidReasons::OnLocalCooldown));
                        continue;
                    }
                    members.extend(x.get_members_in_list(list_id).unwrap());
                    list_ids.push(list_id);
                } else {
                    invalid_lists.push((list_name.to_string(), ListInvalidReasons::DoesNotExist));
                }
            }
        }
        let mut content = String::new();
        if invalid_lists.len() == 0 {
            global.insert(guild_id, timestamp);
            for list_id in list_ids {
                local.insert(list_id, timestamp);
            }

            if members.len() > 0 {
                content = format!("Mentioning {} members:\n", members.len());
                for member in members {
                    content += format!("<@{}>, ", member).as_str();
                }
            } else {
                content += "These lists are empty.";
            }
        } else {
            for falselist in invalid_lists {
                content += match falselist.1 {
                    ListInvalidReasons::ChannelRestrictPing => {
                        format!("\nPings are not allowed in this channel.")
                    }
                    ListInvalidReasons::DoesNotExist => {
                        format!("\nThe lsit {} does not exist.", falselist.0)
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

        Handler::send_text(content, command, ctx).await;
    }

    async fn autocomplete_ping(&self, autocomplete: &AutocompleteInteraction, ctx: &Context) {
        let guild_id = autocomplete.guild_id.expect("No guild data found");
        const SUGGESTIONS: usize = 5;
        let member = autocomplete.member.as_ref().unwrap();
        let member_admin = member
            .permissions
            .unwrap()
            .contains(serenity::model::permissions::Permissions::MANAGE_MESSAGES);

        let mut filter = "";
        for field in &autocomplete.data.options {
            if field.focused {
                filter = field.value.as_ref().unwrap().as_str().unwrap();
            }
        }
        let mut aliases: Vec<String> = Vec::new();

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        if let Ok(mut x) = db.clone().lock() {
            aliases = x
                .get_list_aliases_by_search(guild_id, 0, SUGGESTIONS, filter, member_admin)
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

    async fn add_member(
        &self,
        guild_id: GuildId,
        list_name: &str,
        member_id: UserId,
        as_admin: bool,
        ctx: &Context,
    ) -> bool {
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();
        // member_admin = Handler::can_manage_messages(command);

        if let Ok(mut x) = db.clone().lock() {
            let res_list_id = x.get_list_id_by_name(list_name, guild_id);
            if let Ok(list_id) = res_list_id {
                let (_, restricted_join, _) = x.get_list_permissions(list_id);
                if restricted_join || as_admin {
                    return false;
                }
                x.add_member(member_id, list_id)
                    .expect("Failed to add member to list");
                return true;
            }
        }
        return false;
    }

    async fn remove_member(
        &self,
        guild_id: GuildId,
        list_name: &str,
        member_id: UserId,
        as_admin: bool,
        ctx: &Context,
    ) -> bool {
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        if let Ok(mut x) = db.clone().lock() {
            let get_list_id = x.get_list_id_by_name(list_name, guild_id);
            if let Ok(list_id) = get_list_id {
                if !x.has_member(member_id, list_id) {
                    return false;
                }
                let (_, restricted_join, _) = x.get_list_permissions(list_id);
                if restricted_join || as_admin {
                    return false;
                }
                x.remove_member(member_id, list_id)
                    .expect("Failed to remove membership.");
                return true;
            }
            return false;
        }
        return false;
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

        let mut content = format!(
            "Attempting to add user with id {} to {} lists:",
            member_id,
            list_names.len()
        );

        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();
            if self
                .add_member(guild_id, list_name_str, member_id, member_admin, ctx)
                .await
            {
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
        let as_admin = Handler::can_manage_messages(command);
        let list_names: Vec<CommandDataOption> = command.data.options.clone();

        let mut content = format!(
            "Attempting to remove user with id {} from {} lists:",
            member_id,
            list_names.len()
        );
        for list_name in list_names {
            let list_name_val = list_name.value.unwrap();
            let list_name_str = list_name_val.as_str().unwrap();

            if self
                .remove_member(guild_id, list_name_str, member_id, as_admin, ctx)
                .await
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

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        let mut content = format!("Creating list {}.", list_name);

        if let Ok(mut x) = db.clone().lock() {
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

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        let mut content = String::new();
        if let Ok(mut x) = db.clone().lock() {
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

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();
        let mut content = format!("You are in the following lists:");
        if let Ok(mut x) = db.clone().lock() {
            let list_ids = x.get_lists_with_member(guild_id, member_id).unwrap();
            for list_id in list_ids {
                let list_names = x.get_list_names(list_id, guild_id).unwrap();
                content += format!("\n{}", list_names.join(", ")).as_str();
            }
        }

        Handler::send_text(content, command, ctx).await;
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
        let lists: Vec<structures::structures::PingList>;
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
                                    x.get_list_names(lists[list_index].id, guild_id)
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
        let as_admin = Handler::can_manage_messages(command);
        if !as_admin {
            return;
        }
        let guild_id: GuildId = command.guild_id.unwrap();

        let mut embed = CreateEmbed::default();

        let subcom = &command.data.options[0];

        let data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get::<DB>().unwrap();
        if let Ok(mut x) = db.clone().lock() {
            match subcom {
                CommandDataOption { ref name, .. } if name == "show" => {
                    //TODO: read all these options.
                    embed.color((0,0,0)).description("test")
                    .field("Guild-wide", "allow pings\nshared cooldown\nlist cooldown\nunaffected roles", false)
                    .field("Role-specific", "list of role - bool pairs for roles that cannot ping", false)
                    .field("List-specific", "description\ncooldown\nallowpings\nallowjoins\nvisible\naliases", false)
                    .field("channel-specific", "type (membership, mentioning, proposals, information (depends on what's configured as visible)\npermission (neutral, deny, allow)", false)
                    .field("proposal settings", "enable\ntimeout\nthreshold", false)
                    .field("Role respondance", "TODO", false);
                }
                CommandDataOption {
                    ref name, options, ..
                } if name == "guild" => {
                    for setting in options {
                        match setting.name.as_str() {
                            "allow_ping" => (),
                            "set_guild_ping_cooldown" => (),
                            "set_list_ping_cooldown" => (),
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
                            "disable_propose" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_role_propose(guild_id, role, *b).unwrap();
                                } else {
                                    panic!("The parameter disable_propose for configure role is incorrectly configured");
                                }
                            }
                            "disable_ping" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_role_canping(guild_id, role, *b).unwrap();
                                } else {
                                    panic!("The parameter disable_ping for configure role is incorrectly configured");
                                }
                            }
                            "exclude_from_cooldown" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_role_cooldown(guild_id, role, *b).unwrap();
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
                        .filter(|x| x.name.as_str() == "list")
                        .next()
                        .expect("No list argument given")
                        .resolved
                        .as_ref()
                        .unwrap();
                    let list: ListId = if let CommandDataOptionValue::Integer(ref i) = *list_value {
                        *i as u64
                    } else {
                        panic!("List argument is not a valid integer")
                    };
                    //TODO: list by name
                    for setting in options {
                        match setting.name.as_str() {
                            "description" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::String(ref b) = *temp {
                                    x.set_description(list, b).unwrap();
                                } else {
                                    panic!("The parameter description for configure list is incorrectly configured");
                                }
                            }
                            "cooldown" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Integer(ref b) = *temp {
                                    x.set_cooldown(list, *b as u64).unwrap();
                                } else {
                                    panic!("The parameter cooldown for configure list is incorrectly configured");
                                }
                            }
                            "allow_join" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_joinable(list, *b).unwrap();
                                } else {
                                    panic!("The parameter allow_join for configure list is incorrectly configured");
                                }
                            }
                            "allow_ping" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_pingable(list, *b).unwrap();
                                } else {
                                    panic!("The parameter allow_ping for configure list is incorrectly configured");
                                }
                            }
                            "show" => {
                                let temp = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *temp {
                                    x.set_visible(list, *b).unwrap();
                                } else {
                                    panic!("The parameter show for configure list is incorrectly configured");
                                }
                            }
                            _ => (),
                        }
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
                                let tme = setting.resolved.as_ref().unwrap();
                                if let CommandDataOptionValue::Boolean(ref b) = *tme {}
                                x.set_channel_mentioning(channel, PERMISSION::NEUTRAL)
                                    .unwrap()
                            }
                            "proposing" => x
                                .set_channel_proposing(channel, PERMISSION::NEUTRAL)
                                .unwrap(),
                            "visible_commands" => {
                                x.set_channel_public_visible(channel, true).unwrap()
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
                            "enabled" => (),
                            "timeout" => (),
                            "threshold" => (),
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

    async fn handle_invalid(&self, _command: &ApplicationCommandInteraction) {}

    async fn handle_propose(&self, command: &ApplicationCommandInteraction, ctx: &Context) {
        let guild_id: GuildId = command.guild_id.unwrap();
        let name = command.data.options[0].value.as_ref().unwrap().to_string();
        let proposal_id: u64;

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        if let Ok(mut x) = db.clone().lock() {
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

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        if let Ok(mut x) = db.clone().lock() {
            let list_id = x.get_list_id_by_name(&proposal_name, guild_id).unwrap();
            x.remove_proposal(list_id).unwrap();
        }

        Handler::send_text("canceled proposal".to_string(), command, ctx).await;
    }

    async fn check_proposal(&self, list_id: ListId, ctx: &Context) {
        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        if let Ok(mut x) = db.clone().lock() {
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

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();
        if let Ok(mut x) = db.clone().lock() {
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

        let mut data = ctx.data.write().await;
        let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();

        if let Ok(mut x) = db.clone().lock() {
            x.vote_proposal(list_id, component.user.id).unwrap();
        }
        self.check_proposal(list_id, ctx).await;
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
                "configure" => self.handle_configure(&command, &ctx).await,
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

        {
            let mut data = ctx.data.write().await;
            let BotData { database: db, .. } = data.get_mut::<DB>().unwrap();
            if let Ok(mut x) = db.clone().lock() {
                for guild in ready.guilds {
                    x.add_guild(guild.id).ok();
                }
            }
        }

        add_all_application_commands(&mut GuildId(466163515103641611), ctx).await;
    }
}

#[tokio::main]
async fn main() {
    // Load database
    let database: Database =
        Database::new("database.db".to_string()).expect("Database could not be loaded");
    let database = Mutex::new(database);

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
    // dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // The Application Id is usually the Bot User Id.
    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    let handler = Handler;

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
