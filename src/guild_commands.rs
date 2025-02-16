use serenity::{
    all::{Command, CommandOptionType, CommandType, CreateCommand, CreateCommandOption},
    model::{id::GuildId, permissions},
    prelude::*,
};

pub async fn add_all_application_commands(gid: &mut GuildId, ctx: &Context) -> Vec<Command> {
    let can_manage_messages = permissions::Permissions::MANAGE_MESSAGES;
    match gid
    .set_commands(
        &ctx.http,
        vec![
            CreateCommand::new("ping")
            .description("Pings all given lists")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "list",
                    "A single pinglist",
                )
                .required(true)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l2",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l3",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l4",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l5",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            ),
            CreateCommand::new("ping with context").kind(CommandType::Message),
            CreateCommand::new("create")
            .description("Adds a list")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "the name to give this new list",
                )
                .required(true),
            ),
            CreateCommand::new("remove")
            .description("Removes a list")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "the name of the list to remove",
                )
                .set_autocomplete(true)
                .required(true),
            ),
            CreateCommand::new("join")
            .description("Join all given lists")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "list",
                    "A single pinglist",
                )
                .required(true)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l2",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l3",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l4",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l5",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            ),
            CreateCommand::new("leave")
            .description("Leave all given lists")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "list",
                    "A single pinglist",
                )
                .required(true)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l2",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l3",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l4",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l5",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            ),
            CreateCommand::new("get").description("Get all lists you're subscribed to"),
            CreateCommand::new("list")
            .description("List all lists")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "page",
                    "What part of the list you want to see",
                )
                .required(false),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "filter",
                    "Require this to be present in the name or description of a list.",
                )
                .required(false),
            ),
            CreateCommand::new("alias")
            .description("Add more names to a list")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "A name a list is already known under",
                )
                .required(true)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "alias",
                    "new alternative name",
                )
                .required(true),
            ),
            CreateCommand::new("propose")
            .description("Propose a list")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The proposed name for this ping list",
                )
                .required(true),
            ),
            CreateCommand::new("list_proposals").description("See proposed lists"),
            CreateCommand::new("Cancel proposal")
                .default_member_permissions(can_manage_messages)
                .kind(CommandType::Message),
            CreateCommand::new("Accept proposal")
                .default_member_permissions(can_manage_messages)
                .kind(CommandType::Message),
            CreateCommand::new("add")
                .description("Add a user to lists")
                .default_member_permissions(can_manage_messages)
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The user to add",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "list",
                        "A single pinglist",
                    )
                    .required(true)
                    .set_autocomplete(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "l2",
                        "A single pinglist",
                    )
                    .required(false)
                    .set_autocomplete(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "l3",
                        "A single pinglist",
                    )
                    .required(false)
                    .set_autocomplete(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "l4",
                        "A single pinglist",
                    )
                    .required(false)
                    .set_autocomplete(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "l5",
                        "A single pinglist",
                    )
                    .required(false)
                    .set_autocomplete(true),
                ),
            CreateCommand::new("kick")
            .description("Kick a user from lists")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "The user to remove",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "list",
                    "A single pinglist",
                )
                .required(true)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l2",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l3",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l4",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "l5",
                    "A single pinglist",
                )
                .required(false)
                .set_autocomplete(true),
            ),
            CreateCommand::new("remove_alias")
            .description("Removes a list alias")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The alias to be removed",
                )
                .required(true)
                .set_autocomplete(true),
            ),
            CreateCommand::new("log_purge")
            .description("Log and purge a complicated sequence of messages")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "member",
                    "The user & id to add to this log",
                )
                .required(false),
            ),
            CreateCommand::new("list_auto_responses")
            .description("Show all current automatic responses.")
            .default_member_permissions(can_manage_messages),
            CreateCommand::new("add_auto_response")
            .description("Add a automatic response")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "role_add", "...")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "...",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "message",
                        "...",
                    )
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "role_remove",
                    "...",
                )
                .description("...")
                .kind(CommandOptionType::SubCommand)
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Channel, "channel", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "message", "...")
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "join_server",
                    "...",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Channel, "channel", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "message", "...")
                    .required(true),
                ),
            ),
            CreateCommand::new("remove_auto_response")
            .description("Remove a automatic response")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "role_add", "...")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "role_remove",
                    "...",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                ),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "join_server",
                "...",
            )),
            CreateCommand::new("add_auto_response_condition")
            .description("Add a automatic response condition")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "role_add", "...")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "condition",
                        "...",
                    )
                    .required(true)
                    .add_string_choice("role", "0"),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "required_role",
                        "...",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "invert",
                        "...",
                    )
                    .required(false),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "role_remove",
                    "...",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "condition", "...")
                    .required(true)
                    .add_string_choice("role", "0"),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "required_role",
                        "...",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Boolean, "invert", "...")
                    .required(false),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "join_server",
                    "...",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "condition", "...")
                    .required(true)
                    .add_string_choice("role", "0"),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "required_role",
                        "...",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Boolean, "invert", "...")
                    .required(false),
                ),
            ),
            CreateCommand::new("remove_auto_response_condition")
            .description("Remove a automatic response condition")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "role_add", "...")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "condition",
                        "...",
                    )
                    .required(true)
                    .add_string_choice("role", "0"),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "required_role",
                        "...",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "invert",
                        "...",
                    )
                    .required(false),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "role_remove",
                    "...",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Role, "role", "...")
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "condition", "...")
                    .required(true)
                    .add_string_choice("role", "0"),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "required_role",
                        "...",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Boolean, "invert", "...")
                    .required(false),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "join_server",
                    "...",
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "condition", "...")
                    .required(true)
                    .add_string_choice("role", "0"),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Role,
                        "required_role",
                        "...",
                    )
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Boolean, "invert", "...")
                    .required(false),
                ),
            ),
            CreateCommand::new("configure")
            .description("Houses various configuration subcommands")
            .default_member_permissions(can_manage_messages)
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "show", "Shows all the current settings in a neat embed.")
            )
            .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "guild", "All guild-wide settings")
            .add_sub_option(CreateCommandOption::new(CommandOptionType::Boolean, "allow_ping", "Allow members without explicit permissions to use /ping.")
                .required(false)
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "set_guild_ping_cooldown", "Set the duration of the guild-wide cooldown.")
            .required(false)
        )
        .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "set_list_ping_cooldown", "Set the cooldown between seperate pings to the same list in seconds.")
            .required(false)
        )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "role", "Settings that affect a specific role")
            .add_sub_option(CreateCommandOption::new(CommandOptionType::Role, "role", "The role to configure.")
            .required(true)
        )
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::String, "propose", "Toggle whether or not people with this role can use /propose.")
            .required(false)
            .add_string_choice("Reset", "0")
            .add_string_choice("Deny", "1")
            .add_string_choice("Allow", "2")
        )
        .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "ping", "Toggle whether or not people with this role can use /ping.")
            .required(false)
            .add_string_choice("Reset", "0")
            .add_string_choice("Deny", "1")
            .add_string_choice("Allow", "2")
        )
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::Boolean, "exclude_from_cooldown", "Toggle whether or not people with this role can ignore the guild-wide cooldown.")
            .required(false)
        )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "user", "Settings that affect a specific user")
            .add_sub_option(CreateCommandOption::new(CommandOptionType::User, "user", "The user to configure.")
                .required(true)
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "propose", "Toggle whether or not people with this user can use /propose.")
                .required(false)
                .add_string_choice("Reset", "0")
                .add_string_choice("Deny", "1")
                .add_string_choice("Allow", "2")
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "ping", "Toggle whether or not people with this user can use /ping.")
                .required(false)
                .add_string_choice("Reset", "0")
                .add_string_choice("Deny", "1")
                .add_string_choice("Allow", "2")
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::Boolean, "exclude_from_cooldown", "Toggle whether or not people with this user can ignore the guild-wide cooldown.")
                .required(false)
            )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "All settings that affect a single pinglist")
            .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "list", "The list to configure.")
                .required(true)
                .set_autocomplete(true)
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "description", "Set the list description.")
                .required(false)
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "cooldown", "Override the list cooldown.")
                .required(false)
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "allow_join", "Allow anyone to join or leave this list.")
                .required(false)
                .add_string_choice("Reset", "0")
                .add_string_choice("Deny", "1")
                .add_string_choice("Allow", "2")
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "allow_ping", "Allow anyone to mention this list.")
                .required(false)
                .add_string_choice("Reset", "0")
                .add_string_choice("Deny", "1")
                .add_string_choice("Allow", "2")
            )
            .add_sub_option(CreateCommandOption::new(CommandOptionType::Boolean, "show", "Hide this list from /list.")
                .required(false)
            )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "channel", "Allows you to configure channels.")
            .add_sub_option(CreateCommandOption::new(CommandOptionType::Channel, "channel", "The channel to configure")
                .required(true)
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "mentioning", "Whether or not mentioning lists is allowed by the channel.")
                .required(false)
                .add_string_choice("Reset", "0")
                .add_string_choice("Deny", "1")
                .add_string_choice("Allow", "2")
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "proposing", "Whether or not proposing new lists is allowed by the channel.")
                .required(false)
                .add_string_choice("Reset", "0")
                .add_string_choice("Deny", "1")
                .add_string_choice("Allow", "2")
            )
            .add_sub_option(
                CreateCommandOption::new(
                CommandOptionType::Boolean, "visible_commands", "Whether or not commands like /list are visible in this channel.")
                .required(false)
            )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "proposals", "Configure list proposals.")
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::Boolean, "enabled", "Enable or disable proposals")
                .required(false)
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "timeout", "Set the timeout for a proposal in seconds")
                .required(false)
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "threshold", "Set the amount of votes required to accept a proposal")
                .required(false)
            )
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "log", "Settings that affect logging")
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Channel, "set_channel", "The channel to send logs to.")
                .required(true)
            )
        )
    ]
)
.await
{
    Ok(result) => result,
    Err(error) => panic!("Problem assembling commands: {:?}", error),
}
}
