use serenity::{
    model::{
        application::command::{Command, CommandOptionType, CommandType},
        id::GuildId,
        permissions,
    },
    prelude::*,
};

pub async fn add_all_application_commands(gid: &mut GuildId, ctx: &Context) -> Vec<Command> {
    let can_manage_messages = permissions::Permissions::MANAGE_MESSAGES;
    match gid.set_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| { //ANCHOR ping command
                    command
                        .name("ping")
                        .description("Pings all given lists")
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                })
                .create_application_command(|command| { //ANCHOR ping context
                    command
                        .name("ping with context")
                        .kind(CommandType::Message)
                })
                .create_application_command(|command| {
                    command
                        .name("create")
                        .description("Adds a list")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("the name to give this new list")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("remove")
                        .description("Removes a list")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("the name of the list to remove")
                                .kind(CommandOptionType::String)
                                .set_autocomplete(true)
                                .required(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("join")
                        .description("Join all given lists")
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("leave")
                        .description("Leave all given lists")
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("get")
                        .description("Get all lists you're subscribed to")
                })
                .create_application_command(|command| {
                    command
                        .name("list")
                        .description("List all lists")
                        .create_option(|option| {
                            option
                                .name("page")
                                .description("What part of the list you want to see")
                                .kind(CommandOptionType::Integer)
                                .required(false)
                        }).create_option(|option| {
                            option
                                .name("filter")
                                .description("Require this to be present in the name or description of a list.")
                                .kind(CommandOptionType::String)
                                .required(false)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("alias")
                        .description("Add more names to a list")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("A name a list is already known under")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .set_autocomplete(true)
                        }).create_option(|option| {
                            option
                                .name("alias")
                                .description("new alternative name")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|command| { // Only if proposals are enabled
                    command
                        .name("propose")
                        .description("Propose a list")
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("The proposed name for this ping list")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("list_proposals")
                        .description("See proposed lists")
                })
                .create_application_command(|command| { // Only if proposals are enabled
                    command
                        .name("Cancel proposal")
                        .default_member_permissions(can_manage_messages)
                        .kind(CommandType::Message)
                })
                .create_application_command(|command| { // Only if proposals are enabled
                    command
                        .name("Accept proposal")
                        .default_member_permissions(can_manage_messages)
                        .kind(CommandType::Message)
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("add")
                        .description("Add a user to lists")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|option| {
                            option
                                .name("user")
                                .description("The user to add")
                                .kind(CommandOptionType::User)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("kick")
                        .description("Kick a user from lists")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|option| {
                            option
                                .name("user")
                                .description("The user to remove")
                                .kind(CommandOptionType::User)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .set_autocomplete(true)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("remove_alias")
                        .description("Removes a list alias")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("The alias to be removed")
                                .kind(CommandOptionType::String)
                                .required(true)
                                .set_autocomplete(true)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("log_purge")
                        .description("Log and purge a complicated sequence of messages")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|option| {
                            option
                                .name("member")
                                .description("The user & id to add to this log")
                                .kind(CommandOptionType::User)
                                .required(false)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("list_auto_responses")
                        .description("Show all current automatic responses.")
                        .default_member_permissions(can_manage_messages)
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("add_auto_response")
                        .description("Add a automatic response")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_add")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("channel")
                                        .description("...")
                                        .kind(CommandOptionType::Channel)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("message")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_remove")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("channel")
                                        .description("...")
                                        .kind(CommandOptionType::Channel)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("message")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("join_server")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("channel")
                                        .description("...")
                                        .kind(CommandOptionType::Channel)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("message")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                })
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("remove_auto_response")
                        .description("Remove a automatic response")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_add")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_remove")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("join_server")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("add_auto_response_condition")
                        .description("Add a automatic response condition")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_add")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("condition")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                        .add_string_choice("role", "0")
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("required_role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("invert")
                                        .description("...")
                                        .kind(CommandOptionType::Boolean)
                                        .required(false)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_remove")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("condition")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                        .add_string_choice("role", "0")
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("required_role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("invert")
                                        .description("...")
                                        .kind(CommandOptionType::Boolean)
                                        .required(false)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("join_server")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("condition")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                        .add_string_choice("role", "0")
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("required_role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("invert")
                                        .description("...")
                                        .kind(CommandOptionType::Boolean)
                                        .required(false)
                                })
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("remove_auto_response_condition")
                        .description("Remove a automatic response condition")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_add")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("condition")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                        .add_string_choice("role", "0")
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("required_role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("invert")
                                        .description("...")
                                        .kind(CommandOptionType::Boolean)
                                        .required(false)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("role_remove")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("condition")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                        .add_string_choice("role", "0")
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("required_role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("invert")
                                        .description("...")
                                        .kind(CommandOptionType::Boolean)
                                        .required(false)
                                })
                        })
                        .create_option(|subcommand| {
                            subcommand
                                .name("join_server")
                                .description("...")
                                .kind(CommandOptionType::SubCommand)
                                .create_sub_option(|option| {
                                    option
                                        .name("condition")
                                        .description("...")
                                        .kind(CommandOptionType::String)
                                        .required(true)
                                        .add_string_choice("role", "0")
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("required_role")
                                        .description("...")
                                        .kind(CommandOptionType::Role)
                                        .required(true)
                                })
                                .create_sub_option(|option| {
                                    option
                                        .name("invert")
                                        .description("...")
                                        .kind(CommandOptionType::Boolean)
                                        .required(false)
                                })
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("configure")
                        .description("Houses various configuration subcommands")
                        .default_member_permissions(can_manage_messages)
                        .create_option(|catagory| {
                            catagory
                                .name("show").description("Shows all the current settings in a neat embed.")
                                .kind(CommandOptionType::SubCommand)
                        })
                        .create_option(|catagory| {
                            catagory
                            .name("guild").description("All guild-wide settings")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("allow_ping")
                                    .description("Allow members without explicit permissions to use /ping.")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("set_guild_ping_cooldown")
                                    .description("Set the duration of the guild-wide cooldown.")
                                    .kind(CommandOptionType::Integer)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("set_list_ping_cooldown")
                                    .description("Set the cooldown between seperate pings to the same list in seconds.")
                                    .kind(CommandOptionType::Integer)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("role").description("Settings that affect a specific role")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("role")
                                    .description("The role to configure.")
                                    .kind(CommandOptionType::Role)
                                    .required(true)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("propose")
                                    .description("Toggle whether or not people with this role can use /propose.")
                                    .kind(CommandOptionType::String)
                                    .required(false)
                                    .add_string_choice("Reset", "0")
                                    .add_string_choice("Deny", "1")
                                    .add_string_choice("Allow", "2")
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("ping")
                                    .description("Toggle whether or not people with this role can use /ping.")
                                    .kind(CommandOptionType::String)
                                    .required(false)
                                    .add_string_choice("Reset", "0")
                                    .add_string_choice("Deny", "1")
                                    .add_string_choice("Allow", "2")
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("exclude_from_cooldown")
                                    .description("Toggle whether or not people with this role can ignore the guild-wide cooldown.")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("user").description("Settings that affect a specific user")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("user")
                                    .description("The user to configure.")
                                    .kind(CommandOptionType::User)
                                    .required(true)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("propose")
                                    .description("Toggle whether or not people with this user can use /propose.")
                                    .kind(CommandOptionType::String)
                                    .required(false)
                                    .add_string_choice("Reset", "0")
                                    .add_string_choice("Deny", "1")
                                    .add_string_choice("Allow", "2")
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("ping")
                                    .description("Toggle whether or not people with this user can use /ping.")
                                    .kind(CommandOptionType::String)
                                    .required(false)
                                    .add_string_choice("Reset", "0")
                                    .add_string_choice("Deny", "1")
                                    .add_string_choice("Allow", "2")
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("exclude_from_cooldown")
                                    .description("Toggle whether or not people with this user can ignore the guild-wide cooldown.")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("list").description("All settings that affect a single pinglist")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("list")
                                    .description("The list to configure.")
                                    .kind(CommandOptionType::String)
                                    .required(true)
                                    .set_autocomplete(true)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("description")
                                    .description("Set the list description.")
                                    .kind(CommandOptionType::String)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("cooldown")
                                    .description("Override the list cooldown.")
                                    .kind(CommandOptionType::Integer)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("allow_join")
                                    .description("Allow anyone to join or leave this list.")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("allow_ping")
                                    .description("Allow anyone to mention this list.")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("show")
                                    .description("Hide this list from /list.")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("channel").description("Allows you to configure channels.")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("channel")
                                    .description("The channel to configure")
                                    .kind(CommandOptionType::Channel)
                                    .required(true)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("mentioning")
                                    .description("Whether or not mentioning lists is allowed by the channel.")
                                    .kind(CommandOptionType::String)
                                    .required(false)
                                    .add_string_choice("Reset", "0")
                                    .add_string_choice("Deny", "1")
                                    .add_string_choice("Allow", "2")
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("proposing")
                                    .description("Whether or not proposing new lists is allowed by the channel.")
                                    .kind(CommandOptionType::String)
                                    .required(false)
                                    .add_string_choice("Reset", "0")
                                    .add_string_choice("Deny", "1")
                                    .add_string_choice("Allow", "2")
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("visible_commands")
                                    .description("Whether or not commands like /list are visible in this channel.")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("proposals").description("Configure list proposals.")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("enabled")
                                    .description("Enable or disable proposals")
                                    .kind(CommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("timeout")
                                    .description("Set the timeout for a proposal in seconds")
                                    .kind(CommandOptionType::Integer)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("threshold")
                                    .description("Set the amount of votes required to accept a proposal")
                                    .kind(CommandOptionType::Integer)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("log").description("Settings that affect logging")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("set_channel")
                                    .description("The channel to send logs to.")
                                    .kind(CommandOptionType::Channel)
                                    .required(true)
                            })
                        })
                })

        }).await {
            Ok(result) => result,
            Err(error) => panic!("Problem assembling commands: {:?}", error),
        }
}
