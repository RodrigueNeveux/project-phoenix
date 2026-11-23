//! Runs [`ParityRequest`]s against the Discord REST API.

use phoenix_api::{
    RestClient,
    rest::{
        commands::{self as cmd, ApplicationCommand, CommandIndex, CommandScope, InteractionTarget},
        forum::ThreadSort,
        inbox::MentionsQuery,
    },
};
use serde_json::{Value, json};

use super::{ForumPost, ModAction, ParityReply, ParityRequest, SlashCommand, session_id, slash};
use crate::{
    backend::BackendError,
    discord_backend::convert::{id, message, sf},
    model::Id,
};

fn e(err: phoenix_api::Error) -> BackendError {
    BackendError(err.to_string())
}

fn target(guild: Option<Id>, channel: Id) -> InteractionTarget {
    InteractionTarget { guild: guild.map(sf), channel: sf(channel) }
}

fn ids(v: &[Id]) -> Vec<phoenix_api::models::Snowflake> {
    v.iter().copied().map(sf).collect()
}

/// Chat-input commands of both indexes, deduplicated, sorted by name.
pub fn merge_commands(indexes: &[CommandIndex]) -> Vec<SlashCommand> {
    let mut out: Vec<SlashCommand> = Vec::new();
    for idx in indexes {
        for c in idx.application_commands.iter().filter(|c| c.kind == 1) {
            if out.iter().any(|o| o.raw.get("id") == Some(&json!(c.id))) {
                continue;
            }
            let app_name =
                idx.applications.iter().find(|a| a.id == c.application_id).map(|a| a.name.clone()).unwrap_or_default();
            out.push(SlashCommand {
                name: c.name.clone(),
                description: c.description.clone(),
                app_name,
                raw: serde_json::to_value(c).unwrap_or(Value::Null),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub async fn request(rest: RestClient, req: ParityRequest) -> Result<ParityReply, BackendError> {
    let session = session_id();
    Ok(match req {
        ParityRequest::Commands { guild, channel } => {
            let scope = guild.map(|g| CommandScope::Guild(sf(g))).unwrap_or(CommandScope::Channel(sf(channel)));
            let (a, b) = tokio::join!(rest.get_command_index(scope), rest.get_command_index(CommandScope::Me));
            // A DM without bots answers 403/404: just no commands.
            let list: Vec<_> = [a, b].into_iter().filter_map(Result::ok).collect();
            ParityReply::Commands(merge_commands(&list))
        }
        ParityRequest::RunCommand { guild, channel, command, args } => {
            let c: ApplicationCommand = serde_json::from_value(command.clone()).map_err(|x| BackendError(x.to_string()))?;
            let options = slash::build_options(&command, &args).map_err(BackendError)?;
            rest.create_interaction(cmd::slash_body(&c, target(guild, channel), &session, options)).await.map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::Component { guild, channel, message, app, flags, custom_id, kind, values } => {
            let body = cmd::component_body(
                sf(app),
                target(guild, channel),
                &session,
                sf(message),
                flags,
                &custom_id,
                kind,
                values.as_deref(),
            );
            rest.create_interaction(body).await.map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::ModalSubmit { guild, channel, app, modal, custom_id, fields } => {
            let rows: Vec<Value> = fields
                .into_iter()
                .map(|(id, v)| json!({ "type": 1, "components": [{ "type": 4, "custom_id": id, "value": v }] }))
                .collect();
            let body = cmd::modal_body(sf(app), target(guild, channel), &session, sf(modal), &custom_id, json!(rows));
            rest.create_interaction(body).await.map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::Mentions { before, guild } => {
            let q = MentionsQuery { before: before.map(sf), guild: guild.map(sf), ..Default::default() };
            ParityReply::Messages(rest.get_mentions(q).await.map_err(e)?.iter().map(message).collect())
        }
        ParityRequest::DismissMention(m) => {
            rest.dismiss_mention(sf(m)).await.map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::SavedMessages => {
            let s = rest.get_saved_messages().await.map_err(e)?;
            ParityReply::Messages(s.results.iter().filter_map(|r| r.message.as_ref()).map(message).collect())
        }
        ParityRequest::SetSaved { channel, message: m, saved } => {
            if saved {
                rest.save_message(sf(channel), sf(m)).await
            } else {
                rest.unsave_message(sf(channel), sf(m)).await
            }
            .map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::VotePoll { channel, message: m, answers } => {
            rest.vote_poll(sf(channel), sf(m), &answers).await.map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::ForumPosts { forum, tags, offset } => {
            let r = rest.search_threads(sf(forum), ThreadSort::LastMessage, &ids(&tags), offset, false).await.map_err(e)?;
            let posts = r
                .threads
                .iter()
                .map(|t| ForumPost {
                    thread: id(t.id),
                    title: t.name.clone().unwrap_or_default(),
                    tags: t.applied_tags.iter().copied().map(id).collect(),
                    message_count: t.message_count.unwrap_or(0),
                    first_message: r.first_messages.iter().find(|m| m.channel_id == t.id).map(message),
                })
                .collect();
            ParityReply::ForumPosts { posts, has_more: r.has_more }
        }
        ParityRequest::CreatePost { forum, title, content, tags } => {
            let ch = rest.create_forum_post(sf(forum), &title, &content, &ids(&tags)).await.map_err(e)?;
            ParityReply::Channel(id(ch.id))
        }
        ParityRequest::Moderate { guild, user, action } => {
            let (g, u) = (sf(guild), sf(user));
            match action {
                ModAction::Timeout(s) => rest.timeout_member(g, u, s).await,
                ModAction::Kick => rest.kick_member(g, u).await,
                ModAction::Ban(s) => rest.ban_member(g, u, s).await,
                ModAction::Unban => rest.unban_member(g, u).await,
            }
            .map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::BulkDelete { channel, messages } => {
            rest.bulk_delete_messages(sf(channel), &ids(&messages)).await.map_err(e)?;
            ParityReply::Done
        }
        ParityRequest::CreateGroupDm(users) => {
            let ch = rest.create_group_dm(&ids(&users)).await.map_err(e)?;
            ParityReply::Channel(id(ch.id))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_dedups() {
        let a: CommandIndex = serde_json::from_str(
            r#"{"applications":[{"id":"9","name":"Bot"}],"application_commands":[
              {"id":"2","application_id":"9","type":1,"name":"zeta"},
              {"id":"1","application_id":"9","type":1,"name":"alpha"},
              {"id":"3","application_id":"9","type":3,"name":"Quote"}]}"#,
        )
        .unwrap();
        let m = merge_commands(&[a.clone(), a]);
        assert_eq!(m.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["alpha", "zeta"]);
        assert_eq!(m[0].app_name, "Bot");
    }
}
